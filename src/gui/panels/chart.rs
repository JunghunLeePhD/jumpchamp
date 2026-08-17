// ============================================================================
// Interactive Chart Panel — [0, 1] Normalized Bar Heights & Hover Expand
// ============================================================================

use std::collections::HashSet;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoint, Text};
use crate::gui::state::AppState;
use crate::gui::theme::{self, viridis_color};
use crate::gui::utils::{format_compact_num, format_thousands};

pub fn render(ui: &mut egui::Ui, state: &AppState) {
    let is_dark = theme::is_dark(state.theme_mode);
    let accent = theme::accent_color(is_dark);
    let text_pri = theme::text_primary(is_dark);
    let text_sec = theme::text_secondary(is_dark);
    let card_bg = theme::card_bg(is_dark);
    let card_border = theme::card_border(is_dark);

    if state.freq_data.is_empty() && !state.is_loading && state.error_msg.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.35);
            ui.label(
                egui::RichText::new("🚀 Ready to Compute")
                    .size(24.0)
                    .strong()
                    .color(accent),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Configure your range parameters above and click ▶ Compute to start analysis.")
                    .size(15.0)
                    .color(text_sec),
            );
        });
        return;
    }

    let total_gaps = state.freq_data.len();
    let start_idx = (state.top_min.saturating_sub(1)).min(total_gaps);
    let end_idx = state.top_max.min(total_gaps).max(start_idx);
    let display_data = &state.freq_data[start_idx..end_idx];

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
    let available_h = ui.available_height();

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
            // Detect hovered bar index in plot space only when pointer is actively hovering over the plot canvas
            let is_canvas_hovered = plot_ui.response().hovered();
            let hovered_idx = if is_canvas_hovered {
                plot_ui.pointer_coordinate().map(|pt| pt.x.round() as i32)
            } else {
                None
            };

            // Identify top 3 gaps by frequency (count) to ensure their labels remain visible even when zoomed out
            let mut indexed_counts: Vec<(usize, u64)> = display_data
                .iter()
                .enumerate()
                .map(|(i, &(_, cnt))| (i, cnt))
                .collect();
            indexed_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let top3_indices: HashSet<usize> = indexed_counts
                .iter()
                .take(3)
                .map(|&(i, _)| i)
                .collect();

            let mut all_texts = Vec::new();
            let mut top3_texts = Vec::new();

            let bars: Vec<Bar> = display_data
                .iter()
                .enumerate()
                .map(|(i, &(gap, count))| {
                    let prob = count as f64 / total_f64;
                    let pct = prob * 100.0;
                    let intensity = count as f64 / max_count;
                    let is_hovered = hovered_idx == Some(i as i32);
                    let is_top3 = top3_indices.contains(&i);

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
                        let pct_text = Text::new(
                            PlotPoint::new(x_pos, prob + max_prob * 0.03),
                            format!("{pct:.1}%"),
                        )
                        .color(if is_hovered { accent } else { text_pri });

                        if is_top3 {
                            top3_texts.push(pct_text.clone());
                        }
                        all_texts.push(pct_text);
                    }

                    // 2. Bottom Gap Size Label (beneath y = 0 baseline)
                    let gap_text = Text::new(
                        PlotPoint::new(x_pos, -max_prob * 0.04),
                        format!("{gap}"),
                    )
                    .color(if is_hovered { text_pri } else { text_sec });

                    if is_top3 {
                        top3_texts.push(gap_text.clone());
                    }
                    all_texts.push(gap_text);

                    // Expand width (0.78 -> 0.95) and add accent outline stroke when hovered
                    let (bar_width, bar_stroke) = if is_hovered {
                        (0.95, egui::Stroke::new(2.0_f32, accent))
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
                let grid_c = theme::grid_color(is_dark);
                for fraction in [0.25, 0.50, 0.75, 1.00] {
                    let y_val = max_prob * fraction;
                    let line_points = vec![[-0.5, y_val], [bars_len - 0.5, y_val]];
                    plot_ui.line(Line::new(line_points).color(grid_c).width(1.0_f32));
                }
            }

            // Draw solid baseline at y = 0
            let baseline_c = theme::baseline_color(is_dark);
            plot_ui.line(Line::new(vec![[-0.5, 0.0], [bars_len - 0.5, 0.0]]).color(baseline_c).width(1.5_f32));

            plot_ui.bar_chart(BarChart::new(bars));

            // Adaptive Level-of-Detail (LOD):
            // When zoomed in (<= 25 visible bars), render all bar annotations.
            // When zoomed out (> 25 visible bars), render annotations for the Top 3 gaps.
            let bounds = plot_ui.plot_bounds();
            let visible_range = (bounds.max()[0] - bounds.min()[0]).abs();
            if visible_range <= 25.0 {
                for txt in all_texts {
                    plot_ui.text(txt);
                }
            } else {
                for txt in top3_texts {
                    plot_ui.text(txt);
                }
            }

            // High-contrast floating card tooltip with shadow box & border
            if let Some(idx_val) = hovered_idx {
                if idx_val >= 0 && (idx_val as usize) < display_data.len() {
                    let (gap, count) = display_data[idx_val as usize];
                    let prob = count as f64 / total_f64;
                    let pct = prob * 100.0;

                    // Calculate frequency rank across all gaps (1-based, sorted by count descending)
                    let rank = state
                        .freq_data
                        .iter()
                        .filter(|&&(g, cnt)| cnt > count || (cnt == count && g < gap))
                        .count()
                        + 1;

                    let rank_color = if is_dark {
                        egui::Color32::from_rgb(255, 200, 80)
                    } else {
                        egui::Color32::from_rgb(200, 130, 0)
                    };

                    egui::show_tooltip_at_pointer(
                        &ctx,
                        layer_id,
                        egui::Id::new("chart_hover_card"),
                        |ui| {
                            egui::Frame::none()
                                .fill(card_bg)
                                .stroke(egui::Stroke::new(1.0_f32, card_border))
                                .rounding(6.0_f32)
                                .inner_margin(8.0_f32)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("📊 Gap {gap}"))
                                            .strong()
                                            .size(16.0)
                                            .color(accent),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Rank: #{rank}"))
                                            .strong()
                                            .size(14.5)
                                            .color(rank_color),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Percentage: {pct:.2}%"))
                                            .strong()
                                            .size(15.0)
                                            .color(text_pri),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Count: {}", format_thousands(count)))
                                            .size(14.0)
                                            .color(text_sec),
                                    );
                                });
                        },
                    );
                }
            }
        });

    // Floating Vertical Heat Map Count Meter on the top-right side of the chart canvas
    if state.show_heatmap_meter && !display_data.is_empty() {
        let min_cnt = display_data.iter().map(|&(_, cnt)| cnt).min().unwrap_or(0);
        let max_cnt = display_data.iter().map(|&(_, cnt)| cnt).max().unwrap_or(0);

        // Container height matching half of the chart canvas height
        let meter_height = ((available_h - 90.0) * 0.5).clamp(100.0, 400.0);

        egui::Area::new(egui::Id::new("heatmap_count_meter"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 92.0))
            .interactable(true)
            .show(&ctx, |ui| {
                egui::Frame::none()
                    .fill(card_bg)
                    .stroke(egui::Stroke::new(1.0_f32, card_border))
                    .rounding(6.0_f32)
                    .inner_margin(6.0_f32)
                    .show(ui, |ui| {
                        // Vertical strip + tick marks region allocation (2/3 bar width)
                        let strip_size = egui::vec2(75.0, meter_height);
                        let (rect, response) = ui.allocate_exact_size(strip_size, egui::Sense::hover());
                        if ui.is_rect_visible(rect) {
                            let painter = ui.painter();
                            let bar_width = 9.5_f32; // 2/3 of original 14px
                            let bar_rect = egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.min.x + bar_width, rect.max.y),
                            );

                            // Render vertical Viridis gradient (Top = 1.0 Yellow, Bottom = 0.0 Dark Blue/Purple)
                            let steps = 60;
                            let step_h = bar_rect.height() / steps as f32;
                            for i in 0..steps {
                                let t = 1.0 - (i as f64 / (steps - 1) as f64);
                                let color = viridis_color(t);
                                let y0 = bar_rect.min.y + i as f32 * step_h;
                                let y1 = (y0 + step_h + 0.5).min(bar_rect.max.y);
                                let sub_rect = egui::Rect::from_min_max(
                                    egui::pos2(bar_rect.min.x, y0),
                                    egui::pos2(bar_rect.max.x, y1),
                                );
                                painter.rect_filled(sub_rect, 0.0, color);
                            }

                            // Outline around the vertical gradient bar
                            painter.rect_stroke(
                                bar_rect,
                                2.0,
                                egui::Stroke::new(1.0_f32, card_border),
                            );

                            // Multi-level ticks and scale text labels at 100%, 75%, 50%, 25%, 0%
                            let tick_levels = [
                                (0.00_f32, 1.00_f64),
                                (0.25_f32, 0.75_f64),
                                (0.50_f32, 0.50_f64),
                                (0.75_f32, 0.25_f64),
                                (1.00_f32, 0.00_f64),
                            ];

                            for (y_frac, val_frac) in tick_levels {
                                let y_pos = bar_rect.min.y + y_frac * bar_rect.height();
                                painter.line_segment(
                                    [
                                        egui::pos2(bar_rect.max.x, y_pos),
                                        egui::pos2(bar_rect.max.x + 4.0, y_pos),
                                    ],
                                    egui::Stroke::new(1.0_f32, card_border),
                                );

                                let count_val = (max_cnt as f64 * val_frac) as u64;
                                let text_str = format_compact_num(count_val);

                                painter.text(
                                    egui::pos2(bar_rect.max.x + 7.0, y_pos),
                                    egui::Align2::LEFT_CENTER,
                                    text_str,
                                    egui::FontId::proportional(11.0),
                                    text_sec,
                                );
                            }

                            response.on_hover_text(format!(
                                "Vertical Count Heatmap Meter (Viridis)\nHigh (Top): {}\nLow (Bottom): {}",
                                format_thousands(max_cnt),
                                format_thousands(min_cnt)
                            ));
                        }
                    });
            });
    }
}
