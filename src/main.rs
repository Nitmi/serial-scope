mod app;
mod config;
mod parser;
mod serial;
mod ui;

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
        .with_title("Serial Tool");

    if let Some(icon) = default_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "serial_tool",
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
                return Some((name, FontData::from_owned(font_bytes).into()));
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
            return Some((name, FontData::from_owned(font_bytes).into()));
        }
    }

    None
}

fn configure_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(225, 229, 235));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(45, 95, 155);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 110, 178);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(36, 40, 48);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(24, 27, 33);
    visuals.panel_fill = egui::Color32::from_rgb(18, 21, 26);
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 15, 20);
    visuals.window_fill = egui::Color32::from_rgb(20, 24, 30);
    ctx.set_visuals(visuals);
}

fn default_icon() -> Option<egui::IconData> {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let inside = x > 3 && x < 28 && y > 5 && y < 26;
            let accent = (x + y) % 7 == 0;
            let (r, g, b, a) = if inside {
                if accent {
                    (92, 178, 255, 255)
                } else {
                    (32, 128, 216, 255)
                }
            } else {
                (0, 0, 0, 0)
            };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }

    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}
