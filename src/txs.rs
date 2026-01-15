use crate::alchemy_client::AlchemyClient;
use crate::transactions::{
    create_brc20_transaction, create_commit_tx, create_first_tx, create_runes_tx,
    verify_taproot_input_signature,
};
use crate::utils::build_inscription_script;
use crate::wallets::TaprootWallet;
use bitcoin::key::{Secp256k1, TweakedKeypair};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
use bitcoin::transaction::Version;
use bitcoin::{Address, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};

// 第一笔交易(只是做一个简单的转账) - a7bb32cdb8d77f480804e0743db3b181938a9f4745392b4f825afa5032895c2f
pub async fn tx_first_commit(
    alchemy: &AlchemyClient,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    address: &Address,
    tweaked_keypair: &TweakedKeypair,
) {
    if let Some(tx_out) = alchemy
        .get_tx_out(
            "048b557b5c733c9a782f954712b86df99cd0923dcb51ffcda3116f1d87e895b5",
            0,
            true,
        )
        .await
        .unwrap()
    {
        println!("UTXO value: {} sats", tx_out.value);
        println!("Confirmations: {}", tx_out.confirmations);

        let tx = create_first_tx(&secp, tx_out, &address, &tweaked_keypair).unwrap();
        let txid = alchemy.broadcast_tx(&tx).await.unwrap();
        println!("  📍 TXID: {}", txid);
    }
}

// 第二笔交易(output使用支持 Taproot Script Tree 的地址) f3d108c6d250b8b4f54178de18f1e4c631be280a154d0c5d082a64e1d8c4c2a5
pub async fn tx_inscription_commit(
    alchemy: &AlchemyClient,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    taproot_wallet: &TaprootWallet,
    txid: &str,
    vout_index: u32,
) {
    if let Some(tx_out) = alchemy.get_tx_out(txid, vout_index, true).await.unwrap() {
        println!("UTXO value: {} sats", tx_out.value);
        println!("Confirmations: {}", tx_out.confirmations);

        let (tx, taproot_spend_info) = create_commit_tx(&secp, tx_out, &taproot_wallet).unwrap();
        println!(
            "  📍 Taproot Spend Info: {:?}",
            taproot_spend_info.merkle_root()
        );
        let txid = alchemy.broadcast_tx(&tx).await.unwrap();
        println!("  📍 TXID: {}", txid);
    }
}

pub async fn tx_brc20_deploy(
    alchemy: &AlchemyClient,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    taproot_wallet: &TaprootWallet,
    txid: &str,
    vout_index: u32,
) {
    if let Some(tx_out) = alchemy.get_tx_out(txid, vout_index, true).await.unwrap() {
        println!("UTXO value: {} sats", tx_out.value);
        println!("Confirmations: {}", tx_out.confirmations);

        let tx = create_brc20_transaction(&secp, tx_out, &taproot_wallet).unwrap();
        let txid = alchemy.broadcast_tx(&tx).await.unwrap();
        println!("  📍 TXID: {}", txid);
    }
}

pub async fn tx_rune_deploy(
    alchemy: &AlchemyClient,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    taproot_wallet: &TaprootWallet,
    txid: &str,
    vout_index: u32,
) {
    if let Some(tx_out) = alchemy.get_tx_out(txid, vout_index, true).await.unwrap() {
        println!("UTXO value: {} sats", tx_out.value);
        println!("Confirmations: {}", tx_out.confirmations);
        let tx = create_runes_tx(&secp, tx_out, &taproot_wallet).unwrap();
        let txid = alchemy.broadcast_tx(&tx).await.unwrap();
        println!("  📍 TXID: {}", txid);
    }
}

pub async fn verify_signature(
    alchemy: &AlchemyClient,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    taproot_wallet: &TaprootWallet,
    txid: &str,
    vout_index: u32,
) {
    let Some(utxo) = alchemy.get_tx_out(txid, vout_index, true).await.unwrap() else {
        println!("❌ UTXO not found or already spent");
        return;
    };

    let prevout = TxOut {
        value: Amount::from_sat(utxo.value),
        script_pubkey: ScriptBuf::from_hex(&utxo.script_pubkey.hex).unwrap(),
    };

    if !prevout.script_pubkey.is_p2tr() {
        println!("❌ prevout is not P2TR, script={}", prevout.script_pubkey);
        return;
    }

    // 为了计算 Taproot sighash/签名，必须有一笔具体的交易结构。
    // 这里构造一笔“只用于验证”的临时交易（1 input / 1 output，不广播）。
    let fee: u64 = 200;
    if utxo.value <= fee {
        println!("❌ UTXO value not enough for fee");
        return;
    }
    let send_value = utxo.value - fee;

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: utxo.txid.parse().unwrap(),
                vout: utxo.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(send_value),
            script_pubkey: taproot_wallet.get_internal_address().script_pubkey(),
        }],
    };

    // 只验证这个 UTXO，所以 prevouts 只需要一个元素。
    let prevouts = [prevout];

    let script_pubkey = taproot_wallet.get_internal_address().script_pubkey();

    println!("  📍 Script Pubkey: {}", script_pubkey.to_hex_string());

    // 1) key-path：只有当这个 UTXO 是 wallet 的 key-path 地址时才成立（没有 script tree）。
    if prevouts[0].script_pubkey == script_pubkey {
        let sighash = SighashCache::new(&mut tx)
            .taproot_key_spend_signature_hash(0, &Prevouts::All(&prevouts), TapSighashType::Default)
            .unwrap();
        let sig = taproot_wallet.sign_keypath(
            secp,
            &bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref()).unwrap(),
        );
        tx.input[0].witness.push(sig.as_ref().to_vec());

        match verify_taproot_input_signature(secp, &tx, 0, &prevouts) {
            Ok(true) => println!("✅ ok: key-path spend (offline)"),
            Ok(false) => println!("❌ verify failed: spend failed"),
            Err(e) => println!("❌ verify failed: {}", e),
        }
        return;
    } else {
        println!("❌ verify failed: script pubkey mismatch");
        return;
    }
}
