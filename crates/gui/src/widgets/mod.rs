//! Reusable, state-agnostic widgets.

pub mod range_slider;

pub use range_slider::range_slider;

use crate::gui::format::compact;

/// A compact-formatted `DragValue` for prime indices.
pub fn index_input(
    ui: &mut egui::Ui,
    value: &mut u64,
    range: std::ops::RangeInclusive<u64>,
) -> egui::Response {
    let speed = (*value as f64 / 100.0).max(10.0);
    ui.add_sized(
        [85.0, 18.0],
        egui::DragValue::new(value)
            .speed(speed)
            .range(range)
            .custom_formatter(|v, _| compact(v as u64)),
    )
}

/// Hover text that explains why a control is disabled.
pub fn hint(response: egui::Response, enabled: &str, disabled: &str) -> egui::Response {
    let text = if response.enabled() {
        enabled
    } else {
        disabled
    };
    response.on_hover_text(text)
}
