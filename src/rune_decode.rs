/// =====================================================
/// 正确的 Runes 解析器（修复版）
/// =====================================================
///
/// 关键理解：
/// 1. OP_PUSHNUM_13 后面跟着一个 push operation
/// 2. push operation 的第一字节表示要推送多少字节
/// 3. 之后才是实际的 Runestone 数据
///
use std::collections::HashMap;

/// =====================================================
/// VarInt 解码器
/// =====================================================
pub struct VarIntDecoder {
    data: Vec<u8>,
    pos: usize,
}

impl VarIntDecoder {
    pub fn new(data: Vec<u8>) -> Self {
        VarIntDecoder { data, pos: 0 }
    }

    /// 解码单个 VarInt
    pub fn decode_varint(&mut self) -> Result<u128, String> {
        if self.pos >= self.data.len() {
            return Err("超过数据长度".to_string());
        }

        let byte = self.data[self.pos];
        self.pos += 1;

        match byte {
            // 0-252: 直接值
            0..=252 => Ok(byte as u128),
            // 0xFD: 下 2 字节小端序
            0xFD => {
                if self.pos + 1 >= self.data.len() {
                    return Err("VarInt 数据不足 (0xFD)".to_string());
                }
                let bytes = [self.data[self.pos], self.data[self.pos + 1]];
                self.pos += 2;
                Ok(u16::from_le_bytes(bytes) as u128)
            }
            // 0xFE: 下 4 字节小端序
            0xFE => {
                if self.pos + 3 >= self.data.len() {
                    return Err("VarInt 数据不足 (0xFE)".to_string());
                }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
                self.pos += 4;
                Ok(u32::from_le_bytes(bytes) as u128)
            }
            // 0xFF: 下 8 字节小端序
            0xFF => {
                if self.pos + 7 >= self.data.len() {
                    return Err("VarInt 数据不足 (0xFF)".to_string());
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
                self.pos += 8;
                Ok(u64::from_le_bytes(bytes) as u128)
            }
        }
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    pub fn position(&self) -> usize {
        self.pos
    }
}

/// =====================================================
/// Runes 数据结构
/// =====================================================
#[derive(Debug, Clone)]
pub struct Runestone {
    pub fields: HashMap<u128, u128>,
}

/// =====================================================
/// Runes 解析器（官方规范）
/// =====================================================
pub struct RunesParser;

// 标签定义
const BODY: u128 = 0;
const FLAGS: u128 = 2;
const RUNE: u128 = 4;
const SPACERS: u128 = 5;
const SYMBOL: u128 = 6;
const PREMINE: u128 = 7;
const AMOUNT: u128 = 1;
const CAP: u128 = 11;
const MINT: u128 = 3;
const POINTER: u128 = 8;
const DIVISIBILITY: u128 = 12;
const TERMS: u128 = 9;
const TURBO: u128 = 10;

impl RunesParser {
    /// 从脚本 hex 解析
    pub fn parse_script_hex(script_hex: &str) -> Result<Option<Runestone>, String> {
        let bytes = hex::decode(script_hex).map_err(|e| format!("Hex 解码错误: {}", e))?;

        println!("📄 脚本长度: {} 字节", bytes.len());
        println!("📄 脚本 Hex: {}\n", script_hex);

        // 验证 OP_RETURN
        if bytes.is_empty() || bytes[0] != 0x6a {
            println!("❌ 不是 OP_RETURN 脚本");
            return Ok(None);
        }

        println!("✓ 字节 0: 0x6a = OP_RETURN");

        if bytes.len() < 2 {
            return Ok(None);
        }

        // 验证 OP_PUSHNUM_13
        if bytes[1] != 0x5d {
            println!("❌ 字节 1 不是 OP_PUSHNUM_13");
            return Ok(None);
        }

        println!("✓ 字节 1: 0x5d = OP_PUSHNUM_13");

        // ===== 关键修正：解析 push 操作 =====
        let mut pos = 2;
        let mut runestone_data = Vec::new();

        println!("\n📖 解析 Push 操作:");
        println!("─────────────────────────────────");

        // 读取所有 push 操作
        while pos < bytes.len() {
            let op = bytes[pos];
            pos += 1;

            println!("字节 {}: 0x{:02x}", pos - 1, op);

            match op {
                // OP_PUSHDATA1 (0x4c)
                0x4c => {
                    if pos >= bytes.len() {
                        return Err("OP_PUSHDATA1 后缺少长度字节".to_string());
                    }
                    let len = bytes[pos] as usize;
                    pos += 1;
                    println!("  OP_PUSHDATA1: push {} 字节", len);
                    if pos + len > bytes.len() {
                        return Err("推送数据不足".to_string());
                    }
                    runestone_data.extend_from_slice(&bytes[pos..pos + len]);
                    pos += len;
                }
                // OP_PUSHDATA2 (0x4d)
                0x4d => {
                    if pos + 1 >= bytes.len() {
                        return Err("OP_PUSHDATA2 后缺少长度字节".to_string());
                    }
                    let len = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]) as usize;
                    pos += 2;
                    println!("  OP_PUSHDATA2: push {} 字节", len);
                    if pos + len > bytes.len() {
                        return Err("推送数据不足".to_string());
                    }
                    runestone_data.extend_from_slice(&bytes[pos..pos + len]);
                    pos += len;
                }
                // OP_PUSHDATA4 (0x4e)
                0x4e => {
                    if pos + 3 >= bytes.len() {
                        return Err("OP_PUSHDATA4 后缺少长度字节".to_string());
                    }
                    let len = u32::from_le_bytes([
                        bytes[pos],
                        bytes[pos + 1],
                        bytes[pos + 2],
                        bytes[pos + 3],
                    ]) as usize;
                    pos += 4;
                    println!("  OP_PUSHDATA4: push {} 字节", len);
                    if pos + len > bytes.len() {
                        return Err("推送数据不足".to_string());
                    }
                    runestone_data.extend_from_slice(&bytes[pos..pos + len]);
                    pos += len;
                }
                // 1-75: 直接推送 N 字节
                1..=75 => {
                    let len = op as usize;
                    println!("  PUSH {}: push {} 字节", op, len);
                    if pos + len > bytes.len() {
                        return Err(format!(
                            "推送数据不足: 需要 {}, 有 {}",
                            len,
                            bytes.len() - pos
                        ));
                    }
                    runestone_data.extend_from_slice(&bytes[pos..pos + len]);
                    pos += len;
                }
                // 其他操作码（可能是结束或多重推送的结束）
                _ => {
                    println!("  其他操作码: 0x{:02x}, 停止解析", op);
                    break;
                }
            }
        }

        println!("\n✓ 提取的 Runestone 数据: {} 字节", runestone_data.len());
        println!("Hex: {}\n", hex::encode(&runestone_data));

        // 解析 Runestone 数据
        Self::parse_runestone_data(runestone_data)
    }

