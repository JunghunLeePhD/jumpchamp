//! Top row: settings, reset, k, prime-index range and rank range.

use egui::{RichText, Ui};

use crate::gui::actions::Action;
use crate::gui::format::compact;
use crate::gui::playback::Mode;
use crate::gui::state::{AppState, MAX_K};
use crate::gui::theme::{self, Palette};
use crate::gui::widgets::{hint, index_input, range_slider};

pub fn render(ui: &mut Ui, state: &mut AppState, palette: &Palette) -> Option<Action> {
    let mut action = None;
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui
            .button("⚙")
            .on_hover_text("Settings & Preferences")
            .clicked()
        {
            state.prefs.show_settings ^= true;
        }
        ui.add_enabled_ui(!state.playback.is_busy(), |ui| {
            action = reset_button(ui).or(k_input(ui, state));
            index_range(ui, state, palette);
            rank_range(ui, state, palette);
        });
        busy_badge(ui, state.playback.mode);
    });
    ui.add_space(2.0);
    ui.separator();
    action
}

fn reset_button(ui: &mut Ui) -> Option<Action> {
    let clicked = hint(
        ui.button("🔄"),
        "Reset & Clear Cache\nWipes all in-memory precomputations, clears worker segment cache, and restores initial launch state.",
        "Reset is disabled while animation is running. Pause or stop animation first.",
    )
    .clicked();
    ui.separator();
    clicked.then_some(Action::Reset)
}

fn k_input(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    ui.label("k:");
    let changed = hint(
        ui.add(egui::DragValue::new(&mut state.query.k).range(1..=MAX_K)),
        "Step distance parameter k for prime gaps (Δ_k(n) = p_{n+k} - p_n).\nChanging k modifies the underlying mathematical gap distribution.",
        "Order parameter k is locked while animation is running. Pause or stop first.",
    )
    .changed();
    ui.separator();
    changed.then_some(Action::Show)
}

fn index_range(ui: &mut Ui, state: &mut AppState, palette: &Palette) {
    let limit = state.prefs.max_prime_limit;
    let q = &mut state.query;

    if index_input(ui, &mut q.min, 1..=limit).changed() {
        q.max = q.max.max(q.min);
    }
    range_slider(
        ui,
        (&mut q.min, &mut q.max),
        limit,
        150.0,
        palette.accent,
        palette,
    )
    .on_hover_text(format!(
        "Prime Index Range: n = {} ~ {} (p_n_min ~ p_n_max)",
        compact(q.min),
        compact(q.max)
    ));
    if index_input(ui, &mut q.max, 1..=limit).changed() {
        q.min = q.min.min(q.max);
    }
    ui.separator();
}

fn rank_range(ui: &mut Ui, state: &mut AppState, palette: &Palette) {
    let r = &mut state.rank;
    let limit = state.bars.len().max(20).max(r.max);
    r.min = r.min.clamp(1, r.max);

    ui.label("Rank:");
    ui.add(egui::DragValue::new(&mut r.min).range(1..=r.max));

    let (mut lo, mut hi) = (r.min as u64, r.max as u64);
    range_slider(
        ui,
        (&mut lo, &mut hi),
        limit as u64,
        100.0,
        palette.rank_fill,
        palette,
    )
    .on_hover_text(format!("Rank Range: Rank {lo} ~ Rank {hi}"));
    (r.min, r.max) = (lo as usize, hi as usize);

    ui.add(egui::DragValue::new(&mut r.max).range(r.min..=limit));
    ui.separator();
}

fn busy_badge(ui: &mut Ui, mode: Mode) {
    let (text, color) = match mode {
        Mode::Precaching => ("⚡ Caching...", theme::CACHING),
        Mode::Playing => ("🎬 Playing...", theme::PLAYING),
        Mode::Stopped => return,
    };
    ui.label(RichText::new(text).small().strong().color(color));
}
