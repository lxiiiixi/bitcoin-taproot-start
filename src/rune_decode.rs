use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct RuneId {
    pub block: u64,
    pub tx: u32,
}

#[derive(Debug, Clone)]
pub struct Edict {
    pub id: RuneId,
    pub amount: u128,
    pub output: u32,
}

#[derive(Debug, Clone)]
pub struct Terms {
    pub amount: Option<u128>,
    pub cap: Option<u128>,
    pub height: (Option<u64>, Option<u64>),
    pub offset: (Option<u64>, Option<u64>),
}

#[derive(Debug, Clone)]
pub struct Etching {
    pub divisibility: Option<u8>,
    pub premine: Option<u128>,
    pub rune: Option<u128>, // Rune name (base-26 encoded)
    pub spacers: Option<u32>,
    pub symbol: Option<char>,
    pub terms: Option<Terms>,
}

#[derive(Debug, Clone)]
pub struct Runestone {
    pub edicts: Vec<Edict>,
    pub etching: Option<Etching>,
    pub mint: Option<RuneId>,
    pub pointer: Option<u32>,
}

#[derive(Debug)]
pub struct Message {
    pub fields: BTreeMap<u128, Vec<u128>>,
    pub edicts: Vec<Edict>,
}

pub fn decode_leb128(mut bytes: &[u8]) -> Result<Vec<u128>, String> {
    let mut values = Vec::new();

    while !bytes.is_empty() {
        let mut value: u128 = 0;
        let mut shift = 0;
        let mut consumed = 0;

        for b in bytes {
            consumed += 1;
            let low = (b & 0x7f) as u128;
            value |= low << shift;

            if b & 0x80 == 0 {
                values.push(value);
                bytes = &bytes[consumed..];
                break;
            }

            shift += 7;
            if shift > 127 {
                return Err("LEB128 overflow".into());
            }
            if consumed > 18 {
                return Err("LEB128 too long".into());
            }
        }

        if consumed == bytes.len() {
            return Err("Truncated LEB128".into());
        }
    }

    Ok(values)
}

pub fn parse_message(values: &[u128]) -> Result<Message, String> {
    let mut fields: BTreeMap<u128, Vec<u128>> = BTreeMap::new();
    let mut edicts = Vec::new();

    let mut i = 0;
    let mut base_block: u64 = 0;
    let mut base_tx: u32 = 0;

    while i < values.len() {
        let tag = values[i];
        i += 1;

        if tag == 0 {
            // edicts
            while i + 3 < values.len() {
                let block_delta = values[i];
                let tx_or_delta = values[i + 1];
                let amount = values[i + 2];
                let output = values[i + 3];

                i += 4;

                let (block, tx) = if block_delta == 0 {
                    (base_block, base_tx + tx_or_delta as u32)
                } else {
                    (base_block + block_delta as u64, tx_or_delta as u32)
                };

                if block == 0 && tx != 0 {
                    return Err("Invalid RuneId".into());
                }

                base_block = block;
                base_tx = tx;

                edicts.push(Edict {
                    id: RuneId { block, tx },
                    amount,
                    output: output as u32,
                });
            }
            break;
        }

        if i >= values.len() {
            return Err("Tag without value".into());
        }

        let value = values[i];
        i += 1;

        fields.entry(tag).or_default().push(value);
    }

    Ok(Message { fields, edicts })
}

// Tag enum（你给的那一套）
const TAG_DIVISIBILITY: u128 = 1;
const TAG_FLAGS: u128 = 2;
const TAG_RUNE: u128 = 4;
const TAG_PREMINE: u128 = 6;
const TAG_CAP: u128 = 8;
const TAG_AMOUNT: u128 = 10;
const TAG_HEIGHT_START: u128 = 12;
const TAG_HEIGHT_END: u128 = 14;
const TAG_OFFSET_START: u128 = 16;
const TAG_OFFSET_END: u128 = 18;
const TAG_MINT: u128 = 20;
const TAG_POINTER: u128 = 22;

pub fn parse_runestone(msg: Message) -> Result<Runestone, String> {
    let mut etching: Option<Etching> = None;
    let mut mint: Option<RuneId> = None;
    let mut pointer: Option<u32> = None;

    let fields = msg.fields;

    if fields.contains_key(&TAG_RUNE) || fields.contains_key(&TAG_PREMINE) {
        let mut terms = None;

        if fields.contains_key(&TAG_AMOUNT)
            || fields.contains_key(&TAG_CAP)
            || fields.contains_key(&TAG_HEIGHT_START)
            || fields.contains_key(&TAG_HEIGHT_END)
            || fields.contains_key(&TAG_OFFSET_START)
            || fields.contains_key(&TAG_OFFSET_END)
        {
            terms = Some(Terms {
                amount: fields.get(&TAG_AMOUNT).and_then(|v| v.first().cloned()),
                cap: fields.get(&TAG_CAP).and_then(|v| v.first().cloned()),
                height: (
                    fields
                        .get(&TAG_HEIGHT_START)
                        .and_then(|v| v.first().map(|x| *x as u64)),
                    fields
                        .get(&TAG_HEIGHT_END)
                        .and_then(|v| v.first().map(|x| *x as u64)),
                ),
                offset: (
                    fields
                        .get(&TAG_OFFSET_START)
                        .and_then(|v| v.first().map(|x| *x as u64)),
                    fields
                        .get(&TAG_OFFSET_END)
                        .and_then(|v| v.first().map(|x| *x as u64)),
                ),
            });
        }

        etching = Some(Etching {
            divisibility: fields
                .get(&TAG_DIVISIBILITY)
                .and_then(|v| v.first().map(|x| *x as u8)),
            premine: fields.get(&TAG_PREMINE).and_then(|v| v.first().cloned()),
            rune: fields.get(&TAG_RUNE).and_then(|v| v.first().cloned()),
            spacers: None,
            symbol: None,
            terms,
        });
    }

    if let Some(v) = fields.get(&TAG_MINT).and_then(|v| v.first()) {
        let block = (v >> 32) as u64;
        let tx = (*v & 0xffff_ffff) as u32;
        mint = Some(RuneId { block, tx });
    }

    if let Some(v) = fields.get(&TAG_POINTER).and_then(|v| v.first()) {
        pointer = Some(*v as u32);
    }

    Ok(Runestone {
        edicts: msg.edicts,
        etching,
        mint,
        pointer,
    })
}

pub fn rune_u128_to_name(mut n: u128) -> String {
    let mut chars = Vec::new();

    loop {
        let rem = (n % 26) as u8;
        chars.push((b'A' + rem) as char);

        n /= 26;
        if n == 0 {
            break;
        }
    }

    chars.iter().rev().collect()
}
