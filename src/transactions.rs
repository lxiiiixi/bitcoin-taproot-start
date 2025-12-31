use bitcoin::key::{Secp256k1, TweakedKeypair};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{self, LeafVersion, TapLeaf, TaprootBuilder, TaprootSpendInfo};
use bitcoin::transaction::Version;
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut,
    Txid, Witness, hex,
};
use serde_json::json;

use crate::alchemy_client::TxOut as AlchemyTxOut;
use crate::utils::build_inscription_script;

/// 构造 commit 交易：
/// - 花费一个 UTXO
/// - 创建一个 0.0001 BTC 的新 Taproot UTXO（给自己）
/// - 剩余作为找零
pub fn create_commit_tx(
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

pub fn create_inscription_commit_tx(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    funding_utxo: AlchemyTxOut,
    tweaked_keypair: &TweakedKeypair,
    inscription_script: ScriptBuf,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let commit_value: u64 = 10_000; // 0.0001 BTC
    let fee: u64 = 200;

    if funding_utxo.value < commit_value + fee {
        return Err("funding utxo not enough".into());
    }

    let change_value = funding_utxo.value - commit_value - fee;

    let (internal_xonly, _) = tweaked_keypair.to_keypair().x_only_public_key();

    // ---------- 1️⃣ 构建 Taproot script tree----------
    let taproot_spend_info: TaprootSpendInfo = TaprootBuilder::new()
        .add_leaf(0, inscription_script.clone())?
        .finalize(secp, internal_xonly)
        .unwrap();

    let merkle_root = taproot_spend_info.merkle_root();

    // ---------- 2️⃣ 用 taproot output key 生成 commit 地址 ----------
    let commit_address = Address::p2tr(secp, internal_xonly, merkle_root, Network::Testnet);

    // ---------- 3️⃣ 构造交易 input ----------
    let txin = TxIn {
        previous_output: OutPoint {
            txid: funding_utxo.txid.parse()?,
            vout: funding_utxo.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(),
    };

    // ---------- 4️⃣ 构造交易 outputs ----------
    let commit_output = TxOut {
        value: Amount::from_sat(commit_value),
        script_pubkey: commit_address.script_pubkey(),
    };

    let change_output = TxOut {
        value: Amount::from_sat(change_value),
        script_pubkey: commit_address.script_pubkey(),
    };

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![commit_output, change_output],
    };

    // ---------- 5️⃣ key-path sighash（注意：不是 script-path） ----------
    let mut sighash_cache = SighashCache::new(&mut tx);

    let sighash = sighash_cache.taproot_key_spend_signature_hash(
        0,
        &Prevouts::All(&[TxOut {
            value: Amount::from_sat(funding_utxo.value),
            script_pubkey: ScriptBuf::from_hex(&funding_utxo.script_pubkey.hex)?,
        }]),
        TapSighashType::Default,
    )?;

    // ---------- 6️⃣ Schnorr 签名（internal key） ----------
    let sig = secp.sign_schnorr(
        &bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref())?,
        &tweaked_keypair.to_keypair(),
    );

    tx.input[0].witness.push(sig.as_ref().to_vec());

    // ---------- 返回 ----------
    Ok(tx)
}

pub fn create_brc20_transaction(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    utxo: AlchemyTxOut,
    tweaked_keypair: &TweakedKeypair,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // ---------- 构造 commit value ----------
    let commit_value: u64 = 1_000; // 1_000 sats = 0.00001 BTC
    let fee: u64 = 200; // 100 sats = 0.000001 BTC

    if utxo.value < commit_value + fee {
        return Err("UTXO value not enough".into());
    }

    let change_value = utxo.value - commit_value - fee; // 给自己的找零

    println!("  💰 UTXO Value: {} sat", utxo.value);
    println!("  💰 Commit Value: {} sat", commit_value);
    println!("  💰 Fee: {} sat", fee);
    println!("  💰 Change Value: {} sat", change_value);

    // ---------- 构造 brc20 data 和 inscription script----------
    let brc20_data = json!({
        "p": "brc-20",
        "op": "deploy",
        "tick": "ordi",
        "max": "21000000",
        "lim": "1000"
    })
    .to_string();
    let inscription_script = build_inscription_script(&brc20_data);

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
        script_pubkey: address.script_pubkey(),
    };

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![input],
        output: vec![output],
    };

    println!(
        "inscription script hex: {}",
        inscription_script.to_hex_string()
    );

    // 构造 Taproot script tree
    let internal_pubkey = tweaked_keypair.to_keypair().x_only_public_key().0;
    println!("  🔑 Internal PubKey: {}", internal_pubkey.to_string());

    let taproot_builder = TaprootBuilder::new().add_leaf(0, inscription_script.clone())?;
    let taproot_info = taproot_builder.finalize(&secp, internal_pubkey).unwrap();

    // 获取输出公钥（聚合后的，用于地址）
    let output_pubkey = taproot_info.output_key().clone();
    let output_xonly = output_pubkey.to_x_only_public_key();
    // 创建 Taproot 地址
    let address = bitcoin::Address::p2tr(
        secp,
        output_xonly,
        taproot_info.merkle_root(),
        bitcoin::Network::Testnet,
    );

    println!("  📍 Address: {}", address.to_string());
    println!(
        "  📍 Address Script: {}",
        address.script_pubkey().to_hex_string()
    );

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

    let sig = secp.sign_schnorr(
        &bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref())?,
        &tweaked_keypair.to_keypair(),
    );

    tx.input[0].witness.push(sig.as_ref().to_vec());
    tx.input[0].witness.push(inscription_script.into_bytes());
    tx.input[0].witness.push(control_block.serialize());

    Ok(tx)
}
