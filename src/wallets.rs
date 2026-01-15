use bip39::{Language, Mnemonic};
use bitcoin::{
    Address, Network, PrivateKey, XOnlyPublicKey,
    bip32::{DerivationPath, Xpriv},
    key::{Keypair, Secp256k1, TapTweak, TweakedKeypair},
    taproot::TaprootSpendInfo,
};

use crate::env_config::ENV_CONFIGS;

pub struct TaprootWallet {
    /// Taproot internal key（root identity）
    internal_keypair: Keypair,

    /// Taproot output key（用于签名）
    tweaked_keypair: TweakedKeypair,

    /// Internal x-only pubkey（构造地址 / script tree）
    internal_xonly: XOnlyPublicKey,

    /// 默认 key-path 地址（无 script tree）
    /// 用于接受转账等
    internal_address: Address,
    // Tweaked key-path 地址（有 script tree）
    // tweaked_address: Address,
}

// https://rust-bitcoin.org/book/tx_taproot.html

/// 创建 Taproot 钱包
/// 创建 Taproot 钱包（BIP86, testnet: m/86'/1'/0'/0/0）
pub fn create_taproot_wallet(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
) -> Result<TaprootWallet, Box<dyn std::error::Error>> {
    // 1️⃣ 解析 mnemonic（bip39 v2 正确方式）
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &ENV_CONFIGS.mnemonic)?;

    // 2️⃣ mnemonic -> seed bytes (64 bytes)
    // passphrase 为空字符串
    let seed = mnemonic.to_seed_normalized("");

    // 3️⃣ seed -> master xprv (bitcoin::bip32)
    let master_xprv = Xpriv::new_master(Network::Testnet, &seed)?;

    // 4️⃣ BIP86 路径
    let path: DerivationPath = "m/86'/1'/0'/0/0".parse()?;
    // let path: DerivationPath = "m/86'/1'/0'/0/1".parse()?;
    let child_xprv = master_xprv.derive_priv(secp, &path)?;

    // 5️⃣ bitcoin 中 private_key 就是 secp256k1::SecretKey
    let secret_key = child_xprv.private_key;

    // 6️⃣ SecretKey -> Keypair（internal key）
    // 主要作用是：派生 Taproot 地址、构造 script tree、生成 tweaked key，作为钱包主身份
    // 一般不用来：直接签名，
    let internal_keypair = Keypair::from_secret_key(secp, &secret_key);

    // 8️⃣ Taproot 地址（使用 internal key）
    let (internal_xonly, _) = internal_keypair.x_only_public_key();
    println!("  📍 Internal XOnly: {}", internal_xonly.to_string());
    let internal_address = Address::p2tr(secp, internal_xonly, None, Network::Testnet);
    // let address: Address = Address::p2tr(
    //     secp,
    //     tweaked_keypair.to_keypair().x_only_public_key().0,
    //     None,
    //     Network::Testnet,
    // );

    // 7️⃣ Taproot key-path tweak（无 script tree）
    // 这里的 None 表示没有 script tree，只有 internal key
    let tweaked_keypair: TweakedKeypair = internal_keypair.tap_tweak(secp, None);

    let tweaked_address = Address::p2tr(
        secp,
        tweaked_keypair.to_keypair().x_only_public_key().0,
        None,
        Network::Testnet,
    );

    println!(
        "  📍 Internal key address: {}",
        internal_address.to_string()
    );
    println!(
        "  📍 Tweaked key address(Never should be used): {:?} \n",
        tweaked_address
    );

    Ok(TaprootWallet {
        internal_xonly,
        tweaked_keypair,
        internal_keypair,
        internal_address,
    })
}

impl TaprootWallet {
    /// 用于所有 key-path 签名
    pub fn sign_keypath(
        &self,
        secp: &Secp256k1<bitcoin::secp256k1::All>,
        msg: &bitcoin::secp256k1::Message,
    ) -> bitcoin::secp256k1::schnorr::Signature {
        secp.sign_schnorr(msg, &self.tweaked_keypair.to_keypair())
    }

    /// 用于 tapscript（script-path）里显式放入的 x-only pubkey 的签名。
    /// 注意：这不是 output key（tweaked key），而是脚本里用到的 internal key。
    pub fn sign_internal(
        &self,
        secp: &Secp256k1<bitcoin::secp256k1::All>,
        msg: &bitcoin::secp256k1::Message,
    ) -> bitcoin::secp256k1::schnorr::Signature {
        secp.sign_schnorr(msg, &self.internal_keypair)
    }

    pub fn get_commit_address_with_script_tree(
        &self,
        secp: &Secp256k1<bitcoin::secp256k1::All>,
        script_tree: &TaprootSpendInfo,
    ) -> Address {
        Address::p2tr(
            secp,
            self.internal_xonly(),
            script_tree.merkle_root(),
            Network::Testnet,
        )
    }

    pub fn get_internal_address(&self) -> Address {
        self.internal_address.clone()
    }

    /// 用于构造 script tree
    pub fn internal_xonly(&self) -> bitcoin::secp256k1::XOnlyPublicKey {
        self.internal_xonly
    }
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
