use eframe::egui::{self, Color32, RichText, Stroke};

use crate::app::{mono_text_style, preview_text_line, MainView, SerialToolApp};
use crate::serial::DisplayMode;

const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(112, 120, 130);
const ACCENT: Color32 = Color32::from_rgb(92, 138, 196);
const LINE: Color32 = Color32::from_rgb(216, 221, 229);

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(249, 247, 243))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(RichText::new("接收区").color(INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    view_switch(ui, app);
                });
            });

            ui.add_space(8.0);
            toolbar_row(ui, app);

            ui.add_space(8.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(244, 241, 236))
                .stroke(Stroke::new(1.0, LINE))
                .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("高亮关键字").small().color(MUTED));
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.highlight_keywords)
                                    .desired_width(280.0),
                            )
                            .changed()
                        {
                            app.persist_config();
                        }
                        ui.label(
                            RichText::new("用逗号分隔，例如 error,fail,warning")
                                .small()
                                .color(MUTED),
                        );
                    });
                });

            ui.add_space(10.0);
            log_surface(ui, app);
        });
}

fn view_switch(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::none()
        .fill(Color32::from_rgb(244, 241, 236))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.active_view, MainView::Plot, "数据绘图");
                ui.selectable_value(&mut app.active_view, MainView::Monitor, "串口监视");
            });
        });
}

fn toolbar_row(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::none()
        .fill(Color32::from_rgb(244, 241, 236))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("显示模式").small().color(MUTED));
                for mode in DisplayMode::ALL {
                    if ui
                        .selectable_value(&mut app.receive_mode, mode, mode.label())
                        .changed()
                    {
                        app.persist_config();
                    }
                }

                ui.separator();

                if ui
                    .checkbox(&mut app.show_timestamps, "显示时间戳")
                    .changed()
                {
                    app.persist_config();
                }

                ui.separator();
                ui.label(RichText::new("过滤").small().color(MUTED));
                if ui
                    .add(egui::TextEdit::singleline(&mut app.receive_filter).desired_width(220.0))
                    .changed()
                {
                    app.persist_config();
                }

                ui.separator();

                if ui.button("清空接收").clicked() {
                    app.clear_receive();
                }
                if ui.button("导出日志").clicked() {
                    app.export_receive_log();
                }
            });
        });
}

fn log_surface(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    let filtered = app.filtered_receive_records();
    let highlights = app.highlight_words();

    egui::Frame::none()
        .fill(Color32::from_rgb(255, 255, 255))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.set_min_height(420.0);

            if filtered.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(48.0);
                    ui.label(
                        RichText::new("当前没有可显示的接收记录")
                            .strong()
                            .color(INK),
                    );
                    ui.label(
                        RichText::new("连接串口后，这里会按时间顺序展示收到的原始数据。")
                            .small()
                            .color(MUTED),
                    );
                });
                return;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for (index, record) in filtered.into_iter().enumerate() {
                        let content =
                            crate::app::display_receive_data(app.receive_mode, &record.data);
                        let content_lower = content.to_lowercase();
                        let has_highlight = highlights
                            .iter()
                            .any(|keyword| content_lower.contains(&keyword.to_lowercase()));
                        let row_fill = if has_highlight {
                            Color32::from_rgb(255, 246, 221)
                        } else if index % 2 == 0 {
                            Color32::from_rgb(252, 251, 248)
                        } else {
                            Color32::from_rgb(248, 246, 242)
                        };

                        egui::Frame::none()
                            .fill(row_fill)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(239, 234, 228)))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if app.show_timestamps {
                                        ui.label(
                                            RichText::new(format!("[{}]", record.timestamp))
                                                .monospace()
                                                .color(ACCENT),
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(content.clone())
                                                .text_style(mono_text_style())
                                                .color(INK),
                                        );
                                        if let Some(preview) = preview_text_line(&record.data) {
                                            ui.label(RichText::new(preview).small().color(MUTED));
                                        }
                                    });
                                });
                            });
                        ui.add_space(6.0);
                    }
                });
        });
}
