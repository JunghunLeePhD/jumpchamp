// ============================================================================
// Interactive Chart Panel — [0, 1] Normalized Bar Heights & Hover Expand
// ============================================================================

use egui_plot::{Bar, BarChart, Line, Plot, PlotPoint, Text};
use crate::gui::state::AppState;
use crate::gui::theme::viridis_color;

pub fn render(ui: &mut egui::Ui, state: &AppState) {
    let display_len = state.top_n.min(state.freq_data.len());
    let display_data = &state.freq_data[..display_len];

    let total_count: u64 = display_data.iter().map(|&(_, cnt)| cnt).sum();
    let total_f64 = total_count.max(1) as f64;
    let max_count = display_data
        .iter()
        .map(|&(_, cnt)| cnt)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let max_prob = max_count / total_f64;
    let bars_len = display_data.len().max(1) as f64;

    // Extract ctx and layer_id prior to Plot::show to avoid borrow checker conflict on ui
    let ctx = ui.ctx().clone();
    let layer_id = ui.layer_id();

    Plot::new("histogram")
        .width(ui.available_width())
        .height(ui.available_height() - 2.0)
        .set_margin_fraction(egui::Vec2::ZERO) // Fill 100% width and height without edge padding
        .y_axis_label("Probability P(Δ_k) [0, 1]")
        .show_grid(false) // Disable automatic grid to eliminate negative grid lines

        .show_x(false)    // Hide default X-axis line & tick numbers
        .show_y(false)    // Hide default Y-axis cursor line frequency text
        .show_axes([false, false]) // Remove all axis ticks and tick numbers

        .include_x(-0.5)
        .include_x(bars_len - 0.5)
        .include_y(-max_prob * 0.06)
        .include_y(max_prob * 1.12)   // Headroom margin so top percentage labels are never cut off

        .allow_zoom([false, false])   // Permanent 100% full-width and full-height framing lock
        .allow_drag([false, false])   // Prevent canvas drag drift out of bounds
        .allow_scroll(false)
        .label_formatter(|_, _| String::new()) // Suppress default built-in plot hover line text


        .show(ui, |plot_ui| {
            // Detect hovered bar index in plot space
            let hovered_idx = plot_ui.pointer_coordinate().map(|pt| pt.x.round() as i32);

            let mut texts = Vec::new();

            let bars: Vec<Bar> = display_data
                .iter()
                .enumerate()
                .map(|(i, &(gap, count))| {
                    let prob = count as f64 / total_f64;
                    let pct = prob * 100.0;
                    let intensity = count as f64 / max_count;
                    let is_hovered = hovered_idx == Some(i as i32);

                    let mut color = viridis_color(intensity);
                    if is_hovered {
                        // Lighten Viridis color on hover for visual feedback
                        color = egui::Color32::from_rgb(
                            color.r().saturating_add(40),
                            color.g().saturating_add(40),
                            color.b().saturating_add(40),
                        );
                    }

                    let x_pos = i as f64;

                    // 1. Top Percentage Annotation (if enabled)
                    if state.show_pct_labels {
                        texts.push(
                            Text::new(
                                PlotPoint::new(x_pos, prob + max_prob * 0.03),
                                format!("{pct:.1}%"),
                            )
                            .color(if is_hovered { egui::Color32::WHITE } else { egui::Color32::from_rgb(220, 225, 235) }),
                        );
                    }

                    // 2. Bottom Gap Size Label (beneath y = 0 baseline)
                    texts.push(
                        Text::new(
                            PlotPoint::new(x_pos, -max_prob * 0.04),
                            format!("{gap}"),
                        )
                        .color(if is_hovered { egui::Color32::WHITE } else { egui::Color32::from_rgb(180, 190, 210) }),
                    );

                    // Expand width (0.78 -> 0.95) and add white outline stroke when hovered
                    let (bar_width, bar_stroke) = if is_hovered {
                        (0.95, egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 255, 255)))
                    } else {
                        (0.78, egui::Stroke::NONE)
                    };

                    Bar::new(x_pos, prob)
                        .width(bar_width)
                        .fill(color)
                        .stroke(bar_stroke)
                })
                .collect();

            // Draw 4 positive horizontal reference grid lines (if enabled)
            if state.show_grid_lines {
                let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18);
                for fraction in [0.25, 0.50, 0.75, 1.00] {
                    let y_val = max_prob * fraction;
                    let line_points = vec![[-0.5, y_val], [bars_len - 0.5, y_val]];
                    plot_ui.line(Line::new(line_points).color(grid_color).width(1.0_f32));
                }
            }

            // Draw solid baseline at y = 0
            let baseline_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40);
            plot_ui.line(Line::new(vec![[-0.5, 0.0], [bars_len - 0.5, 0.0]]).color(baseline_color).width(1.5_f32));

            plot_ui.bar_chart(BarChart::new(bars));

            // Adaptive Level-of-Detail (LOD): Only render text labels when zoomed in (<= 25 visible bars)
            let bounds = plot_ui.plot_bounds();
            let visible_range = (bounds.max()[0] - bounds.min()[0]).abs();
            if visible_range <= 25.0 {
                for txt in texts {
                    plot_ui.text(txt);
                }
            }

            // High-contrast floating card tooltip with shadow box & border
            if let Some(idx_val) = hovered_idx {
                if idx_val >= 0 && (idx_val as usize) < display_data.len() {
                    let (gap, count) = display_data[idx_val as usize];
                    let prob = count as f64 / total_f64;
                    let pct = prob * 100.0;

                    egui::show_tooltip_at_pointer(
                        &ctx,
                        layer_id,
                        egui::Id::new("chart_hover_card"),
                        |ui| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(16, 20, 28))
                                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 70, 90)))
                                .rounding(6.0_f32)
                                .inner_margin(8.0_f32)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("📊 Gap {gap}"))
                                            .strong()
                                            .color(egui::Color32::from_rgb(90, 200, 250)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Probability P: {prob:.4}"))
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Count: {count} ({pct:.2}%)"))
                                            .color(egui::Color32::from_rgb(180, 190, 205)),
                                    );
                                });
                        },
                    );
                }
            }
        });
}
