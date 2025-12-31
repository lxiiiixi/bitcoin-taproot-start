mod alchemy_client;
mod env_config;
mod transactions;
mod wallets;

use bitcoin::key::Secp256k1;

use crate::{alchemy_client::AlchemyClient, transactions::create_commit_tx};
use env_config::ENV_CONFIGS;
use wallets::create_taproot_wallet;

#[tokio::main]
async fn main() {
    // let p2tr_addresses = create_taproot_wallet().unwrap();
    // for addr in p2tr_addresses.iter() {
    //     println!("  📍 Taproot 地址: {}", addr);
    // }

    let secp = Secp256k1::<bitcoin::secp256k1::All>::new();
    let (private_key, address, tweaked_keypair) = create_taproot_wallet(&secp).unwrap();
    println!("  📍 Taproot 地址: {}", address);
    println!("  📍 Private Key: {}", private_key.to_wif());
    println!("  📍 Tweaked Keypair: {:?}", tweaked_keypair);

    let alchemy = AlchemyClient::new(&ENV_CONFIGS.alchemy_api_url);

    if let Some(tx_out) = alchemy
        .get_tx_out(
            "048b557b5c733c9a782f954712b86df99cd0923dcb51ffcda3116f1d87e895b5",
            0,
            true,
        )
        .await
        .unwrap()
    {
        println!("UTXO value: {} BTC", tx_out.value);
        println!("Confirmations: {}", tx_out.confirmations);

        let tx = create_commit_tx(&secp, tx_out, &address, &tweaked_keypair).unwrap();
        let txid = alchemy.broadcast_tx(&tx).await.unwrap();
        println!("  📍 TXID: {}", txid);
    }

    // let brc20_data = json!({
    //     "p": "brc-20",
    //     "op": "deploy",
    //     "tick": "ordi",
    //     "max": "21000000",
    //     "lim": "1000"
    // })
    // .to_string();
    // let tx = create_brc20_transaction(&secp, &wallet, selected_utxo, &brc20_data)?;
}

// 第一笔交易 - a7bb32cdb8d77f480804e0743db3b181938a9f4745392b4f825afa5032895c2f
