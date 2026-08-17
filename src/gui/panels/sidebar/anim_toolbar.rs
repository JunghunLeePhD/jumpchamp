// ============================================================================
// Animation Toolbar Controls Component (Playback, Step, Speed FPS)
// ============================================================================

use super::SidebarAction;
use crate::gui::state::AppState;
use crate::gui::utils::format_compact_num;

/// Renders playback control buttons (Pause, Reverse, Play/Resume, Step Back, Step, Reset).
fn render_playback_buttons(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    if state.is_animating {
        if ui
            .button("⏸")
            .on_hover_text("Pause Growth Chart Animation")
            .clicked()
        {
            state.is_animating = false;
        }
    } else {
        if ui.button("◀").clicked() {
            action = SidebarAction::StartReverseAnimation;
        }

        let play_label = if state.anim_current_val > state.min_val && state.anim_current_val < state.max_val {
            "▶"
        } else {
            "▶"
        };
        if ui.button(play_label).clicked() {
            action = SidebarAction::StartAnimation;
        }
    }

    if ui.button("⏮").clicked() {
        state.is_animating = false;
        action = SidebarAction::StepBackAnimation;
    }

    if ui.button("⏭").clicked() {
        state.is_animating = false;
        action = SidebarAction::StepAnimation;
    }

    if ui.button("↺").clicked() {
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


/// Renders speed FPS slider and quick speed preset buttons.
fn render_speed_fps_slider(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label("Speed:");
    ui.add_sized(
        [110.0_f32, 18.0_f32],
        egui::Slider::new(&mut state.anim_speed_fps, 1.0..=120.0)
            .step_by(1.0)
            .suffix(" FPS")
            .clamping(egui::SliderClamping::Always),
    );

    if ui.button("30").on_hover_text("Set to 30 FPS").clicked() {
        state.anim_speed_fps = 30.0;
    }
    if ui.button("60").on_hover_text("Set to 60 FPS").clicked() {
        state.anim_speed_fps = 60.0;
    }
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

        render_speed_fps_slider(ui, state);
        render_frame_counter(ui, state);
    });
    ui.add_space(2.0);

    action
}
