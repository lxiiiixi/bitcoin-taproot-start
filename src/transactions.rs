use bitcoin::key::{Secp256k1, TweakedKeypair};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::transaction::Version;
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid,
    Witness,
};

use crate::alchemy_client::TxOut as AlchemyTxOut;

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

// /// 创建包含 BRC20 数据的交易
// ///
// /// BRC20 Inscription 格式：
// /// Output Script: OP_1 <public_key>
// /// Witness: <signature> OP_IF <content_type> <data> OP_0 OP_ENDIF
// fn create_brc20_transaction(
//     secp: &Secp256k1<bitcoin::secp256k1::All>,
//     wallet: &TaprootWallet,
//     utxo: &UtxoInfo,
//     data: &str,
// ) -> Result<Transaction, Box<dyn std::error::Error>> {
//     println!("  构造输入...");

//     // ===== 构造输入 =====
//     let outpoint = OutPoint {
//         txid: Txid::from_str(&utxo.txid)?,
//         vout: utxo.vout as u32,
//     };

//     let input = TxIn {
//         previous_output: outpoint,
//         script_sig: ScriptBuf::new(),
//         sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
//         witness: bitcoin::Witness::new(),
//     };

//     println!("  构造输出...");

//     // ===== 构造输出 =====
//     // 手续费配置
//     let base_fee = 1000; // 基础费用
//     let data_size = data.len() as u64;
//     let size_fee = data_size * 10; // 每字节 10 satoshis
//     let total_fee = base_fee + size_fee;

//     println!("    基础费用: {} sats", base_fee);
//     println!("    数据大小: {} bytes", data_size);
//     println!("    数据费用: {} sats", size_fee);
//     println!("    总费用: {} sats", total_fee);

//     let output_value = utxo.value.saturating_sub(total_fee);

//     if output_value < 546 {
//         return Err("余额不足，无法支付交易费用".into());
//     }

//     println!("    输出金额: {} sats\n", output_value);

//     // 输出脚本（标准 P2TR）
//     let output = TxOut {
//         value: output_value,
//         script_pubkey: wallet.address.script_pubkey(),
//     };

//     // ===== 创建交易 =====
//     let mut tx = Transaction {
//         version: bitcoin::transaction::Version::TWO,
//         lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
//         input: vec![input],
//         output: vec![output],
//     };

//     println!("  计算签名...");

//     // ===== 签名 =====
//     sign_taproot_transaction(secp, &mut tx, utxo.value, wallet)?;

//     println!("  签名完成\n");

//     Ok(tx)
// }

// // 对 Taproot 交易进行签名
// fn sign_taproot_transaction(
//     secp: &Secp256k1<bitcoin::secp256k1::All>,
//     tx: &mut Transaction,
//     utxo_value: u64,
//     wallet: &TaprootWallet,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     use bitcoin::sighash::{SighashCache, TapSighashType};

//     // 创建 Sighash 缓存
//     let mut sighash_cache = SighashCache::new(tx);

//     // 获取上一个输出的信息
//     let prevout = TxOut {
//         value: utxo_value,
//         script_pubkey: wallet.address.script_pubkey(),
//     };

//     // 计算 Taproot Keypath Sighash
//     let sighash = sighash_cache.taproot_key_spend_signature_hash(
//         0,
//         &bitcoin::sighash::Prevouts::All(&vec![prevout]),
//         TapSighashType::Default,
//     )?;

//     // 创建消息并签名
//     let message = bitcoin::secp256k1::Message::from_digest(sighash.to_byte_array());
//     let schnorr_sig = secp.sign_schnorr(&message, &wallet.keypair);

//     // 填充 witness
//     tx.input[0].witness.push(schnorr_sig.as_ref());

//     Ok(())
// }
