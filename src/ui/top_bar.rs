use eframe::egui::{self, Align2, Color32, ComboBox, FontId, RichText, Sense, Stroke};

use crate::app::SerialToolApp;
use crate::serial::{DataBitsSetting, ParitySetting, StopBitsSetting};

const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(108, 116, 126);
const ACCENT: Color32 = Color32::from_rgb(92, 138, 196);
const SURFACE: Color32 = Color32::from_rgb(255, 255, 255);
const LINE: Color32 = Color32::from_rgb(208, 218, 230);
const COMMON_BAUD_RATES: [u32; 11] = [
    1_200, 2_400, 4_800, 9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600,
];

pub fn show(ctx: &egui::Context, app: &mut SerialToolApp) {
    egui::TopBottomPanel::top("top_bar")
        .resizable(false)
        .show_separator_line(false)
        .show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(RichText::new("串口调试助手").color(INK));
                        ui.add_space(12.0);
                        status_chip(ui, app);
                        ui.add_space(18.0);
                        metric_text(ui, "TX", format!("{} B", app.stats.tx_bytes));
                        ui.add_space(12.0);
                        metric_text(ui, "RX", format!("{} B", app.stats.rx_bytes));
                        ui.add_space(12.0);
                        metric_text(ui, "TX 速率", format!("{:.1} B/s", app.tx_rate_bps));
                        ui.add_space(12.0);
                        metric_text(ui, "RX 速率", format!("{:.1} B/s", app.rx_rate_bps));
                    });

                    ui.add_space(10.0);
                    let side_error = app
                        .last_error
                        .as_ref()
                        .filter(|error| !error.trim().is_empty())
                        .cloned();
                    ui.horizontal(|ui| {
                        let band_response = primary_band().show(ui, |ui| {
                            let label_height = ui.text_style_height(&egui::TextStyle::Small);
                            let label_gap = ui.spacing().item_spacing.y;
                            let field_height = ui.spacing().interact_size.y;
                            let button_extension = 8.0;
                            let button_drop = 2.0;

                            ui.horizontal(|ui| {
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

                                ui.add_space(8.0);

                                labeled_column(ui, "波特率", |ui| {
                                    ComboBox::from_id_salt("baud_rate")
                                        .width(140.0)
                                        .selected_text(app.config.serial.baud_rate.to_string())
                                        .show_ui(ui, |ui| {
                                            for baud_rate in COMMON_BAUD_RATES {
                                                if ui
                                                    .selectable_value(
                                                        &mut app.config.serial.baud_rate,
                                                        baud_rate,
                                                        baud_rate.to_string(),
                                                    )
                                                    .changed()
                                                {
                                                    app.persist_config();
                                                }
                                            }
                                        });
                                });

                                ui.add_space(8.0);

                                ui.vertical(|ui| {
                                    ui.add_space(
                                        (label_height + label_gap - button_extension + button_drop)
                                            .max(0.0),
                                    );
                                    let connect_label = if app.is_connected {
                                        "关闭串口"
                                    } else {
                                        "打开串口"
                                    };
                                    let button_fill = if app.is_connected {
                                        Color32::from_rgb(122, 133, 148)
                                    } else {
                                        ACCENT
                                    };

                                    if centered_action_button(
                                        ui,
                                        connect_label,
                                        egui::vec2(132.0, field_height + button_extension),
                                        button_fill,
                                    )
                                    .clicked()
                                    {
                                        if app.is_connected {
                                            app.close_port();
                                        } else {
                                            app.open_port();
                                        }
                                    }
                                });
                            });
                        });

                        if let Some(error) = side_error.as_deref() {
                            ui.add_space(12.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(
                                    ui.available_width().max(0.0),
                                    band_response.response.rect.height(),
                                ),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    error_card(ui, error);
                                },
                            );
                        }
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
                });
        });
}

fn primary_band() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .outer_margin(egui::Margin::symmetric(0.0, 2.0))
}

fn error_card(ui: &mut egui::Ui, error: &str) {
    egui::Frame::none()
        .fill(Color32::from_rgb(249, 232, 232))
        .stroke(Stroke::new(1.0, Color32::from_rgb(235, 198, 198)))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.label(RichText::new(error).color(Color32::from_rgb(184, 82, 82)));
        });
}

fn centered_action_button(
    ui: &mut egui::Ui,
    label: &str,
    size: egui::Vec2,
    fill: Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let final_fill = if response.hovered() {
        fill.gamma_multiply(1.05)
    } else {
        fill
    };

    ui.painter()
        .rect(rect, egui::Rounding::same(4.0), final_fill, Stroke::NONE);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(16.0),
        Color32::WHITE,
    );

    response
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

fn metric_text(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).small().color(MUTED));
        ui.label(RichText::new(value).color(MUTED));
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
