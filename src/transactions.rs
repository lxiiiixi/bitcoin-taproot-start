use bitcoin::key::{Keypair, Secp256k1, TweakedKeypair};
use bitcoin::script::Builder;
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{self, LeafVersion, TapLeaf, TaprootBuilder, TaprootSpendInfo};
use bitcoin::transaction::Version;
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut,
    Txid, Witness, hex,
};

use crate::alchemy_client::TxOut as AlchemyTxOut;
use crate::utils::{build_inscription_script, build_rune_op_return};
use crate::wallets::TaprootWallet;

fn parse_taproot_schnorr_signature(
    sig_bytes: &[u8],
) -> Result<(bitcoin::secp256k1::schnorr::Signature, TapSighashType), Box<dyn std::error::Error>> {
    match sig_bytes.len() {
        64 => Ok((
            bitcoin::secp256k1::schnorr::Signature::from_slice(sig_bytes)?,
            TapSighashType::Default,
        )),
        65 => Ok((
            bitcoin::secp256k1::schnorr::Signature::from_slice(&sig_bytes[..64])?,
            TapSighashType::from_consensus_u8(sig_bytes[64])?,
        )),
        n => Err(format!("invalid schnorr signature length: {}", n).into()),
    }
}

fn p2tr_output_key_from_script_pubkey(
    spk: &ScriptBuf,
) -> Result<bitcoin::secp256k1::XOnlyPublicKey, Box<dyn std::error::Error>> {
    if !spk.is_p2tr() {
        return Err("prevout is not P2TR".into());
    }
    let bytes = spk.as_bytes();
    if bytes.len() != 34 {
        return Err("invalid v1 P2TR scriptPubKey length".into());
    }
    Ok(bitcoin::secp256k1::XOnlyPublicKey::from_slice(
        &bytes[2..34],
    )?)
}

/// 离线验证：检查某个 input 的 Taproot witness 签名是否能花费对应的 prevout。
///
/// 注意：这里要求 `prevouts.len() == tx.input.len()`，因为 Taproot sighash 会承诺所有 prevouts。
pub fn verify_taproot_input_signature(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    tx: &Transaction,
    input_index: usize,
    prevouts: &[TxOut],
) -> Result<bool, Box<dyn std::error::Error>> {
    if prevouts.len() != tx.input.len() {
        return Err(format!(
            "prevouts length mismatch: prevouts={} tx.inputs={}",
            prevouts.len(),
            tx.input.len()
        )
        .into());
    }
    let witness = &tx
        .input
        .get(input_index)
        .ok_or("input_index out of range")?
        .witness;

    if witness.is_empty() {
        return Err("empty witness".into());
    }

    let prevout = prevouts
        .get(input_index)
        .ok_or("missing prevout for input_index")?;

    if witness.len() == 1 {
        let sig_bytes = witness.nth(0).ok_or("missing signature")?;
        let (sig, sighash_type) = parse_taproot_schnorr_signature(sig_bytes)?;
        let output_key = p2tr_output_key_from_script_pubkey(&prevout.script_pubkey)?;
        let sighash = SighashCache::new(tx).taproot_key_spend_signature_hash(
            input_index,
            &Prevouts::All(prevouts),
            sighash_type,
        )?;
        let msg = bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref())?;
        secp.verify_schnorr(&sig, &msg, &output_key)?;
        return Ok(true);
    } else {
        return Err("witness length is not 1".into());
    }
}

/// 构造 commit 交易：
/// - 花费一个 UTXO
/// - 创建一个 0.0001 BTC 的新 Taproot UTXO（给自己）
/// - 剩余作为找零
pub fn create_first_tx(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    utxo: AlchemyTxOut,
    destination: &Address,
    tweaked_keypair: &TweakedKeypair,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let commit_value: u64 = 10_000; // 10_000 sats = 0.0001 BTC
    let fee: u64 = 200; // 100 sats = 0.000001 BTC

    if utxo.value < commit_value + fee {
        return Err("UTXO value not enough".into());
    }

    let change_value = utxo.value - commit_value - fee; // 给自己的找零

    println!("  💰 UTXO Value: {} sat", utxo.value);
    println!("  💰 Commit Value: {} sat", commit_value);
    println!("  💰 Fee: {} sat", fee);
    println!("  💰 Change Value: {} sat", change_value);

    // 1️⃣ Input
    let txin = TxIn {
        previous_output: OutPoint {
            txid: utxo.txid.parse()?,
            vout: utxo.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(),
    };

    // 2️⃣ Outputs
    let commit_output = TxOut {
        value: Amount::from_sat(commit_value),
        script_pubkey: destination.script_pubkey(),
    };

    let change_output = TxOut {
        value: Amount::from_sat(change_value),
        script_pubkey: destination.script_pubkey(),
    };

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![commit_output, change_output],
    };

    // 3️⃣ Taproot key-path sighash
    let mut sighash_cache = SighashCache::new(&mut tx);

    let sighash = sighash_cache.taproot_key_spend_signature_hash(
        0,
        &Prevouts::All(&[TxOut {
            value: Amount::from_sat(utxo.value),
            script_pubkey: ScriptBuf::from_hex(&utxo.script_pubkey.hex)?,
        }]),
        TapSighashType::Default,
    )?;

    // 4️⃣ Schnorr 签名
    let sig = secp.sign_schnorr(
        &bitcoin::secp256k1::Message::from_slice(sighash.as_ref())?,
        &tweaked_keypair.to_keypair(),
    );

    // 5️⃣ 填充 witness（key-path 只有一个元素）
    tx.input[0].witness.push(sig.as_ref().to_vec());

    Ok(tx)
}

