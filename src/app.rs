use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::Local;
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use eframe::egui::{self, TextStyle};
use egui_plot::PlotBounds;
use rfd::FileDialog;

use crate::config::{
    AppConfig, AutoSendConfig, PlotLayoutConfig, PlotXAxisMode, ProtocolAssistantConfig,
    QuickCommandConfig,
};
use crate::parser::{read_plot_csv, ImportedPlotData, LineAccumulator, ParsedLine, ParsedSchema};
use crate::serial::{
    available_port_options, build_port_options, bytes_to_ascii_display, bytes_to_hex_display,
    DisplayMode, GuiToSerialMessage, SerialEvent, SerialManager, SerialPortOption, SerialSettings,
};
use crate::ui::{panel_shell, plot_panel, receive_panel, send_panel, top_bar};
use crate::update::{self, UpdateCheckResult, UpdateEvent, UpdateState};

const MAX_LOG_LINES: usize = 100_000;
const MAX_PLOT_POINTS: usize = 200_000;
const GUI_REFRESH_MS: u64 = 30;
const PORT_REFRESH_MS: u64 = 1_500;
const MAX_SEND_HISTORY: usize = 20;
const UPDATE_CHECK_DELAY_MS: u64 = 2_000;

pub struct SerialToolApp {
    pub config: AppConfig,
    serial_manager: SerialManager,
    serial_events: Receiver<SerialEvent>,
    pub port_names: Vec<String>,
    pub port_options: Vec<SerialPortOption>,
    pub status_text: String,
    pub last_error: Option<String>,
    pub receive_mode: DisplayMode,
    pub send_mode: DisplayMode,
    pub show_timestamps: bool,
    pub send_input: String,
    pub receive_lines: VecDeque<ReceiveRecord>,
    pending_receive: Option<Vec<u8>>,
    line_accumulator: LineAccumulator,
    pub chart_state: PlotState,
    pub stats: TransferStats,
    pub is_connected: bool,
    last_refresh: Instant,
    pub active_view: MainView,
    pub start_time: Instant,
    pub rx_rate_bps: f64,
    pub tx_rate_bps: f64,
    last_rate_snapshot: Instant,
    last_rx_snapshot: u64,
    last_tx_snapshot: u64,
    pub auto_send_enabled: bool,
    pub auto_send_interval_ms: u64,
    pub auto_send_repeat_limit: u32,
    pub auto_send_counter: u32,
    next_auto_send_at: Option<Instant>,
    pub quick_commands: Vec<QuickCommandConfig>,
    pub selected_quick_command: usize,
    pub send_history: VecDeque<QuickCommandConfig>,
    pub receive_filter: String,
    pub highlight_keywords: String,
    pub receive_follow_mode: ReceiveFollowMode,
    pub pending_receive_count: usize,
    pub protocol_assistant: ProtocolAssistantConfig,
    pub update_state: UpdateState,
    update_events_tx: Sender<UpdateEvent>,
    update_events: Receiver<UpdateEvent>,
    next_update_check_at: Option<Instant>,
}

impl SerialToolApp {
    pub fn new(config: AppConfig) -> Self {
        let serial_manager = SerialManager::start();
        let serial_events = serial_manager.subscribe();
        let quick_commands = config.quick_commands.clone();
        let auto_send = config.auto_send.clone();
        let protocol_assistant = config.protocol_assistant.clone();
        let receive_filter = config.receive_filter.clone();
        let highlight_keywords = config.highlight_keywords.clone();
        let (update_events_tx, update_events) = unbounded();
        let mut app = Self {
            port_names: Vec::new(),
            port_options: Vec::new(),
            status_text: "未连接".to_owned(),
            last_error: None,
            receive_mode: config.receive_mode,
            send_mode: config.send_mode,
            show_timestamps: config.show_timestamps,
            send_input: String::new(),
            receive_lines: VecDeque::new(),
            pending_receive: None,
            line_accumulator: LineAccumulator::default(),
            chart_state: PlotState::from_config(&config.plot_layout),
            stats: TransferStats::default(),
            is_connected: false,
            last_refresh: Instant::now() - Duration::from_secs(1),
            active_view: MainView::Monitor,
            start_time: Instant::now(),
            rx_rate_bps: 0.0,
            tx_rate_bps: 0.0,
            last_rate_snapshot: Instant::now(),
            last_rx_snapshot: 0,
            last_tx_snapshot: 0,
            auto_send_enabled: auto_send.enabled,
            auto_send_interval_ms: auto_send.interval_ms,
            auto_send_repeat_limit: auto_send.repeat_limit,
            auto_send_counter: 0,
            next_auto_send_at: None,
            quick_commands,
            selected_quick_command: 0,
            send_history: VecDeque::new(),
            receive_filter,
            highlight_keywords,
            receive_follow_mode: ReceiveFollowMode::Follow,
            pending_receive_count: 0,
            protocol_assistant,
            update_state: UpdateState::Idle,
            update_events_tx,
            update_events,
            next_update_check_at: Some(
                Instant::now() + Duration::from_millis(UPDATE_CHECK_DELAY_MS),
            ),
            config,
            serial_manager,
            serial_events,
        };
        app.refresh_ports();
        app
    }

    pub fn refresh_ports(&mut self) {
        match available_port_options() {
            Ok(port_options) => {
                let ports_changed = self.port_options != port_options;
                self.port_names = port_options
                    .iter()
                    .map(|port| port.port_name.clone())
                    .collect();
                self.port_options = port_options;

                let mut selection_changed = false;
                if self.port_names.is_empty() {
                    self.status_text = "未发现串口设备".to_owned();
                } else if self.config.serial.port_name.is_empty()
                    || !self.port_names.contains(&self.config.serial.port_name)
                {
                    self.config.serial.port_name = self.port_names[0].clone();
                    selection_changed = true;
                }

                if ports_changed || selection_changed {
                    self.persist_config();
                }
            }
            Err(err) => self.push_error(format!("刷新串口失败: {err}")),
        }
    }

    pub fn persist_config(&mut self) {
        self.config.receive_mode = self.receive_mode;
        self.config.send_mode = self.send_mode;
        self.config.show_timestamps = self.show_timestamps;
        self.config.quick_commands = self.quick_commands.clone();
        self.config.auto_send = AutoSendConfig {
            enabled: self.auto_send_enabled,
            interval_ms: self.auto_send_interval_ms,
            repeat_limit: self.auto_send_repeat_limit,
        };
        self.config.protocol_assistant = self.protocol_assistant.clone();
        self.config.receive_filter = self.receive_filter.clone();
        self.config.highlight_keywords = self.highlight_keywords.clone();
        self.config.plot_layout = PlotLayoutConfig {
            auto_sidebar_width: self.chart_state.auto_sidebar_width,
            sidebar_width: self.chart_state.sidebar_width,
            x_axis_mode: self.chart_state.x_axis_mode,
        };
        if let Err(err) = self.config.save() {
            self.last_error = Some(format!("保存配置失败: {err}"));
        }
    }

