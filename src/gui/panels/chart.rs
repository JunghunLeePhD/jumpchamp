// ============================================================================
// Interactive Chart Panel — Permanent [0, 1] Normalized Probability Histogram
// ============================================================================

use egui_plot::{Bar, BarChart, Plot, PlotPoint, Text};
use crate::gui::state::AppState;
use crate::gui::theme::viridis_color;

pub fn render(ui: &mut egui::Ui, state: &AppState) {
    let total_count: u64 = state.freq_data.iter().map(|&(_, cnt)| cnt).sum();
    let total_f64 = total_count.max(1) as f64;
    let max_count = state
        .freq_data
        .first()
        .map(|&(_, cnt)| cnt)
        .unwrap_or(1)
        .max(1) as f64;

    let max_prob = max_count / total_f64;
    let mut texts = Vec::new();

    let bars: Vec<Bar> = state
        .freq_data
        .iter()
        .enumerate()
        .map(|(i, &(gap, count))| {
            let prob = count as f64 / total_f64;
            let pct = prob * 100.0;
            let intensity = count as f64 / max_count;
            let color = viridis_color(intensity);

            // On-graph text label above bar showing probability
            texts.push(
                Text::new(
                    PlotPoint::new(i as f64, prob + max_prob * 0.02),
                    format!("{prob:.4}"),
                )
                .color(egui::Color32::from_rgb(220, 225, 235)),
            );

            Bar::new(i as f64, prob)
                .width(0.8)
                .fill(color)
                .name(format!("Gap {gap}: {count} ({pct:.2}%) | P = {prob:.4}"))
        })
        .collect();

    Plot::new("histogram")
        .height(ui.available_height() - 4.0)
        .x_axis_label(format!("{}-Step Gap Size (Δ_{})", state.k, state.k))
        .y_axis_label("Probability P(Δ_k) [0, 1]")
        .allow_zoom(true)
        .allow_drag(true)
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(BarChart::new(bars).name("freq"));
            for txt in texts {
                plot_ui.text(txt);
            }
        });
}
