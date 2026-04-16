use eframe::egui::{self, Color32, ComboBox, RichText, Stroke};

use crate::app::SerialToolApp;
use crate::config::ParserMode;
use crate::serial::{DataBitsSetting, ParitySetting, StopBitsSetting};

const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(108, 116, 126);
const ACCENT: Color32 = Color32::from_rgb(92, 138, 196);
const SURFACE: Color32 = Color32::from_rgb(250, 248, 244);
const SURFACE_SOFT: Color32 = Color32::from_rgb(244, 241, 236);
const LINE: Color32 = Color32::from_rgb(214, 220, 228);

pub fn show(ctx: &egui::Context, app: &mut SerialToolApp) {
    egui::TopBottomPanel::top("top_bar")
        .resizable(false)
        .show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(RichText::new("串口调试助手").color(INK));
                        status_chip(ui, app);
                        metric_chip(ui, "TX", format!("{} B", app.stats.tx_bytes));
                        metric_chip(ui, "RX", format!("{} B", app.stats.rx_bytes));
                        metric_chip(ui, "TX 速率", format!("{:.1} B/s", app.tx_rate_bps));
                        metric_chip(ui, "RX 速率", format!("{:.1} B/s", app.rx_rate_bps));
                        metric_chip(ui, "运行时长", app.uptime_text());
                    });

                    ui.add_space(10.0);
                    primary_band().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            labeled_column(ui, "串口设备", |ui| {
                                ComboBox::from_id_salt("port_name")
                                    .width(240.0)
                                    .selected_text(if app.config.serial.port_name.is_empty() {
                                        "选择串口".to_owned()
                                    } else {
                                        app.config.serial.port_name.clone()
                                    })
                                    .show_ui(ui, |ui| {
                                        let ports = app.port_names.clone();
                                        for port in ports {
                                            if ui
                                                .selectable_value(
                                                    &mut app.config.serial.port_name,
                                                    port.clone(),
                                                    port.as_str(),
                                                )
                                                .changed()
                                            {
                                                app.persist_config();
                                            }
                                        }
                                    });
                            });

                            labeled_column(ui, "波特率", |ui| {
                                let baud_response = ui.add(
                                    egui::TextEdit::singleline(app.baud_rate_input())
                                        .desired_width(120.0)
                                        .hint_text("115200"),
                                );
                                if baud_response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    app.apply_baud_rate_input();
                                }
                                if baud_response.changed() {
                                    app.last_error = None;
                                }
                            });

                            ui.vertical(|ui| {
                                ui.label(RichText::new("串口控制").small().color(MUTED));
                                ui.horizontal(|ui| {
                                    let connect_label = if app.is_connected {
                                        "关闭串口"
                                    } else {
                                        "打开串口"
                                    };
                                    let connect_button = egui::Button::new(
                                        RichText::new(connect_label).strong().color(Color32::WHITE),
                                    )
                                    .min_size(egui::vec2(106.0, 30.0))
                                    .fill(if app.is_connected {
                                        Color32::from_rgb(122, 133, 148)
                                    } else {
                                        ACCENT
                                    });

                                    if ui.add(connect_button).clicked() {
                                        if app.is_connected {
                                            app.close_port();
                                        } else {
                                            app.apply_baud_rate_input();
                                            if app.last_error.is_none() {
                                                app.open_port();
                                            }
                                        }
                                    }
                                });
                                ui.label(
                                    RichText::new("串口列表会在未连接时自动刷新")
                                        .small()
                                        .color(MUTED),
                                );
                            });
                        });
                    });

                    ui.add_space(8.0);
                    secondary_band().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            labeled_inline(ui, "解析", |ui| {
                                ComboBox::from_id_salt("parser_mode")
                                    .selected_text(app.config.parser.mode.label())
                                    .show_ui(ui, |ui| {
                                        for mode in ParserMode::ALL {
                                            if ui
                                                .selectable_value(
                                                    &mut app.config.parser.mode,
                                                    mode,
                                                    mode.label(),
                                                )
                                                .changed()
                                            {
                                                app.persist_config();
                                            }
                                        }
                                    });
                            });

                            ui.separator();

                            if ui.button("导出曲线 CSV").clicked() {
                                app.export_plot_csv();
                            }
                            if ui.button("导出接收日志").clicked() {
                                app.export_receive_log();
                            }
                        });

                        ui.add_space(6.0);
                        egui::CollapsingHeader::new(
                            RichText::new("更多解析/导出设置").small().color(MUTED),
                        )
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                labeled_inline(ui, "CSV 分隔符", |ui| {
                                    let mut delimiter_text =
                                        app.config.parser.csv_delimiter.to_string();
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(&mut delimiter_text)
                                                .desired_width(44.0),
                                        )
                                        .changed()
                                    {
                                        if let Some(ch) = delimiter_text.chars().next() {
                                            app.config.parser.csv_delimiter = ch;
                                            app.persist_config();
                                        }
                                    }
                                });

                                labeled_inline(ui, "通道名", |ui| {
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut app.config.parser.csv_channel_names,
                                            )
                                            .desired_width(240.0),
                                        )
                                        .changed()
                                    {
                                        app.persist_config();
                                    }
                                });

                                labeled_inline(ui, "导出前缀", |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.export_base_name)
                                            .desired_width(220.0),
                                    );
                                });
                            });
                        });
                    });

                    ui.add_space(8.0);
                    egui::CollapsingHeader::new(
                        RichText::new("高级串口参数").color(MUTED).strong(),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            labeled_inline(ui, "数据位", |ui| enum_combo_data_bits(ui, app));
                            labeled_inline(ui, "停止位", |ui| enum_combo_stop_bits(ui, app));
                            labeled_inline(ui, "校验位", |ui| enum_combo_parity(ui, app));
                        });
                    });

                    if let Some(error) = &app.last_error {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(Color32::from_rgb(249, 232, 232))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(235, 198, 198)))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("错误")
                                            .strong()
                                            .color(Color32::from_rgb(184, 82, 82)),
                                    );
                                    ui.label(
                                        RichText::new(error).color(Color32::from_rgb(184, 82, 82)),
                                    );
                                });
                            });
                    }
                });
        });
}

