use bip39::{Language, Mnemonic};
use bitcoin::{
    Address, Network, PrivateKey,
    bip32::{DerivationPath, Xpriv},
    key::{Keypair, Secp256k1, TapTweak, TweakedKeypair},
};

use crate::env_config::ENV_CONFIGS;

// https://rust-bitcoin.org/book/tx_taproot.html

/// 创建 Taproot 钱包
/// 创建 Taproot 钱包（BIP86, testnet: m/86'/1'/0'/0/0）
pub fn create_taproot_wallet(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
) -> Result<(PrivateKey, Address, TweakedKeypair), Box<dyn std::error::Error>> {
    // 1️⃣ 解析 mnemonic（bip39 v2 正确方式）
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &ENV_CONFIGS.mnemonic)?;

    // 2️⃣ mnemonic -> seed bytes (64 bytes)
    // passphrase 为空字符串
    let seed = mnemonic.to_seed_normalized("");

    // 3️⃣ seed -> master xprv (bitcoin::bip32)
    let master_xprv = Xpriv::new_master(Network::Testnet, &seed)?;

    // 4️⃣ BIP86 路径
    let path: DerivationPath = "m/86'/1'/0'/0/0".parse()?;
    let child_xprv = master_xprv.derive_priv(secp, &path)?;

    // 5️⃣ bitcoin 中 private_key 就是 secp256k1::SecretKey
    let secret_key = child_xprv.private_key;

    // 6️⃣ SecretKey -> Keypair
    let keypair = Keypair::from_secret_key(secp, &secret_key);

    // 7️⃣ Taproot key-path tweak（无 script tree）
    let tweaked_keypair: TweakedKeypair = keypair.tap_tweak(secp, None);

    // 8️⃣ Taproot 地址（使用 internal key）
    let (internal_xonly, _) = keypair.x_only_public_key();
    let address = Address::p2tr(secp, internal_xonly, None, Network::Testnet);

    // 9️⃣ 返回一个带 network 的 PrivateKey（方便后续）
    let private_key = PrivateKey::new(secret_key, Network::Testnet);

    Ok((private_key, address, tweaked_keypair))
}

// pub fn create_taproot_wallet() -> Result<Vec<String>, Box<dyn std::error::Error>> {
//     // Generate a default 12-word mnemonic in English
//     // let mnemonic = generate_mnemonic(None, None).unwrap();
//     let mnemonic = &ENV_CONFIGS.mnemonic;
//     println!("Generated mnemonic: {}", mnemonic);

//     let p2tr_addresses = derive_bitcoin_addresses(
//         &mnemonic,
//         Some("m/86'/1'/0'"), // testnet
//         Some(Network::Testnet),
//         None,
//         Some(false), // Change addresses (false = receiving, true = change)
//         Some(0),     // Start index
//         Some(2),     // Number of addresses to generate
//     )
//     .unwrap();

//     println!("\n ✓ Taproot addresses:");
//     for addr in p2tr_addresses.addresses.iter() {
//         println!("  📍 {} (path: {})", addr.address, addr.path);
//     }

//     Ok(p2tr_addresses
//         .addresses
//         .iter()
//         .map(|addr| addr.address.clone().to_string())
//         .collect())
// }
