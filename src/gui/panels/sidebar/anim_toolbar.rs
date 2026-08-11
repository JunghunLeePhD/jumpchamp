// ============================================================================
// Animation Toolbar Controls Component (Playback, Step, Speed FPS)
// ============================================================================

use super::SidebarAction;
use crate::gui::state::AppState;
use crate::gui::utils::format_compact_num;

/// Renders playback control buttons (Pause, Reverse, Play/Resume, Step Back, Step, Reset).
fn render_playback_buttons(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.label(egui::RichText::new("🎬").strong());

    if state.is_animating {
        if ui
            .button("⏸ Pause")
            .on_hover_text("Pause Growth Chart Animation")
            .clicked()
        {
            state.is_animating = false;
        }
    } else {
        if ui.button("◀ Reverse").clicked() {
            action = SidebarAction::StartReverseAnimation;
        }

        let play_label = if state.anim_current_val > state.min_val && state.anim_current_val < state.max_val {
            "▶ Resume"
        } else {
            "▶ Play"
        };
        if ui.button(play_label).clicked() {
            action = SidebarAction::StartAnimation;
        }
    }

    if ui.button("⏮ Step Back").clicked() {
        state.is_animating = false;
        action = SidebarAction::StepBackAnimation;
    }

    if ui.button("⏭ Step").clicked() {
        state.is_animating = false;
        action = SidebarAction::StepAnimation;
    }

    if ui.button("↺ Reset").clicked() {
        state.is_animating = false;
        state.anim_current_val = state.min_val;
        action = SidebarAction::StepAnimation;
    }
    ui.separator();

    action
}

/// Renders drag input for current animation value.
fn render_current_val_input(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.label("Current:");
    let bound_speed = (state.max_val as f64 / 100.0).max(10.0);
    if ui
        .add_sized(
            [85.0_f32, 18.0_f32],
            egui::DragValue::new(&mut state.anim_current_val)
                .speed(bound_speed)
                .range(state.min_val..=state.max_val)
                .custom_formatter(|v, _| format_compact_num(v as u64)),
        )
        .changed()
    {
        state.anim_current_val = state.anim_current_val.clamp(state.min_val, state.max_val);
        if !state.is_animating {
            action = SidebarAction::StepAnimation;
        }
    }
    ui.separator();

    action
}

/// Renders drag input for animation step size.
fn render_step_size_input(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label("Step:");
    let prime_range = state.max_val.saturating_sub(state.min_val).max(50);
    let dynamic_step_speed = (prime_range as f64 / 500.0).max(1.0);
    ui.add_sized(
        [85.0_f32, 18.0_f32],
        egui::DragValue::new(&mut state.anim_step_size)
            .speed(dynamic_step_speed)
            .range(1..=prime_range)
            .custom_formatter(|v, _| format_compact_num(v as u64)),
    );
    ui.separator();
}

/// Renders speed FPS slider.
fn render_speed_fps_slider(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label("Speed:");
    ui.add(
        egui::Slider::new(&mut state.anim_speed_fps, 1.0..=30.0)
            .suffix(" FPS")
            .show_value(true),
    );
    ui.separator();
}

/// Renders 300-frame progress counter label.
fn render_frame_counter(ui: &mut egui::Ui, state: &AppState) {
    let current_step = if state.anim_step_size > 0 {
        ((state.anim_current_val.saturating_sub(state.min_val)) / state.anim_step_size).min(300) + 1
    } else {
        1
    };
    ui.label(egui::RichText::new(format!("Frame {}/300", current_step)).strong().color(egui::Color32::from_rgb(0, 180, 220)));
}

/// Renders the animation toolbar row.
pub fn render_anim_toolbar(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.horizontal(|ui| {
        let btn_action = render_playback_buttons(ui, state);
        if btn_action != SidebarAction::None {
            action = btn_action;
        }

        let input_action = render_current_val_input(ui, state);
        if input_action != SidebarAction::None {
            action = input_action;
        }

        render_step_size_input(ui, state);
        render_speed_fps_slider(ui, state);
        render_frame_counter(ui, state);
    });
    ui.add_space(2.0);

    action
}