fn primary_band() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .outer_margin(egui::Margin::same(0.0))
}

fn secondary_band() -> egui::Frame {
    egui::Frame::none()
        .fill(Color32::from_rgb(246, 243, 238))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .outer_margin(egui::Margin::same(0.0))
}

fn status_chip(ui: &mut egui::Ui, app: &SerialToolApp) {
    let (text, fill, ink) = if app.is_connected {
        (
            "已连接",
            Color32::from_rgb(222, 240, 228),
            Color32::from_rgb(52, 122, 88),
        )
    } else {
        (
            "未连接",
            Color32::from_rgb(249, 226, 226),
            Color32::from_rgb(184, 85, 85),
        )
    };

    egui::Frame::none()
        .fill(fill)
        .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).strong().color(ink));
        });
}

fn metric_chip(ui: &mut egui::Ui, label: &str, value: String) {
    egui::Frame::none()
        .fill(SURFACE_SOFT)
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).small().color(MUTED));
                ui.label(RichText::new(value).strong().color(INK));
            });
        });
}

fn labeled_column(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(MUTED));
        add_contents(ui);
    });
}

fn labeled_inline(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).small().color(MUTED));
        add_contents(ui);
    });
}

fn enum_combo_data_bits(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    ComboBox::from_id_salt("data_bits")
        .selected_text(app.config.serial.data_bits.label())
        .show_ui(ui, |ui| {
            for value in DataBitsSetting::ALL {
                if ui
                    .selectable_value(&mut app.config.serial.data_bits, value, value.label())
                    .changed()
                {
                    app.persist_config();
                }
            }
        });
}

fn enum_combo_stop_bits(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    ComboBox::from_id_salt("stop_bits")
        .selected_text(app.config.serial.stop_bits.label())
        .show_ui(ui, |ui| {
            for value in StopBitsSetting::ALL {
                if ui
                    .selectable_value(&mut app.config.serial.stop_bits, value, value.label())
                    .changed()
                {
                    app.persist_config();
                }
            }
        });
}

fn enum_combo_parity(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    ComboBox::from_id_salt("parity")
        .selected_text(app.config.serial.parity.label())
        .show_ui(ui, |ui| {
            for value in ParitySetting::ALL {
                if ui
                    .selectable_value(&mut app.config.serial.parity, value, value.label())
                    .changed()
                {
                    app.persist_config();
                }
            }
        });
}
