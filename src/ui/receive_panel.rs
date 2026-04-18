use eframe::egui::{self, Color32, RichText, Stroke};

use crate::app::{mono_text_style, receive_display_text, MainView, SerialToolApp};
use crate::serial::DisplayMode;

const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(112, 120, 130);
const ACCENT: Color32 = Color32::from_rgb(92, 138, 196);
const LINE: Color32 = Color32::from_rgb(216, 221, 229);
const LOG_SURFACE_VERTICAL_PADDING: f32 = 16.0;

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    let frame_vertical_padding = 24.0;
    let panel_content_height = (ui.available_height() - frame_vertical_padding).max(0.0);
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(249, 247, 243))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            ui.set_height(panel_content_height);
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
            let content_bottom_gap = 2.0;
            let log_height = (ui.available_height() - content_bottom_gap).max(0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), log_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    log_surface(ui, app, log_height);
                },
            );
            ui.add_space(content_bottom_gap);
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

fn log_surface(ui: &mut egui::Ui, app: &mut SerialToolApp, min_height: f32) {
    egui::Frame::none()
        .fill(Color32::from_rgb(255, 255, 255))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.set_height((min_height - LOG_SURFACE_VERTICAL_PADDING).max(0.0));
            let anticipated_log_rect = ui.available_rect_before_wrap();
            let pointer_over_log = ui
                .ctx()
                .pointer_hover_pos()
                .is_some_and(|pos| anticipated_log_rect.contains(pos));
            let user_requested_history = app.receive_follow_mode
                == crate::app::ReceiveFollowMode::Follow
                && pointer_over_log
                && ui.ctx().input(|input| {
                    input.raw_scroll_delta.y.abs() > 0.0
                        || input.smooth_scroll_delta.y.abs() > 0.0
                        || (input.pointer.primary_down() && input.pointer.delta().y.abs() > 0.0)
                });
            if user_requested_history {
                app.pause_receive_auto_follow();
            }

            let filtered = app.filtered_receive_records();
            let highlights = app.highlight_words();

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

            let output = egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(app.receive_is_following())
                .show(ui, |ui| {
                    for (index, record) in filtered.into_iter().enumerate() {
                        let content = receive_display_text(app.receive_mode, &record.data);
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
                                    ui.label(
                                        RichText::new(content.clone())
                                            .text_style(mono_text_style())
                                            .color(INK),
                                    );
                                });
                            });
                        ui.add_space(6.0);
                    }

                    if app.receive_is_following() && !user_requested_history {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });

            let max_offset_y = (output.content_size.y - output.inner_rect.height()).max(0.0);
            let is_at_bottom = max_offset_y <= 1.0 || output.state.offset.y >= max_offset_y - 12.0;
            let resume_requested = if app.receive_is_manual() && !is_at_bottom {
                show_follow_resume_button(ui.ctx(), output.inner_rect, app)
            } else {
                false
            };

            if resume_requested {
                app.jump_receive_to_latest();
            } else if app.receive_is_recovering() {
                if is_at_bottom {
                    app.resume_receive_auto_follow();
                } else {
                    app.keep_receive_auto_follow_pending();
                }
            } else if app.receive_is_manual() && is_at_bottom {
                app.resume_receive_auto_follow();
            } else if app.receive_is_manual() {
                app.pause_receive_auto_follow();
            }
        });
}

fn show_follow_resume_button(
    ctx: &egui::Context,
    anchor_rect: egui::Rect,
    app: &SerialToolApp,
) -> bool {
    let button_text = if app.pending_receive_count > 0 {
        format!("回到最新 · {}", app.pending_receive_count)
    } else {
        "回到最新".to_owned()
    };

    let button_padding = egui::vec2(10.0, 5.0);
    let font_id = egui::TextStyle::Button.resolve(&ctx.style());
    let text_width = ctx.fonts(|fonts| {
        fonts
            .layout(button_text.clone(), font_id, Color32::from_rgb(66, 112, 168), f32::INFINITY)
            .size()
            .x
    });
    let estimated_width = (text_width + button_padding.x * 2.0).max(88.0);
    let area_pos = egui::pos2(
        anchor_rect.max.x - estimated_width - 8.0,
        anchor_rect.max.y - 42.0,
    );
    let mut clicked = false;

    egui::Area::new(egui::Id::new("receive_follow_resume_button"))
        .order(egui::Order::Foreground)
        .fixed_pos(area_pos)
        .show(ctx, |ui| {
            ui.spacing_mut().button_padding = button_padding;
            if ui
                .add_sized(
                    egui::vec2(estimated_width, 28.0),
                    egui::Button::new(
                        RichText::new(button_text).color(Color32::from_rgb(66, 112, 168)),
                    )
                    .wrap_mode(egui::TextWrapMode::Extend)
                    .fill(Color32::from_rgba_unmultiplied(241, 246, 252, 248))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(188, 206, 228)))
                    .min_size(egui::vec2(0.0, 28.0)),
                )
                .clicked()
            {
                clicked = true;
            }
        });

    clicked
}
