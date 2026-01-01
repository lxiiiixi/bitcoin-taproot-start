use crate::alchemy_client::AlchemyClient;
use crate::transactions::{create_brc20_transaction, create_commit_tx, create_first_tx};
use crate::wallets::TaprootWallet;
use bitcoin::key::{Keypair, Secp256k1, TweakedKeypair};
use bitcoin::script::Builder;
use bitcoin::{Address, Network};
use serde_json::json;

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
