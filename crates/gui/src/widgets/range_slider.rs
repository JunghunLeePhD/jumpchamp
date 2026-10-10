//! Single-track, dual-thumb range slider over `1..=limit`.

use egui::{pos2, vec2, Color32, Response, Sense, Stroke, Ui};

use crate::theme::Palette;

const HEIGHT: f32 = 18.0;
const PAD: f32 = 6.0;
const THUMB: f32 = 6.0;

/// Edits `lo..=hi` within `1..=limit`; dragging moves whichever thumb is closer.
pub fn range_slider(
    ui: &mut Ui,
    (lo, hi): (&mut u64, &mut u64),
    limit: u64,
    width: f32,
    fill: Color32,
    palette: &Palette,
) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::drag());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let limit = limit.max(1);
    let (left, right) = (rect.min.x + PAD, rect.max.x - PAD);
    let span = (right - left).max(1.0);
    let x_of = |v: u64| left + (v as f32 / limit as f32).clamp(0.0, 1.0) * span;

    let enabled = ui.is_enabled();
    if enabled && response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let frac = ((pointer.x - left) / span).clamp(0.0, 1.0) as f64;
            let v = ((frac * limit as f64).round() as u64).clamp(1, limit);
            if (pointer.x - x_of(*lo)).abs() <= (pointer.x - x_of(*hi)).abs() {
                *lo = v.min(*hi);
            } else {
                *hi = v.max(*lo);
            }
            response.mark_changed();
        }
    }

    let fill = if enabled { fill } else { palette.disabled };
    let (x_lo, x_hi, y) = (x_of(*lo), x_of(*hi), rect.center().y);
    let painter = ui.painter();
    painter.line_segment(
        [pos2(left, y), pos2(right, y)],
        Stroke::new(4.0_f32, palette.rail),
    );
    painter.line_segment([pos2(x_lo, y), pos2(x_hi, y)], Stroke::new(4.0_f32, fill));

    painter.circle_filled(pos2(x_lo, y), THUMB, palette.card_bg);
    painter.circle_stroke(pos2(x_lo, y), THUMB, Stroke::new(1.5_f32, fill));
    painter.circle_filled(pos2(x_hi, y), THUMB, fill);
    painter.circle_stroke(
        pos2(x_hi, y),
        THUMB,
        Stroke::new(1.5_f32, if enabled { palette.text } else { fill }),
    );

    response
}
