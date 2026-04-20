use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serialport::{DataBits, Parity, StopBits};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    Ascii,
    Hex,
}

impl DisplayMode {
    pub const ALL: [DisplayMode; 2] = [DisplayMode::Ascii, DisplayMode::Hex];

    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::Ascii => "ASCII",
            DisplayMode::Hex => "HEX",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataBitsSetting {
    Five,
    Six,
    Seven,
    Eight,
}

impl DataBitsSetting {
    pub const ALL: [DataBitsSetting; 4] = [
        DataBitsSetting::Five,
        DataBitsSetting::Six,
        DataBitsSetting::Seven,
        DataBitsSetting::Eight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DataBitsSetting::Five => "5",
            DataBitsSetting::Six => "6",
            DataBitsSetting::Seven => "7",
            DataBitsSetting::Eight => "8",
        }
    }
}

impl From<DataBitsSetting> for DataBits {
    fn from(value: DataBitsSetting) -> Self {
        match value {
            DataBitsSetting::Five => DataBits::Five,
            DataBitsSetting::Six => DataBits::Six,
            DataBitsSetting::Seven => DataBits::Seven,
            DataBitsSetting::Eight => DataBits::Eight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopBitsSetting {
    One,
    Two,
}

impl StopBitsSetting {
    pub const ALL: [StopBitsSetting; 2] = [StopBitsSetting::One, StopBitsSetting::Two];

    pub fn label(self) -> &'static str {
        match self {
            StopBitsSetting::One => "1",
            StopBitsSetting::Two => "2",
        }
    }
}

impl From<StopBitsSetting> for StopBits {
    fn from(value: StopBitsSetting) -> Self {
        match value {
            StopBitsSetting::One => StopBits::One,
            StopBitsSetting::Two => StopBits::Two,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParitySetting {
    None,
    Odd,
    Even,
}

impl ParitySetting {
    pub const ALL: [ParitySetting; 3] =
        [ParitySetting::None, ParitySetting::Odd, ParitySetting::Even];

    pub fn label(self) -> &'static str {
        match self {
            ParitySetting::None => "None",
            ParitySetting::Odd => "Odd",
            ParitySetting::Even => "Even",
        }
    }
}

impl From<ParitySetting> for Parity {
    fn from(value: ParitySetting) -> Self {
        match value {
            ParitySetting::None => Parity::None,
            ParitySetting::Odd => Parity::Odd,
            ParitySetting::Even => Parity::Even,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialPortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: DataBitsSetting,
    pub stop_bits: StopBitsSetting,
    pub parity: ParitySetting,
}

#[derive(Debug, Clone)]
pub struct SerialSettings {
    pub baud_rate: u32,
    pub data_bits: DataBitsSetting,
    pub stop_bits: StopBitsSetting,
    pub parity: ParitySetting,
}

impl From<SerialPortConfig> for SerialSettings {
    fn from(value: SerialPortConfig) -> Self {
        Self {
            baud_rate: value.baud_rate,
            data_bits: value.data_bits,
            stop_bits: value.stop_bits,
            parity: value.parity,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GuiToSerialMessage {
    Open {
        port_name: String,
        settings: SerialSettings,
    },
    Close,
    Send(Vec<u8>),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum SerialEvent {
    Connected(String),
    Disconnected(String),
    Status(String),
    Error(String),
    DataReceived(Vec<u8>),
}

pub fn available_port_names() -> Result<Vec<String>> {
    let ports = serialport::available_ports().map_err(|err| anyhow!("获取串口列表失败: {err}"))?;
    Ok(ports.into_iter().map(|p| p.port_name).collect())
}
