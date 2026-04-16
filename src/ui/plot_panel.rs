use eframe::egui::{self, vec2, Color32, DragValue, RichText, Slider};
use egui_plot::{Legend, Line, Plot, PlotBounds, PlotPoints};

use crate::app::{preview_text_line, SerialToolApp};

pub fn show(ui: &mut egui::Ui, app: &mut SerialToolApp) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_height(260.0);
        ui.horizontal_wrapped(|ui| {
            ui.heading("实时曲线");
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
            ui.checkbox(&mut app.chart_state.auto_follow, "自动定位");
            ui.separator();
            ui.label(RichText::new(format!("通道数: {}", app.chart_state.series.len())).strong());
            ui.label(RichText::new("支持 CSV 数字行与 key=value 数字行解析").italics());
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
                ui.label(RichText::new(format!("最近文本行: {preview}")).small());
            }
        }
        if app.chart_state.series.is_empty() {
            ui.label(
                RichText::new("示例输入: 1.23,4.56,7.89 或 flag=143,key=1 都会自动生成曲线。")
                    .color(Color32::from_rgb(255, 196, 120)),
            );
        }

        ui.add_space(6.0);
        let available_size = ui.available_size();
        let sidebar_width = app.chart_state.sidebar_width.clamp(240.0, 460.0);
        app.chart_state.sidebar_width = sidebar_width;

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                let plot_width = (available_size.x - sidebar_width - 30.0).max(520.0);
                let plot_height = available_size.y.max(520.0);
                ui.set_width(plot_width);

                let plot = Plot::new("serial_plot")
                    .legend(Legend::default())
                    .allow_scroll(true)
                    .allow_zoom(true)
                    .allow_drag(true)
                    .include_y(0.0)
                    .width(plot_width)
                    .height(plot_height);

                plot.show(ui, |plot_ui| {
                    if app.chart_state.auto_follow {
                        if let Some((min_x, max_x)) = app.chart_state.x_bounds() {
                            let (min_y, max_y) = app.chart_state.y_bounds().unwrap_or((-1.0, 1.0));
                            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                                [min_x, min_y],
                                [max_x, max_y],
                            ));
                        }
                    }

                    for name in app.chart_state.visible_series_names() {
                        if let Some(values) = app.chart_state.series.get(&name) {
                            let points = PlotPoints::from_iter(values.iter().map(|p| [p[0], p[1]]));
                            plot_ui.line(
                                Line::new(points)
                                    .name(name.clone())
                                    .color(series_color(&name)),
                            );
                        }
                    }
                });
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.set_width(sidebar_width);
                ui.horizontal(|ui| {
                    ui.heading("曲线面板");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            DragValue::new(&mut app.chart_state.sidebar_width)
                                .prefix("宽度 ")
                                .speed(1.0)
                                .range(240.0..=460.0),
                        );
                    });
                });
                ui.label(RichText::new("X/Y 缩放互相独立，可单独隐藏或清空曲线。").small());
                ui.add_space(4.0);
                ui.add(Slider::new(&mut app.chart_state.x_zoom, 0.2..=5.0).text("X 轴缩放"));
                ui.add(Slider::new(&mut app.chart_state.y_zoom, 0.2..=5.0).text("Y 轴缩放"));
                ui.add_space(8.0);

                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_min_height(available_size.y.max(520.0));
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let names = app.chart_state.series.keys().cloned().collect::<Vec<_>>();

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
                                        let mut visible = app.chart_state.visible.contains(&name);
                                        let stats = app
                                            .chart_state
                                            .series
                                            .get(&name)
                                            .map(|values| {
                                                let mut min_value = f64::INFINITY;
                                                let mut max_value = f64::NEG_INFINITY;
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
                                            let (rect, _) = ui.allocate_exact_size(
                                                vec2(10.0, 10.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(rect.center(), 5.0, color);
                                            ui.label(RichText::new(name.clone()).strong());
                                        });

                                        if ui.checkbox(&mut visible, "").changed() {
                                            if visible {
                                                app.chart_state.visible.insert(name.clone());
                                            } else {
                                                app.chart_state.visible.remove(&name);
                                            }
                                        }

                                        ui.label(
                                            RichText::new(format!("{:.3}", stats.0)).monospace(),
                                        );
                                        ui.label(
                                            RichText::new(format!("{:.3}", stats.1)).monospace(),
                                        );
                                        ui.label(
                                            RichText::new(format!("{:.3}", stats.2))
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
            });
        });

        if app.chart_state.is_paused() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("绘图已暂停，新的解析数据不会追加到曲线。")
                    .color(Color32::from_rgb(255, 196, 120)),
            );
        }
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
