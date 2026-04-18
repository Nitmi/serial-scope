use eframe::egui::{self, vec2, Color32, RichText, Slider, Stroke};
use egui_plot::{Legend, Line, Plot, PlotBounds, PlotPoints};

use crate::app::{preview_text_line, MainView, SerialToolApp};

const RESIZE_HANDLE_WIDTH: f32 = 12.0;
const CONTENT_EDGE_SAFETY: f32 = 2.0;
const INK: Color32 = Color32::from_rgb(48, 56, 66);
const MUTED: Color32 = Color32::from_rgb(112, 120, 130);
const LINE: Color32 = Color32::from_rgb(208, 218, 230);

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    let frame_vertical_padding = 24.0;
    let panel_content_height = (ui.available_height() - frame_vertical_padding).max(0.0);
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(255, 255, 255))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .outer_margin(egui::Margin::symmetric(0.0, 2.0))
        .show(ui, |ui| {
            ui.set_height(panel_content_height);
            ui.horizontal_wrapped(|ui| {
                ui.heading(RichText::new("实时曲线").color(INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    view_switch(ui, app);
                });
                if ui.button(app.chart_state.paused_label()).clicked() {
                    app.chart_state.toggle_pause();
                }
                if ui.button("清空全部").clicked() {
                    app.clear_plot();
                }
                if ui.button("导出 CSV").clicked() {
                    app.export_plot_csv();
                }
                ui.separator();
                ui.label(
                    RichText::new(format!("曲线数: {}", app.chart_state.series.len())).strong(),
                );
                ui.label(
                    RichText::new("支持 CSV 数字行与 key=value 数字行解析")
                        .italics()
                        .color(MUTED),
                );
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(app.chart_state.latest_points_summary())
                    .color(Color32::from_rgb(120, 172, 255)),
            );
            ui.label(
                RichText::new(app.chart_state.schema_status_text())
                    .small()
                    .color(Color32::from_rgb(255, 196, 120)),
            );
            if let Some(record) = app.receive_lines.back() {
                if let Some(preview) = preview_text_line(&record.data) {
                    ui.label(
                        RichText::new(format!("最近文本行: {preview}"))
                            .small()
                            .color(MUTED),
                    );
                }
            }
            if app.chart_state.series.is_empty() {
                ui.label(
                    RichText::new("示例输入: 1.23,4.56,7.89 或 flag=143,key=1 都会自动生成曲线。")
                        .color(Color32::from_rgb(255, 196, 120)),
                );
            }

            ui.add_space(6.0);
            let content_height = ui.available_height();
            let available_size = egui::vec2(ui.available_width(), content_height);
            let min_plot_width = 220.0;
            let max_sidebar_width =
                (available_size.x - min_plot_width - RESIZE_HANDLE_WIDTH - CONTENT_EDGE_SAFETY)
                    .max(0.0);
            let sidebar_width = app
                .chart_state
                .effective_sidebar_width(available_size.x)
                .min(max_sidebar_width);
            let plot_height = available_size.y.max(0.0);
            let plot_width =
                (available_size.x - sidebar_width - RESIZE_HANDLE_WIDTH - CONTENT_EDGE_SAFETY)
                    .max(0.0);

            ui.allocate_ui_with_layout(
                available_size,
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    ui.allocate_ui_with_layout(
                        egui::vec2(plot_width, plot_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let plot = Plot::new("serial_plot")
                                .legend(Legend::default())
                                .allow_scroll(true)
                                .allow_zoom(true)
                                .allow_drag(true)
                                .include_y(0.0)
                                .width(plot_width)
                                .height(plot_height);

                            let anticipated_plot_rect = egui::Rect::from_min_size(
                                ui.available_rect_before_wrap().min,
                                vec2(plot_width, plot_height),
                            );
                            let user_requested_history = app.chart_state.is_following()
                                && plot_history_navigation_requested(ui.ctx(), anticipated_plot_rect);
                            if user_requested_history {
                                app.chart_state.pause_auto_follow();
                            }

                            let plot_response = plot.show(ui, |plot_ui| {
                                if app.chart_state.is_following() {
                                    if let Some((min_x, max_x)) = app.chart_state.x_bounds() {
                                        let (min_y, max_y) =
                                            app.chart_state.y_bounds().unwrap_or((-1.0, 1.0));
                                        plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                                            [min_x, min_y],
                                            [max_x, max_y],
                                        ));
                                    }
                                    app.chart_state.sync_zoom_tracking();
                                } else if let Some(bounds) =
                                    app.chart_state.manual_zoomed_bounds(plot_ui.plot_bounds())
                                {
                                    plot_ui.set_plot_bounds(bounds);
                                }

                                for key in app.chart_state.visible_series_keys() {
                                    if let Some(values) = app.chart_state.series.get(&key) {
                                        let points = PlotPoints::from_iter(
                                            values.iter().map(|p| [p[0], p[1]]),
                                        );
                                        plot_ui.line(
                                            Line::new(points)
                                                .name(app.chart_state.display_name(&key))
                                                .color(series_color(&key)),
                                        );
                                    }
                                }
                            });

                            if plot_interaction_changed_bounds(ui.ctx(), &plot_response.response) {
                                app.chart_state.pause_auto_follow();
                            }

                            if app.chart_state.is_manual()
                                && show_plot_follow_resume_button(
                                    ui.ctx(),
                                    plot_response.response.rect,
                                )
                            {
                                app.chart_state.resume_auto_follow();
                            }
                        },
                    );

                    let (handle_rect, handle_response) = ui.allocate_exact_size(
                        vec2(RESIZE_HANDLE_WIDTH, plot_height),
                        egui::Sense::click_and_drag(),
                    );
                    let handle_color = if handle_response.dragged() {
                        Color32::from_rgb(118, 150, 196)
                    } else if handle_response.hovered() {
                        Color32::from_rgb(176, 186, 202)
                    } else {
                        Color32::from_rgb(214, 220, 228)
                    };
                    ui.painter().line_segment(
                        [
                            handle_rect.center_top() + vec2(0.0, 20.0),
                            handle_rect.center_bottom() - vec2(0.0, 20.0),
                        ],
                        Stroke::new(if handle_response.dragged() { 2.0 } else { 1.0 }, handle_color),
                    );
                    if handle_response.hovered() || handle_response.dragged() {
                        ui.painter()
                            .circle_filled(handle_rect.center(), 3.0, handle_color);
                    }

                    if handle_response.dragged() {
                        let drag_delta_x = ui.input(|input| input.pointer.delta().x);
                        if drag_delta_x.abs() > f32::EPSILON {
                            app.chart_state
                                .set_manual_sidebar_width(sidebar_width - drag_delta_x);
                            app.persist_config();
                        }
                    }

                    ui.allocate_ui_with_layout(
                        egui::vec2(sidebar_width, plot_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                        ui.horizontal(|ui| {
                            ui.heading("曲线面板");
                        });
                        ui.label(
                            RichText::new("拖动中间分隔线可调整宽度，X/Y 缩放互相独立。").small(),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            Slider::new(&mut app.chart_state.x_zoom, 0.2..=5.0).text("X 轴缩放"),
                        );
                        ui.add(
                            Slider::new(&mut app.chart_state.y_zoom, 0.2..=5.0).text("Y 轴缩放"),
                        );
                        ui.add_space(8.0);
                        let sidebar_panel_height = ui.available_height().max(0.0);

                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), sidebar_panel_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgb(252, 253, 255))
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(209, 217, 226)))
                                    .show(ui, |ui| {
                                        ui.set_height(sidebar_panel_height);
                                        ui.set_width(ui.available_width());
                                        egui::ScrollArea::both()
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                let names = app
                                                    .chart_state
                                                    .series
                                                    .keys()
                                                    .cloned()
                                                    .collect::<Vec<_>>();

                                                if names.is_empty() {
                                                    ui.label("暂无数据");
                                                    return;
                                                }

                                                egui::Grid::new("plot_series_stats")
                                                    .num_columns(6)
                                                    .spacing([10.0, 8.0])
                                                    .striped(true)
                                                    .show(ui, |ui| {
                                                        ui.label("");
                                                        ui.label(RichText::new("显示").strong());
                                                        ui.label(RichText::new("最小值").strong());
                                                        ui.label(RichText::new("最大值").strong());
                                                        ui.label(RichText::new("当前值").strong());
                                                        ui.label(RichText::new("操作").strong());
                                                        ui.end_row();

                                                        for name in names {
                                                            let color = series_color(&name);
                                                            let mut visible = app
                                                                .chart_state
                                                                .visible
                                                                .contains(&name);
                                                            let stats = app
                                                                .chart_state
                                                                .series
                                                                .get(&name)
                                                                .map(|values| {
                                                                    let mut min_value =
                                                                        f64::INFINITY;
                                                                    let mut max_value =
                                                                        f64::NEG_INFINITY;
                                                                    for point in values {
                                                                        min_value = min_value.min(point[1]);
                                                                        max_value = max_value.max(point[1]);
                                                                    }
                                                                    let current = values
                                                                        .back()
                                                                        .map(|point| point[1])
                                                                        .unwrap_or(0.0);
                                                                    (min_value, max_value, current)
                                                                })
                                                                .unwrap_or((0.0, 0.0, 0.0));

                                                            ui.horizontal(|ui| {
                                                                let (rect, _) =
                                                                    ui.allocate_exact_size(
                                                                        vec2(10.0, 10.0),
                                                                        egui::Sense::hover(),
                                                                    );
                                                                ui.painter().circle_filled(
                                                                    rect.center(),
                                                                    5.0,
                                                                    color,
                                                                );
                                                                if app
                                                                    .chart_state
                                                                    .is_renaming_series(&name)
                                                                {
                                                                    let mut commit = false;
                                                                    let mut cancel = false;

                                                                    if let Some(editing_name) = app
                                                                        .chart_state
                                                                        .renaming_series_name_mut()
                                                                    {
                                                                        let response = ui.add(
                                                                            egui::TextEdit::singleline(
                                                                                editing_name,
                                                                            )
                                                                            .desired_width(120.0),
                                                                        );
                                                                        commit = response.lost_focus()
                                                                            && ui.input(|input| {
                                                                                input.key_pressed(
                                                                                    egui::Key::Enter,
                                                                                )
                                                                            });
                                                                        cancel = ui.input(|input| {
                                                                            input.key_pressed(
                                                                                egui::Key::Escape,
                                                                            )
                                                                        });
                                                                    }

                                                                    if ui.small_button("确定").clicked() {
                                                                        commit = true;
                                                                    }
                                                                    if ui.small_button("取消").clicked() {
                                                                        cancel = true;
                                                                    }

                                                                    if commit {
                                                                        app.chart_state
                                                                            .commit_series_renaming();
                                                                    } else if cancel {
                                                                        app.chart_state
                                                                            .cancel_series_renaming();
                                                                    }
                                                                } else {
                                                                    let label = ui.add(
                                                                        egui::Label::new(
                                                                            RichText::new(
                                                                                app.chart_state.display_name(
                                                                                    &name,
                                                                                ),
                                                                            )
                                                                            .strong()
                                                                            .underline(),
                                                                        )
                                                                        .sense(egui::Sense::click()),
                                                                    );
                                                                    if label.clicked() {
                                                                        app.chart_state
                                                                            .start_series_renaming(
                                                                                &name,
                                                                            );
                                                                    }
                                                                }
                                                            });

                                                            if ui.checkbox(&mut visible, "").changed()
                                                            {
                                                                if visible {
                                                                    app.chart_state
                                                                        .visible
                                                                        .insert(name.clone());
                                                                } else {
                                                                    app.chart_state
                                                                        .visible
                                                                        .remove(&name);
                                                                }
                                                            }

                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "{:.3}",
                                                                    stats.0
                                                                ))
                                                                .monospace(),
                                                            );
                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "{:.3}",
                                                                    stats.1
                                                                ))
                                                                .monospace(),
                                                            );
                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "{:.3}",
                                                                    stats.2
                                                                ))
                                                                .monospace()
                                                                .color(color),
                                                            );

                                                            if ui.button("清除").clicked() {
                                                                app.chart_state.clear_series(&name);
                                                            }
                                                            ui.end_row();
                                                        }
                                                    });
                                            });
                                    });
                            },
                        );
                    },
                    );
                },
            );

            if app.chart_state.is_paused() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("绘图已暂停，新的解析数据不会追加到曲线。")
                        .color(Color32::from_rgb(255, 196, 120)),
                );
            }
        });
}

