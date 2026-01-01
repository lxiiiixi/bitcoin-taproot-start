mod alchemy_client;
mod env_config;
mod transactions;
mod txs;
mod utils;
mod wallets;

use bitcoin::key::Secp256k1;

use crate::{
    alchemy_client::AlchemyClient, transactions::create_commit_tx, utils::build_inscription_script,
};
use env_config::ENV_CONFIGS;
use wallets::create_taproot_wallet;

#[tokio::main]
async fn main() {
    let alchemy = AlchemyClient::new(&ENV_CONFIGS.alchemy_api_url);

    let secp = Secp256k1::<bitcoin::secp256k1::All>::new();
    let (private_key, secret_keypair, address, tweaked_keypair) =
        create_taproot_wallet(&secp).unwrap();
    println!("  📍 Taproot 地址: {}", address);
    println!("  📍 Private Key: {}", private_key.to_wif());
    println!("  📍 Tweaked Keypair: {:?}", tweaked_keypair);

    let txid1 = "aaeb4cde567a87b332bbc9bf983e1059abea623470a40aff43d886493a32067c";
    let txid2 = "ec2a26543197c61dfebed3c05f95c78d30b500cf260e7a0ee8697e42505f0ba0";
    let txid3 = "b1a49c7d0b2ce71a606c3cc2d74f0feac9b749d0d4aa1e4ce7659f7e682b45eb";

    // txs::tx_brc20_deploy(&alchemy, &secp, &tweaked_keypair, txid2, 0).await;
    txs::tx_inscription_commit(&alchemy, &secp, &secret_keypair, &tweaked_keypair, txid2, 1).await;
}

// https://mempool.space/zh/testnet/tx/b1a49c7d0b2ce71a606c3cc2d74f0feac9b749d0d4aa1e4ce7659f7e682b45eb 格式不规范版本
