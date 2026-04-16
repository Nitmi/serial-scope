use eframe::egui::{self, Color32, ComboBox, RichText, Stroke};

use crate::app::SerialToolApp;
use crate::config::ParserMode;
use crate::serial::{DataBitsSetting, ParitySetting, StopBitsSetting};

pub fn show(ctx: &egui::Context, app: &mut SerialToolApp) {
    egui::TopBottomPanel::top("top_bar")
        .resizable(false)
        .show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("串口调试助手");
                        ui.separator();
                        ui.label(app.connection_rich_text());
                        ui.separator();
                        ui.label(format!("TX: {} B", app.stats.tx_bytes));
                        ui.label(format!("RX: {} B", app.stats.rx_bytes));
                        ui.separator();
                        ui.label(format!("TX 速率: {:.1} B/s", app.tx_rate_bps));
                        ui.label(format!("RX 速率: {:.1} B/s", app.rx_rate_bps));
                        ui.separator();
                        ui.label(format!("运行时长: {}", app.uptime_text()));
                    });

                    ui.add_space(8.0);
                    primary_toolbar_frame(ui.style()).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("串口设备").strong());
                                ComboBox::from_id_salt("port_name")
                                    .width(220.0)
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

                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("连接").strong());
                                if ui
                                    .add_sized([86.0, 28.0], egui::Button::new("刷新串口"))
                                    .clicked()
                                {
                                    app.refresh_ports();
                                }
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.label(RichText::new("波特率").strong());
                                let baud_response = ui.add(
                                    egui::TextEdit::singleline(app.baud_rate_input())
                                        .desired_width(110.0)
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

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.label(RichText::new("串口开关").strong());
                                ui.horizontal(|ui| {
                                    let open_btn = ui.add_enabled(
                                        !app.is_connected,
                                        egui::Button::new(RichText::new("打开串口").strong()),
                                    );
                                    if open_btn.clicked() {
                                        app.apply_baud_rate_input();
                                        if app.last_error.is_none() {
                                            app.open_port();
                                        }
                                    }

                                    let close_btn = ui.add_enabled(
                                        app.is_connected,
                                        egui::Button::new("关闭串口"),
                                    );
                                    if close_btn.clicked() {
                                        app.close_port();
                                    }
                                });
                            });
                        });
                    });

                    ui.add_space(6.0);
                    egui::CollapsingHeader::new("高级串口参数")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("数据位");
                                enum_combo_data_bits(ui, app);
                                ui.separator();
                                ui.label("停止位");
                                enum_combo_stop_bits(ui, app);
                                ui.separator();
                                ui.label("校验位");
                                enum_combo_parity(ui, app);
                            });
                        });

                    ui.add_space(6.0);
                    secondary_toolbar_frame(ui.style()).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("解析").strong());
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
                            ui.label("CSV 分隔符");
                            let mut delimiter_text = app.config.parser.csv_delimiter.to_string();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut delimiter_text)
                                        .desired_width(40.0),
                                )
                                .changed()
                            {
                                if let Some(ch) = delimiter_text.chars().next() {
                                    app.config.parser.csv_delimiter = ch;
                                    app.persist_config();
                                }
                            }
                            ui.label("通道名");
                            if ui
                                .add(
                                    egui::TextEdit::singleline(
                                        &mut app.config.parser.csv_channel_names,
                                    )
                                    .desired_width(220.0),
                                )
                                .changed()
                            {
                                app.persist_config();
                            }

                            ui.separator();
                            ui.label(RichText::new("导出前缀").strong());
                            ui.add(
                                egui::TextEdit::singleline(&mut app.export_base_name)
                                    .desired_width(180.0),
                            );
                            if ui.button("导出曲线 CSV").clicked() {
                                app.export_plot_csv();
                            }
                            if ui.button("导出接收日志").clicked() {
                                app.export_receive_log();
                            }
                        });
                    });

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("状态")
                                .strong()
                                .color(Color32::from_rgb(120, 172, 255)),
                        );
                        ui.label(&app.status_text);
                        if let Some(error) = &app.last_error {
                            ui.separator();
                            ui.label(RichText::new(error).color(Color32::from_rgb(255, 128, 128)));
                        }
                    });
                });
        });
}

fn primary_toolbar_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(Color32::from_rgb(22, 28, 35))
        .stroke(Stroke::new(1.0, Color32::from_rgb(52, 110, 178)))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
}

fn secondary_toolbar_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::group(style)
        .fill(Color32::from_rgb(20, 24, 30))
        .stroke(Stroke::new(1.0, Color32::from_rgb(44, 52, 62)))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
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
