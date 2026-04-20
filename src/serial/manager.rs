use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};

use super::{GuiToSerialMessage, SerialEvent, SerialSettings};

pub struct SerialManager {
    sender: Sender<GuiToSerialMessage>,
    receiver: Receiver<SerialEvent>,
}

impl SerialManager {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = unbounded::<GuiToSerialMessage>();
        let (evt_tx, evt_rx) = unbounded::<SerialEvent>();

        thread::spawn(move || serial_worker(cmd_rx, evt_tx));

        Self {
            sender: cmd_tx,
            receiver: evt_rx,
        }
    }

    pub fn send(&self, message: GuiToSerialMessage) {
        let _ = self.sender.send(message);
    }

    pub fn subscribe(&self) -> Receiver<SerialEvent> {
        self.receiver.clone()
    }
}

fn serial_worker(cmd_rx: Receiver<GuiToSerialMessage>, evt_tx: Sender<SerialEvent>) {
    let mut current_name: Option<String> = None;
    let mut port: Option<Box<dyn serialport::SerialPort>> = None;
    let poll_delay = Duration::from_millis(10);
    let mut read_buffer = [0u8; 2048];

    loop {
        while let Ok(message) = cmd_rx.try_recv() {
            match message {
                GuiToSerialMessage::Open {
                    port_name,
                    settings,
                } => {
                    if port.is_some() {
                        let _ = evt_tx.send(SerialEvent::Status("正在切换串口连接".to_owned()));
                    }
                    port = None;
                    current_name = None;
                    let _ = evt_tx.send(SerialEvent::Status(format!("正在打开: {port_name}")));

                    match open_port(&port_name, &settings) {
                        Ok(opened) => {
                            current_name = Some(port_name.clone());
                            port = Some(opened);
                            let _ = evt_tx.send(SerialEvent::Connected(port_name));
                        }
                        Err(err) => {
                            let _ = evt_tx.send(SerialEvent::Error(friendly_open_port_error(
                                &port_name, &err,
                            )));
                        }
                    }
                }
                GuiToSerialMessage::Close => {
                    if let Some(name) = current_name.take() {
                        port = None;
                        let _ = evt_tx.send(SerialEvent::Disconnected(format!("{name} 已关闭")));
                    } else {
                        port = None;
                    }
                }
                GuiToSerialMessage::Send(data) => {
                    if let Some(serial_port) = port.as_mut() {
                        if let Err(err) = serial_port.write_all(&data) {
                            let _ = evt_tx.send(SerialEvent::Error(format!("发送失败: {err}")));
                            port = None;
                            current_name = None;
                            let _ =
                                evt_tx.send(SerialEvent::Disconnected("发送时连接中断".to_owned()));
                        }
                    } else {
                        let _ = evt_tx.send(SerialEvent::Error("串口未打开，无法发送".to_owned()));
                    }
                }
                GuiToSerialMessage::Shutdown => {
                    let _ = evt_tx.send(SerialEvent::Status("串口线程已停止".to_owned()));
                    return;
                }
            }
        }

        if let Some(serial_port) = port.as_mut() {
            match serial_port.read(&mut read_buffer) {
                Ok(count) if count > 0 => {
                    let _ = evt_tx.send(SerialEvent::DataReceived(read_buffer[..count].to_vec()));
                }
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
                Err(err) => {
                    let name = current_name.take().unwrap_or_else(|| "未知串口".to_owned());
                    port = None;
                    let _ = evt_tx.send(SerialEvent::Error(format!("读取失败: {err}")));
                    let _ = evt_tx.send(SerialEvent::Disconnected(format!("{name} 已断开")));
                }
            }
        }

        thread::sleep(poll_delay);
    }
}

fn open_port(
    port_name: &str,
    settings: &SerialSettings,
) -> anyhow::Result<Box<dyn serialport::SerialPort>> {
    let port = serialport::new(port_name, settings.baud_rate)
        .data_bits(settings.data_bits.into())
        .stop_bits(settings.stop_bits.into())
        .parity(settings.parity.into())
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(20))
        .open()?;
    Ok(port)
}

fn friendly_open_port_error(port_name: &str, err: &anyhow::Error) -> String {
    let raw = err.to_string();
    let lower = raw.to_lowercase();

    let reason = if lower.contains("拒绝访问")
        || lower.contains("access is denied")
        || lower.contains("permission denied")
        || lower.contains("resource busy")
        || lower.contains("device or resource busy")
        || lower.contains("used by another process")
        || lower.contains("被占用")
    {
        "串口当前正被其他程序占用，请关闭占用程序后重试。".to_owned()
    } else if lower.contains("系统找不到指定的文件")
        || lower.contains("找不到指定的文件")
        || lower.contains("no such file")
        || lower.contains("not found")
    {
        "串口当前不可用。".to_owned()
    } else if lower.contains("invalid parameter")
        || lower.contains("参数错误")
        || lower.contains("invalid input")
    {
        "串口参数无效，请检查波特率和高级串口参数设置。".to_owned()
    } else {
        format!("请检查设备连接和串口参数。({raw})")
    };

    format!("打开串口失败: {port_name}，{reason}")
}

#[cfg(test)]
mod tests {
    use super::friendly_open_port_error;

    #[test]
    fn maps_not_found_open_error_to_friendly_message() {
        let err = anyhow::anyhow!("系统找不到指定的文件。");
        let message = friendly_open_port_error("COM27", &err);
        assert!(message.contains("打开串口失败: COM27"));
        assert!(message.contains("串口当前不可用"));
        assert!(!message.contains("系统找不到指定的文件"));
    }

    #[test]
    fn maps_access_denied_open_error_to_busy_message() {
        let err = anyhow::anyhow!("拒绝访问。");
        let message = friendly_open_port_error("COM27", &err);
        assert!(message.contains("打开串口失败: COM27"));
        assert!(message.contains("其他程序占用"));
    }

    #[test]
    fn keeps_fallback_detail_for_unknown_open_error() {
        let err = anyhow::anyhow!("mystery failure");
        let message = friendly_open_port_error("COM27", &err);
        assert!(message.contains("请检查设备连接和串口参数"));
        assert!(message.contains("mystery failure"));
    }
}
