use eframe::egui::{self, vec2, Color32, Stroke, UiBuilder};

pub const MAIN_PANEL_ALIGNMENT_TRIM: f32 = 0.0;
pub const MAIN_PANEL_INNER_MARGIN: egui::Margin = egui::Margin {
    left: 14.0,
    right: 14.0,
    top: 12.0,
    bottom: 10.0,
};
pub const MAIN_PANEL_OUTER_MARGIN: egui::Margin = egui::Margin {
    left: 0.0,
    right: 0.0,
    top: 2.0,
    bottom: 2.0,
};
pub const MAIN_CONTENT_BOTTOM_INSET: f32 = 4.0;

const MAIN_PANEL_LINE: Color32 = Color32::from_rgb(208, 218, 230);
const MAIN_PANEL_FILL: Color32 = Color32::from_rgb(255, 255, 255);
const MAIN_PANEL_RADIUS: f32 = 12.0;
pub fn main_content_size(ui: &egui::Ui) -> egui::Vec2 {
    egui::vec2(
        ui.available_width().max(0.0),
        (ui.available_height() - MAIN_CONTENT_BOTTOM_INSET).max(0.0),
    )
}

pub fn show_main_panel(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let outer_size = ui.available_size();
    let (reserved_rect, _) = ui.allocate_exact_size(outer_size, egui::Sense::hover());

    let frame_rect = egui::Rect::from_min_max(
        reserved_rect.min + vec2(MAIN_PANEL_OUTER_MARGIN.left, MAIN_PANEL_OUTER_MARGIN.top),
        reserved_rect.max
            - vec2(
                MAIN_PANEL_OUTER_MARGIN.right,
                MAIN_PANEL_OUTER_MARGIN.bottom,
            ),
    );

    ui.painter().rect(
        frame_rect,
        egui::Rounding::same(MAIN_PANEL_RADIUS),
        MAIN_PANEL_FILL,
        Stroke::new(1.0, MAIN_PANEL_LINE),
    );

    let inner_rect = egui::Rect::from_min_max(
        frame_rect.min + vec2(MAIN_PANEL_INNER_MARGIN.left, MAIN_PANEL_INNER_MARGIN.top),
        frame_rect.max
            - vec2(
                MAIN_PANEL_INNER_MARGIN.right,
                MAIN_PANEL_INNER_MARGIN.bottom,
            ),
    );

    ui.scope_builder(
        UiBuilder::new()
            .max_rect(inner_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        add_contents,
    );
}
