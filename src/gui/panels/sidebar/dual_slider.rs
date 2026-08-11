// ============================================================================
// Dual-Thumb Range Slider Widget & Helpers
// ============================================================================

use crate::gui::theme;
use crate::gui::utils::format_compact_num;

/// Renders custom low-level egui painter implementation for a single-track dual-thumb range slider.
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

/// Renders dual slider for Prime Index Range (`n = min ~ max`).
pub fn render_dual_range_slider(
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

/// Renders dual slider for Rank Range (`Rank min ~ max`).
pub fn render_dual_top_range_slider(
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