    pub fn open_port(&mut self) {
        let port_name = self.config.serial.port_name.trim().to_owned();
        if port_name.is_empty() {
            self.push_error("请选择串口".to_owned());
            return;
        }

        self.last_error = None;
        self.status_text = format!("正在打开: {port_name}");
        let settings = SerialSettings::from(self.config.serial.clone());
        self.serial_manager.send(GuiToSerialMessage::Open {
            port_name,
            settings,
        });
    }

    pub fn close_port(&mut self) {
        self.serial_manager.send(GuiToSerialMessage::Close);
    }

    pub fn show_send_panel(&mut self) {
        if !self.config.show_send_panel {
            self.config.show_send_panel = true;
            self.persist_config();
        }
    }

    pub fn hide_send_panel(&mut self) {
        if self.config.show_send_panel {
            self.config.show_send_panel = false;
            self.persist_config();
        }
    }

    pub fn send_current_input(&mut self) {
        let input = self.send_input.clone();
        let mode = self.send_mode;
        if let Err(err) = self.send_payload(mode, &input, true) {
            self.push_error(err.to_string());
        }
    }

    pub fn send_quick_command(&mut self, index: usize) {
        if let Some(command) = self.quick_commands.get(index).cloned() {
            self.send_mode = command.mode;
            self.send_input = command.payload.clone();
            if let Err(err) = self.send_payload(command.mode, &command.payload, false) {
                self.push_error(err.to_string());
            }
        }
    }

    fn send_payload(&mut self, mode: DisplayMode, input: &str, update_editor: bool) -> Result<()> {
        if !self.is_connected {
            return Err(anyhow!("串口尚未打开"));
        }

        let payload = build_tx_payload(mode, input, &self.protocol_assistant)?;
        if payload.is_empty() {
            return Err(anyhow!("发送内容为空"));
        }

        let payload_len = payload.len();
        self.stats.tx_bytes += payload_len as u64;
        self.serial_manager.send(GuiToSerialMessage::Send(payload));
        self.status_text = format!(
            "本次发送 {} 字节，累计 TX {} 字节",
            payload_len, self.stats.tx_bytes
        );

        let history_item = QuickCommandConfig {
            name: format!("{} {}", mode.label(), Local::now().format("%H:%M:%S")),
            payload: input.to_owned(),
            mode,
        };
        self.send_history.push_front(history_item);
        while self.send_history.len() > MAX_SEND_HISTORY {
            self.send_history.pop_back();
        }

        if update_editor {
            self.persist_config();
        }
        Ok(())
    }

    pub fn add_quick_command_from_input(&mut self, name: String) {
        let trimmed = name.trim();
        if trimmed.is_empty() || self.send_input.trim().is_empty() {
            self.push_error("快捷命令名称和内容不能为空".to_owned());
            return;
        }

        self.quick_commands.push(QuickCommandConfig {
            name: trimmed.to_owned(),
            payload: self.send_input.clone(),
            mode: self.send_mode,
        });
        self.persist_config();
    }

    pub fn remove_quick_command(&mut self, index: usize) {
        if index < self.quick_commands.len() {
            self.quick_commands.remove(index);
            if self.selected_quick_command >= self.quick_commands.len() {
                self.selected_quick_command = self.quick_commands.len().saturating_sub(1);
            }
            self.persist_config();
        }
    }

    pub fn toggle_auto_send(&mut self, enabled: bool) {
        self.auto_send_enabled = enabled;
        self.auto_send_counter = 0;
        self.next_auto_send_at = if enabled {
            Some(Instant::now() + Duration::from_millis(self.auto_send_interval_ms.max(50)))
        } else {
            None
        };
        self.persist_config();
    }

    pub fn export_receive_log(&mut self) {
        let default_name = export_file_name("receive", "txt");
        let Some(path) = FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("文本日志", &["txt", "log"])
            .save_file()
        else {
            return;
        };
        let mut text = String::new();
        for record in &self.receive_lines {
            let content = receive_display_text(self.receive_mode, &record.data);
            if self.show_timestamps {
                text.push_str(&format!("[{}] {}\n", record.timestamp, content));
            } else {
                text.push_str(&format!("{}\n", content));
            }
        }
        match fs::write(&path, text) {
            Ok(_) => {
                self.status_text = format!("接收日志已导出到 {}", path.display());
            }
            Err(err) => self.push_error(format!("导出接收日志失败: {err}")),
        }
    }

    pub fn export_plot_csv(&mut self) {
        if self.chart_state.visible_series_keys().is_empty() {
            self.push_error("没有可导出的可见曲线".to_owned());
            return;
        }

        let default_name = export_file_name("plot", "csv");
        let Some(path) = FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("CSV 文件", &["csv"])
            .save_file()
        else {
            return;
        };

        let export_result = (|| -> Result<()> {
            let file = fs::File::create(&path)?;
            self.chart_state.write_plot_csv(file)
        })();

        match export_result {
            Ok(_) => {
                self.status_text = format!("曲线数据已导出到 {}", path.display());
            }
            Err(err) => self.push_error(format!("导出曲线失败: {err}")),
        }
    }

    pub fn import_plot_csv(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("CSV 文件", &["csv"])
            .pick_file()
        else {
            return;
        };

        match read_plot_csv(&path, MAX_PLOT_POINTS) {
            Ok(data) => {
                let series_count = data.series.len();
                let point_count = data.row_count;
                let axis_summary = match (data.has_point_axis, data.has_time_axis) {
                    (true, true) => "可切换点数/时间轴",
                    (true, false) => "文件仅包含点数轴",
                    (false, true) => "文件仅包含时间轴",
                    (false, false) => "文件未包含有效 X 轴",
                };
                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.chart_state.load_imported_data(data, file_name.clone());
                self.last_error = None;
                self.status_text = format!(
                    "已载入历史数据 {file_name}: {series_count} 条曲线，{point_count} 行，{axis_summary}；实时绘图已暂停"
                );
                self.persist_config();
            }
            Err(err) => self.push_error(format!("导入曲线失败: {err}")),
        }
    }

    pub fn filtered_receive_records(&self) -> Vec<&ReceiveRecord> {
        if self.receive_filter.trim().is_empty() {
            return self.receive_lines.iter().collect();
        }

        let needle = self.receive_filter.to_lowercase();
        self.receive_lines
            .iter()
            .filter(|record| {
                receive_display_text(self.receive_mode, &record.data)
                    .to_lowercase()
                    .contains(&needle)
            })
            .collect()
    }

