mod alchemy_client;
mod env_config;
mod wallets;

use crate::alchemy_client::AlchemyClient;
use env_config::ENV_CONFIGS;
use wallets::create_taproot_wallet;

#[tokio::main]
async fn main() {
    // let p2tr_addresses = create_taproot_wallet().unwrap();
    // for addr in p2tr_addresses.iter() {
    //     println!("  📍 Taproot 地址: {}", addr);
    // }

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
    }
}
