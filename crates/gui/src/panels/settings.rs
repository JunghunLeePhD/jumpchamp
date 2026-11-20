//! Modal settings window: theme, zero-copy storage, global limit, display toggles.

use egui::{Color32, Context, RichText, Ui};

use jumpchamp_core::storage::MAX_DATASET_PRIMES;

use crate::actions::Action;
use crate::format::{compact, thousands};
use crate::prefs::{ThemeMode, MAX_LIMIT, MIN_LIMIT};
use crate::state::AppState;
use crate::theme::{self, Palette};

const LIMIT_PRESETS: [(&str, u64); 6] = [
    ("10M", 10_000_000),
    ("100M", 100_000_000),
    ("1B", 1_000_000_000),
    ("10B", 10_000_000_000),
    ("37.6B", MAX_DATASET_PRIMES),
    ("100B", 100_000_000_000),
];

pub fn render(ctx: &Context, state: &mut AppState, palette: &Palette) -> Option<Action> {
    let mut open = state.prefs.show_settings;
    let mut action = None;
    egui::Window::new("⚙ JumpChamp Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size(egui::vec2(470.0, 470.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);
            section(ui, palette, "Theme Mode", |ui| {
                theme_choice(ui, &mut state.prefs.theme)
            });
            ui.add_space(6.0);
            let busy = state.playback.is_busy();
            ui.add_enabled_ui(!busy, |ui| {
                section(ui, palette, "Zero-Copy Binary Storage (Mmap)", |ui| {
                    action = storage_controls(ui, state, palette);
                });
                ui.add_space(6.0);
                section(ui, palette, "Global Numerical Limits", |ui| {
                    limits(ui, state, busy)
                });
            });
            ui.add_space(6.0);
            section(ui, palette, "Display & Chart Preferences", |ui| {
                toggles(ui, state)
            });
        });
    state.prefs.show_settings = open;
    action
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

fn storage_controls(ui: &mut Ui, state: &mut AppState, palette: &Palette) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.label("Block Index:");
        ui.add(
            egui::TextEdit::singleline(&mut state.prefs.index_path)
                .desired_width(290.0)
                .hint_text("block_index.bin"),
        );
    });
    ui.horizontal(|ui| {
        ui.label("Raw Gaps:   ");
        ui.add(
            egui::TextEdit::singleline(&mut state.prefs.gaps_path)
                .desired_width(290.0)
                .hint_text("Optional gaps.bin (for k > 1 & sub-block exact)"),
        );
    });
    ui.horizontal(|ui| {
        if ui.button("📂 Load Index").clicked() {
            action = Some(Action::LoadDataset);
        }
        if state.dataset.is_some() && ui.button("⏏ Unload").clicked() {
            action = Some(Action::UnloadDataset);
        }
    });

    if let Some(ref err) = state.dataset_error {
        ui.label(
            RichText::new(err)
                .small()
                .color(Color32::from_rgb(210, 60, 60)),
        );
    } else if let Some(ref meta) = state.dataset {
        let raw_note = if meta.has_raw_gaps {
            " + raw gaps"
        } else {
            " (index only)"
        };
        ui.label(
            RichText::new(format!(
                "Mounted: {} blocks, {} primes{raw_note}",
                thousands(meta.block_count as u64),
                compact(meta.max_primes)
            ))
            .small()
            .color(palette.accent),
        );
    } else {
        ui.label(
            RichText::new("No binary index mounted — using in-memory parallel sieve.")
                .small()
                .color(palette.text_dim),
        );
    }
    action
}

fn limits(ui: &mut Ui, state: &mut AppState, locked: bool) {
    if locked {
        ui.label(
            RichText::new("🔒 Locked while animation is running")
                .small()
                .italics()
                .color(theme::LOCKED),
        );
    }
    ui.horizontal(|ui| {
        ui.label("Max Prime Index Limit (n):");
        let mut limit = state.prefs.max_prime_limit;
        if ui
            .add(
                egui::DragValue::new(&mut limit)
                    .speed(10_000_000)
                    .range(MIN_LIMIT..=MAX_LIMIT),
            )
            .changed()
        {
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
    ui.checkbox(
        &mut p.show_pct_labels,
        "Show Percentage Annotations on Bars",
    );
    ui.checkbox(&mut p.show_grid_lines, "Show Reference Grid Lines");
    ui.checkbox(
        &mut p.show_heatmap_meter,
        "Show Heat Map Count Meter (Top-Right)",
    );
    ui.checkbox(&mut p.show_bar_tooltip, "Show Cursor Hover Details Tooltip");
}