    pub fn highlight_words(&self) -> Vec<String> {
        self.highlight_keywords
            .split(',')
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| item.to_owned())
            .collect()
    }

    fn process_serial_events(&mut self) {
        loop {
            match self.serial_events.try_recv() {
                Ok(event) => self.handle_serial_event(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.push_error("串口事件通道已断开".to_owned());
                    break;
                }
            }
        }
    }

    fn handle_serial_event(&mut self, event: SerialEvent) {
        match event {
            SerialEvent::Connected(name) => {
                self.is_connected = true;
                self.pending_receive = None;
                self.line_accumulator.clear();
                self.receive_follow_mode = ReceiveFollowMode::Follow;
                self.pending_receive_count = 0;
                self.start_time = Instant::now();
                self.last_rate_snapshot = Instant::now();
                self.last_rx_snapshot = self.stats.rx_bytes;
                self.last_tx_snapshot = self.stats.tx_bytes;
                self.status_text = format!("已连接: {name}");
                self.last_error = None;
            }
            SerialEvent::Disconnected(reason) => {
                self.is_connected = false;
                self.pending_receive = None;
                self.line_accumulator.clear();
                self.receive_follow_mode = ReceiveFollowMode::Follow;
                self.pending_receive_count = 0;
                self.auto_send_enabled = false;
                self.next_auto_send_at = None;
                self.status_text = format!("已断开: {reason}");
                self.refresh_ports();
            }
            SerialEvent::Status(text) => self.status_text = text,
            SerialEvent::Error(text) => self.push_error(text),
            SerialEvent::DataReceived(bytes) => {
                self.stats.rx_bytes += bytes.len() as u64;
                self.push_receive_record(bytes.clone());
                let parsed = self
                    .line_accumulator
                    .push_bytes(&bytes, &self.config.parser);
                self.consume_parsed_lines(parsed);
            }
        }
    }

    fn push_receive_record(&mut self, bytes: Vec<u8>) {
        let mut buffer = self.pending_receive.take().unwrap_or_default();
        buffer.extend_from_slice(&bytes);

        while let Some((line, consumed)) = take_complete_line(&buffer) {
            if !line.is_empty() {
                self.store_receive_record(ReceiveRecord {
                    timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
                    data: line,
                });
            }
            buffer.drain(..consumed);
        }

        if !buffer.is_empty() {
            self.pending_receive = Some(buffer);
        }
    }

    fn store_receive_record(&mut self, record: ReceiveRecord) {
        if self.receive_follow_mode == ReceiveFollowMode::Manual {
            self.pending_receive_count = self.pending_receive_count.saturating_add(1);
        }
        self.receive_lines.push_back(record);
        while self.receive_lines.len() > MAX_LOG_LINES {
            self.receive_lines.pop_front();
        }
    }

    fn consume_parsed_lines(&mut self, parsed_lines: Vec<ParsedLine>) {
        if self.chart_state.is_paused() {
            return;
        }
        for parsed in parsed_lines {
            let elapsed_secs = self.start_time.elapsed().as_secs_f64();
            self.chart_state
                .ingest(parsed, elapsed_secs, MAX_PLOT_POINTS);
        }
    }

    fn handle_auto_send(&mut self) {
        if !self.auto_send_enabled {
            return;
        }
        if !self.is_connected {
            self.auto_send_enabled = false;
            self.next_auto_send_at = None;
            return;
        }

        let now = Instant::now();
        if self
            .next_auto_send_at
            .is_some_and(|deadline| now >= deadline)
        {
            let input = self.send_input.clone();
            let mode = self.send_mode;
            if let Err(err) = self.send_payload(mode, &input, false) {
                self.push_error(format!("自动发送失败: {err}"));
                self.auto_send_enabled = false;
                self.next_auto_send_at = None;
                return;
            }
            self.auto_send_counter = self.auto_send_counter.saturating_add(1);
            if self.auto_send_repeat_limit > 0
                && self.auto_send_counter >= self.auto_send_repeat_limit
            {
                self.auto_send_enabled = false;
                self.next_auto_send_at = None;
            } else {
                self.next_auto_send_at =
                    Some(now + Duration::from_millis(self.auto_send_interval_ms.max(50)));
            }
        }
    }

    fn update_transfer_rates(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_rate_snapshot).as_secs_f64();
        if elapsed < 0.5 {
            return;
        }
        self.rx_rate_bps =
            (self.stats.rx_bytes.saturating_sub(self.last_rx_snapshot)) as f64 / elapsed;
        self.tx_rate_bps =
            (self.stats.tx_bytes.saturating_sub(self.last_tx_snapshot)) as f64 / elapsed;
        self.last_rate_snapshot = now;
        self.last_rx_snapshot = self.stats.rx_bytes;
        self.last_tx_snapshot = self.stats.tx_bytes;
    }

    pub fn clear_receive(&mut self) {
        self.receive_lines.clear();
        self.pending_receive = None;
        self.line_accumulator.clear();
        self.receive_follow_mode = ReceiveFollowMode::Follow;
        self.pending_receive_count = 0;
        self.status_text = "接收区已清空".to_owned();
    }

    pub fn resume_receive_auto_follow(&mut self) {
        self.receive_follow_mode = ReceiveFollowMode::Follow;
        self.pending_receive_count = 0;
    }

    pub fn pause_receive_auto_follow(&mut self) {
        self.receive_follow_mode = ReceiveFollowMode::Manual;
    }

    pub fn jump_receive_to_latest(&mut self) {
        self.receive_follow_mode = ReceiveFollowMode::Recovering;
        self.pending_receive_count = 0;
    }

    pub fn keep_receive_auto_follow_pending(&mut self) {
        self.receive_follow_mode = ReceiveFollowMode::Recovering;
    }

    pub fn receive_is_following(&self) -> bool {
        matches!(
            self.receive_follow_mode,
            ReceiveFollowMode::Follow | ReceiveFollowMode::Recovering
        )
    }

    pub fn receive_is_manual(&self) -> bool {
        self.receive_follow_mode == ReceiveFollowMode::Manual
    }

    pub fn receive_is_recovering(&self) -> bool {
        self.receive_follow_mode == ReceiveFollowMode::Recovering
    }

    pub fn clear_plot(&mut self) {
        self.chart_state.clear();
        self.status_text = "曲线数据已清空".to_owned();
    }

    pub fn push_error(&mut self, message: String) {
        self.last_error = Some(message.clone());
        self.status_text = message;
    }

    pub fn trigger_update_check(&mut self) {
        if matches!(
            self.update_state,
            UpdateState::Checking | UpdateState::Downloading { .. }
        ) {
            return;
        }

        self.update_state = UpdateState::Checking;
        self.next_update_check_at = None;
        update::spawn_check(self.update_events_tx.clone());
    }

    pub fn trigger_update_install(&mut self) {
        let version = match self.update_state.clone() {
            UpdateState::Available { version, .. } => version,
            _ => return,
        };

        self.update_state = UpdateState::Downloading {
            version: version.clone(),
        };
        update::spawn_install(version, self.update_events_tx.clone());
    }

    pub fn restart_after_update(&mut self) {
        let current_exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(err) => {
                self.update_state = UpdateState::Error(format!("重启失败: {err}"));
                return;
            }
        };

        if let Err(err) = Command::new(&current_exe).spawn() {
            self.update_state = UpdateState::Error(format!("重启失败: {err}"));
            return;
        }

        std::process::exit(0);
    }

    fn process_update_events(&mut self) {
        loop {
            match self.update_events.try_recv() {
                Ok(UpdateEvent::CheckCompleted(result)) => match result {
                    Ok(UpdateCheckResult::Available { version, notes }) => {
                        self.update_state = UpdateState::Available { version, notes };
                    }
                    Ok(UpdateCheckResult::UpToDate) => {
                        self.update_state = UpdateState::UpToDate;
                    }
                    Err(message) => {
                        let _ = message;
                        self.update_state = UpdateState::Idle;
                    }
                },
                Ok(UpdateEvent::InstallCompleted(result)) => match result {
                    Ok(version) => {
                        self.update_state = UpdateState::ReadyToRestart { version };
                    }
                    Err(message) => {
                        self.update_state = UpdateState::Error(message);
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if matches!(self.update_state, UpdateState::Checking) {
                        self.update_state = UpdateState::Idle;
                    } else {
                        self.update_state = UpdateState::Error("更新任务通道已断开".to_owned());
                    }
                    break;
                }
            }
        }
    }
}

