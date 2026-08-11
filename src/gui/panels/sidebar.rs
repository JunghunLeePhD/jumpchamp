// ============================================================================
// Top Control Bar — User Controls & Single-Track Dual-Thumb Range Slider
// ============================================================================

use crate::gui::state::{AppState, SortOrder};
use crate::gui::theme;

pub enum SidebarAction {
    None,
    Compute,
    Cancel,
    StartAnimation,
    StepAnimation,
}

pub fn format_compact_num(val: u64) -> String {
    if val >= 1_000_000_000_000 {
        format!("{:.2} T", val as f64 / 1e12)
    } else if val >= 1_000_000_000 {
        format!("{:.2} B", val as f64 / 1e9)
    } else if val >= 1_000_000 {
        format!("{:.2} M", val as f64 / 1e6)
    } else if val >= 1_000 {
        format!("{:.1} K", val as f64 / 1e3)
    } else {
        format!("{}", val)
    }
}

pub fn format_thousands(val: u64) -> String {
    let s = val.to_string();
    let mut result = String::new();
    let len = s.len();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

fn render_dual_range_slider(
    ui: &mut egui::Ui,
    min_val: &mut u64,
    max_val: &mut u64,
    limit: u64,
    is_dark: bool,
) -> egui::Response {
    let desired_size = egui::vec2(150.0, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track_y = rect.center().y;
        let track_left = rect.min.x + 6.0;
        let track_right = rect.max.x - 6.0;
        let track_width = (track_right - track_left).max(1.0);
        let max_lim = limit.max(1) as f64;

        let val_to_x = |v: u64| -> f32 {
            let frac = (v as f64 / max_lim).clamp(0.0, 1.0) as f32;
            track_left + frac * track_width
        };

        let x_to_val = |x: f32| -> u64 {
            let frac = ((x - track_left) / track_width).clamp(0.0, 1.0) as f64;
            ((frac * max_lim).round() as u64).clamp(1, limit)
        };

        let x_min = val_to_x(*min_val);
        let x_max = val_to_x(*max_val);

        // 1. Draw Background Rail (Single Track)
        painter.line_segment(
            [egui::pos2(track_left, track_y), egui::pos2(track_right, track_y)],
            egui::Stroke::new(4.0_f32, theme::slider_rail_bg(is_dark)),
        );

        // 2. Draw Active Range Fill Line
        let accent = theme::accent_color(is_dark);
        painter.line_segment(
            [egui::pos2(x_min, track_y), egui::pos2(x_max, track_y)],
            egui::Stroke::new(4.0_f32, accent),
        );

        // 3. Handle Pointer Dragging for Min & Max Thumbs
        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let d_min = (pointer_pos.x - x_min).abs();
                let d_max = (pointer_pos.x - x_max).abs();
                let new_val = x_to_val(pointer_pos.x);

                if d_min <= d_max {
                    *min_val = new_val.min(*max_val);
                } else {
                    *max_val = new_val.max(*min_val).min(limit);
                }
                response.mark_changed();
            }
        }

        // 4. Draw Min Thumb
        let thumb_r = 6.0_f32;
        let min_circle = egui::pos2(x_min, track_y);
        painter.circle_filled(min_circle, thumb_r, theme::card_bg(is_dark));
        painter.circle_stroke(min_circle, thumb_r, egui::Stroke::new(1.5_f32, accent));

        // 5. Draw Max Thumb
        let max_circle = egui::pos2(x_max, track_y);
        painter.circle_filled(max_circle, thumb_r, accent);
        painter.circle_stroke(max_circle, thumb_r, egui::Stroke::new(1.5_f32, theme::text_primary(is_dark)));
    }

    response.on_hover_text(format!(
        "Prime Numerical Range: {} ~ {} ({})",
        format_compact_num(*min_val),
        format_compact_num(*max_val),
        format!("{}..{}", min_val, max_val)
    ))
}

