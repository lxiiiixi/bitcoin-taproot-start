mod alchemy_client;
mod env_config;
mod transactions;
mod txs;
mod utils;
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

    let alchemy = AlchemyClient::new(&ENV_CONFIGS.alchemy_api_url);

    let secp = Secp256k1::<bitcoin::secp256k1::All>::new();
    let (private_key, address, tweaked_keypair) = create_taproot_wallet(&secp).unwrap();
    println!("  📍 Taproot 地址: {}", address);
    println!("  📍 Private Key: {}", private_key.to_wif());
    println!("  📍 Tweaked Keypair: {:?}", tweaked_keypair);

    txs::tx_brc20_deploy(&alchemy, &secp, &address, &tweaked_keypair).await;
}