    /// 解析 Runestone 数据
    pub fn parse_runestone_data(data: Vec<u8>) -> Result<Option<Runestone>, String> {
        let mut decoder = VarIntDecoder::new(data);
        let mut fields: HashMap<u128, u128> = HashMap::new();

        println!("📖 解析 Tag-Value 对:");
        println!("─────────────────────────────────");

        let mut pair_count = 0;
        while !decoder.is_eof() {
            let tag = decoder.decode_varint()?;
            pair_count += 1;

            println!("\n对 {}:", pair_count);
            println!("  Tag: {}", Self::tag_name(tag));

            // Tag 0 = BODY，结束
            if tag == BODY {
                println!("  → 结束符");
                break;
            }

            let value = decoder.decode_varint()?;
            println!("  值: {} (0x{:x})", value, value);

            fields.insert(tag, value);
        }

        println!("\n✅ 解析完成\n");

        println!("📊 字段汇总:");
        println!("─────────────────────────────────");
        for (tag, value) in &fields {
            println!("{}: {} (0x{:x})", Self::tag_name(*tag), value, value);
        }

        let runestone = Runestone { fields };
        Ok(Some(runestone))
    }

    fn tag_name(tag: u128) -> String {
        match tag {
            0 => "BODY".to_string(),
            1 => "AMOUNT".to_string(),
            2 => "FLAGS".to_string(),
            3 => "MINT".to_string(),
            4 => "RUNE".to_string(),
            5 => "SPACERS".to_string(),
            6 => "SYMBOL".to_string(),
            7 => "PREMINE".to_string(),
            8 => "POINTER".to_string(),
            9 => "TERMS".to_string(),
            10 => "TURBO".to_string(),
            11 => "CAP".to_string(),
            12 => "DIVISIBILITY".to_string(),
            _ => format!("TAG_{}", tag),
        }
    }
}

/// =====================================================
/// 测试
/// =====================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_satoshi_nakamoto() {
        let script_hex = "6a5d28020704eadaa9ea92e0aacaaf850105b0\
                          09c010340010806080b9f6cdbf5f08c0a00a0a\
                          80c8afa025";

        match RunesParser::parse_script_hex(script_hex) {
            Ok(Some(runestone)) => {
                println!("\n✓ 解析成功");
                println!("字段数: {}", runestone.fields.len());
                for (tag, value) in &runestone.fields {
                    println!("  Tag {}: {}", tag, value);
                }
            }
            Ok(None) => println!("❌ 不是 Runestone"),
            Err(e) => panic!("❌ 解析错误: {}", e),
        }
    }

    #[test]
    fn test_varint() {
        let mut decoder = VarIntDecoder::new(vec![0x02, 0x07, 0x04]);
        assert_eq!(decoder.decode_varint().unwrap(), 2);
        assert_eq!(decoder.decode_varint().unwrap(), 7);
        assert_eq!(decoder.decode_varint().unwrap(), 4);
    }
}
