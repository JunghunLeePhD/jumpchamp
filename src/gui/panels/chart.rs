// ============================================================================
// Interactive Chart Panel — egui_plot Histogram & Scatter Views
// ============================================================================

use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use crate::gui::state::{AppState, ChartMode};
use crate::gui::theme::viridis_color;

pub fn render(ui: &mut egui::Ui, state: &AppState) {
    match state.chart_mode {
        ChartMode::Histogram => render_histogram(ui, state),
        ChartMode::Scatter => render_scatter(ui, state),
    }
}

fn render_histogram(ui: &mut egui::Ui, state: &AppState) {
    let max_count = state
        .freq_data
        .first()
        .map(|&(_, cnt)| cnt)
        .unwrap_or(1)
        .max(1) as f64;

    let bars: Vec<Bar> = state
        .freq_data
        .iter()
        .enumerate()
        .map(|(i, &(gap, count))| {
            let intensity = count as f64 / max_count;
            let color = viridis_color(intensity);
            Bar::new(i as f64, count as f64)
                .width(0.8)
                .fill(color)
                .name(format!("Gap {gap}"))
        })
        .collect();

    Plot::new("histogram")
        .height(ui.available_height() - 4.0)
        .x_axis_label(format!("{}-Step Gap Size (Δ_{})", state.k, state.k))
        .y_axis_label("Frequency")
        .allow_zoom(true)
        .allow_drag(true)
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(BarChart::new(bars).name("freq"));
        });
}

fn render_scatter(ui: &mut egui::Ui, state: &AppState) {
    let points = PlotPoints::new(state.scatter_data.clone());
    Plot::new("scatter")
        .height(ui.available_height() - 4.0)
        .x_axis_label("Prime Index n")
        .y_axis_label(format!("Gap Δ_{}", state.k))
        .allow_zoom(true)
        .allow_drag(true)
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(points).name("gaps").color(egui::Color32::from_rgb(59, 82, 139)));
        });
}

