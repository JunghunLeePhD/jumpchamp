//! Colors. `Palette::of(mode)` gives every themed color in one value.

use egui::{Color32, Visuals};

use super::prefs::ThemeMode;

/// All theme-dependent colors.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub grid: Color32,
    pub baseline: Color32,
    pub rail: Color32,
    pub disabled: Color32,
    /// Pinned bar fill.
    pub highlight: Color32,
    /// Rank labels and the rank slider.
    pub rank: Color32,
    pub rank_fill: Color32,
}

/// Fixed status colors (theme independent).
pub const CACHING: Color32 = Color32::from_rgb(255, 170, 0);
pub const PLAYING: Color32 = Color32::from_rgb(0, 180, 220);
pub const LOCKED: Color32 = Color32::from_rgb(220, 150, 0);

impl Palette {
    pub fn of(mode: ThemeMode) -> Self {
        let rgb = Color32::from_rgb;
        let rgba = Color32::from_rgba_unmultiplied;
        match mode {
            ThemeMode::Dark => Self {
                dark: true,
                text: rgb(220, 225, 235),
                text_dim: rgb(180, 190, 210),
                accent: rgb(90, 200, 250),
                card_bg: rgba(16, 20, 28, 230),
                card_border: rgb(60, 70, 90),
                grid: rgba(255, 255, 255, 18),
                baseline: rgba(255, 255, 255, 40),
                rail: rgb(45, 55, 75),
                disabled: rgb(70, 75, 85),
                highlight: rgb(255, 215, 0),
                rank: rgb(255, 200, 80),
                rank_fill: rgb(255, 180, 0),
            },
            ThemeMode::Light => Self {
                dark: false,
                text: rgb(24, 28, 36),
                text_dim: rgb(80, 90, 110),
                accent: rgb(0, 120, 210),
                card_bg: rgba(255, 255, 255, 240),
                card_border: rgb(190, 200, 215),
                grid: rgba(0, 0, 0, 25),
                baseline: rgba(0, 0, 0, 60),
                rail: rgb(210, 218, 230),
                disabled: rgb(180, 185, 195),
                highlight: rgb(230, 160, 0),
                rank: rgb(200, 130, 0),
                rank_fill: rgb(220, 140, 0),
            },
        }
    }
}

/// Applies the window/panel visuals for `mode`.
pub fn apply(ctx: &egui::Context, mode: ThemeMode) {
    let (mut visuals, window, panel) = match mode {
        ThemeMode::Dark => (
            Visuals::dark(),
            Color32::from_rgb(14, 17, 23),
            Color32::from_rgb(26, 30, 38),
        ),
        ThemeMode::Light => (
            Visuals::light(),
            Color32::from_rgb(245, 247, 250),
            Color32::WHITE,
        ),
    };
    visuals.window_fill = window;
    visuals.panel_fill = panel;
    visuals.override_text_color = Some(Palette::of(mode).text);
    ctx.set_visuals(visuals);
}

/// Viridis colormap, `t` in `[0, 1]`.
pub fn viridis(t: f64) -> Color32 {
    const STOPS: [(u8, u8, u8); 5] = [
        (68, 1, 84),
        (59, 82, 139),
        (33, 145, 140),
        (94, 201, 98),
        (253, 231, 37),
    ];
    let x = t.clamp(0.0, 1.0) * (STOPS.len() - 1) as f64;
    let i = (x.floor() as usize).min(STOPS.len() - 2);
    let frac = x - i as f64;
    let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * frac) as u8;
    let ((r1, g1, b1), (r2, g2, b2)) = (STOPS[i], STOPS[i + 1]);
    Color32::from_rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viridis_endpoints() {
        assert_eq!(viridis(0.0), Color32::from_rgb(68, 1, 84));
        assert_eq!(viridis(1.0), Color32::from_rgb(253, 231, 37));
        assert_eq!(viridis(-5.0), viridis(0.0));
    }
}
