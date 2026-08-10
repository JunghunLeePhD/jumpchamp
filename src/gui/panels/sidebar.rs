// ============================================================================
// Top Control Bar — User Controls & Single-Track Dual-Thumb Range Slider
// ============================================================================

use crate::gui::state::{AppState, SortOrder};

pub enum SidebarAction {
    None,
    Compute,
    Cancel,
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

fn render_dual_range_slider(
    ui: &mut egui::Ui,
    min_val: &mut u64,
    max_val: &mut u64,
    limit: u64,
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
            egui::Stroke::new(4.0_f32, egui::Color32::from_rgb(45, 55, 75)),
        );

        // 2. Draw Active Range Fill Line
        painter.line_segment(
            [egui::pos2(x_min, track_y), egui::pos2(x_max, track_y)],
            egui::Stroke::new(4.0_f32, egui::Color32::from_rgb(90, 200, 250)),
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
        painter.circle_filled(min_circle, thumb_r, egui::Color32::from_rgb(220, 225, 235));
        painter.circle_stroke(min_circle, thumb_r, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(90, 200, 250)));

        // 5. Draw Max Thumb
        let max_circle = egui::pos2(x_max, track_y);
        painter.circle_filled(max_circle, thumb_r, egui::Color32::from_rgb(90, 200, 250));
        painter.circle_stroke(max_circle, thumb_r, egui::Stroke::new(1.5_f32, egui::Color32::WHITE));
    }

    response.on_hover_text(format!(
        "Prime Numerical Range: {} ~ {} ({})",
        format_compact_num(*min_val),
        format_compact_num(*max_val),
        format!("{}..{}", min_val, max_val)
    ))
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    let max_limit = 100_000_000_000u64; // 1e11 (100 Billion)

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        // Group 1: Gap order k parameter
        ui.label("k:");
        if ui
            .add(egui::DragValue::new(&mut state.k).range(1..=20))
            .on_hover_text("Gap order k (e.g. k=1 for consecutive prime gaps)")
            .changed()
        {
            action = SidebarAction::Compute;
        }

        ui.separator();
        // Group 2: Sort Order
        if ui.radio_value(&mut state.sort_by, SortOrder::ByFrequency, "Freq").changed() {
            action = SidebarAction::Compute;
        }
        if ui.radio_value(&mut state.sort_by, SortOrder::ByGapSize, "Gap").changed() {
            action = SidebarAction::Compute;
        }

        ui.separator();
        // Group 3: Numerical Prime Value Range [N_min, N_max]
        let min_speed = (state.min_val as f64 / 100.0).max(10.0);
        if ui
            .add(
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
        }

        render_dual_range_slider(ui, &mut state.min_val, &mut state.max_val, max_limit);

        let max_speed = (state.max_val as f64 / 100.0).max(10.0);
        if ui
            .add(
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
        }

        ui.separator();
        // Group 4: Dynamic Top N Slider (Bounded by unique gaps found in range)
        let dynamic_top_n_max = state.freq_data.len().max(5);
        state.top_n = state.top_n.clamp(5, dynamic_top_n_max);
        ui.add_sized(
            [70.0_f32, 18.0_f32],
            egui::Slider::new(&mut state.top_n, 5..=dynamic_top_n_max),
        );

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

    action
}
