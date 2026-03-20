use eframe::egui::{self, RichText};

use crate::app::SerialToolApp;
use crate::serial::DisplayMode;

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_height(220.0);
        ui.heading("发送区");
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label("发送模式");
            for mode in DisplayMode::ALL {
                if ui
                    .selectable_value(&mut app.send_mode, mode, mode.label())
                    .changed()
                {
                    app.persist_config();
                }
            }
        });

        ui.add_space(8.0);
        ui.collapsing("协议助手", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .checkbox(&mut app.protocol_assistant.append_newline, "追加 CRLF")
                    .changed()
                {
                    app.persist_config();
                }
                if ui
                    .checkbox(&mut app.protocol_assistant.append_crc16, "追加 CRC16(Modbus)")
                    .changed()
                {
                    app.persist_config();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("前缀 HEX");
                if ui
                    .add(egui::TextEdit::singleline(&mut app.protocol_assistant.prefix_hex).desired_width(180.0))
                    .changed()
                {
                    app.persist_config();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("后缀 HEX");
                if ui
                    .add(egui::TextEdit::singleline(&mut app.protocol_assistant.suffix_hex).desired_width(180.0))
                    .changed()
                {
                    app.persist_config();
                }
            });
        });

        ui.add_space(8.0);
        let hint = if app.send_mode == DisplayMode::Hex {
            "例如: AA BB 01 03"
        } else {
            "输入要发送的文本"
        };

        ui.add(
            egui::TextEdit::multiline(&mut app.send_input)
                .desired_rows(8)
                .lock_focus(true)
                .hint_text(hint),
        );

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button(RichText::new("发送").strong()).clicked() {
                app.send_current_input();
            }
            if ui.button("清空输入").clicked() {
                app.send_input.clear();
            }
            if ui.button("保存为快捷命令").clicked() {
                let name = format!("命令{}", app.quick_commands.len() + 1);
                app.add_quick_command_from_input(name);
            }
        });

        ui.add_space(8.0);
        ui.collapsing("自动发送", |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut enabled = app.auto_send_enabled;
                if ui.checkbox(&mut enabled, "启用").changed() {
                    app.toggle_auto_send(enabled);
                }
                ui.label("间隔 ms");
                if ui
                    .add(egui::DragValue::new(&mut app.auto_send_interval_ms).range(50..=60_000))
                    .changed()
                {
                    app.persist_config();
                }
                ui.label("次数(0=无限)");
                if ui
                    .add(egui::DragValue::new(&mut app.auto_send_repeat_limit).range(0..=1_000_000))
                    .changed()
                {
                    app.persist_config();
                }
            });
            ui.label(format!("当前已自动发送 {} 次", app.auto_send_counter));
        });

        ui.add_space(8.0);
        ui.collapsing("快捷发送", |ui| {
            let quick_commands = app.quick_commands.clone();
            for (index, command) in quick_commands.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.button(format!("发送 {}", command.name)).clicked() {
                        app.send_quick_command(index);
                    }
                    ui.label(format!("{} | {}", command.mode.label(), command.payload));
                    if ui.button("删除").clicked() {
                        app.remove_quick_command(index);
                    }
                });
            }
            if quick_commands.is_empty() {
                ui.label("暂无快捷命令");
            }
        });

        ui.add_space(8.0);
        ui.collapsing("发送历史", |ui| {
            if app.send_history.is_empty() {
                ui.label("暂无发送历史");
            } else {
                let history = app.send_history.clone();
                for item in history {
                    if ui.button(format!("{} | {}", item.mode.label(), item.payload)).clicked() {
                        app.send_mode = item.mode;
                        app.send_input = item.payload;
                    }
                }
            }
        });

        ui.add_space(8.0);
        ui.label(RichText::new("说明").strong());
        ui.label("支持快捷命令、自动发送、CRC16/前后缀处理和发送历史复用。");
    });
}
