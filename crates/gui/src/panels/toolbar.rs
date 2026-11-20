//! Top row: settings, reset, k, prime-index range, rank range, and chart view selector.

use egui::{RichText, Ui};

use crate::actions::Action;
use crate::format::compact;
use crate::playback::Mode;
use crate::prefs::ChartView;
use crate::state::{AppState, MAX_K};
use crate::theme::{self, Palette};
use crate::widgets::{hint, index_input, range_slider};

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
        let reset_act = reset_button(ui);
        let mut enabled_act = None;
        ui.add_enabled_ui(!state.playback.is_busy(), |ui| {
            let k_act = k_input(ui, state);
            let range_act = index_range(ui, state, palette);
            rank_range(ui, state, palette);
            enabled_act = k_act.or(range_act);
        });
        let view_act = view_selector(ui, state);
        action = reset_act.or(enabled_act).or(view_act);
        busy_badge(ui, state.playback.mode);
    });
    ui.add_space(2.0);
    ui.separator();
    action
}

fn reset_button(ui: &mut Ui) -> Option<Action> {
    let clicked = ui
        .button("🔄")
        .on_hover_text("Reset & Clear Cache\nImmediately cancels any active computation, wipes all in-memory precomputations, clears worker segment cache, and restores initial launch state.")
        .clicked();
    ui.separator();
    clicked.then_some(Action::Reset)
}

fn k_input(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    let lock_k_to_one = state.dataset.as_ref().is_some_and(|d| !d.has_raw_gaps);
    ui.label("k:");
    let changed = ui
        .add_enabled_ui(!lock_k_to_one, |ui| {
            if lock_k_to_one {
                state.query.k = 1;
            }
            hint(
                ui.add(egui::DragValue::new(&mut state.query.k).range(1..=MAX_K)),
                "Step distance parameter k for prime gaps (Δ_k(n) = p_{n+k} - p_n).\nChanging k modifies the underlying mathematical gap distribution.",
                "Order parameter k is locked to 1 (index-only mode). Provide gaps.bin in Settings to evaluate k > 1 gaps on the fly.",
            )
            .changed()
        })
        .inner;
    ui.separator();
    if changed {
        state.frames = None;
        state.spectral_key = None;
        Some(Action::Show)
    } else {
        None
    }
}

fn index_range(ui: &mut Ui, state: &mut AppState, palette: &Palette) -> Option<Action> {
    let limit = state
        .dataset
        .as_ref()
        .map_or(state.prefs.max_prime_limit, |d| d.max_primes.max(1));
    let mut changed = false;
    {
        let q = &mut state.query;

        if index_input(ui, &mut q.min, 1..=limit).changed() {
            q.max = q.max.max(q.min);
            changed = true;
        }
        if range_slider(
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
        ))
        .changed()
        {
            changed = true;
        }
        if index_input(ui, &mut q.max, 1..=limit).changed() {
            q.min = q.min.min(q.max);
            changed = true;
        }
    }
    ui.separator();

    if changed {
        state.frames = None;
        state.spectral_key = None;
        state.playback.position = state
            .playback
            .position
            .clamp(state.query.min, state.query.max);
        if state.dataset.is_some() {
            state.playback.position = state.query.max;
            return Some(Action::Show);
        }
    }
    None
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

fn view_selector(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    let mut refresh = false;
    for (mode, label, tip) in [
        (ChartView::BarChart, "📊 Bars", "Normalized Gap Bar Chart"),
        (
            ChartView::SpectralHeatmap,
            "🌈 Heatmap",
            "2D Spectral Gap Density Heatmap",
        ),
        (
            ChartView::Split,
            "⊞ Split",
            "Combined Bar Chart + 2D Spectral Heatmap",
        ),
    ] {
        if ui
            .selectable_label(state.prefs.view_mode == mode, label)
            .on_hover_text(tip)
            .clicked()
        {
            state.prefs.view_mode = mode;
            if mode != ChartView::BarChart
                && state.spectral_key != Some(state.current_spectral_key())
            {
                refresh = true;
            }
        }
    }
    ui.separator();
    refresh.then_some(Action::RefreshSpectral)
}

fn busy_badge(ui: &mut Ui, mode: Mode) {
    let (text, color) = match mode {
        Mode::Precaching => ("⚡ Caching...", theme::CACHING),
        Mode::Playing => ("🎬 Playing...", theme::PLAYING),
        Mode::Stopped => return,
    };
    ui.label(RichText::new(text).small().strong().color(color));
}
