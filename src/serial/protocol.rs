use std::fmt::Write;

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
    sanitize_display_text(&String::from_utf8_lossy(&normalized))
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

fn sanitize_display_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' | '\t' => sanitized.push(ch),
            ch if ch.is_control() => push_control_escape(&mut sanitized, ch),
            ch => sanitized.push(ch),
        }
    }
    sanitized
}

fn push_control_escape(output: &mut String, ch: char) {
    let code = ch as u32;
    if code <= 0xFF {
        let _ = write!(output, "\\x{code:02X}");
    } else {
        let _ = write!(output, "\\u{{{code:04X}}}");
    }
}

pub fn bytes_to_hex_display(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::bytes_to_ascii_display;

    #[test]
    fn ascii_display_escapes_clipboard_hostile_control_bytes() {
        assert_eq!(
            bytes_to_ascii_display(b"ok\0\x1B[31m\x7F"),
            "ok\\x00\\x1B[31m\\x7F"
        );
    }

    #[test]
    fn ascii_display_preserves_valid_utf8_and_replaces_invalid_bytes() {
        let mut bytes = "温度".as_bytes().to_vec();
        bytes.extend_from_slice(&[0xFF, b'A']);

        assert_eq!(bytes_to_ascii_display(&bytes), "温度\u{FFFD}A");
    }

    #[test]
    fn ascii_display_keeps_existing_line_and_tab_normalization() {
        assert_eq!(bytes_to_ascii_display(b"a\r\nb\tc\rd\x08e"), "a\nb\tcde");
    }
}
