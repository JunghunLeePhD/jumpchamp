// ============================================================================
// Top Control Bar — User Controls & Single-Track Dual-Thumb Range Slider
// ============================================================================

use crate::gui::state::{AppState};
use crate::gui::theme;
use crate::gui::utils::{format_compact_num};

pub enum SidebarAction {
    None,
    Compute,
    Cancel,
    StartAnimation,
    StartReverseAnimation,
    StepAnimation,
    StepBackAnimation,
}

fn render_dual_slider_impl(
    ui: &mut egui::Ui,
    min_x_frac: f32,
    max_x_frac: f32,
    width: f32,
    fill_color: egui::Color32,
    is_dark: bool,
    hover_text: String,
    mut on_drag: impl FnMut(f32, bool),
) -> egui::Response {
    let desired_size = egui::vec2(width, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track_y = rect.center().y;
        let track_left = rect.min.x + 6.0;
        let track_right = rect.max.x - 6.0;
        let track_width = (track_right - track_left).max(1.0);

        let x_min = track_left + min_x_frac.clamp(0.0, 1.0) * track_width;
        let x_max = track_left + max_x_frac.clamp(0.0, 1.0) * track_width;

        // 1. Draw Background Rail (Single Track)
        painter.line_segment(
            [egui::pos2(track_left, track_y), egui::pos2(track_right, track_y)],
            egui::Stroke::new(4.0_f32, theme::slider_rail_bg(is_dark)),
        );

        // 2. Draw Active Range Fill Line
        painter.line_segment(
            [egui::pos2(x_min, track_y), egui::pos2(x_max, track_y)],
            egui::Stroke::new(4.0_f32, fill_color),
        );

        // 3. Handle Pointer Dragging for Min & Max Thumbs
        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let d_min = (pointer_pos.x - x_min).abs();
                let d_max = (pointer_pos.x - x_max).abs();
                let pointer_frac = ((pointer_pos.x - track_left) / track_width).clamp(0.0, 1.0);

                let is_min_thumb = d_min <= d_max;
                on_drag(pointer_frac, is_min_thumb);
                response.mark_changed();
            }
        }

        // 4. Draw Min Thumb
        let thumb_r = 6.0_f32;
        let min_circle = egui::pos2(x_min, track_y);
        painter.circle_filled(min_circle, thumb_r, theme::card_bg(is_dark));
        painter.circle_stroke(min_circle, thumb_r, egui::Stroke::new(1.5_f32, fill_color));

        // 5. Draw Max Thumb
        let max_circle = egui::pos2(x_max, track_y);
        painter.circle_filled(max_circle, thumb_r, fill_color);
        painter.circle_stroke(max_circle, thumb_r, egui::Stroke::new(1.5_f32, theme::text_primary(is_dark)));
    }

    response.on_hover_text(hover_text)
}

fn render_dual_range_slider(
    ui: &mut egui::Ui,
    min_val: &mut u64,
    max_val: &mut u64,
    limit: u64,
    is_dark: bool,
) -> egui::Response {
    let limit_f64 = limit.max(1) as f64;
    let min_frac = (*min_val as f64 / limit_f64) as f32;
    let max_frac = (*max_val as f64 / limit_f64) as f32;
    let accent = theme::accent_color(is_dark);

    render_dual_slider_impl(
        ui, min_frac, max_frac, 150.0, accent, is_dark,
        format!(
            "Prime Index Range: n = {} ~ {} (p_n_min ~ p_n_max)",
            format_compact_num(*min_val),
            format_compact_num(*max_val)
        ),
        |frac, is_min| {
            let new_val = ((frac as f64 * limit_f64).round() as u64).clamp(1, limit);
            if is_min {
                *min_val = new_val.min(*max_val);
            } else {
                *max_val = new_val.max(*min_val).min(limit);
            }
        },
    )
}

fn render_dual_top_range_slider(
    ui: &mut egui::Ui,
    min_val: &mut usize,
    max_val: &mut usize,
    limit: usize,
    is_dark: bool,
) -> egui::Response {
    let limit_f64 = limit.max(1) as f64;
    let min_frac = (*min_val as f64 / limit_f64) as f32;
    let max_frac = (*max_val as f64 / limit_f64) as f32;
    let top_fill_color = if is_dark { egui::Color32::from_rgb(255, 180, 0) } else { egui::Color32::from_rgb(220, 140, 0) };

    render_dual_slider_impl(
        ui, min_frac, max_frac, 100.0, top_fill_color, is_dark,
        format!("Rank Range: Rank {} ~ Rank {}", min_val, max_val),
        |frac, is_min| {
            let new_val = ((frac as f64 * limit_f64).round() as usize).clamp(1, limit);
            if is_min {
                *min_val = new_val.min(*max_val);
            } else {
                *max_val = new_val.max(*min_val).min(limit);
            }
        },
    )
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    let max_limit = state.max_prime_limit;
    let max_k = state.max_k_limit;

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        // Group 0: Settings Button (Front Position)
        if ui.button("⚙ Settings").clicked() {
            state.show_settings = !state.show_settings;
        }
        ui.separator();

        // Group 1: Gap order k parameter
        ui.label("k:");
        ui.add(egui::DragValue::new(&mut state.k).range(1..=max_k));
        ui.separator();

        // Group 2: Numerical Prime Value Range [N_min, N_max]
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

        let is_dark = theme::is_dark(state.theme_mode);

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

        // Group 3: Dynamic Min/Max Range Slider (Bounded by unique gaps found in range)
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

        // Group 4: Action Button / Progress Bar
        if state.is_loading && !state.is_animating {
            ui.add_sized(
                [80.0_f32, 18.0_f32],
                egui::ProgressBar::new(state.progress).show_percentage(),
            );
            if ui.button("✖ Cancel").clicked() {
                action = SidebarAction::Cancel;
            }
        } else if ui.button("▶ Compute").clicked() {
            action = SidebarAction::Compute;
        }
    });
    ui.add_space(2.0);
    ui.separator();

    // Group 5: Animation Toolbar Row (Cumulative Growth Animation)
    ui.horizontal(|ui| {
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
            if ui
                .button("◀ Reverse")
                .clicked()
            {
                action = SidebarAction::StartReverseAnimation;
            }

            let play_label = if state.anim_current_val > state.min_val && state.anim_current_val < state.max_val {
                "▶ Resume"
            } else {
                "▶ Play"
            };
            if ui
                .button(play_label)
                .clicked()
            {
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

        // Speed FPS Slider
        ui.label("Speed:");
        ui.add(
            egui::Slider::new(&mut state.anim_speed_fps, 1.0..=30.0)
                .suffix(" FPS")
                .show_value(true),
        );
    });
    ui.add_space(2.0);

    action
}
