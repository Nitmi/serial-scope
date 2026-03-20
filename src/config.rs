use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::serial::{DataBitsSetting, DisplayMode, ParitySetting, SerialPortConfig, StopBitsSetting};

const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub serial: SerialPortConfig,
    pub receive_mode: DisplayMode,
    pub send_mode: DisplayMode,
    pub show_timestamps: bool,
    pub quick_commands: Vec<QuickCommandConfig>,
    pub auto_send: AutoSendConfig,
    pub protocol_assistant: ProtocolAssistantConfig,
    pub parser: ParserConfig,
    pub receive_filter: String,
    pub highlight_keywords: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickCommandConfig {
    pub name: String,
    pub payload: String,
    pub mode: DisplayMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSendConfig {
    pub enabled: bool,
    pub interval_ms: u64,
    pub repeat_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolAssistantConfig {
    pub append_newline: bool,
    pub append_crc16: bool,
    pub prefix_hex: String,
    pub suffix_hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParserMode {
    Auto,
    Csv,
    KeyValue,
}

impl ParserMode {
    pub const ALL: [ParserMode; 3] = [ParserMode::Auto, ParserMode::Csv, ParserMode::KeyValue];

    pub fn label(self) -> &'static str {
        match self {
            ParserMode::Auto => "自动",
            ParserMode::Csv => "CSV",
            ParserMode::KeyValue => "Key=Value",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub mode: ParserMode,
    pub csv_delimiter: char,
    pub csv_channel_names: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            serial: SerialPortConfig::default(),
            receive_mode: DisplayMode::Ascii,
            send_mode: DisplayMode::Ascii,
            show_timestamps: true,
            quick_commands: vec![
                QuickCommandConfig {
                    name: "AT".to_owned(),
                    payload: "AT\r\n".to_owned(),
                    mode: DisplayMode::Ascii,
                },
                QuickCommandConfig {
                    name: "读取寄存器".to_owned(),
                    payload: "01 03 00 00 00 02".to_owned(),
                    mode: DisplayMode::Hex,
                },
            ],
            auto_send: AutoSendConfig {
                enabled: false,
                interval_ms: 1000,
                repeat_limit: 0,
            },
            protocol_assistant: ProtocolAssistantConfig {
                append_newline: false,
                append_crc16: false,
                prefix_hex: String::new(),
                suffix_hex: String::new(),
            },
            parser: ParserConfig {
                mode: ParserMode::Auto,
                csv_delimiter: ',',
                csv_channel_names: "ch1,ch2,ch3".to_owned(),
            },
            receive_filter: String::new(),
            highlight_keywords: "error,fail,warning".to_owned(),
        }
    }
}

impl Default for SerialPortConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115_200,
            data_bits: DataBitsSetting::Eight,
            stop_bits: StopBitsSetting::One,
            parity: ParitySetting::None,
        }
    }
}

impl AppConfig {
    pub fn load_or_default() -> Self {
        let path = Path::new(CONFIG_PATH);
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|_| Self::default()),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self)?;
        fs::write(CONFIG_PATH, text)?;
        Ok(())
    }
}
