//! Modal settings window: theme, global limit, display toggles.

use egui::{Context, RichText, Ui};

use crate::gui::format::compact;
use crate::gui::prefs::{ThemeMode, MAX_LIMIT, MIN_LIMIT};
use crate::gui::state::AppState;
use crate::gui::theme::{self, Palette};

const LIMIT_PRESETS: [(&str, u64); 5] = [
    ("10M", 10_000_000),
    ("100M", 100_000_000),
    ("1B", 1_000_000_000),
    ("10B", 10_000_000_000),
    ("100B", 100_000_000_000),
];

pub fn render(ctx: &Context, state: &mut AppState, palette: &Palette) {
    let mut open = state.prefs.show_settings;
    egui::Window::new("⚙ JumpChamp Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size(egui::vec2(420.0, 315.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);
            section(ui, palette, "Theme Mode", |ui| theme_choice(ui, &mut state.prefs.theme));
            ui.add_space(6.0);
            let busy = state.playback.is_busy();
            ui.add_enabled_ui(!busy, |ui| {
                section(ui, palette, "Global Numerical Limits", |ui| limits(ui, state, busy));
            });
            ui.add_space(6.0);
            section(ui, palette, "Display & Chart Preferences", |ui| toggles(ui, state));
        });
    state.prefs.show_settings = open;
}

fn section(ui: &mut Ui, palette: &Palette, title: &str, body: impl FnOnce(&mut Ui)) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong().color(palette.accent));
        ui.add_space(4.0);
        body(ui);
    });
}

fn theme_choice(ui: &mut Ui, mode: &mut ThemeMode) {
    ui.horizontal(|ui| {
        ui.radio_value(mode, ThemeMode::Light, "Light Mode");
        ui.add_space(12.0);
        ui.radio_value(mode, ThemeMode::Dark, "Dark Mode");
    });
}

fn limits(ui: &mut Ui, state: &mut AppState, locked: bool) {
    if locked {
        ui.label(RichText::new("🔒 Locked while animation is running").small().italics().color(theme::LOCKED));
    }
    ui.horizontal(|ui| {
        ui.label("Max Prime Index Limit (n):");
        let mut limit = state.prefs.max_prime_limit;
        if ui.add(egui::DragValue::new(&mut limit).speed(10_000_000).range(MIN_LIMIT..=MAX_LIMIT)).changed() {
            state.set_max_prime_limit(limit);
        }
        ui.label(format!("({})", compact(state.prefs.max_prime_limit)));
    });
    ui.horizontal(|ui| {
        ui.label("Quick Presets:");
        for (label, limit) in LIMIT_PRESETS {
            if ui.button(label).clicked() {
                state.set_max_prime_limit(limit);
            }
        }
    });
}

fn toggles(ui: &mut Ui, state: &mut AppState) {
    let p = &mut state.prefs;
    ui.checkbox(&mut p.show_pct_labels, "Show Percentage Annotations on Bars");
    ui.checkbox(&mut p.show_grid_lines, "Show Reference Grid Lines");
    ui.checkbox(&mut p.show_heatmap_meter, "Show Heat Map Count Meter (Top-Right)");
    ui.checkbox(&mut p.show_bar_tooltip, "Show Cursor Hover Details Tooltip");
}