fn plot_history_navigation_requested(ctx: &egui::Context, plot_rect: egui::Rect) -> bool {
    let pointer_over_plot = ctx
        .pointer_hover_pos()
        .is_some_and(|pos| plot_rect.contains(pos));
    pointer_over_plot
        && ctx.input(|input| {
            input.raw_scroll_delta.y.abs() > 0.0
                || input.raw_scroll_delta.x.abs() > 0.0
                || input.smooth_scroll_delta.y.abs() > 0.0
                || input.smooth_scroll_delta.x.abs() > 0.0
                || (input.pointer.primary_down() && input.pointer.delta().length_sq() > 0.0)
        })
}

fn plot_interaction_changed_bounds(ctx: &egui::Context, response: &egui::Response) -> bool {
    response.dragged()
        || (response.hovered()
            && ctx.input(|input| {
                input.raw_scroll_delta.y.abs() > 0.0
                    || input.raw_scroll_delta.x.abs() > 0.0
                    || input.smooth_scroll_delta.y.abs() > 0.0
                    || input.smooth_scroll_delta.x.abs() > 0.0
            }))
}

fn show_plot_follow_resume_button(ctx: &egui::Context, anchor_rect: egui::Rect) -> bool {
    let button_text = "回到最新视图";
    let button_padding = egui::vec2(10.0, 5.0);
    let font_id = egui::TextStyle::Button.resolve(&ctx.style());
    let text_width = ctx.fonts(|fonts| {
        fonts
            .layout(
                button_text.to_owned(),
                font_id,
                Color32::from_rgb(66, 112, 168),
                f32::INFINITY,
            )
            .size()
            .x
    });
    let estimated_width = (text_width + button_padding.x * 2.0).max(104.0);
    let area_pos = egui::pos2(
        anchor_rect.max.x - estimated_width - 8.0,
        anchor_rect.max.y - 40.0,
    );
    let mut clicked = false;

    egui::Area::new(egui::Id::new("plot_follow_resume_button"))
        .order(egui::Order::Foreground)
        .fixed_pos(area_pos)
        .show(ctx, |ui| {
            ui.spacing_mut().button_padding = button_padding;
            if ui
                .add_sized(
                    egui::vec2(estimated_width, 28.0),
                    egui::Button::new(RichText::new(button_text).color(Color32::from_rgb(66, 112, 168)))
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

fn view_switch(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::none()
        .fill(Color32::from_rgb(240, 246, 252))
        .stroke(Stroke::new(1.0, Color32::from_rgb(209, 217, 226)))
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.active_view, MainView::Plot, "数据绘图");
                ui.selectable_value(&mut app.active_view, MainView::Monitor, "串口监视");
            });
        });
}

fn series_color(name: &str) -> Color32 {
    const PALETTE: [Color32; 8] = [
        Color32::from_rgb(230, 92, 92),
        Color32::from_rgb(82, 137, 230),
        Color32::from_rgb(165, 204, 84),
        Color32::from_rgb(245, 166, 35),
        Color32::from_rgb(66, 196, 181),
        Color32::from_rgb(190, 120, 230),
        Color32::from_rgb(240, 110, 170),
        Color32::from_rgb(130, 210, 255),
    ];

    let mut hash = 0usize;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
    }
    PALETTE[hash % PALETTE.len()]
}
