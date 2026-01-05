mod alchemy_client;
mod env_config;
mod rune_decode;
mod runes_builder;
mod transactions;
mod txs;
mod utils;
mod wallets;
mod wallets_copy;

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
    let taproot_wallet = create_taproot_wallet(&secp).unwrap();

    // let txid1 = "aaeb4cde567a87b332bbc9bf983e1059abea623470a40aff43d886493a32067c";
    // let txid2 = "ec2a26543197c61dfebed3c05f95c78d30b500cf260e7a0ee8697e42505f0ba0";
    // let txid3 = "b1a49c7d0b2ce71a606c3cc2d74f0feac9b749d0d4aa1e4ce7659f7e682b45eb";

    let txid4 = "86f80251d4ff271863bf7ce7f6ce1ba2e9551110ca2d86f5cbdcfda12111df37";
    let txid5 = "43e447c5cb23868653680858a51dce44f1e08a84dbf79a29194f618c70eb3826";
    let txid6 = "bce080d10728e82a20f50e861580e6d6da9a116d493026348ad36aca981d510e";

    // txs::tx_inscription_commit(&alchemy, &secp, &taproot_wallet, txid4, 1).await;
    // txs::tx_brc20_deploy(&alchemy, &secp, &taproot_wallet, txid5, 0).await;

    txs::tx_rune_deploy(&alchemy, &secp, &taproot_wallet, txid6, 0).await;
}

// async fn main() {
//     let hex_string =
//         "020704eadaa9ea92e0aacaaf850105b09c0103400108068080b9f6cdbf5f08c0a00a0a80c8afa025";
//     let payload = hex::decode(hex_string).unwrap();

//     let values = decode_leb128(&payload).unwrap();
//     let msg = parse_message(&values).unwrap();
//     let runestone = parse_runestone(msg).unwrap();

//     println!("{:#?}", runestone);

//     let name = rune_u128_to_name(1230137034139564141930);
//     println!("{}", name);
// }
