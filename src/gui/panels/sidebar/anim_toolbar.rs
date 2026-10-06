// ============================================================================
// Animation Toolbar Controls Component (Playback, Step, Speed FPS)
// ============================================================================

use super::SidebarAction;
use crate::gui::state::AppState;
use crate::gui::utils::format_compact_num;

/// Renders playback control buttons (Pause, Stop, Reverse, Play/Resume, Step Back, Step, Reset).
fn render_playback_buttons(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;
    let btn_size = egui::vec2(26.0, 20.0);

    if state.is_precaching {
        // While pre-caching: Provide Stop/Cancel button to abort calculation
        let stop_btn_size = egui::vec2(btn_size.x * 2.0 + ui.spacing().item_spacing.x, btn_size.y);
        if ui
            .add_sized(stop_btn_size, egui::Button::new("⏹ Cancel"))
            .on_hover_text("Cancel in-progress pre-caching computation")
            .clicked()
        {
            action = SidebarAction::Cancel;
        }
    } else if state.is_animating {
        // While animating: Provide Pause (⏸) and Stop (⏹) buttons
        if ui
            .add_sized(btn_size, egui::Button::new("⏸"))
            .on_hover_text("Pause Growth Chart Animation at current frame")
            .clicked()
        {
            state.is_animating = false;
        }

        if ui
            .add_sized(btn_size, egui::Button::new("⏹"))
            .on_hover_text("Stop Animation and reset frame to start (n_min)")
            .clicked()
        {
            state.is_animating = false;
            state.anim_current_val = state.min_val;
            action = SidebarAction::StepAnimation;
        }
    } else {
        // Idle / paused: Provide Play Reverse and Play Forward buttons
        if ui
            .add_sized(btn_size, egui::Button::new("◀"))
            .on_hover_text("Play Reverse Animation")
            .clicked()
        {
            action = SidebarAction::StartReverseAnimation;
        }

        if ui
            .add_sized(btn_size, egui::Button::new("▶"))
            .on_hover_text("Play Forward Animation")
            .clicked()
        {
            action = SidebarAction::StartAnimation;
        }
    }

    // Step back, Step forward, Reset to start (disabled while animation is running)
    let is_running = state.is_animation_running();
    ui.add_enabled_ui(!is_running, |ui| {
        if ui
            .add_sized(btn_size, egui::Button::new("⏮"))
            .on_hover_text(if !is_running {
                "Step Back One Frame"
            } else {
                "Step Back is disabled while animation is running. Pause or stop first."
            })
            .clicked()
        {
            state.is_animating = false;
            action = SidebarAction::StepBackAnimation;
        }

        if ui
            .add_sized(btn_size, egui::Button::new("⏭"))
            .on_hover_text(if !is_running {
                "Step Forward One Frame"
            } else {
                "Step Forward is disabled while animation is running. Pause or stop first."
            })
            .clicked()
        {
            state.is_animating = false;
            action = SidebarAction::StepAnimation;
        }

        if ui
            .add_sized(btn_size, egui::Button::new("↺"))
            .on_hover_text(if !is_running {
                "Reset to Start"
            } else {
                "Reset is disabled while animation is running. Pause or stop first."
            })
            .clicked()
        {
            state.is_animating = false;
            state.anim_current_val = state.min_val;
            action = SidebarAction::StepAnimation;
        }
    });
    ui.separator();

    action
}

/// Renders drag input for current animation value.
fn render_current_val_input(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;
    let is_running = state.is_animation_running();

    ui.add_enabled_ui(!is_running, |ui| {
        let bound_speed = (state.max_val as f64 / 100.0).max(10.0);
        let resp = ui.add_sized(
            [85.0_f32, 18.0_f32],
            egui::DragValue::new(&mut state.anim_current_val)
                .speed(bound_speed)
                .range(state.min_val..=state.max_val)
                .custom_formatter(|v, _| format_compact_num(v as u64)),
        );
        let resp = if !is_running {
            resp.on_hover_text("Current prime index position (n). Drag to scrub frames.")
        } else {
            resp.on_hover_text("Frame scrubbing is locked while animation is running. Pause or stop first.")
        };
        if resp.changed() {
            state.anim_current_val = state.anim_current_val.clamp(state.min_val, state.max_val);
            if !state.is_animating {
                action = SidebarAction::StepAnimation;
            }
        }
    });
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

    if ui.button("15").on_hover_text("Set to 15 FPS").clicked() {
        state.anim_speed_fps = 15.0;
    }
    if ui.button("30").on_hover_text("Set to 30 FPS").clicked() {
        state.anim_speed_fps = 30.0;
    }
    if ui.button("60").on_hover_text("Set to 60 FPS").clicked() {
        state.anim_speed_fps = 60.0;
    }
    if ui.button("120").on_hover_text("Set to 120 FPS").clicked() {
        state.anim_speed_fps = 120.0;
    }
    ui.separator();
}

/// Renders progress bar showing either pre-caching progress or animation playback progress.
fn render_animation_progress_bar(ui: &mut egui::Ui, state: &AppState) {
    if state.is_precaching {
        let pct = state.progress;
        let block_info = if state.total_blocks > 0 {
            format!(" (Block {}/{})", state.current_block, state.total_blocks)
        } else {
            String::new()
        };
        ui.add_sized(
            [120.0_f32, 18.0_f32],
            egui::ProgressBar::new(pct)
                .show_percentage()
                .text(format!("Caching: {:.0}%", pct * 100.0)),
        )
        .on_hover_text(format!(
            "Pre-caching 300 animation frames in background{block_info}..."
        ));
    } else {
        let anim_prog = state.animation_progress();
        ui.add_sized(
            [120.0_f32, 18.0_f32],
            egui::ProgressBar::new(anim_prog)
                .show_percentage()
                .text(format!("{:.0}%", anim_prog * 100.0)),
        )
        .on_hover_text(format!(
            "Animation progress: {:.1}% (Current n = {} / {})",
            anim_prog * 100.0,
            format_compact_num(state.anim_current_val),
            format_compact_num(state.max_val)
        ));
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
    ui.label(
        egui::RichText::new(format!("Frame {}/300", current_step))
            .strong()
            .color(egui::Color32::from_rgb(0, 180, 220)),
    );
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
        render_animation_progress_bar(ui, state);
        render_frame_counter(ui, state);
    });
    ui.add_space(2.0);

    action
}
