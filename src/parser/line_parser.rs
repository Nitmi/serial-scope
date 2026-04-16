use std::collections::BTreeMap;

use crate::config::{ParserConfig, ParserMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSchema {
    Csv { channels: usize },
    KeyValue { keys: Vec<String> },
}

impl ParsedSchema {
    pub fn label(&self) -> String {
        match self {
            ParsedSchema::Csv { channels } => format!("CSV({channels} 通道)"),
            ParsedSchema::KeyValue { keys } => format!("Key=Value({})", keys.join(", ")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub schema: ParsedSchema,
    pub values: Vec<(String, f32)>,
}

#[derive(Default)]
pub struct LineAccumulator {
    buffer: Vec<u8>,
}

impl LineAccumulator {
    pub fn push_bytes(&mut self, bytes: &[u8], config: &ParserConfig) -> Vec<ParsedLine> {
        self.buffer.extend_from_slice(bytes);
        let mut results = Vec::new();

        while let Some(pos) = self.buffer.iter().position(|b| *b == b'\n') {
            let mut line = self.buffer.drain(..=pos).collect::<Vec<u8>>();
            while matches!(line.last(), Some(b'\n' | b'\r')) {
                line.pop();
            }
            if line.is_empty() {
                continue;
            }
            if let Ok(text) = String::from_utf8(line) {
                if let Some(parsed) = parse_text_line_with_config(&text, config) {
                    results.push(parsed);
                }
            }
        }

        results
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

pub fn parse_text_line_with_config(line: &str, config: &ParserConfig) -> Option<ParsedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    match config.mode {
        ParserMode::Auto => parse_key_value_line(trimmed).or_else(|| {
            if trimmed.contains(config.csv_delimiter) {
                parse_csv_numbers(trimmed, config)
            } else {
                None
            }
        }),
        ParserMode::Csv => parse_csv_numbers(trimmed, config),
        ParserMode::KeyValue => parse_key_value_line(trimmed),
    }
}

fn parse_key_value_line(line: &str) -> Option<ParsedLine> {
    let mut values = BTreeMap::new();

    for segment in line.split(',') {
        let (key, value) = segment.split_once('=')?;
        let key = key.trim();
        let value = parse_numeric_token(value)?;
        if key.is_empty() {
            return None;
        }
        values.insert(key.to_owned(), value);
    }

    if values.is_empty() {
        None
    } else {
        let keys = values.keys().cloned().collect::<Vec<_>>();
        Some(ParsedLine {
            schema: ParsedSchema::KeyValue { keys },
            values: values.into_iter().collect(),
        })
    }
}

fn parse_csv_numbers(line: &str, config: &ParserConfig) -> Option<ParsedLine> {
    let delimiter = config.csv_delimiter;
    let custom_names = config
        .csv_channel_names
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_owned())
        .collect::<Vec<_>>();

    let tokens = line
        .split(delimiter)
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let mut values = Vec::new();
    let mut started = false;

    for segment in tokens {
        let Some(value) = parse_numeric_token(segment) else {
            if started {
                return None;
            }
            continue;
        };

        started = true;
        let idx = values.len();
        let name = custom_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("ch{}", idx + 1));
        values.push((name, value));
    }

    if values.is_empty() {
        None
    } else {
        Some(ParsedLine {
            schema: ParsedSchema::Csv {
                channels: values.len(),
            },
            values,
        })
    }
}

fn parse_numeric_token(token: &str) -> Option<f32> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    let bytes = trimmed.as_bytes();
    let mut index = 0usize;

    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }

    let integer_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }

    let mut has_digit = index > integer_start;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        has_digit |= index > fraction_start;
    }

    if !has_digit {
        return None;
    }

    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }

        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }

        if index == exponent_start {
            return None;
        }
    }

    let (number_text, unit_text) = trimmed.split_at(index);
    if !valid_unit_suffix(unit_text) {
        return None;
    }

    number_text.parse::<f32>().ok()
}

fn valid_unit_suffix(unit_text: &str) -> bool {
    let trimmed = unit_text.trim();
    trimmed.is_empty() || trimmed.chars().all(is_unit_char)
}

fn is_unit_char(ch: char) -> bool {
    ch.is_alphabetic()
        || matches!(
            ch,
            '%' | '/' | '_' | '-' | '°' | 'Ω' | 'µ' | 'μ' | '*' | '.' | '(' | ')' | '[' | ']'
        )
}

#[cfg(test)]
mod tests {
    use super::{parse_text_line_with_config, ParsedSchema};
    use crate::config::{ParserConfig, ParserMode};

    fn parser_config() -> ParserConfig {
        ParserConfig {
            mode: ParserMode::Auto,
            csv_delimiter: ',',
            csv_channel_names: "a,b,c,d".to_owned(),
        }
    }

    #[test]
    fn parses_plain_csv_numbers() {
        let parsed = parse_text_line_with_config("1,2,3", &parser_config()).unwrap();
        assert_eq!(parsed.schema, ParsedSchema::Csv { channels: 3 });
        assert_eq!(
            parsed.values,
            vec![
                ("a".to_owned(), 1.0),
                ("b".to_owned(), 2.0),
                ("c".to_owned(), 3.0),
            ]
        );
    }

    #[test]
    fn parses_prefixed_csv_with_trailing_delimiter() {
        let parsed = parse_text_line_with_config("P, 1, 2, 3, 4,", &parser_config()).unwrap();
        assert_eq!(parsed.schema, ParsedSchema::Csv { channels: 4 });
        assert_eq!(parsed.values.len(), 4);
        assert_eq!(parsed.values[0].1, 1.0);
        assert_eq!(parsed.values[3].1, 4.0);
    }

    #[test]
    fn parses_csv_numbers_with_units() {
        let parsed = parse_text_line_with_config("12.3V, 200mA, -5.6°C", &parser_config()).unwrap();
        assert_eq!(parsed.schema, ParsedSchema::Csv { channels: 3 });
        assert_eq!(parsed.values[0].1, 12.3);
        assert_eq!(parsed.values[1].1, 200.0);
        assert_eq!(parsed.values[2].1, -5.6);
    }

    #[test]
    fn parses_key_value_numbers_with_units() {
        let parsed = parse_text_line_with_config("temp=25.6C,hum=60.2%", &parser_config()).unwrap();
        assert_eq!(
            parsed.schema,
            ParsedSchema::KeyValue {
                keys: vec!["hum".to_owned(), "temp".to_owned()],
            }
        );
        assert_eq!(
            parsed.values,
            vec![("hum".to_owned(), 60.2), ("temp".to_owned(), 25.6)]
        );
    }

    #[test]
    fn rejects_noise_after_numeric_csv_sequence() {
        let parsed = parse_text_line_with_config("1,2,debug,3", &parser_config());
        assert!(parsed.is_none());
    }
}
