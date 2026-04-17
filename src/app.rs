use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::Local;
use crossbeam_channel::{Receiver, TryRecvError};
use eframe::egui::{self, TextStyle};

use crate::config::{
    AppConfig, AutoSendConfig, PlotLayoutConfig, ProtocolAssistantConfig, QuickCommandConfig,
};
use crate::parser::{LineAccumulator, ParsedLine, ParsedSchema};
use crate::serial::{
    available_port_names, build_port_options, bytes_to_ascii_display, bytes_to_hex_display,
    DisplayMode, GuiToSerialMessage, SerialEvent, SerialManager, SerialSettings,
};
use crate::ui::{plot_panel, receive_panel, send_panel, top_bar};

const MAX_LOG_LINES: usize = 1_000;
const MAX_PLOT_POINTS: usize = 2_000;
const GUI_REFRESH_MS: u64 = 30;
const PORT_REFRESH_MS: u64 = 1_500;
const MAX_LINE_PREVIEW_CHARS: usize = 120;
const MAX_SEND_HISTORY: usize = 20;

pub struct SerialToolApp {
    pub config: AppConfig,
    serial_manager: SerialManager,
    serial_events: Receiver<SerialEvent>,
    pub port_names: Vec<String>,
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
    pub export_base_name: String,
    pub protocol_assistant: ProtocolAssistantConfig,
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
        let mut app = Self {
            port_names: Vec::new(),
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
            export_base_name: format!("serial_log_{}", Local::now().format("%Y%m%d_%H%M%S")),
            protocol_assistant,
            config,
            serial_manager,
            serial_events,
        };
        app.refresh_ports();
        app
    }

    pub fn refresh_ports(&mut self) {
        match available_port_names() {
            Ok(ports) => {
                let ports_changed = self.port_names != ports;
                self.port_names = ports;

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

        let settings = SerialSettings::from(self.config.serial.clone());
        self.serial_manager.send(GuiToSerialMessage::Open {
            port_name,
            settings,
        });
    }

    pub fn close_port(&mut self) {
        self.serial_manager.send(GuiToSerialMessage::Close);
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
        let path = format!("{}_receive.txt", self.export_base_name.trim());
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
            Ok(_) => self.status_text = format!("接收日志已导出到 {path}"),
            Err(err) => self.push_error(format!("导出接收日志失败: {err}")),
        }
    }

    pub fn export_plot_csv(&mut self) {
        let path = format!("{}_plot.csv", self.export_base_name.trim());
        let mut headers = vec!["x".to_owned()];
        let visible = self.chart_state.visible_series_names();
        headers.extend(visible.iter().cloned());
        let mut rows = vec![headers.join(",")];
        let max_len = self
            .chart_state
            .series
            .values()
            .map(|values| values.len())
            .max()
            .unwrap_or(0);

        for index in 0..max_len {
            let mut row = Vec::new();
            let mut x_value = String::new();
            for name in &visible {
                if let Some(values) = self.chart_state.series.get(name) {
                    if let Some(point) = values.get(index) {
                        if x_value.is_empty() {
                            x_value = format!("{:.0}", point[0]);
                        }
                    }
                }
            }
            row.push(x_value);
            for name in &visible {
                let cell = self
                    .chart_state
                    .series
                    .get(name)
                    .and_then(|values| values.get(index))
                    .map(|point| format!("{:.6}", point[1]))
                    .unwrap_or_default();
                row.push(cell);
            }
            rows.push(row.join(","));
        }

        match fs::write(&path, rows.join("\n")) {
            Ok(_) => self.status_text = format!("曲线数据已导出到 {path}"),
            Err(err) => self.push_error(format!("导出曲线失败: {err}")),
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
            self.chart_state.ingest(parsed, MAX_PLOT_POINTS);
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
        self.status_text = "接收区已清空".to_owned();
    }

    pub fn clear_plot(&mut self) {
        self.chart_state.clear();
        self.status_text = "曲线数据已清空".to_owned();
    }

    pub fn push_error(&mut self, message: String) {
        self.last_error = Some(message.clone());
        self.status_text = message;
    }
}

impl eframe::App for SerialToolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_serial_events();
        self.handle_auto_send();
        self.update_transfer_rates();

        if !self.is_connected
            && self.last_refresh.elapsed() >= Duration::from_millis(PORT_REFRESH_MS)
        {
            self.refresh_ports();
            self.last_refresh = Instant::now();
        }

        top_bar::show(ctx, self);

        egui::SidePanel::right("send_panel")
            .resizable(true)
            .default_width(372.0)
            .min_width(320.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                send_panel::show(ui, self);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let panel_alignment_trim = 4.5;
            let panel_height = (ui.available_height() - panel_alignment_trim).max(0.0);
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

pub struct PlotState {
    next_index: f64,
    paused: bool,
    pub auto_follow: bool,
    pub x_zoom: f64,
    pub y_zoom: f64,
    pub auto_sidebar_width: bool,
    pub sidebar_width: f32,
    locked_schema: Option<ParsedSchema>,
    pending_schema: Option<PendingSchema>,
    last_filter_message: Option<String>,
    pub series: BTreeMap<String, VecDeque<[f64; 2]>>,
    pub visible: BTreeSet<String>,
}

impl Default for PlotState {
    fn default() -> Self {
        Self {
            next_index: 0.0,
            paused: false,
            auto_follow: true,
            x_zoom: 1.0,
            y_zoom: 1.0,
            auto_sidebar_width: true,
            sidebar_width: 280.0,
            locked_schema: None,
            pending_schema: None,
            last_filter_message: None,
            series: BTreeMap::new(),
            visible: BTreeSet::new(),
        }
    }
}

struct PendingSchema {
    schema: ParsedSchema,
    lines: Vec<ParsedLine>,
}

impl PlotState {
    pub fn from_config(config: &PlotLayoutConfig) -> Self {
        Self {
            auto_sidebar_width: config.auto_sidebar_width,
            sidebar_width: config.sidebar_width.clamp(260.0, 420.0),
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

    pub fn reset_sidebar_width(&mut self) {
        self.auto_sidebar_width = true;
    }

    pub fn ingest(&mut self, parsed: ParsedLine, max_points: usize) {
        if self.locked_schema.as_ref() == Some(&parsed.schema) {
            self.pending_schema = None;
            self.push_line(parsed, max_points);
            return;
        }

        let schema_label = parsed.schema.label();
        let pending = self.pending_schema.take();

        if self.locked_schema.is_some() {
            let mut pending = match pending {
                Some(mut pending) if pending.schema == parsed.schema => {
                    pending.lines.push(parsed);
                    pending
                }
                _ => PendingSchema {
                    schema: parsed.schema.clone(),
                    lines: vec![parsed],
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
                    self.push_line(line, max_points);
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
                pending.lines.push(parsed);
                pending
            }
            _ => PendingSchema {
                schema: parsed.schema.clone(),
                lines: vec![parsed],
            },
        };

        if pending.lines.len() >= 3 {
            let lines = pending.lines.drain(..).collect::<Vec<_>>();
            let locked = pending.schema.clone();
            self.locked_schema = Some(locked.clone());
            self.last_filter_message = Some(format!("已锁定绘图格式: {}", locked.label()));
            for line in lines {
                self.push_line(line, max_points);
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

    fn push_line(&mut self, parsed: ParsedLine, max_points: usize) {
        for (name, value) in parsed.values {
            let points = self.series.entry(name.clone()).or_default();
            self.visible.insert(name);
            points.push_back([self.next_index, value as f64]);
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
    }

    fn clear_series_data(&mut self) {
        self.next_index = 0.0;
        self.series.clear();
        self.visible.clear();
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

    pub fn x_bounds(&self) -> Option<(f64, f64)> {
        let max_x = self.max_x()?;
        let span = (200.0 / self.x_zoom.max(0.1)).max(10.0);
        Some(((max_x - span).max(0.0), max_x + 5.0))
    }

    pub fn y_bounds(&self) -> Option<(f64, f64)> {
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for (name, values) in &self.series {
            if !self.visible.contains(name) {
                continue;
            }
            for point in values {
                min_y = min_y.min(point[1]);
                max_y = max_y.max(point[1]);
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
    }

    pub fn schema_status_text(&self) -> String {
        if let Some(message) = &self.last_filter_message {
            if let Some(locked) = &self.locked_schema {
                return format!("{} | 当前主格式: {}", message, locked.label());
            }
            return message.clone();
        }

        if let Some(locked) = &self.locked_schema {
            return format!("当前主格式: {}", locked.label());
        }

        "等待稳定的绘图数据格式（连续 3 条一致数据后开始绘图）".to_owned()
    }

    pub fn visible_series_names(&self) -> Vec<String> {
        self.series
            .keys()
            .filter(|name| self.visible.contains(*name))
            .cloned()
            .collect()
    }

    fn max_x(&self) -> Option<f64> {
        self.series
            .iter()
            .filter(|(name, _)| self.visible.contains(*name))
            .filter_map(|(_, values)| values.back().map(|point| point[0]))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn latest_points_summary(&self) -> String {
        if self.series.is_empty() {
            return "尚未解析到可绘图数据，当前支持 CSV 与 key=value 行解析。".to_owned();
        }
        let mut parts = Vec::new();
        for (name, values) in &self.series {
            if !self.visible.contains(name) {
                continue;
            }
            if let Some(last) = values.back() {
                parts.push(format!("{name}={:.3}", last[1]));
            }
        }
        if parts.is_empty() {
            "当前没有启用中的曲线。".to_owned()
        } else {
            format!("最新数据: {}", parts.join(", "))
        }
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

pub fn preview_text_line(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let line = normalized
        .lines()
        .find(|line| !line.trim().is_empty())?
        .trim();
    let preview = if line.chars().count() > MAX_LINE_PREVIEW_CHARS {
        let mut shortened = line
            .chars()
            .take(MAX_LINE_PREVIEW_CHARS)
            .collect::<String>();
        shortened.push_str("...");
        shortened
    } else {
        line.to_owned()
    };
    Some(preview)
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
    use super::PlotState;
    use crate::parser::{ParsedLine, ParsedSchema};

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
    fn locks_schema_after_three_matching_lines() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 32);
        state.ingest(csv_line(2.0), 32);
        assert!(state.series.is_empty());

        state.ingest(csv_line(3.0), 32);
        assert_eq!(state.visible_series_names().len(), 2);
        assert_eq!(state.series["ch1"].len(), 3);
        assert!(state.schema_status_text().contains("当前主格式"));
    }

    #[test]
    fn drops_single_mismatched_line_after_lock() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 32);
        state.ingest(csv_line(2.0), 32);
        state.ingest(csv_line(3.0), 32);
        state.ingest(kv_line(50.0), 32);

        assert_eq!(state.series["ch1"].len(), 3);
        assert!(!state.series.contains_key("hum"));
        assert!(state.schema_status_text().contains("已过滤不匹配数据"));
    }

    #[test]
    fn switches_schema_after_three_consecutive_new_lines() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 32);
        state.ingest(csv_line(2.0), 32);
        state.ingest(csv_line(3.0), 32);
        state.ingest(kv_line(50.0), 32);
        state.ingest(kv_line(51.0), 32);
        state.ingest(kv_line(52.0), 32);

        assert!(!state.series.contains_key("ch1"));
        assert_eq!(state.series["hum"].len(), 3);
        assert!(state.schema_status_text().contains("已切换到"));
    }

    #[test]
    fn clear_resets_locked_schema_and_candidates() {
        let mut state = PlotState::default();

        state.ingest(csv_line(1.0), 32);
        state.ingest(csv_line(2.0), 32);
        state.clear();

        assert!(state.series.is_empty());
        assert!(state
            .schema_status_text()
            .contains("等待稳定的绘图数据格式"));
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
}
