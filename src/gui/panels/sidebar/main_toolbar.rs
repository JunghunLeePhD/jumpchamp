// ============================================================================
// Main Control Toolbar Component (Settings, k-Order, Prime Range, Rank, Action)
// ============================================================================

use super::dual_slider::{render_dual_range_slider, render_dual_top_range_slider};
use super::SidebarAction;
use crate::gui::state::{AppState, ViewMode};
use crate::gui::theme;
use crate::gui::utils::format_compact_num;

/// Renders the view mode toggle buttons (Static Chart vs Animation View).
fn render_mode_selector(ui: &mut egui::Ui, state: &mut AppState) {
    if ui.selectable_label(state.view_mode == ViewMode::Static, "📊 Static").clicked() {
        state.set_view_mode(ViewMode::Static);
    }
    if ui.selectable_label(state.view_mode == ViewMode::Animation, "🎬 Animation").clicked() {
        state.set_view_mode(ViewMode::Animation);
    }
    ui.separator();
}

/// Renders the settings toggle button.
fn render_settings_button(ui: &mut egui::Ui, state: &mut AppState) {
    if ui.button("⚙ Settings").clicked() {
        state.show_settings = !state.show_settings;
    }
    ui.separator();
}

/// Renders the gap order parameter `k` input control.
fn render_k_selector(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label("k:");
    ui.add(egui::DragValue::new(&mut state.k).range(1..=usize::MAX));
    ui.separator();
}

/// Renders the numerical prime range control section (Min drag input, dual range slider, Max drag input).
fn render_prime_range_section(ui: &mut egui::Ui, state: &mut AppState, is_dark: bool) {
    let max_limit = state.max_prime_limit;

    let min_speed = (state.min_val as f64 / 100.0).max(10.0);
    if ui
        .add_sized(
            [85.0_f32, 18.0_f32],
            egui::DragValue::new(&mut state.min_val)
                .speed(min_speed)
                .range(1..=max_limit)
                .custom_formatter(|v, _| format_compact_num(v as u64)),
        )
        .changed()
    {
        state.min_val = state.min_val.clamp(1, max_limit);
        if state.min_val > state.max_val {
            state.max_val = state.min_val;
        }
        state.recalculate_dynamic_step();
    }

    if render_dual_range_slider(ui, &mut state.min_val, &mut state.max_val, max_limit, is_dark).changed() {
        state.recalculate_dynamic_step();
    }

    let max_speed = (state.max_val as f64 / 100.0).max(10.0);
    if ui
        .add_sized(
            [85.0_f32, 18.0_f32],
            egui::DragValue::new(&mut state.max_val)
                .speed(max_speed)
                .range(1..=max_limit)
                .custom_formatter(|v, _| format_compact_num(v as u64)),
        )
        .changed()
    {
        state.max_val = state.max_val.clamp(1, max_limit);
        if state.max_val < state.min_val {
            state.min_val = state.max_val;
        }
        state.recalculate_dynamic_step();
    }
    ui.separator();
}

/// Renders the rank range section (Rank min drag input, dual rank slider, Rank max drag input).
fn render_rank_range_section(ui: &mut egui::Ui, state: &mut AppState, is_dark: bool) {
    let max_slider_limit = state.freq_data.len().max(20).max(state.top_max);
    state.top_min = state.top_min.clamp(1, state.top_max);

    ui.label("Rank:");
    if ui
        .add(egui::DragValue::new(&mut state.top_min).range(1..=state.top_max))
        .changed()
    {
        state.top_min = state.top_min.clamp(1, state.top_max);
    }

    render_dual_top_range_slider(ui, &mut state.top_min, &mut state.top_max, max_slider_limit, is_dark);

    if ui
        .add(egui::DragValue::new(&mut state.top_max).range(state.top_min..=max_slider_limit))
        .changed()
    {
        state.top_max = state.top_max.clamp(state.top_min, max_slider_limit);
    }
    ui.separator();
}

/// Renders the primary action button or progress bar (Compute / Cancel).
fn render_compute_action(ui: &mut egui::Ui, state: &AppState) -> SidebarAction {
    if state.is_loading && !state.is_animating {
        ui.add_sized(
            [80.0_f32, 18.0_f32],
            egui::ProgressBar::new(state.progress).show_percentage(),
        );
        if ui.button("✖ Cancel").clicked() {
            return SidebarAction::Cancel;
        }
    } else if ui.button("▶ Compute").clicked() {
        return SidebarAction::Compute;
    }
    SidebarAction::None
}

/// Renders the top main control toolbar row.
pub fn render_main_toolbar(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;
    let is_dark = theme::is_dark(state.theme_mode);

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        render_mode_selector(ui, state);
        render_settings_button(ui, state);
        render_k_selector(ui, state);
        render_prime_range_section(ui, state, is_dark);
        render_rank_range_section(ui, state, is_dark);
        if state.view_mode == ViewMode::Static {
            action = render_compute_action(ui, state);
        }
    });
    ui.add_space(2.0);
    ui.separator();

    action
}
