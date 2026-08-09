// ============================================================================
// Custom Theme and Styling Constants for JumpChamp GUI
// ============================================================================

use egui::{Color32, Visuals};

pub const COLOR_ACCENT: Color32 = Color32::from_rgb(68, 1, 84);
pub const COLOR_BG: Color32 = Color32::from_rgb(14, 17, 23);
pub const COLOR_SURFACE: Color32 = Color32::from_rgb(26, 30, 38);
pub const COLOR_TEXT: Color32 = Color32::from_rgb(220, 225, 235);

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = Visuals::dark();
    style.visuals.window_fill = COLOR_BG;
    style.visuals.panel_fill = COLOR_SURFACE;
    style.visuals.override_text_color = Some(COLOR_TEXT);

    ctx.set_style(style);
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
