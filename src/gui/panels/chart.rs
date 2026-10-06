// ============================================================================
// Interactive Chart Panel — [0, 1] Normalized Bar Heights, Focus Lock & Hover
// ============================================================================

use egui_plot::{Bar, BarChart, Line, Plot, PlotPoint, Text};
use crate::gui::state::AppState;
use crate::gui::theme::{self, viridis_color};
use crate::gui::utils::{format_compact_num, format_thousands};

/// Finds the top 5 gap indices by frequency count in a single O(N) pass with zero heap allocations.
/// Returns an array of `(bar_index, count)` sorted in descending order of count.
#[inline]
pub fn find_top5_indices(display_data: &[(u64, u64)]) -> [(usize, u64); 5] {
    let mut top5 = [(0usize, 0u64); 5];
    for (i, &(_, count)) in display_data.iter().enumerate() {
        if count > top5[4].1 {
            let mut pos = 4;
            while pos > 0 && count > top5[pos - 1].1 {
                top5[pos] = top5[pos - 1];
                pos -= 1;
            }
            top5[pos] = (i, count);
        }
    }
    top5
}

/// Checks whether a bar index belongs to the top 5 gaps with non-zero count.
#[inline]
pub fn is_top5(top5: &[(usize, u64); 5], data_len: usize, idx: usize) -> bool {
    let valid_len = data_len.min(5);
    top5[..valid_len].iter().any(|&(top_idx, count)| count > 0 && top_idx == idx)
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
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
                egui::RichText::new("Configure your range parameters above and click ▶ Play or ⏭ Step to start animation.")
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

    let ctx = ui.ctx().clone();
    let layer_id = ui.layer_id();
    let available_h = ui.available_height();
    let y_axis_label = format!("Probability P(Δ_{}) [0, 1]", state.k);

    let mut new_selected_gap = state.selected_gap;
    let mut hovered_item: Option<(usize, u64, u64)> = None; // (index, gap, count)
    let mut pinned_info: Option<(egui::Pos2, u64, u64, f64, usize)> = None; // (screen_pos, gap, count, pct, rank)

    Plot::new("histogram")
        .width(ui.available_width())
        .height(ui.available_height() - 2.0)
        .set_margin_fraction(egui::Vec2::ZERO)
        .y_axis_label(y_axis_label)
        .show_grid(false)
        .show_x(false)
        .show_y(false)
        .show_axes([false, false])
        .include_x(-0.5)
        .include_x(bars_len - 0.5)
        .include_y(-max_prob * 0.06)
        .include_y(max_prob * 1.15) // Headroom for anchored pinned tooltip card
        .allow_zoom([false, false])
        .allow_drag([false, false])
        .allow_scroll(false)
        .label_formatter(|_, _| String::new())
        .show(ui, |plot_ui| {
            let is_canvas_hovered = plot_ui.response().hovered();
            let hovered_idx = if is_canvas_hovered {
                plot_ui.pointer_coordinate().map(|pt| pt.x.round() as i32)
            } else {
                None
            };

            // Handle Bar Click Selection / Deselection (Toggle Focus Lock)
            if plot_ui.response().clicked() {
                if let Some(coord) = plot_ui.pointer_coordinate() {
                    let clicked_idx = coord.x.round() as i32;
                    if clicked_idx >= 0 && (clicked_idx as usize) < display_data.len() {
                        let clicked_gap = display_data[clicked_idx as usize].0;
                        if new_selected_gap == Some(clicked_gap) {
                            new_selected_gap = None; // Deselect on clicking the same bar
                        } else {
                            new_selected_gap = Some(clicked_gap); // Select newly clicked bar
                        }
                    } else {
                        new_selected_gap = None; // Deselect when clicking empty space
                    }
                }
            }

            // Single O(N) pass to find top 5 gaps with zero heap allocations
            let top5 = find_top5_indices(display_data);

            let mut all_texts = Vec::with_capacity(display_data.len() * 2);
            let mut top5_texts = Vec::with_capacity(10);

            let bars: Vec<Bar> = display_data
                .iter()
                .enumerate()
                .map(|(i, &(gap, count))| {
                    let prob = count as f64 / total_f64;
                    let pct = prob * 100.0;
                    let intensity = count as f64 / max_count;
                    let is_hovered = hovered_idx == Some(i as i32);
                    let is_selected = new_selected_gap == Some(gap);
                    let is_top5 = is_top5(&top5, display_data.len(), i);

                    if is_hovered {
                        hovered_item = Some((i, gap, count));
                    }

                    let x_pos = i as f64;

                    // Compute Screen Position of the Pinned Bar's Peak for Anchored Tooltip
                    if is_selected {
                        let screen_pos = plot_ui.screen_from_plot(PlotPoint::new(x_pos, prob));
                        let rank = state
                            .freq_data
                            .iter()
                            .filter(|&&(g, cnt)| cnt > count || (cnt == count && g < gap))
                            .count()
                            + 1;
                        pinned_info = Some((screen_pos, gap, count, pct, rank));
                    }

                    let mut color = viridis_color(intensity);
                    if is_selected {
                        color = if is_dark {
                            egui::Color32::from_rgb(255, 215, 0)
                        } else {
                            egui::Color32::from_rgb(230, 160, 0)
                        };
                    } else if is_hovered {
                        color = egui::Color32::from_rgb(
                            color.r().saturating_add(40),
                            color.g().saturating_add(40),
                            color.b().saturating_add(40),
                        );
                    }

                    // 1. Top Percentage Annotation (hide if pinned tooltip card is positioned right above it)
                    if state.show_pct_labels && !is_selected {
                        let text_color = if is_hovered { accent } else { text_pri };
                        let pct_text = Text::new(
                            PlotPoint::new(x_pos, prob + max_prob * 0.03),
                            format!("{pct:.1}%"),
                        )
                        .color(text_color);

                        if is_top5 {
                            top5_texts.push(pct_text.clone());
                        }
                        all_texts.push(pct_text);
                    }

                    // 2. Bottom Gap Size Label
                    let gap_color = if is_selected {
                        accent
                    } else if is_hovered {
                        text_pri
                    } else {
                        text_sec
                    };

                    let gap_text = Text::new(
                        PlotPoint::new(x_pos, -max_prob * 0.04),
                        format!("{gap}"),
                    )
                    .color(gap_color);

                    if is_top5 || is_selected {
                        top5_texts.push(gap_text.clone());
                    }
                    all_texts.push(gap_text);

                    // Width and outline stroke (Prominent stroke on selected/hovered)
                    let (bar_width, bar_stroke) = if is_selected {
                        (0.95, egui::Stroke::new(2.5_f32, accent))
                    } else if is_hovered {
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

            // Draw 4 positive horizontal reference grid lines
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
            let bounds = plot_ui.plot_bounds();
            let visible_range = (bounds.max()[0] - bounds.min()[0]).abs();
            if visible_range <= 25.0 {
                for txt in all_texts {
                    plot_ui.text(txt);
                }
            } else {
                for txt in top5_texts {
                    plot_ui.text(txt);
                }
            }

            // Transient Hover Tooltip Card (Shown when hovering over a different, non-pinned bar)
            if state.show_bar_tooltip {
                if let Some((_, gap, count)) = hovered_item {
                    if new_selected_gap != Some(gap) {
                    let prob = count as f64 / total_f64;
                    let pct = prob * 100.0;

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
                                        egui::RichText::new(format!("📊 {}-Step Gap (Δ_{}) = {gap}", state.k, state.k))
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
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new("Click to lock focus")
                                            .italics()
                                            .size(11.0)
                                            .color(text_sec),
                                    );
                                });
                        },
                    );
                }
            }
        }
        });

    state.selected_gap = new_selected_gap;

    // Always-Visible Tooltip Card Anchored Directly Above the Pinned Bar (Clean, without X button)
    if let Some((pinned_screen_pos, pinned_gap, count, pct, rank)) = pinned_info {
        let rank_color = if is_dark {
            egui::Color32::from_rgb(255, 200, 80)
        } else {
            egui::Color32::from_rgb(200, 130, 0)
        };

        // Anchor slightly above the bar peak
        let anchor_pos = egui::pos2(pinned_screen_pos.x, pinned_screen_pos.y - 10.0);

        egui::Area::new(egui::Id::new("pinned_bar_anchored_tooltip"))
            .fixed_pos(anchor_pos)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .interactable(false)
            .show(&ctx, |ui| {
                egui::Frame::none()
                    .fill(card_bg)
                    .stroke(egui::Stroke::new(1.5_f32, accent))
                    .rounding(6.0_f32)
                    .inner_margin(7.0_f32)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("📌 Gap Δ_{} = {pinned_gap}", state.k))
                                .strong()
                                .size(14.5)
                                .color(accent),
                        );
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("#{rank}"))
                                    .strong()
                                    .size(13.5)
                                    .color(rank_color),
                            );
                            ui.label(
                                egui::RichText::new(format!("{pct:.2}%"))
                                    .strong()
                                    .size(13.5)
                                    .color(text_pri),
                            );
                            ui.label(
                                egui::RichText::new(format!("({})", format_compact_num(count)))
                                    .size(12.5)
                                    .color(text_sec),
                            );
                        });
                    });
            });
    }

    // Floating Vertical Heat Map Count Meter on the top-right side of the chart canvas
    if state.show_heatmap_meter && !display_data.is_empty() {
        let min_cnt = display_data.iter().map(|&(_, cnt)| cnt).min().unwrap_or(0);
        let max_cnt = display_data.iter().map(|&(_, cnt)| cnt).max().unwrap_or(0);

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
                        let strip_size = egui::vec2(75.0, meter_height);
                        let (rect, response) = ui.allocate_exact_size(strip_size, egui::Sense::hover());
                        if ui.is_rect_visible(rect) {
                            let painter = ui.painter();
                            let bar_width = 9.5_f32;
                            let bar_rect = egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.min.x + bar_width, rect.max.y),
                            );

                            let steps = 60;
                            let step_h = bar_rect.height() / steps as f32;
                            for i in 0..steps {
                                let t = 1.0 - (i as f64 / (steps - 1) as f64);
                                let color = viridis_color(t);
                                let y0 = bar_rect.min.y + i as f32 * step_h;
                                let y1 = (y0 + step_h + 0.5).min(bar_rect.max.y);
                                sub_rect_painter(painter, bar_rect.min.x, y0, bar_rect.max.x, y1, color);
                            }

                            painter.rect_stroke(
                                bar_rect,
                                2.0,
                                egui::Stroke::new(1.0_f32, card_border),
                            );

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
                                "Vertical Count Heatmap Meter (Viridis)\nStep Size: k = {}\nHigh (Top): {}\nLow (Bottom): {}",
                                state.k,
                                format_thousands(max_cnt),
                                format_thousands(min_cnt)
                            ));
                        }
                    });
            });
    }
}

