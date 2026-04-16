#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;
mod parser;
mod serial;
mod ui;

use std::fs;

use app::SerialToolApp;
use config::AppConfig;
use eframe::egui;
use egui::{FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query};

fn main() -> eframe::Result<()> {
    let config = AppConfig::load_or_default();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([960.0, 640.0])
        .with_title("Serial Scope");

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "serial-scope",
        options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);
            configure_theme(&cc.egui_ctx);
            Ok(Box::new(SerialToolApp::new(config)))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    if let Some((font_name, font_data)) = load_system_cjk_font() {
        fonts.font_data.insert(font_name.clone(), font_data);

        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, font_name.clone());

        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push(font_name);
    }

    ctx.set_fonts(fonts);
}

fn load_system_cjk_font() -> Option<(String, FontData)> {
    let mut db = Database::new();
    db.load_system_fonts();

    let preferred_families = [
        "Microsoft YaHei UI",
        "Microsoft YaHei",
        "Noto Sans CJK SC",
        "Noto Sans SC",
        "WenQuanYi Micro Hei",
        "Source Han Sans SC",
        "SimHei",
    ];

    for family_name in preferred_families {
        let family = Family::Name(family_name);
        let query = Query {
            families: std::slice::from_ref(&family),
            ..Query::default()
        };

        if let Some(id) = db.query(&query) {
            let face = db.face(id)?;
            if let Some(font_bytes) = db.with_face_data(id, |data, _| data.to_vec()) {
                let name = format!("system-font-{}", face.post_script_name.clone());
                return Some((name, FontData::from_owned(font_bytes)));
            }
        }
    }

    let fallback_family = Family::SansSerif;
    let fallback_query = Query {
        families: std::slice::from_ref(&fallback_family),
        ..Query::default()
    };

    if let Some(id) = db.query(&fallback_query) {
        let face = db.face(id)?;
        if let Some(font_bytes) = db.with_face_data(id, |data, _| data.to_vec()) {
            let name = format!("system-font-{}", face.post_script_name.clone());
            return Some((name, FontData::from_owned(font_bytes)));
        }
    }

    None
}

fn configure_theme(ctx: &egui::Context) {
    let ink = egui::Color32::from_rgb(47, 55, 65);
    let muted = egui::Color32::from_rgb(104, 114, 126);
    let accent = egui::Color32::from_rgb(92, 138, 196);
    let accent_soft = egui::Color32::from_rgb(214, 229, 246);
    let line = egui::Color32::from_rgb(214, 220, 228);

    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(ink);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent_soft;
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(246, 244, 239);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, line);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, muted);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(252, 251, 248);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, line);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(240, 244, 249);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(72, 116, 172));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.faint_bg_color = egui::Color32::from_rgb(239, 236, 231);
    visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
    visuals.code_bg_color = egui::Color32::from_rgb(248, 246, 242);
    visuals.panel_fill = egui::Color32::from_rgb(243, 241, 236);
    visuals.window_fill = egui::Color32::from_rgb(246, 244, 240);
    visuals.window_stroke = egui::Stroke::new(1.0, line);
    visuals.warn_fg_color = egui::Color32::from_rgb(184, 120, 46);
    visuals.error_fg_color = egui::Color32::from_rgb(196, 92, 92);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.indent = 18.0;
    style.spacing.combo_width = 140.0;
    ctx.set_style(style);
}

fn load_app_icon() -> Option<egui::IconData> {
    let bytes = fs::read("assets/app-icon.png").ok()?;
    let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();

    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