fn render_dual_top_range_slider(
    ui: &mut egui::Ui,
    min_val: &mut usize,
    max_val: &mut usize,
    limit: usize,
    is_dark: bool,
) -> egui::Response {
    let desired_size = egui::vec2(100.0, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track_y = rect.center().y;
        let track_left = rect.min.x + 6.0;
        let track_right = rect.max.x - 6.0;
        let track_width = (track_right - track_left).max(1.0);
        let max_lim = limit.max(1) as f64;

        let val_to_x = |v: usize| -> f32 {
            let frac = (v as f64 / max_lim).clamp(0.0, 1.0) as f32;
            track_left + frac * track_width
        };

        let x_to_val = |x: f32| -> usize {
            let frac = ((x - track_left) / track_width).clamp(0.0, 1.0) as f64;
            ((frac * max_lim).round() as usize).clamp(1, limit)
        };

        let x_min = val_to_x(*min_val);
        let x_max = val_to_x(*max_val);

        // 1. Draw Background Rail (Single Track)
        painter.line_segment(
            [egui::pos2(track_left, track_y), egui::pos2(track_right, track_y)],
            egui::Stroke::new(4.0_f32, theme::slider_rail_bg(is_dark)),
        );

        // 2. Draw Active Range Fill Line
        let top_fill_color = if is_dark { egui::Color32::from_rgb(255, 180, 0) } else { egui::Color32::from_rgb(220, 140, 0) };
        painter.line_segment(
            [egui::pos2(x_min, track_y), egui::pos2(x_max, track_y)],
            egui::Stroke::new(4.0_f32, top_fill_color),
        );

        // 3. Handle Pointer Dragging for Min & Max Thumbs
        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let d_min = (pointer_pos.x - x_min).abs();
                let d_max = (pointer_pos.x - x_max).abs();
                let new_val = x_to_val(pointer_pos.x);

                if d_min <= d_max {
                    *min_val = new_val.min(*max_val);
                } else {
                    *max_val = new_val.max(*min_val).min(limit);
                }
                response.mark_changed();
            }
        }

        // 4. Draw Min Thumb
        let thumb_r = 6.0_f32;
        let min_circle = egui::pos2(x_min, track_y);
        painter.circle_filled(min_circle, thumb_r, theme::card_bg(is_dark));
        painter.circle_stroke(min_circle, thumb_r, egui::Stroke::new(1.5_f32, top_fill_color));

        // 5. Draw Max Thumb
        let max_circle = egui::pos2(x_max, track_y);
        painter.circle_filled(max_circle, thumb_r, top_fill_color);
        painter.circle_stroke(max_circle, thumb_r, egui::Stroke::new(1.5_f32, theme::text_primary(is_dark)));
    }

    response.on_hover_text(format!(
        "Rank Range: Rank {} ~ Rank {}",
        min_val, max_val
    ))
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    let max_limit = state.max_prime_limit;
    let max_k = state.max_k_limit;

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        // Group 0: Settings Button (Front Position)
        if ui.button("⚙ Settings").on_hover_text("Open Settings Window").clicked() {
            state.show_settings = !state.show_settings;
        }

        ui.separator();
        // Group 1: Gap order k parameter
        ui.label("k:");
        ui.add(egui::DragValue::new(&mut state.k).range(1..=max_k))
            .on_hover_text("Gap order k (e.g. k=1 for consecutive prime gaps)");

        ui.separator();
        // Group 2: Sort Order
        ui.radio_value(&mut state.sort_by, SortOrder::ByGapSize, "Gap");
        ui.radio_value(&mut state.sort_by, SortOrder::ByFrequency, "Rank");

        ui.separator();
        // Group 3: Numerical Prime Value Range [N_min, N_max]
        let min_speed = (state.min_val as f64 / 100.0).max(10.0);
        if ui
            .add_sized(
                [85.0_f32, 18.0_f32],
                egui::DragValue::new(&mut state.min_val)
                    .speed(min_speed)
                    .range(1..=max_limit),
            )
            .on_hover_text(format!("Min Prime Value N_min: {}", format_compact_num(state.min_val)))
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
                    .range(1..=max_limit),
            )
            .on_hover_text(format!("Max Prime Value N_max: {}", format_compact_num(state.max_val)))
            .changed()
        {
            state.max_val = state.max_val.clamp(1, max_limit);
            if state.max_val < state.min_val {
                state.min_val = state.max_val;
            }
            state.recalculate_dynamic_step();
        }

        ui.separator();
        // Group 4: Dynamic Min/Max Range Slider (Bounded by unique gaps found in range)
        let max_slider_limit = state.freq_data.len().max(20).max(state.top_max);
        state.top_min = state.top_min.clamp(1, state.top_max);

        ui.label("Rank:");
        if ui
            .add(egui::DragValue::new(&mut state.top_min).range(1..=state.top_max))
            .on_hover_text("Rank Min (N_min)")
            .changed()
        {
            state.top_min = state.top_min.clamp(1, state.top_max);
        }

        render_dual_top_range_slider(ui, &mut state.top_min, &mut state.top_max, max_slider_limit, is_dark);

        if ui
            .add(egui::DragValue::new(&mut state.top_max).range(state.top_min..=max_slider_limit))
            .on_hover_text("Rank Max (N_max)")
            .changed()
        {
            state.top_max = state.top_max.clamp(state.top_min, max_slider_limit);
        }

        ui.separator();
        // Group 5: Action Button / Progress Bar
        if state.is_loading {
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

    // Group 6: Animation Toolbar Row (Cumulative Growth Animation)
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🎬 Animation:").strong());

        if state.is_animating || state.is_precaching {
            if ui
                .button("⏸ Pause")
                .on_hover_text("Pause Growth Chart Animation")
                .clicked()
            {
                state.is_animating = false;
                state.is_precaching = false;
            }
        } else {
            let play_label = if state.anim_current_val > state.min_val && state.anim_current_val < state.max_val {
                "▶ Resume Animation"
            } else {
                "▶ Play Animation"
            };
            if ui
                .button(play_label)
                .on_hover_text("Pre-compute dataset and Play/Resume Cumulative Growth Animation")
                .clicked()
            {
                action = SidebarAction::StartAnimation;
            }
        }

        if ui.button("⏭ Step").on_hover_text("Advance 1 animation step").clicked() {
            state.is_animating = false;
            action = SidebarAction::StepAnimation;
        }

        if ui.button("↺ Reset").on_hover_text("Reset animation bound to Min Prime value").clicked() {
            state.is_animating = false;
            state.anim_current_val = state.min_val;
            action = SidebarAction::Compute;
        }

        ui.separator();

        // Scrubber Slider
        ui.label("Bound:");
        let mut scrub_val = state.anim_current_val.clamp(state.min_val, state.max_val);
        if ui
            .add(
                egui::Slider::new(&mut scrub_val, state.min_val..=state.max_val)
                    .custom_formatter(|v, _| format_compact_num(v as u64))
                    .show_value(true),
            )
            .on_hover_text(format!("Animation Prime Bound: {}", format_thousands(scrub_val)))
            .changed()
        {
            state.anim_current_val = scrub_val;
            if !state.is_animating {
                action = SidebarAction::StepAnimation;
            }
        }

        ui.separator();

        // Step size control (Dynamically scaled with prime range)
        ui.label("Step:");
        let prime_range = state.max_val.saturating_sub(state.min_val).max(50);
        let dynamic_step_speed = (prime_range as f64 / 500.0).max(1.0);
        ui.add(
            egui::DragValue::new(&mut state.anim_step_size)
                .speed(dynamic_step_speed)
                .range(1..=prime_range)
                .custom_formatter(|v, _| format_compact_num(v as u64)),
        )
        .on_hover_text("Prime index additive step increment per frame (dynamically scaled)");

        ui.separator();

        // Speed FPS Slider
        ui.label("Speed:");
        ui.add(
            egui::Slider::new(&mut state.anim_speed_fps, 1.0..=30.0)
                .suffix(" FPS")
                .show_value(true),
        )
        .on_hover_text("Animation frame rate in frames per second");
    });
    ui.add_space(2.0);

    action
}