pub fn create_commit_tx(
    secp: &Secp256k1<bitcoin::secp256k1::All>,

    // 用来“出钱”的普通 UTXO（funding utxo）
    funding_utxo: AlchemyTxOut,

    taproot_wallet: &TaprootWallet,
) -> Result<(Transaction, TaprootSpendInfo), Box<dyn std::error::Error>> {
    // ---------------- 参数 ----------------
    let commit_value: u64 = 10_000;
    let fee: u64 = 200; // 给足 fee，避免 mempool 拒绝

    if funding_utxo.value < commit_value + fee {
        return Err("funding utxo not enough".into());
    }

    let change_value = funding_utxo.value - commit_value - fee;

    // ---------------- 1️⃣ 构造 Taproot script tree（核心） ----------------
    let inscription_script = build_inscription_script(taproot_wallet.internal_xonly());

    let taproot_spend_info: TaprootSpendInfo = TaprootBuilder::new()
        .add_leaf(0, inscription_script.clone())?
        .finalize(secp, taproot_wallet.internal_xonly())
        .unwrap();

    // ---------------- 2️⃣ 构造 commit 地址（承诺脚本树） ----------------
    // 地址 ≈ script_pubkey 的人类编码
    let commit_address =
        taproot_wallet.get_commit_address_with_script_tree(secp, &taproot_spend_info);

    println!("  📍 Commit Address: {}", commit_address.to_string());

    // ---------------- 3️⃣ 构造交易 input（花费 funding utxo） ----------------
    let txin = TxIn {
        previous_output: OutPoint {
            txid: funding_utxo.txid.parse()?,
            vout: funding_utxo.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(),
    };

    // ---------------- 4️⃣ 构造交易 outputs ----------------
    // ① commit output：承诺 script tree 的 P2TR UTXO
    let commit_output = TxOut {
        value: Amount::from_sat(commit_value),
        script_pubkey: commit_address.script_pubkey(),
    };

    // ② 找零（通常回到普通钱包地址，这里示例用同一个 internal key）
    let change_address = taproot_wallet.get_internal_address();

    println!("  📍 Change Address: {}", change_address.to_string());

    let change_output = TxOut {
        value: Amount::from_sat(change_value),
        script_pubkey: change_address.script_pubkey(),
    };

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![commit_output, change_output],
    };
    // 虽然这里用的是跟创建钱包时同样的 internal key 以及同样的规则，但是还是会生成一个新的地址
    // 是可以被同一个私钥控制的，但是地址是不同的，有利于隐私保护

    // ---------------- 5️⃣ key-path sighash（不是 script-path） ----------------
    let mut sighash_cache = SighashCache::new(&mut tx);

    let sighash = sighash_cache.taproot_key_spend_signature_hash(
        0,
        &Prevouts::All(&[TxOut {
            value: Amount::from_sat(funding_utxo.value),
            script_pubkey: ScriptBuf::from_hex(&funding_utxo.script_pubkey.hex)?,
        }]),
        TapSighashType::Default,
    )?;

    // ---------------- 6️⃣ Schnorr 签名（internal key） ----------------
    let sig = taproot_wallet.sign_keypath(
        secp,
        &bitcoin::secp256k1::Message::from_slice(sighash.as_ref())?,
    );

    tx.input[0].witness.push(sig.as_ref().to_vec());

    // ---------------- 返回 ----------------
    // 要把 taproot_spend_info 返回，reveal tx 需要它拿 control_block
    Ok((tx, taproot_spend_info))
}

