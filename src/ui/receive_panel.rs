use eframe::egui::{self, RichText};

use crate::app::{mono_text_style, preview_text_line, SerialToolApp};
use crate::serial::DisplayMode;

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_height(320.0);
        ui.horizontal_wrapped(|ui| {
            ui.heading("接收区");
            ui.separator();
            ui.label("显示模式");
            for mode in DisplayMode::ALL {
                if ui
                    .selectable_value(&mut app.receive_mode, mode, mode.label())
                    .changed()
                {
                    app.persist_config();
                }
            }
            ui.separator();
            if ui.checkbox(&mut app.show_timestamps, "显示时间戳").changed() {
                app.persist_config();
            }
            ui.separator();
            ui.label("过滤");
            if ui
                .add(egui::TextEdit::singleline(&mut app.receive_filter).desired_width(180.0))
                .changed()
            {
                app.persist_config();
            }
            if ui.button("清空接收").clicked() {
                app.clear_receive();
            }
            if ui.button("导出日志").clicked() {
                app.export_receive_log();
            }
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("高亮关键字(逗号分隔)").small());
            if ui
                .add(egui::TextEdit::singleline(&mut app.highlight_keywords).desired_width(260.0))
                .changed()
            {
                app.persist_config();
            }
        });
        ui.label(RichText::new("保留最近 1000 条记录，支持过滤、时间戳切换和关键字高亮。"));
        ui.separator();

        let filtered = app.filtered_receive_records();
        let highlights = app.highlight_words();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for record in filtered {
                    let content = crate::app::display_receive_data(app.receive_mode, &record.data);
                    let content_lower = content.to_lowercase();
                    let has_highlight = highlights
                        .iter()
                        .any(|keyword| content_lower.contains(&keyword.to_lowercase()));
                    let text_color = if has_highlight {
                        egui::Color32::from_rgb(255, 210, 120)
                    } else {
                        egui::Color32::from_rgb(210, 214, 220)
                    };

                    ui.horizontal_wrapped(|ui| {
                        if app.show_timestamps {
                            ui.label(
                                RichText::new(format!("[{}]", record.timestamp))
                                    .monospace()
                                    .color(egui::Color32::from_rgb(120, 172, 255)),
                            );
                        }
                        ui.vertical(|ui| {
                            ui.label(RichText::new(content).text_style(mono_text_style()).color(text_color));
                            if let Some(preview) = preview_text_line(&record.data) {
                                ui.label(RichText::new(preview).small().color(egui::Color32::from_rgb(140, 148, 160)));
                            }
                        });
                    });
                    ui.separator();
                }
            });
    });
}
