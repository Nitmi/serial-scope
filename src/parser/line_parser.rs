use std::collections::BTreeMap;

use crate::config::{ParserConfig, ParserMode};

#[derive(Debug, Clone)]
pub struct ParsedLine {
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
    let mut values = Vec::new();

    for segment in line.split(',') {
        let (key, value) = segment.split_once('=')?;
        let key = key.trim();
        let value = value.trim().parse::<f32>().ok()?;
        if key.is_empty() {
            return None;
        }
        values.push((key.to_owned(), value));
    }

    if values.is_empty() {
        None
    } else {
        Some(ParsedLine { values })
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

    let mut map = BTreeMap::new();
    for (idx, segment) in line.split(delimiter).enumerate() {
        let value = segment.trim().parse::<f32>().ok()?;
        let name = custom_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("ch{}", idx + 1));
        map.insert(name, value);
    }

    if map.is_empty() {
        None
    } else {
        Some(ParsedLine {
            values: map.into_iter().collect(),
        })
    }
}