pub fn create_brc20_transaction(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    utxo: AlchemyTxOut,
    taproot_wallet: &TaprootWallet,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // ---------- 构造 commit value ----------
    let commit_value: u64 = 9_800; // 9_800 sats = 0.000098 BTC
    let fee: u64 = 200; // 100 sats = 0.000001 BTC

    if utxo.value < commit_value + fee {
        return Err("UTXO value not enough".into());
    }

    let change_value = utxo.value - commit_value - fee; // 给自己的找零

    println!("  💰 UTXO Value: {} sat", utxo.value);
    println!("  💰 Commit Value: {} sat", commit_value);
    println!("  💰 Fee: {} sat", fee);
    println!("  💰 Change Value: {} sat", change_value);

    let input = TxIn {
        previous_output: OutPoint {
            txid: utxo.txid.parse()?,
            vout: utxo.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(),
    };

    let output = TxOut {
        value: Amount::from_sat(commit_value),
        script_pubkey: taproot_wallet.get_internal_address().script_pubkey(),
    };

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![input],
        output: vec![output],
    };

    // ---------- 构造 brc20 data 和 inscription script----------
    let inscription_script = build_inscription_script(taproot_wallet.internal_xonly());

    println!(
        "inscription script hex: {}",
        inscription_script.to_hex_string()
    );

    // 构造 Taproot script tree
    let taproot_builder = TaprootBuilder::new().add_leaf(0, inscription_script.clone())?;
    let taproot_info = taproot_builder
        .finalize(&secp, taproot_wallet.internal_xonly())
        .unwrap();

    // 获取输出公钥（聚合后的，用于地址）
    // let output_pubkey = taproot_info.output_key().clone();
    // let output_xonly = output_pubkey.to_x_only_public_key();
    // 创建 Taproot 地址
    // let address = bitcoin::Address::p2tr(
    //     secp,
    //     output_xonly,
    //     taproot_info.merkle_root(),
    //     bitcoin::Network::Testnet,
    // );

    // println!("  📍 Address: {}", address.to_string());
    // println!(
    //     "  📍 Address Script: {}",
    //     address.script_pubkey().to_hex_string()
    // );

    let control_block = taproot_info
        .control_block(&(
            inscription_script.clone(),
            bitcoin::taproot::LeafVersion::TapScript,
        ))
        .unwrap();

    let mut sighash_cache = SighashCache::new(&mut tx);

    let prevout = TxOut {
        value: Amount::from_sat(utxo.value),
        script_pubkey: ScriptBuf::from_hex(&utxo.script_pubkey.hex)?,
    };

    let leaf_hash = TapLeafHash::from_script(&inscription_script, LeafVersion::TapScript);

    let sighash = sighash_cache.taproot_script_spend_signature_hash(
        0, // input index
        // 签名 prevout 的 (value, scriptPubKey)
        &Prevouts::All(&[prevout]),
        leaf_hash,
        TapSighashType::Default,
    )?;

    let sig = taproot_wallet.sign_internal(
        secp,
        &bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref())?,
    );

    tx.input[0].witness.push(sig.as_ref().to_vec());
    tx.input[0].witness.push(inscription_script.into_bytes());
    tx.input[0].witness.push(control_block.serialize());

    Ok(tx)
}

pub fn create_runes_tx(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    utxo: AlchemyTxOut,
    taproot_wallet: &TaprootWallet,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let fee: u64 = 200;

    if utxo.value < fee {
        return Err("UTXO value not enough".into());
    }

    let change_value = utxo.value - fee; // 给自己的找零

    // -------- Input --------
    let input = TxIn {
        previous_output: OutPoint {
            txid: utxo.txid.parse()?,
            vout: utxo.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Default::default(),
    };

    // -------- Output 0: 找零 --------
    let change_output = TxOut {
        value: Amount::from_sat(change_value),
        script_pubkey: taproot_wallet.get_internal_address().script_pubkey(),
    };

    // -------- Output 1: OP_RETURN (Rune) --------
    let rune_output = TxOut {
        value: Amount::from_sat(0),
        // script_pubkey: build_rune_op_return(),
        script_pubkey: Builder::new()
            .push_slice(&[
                0x6a, 0x5d, 0x28, 0x02, 0x07, 0x04, 0xea, 0xda, 0xa9, 0xea, 0x92, 0xe0, 0xaa, 0xca,
                0xaf, 0x85, 0x01, 0x05, 0xb0, 0x09, 0xc0, 0x10, 0x34, 0x00, 0x10, 0x80, 0x60, 0x80,
                0x80, 0xb9, 0xf6, 0xcd, 0xbf, 0x5f, 0x08, 0xc0, 0xa0, 0x0a, 0x0a, 0x80, 0xc8, 0xaf,
                0xa0, 0x25,
            ])
            .into_script(),
    };

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![input],
        output: vec![change_output, rune_output],
    };

    for (i, out) in tx.output.iter().enumerate() {
        println!(
            "output[{}] value={} script={}",
            i,
            out.value.to_sat(),
            out.script_pubkey.to_hex_string()
        );
    }

    let mut sighash_cache = SighashCache::new(&mut tx);

    let sighash = sighash_cache.taproot_key_spend_signature_hash(
        0,
        &Prevouts::All(&[TxOut {
            value: Amount::from_sat(utxo.value),
            script_pubkey: ScriptBuf::from_hex(&utxo.script_pubkey.hex)?,
        }]),
        TapSighashType::Default,
    )?;

    let sig = taproot_wallet.sign_keypath(
        secp,
        &bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref())?,
    );

    tx.input[0].witness.push(sig.as_ref().to_vec());

    Ok(tx)
}
