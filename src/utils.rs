use bitcoin::ScriptBuf;
use bitcoin::opcodes::OP_FALSE;
use bitcoin::opcodes::all::{OP_ENDIF, OP_IF};
use bitcoin::script::{Builder, PushBytesBuf};
use serde_json::json;

pub fn build_inscription_script() -> ScriptBuf {
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

    // push_slice 要求实现 PushBytes 特征（不能超过 2^32 字节）
    Builder::new()
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