impl eframe::App for SerialToolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_serial_events();
        self.process_update_events();
        self.handle_auto_send();
        self.update_transfer_rates();

        if self
            .next_update_check_at
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.trigger_update_check();
        }

        if !self.is_connected
            && self.last_refresh.elapsed() >= Duration::from_millis(PORT_REFRESH_MS)
        {
            self.refresh_ports();
            self.last_refresh = Instant::now();
        }

        top_bar::show(ctx, self);

        if self.config.show_send_panel {
            egui::SidePanel::right("send_panel")
                .resizable(true)
                .default_width(372.0)
                .min_width(320.0)
                .max_width(440.0)
                .show(ctx, |ui| {
                    const SEND_PANEL_TOP_OFFSET: f32 = 6.0;
                    const SEND_PANEL_HEIGHT_TRIM: f32 = 6.0;
                    let available_height = ui.available_height();
                    ui.add_space(SEND_PANEL_TOP_OFFSET);
                    let panel_height = (available_height
                        - panel_shell::MAIN_PANEL_ALIGNMENT_TRIM
                        - SEND_PANEL_TOP_OFFSET)
                        - SEND_PANEL_HEIGHT_TRIM.max(0.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), panel_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            send_panel::show(ui, self);
                        },
                    );
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let panel_height =
                (ui.available_height() - panel_shell::MAIN_PANEL_ALIGNMENT_TRIM).max(0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), panel_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| match self.active_view {
                    MainView::Monitor => receive_panel::show(ui, self),
                    MainView::Plot => plot_panel::show(ui, self),
                },
            );
        });

        ctx.request_repaint_after(Duration::from_millis(GUI_REFRESH_MS));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.serial_manager.send(GuiToSerialMessage::Shutdown);
        self.persist_config();
    }
}

#[derive(Clone)]
pub struct ReceiveRecord {
    pub timestamp: String,
    pub data: Vec<u8>,
}

fn take_complete_line(buffer: &[u8]) -> Option<(Vec<u8>, usize)> {
    for (index, byte) in buffer.iter().enumerate() {
        if *byte == b'\n' || *byte == b'\r' {
            let consumed = if *byte == b'\r' && buffer.get(index + 1) == Some(&b'\n') {
                index + 2
            } else {
                index + 1
            };
            return Some((buffer[..index].to_vec(), consumed));
        }
    }
    None
}

