// ============================================================================
// Modal Settings Window Component
// ============================================================================

use crate::gui::panels::sidebar::format_compact_num;
use crate::gui::state::AppState;

pub fn render(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_settings {
        return;
    }

    let mut is_open = state.show_settings;
    let mut should_close = false;

    egui::Window::new("⚙ JumpChamp Settings")
        .open(&mut is_open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(420.0, 260.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("🌐 Global Numerical Limits").strong().color(egui::Color32::from_rgb(90, 200, 250)));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Max Prime Value:");
                    ui.add(egui::DragValue::new(&mut state.max_prime_limit).speed(1_000_000_000).range(1_000_000..=1_000_000_000_000u64));
                    ui.label(format!("({})", format_compact_num(state.max_prime_limit)));
                });

                ui.horizontal(|ui| {
                    ui.label("Quick Presets:");
                    if ui.button("10B").clicked() {
                        state.max_prime_limit = 10_000_000_000;
                    }
                    if ui.button("100B").clicked() {
                        state.max_prime_limit = 100_000_000_000;
                    }
                    if ui.button("1T").clicked() {
                        state.max_prime_limit = 1_000_000_000_000;
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Max Gap Step k Limit:");
                    ui.add(egui::DragValue::new(&mut state.max_k_limit).range(1..=100));
                    ui.label(format!("(Max k={})", state.max_k_limit));
                });
            });

            ui.add_space(6.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("🎨 Display & Chart Preferences").strong().color(egui::Color32::from_rgb(90, 200, 250)));
                ui.add_space(4.0);

                ui.checkbox(&mut state.show_pct_labels, "Show Percentage Annotations on Bars");
                ui.checkbox(&mut state.show_grid_lines, "Show Reference Grid Lines");
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Apply & Close").clicked() {
                    should_close = true;
                }
            });
        });

    state.show_settings = is_open && !should_close;
}
