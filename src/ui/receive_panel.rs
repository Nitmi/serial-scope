use eframe::egui::{self, Color32, RichText, Stroke};

use super::panel_shell;
use crate::app::{mono_text_style, receive_display_text, MainView, ReceiveRecord, SerialToolApp};
use crate::serial::DisplayMode;

const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(112, 120, 130);
const ACCENT: Color32 = Color32::from_rgb(92, 138, 196);
const LINE: Color32 = Color32::from_rgb(208, 218, 230);
const SOFT_RADIUS: f32 = 9.0;
const ROW_RADIUS: f32 = 8.0;
const SEGMENT_OUTER_RADIUS: f32 = 18.0;
const SEGMENT_INNER_RADIUS: f32 = 13.0;
const LOG_ROW_HEIGHT: f32 = 42.0;
const LOG_ROW_VERTICAL_MARGIN: f32 = 7.0;

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    panel_shell::show_main_panel(ui, |ui| {
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
            .fill(Color32::from_rgb(240, 246, 252))
            .stroke(Stroke::new(1.0, LINE))
            .rounding(egui::Rounding::same(SOFT_RADIUS))
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
        let content_size = panel_shell::main_content_size(ui);
        let log_height = content_size.y;
        ui.allocate_ui_with_layout(
            content_size,
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                log_surface(ui, app, log_height);
            },
        );
    });
}

fn view_switch(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::none()
        .fill(Color32::from_rgb(243, 248, 253))
        .stroke(Stroke::new(1.0, Color32::from_rgb(198, 213, 229)))
        .rounding(egui::Rounding::same(SEGMENT_OUTER_RADIUS))
        .inner_margin(egui::Margin::symmetric(4.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                segmented_view_button(ui, &mut app.active_view, MainView::Plot, "数据绘图");
                segmented_view_button(ui, &mut app.active_view, MainView::Monitor, "串口监视");
            });
        });
}

fn segmented_view_button(
    ui: &mut egui::Ui,
    active_view: &mut MainView,
    value: MainView,
    label: &str,
) {
    let selected = *active_view == value;
    let text_color = if selected {
        Color32::from_rgb(37, 107, 164)
    } else {
        INK
    };
    let fill = if selected {
        Color32::from_rgb(180, 223, 251)
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if selected {
        Stroke::new(1.0, Color32::from_rgb(165, 208, 240))
    } else {
        Stroke::NONE
    };

    let button = egui::Button::new(RichText::new(label).color(text_color).strong())
        .fill(fill)
        .stroke(stroke)
        .rounding(egui::Rounding::same(SEGMENT_INNER_RADIUS))
        .min_size(egui::vec2(0.0, 30.0));

    if ui.add(button).clicked() {
        *active_view = value;
    }
}

fn toolbar_row(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::none()
        .fill(Color32::from_rgb(240, 246, 252))
        .stroke(Stroke::new(1.0, LINE))
        .rounding(egui::Rounding::same(SOFT_RADIUS))
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
    let surface_size = egui::vec2(ui.available_width(), min_height.max(0.0));
    let (surface_rect, _) = ui.allocate_exact_size(surface_size, egui::Sense::hover());
    ui.painter().rect(
        surface_rect,
        egui::Rounding::same(SOFT_RADIUS),
        Color32::from_rgb(252, 253, 255),
        Stroke::new(1.0, LINE),
    );

    let inner_rect = surface_rect.shrink(8.0);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(inner_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
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

            let highlights = app.highlight_words();

            if app.receive_filter.trim().is_empty() && app.receive_lines.is_empty() {
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

            let output = if app.receive_filter.trim().is_empty() {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(app.receive_is_following())
                    .show_rows(
                        ui,
                        LOG_ROW_HEIGHT,
                        app.receive_lines.len(),
                        |ui, row_range| {
                            for index in row_range {
                                render_log_row(
                                    ui,
                                    app,
                                    &app.receive_lines[index],
                                    index,
                                    &highlights,
                                );
                            }

                            if app.receive_is_following() && !user_requested_history {
                                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            }
                        },
                    )
            } else {
                let filtered = app.filtered_receive_records();
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
                    .stick_to_bottom(app.receive_is_following())
                    .show_rows(ui, LOG_ROW_HEIGHT, filtered.len(), |ui, row_range| {
                        for index in row_range {
                            render_log_row(ui, app, filtered[index], index, &highlights);
                        }

                        if app.receive_is_following() && !user_requested_history {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    })
            };

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
        },
    );
}

fn render_log_row(
    ui: &mut egui::Ui,
    app: &SerialToolApp,
    record: &ReceiveRecord,
    index: usize,
    highlights: &[String],
) {
    let content = receive_display_text(app.receive_mode, &record.data);
    let content_lower = content.to_lowercase();
    let has_highlight = highlights
        .iter()
        .any(|keyword| content_lower.contains(&keyword.to_lowercase()));
    let row_fill = if has_highlight {
        Color32::from_rgb(255, 246, 221)
    } else if index.is_multiple_of(2) {
        Color32::from_rgb(250, 252, 255)
    } else {
        Color32::from_rgb(246, 249, 253)
    };

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), LOG_ROW_HEIGHT),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Frame::none()
                .fill(row_fill)
                .stroke(Stroke::new(1.0, Color32::from_rgb(228, 236, 244)))
                .rounding(egui::Rounding::same(ROW_RADIUS))
                .inner_margin(egui::Margin::symmetric(10.0, LOG_ROW_VERTICAL_MARGIN))
                .show(ui, |ui| {
                    ui.set_min_height((LOG_ROW_HEIGHT - LOG_ROW_VERTICAL_MARGIN * 2.0).max(0.0));
                    ui.horizontal(|ui| {
                        if app.show_timestamps {
                            ui.label(
                                RichText::new(format!("[{}]", record.timestamp))
                                    .monospace()
                                    .color(ACCENT),
                            );
                        }
                        ui.add(
                            egui::Label::new(
                                RichText::new(content)
                                    .text_style(mono_text_style())
                                    .color(INK),
                            )
                            .truncate(),
                        );
                    });
                });
        },
    );
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
            .layout(
                button_text.clone(),
                font_id,
                Color32::from_rgb(66, 112, 168),
                f32::INFINITY,
            )
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