#[derive(Default)]
pub struct TransferStats {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Monitor,
    Plot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveFollowMode {
    Follow,
    Manual,
    Recovering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotFollowMode {
    Follow,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlotXAxisAvailability {
    point: bool,
    time: bool,
}

impl Default for PlotXAxisAvailability {
    fn default() -> Self {
        Self {
            point: true,
            time: true,
        }
    }
}

pub struct PlotState {
    next_index: f64,
    paused: bool,
    pub follow_mode: PlotFollowMode,
    pub x_axis_mode: PlotXAxisMode,
    x_axis_availability: PlotXAxisAvailability,
    pub x_zoom: f64,
    pub y_zoom: f64,
    last_applied_x_zoom: f64,
    last_applied_y_zoom: f64,
    pub auto_sidebar_width: bool,
    pub sidebar_width: f32,
    locked_schema: Option<ParsedSchema>,
    pending_schema: Option<PendingSchema>,
    last_filter_message: Option<String>,
    pub series: BTreeMap<String, VecDeque<PlotPoint>>,
    pub visible: BTreeSet<String>,
    display_names: BTreeMap<String, String>,
    imported_file_name: Option<String>,
    renaming_series_key: Option<String>,
    renaming_series_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotPoint {
    pub index: f64,
    pub elapsed_secs: f64,
    pub value: f64,
}

impl PlotPoint {
    pub fn x(&self, mode: PlotXAxisMode) -> f64 {
        match mode {
            PlotXAxisMode::Point => self.index,
            PlotXAxisMode::Time => self.elapsed_secs,
        }
    }
}

impl Default for PlotState {
    fn default() -> Self {
        Self {
            next_index: 0.0,
            paused: false,
            follow_mode: PlotFollowMode::Follow,
            x_axis_mode: PlotXAxisMode::Point,
            x_axis_availability: PlotXAxisAvailability::default(),
            x_zoom: 1.0,
            y_zoom: 1.0,
            last_applied_x_zoom: 1.0,
            last_applied_y_zoom: 1.0,
            auto_sidebar_width: true,
            sidebar_width: 280.0,
            locked_schema: None,
            pending_schema: None,
            last_filter_message: None,
            series: BTreeMap::new(),
            visible: BTreeSet::new(),
            display_names: BTreeMap::new(),
            imported_file_name: None,
            renaming_series_key: None,
            renaming_series_name: String::new(),
        }
    }
}

struct PendingSchema {
    schema: ParsedSchema,
    lines: Vec<PendingPlotLine>,
}

struct PendingPlotLine {
    parsed: ParsedLine,
    elapsed_secs: f64,
}

impl PlotState {
    pub fn from_config(config: &PlotLayoutConfig) -> Self {
        Self {
            auto_sidebar_width: config.auto_sidebar_width,
            sidebar_width: config.sidebar_width.clamp(260.0, 420.0),
            x_axis_mode: config.x_axis_mode,
            ..Self::default()
        }
    }

    pub fn effective_sidebar_width(&self, available_width: f32) -> f32 {
        let auto_width = (available_width * 0.26).clamp(260.0, 420.0);
        if self.auto_sidebar_width {
            auto_width
        } else {
            self.sidebar_width.clamp(260.0, 420.0)
        }
    }

    pub fn set_manual_sidebar_width(&mut self, width: f32) {
        self.auto_sidebar_width = false;
        self.sidebar_width = width.clamp(260.0, 420.0);
    }

    #[cfg(test)]
    pub fn reset_sidebar_width(&mut self) {
        self.auto_sidebar_width = true;
    }

    pub fn ingest(&mut self, parsed: ParsedLine, elapsed_secs: f64, max_points: usize) {
        if self.locked_schema.as_ref() == Some(&parsed.schema) {
            self.pending_schema = None;
            self.push_line(parsed, elapsed_secs, max_points);
            return;
        }

        let schema_label = parsed.schema.label();
        let pending = self.pending_schema.take();

        if self.locked_schema.is_some() {
            let mut pending = match pending {
                Some(mut pending) if pending.schema == parsed.schema => {
                    pending.lines.push(PendingPlotLine {
                        parsed,
                        elapsed_secs,
                    });
                    pending
                }
                _ => PendingSchema {
                    schema: parsed.schema.clone(),
                    lines: vec![PendingPlotLine {
                        parsed,
                        elapsed_secs,
                    }],
                },
            };

            if pending.lines.len() >= 3 {
                let lines = pending.lines.drain(..).collect::<Vec<_>>();
                let new_schema = pending.schema.clone();
                self.clear_series_data();
                self.locked_schema = Some(new_schema.clone());
                self.last_filter_message = Some(format!(
                    "检测到新的稳定格式，已切换到 {}",
                    new_schema.label()
                ));
                for line in lines {
                    self.push_line(line.parsed, line.elapsed_secs, max_points);
                }
            } else {
                self.pending_schema = Some(pending);
                self.last_filter_message =
                    Some(format!("已过滤不匹配数据，候选格式: {schema_label}"));
            }
            return;
        }

        let mut pending = match pending {
            Some(mut pending) if pending.schema == parsed.schema => {
                pending.lines.push(PendingPlotLine {
                    parsed,
                    elapsed_secs,
                });
                pending
            }
            _ => PendingSchema {
                schema: parsed.schema.clone(),
                lines: vec![PendingPlotLine {
                    parsed,
                    elapsed_secs,
                }],
            },
        };

        if pending.lines.len() >= 3 {
            let lines = pending.lines.drain(..).collect::<Vec<_>>();
            let locked = pending.schema.clone();
            self.locked_schema = Some(locked.clone());
            self.last_filter_message = Some(format!("已锁定绘图格式: {}", locked.label()));
            for line in lines {
                self.push_line(line.parsed, line.elapsed_secs, max_points);
            }
        } else {
            self.last_filter_message = Some(format!(
                "正在识别主格式: {} ({}/3)",
                schema_label,
                pending.lines.len()
            ));
            self.pending_schema = Some(pending);
        }
    }

    fn push_line(&mut self, parsed: ParsedLine, elapsed_secs: f64, max_points: usize) {
        for (name, value) in parsed.values {
            self.display_names
                .entry(name.clone())
                .or_insert_with(|| name.clone());
            let points = self.series.entry(name.clone()).or_default();
            self.visible.insert(name);
            points.push_back(PlotPoint {
                index: self.next_index,
                elapsed_secs,
                value: value as f64,
            });
            while points.len() > max_points {
                points.pop_front();
            }
        }
        self.next_index += 1.0;
    }

    pub fn clear(&mut self) {
        self.clear_series_data();
        self.locked_schema = None;
        self.pending_schema = None;
        self.last_filter_message = None;
        self.resume_auto_follow();
    }

    pub fn load_imported_data(&mut self, data: ImportedPlotData, file_name: String) {
        self.clear();
        self.x_axis_mode = data.x_axis_mode;
        self.x_axis_availability = PlotXAxisAvailability {
            point: data.has_point_axis,
            time: data.has_time_axis,
        };
        self.paused = true;
        self.imported_file_name = Some(file_name);
        self.next_index = data
            .series
            .iter()
            .flat_map(|series| series.points.iter())
            .filter_map(|point| point.index)
            .max_by(f64::total_cmp)
            .map(|index| index + 1.0)
            .unwrap_or(data.row_count as f64);

        for imported_series in data.series {
            let key = self.unique_series_key(&imported_series.name);
            let points = imported_series
                .points
                .into_iter()
                .map(|point| {
                    let row_index = point.row_index as f64;
                    PlotPoint {
                        index: point.index.unwrap_or(row_index),
                        elapsed_secs: point.elapsed_secs.unwrap_or(row_index),
                        value: point.value,
                    }
                })
                .collect::<VecDeque<_>>();
            self.visible.insert(key.clone());
            self.display_names.insert(key.clone(), imported_series.name);
            self.series.insert(key, points);
        }
    }

    fn unique_series_key(&self, name: &str) -> String {
        if !self.series.contains_key(name) {
            return name.to_owned();
        }

        (2..)
            .map(|suffix| format!("{name}#{suffix}"))
            .find(|candidate| !self.series.contains_key(candidate))
            .unwrap_or_else(|| format!("{name}#imported"))
    }

    fn clear_series_data(&mut self) {
        self.next_index = 0.0;
        self.series.clear();
        self.visible.clear();
        self.display_names.clear();
        self.x_axis_availability = PlotXAxisAvailability::default();
        self.imported_file_name = None;
        self.renaming_series_key = None;
        self.renaming_series_name.clear();
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn paused_label(&self) -> &'static str {
        if self.paused {
            "恢复绘图"
        } else {
            "暂停绘图"
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn pause_auto_follow(&mut self) {
        self.follow_mode = PlotFollowMode::Manual;
    }

    pub fn resume_auto_follow(&mut self) {
        self.follow_mode = PlotFollowMode::Follow;
        self.sync_zoom_tracking();
    }

    pub fn is_following(&self) -> bool {
        self.follow_mode == PlotFollowMode::Follow
    }

    pub fn is_manual(&self) -> bool {
        self.follow_mode == PlotFollowMode::Manual
    }

    pub fn set_x_axis_mode(&mut self, mode: PlotXAxisMode) {
        if self.x_axis_available(mode) && self.x_axis_mode != mode {
            self.x_axis_mode = mode;
            self.resume_auto_follow();
        }
    }

    pub fn x_axis_available(&self, mode: PlotXAxisMode) -> bool {
        match mode {
            PlotXAxisMode::Point => self.x_axis_availability.point,
            PlotXAxisMode::Time => self.x_axis_availability.time,
        }
    }

    pub fn x_axis_label(&self) -> &'static str {
        Self::x_axis_label_for(self.x_axis_mode)
    }

    fn x_axis_label_for(mode: PlotXAxisMode) -> &'static str {
        match mode {
            PlotXAxisMode::Point => "点数",
            PlotXAxisMode::Time => "时间(s)",
        }
    }

    fn format_x_value_for(mode: PlotXAxisMode, value: f64) -> String {
        match mode {
            PlotXAxisMode::Point => format!("{value:.0}"),
            PlotXAxisMode::Time => format!("{value:.6}"),
        }
    }

    fn export_x_axis_modes(&self) -> Vec<PlotXAxisMode> {
        let mut modes = Vec::with_capacity(2);
        if self.x_axis_available(self.x_axis_mode) {
            modes.push(self.x_axis_mode);
        }
        for mode in [PlotXAxisMode::Point, PlotXAxisMode::Time] {
            if self.x_axis_available(mode) && !modes.contains(&mode) {
                modes.push(mode);
            }
        }
        modes
    }

    fn write_plot_csv(&self, output: impl Write) -> Result<()> {
        let visible = self.visible_series_keys();
        if visible.is_empty() {
            return Err(anyhow!("没有可导出的可见曲线"));
        }

        let x_axis_modes = self.export_x_axis_modes();
        if x_axis_modes.is_empty() {
            return Err(anyhow!("当前数据没有可导出的 X 轴"));
        }

        let mut headers = x_axis_modes
            .iter()
            .map(|mode| Self::export_x_axis_header(*mode).to_owned())
            .collect::<Vec<_>>();
        headers.extend(visible.iter().map(|name| self.display_name(name)));

        let mut row_indices = visible
            .iter()
            .filter_map(|name| self.series.get(name))
            .flat_map(|values| values.iter().map(|point| point.index))
            .collect::<Vec<_>>();
        row_indices.sort_by(f64::total_cmp);
        row_indices.dedup_by(|left, right| left.total_cmp(right).is_eq());

        let mut writer = csv::WriterBuilder::new()
            .terminator(csv::Terminator::Any(b'\n'))
            .from_writer(output);
        writer.write_record(&headers)?;

        let mut positions = vec![0usize; visible.len()];
        for row_index in row_indices {
            let mut x_cells = vec![String::new(); x_axis_modes.len()];
            let mut value_cells = Vec::with_capacity(visible.len());

            for (series_index, name) in visible.iter().enumerate() {
                let Some(values) = self.series.get(name) else {
                    value_cells.push(String::new());
                    continue;
                };
                let position = &mut positions[series_index];
                while values
                    .get(*position)
                    .is_some_and(|point| point.index.total_cmp(&row_index).is_lt())
                {
                    *position += 1;
                }

                let point = values
                    .get(*position)
                    .filter(|point| point.index.total_cmp(&row_index).is_eq());
                if let Some(point) = point {
                    if x_cells[0].is_empty() {
                        for (cell, mode) in x_cells.iter_mut().zip(&x_axis_modes) {
                            *cell = Self::format_x_value_for(*mode, point.x(*mode));
                        }
                    }
                    value_cells.push(format!("{:.6}", point.value));
                } else {
                    value_cells.push(String::new());
                }
            }

            x_cells.extend(value_cells);
            writer.write_record(&x_cells)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn export_x_axis_header(mode: PlotXAxisMode) -> &'static str {
        match mode {
            PlotXAxisMode::Point => "X轴:点数",
            PlotXAxisMode::Time => "X轴:时间(s)",
        }
    }

    pub fn sync_zoom_tracking(&mut self) {
        self.last_applied_x_zoom = self.x_zoom.max(0.1);
        self.last_applied_y_zoom = self.y_zoom.max(0.1);
    }

    pub fn manual_zoomed_bounds(&mut self, current_bounds: PlotBounds) -> Option<PlotBounds> {
        let previous_x_zoom = self.last_applied_x_zoom.max(0.1);
        let previous_y_zoom = self.last_applied_y_zoom.max(0.1);
        let current_x_zoom = self.x_zoom.max(0.1);
        let current_y_zoom = self.y_zoom.max(0.1);

        let x_changed = (previous_x_zoom - current_x_zoom).abs() > f64::EPSILON;
        let y_changed = (previous_y_zoom - current_y_zoom).abs() > f64::EPSILON;
        if !x_changed && !y_changed {
            return None;
        }

        let min = current_bounds.min();
        let max = current_bounds.max();
        let center_x = (min[0] + max[0]) * 0.5;
        let center_y = (min[1] + max[1]) * 0.5;
        let current_span_x = (max[0] - min[0]).abs().max(1e-6);
        let current_span_y = (max[1] - min[1]).abs().max(1e-6);

        let next_span_x = if x_changed {
            (current_span_x * previous_x_zoom / current_x_zoom).max(1e-6)
        } else {
            current_span_x
        };
        let next_span_y = if y_changed {
            (current_span_y * previous_y_zoom / current_y_zoom).max(1e-6)
        } else {
            current_span_y
        };

        self.sync_zoom_tracking();
        Some(PlotBounds::from_min_max(
            [center_x - next_span_x * 0.5, center_y - next_span_y * 0.5],
            [center_x + next_span_x * 0.5, center_y + next_span_y * 0.5],
        ))
    }

    pub fn x_bounds(&self) -> Option<(f64, f64)> {
        let max_x = self.max_x()?;
        let base_span = match self.x_axis_mode {
            PlotXAxisMode::Point => 200.0,
            PlotXAxisMode::Time => 30.0,
        };
        let min_span = match self.x_axis_mode {
            PlotXAxisMode::Point => 10.0,
            PlotXAxisMode::Time => 1.0,
        };
        let lead = match self.x_axis_mode {
            PlotXAxisMode::Point => 5.0,
            PlotXAxisMode::Time => 0.5,
        };
        let span = (base_span / self.x_zoom.max(0.1)).max(min_span);
        Some(((max_x - span).max(0.0), max_x + lead))
    }

    pub fn y_bounds(&self) -> Option<(f64, f64)> {
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for (name, values) in &self.series {
            if !self.visible.contains(name) {
                continue;
            }
            for point in values {
                min_y = min_y.min(point.value);
                max_y = max_y.max(point.value);
            }
        }
        if !min_y.is_finite() || !max_y.is_finite() {
            return None;
        }
        let center = (min_y + max_y) * 0.5;
        let raw_span = (max_y - min_y).max(1.0);
        let span = (raw_span / self.y_zoom.max(0.1)).max(0.5);
        Some((center - span * 0.6, center + span * 0.6))
    }

    pub fn clear_series(&mut self, name: &str) {
        self.series.remove(name);
        self.visible.remove(name);
        self.display_names.remove(name);
        if self.renaming_series_key.as_deref() == Some(name) {
            self.cancel_series_renaming();
        }
    }

    pub fn visible_series_keys(&self) -> Vec<String> {
        self.series
            .keys()
            .filter(|name| self.visible.contains(*name))
            .cloned()
            .collect()
    }

    pub fn display_name(&self, key: &str) -> String {
        self.display_names
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn imported_file_name(&self) -> Option<&str> {
        self.imported_file_name.as_deref()
    }

    pub fn start_series_renaming(&mut self, key: &str) {
        self.renaming_series_key = Some(key.to_owned());
        self.renaming_series_name = self.display_name(key);
    }

    pub fn is_renaming_series(&self, key: &str) -> bool {
        self.renaming_series_key.as_deref() == Some(key)
    }

    pub fn renaming_series_name_mut(&mut self) -> Option<&mut String> {
        self.renaming_series_key.as_ref()?;
        Some(&mut self.renaming_series_name)
    }

    pub fn commit_series_renaming(&mut self) -> bool {
        let Some(key) = self.renaming_series_key.clone() else {
            return false;
        };
        let renamed = self.renaming_series_name.trim();
        if renamed.is_empty() {
            return false;
        }

        self.display_names.insert(key, renamed.to_owned());
        self.renaming_series_key = None;
        self.renaming_series_name.clear();
        true
    }

    pub fn cancel_series_renaming(&mut self) {
        self.renaming_series_key = None;
        self.renaming_series_name.clear();
    }

    fn max_x(&self) -> Option<f64> {
        self.series
            .iter()
            .filter(|(name, _)| self.visible.contains(*name))
            .filter_map(|(_, values)| values.back().map(|point| point.x(self.x_axis_mode)))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

pub fn display_receive_data(mode: DisplayMode, bytes: &[u8]) -> String {
    match mode {
        DisplayMode::Ascii => bytes_to_ascii_display(bytes),
        DisplayMode::Hex => bytes_to_hex_display(bytes),
    }
}

pub fn receive_display_text(mode: DisplayMode, bytes: &[u8]) -> String {
    display_receive_data(mode, bytes)
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}

pub fn mono_text_style() -> TextStyle {
    TextStyle::Monospace
}

pub fn build_tx_payload(
    mode: DisplayMode,
    input: &str,
    assistant: &ProtocolAssistantConfig,
) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    if !assistant.prefix_hex.trim().is_empty() {
        payload.extend(build_port_options(DisplayMode::Hex, &assistant.prefix_hex)?);
    }
    payload.extend(build_port_options(mode, input)?);
    if assistant.append_newline {
        payload.extend_from_slice(b"\r\n");
    }
    if !assistant.suffix_hex.trim().is_empty() {
        payload.extend(build_port_options(DisplayMode::Hex, &assistant.suffix_hex)?);
    }
    if assistant.append_crc16 {
        let crc = modbus_crc16(&payload);
        payload.push((crc & 0xFF) as u8);
        payload.push((crc >> 8) as u8);
    }
    Ok(payload)
}

fn export_file_name(kind: &str, extension: &str) -> String {
    format!(
        "serial_log_{}_{}.{}",
        Local::now().format("%Y%m%d_%H%M%S"),
        kind,
        extension
    )
}

fn modbus_crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for byte in data {
        crc ^= *byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::{
        PlotFollowMode, PlotState, ReceiveFollowMode, ReceiveRecord, SerialToolApp, MAX_PLOT_POINTS,
    };
    use crate::config::{AppConfig, PlotXAxisMode};
    use crate::parser::plot_csv::{parse_plot_csv, ImportedPlotPoint, ImportedPlotSeries};
    use crate::parser::{ImportedPlotData, ParsedLine, ParsedSchema};
    use egui_plot::PlotBounds;

    fn csv_line(index: f32) -> ParsedLine {
        ParsedLine {
            schema: ParsedSchema::Csv { channels: 2 },
            values: vec![("ch1".to_owned(), index), ("ch2".to_owned(), index + 10.0)],
        }
    }

    fn kv_line(value: f32) -> ParsedLine {
        ParsedLine {
            schema: ParsedSchema::KeyValue {
                keys: vec!["hum".to_owned(), "temp".to_owned()],
            },
            values: vec![("hum".to_owned(), value), ("temp".to_owned(), value + 1.0)],
        }
    }

    #[test]
    fn imported_history_replaces_series_and_preserves_time_positions() {
        let mut state = PlotState::default();
        state.ingest(csv_line(1.0), 0.0, 32);
        state.ingest(csv_line(2.0), 0.1, 32);
        state.ingest(csv_line(3.0), 0.2, 32);

        state.load_imported_data(
            ImportedPlotData {
                x_axis_mode: PlotXAxisMode::Time,
                has_point_axis: true,
                has_time_axis: true,
                series: vec![ImportedPlotSeries {
                    name: "温度".to_owned(),
                    points: vec![
                        ImportedPlotPoint {
                            row_index: 0,
                            index: Some(10.0),
                            elapsed_secs: Some(0.5),
                            value: 20.0,
                        },
                        ImportedPlotPoint {
                            row_index: 2,
                            index: Some(12.0),
                            elapsed_secs: Some(1.5),
                            value: 22.0,
                        },
                    ],
                }],
                row_count: 3,
            },
            "history.csv".to_owned(),
        );

        assert!(state.is_paused());
        assert!(state.is_following());
        assert_eq!(state.x_axis_mode, PlotXAxisMode::Time);
        assert_eq!(state.imported_file_name(), Some("history.csv"));
        assert!(!state.series.contains_key("ch1"));
        assert_eq!(state.series["温度"].len(), 2);
        assert_eq!(state.series["温度"][1].index, 12.0);
        assert_eq!(state.series["温度"][1].elapsed_secs, 1.5);
        assert_eq!(state.next_index, 13.0);
        assert!(state.x_axis_available(PlotXAxisMode::Point));
        assert!(state.x_axis_available(PlotXAxisMode::Time));

        state.set_x_axis_mode(PlotXAxisMode::Point);
        assert_eq!(state.x_axis_mode, PlotXAxisMode::Point);
        assert_eq!(state.series["温度"][1].x(state.x_axis_mode), 12.0);
    }

    #[test]
    fn legacy_import_does_not_enable_a_missing_x_axis() {
        let mut state = PlotState::default();
        state.load_imported_data(
            ImportedPlotData {
                x_axis_mode: PlotXAxisMode::Point,
                has_point_axis: true,
                has_time_axis: false,
                series: vec![ImportedPlotSeries {
                    name: "ch1".to_owned(),
                    points: vec![ImportedPlotPoint {
                        row_index: 0,
                        index: Some(5.0),
                        elapsed_secs: None,
                        value: 20.0,
                    }],
                }],
                row_count: 1,
            },
            "legacy.csv".to_owned(),
        );

        assert!(state.x_axis_available(PlotXAxisMode::Point));
        assert!(!state.x_axis_available(PlotXAxisMode::Time));
        state.set_x_axis_mode(PlotXAxisMode::Time);
        assert_eq!(state.x_axis_mode, PlotXAxisMode::Point);
    }

    #[test]
    fn exported_csv_round_trips_both_x_axes_with_current_axis_first() {
        let mut state = PlotState::default();
        state.ingest(csv_line(1.0), 0.125, 32);
        state.ingest(csv_line(2.0), 0.250, 32);
        state.ingest(csv_line(3.0), 0.375, 32);
        state.set_x_axis_mode(PlotXAxisMode::Time);

        let mut csv = Vec::new();
        state.write_plot_csv(&mut csv).unwrap();
        let text = String::from_utf8(csv.clone()).unwrap();
        assert!(text.starts_with("X轴:时间(s),X轴:点数,ch1,ch2\n"));

        let imported = parse_plot_csv(csv.as_slice(), 32).unwrap();
        assert_eq!(imported.x_axis_mode, PlotXAxisMode::Time);
        assert!(imported.has_point_axis);
        assert!(imported.has_time_axis);
        assert_eq!(imported.series[0].points[2].index, Some(2.0));
        assert_eq!(imported.series[0].points[2].elapsed_secs, Some(0.375));
    }

    #[test]
    fn exported_csv_keeps_sparse_series_aligned_by_point_index() {
        let mut state = PlotState::default();
        state.load_imported_data(
            ImportedPlotData {
                x_axis_mode: PlotXAxisMode::Point,
                has_point_axis: true,
                has_time_axis: true,
                series: vec![
                    ImportedPlotSeries {
                        name: "a".to_owned(),
                        points: vec![
                            ImportedPlotPoint {
                                row_index: 0,
                                index: Some(0.0),
                                elapsed_secs: Some(0.0),
                                value: 10.0,
                            },
                            ImportedPlotPoint {
                                row_index: 2,
                                index: Some(2.0),
                                elapsed_secs: Some(0.2),
                                value: 12.0,
                            },
                        ],
                    },
                    ImportedPlotSeries {
                        name: "b".to_owned(),
                        points: (0..3)
                            .map(|index| ImportedPlotPoint {
                                row_index: index,
                                index: Some(index as f64),
                                elapsed_secs: Some(index as f64 * 0.1),
                                value: 20.0 + index as f64,
                            })
                            .collect(),
                    },
                ],
                row_count: 3,
            },
            "sparse.csv".to_owned(),
        );

        let mut csv = Vec::new();
        state.write_plot_csv(&mut csv).unwrap();
        let imported = parse_plot_csv(csv.as_slice(), 32).unwrap();

        assert_eq!(imported.series[0].points.len(), 2);
        assert_eq!(imported.series[0].points[1].index, Some(2.0));
        assert_eq!(imported.series[1].points.len(), 3);
        assert_eq!(imported.series[1].points[1].index, Some(1.0));
    }

    #[test]
    fn locks_schema_after_three_matching_lines() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 0.0, 32);
        state.ingest(csv_line(2.0), 0.0, 32);
        assert!(state.series.is_empty());

        state.ingest(csv_line(3.0), 0.0, 32);
        assert_eq!(state.visible_series_keys().len(), 2);
        assert_eq!(state.series["ch1"].len(), 3);
        assert_eq!(state.locked_schema, Some(ParsedSchema::Csv { channels: 2 }));
    }

    #[test]
    fn drops_single_mismatched_line_after_lock() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 0.0, 32);
        state.ingest(csv_line(2.0), 0.0, 32);
        state.ingest(csv_line(3.0), 0.0, 32);
        state.ingest(kv_line(50.0), 0.0, 32);

        assert_eq!(state.series["ch1"].len(), 3);
        assert!(!state.series.contains_key("hum"));
        assert!(state
            .last_filter_message
            .as_deref()
            .is_some_and(|message| message.contains("已过滤不匹配数据")));
    }

    #[test]
    fn switches_schema_after_three_consecutive_new_lines() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 0.0, 32);
        state.ingest(csv_line(2.0), 0.0, 32);
        state.ingest(csv_line(3.0), 0.0, 32);
        state.ingest(kv_line(50.0), 0.0, 32);
        state.ingest(kv_line(51.0), 0.0, 32);
        state.ingest(kv_line(52.0), 0.0, 32);

        assert!(!state.series.contains_key("ch1"));
        assert_eq!(state.series["hum"].len(), 3);
        assert_eq!(
            state.locked_schema,
            Some(ParsedSchema::KeyValue {
                keys: vec!["hum".to_owned(), "temp".to_owned()],
            })
        );
        assert!(state
            .last_filter_message
            .as_deref()
            .is_some_and(|message| message.contains("已切换到")));
    }

    #[test]
    fn clear_resets_locked_schema_and_candidates() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 0.0, 32);
        state.ingest(csv_line(2.0), 0.0, 32);
        state.clear();