fn sub_rect_painter(painter: &egui::Painter, x0: f32, y0: f32, x1: f32, y1: f32, color: egui::Color32) {
    let sub_rect = egui::Rect::from_min_max(
        egui::pos2(x0, y0),
        egui::pos2(x1, y1),
    );
    painter.rect_filled(sub_rect, 0.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_top5_empty() {
        let data: Vec<(u64, u64)> = vec![];
        let top5 = find_top5_indices(&data);
        assert_eq!(top5, [(0, 0); 5]);
        assert!(!is_top5(&top5, data.len(), 0));
    }

    #[test]
    fn test_find_top5_fewer_than_5() {
        let data = vec![(2, 100), (4, 300), (6, 200)];
        let top5 = find_top5_indices(&data);
        assert_eq!(top5[0], (1, 300));
        assert_eq!(top5[1], (2, 200));
        assert_eq!(top5[2], (0, 100));
        assert_eq!(top5[3], (0, 0));
        assert_eq!(top5[4], (0, 0));

        assert!(is_top5(&top5, data.len(), 0));
        assert!(is_top5(&top5, data.len(), 1));
        assert!(is_top5(&top5, data.len(), 2));
        assert!(!is_top5(&top5, data.len(), 3));
    }

    #[test]
    fn test_find_top5_exact_5() {
        let data = vec![(2, 10), (4, 50), (6, 30), (8, 20), (10, 40)];
        let top5 = find_top5_indices(&data);
        assert_eq!(top5[0], (1, 50));
        assert_eq!(top5[1], (4, 40));
        assert_eq!(top5[2], (2, 30));
        assert_eq!(top5[3], (3, 20));
        assert_eq!(top5[4], (0, 10));

        for i in 0..5 {
            assert!(is_top5(&top5, data.len(), i));
        }
        assert!(!is_top5(&top5, data.len(), 5));
    }

    #[test]
    fn test_find_top5_more_than_5() {
        let data = vec![
            (2, 10),   // idx 0
            (4, 500),  // idx 1 - rank 1
            (6, 400),  // idx 2 - rank 2
            (8, 20),   // idx 3
            (10, 300), // idx 4 - rank 3
            (12, 200), // idx 5 - rank 4
            (14, 5),   // idx 6
            (16, 100), // idx 7 - rank 5
            (18, 50),  // idx 8
        ];
        let top5 = find_top5_indices(&data);
        assert_eq!(top5[0], (1, 500));
        assert_eq!(top5[1], (2, 400));
        assert_eq!(top5[2], (4, 300));
        assert_eq!(top5[3], (5, 200));
        assert_eq!(top5[4], (7, 100));

        // The top 5 indices (1, 2, 4, 5, 7) should be true
        assert!(is_top5(&top5, data.len(), 1));
        assert!(is_top5(&top5, data.len(), 2));
        assert!(is_top5(&top5, data.len(), 4));
        assert!(is_top5(&top5, data.len(), 5));
        assert!(is_top5(&top5, data.len(), 7));

        // Non-top-5 indices should be false
        assert!(!is_top5(&top5, data.len(), 0));
        assert!(!is_top5(&top5, data.len(), 3));
        assert!(!is_top5(&top5, data.len(), 6));
        assert!(!is_top5(&top5, data.len(), 8));
    }

    #[test]
    fn test_find_top5_zeros() {
        let data = vec![(2, 0), (4, 0), (6, 0)];
        let top5 = find_top5_indices(&data);
        assert_eq!(top5, [(0, 0); 5]);
        for i in 0..3 {
            assert!(!is_top5(&top5, data.len(), i));
        }
    }

    #[test]
    fn test_find_top5_ties() {
        let data = vec![(2, 100), (4, 100), (6, 100), (8, 100), (10, 100), (12, 50)];
        let top5 = find_top5_indices(&data);
        for item in &top5 {
            assert_eq!(item.1, 100);
        }
        assert!(is_top5(&top5, data.len(), 0));
        assert!(is_top5(&top5, data.len(), 1));
        assert!(is_top5(&top5, data.len(), 2));
        assert!(is_top5(&top5, data.len(), 3));
        assert!(is_top5(&top5, data.len(), 4));
        assert!(!is_top5(&top5, data.len(), 5));
    }
}
