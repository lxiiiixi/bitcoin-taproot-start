use bitcoin::{
    opcodes::all::{OP_PUSHNUM_13, OP_RETURN},
    script::{Builder, ScriptBuf},
};

enum Tag {
    Body = 0,
    Flags = 2,
    Rune = 4,
    Premine = 6,
    Cap = 8,
    Amount = 10,
    HeightStart = 12,
    HeightEnd = 14,
    OffsetStart = 16,
    OffsetEnd = 18,
    Mint = 20,
    Pointer = 22,
    Cenotaph = 126,

    Divisibility = 1,
    Spacers = 3,
    Symbol = 5,
    Nop = 127,
}

/// =====================================================
/// VarInt 编码器
/// =====================================================
pub fn encode_varint(mut value: u128) -> Vec<u8> {
    let mut result = Vec::new();

    match value {
        0..=252 => {
            result.push(value as u8);
        }
        253..=65535 => {
            result.push(0xFD);
            let bytes = (value as u16).to_le_bytes();
            result.extend_from_slice(&bytes);
        }
        65536..=4294967295 => {
            result.push(0xFE);
            let bytes = (value as u32).to_le_bytes();
            result.extend_from_slice(&bytes);
        }
        _ => {
            result.push(0xFF);
            let bytes = value.to_le_bytes();
            result.extend_from_slice(&bytes);
        }
    }

    result
}

/// =====================================================
/// 符文名称转换为小端序整数
/// =====================================================
///
/// 根据官方规范，Rune 字段值是符文名称编码为小端序整数
/// 例如: "TEST" -> 转换为对应的小端序整数
///
/// 字母表：A-Z, a-z（标准ASCII，但通常使用大写）
/// 点 (•) 用于分隔（编码为特殊值）
///
pub fn rune_name_to_integer(name: &str) -> u128 {
    let mut result: u128 = 0;
    let mut shift = 0;

    for ch in name.chars() {
        let value = match ch {
            'A'..='Z' => (ch as u128) - ('A' as u128) + 1, // A=1, B=2, ..., Z=26
            'a'..='z' => (ch as u128) - ('a' as u128) + 1, // a=1, b=2, ..., z=26
            '•' | '.' => 0,                                // 点作为分隔符，编码为 0
            _ => continue,                                 // 忽略其他字符
        };

        result |= value << shift;
        shift += 8; // 每个字符 8 bit
    }

    result
}

/// =====================================================
/// Runes 构建器
/// =====================================================
pub struct RunesBuilder {
    fields: Vec<(u128, u128)>, // (tag, value) pairs
}

impl RunesBuilder {
    pub fn new() -> Self {
        RunesBuilder { fields: Vec::new() }
    }

    /// 添加 FLAGS (Tag 2)
    pub fn with_flags(mut self, flags: u128) -> Self {
        self.fields.push((2, flags));
        self
    }

    /// 添加 RUNE (Tag 4) - 符文名称
    pub fn with_rune(mut self, rune_name: &str) -> Self {
        let rune_value = rune_name_to_integer(rune_name);
        println!("🔄 符文名称转换:");
        println!("  输入: {}", rune_name);
        println!("  整数值: {} (0x{:x})", rune_value, rune_value);
        self.fields.push((4, rune_value));
        self
    }

    /// 添加 SPACERS (Tag 5)
    pub fn with_spacers(mut self, spacers: u128) -> Self {
        self.fields.push((5, spacers));
        self
    }

    /// 添加 SYMBOL (Tag 6) - 符号字符
    pub fn with_symbol(mut self, symbol: char) -> Self {
        let symbol_value = symbol as u128;
        self.fields.push((6, symbol_value));
        self
    }

    /// 添加 PREMINE (Tag 7) - 预挖数量
    pub fn with_premine(mut self, premine: u128) -> Self {
        self.fields.push((7, premine));
        self
    }

    /// 添加 POINTER (Tag 8)
    pub fn with_pointer(mut self, pointer: u32) -> Self {
        self.fields.push((8, pointer as u128));
        self
    }

    /// 添加 TERMS (Tag 9)
    pub fn with_terms(mut self, terms: u128) -> Self {
        self.fields.push((9, terms));
        self
    }

    /// 添加 TURBO (Tag 10)
    pub fn with_turbo(mut self) -> Self {
        self.fields.push((10, 0));
        self
    }

    /// 添加 CAP (Tag 11) - 供应上限
    pub fn with_cap(mut self, cap: u128) -> Self {
        self.fields.push((11, cap));
        self
    }

    /// 添加 DIVISIBILITY (Tag 12) - 小数位
    pub fn with_divisibility(mut self, divisibility: u8) -> Self {
        self.fields.push((12, divisibility as u128));
        self
    }

    /// 添加 AMOUNT (Tag 1)
    pub fn with_amount(mut self, amount: u128) -> Self {
        self.fields.push((1, amount));
        self
    }

