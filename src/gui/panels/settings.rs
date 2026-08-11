// ============================================================================
// Modal Settings Window Component
// ============================================================================

use crate::gui::state::{AppState, ThemeMode};
use crate::gui::theme;
use crate::gui::utils::format_compact_num;

pub fn render(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_settings {
        return;
    }

    let is_dark = theme::is_dark(state.theme_mode);
    let accent = theme::accent_color(is_dark);

    let mut is_open = state.show_settings;

    egui::Window::new("⚙ JumpChamp Settings")
        .open(&mut is_open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(420.0, 290.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Theme Mode").strong().color(accent));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.radio_value(&mut state.theme_mode, ThemeMode::Dark, "Dark Mode");
                    ui.add_space(12.0);
                    ui.radio_value(&mut state.theme_mode, ThemeMode::Light, "Light Mode");
                });
            });

            ui.add_space(6.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Global Numerical Limits").strong().color(accent));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Max Prime Index Limit (n):");
                    ui.add(egui::DragValue::new(&mut state.max_prime_limit).speed(10_000_000).range(1_000_000..=100_000_000_000u64));
                    ui.label(format!("({})", format_compact_num(state.max_prime_limit)));
                });

                ui.horizontal(|ui| {
                    ui.label("Quick Presets:");
                    if ui.button("10M").clicked() {
                        state.max_prime_limit = 10_000_000;
                    }
                    if ui.button("100M").clicked() {
                        state.max_prime_limit = 100_000_000;
                    }
                    if ui.button("1B").clicked() {
                        state.max_prime_limit = 1_000_000_000;
                    }
                    if ui.button("10B").clicked() {
                        state.max_prime_limit = 10_000_000_000;
                    }
                    if ui.button("100B").clicked() {
                        state.max_prime_limit = 100_000_000_000;
                    }
                });
            });

            ui.add_space(6.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Display & Chart Preferences").strong().color(accent));
                ui.add_space(4.0);

                ui.checkbox(&mut state.show_pct_labels, "Show Percentage Annotations on Bars");
                ui.checkbox(&mut state.show_grid_lines, "Show Reference Grid Lines");
                ui.checkbox(&mut state.show_heatmap_meter, "Show Heat Map Count Meter (Top-Right)");
            });
        });

    state.show_settings = is_open;
}
