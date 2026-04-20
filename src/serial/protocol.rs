use anyhow::{anyhow, Result};

use super::DisplayMode;

pub fn build_port_payload(mode: DisplayMode, input: &str) -> Result<Vec<u8>> {
    match mode {
        DisplayMode::Ascii => Ok(input.as_bytes().to_vec()),
        DisplayMode::Hex => parse_hex_input(input),
    }
}

pub fn parse_hex_input(input: &str) -> Result<Vec<u8>> {
    let compact = input.split_whitespace().collect::<String>();
    if compact.is_empty() {
        return Ok(Vec::new());
    }
    if compact.len() % 2 != 0 {
        return Err(anyhow!("HEX 输入长度必须是偶数"));
    }
    hex::decode(&compact).map_err(|err| anyhow!("HEX 输入格式错误: {err}"))
}

pub fn bytes_to_ascii_display(bytes: &[u8]) -> String {
    let normalized = normalize_for_display(bytes);
    String::from_utf8_lossy(&normalized).into_owned()
}

fn normalize_for_display(bytes: &[u8]) -> Vec<u8> {
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                if bytes.get(index + 1) == Some(&b'\n') {
                    normalized.push(b'\n');
                    index += 2;
                } else {
                    index += 1;
                }
            }
            0x08 | 0x0B | 0x0C => {
                index += 1;
            }
            value => {
                normalized.push(value);
                index += 1;
            }
        }
    }

    normalized
}

pub fn bytes_to_hex_display(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
