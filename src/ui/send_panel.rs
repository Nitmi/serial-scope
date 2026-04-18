use eframe::egui::{self, Color32, RichText, Stroke};

use super::panel_shell;
use crate::app::SerialToolApp;
use crate::serial::DisplayMode;

const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(112, 120, 130);
const ACCENT: Color32 = Color32::from_rgb(92, 138, 196);
const LINE: Color32 = Color32::from_rgb(208, 218, 230);
const SOFT_RADIUS: f32 = 9.0;

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    panel_shell::show_main_panel(ui, |ui| {
            ui.heading(RichText::new("发送区").color(INK));

            ui.add_space(10.0);
            mode_toolbar_frame().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("发送模式").small().color(MUTED));
                    for mode in DisplayMode::ALL {
                        if ui
                            .selectable_value(&mut app.send_mode, mode, mode.label())
                            .changed()
                        {
                            app.persist_config();
                        }
                    }
                });
            });

            ui.add_space(8.0);
            let hint = if app.send_mode == DisplayMode::Hex {
                "例如: AA BB 01 03"
            } else {
                "输入要发送的文本"
            };

            section_frame().show(ui, |ui| {
                ui.label(RichText::new("发送内容").small().color(MUTED));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::multiline(&mut app.send_input)
                        .desired_rows(10)
                        .lock_focus(true)
                        .hint_text(hint),
                );

                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let send_button =
                        egui::Button::new(RichText::new("发送").strong().color(Color32::WHITE))
                            .fill(ACCENT);
                    if ui.add(send_button).clicked() {
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
            });

            ui.add_space(8.0);
            section_frame().show(ui, |ui| {
                ui.collapsing("协议助手", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .checkbox(&mut app.protocol_assistant.append_newline, "追加 CRLF")
                            .changed()
                        {
                            app.persist_config();
                        }
                        if ui
                            .checkbox(
                                &mut app.protocol_assistant.append_crc16,
                                "追加 CRC16(Modbus)",
                            )
                            .changed()
                        {
                            app.persist_config();
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("前缀 HEX");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.protocol_assistant.prefix_hex)
                                    .desired_width(200.0),
                            )
                            .changed()
                        {
                            app.persist_config();
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("后缀 HEX");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.protocol_assistant.suffix_hex)
                                    .desired_width(200.0),
                            )
                            .changed()
                        {
                            app.persist_config();
                        }
                    });
                });

                ui.collapsing("自动发送", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let mut enabled = app.auto_send_enabled;
                        if ui.checkbox(&mut enabled, "启用").changed() {
                            app.toggle_auto_send(enabled);
                        }
                        ui.label("间隔 ms");
                        if ui
                            .add(
                                egui::DragValue::new(&mut app.auto_send_interval_ms)
                                    .range(50..=60_000),
                            )
                            .changed()
                        {
                            app.persist_config();
                        }
                        ui.label("次数(0=无限)");
                        if ui
                            .add(
                                egui::DragValue::new(&mut app.auto_send_repeat_limit)
                                    .range(0..=1_000_000),
                            )
                            .changed()
                        {
                            app.persist_config();
                        }
                    });
                    ui.label(
                        RichText::new(format!("当前已自动发送 {} 次", app.auto_send_counter))
                            .small()
                            .color(MUTED),
                    );
                });

                ui.collapsing("快捷发送", |ui| {
                    let quick_commands = app.quick_commands.clone();
                    for (index, command) in quick_commands.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(format!("发送 {}", command.name)).clicked() {
                                app.send_quick_command(index);
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{} | {}",
                                    command.mode.label(),
                                    command.payload
                                ))
                                .color(INK),
                            );
                            if ui.button("删除").clicked() {
                                app.remove_quick_command(index);
                            }
                        });
                    }
                    if quick_commands.is_empty() {
                        ui.label(RichText::new("暂无快捷命令").small().color(MUTED));
                    }
                });

                ui.collapsing("发送历史", |ui| {
                    if app.send_history.is_empty() {
                        ui.label(RichText::new("暂无发送历史").small().color(MUTED));
                    } else {
                        let history = app.send_history.clone();
                        for item in history {
                            if ui
                                .button(format!("{} | {}", item.mode.label(), item.payload))
                                .clicked()
                            {
                                app.send_mode = item.mode;
                                app.send_input = item.payload;
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("支持快捷命令、自动发送、CRC16/前后缀处理和发送历史复用。")
                    .small()
                    .color(MUTED),
            );
    });
}

fn section_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Color32::from_rgb(252, 253, 255))
        .stroke(Stroke::new(1.0, LINE))
        .rounding(egui::Rounding::same(SOFT_RADIUS))
        .inner_margin(egui::Margin::symmetric(10.0, 10.0))
}

fn mode_toolbar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Color32::from_rgb(240, 246, 252))
        .stroke(Stroke::new(1.0, LINE))
        .rounding(egui::Rounding::same(SOFT_RADIUS))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
}
