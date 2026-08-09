// ============================================================================
// Top Control Bar — User Controls & Single-Track Dual-Thumb Range Slider
// ============================================================================

use crate::gui::state::{AppState, SortOrder};

pub enum SidebarAction {
    None,
    Load,
    Cancel,
    OpenFilePicker,
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
        let max_lim = limit.max(1) as f32;

        let val_to_x = |v: u64| -> f32 {
            let frac = (v as f32 / max_lim).clamp(0.0, 1.0);
            track_left + frac * track_width
        };

        let x_to_val = |x: f32| -> u64 {
            let frac = ((x - track_left) / track_width).clamp(0.0, 1.0);
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

        // 4. Draw Min Thumb (Left Button Handle)
        let thumb_r = 6.0_f32;
        let min_circle = egui::pos2(x_min, track_y);
        painter.circle_filled(min_circle, thumb_r, egui::Color32::from_rgb(220, 225, 235));
        painter.circle_stroke(min_circle, thumb_r, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(90, 200, 250)));

        // 5. Draw Max Thumb (Right Button Handle)
        let max_circle = egui::pos2(x_max, track_y);
        painter.circle_filled(max_circle, thumb_r, egui::Color32::from_rgb(90, 200, 250));
        painter.circle_stroke(max_circle, thumb_r, egui::Stroke::new(1.5_f32, egui::Color32::WHITE));
    }

    response.on_hover_text(format!("Index Range n: {} ~ {}", min_val, max_val))
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    let max_limit = state
        .metadata
        .as_ref()
        .map(|m| m.total_rows)
        .unwrap_or(10_000_000);

    ui.add_space(2.0);
    // Non-wrapping single horizontal line container
    ui.horizontal(|ui| {
        // Group 1: Dataset File Picker
        ui.label("📂 File:");
        ui.add(egui::TextEdit::singleline(&mut state.file_path).desired_width(120.0));
        if ui.button("…").on_hover_text("Browse for .parquet file").clicked() {
            action = SidebarAction::OpenFilePicker;
        }

        ui.separator();
        // Group 2: Sort Order (Right next to File option)
        ui.radio_value(&mut state.sort_by, SortOrder::ByFrequency, "Freq");
        ui.radio_value(&mut state.sort_by, SortOrder::ByGapSize, "Gap");

        ui.separator();
        // Group 3: Clean Index Range (No text clutter)
        if ui.add(egui::DragValue::new(&mut state.min_idx).speed(100_000)).changed() {
            state.min_idx = state.min_idx.clamp(1, max_limit);
            if state.min_idx > state.max_idx {
                state.max_idx = state.min_idx;
            }
        }

        render_dual_range_slider(ui, &mut state.min_idx, &mut state.max_idx, max_limit);

        if ui.add(egui::DragValue::new(&mut state.max_idx).speed(100_000)).changed() {
            state.max_idx = state.max_idx.clamp(1, max_limit);
            if state.max_idx < state.min_idx {
                state.min_idx = state.max_idx;
            }
        }

        ui.separator();
        // Group 4: Clean Top N Slider (No text label)
        ui.add_sized([70.0_f32, 18.0_f32], egui::Slider::new(&mut state.top_n, 5..=200));

        ui.separator();
        // Group 5: Action Button / Progress Bar (Strict 1 Line)
        if state.is_loading {
            ui.add_sized(
                [80.0_f32, 18.0_f32],
                egui::ProgressBar::new(state.progress).show_percentage(),
            );
            if ui.button("✖ Cancel").clicked() {
                action = SidebarAction::Cancel;
            }
        } else if ui.button("▶ Load").clicked() {
            action = SidebarAction::Load;
        }

    });
    ui.add_space(2.0);

    action
}
