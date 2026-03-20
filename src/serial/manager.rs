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
                GuiToSerialMessage::Open { port_name, settings } => {
                    if port.is_some() {
                        let _ = evt_tx.send(SerialEvent::Status("正在切换串口连接".to_owned()));
                    }
                    port = None;
                    current_name = None;

                    match open_port(&port_name, &settings) {
                        Ok(opened) => {
                            current_name = Some(port_name.clone());
                            port = Some(opened);
                            let _ = evt_tx.send(SerialEvent::Connected(port_name));
                        }
                        Err(err) => {
                            let _ = evt_tx.send(SerialEvent::Error(format!("打开串口失败: {err}")));
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
                            let _ = evt_tx.send(SerialEvent::Disconnected("发送时连接中断".to_owned()));
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
