use egui::{Color32, Visuals};
use crate::gui::state::ThemeMode;

pub fn is_dark(mode: ThemeMode) -> bool {
    mode == ThemeMode::Dark
}

pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    let mut style = (*ctx.style()).clone();
    match mode {
        ThemeMode::Dark => {
            style.visuals = Visuals::dark();
            style.visuals.window_fill = Color32::from_rgb(14, 17, 23);
            style.visuals.panel_fill = Color32::from_rgb(26, 30, 38);
            style.visuals.override_text_color = Some(Color32::from_rgb(220, 225, 235));
        }
        ThemeMode::Light => {
            style.visuals = Visuals::light();
            style.visuals.window_fill = Color32::from_rgb(245, 247, 250);
            style.visuals.panel_fill = Color32::from_rgb(255, 255, 255);
            style.visuals.override_text_color = Some(Color32::from_rgb(24, 28, 36));
        }
    }
    ctx.set_style(style);
}

pub fn text_primary(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(220, 225, 235)
    } else {
        Color32::from_rgb(24, 28, 36)
    }
}

pub fn text_secondary(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(180, 190, 210)
    } else {
        Color32::from_rgb(80, 90, 110)
    }
}

pub fn card_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgba_unmultiplied(16, 20, 28, 230)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 240)
    }
}

pub fn card_border(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(60, 70, 90)
    } else {
        Color32::from_rgb(190, 200, 215)
    }
}

pub fn grid_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 18)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 25)
    }
}

pub fn baseline_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 40)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 60)
    }
}

pub fn accent_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(90, 200, 250)
    } else {
        Color32::from_rgb(0, 120, 210)
    }
}

pub fn slider_rail_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(45, 55, 75)
    } else {
        Color32::from_rgb(210, 218, 230)
    }
}

pub fn viridis_color(t: f64) -> Color32 {
    let palette = [
        (68u8, 1, 84),
        (59, 82, 139),
        (33, 145, 140),
        (94, 201, 98),
        (253, 231, 37),
    ];
    let t_clamped = t.clamp(0.0, 1.0);
    let idx = (t_clamped * (palette.len() - 1) as f64).floor() as usize;
    let idx = idx.min(palette.len() - 2);
    let frac = t_clamped * (palette.len() - 1) as f64 - idx as f64;
    let (r1, g1, b1) = palette[idx];
    let (r2, g2, b2) = palette[idx + 1];
    let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * frac) as u8;
    Color32::from_rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
}
