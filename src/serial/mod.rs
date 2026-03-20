pub mod manager;
pub mod protocol;
pub mod types;

pub use manager::SerialManager;
pub use protocol::{bytes_to_ascii_display, bytes_to_hex_display};
pub use protocol::build_port_payload as build_port_options;
pub use types::{
    available_port_names, DataBitsSetting, DisplayMode, GuiToSerialMessage, ParitySetting,
    SerialEvent, SerialPortConfig, SerialSettings, StopBitsSetting,
};
