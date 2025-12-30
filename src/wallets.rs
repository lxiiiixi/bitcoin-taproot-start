use bitcoin::{
    Address, Network, PrivateKey, PublicKey,
    key::{Keypair, Secp256k1, TapTweak, TweakedKeypair, TweakedPublicKey, rand},
    taproot::TaprootBuilder,
};
use bitcoin_address_generator::{derive_bitcoin_addresses, generate_mnemonic};

use crate::env_config::ENV_CONFIGS;

// https://rust-bitcoin.org/book/tx_taproot.html

/// 创建 Taproot 钱包
// pub fn create_taproot_wallet(
//     secp: &Secp256k1<bitcoin::secp256k1::All>,
// ) -> Result<(PrivateKey, Address, TweakedKeypair), Box<dyn std::error::Error>> {
//     // 生成一个随机的 256 位（32 字节）的私钥
//     let secret_key = bitcoin::secp256k1::SecretKey::new(&mut rand::thread_rng());
//     // 将私钥转换为 bitcoin 库的 PrivateKey 对象
//     let private_key = PrivateKey::new(secret_key, Network::Testnet);

//     // 获取公钥
//     let public_key = PublicKey::new(secret_key.public_key(secp));

//     // 创建空的 Taproot Builder (没有脚本树，只使用 Keypath Spend)
//     // 这是最简单的 Taproot 形式：直接使用密钥签名
//     let builder = TaprootBuilder::new();
//     let secp_public_key = secret_key.public_key(secp);
//     let xonly_pk = secp_public_key.x_only_public_key().0;
//     let taproot_spend_info = builder.finalize(secp, xonly_pk).unwrap();

//     // 创建 Tweaked Keypair (聚合后的密钥对)
//     let tweak = taproot_spend_info.tap_tweak(); // 提取脚本树根哈希，用于密钥聚合
//     let tweaked_keypair = Keypair::from_secret_key(secp, &secret_key).tap_tweak(secp, Some());

//     // 创建 Taproot 地址
//     let taproot_pk: TweakedPublicKey = tweaked_keypair.x_only_public_key();
//     let address = Address::p2tr(secp, taproot_pk.to_x_only_pub(), None, Network::Testnet);

//     Ok((private_key, address, tweaked_keypair))
// }

pub fn create_taproot_wallet() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Generate a default 12-word mnemonic in English
    // let mnemonic = generate_mnemonic(None, None).unwrap();
    let mnemonic = &ENV_CONFIGS.mnemonic;
    println!("Generated mnemonic: {}", mnemonic);

    // honey hundred air thumb claim action situate upgrade cry amazing type trust

    let p2tr_addresses = derive_bitcoin_addresses(
        &mnemonic,
        Some("m/86'/1'/0'"), // testnet
        Some(Network::Testnet),
        None,
        Some(false), // Change addresses (false = receiving, true = change)
        Some(0),     // Start index
        Some(2),     // Number of addresses to generate
    )
    .unwrap();

    println!("\n ✓ Taproot addresses:");
    for addr in p2tr_addresses.addresses.iter() {
        println!("  📍 {} (path: {})", addr.address, addr.path);
    }

    Ok(p2tr_addresses
        .addresses
        .iter()
        .map(|addr| addr.address.clone().to_string())
        .collect())
}
