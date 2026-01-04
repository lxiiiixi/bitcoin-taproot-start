use bitcoin::opcodes::OP_FALSE;
use bitcoin::opcodes::all::{OP_CHECKSIG, OP_ENDIF, OP_IF, OP_PUSHNUM_13, OP_RETURN};
use bitcoin::script::{Builder, PushBytesBuf};
use bitcoin::{ScriptBuf, XOnlyPublicKey};
use serde::Serialize;
use serde_json::json;

use crate::runes_builder::RunesBuilder;

pub fn build_inscription_script(xonly_pubkey: XOnlyPublicKey) -> ScriptBuf {
    let brc20_data = serde_json::to_string_pretty(&json!({
        "p": "brc-20",
        "op": "deploy",
        "tick": "ordi",
        "max": "21000000",
        "lim": "1000"
    }))
    .expect("Failed to format JSON");

    // let json_bytes = brc20_json.as_bytes();
    let mut json_pb = PushBytesBuf::new();
    json_pb
        .extend_from_slice(brc20_data.as_bytes())
        .expect("Failed to push slice");

    let mut pk_pb = PushBytesBuf::new();
    pk_pb
        .extend_from_slice(&xonly_pubkey.serialize())
        .expect("Failed to push pubkey");

    // push_slice 要求实现 PushBytes 特征（不能超过 2^32 字节）
    Builder::new()
        .push_slice(pk_pb)
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_FALSE)
        .push_opcode(OP_IF)
        .push_slice(b"ord")
        .push_slice(&[1u8]) // ord version
        .push_slice(b"text/plain;charset=utf-8")
        .push_slice(&[0u8]) // separator
        .push_slice(json_pb)
        .push_opcode(OP_ENDIF)
        .into_script()
}

/// =====================================================
/// Runes 协议规范（官方）
/// =====================================================
///
/// Runes 脚本格式：
/// OP_RETURN <magic> <runestone>
///
/// magic = 0x52 (单个字节)
///
/// runestone = [edicts] [fields]
///
/// fields 使用 Tag-Value 编码：
/// <tag: varint> <value_len: varint> <value: bytes>
///
pub fn build_rune_op_return() -> ScriptBuf {
    // let mut data: Vec<u8> = Vec::new();

    // data.push(0x52); // Magic

    // // Tag 12: RUNE（符文名称）
    // data.extend_from_slice(&encode_varint(Tag::Rune as u64));
    // let rune = b"TEST";
    // data.extend_from_slice(&encode_varint(rune.len() as u64));
    // data.extend_from_slice(rune);

    // // Tag 4: DIVISIBILITY
    // data.extend_from_slice(&encode_varint(Tag::Divisibility as u64));
    // data.extend_from_slice(&encode_varint(8));

    // // Tag 3: CAP - 添加长度编码
    // data.extend_from_slice(&encode_varint(Tag::Cap as u64));
    // let cap = 1_000_000u128.to_le_bytes();
    // data.extend_from_slice(&encode_varint(16));
    // data.extend_from_slice(&cap);

    // // Tag 0: BODY - 添加这个
    // data.extend_from_slice(&encode_varint(0));

    // let mut pb = bitcoin::script::PushBytesBuf::new();
    // pb.extend_from_slice(&data).expect("Failed");

    // Builder::new()
    //     .push_opcode(OP_RETURN)
    //     .push_opcode(OP_PUSHNUM_13)
    //     .push_slice(pb)
    //     .into_script()

    let script = RunesBuilder::new()
        .with_flags(7) // FLAGS = 7
        .with_rune("TEST") // 符文名称
        .with_premine(4_200_000) // 预挖 420 万
        .with_cap(21_000_000) // 上限 2100 万
        .with_divisibility(0) // 无小数位
        .build()
        .unwrap();

    script
}
