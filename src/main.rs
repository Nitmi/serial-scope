#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;
mod parser;
mod serial;
mod ui;
mod update;

use app::SerialToolApp;
use config::AppConfig;
use eframe::egui;
use egui::{FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query};

fn main() -> eframe::Result<()> {
    let config = AppConfig::load_or_default();
    let viewport_placement = initial_viewport_placement();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(viewport_placement.inner_size)
        .with_min_inner_size([960.0, 640.0])
        .with_clamp_size_to_monitor_size(true)
        .with_title("Serial Scope");

    if let Some(position) = viewport_placement.position {
        viewport = viewport.with_position(position);
    }

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
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

struct ViewportPlacement {
    inner_size: [f32; 2],
    position: Option<[f32; 2]>,
}

fn initial_viewport_placement() -> ViewportPlacement {
    #[cfg(target_os = "windows")]
    if let Some(placement) = windows_work_area_placement([1280.0, 800.0]) {
        return placement;
    }

    ViewportPlacement {
        inner_size: [1280.0, 800.0],
        position: None,
    }
}

#[cfg(target_os = "windows")]
fn windows_work_area_placement(desired_size: [f32; 2]) -> Option<ViewportPlacement> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor) == 0 {
            return None;
        }

        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }

        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if GetMonitorInfoW(monitor, &mut monitor_info as *mut MONITORINFO) == 0 {
            return None;
        }

        let work_area = monitor_info.rcWork;
        let work_width = (work_area.right - work_area.left).max(1) as f32;
        let work_height = (work_area.bottom - work_area.top).max(1) as f32;

        let inner_width = desired_size[0].min(work_width);
        let inner_height = desired_size[1].min(work_height);

        let x = work_area.left as f32 + (work_width - inner_width) * 0.5;
        let y = work_area.top as f32 + (work_height - inner_height) * 0.5;

        Some(ViewportPlacement {
            inner_size: [inner_width, inner_height],
            position: Some([x, y]),
        })
    }
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
    let muted = egui::Color32::from_rgb(95, 110, 126);
    let accent = egui::Color32::from_rgb(84, 136, 202);
    let accent_soft = egui::Color32::from_rgb(219, 235, 250);
    let line = egui::Color32::from_rgb(205, 216, 229);

    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(ink);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent_soft;
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(244, 248, 253);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, line);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, muted);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(250, 252, 255);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, line);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 245, 252);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(72, 116, 172));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.faint_bg_color = egui::Color32::from_rgb(236, 242, 248);
    visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
    visuals.code_bg_color = egui::Color32::from_rgb(248, 250, 253);
    visuals.panel_fill = egui::Color32::from_rgb(244, 248, 253);
    visuals.window_fill = egui::Color32::from_rgb(245, 249, 254);
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
    let bytes = include_bytes!("../assets/app-icon.png");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();

    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
