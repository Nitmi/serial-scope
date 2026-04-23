use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serialport::{DataBits, Parity, SerialPortInfo, SerialPortType, StopBits, UsbPortInfo};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortOption {
    pub port_name: String,
    pub display_label: String,
    pub detail_label: Option<String>,
}

impl SerialPortOption {
    fn from_info(info: SerialPortInfo) -> Self {
        let labels = port_type_labels(&info.port_type, &info.port_name);
        let display_label = match labels.display.as_deref() {
            Some(detail) if !detail.trim().is_empty() => format!("{} {detail}", info.port_name),
            _ => info.port_name.clone(),
        };

        Self {
            port_name: info.port_name,
            display_label,
            detail_label: labels.detail,
        }
    }
}

struct PortTypeLabels {
    display: Option<String>,
    detail: Option<String>,
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

pub fn available_port_options() -> Result<Vec<SerialPortOption>> {
    let ports = serialport::available_ports().map_err(|err| anyhow!("获取串口列表失败: {err}"))?;
    Ok(ports.into_iter().map(SerialPortOption::from_info).collect())
}

fn port_type_labels(port_type: &SerialPortType, port_name: &str) -> PortTypeLabels {
    match port_type {
        SerialPortType::UsbPort(info) => PortTypeLabels {
            display: usb_display_label(info, port_name),
            detail: Some(usb_detail_label(info)),
        },
        SerialPortType::BluetoothPort => PortTypeLabels {
            display: Some("Bluetooth".to_owned()),
            detail: Some("Bluetooth".to_owned()),
        },
        SerialPortType::PciPort => PortTypeLabels {
            display: Some("PCI".to_owned()),
            detail: Some("PCI".to_owned()),
        },
        SerialPortType::Unknown => PortTypeLabels {
            display: None,
            detail: None,
        },
    }
}

fn usb_display_label(info: &UsbPortInfo, port_name: &str) -> Option<String> {
    for value in [info.product.as_deref(), info.manufacturer.as_deref()]
        .into_iter()
        .flatten()
    {
        let compact = compact_usb_name(value, port_name);
        if !compact.is_empty() {
            return Some(compact);
        }
    }

    None
}

fn usb_detail_label(info: &UsbPortInfo) -> String {
    let mut parts = Vec::new();

    push_unique_detail(&mut parts, info.product.as_deref());
    push_unique_detail(&mut parts, info.manufacturer.as_deref());
    push_unique_detail(&mut parts, info.serial_number.as_deref());

    if parts.is_empty() {
        parts.push(format!("USB {:04X}:{:04X}", info.vid, info.pid));
    }

    parts.join(" ")
}

fn push_unique_detail(parts: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value.map(normalize_detail_text) else {
        return;
    };
    if value.is_empty() {
        return;
    }

    let value_lower = value.to_lowercase();
    if parts
        .iter()
        .any(|existing| existing.to_lowercase().contains(&value_lower))
    {
        return;
    }

    parts.push(value);
}

fn normalize_detail_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn compact_usb_name(value: &str, port_name: &str) -> String {
    let normalized = strip_port_name_repetition(&normalize_detail_text(value), port_name);
    if normalized.is_empty() {
        return String::new();
    }

    let lower = normalized.to_lowercase();
    for (needle, label) in [
        ("ch340", "CH340"),
        ("ch341", "CH341"),
        ("ch343", "CH343"),
        ("ch9102", "CH9102"),
        ("cp2102", "CP2102"),
        ("cp2104", "CP2104"),
        ("cp210x", "CP210x"),
        ("ft232", "FT232"),
        ("ft231", "FT231"),
        ("ft230", "FT230"),
        ("pl2303", "PL2303"),
        ("st-link", "STLink"),
        ("stlink", "STLink"),
        ("j-link", "J-Link"),
        ("jlink", "J-Link"),
        ("cmsis-dap", "CMSIS-DAP"),
        ("daplink", "DAPLink"),
        ("arduino", "Arduino"),
        ("esp32", "ESP32"),
        ("esp8266", "ESP8266"),
        ("rp2040", "RP2040"),
    ] {
        if lower.contains(needle) {
            return label.to_owned();
        }
    }

    let compact = normalized
        .replace("USB-SERIAL", "")
        .replace("USB Serial", "")
        .replace("USB UART", "")
        .replace("USB CDC", "")
        .replace("Serial Port", "")
        .replace("COM Port", "");
    let compact = normalize_detail_text(&compact);
    if compact.is_empty()
        || compact.eq_ignore_ascii_case("usb")
        || compact.eq_ignore_ascii_case("port")
    {
        return String::new();
    }

    compact
}

fn strip_port_name_repetition(value: &str, port_name: &str) -> String {
    let trimmed = value.trim();
    if port_name.is_empty() {
        return trimmed.to_owned();
    }

    let without_parenthesized_port = trimmed
        .replace(&format!("({port_name})"), "")
        .replace(&format!("[{port_name}]"), "");
    let without_port_name = without_parenthesized_port.replace(port_name, "");

    normalize_detail_text(
        without_port_name
            .trim_matches(|ch: char| ch == '-' || ch == '_' || ch == ':' || ch.is_whitespace()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_usb_port_with_device_details() {
        let option = SerialPortOption::from_info(SerialPortInfo {
            port_name: "COM13".to_owned(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x1A86,
                pid: 0x7523,
                serial_number: Some("A50285BI".to_owned()),
                manufacturer: Some("wch.cn".to_owned()),
                product: Some("USB-SERIAL CH340".to_owned()),
            }),
        });

        assert_eq!(option.port_name, "COM13");
        assert_eq!(option.display_label, "COM13 CH340");
        assert_eq!(
            option.detail_label.as_deref(),
            Some("USB-SERIAL CH340 wch.cn A50285BI")
        );
    }

    #[test]
    fn uses_vid_pid_when_usb_port_has_no_text_details() {
        let option = SerialPortOption::from_info(SerialPortInfo {
            port_name: "COM14".to_owned(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x0483,
                pid: 0x374B,
                serial_number: None,
                manufacturer: None,
                product: None,
            }),
        });

        assert_eq!(option.display_label, "COM14");
        assert_eq!(option.detail_label.as_deref(), Some("USB 0483:374B"));
    }

    #[test]
    fn removes_repeated_com_name_from_generic_port_label() {
        let option = SerialPortOption::from_info(SerialPortInfo {
            port_name: "COM5".to_owned(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x1234,
                pid: 0x5678,
                serial_number: None,
                manufacturer: None,
                product: Some("Port (COM5)".to_owned()),
            }),
        });

        assert_eq!(option.display_label, "COM5");
    }

    #[test]
    fn compacts_stlink_product_name() {
        let option = SerialPortOption::from_info(SerialPortInfo {
            port_name: "COM14".to_owned(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x0483,
                pid: 0x374B,
                serial_number: Some("066AFF494849887767131337".to_owned()),
                manufacturer: Some("STMicroelectronics".to_owned()),
                product: Some("ST-Link Debug".to_owned()),
            }),
        });

        assert_eq!(option.display_label, "COM14 STLink");
    }

    #[test]
    fn keeps_unknown_port_label_as_port_name_only() {
        let option = SerialPortOption::from_info(SerialPortInfo {
            port_name: "/dev/ttyS0".to_owned(),
            port_type: SerialPortType::Unknown,
        });

        assert_eq!(option.display_label, "/dev/ttyS0");
        assert_eq!(option.detail_label, None);
    }
}