        assert!(state.series.is_empty());
        assert!(state.locked_schema.is_none());
        assert!(state.pending_schema.is_none());
        assert!(state.last_filter_message.is_none());
    }

    #[test]
    fn auto_sidebar_width_stays_within_expected_range() {
        let state = PlotState::default();

        assert_eq!(state.effective_sidebar_width(800.0), 260.0);
        assert_eq!(state.effective_sidebar_width(1_800.0), 420.0);
    }

    #[test]
    fn manual_sidebar_width_can_override_and_reset() {
        let mut state = PlotState::default();

        state.set_manual_sidebar_width(390.0);
        assert!(!state.auto_sidebar_width);
        assert_eq!(state.effective_sidebar_width(1_200.0), 390.0);

        state.reset_sidebar_width();
        assert!(state.auto_sidebar_width);
        assert_eq!(state.effective_sidebar_width(1_200.0), 312.0);
    }

    #[test]
    fn plot_follow_mode_switches_between_follow_and_manual() {
        let mut state = PlotState::default();

        assert_eq!(state.follow_mode, PlotFollowMode::Follow);
        state.pause_auto_follow();
        assert_eq!(state.follow_mode, PlotFollowMode::Manual);

        state.resume_auto_follow();
        assert_eq!(state.follow_mode, PlotFollowMode::Follow);
    }

    #[test]
    fn series_display_name_can_be_renamed_without_changing_key() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 0.0, 32);
        state.ingest(csv_line(2.0), 0.0, 32);
        state.ingest(csv_line(3.0), 0.0, 32);
        state.start_series_renaming("ch1");
        *state.renaming_series_name_mut().unwrap() = "温度".to_owned();

        assert!(state.commit_series_renaming());
        assert_eq!(state.display_name("ch1"), "温度");
        assert!(state.series.contains_key("ch1"));
    }

    #[test]
    fn x_axis_mode_switches_between_point_index_and_time() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 1.5, 32);
        state.ingest(csv_line(2.0), 2.5, 32);
        state.ingest(csv_line(3.0), 3.5, 32);

        let first = state.series["ch1"].front().unwrap();
        assert_eq!(first.x(PlotXAxisMode::Point), 0.0);
        assert_eq!(first.x(PlotXAxisMode::Time), 1.5);

        state.set_x_axis_mode(PlotXAxisMode::Time);
        assert_eq!(state.x_axis_label(), "时间(s)");
        assert_eq!(state.max_x(), Some(3.5));
    }

    #[test]
    fn clearing_plot_restores_plot_follow_mode() {
        let mut state = PlotState::default();

        state.pause_auto_follow();
        state.clear();

        assert_eq!(state.follow_mode, PlotFollowMode::Follow);
    }

    #[test]
    fn manual_zoomed_bounds_updates_current_view_without_follow_mode() {
        let mut state = PlotState::default();
        state.pause_auto_follow();
        state.x_zoom = 2.0;

        let bounds = state
            .manual_zoomed_bounds(PlotBounds::from_min_max([0.0, 0.0], [100.0, 10.0]))
            .unwrap();

        assert_eq!(bounds.min(), [25.0, 0.0]);
        assert_eq!(bounds.max(), [75.0, 10.0]);
        assert_eq!(state.last_applied_x_zoom, 2.0);
    }

    #[test]
    fn receive_record_increments_pending_count_when_auto_follow_is_paused() {
        let mut app = SerialToolApp::new(AppConfig::default());
        app.pause_receive_auto_follow();

        app.store_receive_record(ReceiveRecord {
            timestamp: "12:00:00.000".to_owned(),
            data: b"weight(g)=32.15".to_vec(),
        });

        assert_eq!(app.pending_receive_count, 1);
    }

    #[test]
    fn resuming_receive_auto_follow_clears_pending_count() {
        let mut app = SerialToolApp::new(AppConfig::default());
        app.pause_receive_auto_follow();
        app.pending_receive_count = 3;

        app.resume_receive_auto_follow();

        assert_eq!(app.receive_follow_mode, ReceiveFollowMode::Follow);
        assert_eq!(app.pending_receive_count, 0);
    }

    #[test]
    fn receive_history_keeps_more_than_old_short_limit() {
        let mut app = SerialToolApp::new(AppConfig::default());

        for index in 0..1_500 {
            app.store_receive_record(ReceiveRecord {
                timestamp: "12:00:00.000".to_owned(),
                data: format!("line {index}").into_bytes(),
            });
        }

        assert_eq!(app.receive_lines.len(), 1_500);
        assert_eq!(app.receive_lines.front().unwrap().data, b"line 0");
    }

    #[test]
    fn plot_history_keeps_more_than_old_short_limit() {
        let mut state = PlotState::default();

        for index in 0..2_500 {
            state.ingest(csv_line(index as f32), index as f64, MAX_PLOT_POINTS);
        }

        assert_eq!(state.series["ch1"].len(), 2_500);
        assert_eq!(state.series["ch1"].front().unwrap().index, 0.0);
    }
}