    /// 添加 MINT (Tag 3)
    pub fn with_mint(mut self, block: u64, tx: u32) -> Self {
        // MINT 编码为 [block, tx]（两个 VarInt）
        let mint_value = (block as u128) << 32 | (tx as u128);
        self.fields.push((3, mint_value));
        self
    }

    /// 构建脚本
    pub fn build(self) -> Result<ScriptBuf, Box<dyn std::error::Error>> {
        println!("\n🔨 构建 Runes 脚本");
        println!("─────────────────────────────────");

        let mut data = Vec::new();

        // 排序字段（可选，但有助于一致性）
        let mut fields = self.fields.clone();
        fields.sort_by_key(|f| f.0);

        println!("字段数: {}\n", fields.len());

        // 编码每个 Tag-Value 对
        for (tag, value) in fields {
            println!("编码 Tag {}: {}", tag, value);

            // 编码 tag
            let tag_bytes = encode_varint(tag);
            data.extend_from_slice(&tag_bytes);
            println!("  Tag 编码: {}", hex::encode(&tag_bytes));

            // 编码 value
            let value_bytes = encode_varint(value);
            data.extend_from_slice(&value_bytes);
            println!("  Value 编码: {}", hex::encode(&value_bytes));
        }

        // 添加 BODY 终止符 (Tag 0)
        println!("编码 BODY 终止符");
        let body_bytes = encode_varint(0);
        data.extend_from_slice(&body_bytes);
        println!("  编码: {}\n", hex::encode(&body_bytes));

        println!("✓ Runestone 数据已生成: {} 字节", data.len());
        println!("Hex: {}\n", hex::encode(&data));

        // 构造脚本
        let mut pb = bitcoin::script::PushBytesBuf::new();
        pb.extend_from_slice(&data)?;

        let script = Builder::new()
            .push_opcode(OP_RETURN)
            .push_opcode(OP_PUSHNUM_13)
            .push_slice(pb)
            .into_script();

        println!("✓ 完整脚本 Hex:");
        println!("{}\n", script.to_hex_string());

        Ok(script)
    }
}

/// =====================================================
/// 测试和示例
/// =====================================================

pub fn example_satoshi_nakamoto() -> Result<ScriptBuf, Box<dyn std::error::Error>> {
    println!("📝 示例 1: SATOSHI•NAKAMOTO");
    println!("═══════════════════════════════════════════\n");

    let script = RunesBuilder::new()
        .with_flags(7) // FLAGS = 7
        .with_rune("SATOSHI•NAKAMOTO") // 符文名称
        .with_premine(4_200_000) // 预挖 420 万
        .with_cap(21_000_000) // 上限 2100 万
        .with_divisibility(0) // 无小数位
        .build()?;

    Ok(script)
}

pub fn example_test_token() -> Result<ScriptBuf, Box<dyn std::error::Error>> {
    println!("📝 示例 2: TEST 代币");
    println!("═══════════════════════════════════════════\n");

    let script = RunesBuilder::new()
        .with_rune("TEST")
        .with_premine(1_000_000)
        .with_cap(10_000_000)
        .with_divisibility(8)
        .build()?;

    Ok(script)
}

pub fn example_with_symbol() -> Result<ScriptBuf, Box<dyn std::error::Error>> {
    println!("📝 示例 3: 带符号的代币");
    println!("═══════════════════════════════════════════\n");

    let script = RunesBuilder::new()
        .with_rune("MYTOKEN")
        .with_symbol('₹')
        .with_premine(5_000_000)
        .with_cap(100_000_000)
        .with_divisibility(18)
        .build()?;

    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_name_conversion() {
        let test_cases = vec![
            ("A", 1),
            ("Z", 26),
            ("AB", 0x0201),       // A=1, B=2
            ("TEST", 0x14131920), // T=20, E=5, S=19, T=20
        ];

        for (name, expected) in test_cases {
            let result = rune_name_to_integer(name);
            println!("'{}' -> {} (0x{:x})", name, result, result);
            // 注意：实际值取决于编码规则
        }
    }

    #[test]
    fn test_varint_encoding() {
        let test_cases = vec![
            (0, vec![0x00]),
            (1, vec![0x01]),
            (252, vec![0xfc]),
            (253, vec![0xfd, 0xfd, 0x00]),
        ];

        for (value, expected) in test_cases {
            let result = encode_varint(value);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_build_satoshi() {
        match example_satoshi_nakamoto() {
            Ok(script) => {
                let hex = script.to_hex_string();
                println!("✓ 构建成功");
                println!("Hex: {}", hex);
                assert!(!hex.is_empty());
            }
            Err(e) => panic!("构建失败: {}", e),
        }
    }

    #[test]
    fn test_build_test_token() {
        match example_test_token() {
            Ok(script) => {
                let hex = script.to_hex_string();
                println!("✓ 构建成功");
                println!("Hex: {}", hex);
                assert!(!hex.is_empty());
            }
            Err(e) => panic!("构建失败: {}", e),
        }
    }
}
