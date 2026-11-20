//! 2D spectral prime-gap density heatmap rendered via `egui::ColorImage` texture.

use egui::{pos2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, Ui};

use jumpchamp_core::storage::SpectralHeatmap;

use crate::actions::Action;
use crate::format::{compact, thousands};
use crate::state::AppState;
use crate::theme::{self, Palette};

const MAX_GAP_PRESETS: [u64; 4] = [60, 120, 240, 540];
const LEFT_MARGIN: f32 = 46.0;
const RIGHT_MARGIN: f32 = 64.0;
const TOP_MARGIN: f32 = 10.0;
const BOTTOM_MARGIN: f32 = 24.0;

/// Converts a [`SpectralHeatmap`] matrix into an `egui::ColorImage` (smallest gap at bottom).
pub fn build_color_image(spectral: &SpectralHeatmap, palette: &Palette) -> egui::ColorImage {
    if spectral.cols == 0 || spectral.rows == 0 {
        return egui::ColorImage::new([1, 1], Color32::TRANSPARENT);
    }

    let zero_bg = if palette.dark {
        Color32::from_rgb(18, 22, 30)
    } else {
        Color32::from_rgb(236, 240, 246)
    };

    let mut pixels = Vec::with_capacity(spectral.cols * spectral.rows);
    for py in 0..spectral.rows {
        let gap_row = spectral.rows - 1 - py;
        for col in 0..spectral.cols {
            let idx = gap_row * spectral.cols + col;
            let count = spectral.counts[idx];
            let color = if count == 0 {
                zero_bg
            } else {
                theme::viridis(spectral.intensities[idx] as f64)
            };
            pixels.push(color);
        }
    }

    egui::ColorImage {
        size: [spectral.cols, spectral.rows],
        pixels,
    }
}

pub fn render(
    ui: &mut Ui,
    state: &mut AppState,
    texture: Option<&TextureHandle>,
    palette: &Palette,
) -> Option<Action> {
    let mut action = controls_bar(ui, state, palette);
    ui.separator();

    let Some(spectral) = state.spectral.as_ref() else {
        action = action.or(empty_prompt(ui, state, palette));
        return action;
    };

    if spectral.cols == 0 || spectral.rows == 0 {
        action = action.or(empty_prompt(ui, state, palette));
        return action;
    }

    let avail = ui.available_size();
    if avail.x < LEFT_MARGIN + RIGHT_MARGIN + 20.0 || avail.y < TOP_MARGIN + BOTTOM_MARGIN + 20.0 {
        return action;
    }

    let (full_rect, response) = ui.allocate_exact_size(avail, Sense::click());
    let plot_rect = Rect::from_min_max(
        pos2(full_rect.min.x + LEFT_MARGIN, full_rect.min.y + TOP_MARGIN),
        pos2(
            full_rect.max.x - RIGHT_MARGIN,
            full_rect.max.y - BOTTOM_MARGIN,
        ),
    );

    if ui.is_rect_visible(full_rect) {
        let painter = ui.painter();
        if let Some(tex) = texture {
            painter.image(
                tex.id(),
                plot_rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        painter.rect_stroke(plot_rect, 2.0, Stroke::new(1.0_f32, palette.card_border));

        draw_axes(painter, plot_rect, spectral, palette);
        draw_colorbar(painter, plot_rect, spectral.log_scale, palette);

        if let Some(sel_gap) = state.selected_gap {
            if let Some(row) = spectral.gap_values.iter().position(|&g| g == sel_gap) {
                let y = row_center_y(plot_rect, row, spectral.rows);
                painter.line_segment(
                    [pos2(plot_rect.min.x, y), pos2(plot_rect.max.x, y)],
                    Stroke::new(1.5_f32, palette.highlight),
                );
            }
        }

        if let Some(pos) = response.hover_pos().filter(|p| plot_rect.contains(*p)) {
            let (col, row) = cell_at_pos(pos, plot_rect, spectral.cols, spectral.rows);
            let gap = spectral.gap_values[row];
            let (count, _) = spectral.cell(row, col).unwrap_or((0, 0.0));
            let col_total = spectral.col_totals[col].max(1);
            let pct = (count as f64 / col_total as f64) * 100.0;
            let (lo, hi) = spectral.col_ranges[col];

            let cy = row_center_y(plot_rect, row, spectral.rows);
            let cx = col_center_x(plot_rect, col, spectral.cols);
            let cross = Stroke::new(1.0_f32, palette.accent.linear_multiply(0.7));
            painter.line_segment(
                [pos2(cx, plot_rect.min.y), pos2(cx, plot_rect.max.y)],
                cross,
            );
            painter.line_segment(
                [pos2(plot_rect.min.x, cy), pos2(plot_rect.max.x, cy)],
                cross,
            );

            egui::show_tooltip_at_pointer(
                ui.ctx(),
                ui.layer_id(),
                egui::Id::new("spectral_heatmap_tooltip"),
                |ui| {
                    ui.label(
                        egui::RichText::new(format!("🌈 Gap Δ_{} = {gap}", spectral.k))
                            .strong()
                            .color(palette.accent),
                    );
                    ui.label(format!("Prime Index: {} ~ {}", compact(lo), compact(hi)));
                    ui.label(format!("Count: {} ({pct:.2}%)", thousands(count)));
                },
            );

            if response.clicked() {
                state.selected_gap = if state.selected_gap == Some(gap) {
                    None
                } else {
                    Some(gap)
                };
            }
        }
    }

    action
}

fn controls_bar(ui: &mut Ui, state: &mut AppState, palette: &Palette) -> Option<Action> {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🌈 Spectral Density")
                .strong()
                .color(palette.accent),
        );
        ui.separator();
        changed |= ui
            .checkbox(&mut state.prefs.heatmap_log_scale, "Log Scale")
            .changed();
        changed |= ui
            .checkbox(&mut state.prefs.heatmap_even_only, "Even Gaps Only")
            .changed();
        ui.separator();
        ui.label("Max Gap:");
        for preset in MAX_GAP_PRESETS {
            if ui
                .selectable_label(state.prefs.heatmap_max_gap == preset, format!("{preset}"))
                .clicked()
            {
                state.prefs.heatmap_max_gap = preset;
                changed = true;
            }
        }
        ui.separator();
        if ui.button("↻ Refresh").clicked() {
            state.spectral_key = None;
            changed = true;
        }
    });
    changed.then_some(Action::RefreshSpectral)
}

fn empty_prompt(ui: &mut Ui, state: &AppState, palette: &Palette) -> Option<Action> {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.25).max(16.0));
        ui.label(
            egui::RichText::new("🌈 2D Spectral Distribution Heatmap")
                .size(18.0)
                .strong()
                .color(palette.accent),
        );
        ui.add_space(6.0);
        let msg = if state.dataset.is_some() {
            "Rendering 2D spectral heatmap across prime index blocks..."
        } else {
            "Click below to build a 2D spectral gap heatmap for the selected range."
        };
        ui.label(egui::RichText::new(msg).color(palette.text_dim));
        ui.add_space(8.0);
        if ui.button("Compute Spectral Heatmap").clicked() {
            clicked = true;
        }
    });
    clicked.then_some(Action::RefreshSpectral)
}

fn draw_axes(
    painter: &egui::Painter,
    plot_rect: Rect,
    spectral: &SpectralHeatmap,
    palette: &Palette,
) {
    let y_ticks = 5usize.min(spectral.rows);
    for t in 0..y_ticks {
        let row = if y_ticks <= 1 {
            0
        } else {
            t * (spectral.rows - 1) / (y_ticks - 1)
        };
        let gap = spectral.gap_values[row];
        let y = row_center_y(plot_rect, row, spectral.rows);
        painter.line_segment(
            [pos2(plot_rect.min.x - 4.0, y), pos2(plot_rect.min.x, y)],
            Stroke::new(1.0_f32, palette.card_border),
        );
        painter.text(
            pos2(plot_rect.min.x - 6.0, y),
            Align2::RIGHT_CENTER,
            format!("{gap}"),
            FontId::proportional(11.0),
            palette.text_dim,
        );
    }

    let x_ticks = 5usize.min(spectral.cols);
    for t in 0..x_ticks {
        let col = if x_ticks <= 1 {
            0
        } else {
            t * (spectral.cols - 1) / (x_ticks - 1)
        };
        let x = col_center_x(plot_rect, col, spectral.cols);
        let (lo, hi) = spectral.col_ranges[col];
        let label_val = if t + 1 == x_ticks { hi } else { lo };
        painter.line_segment(
            [pos2(x, plot_rect.max.y), pos2(x, plot_rect.max.y + 4.0)],
            Stroke::new(1.0_f32, palette.card_border),
        );
        let align = if t == 0 {
            Align2::LEFT_TOP
        } else if t + 1 == x_ticks {
            Align2::RIGHT_TOP
        } else {
            Align2::CENTER_TOP
        };
        painter.text(
            pos2(x, plot_rect.max.y + 6.0),
            align,
            compact(label_val),
            FontId::proportional(11.0),
            palette.text_dim,
        );
    }
}

fn draw_colorbar(painter: &egui::Painter, plot_rect: Rect, log_scale: bool, palette: &Palette) {
    let bar_rect = Rect::from_min_max(
        pos2(plot_rect.max.x + 12.0, plot_rect.min.y + 8.0),
        pos2(plot_rect.max.x + 22.0, plot_rect.max.y - 8.0),
    );
    if bar_rect.height() <= 10.0 {
        return;
    }

    let steps = 40;
    let step_h = bar_rect.height() / steps as f32;
    for i in 0..steps {
        let t = 1.0 - (i as f64 / (steps - 1) as f64);
        let y0 = bar_rect.min.y + i as f32 * step_h;
        let y1 = (y0 + step_h + 0.5).min(bar_rect.max.y);
        painter.rect_filled(
            Rect::from_min_max(pos2(bar_rect.min.x, y0), pos2(bar_rect.max.x, y1)),
            0.0,
            theme::viridis(t),
        );
    }
    painter.rect_stroke(bar_rect, 1.0, Stroke::new(1.0_f32, palette.card_border));

    let top_label = if log_scale { "Log" } else { "Max" };
    painter.text(
        pos2(bar_rect.max.x + 5.0, bar_rect.min.y),
        Align2::LEFT_TOP,
        top_label,
        FontId::proportional(10.5),
        palette.text_dim,
    );
    painter.text(
        pos2(bar_rect.max.x + 5.0, bar_rect.max.y),
        Align2::LEFT_BOTTOM,
        "0",
        FontId::proportional(10.5),
        palette.text_dim,
    );
}

fn row_center_y(plot_rect: Rect, row: usize, rows: usize) -> f32 {
    let from_top = (rows - 1 - row) as f32 + 0.5;
    plot_rect.min.y + (from_top / rows as f32) * plot_rect.height()
}

fn col_center_x(plot_rect: Rect, col: usize, cols: usize) -> f32 {
    plot_rect.min.x + ((col as f32 + 0.5) / cols as f32) * plot_rect.width()
}

fn cell_at_pos(pos: Pos2, plot_rect: Rect, cols: usize, rows: usize) -> (usize, usize) {
    let x_frac = ((pos.x - plot_rect.min.x) / plot_rect.width()).clamp(0.0, 0.999_999);
    let y_frac = ((pos.y - plot_rect.min.y) / plot_rect.height()).clamp(0.0, 0.999_999);
    let col = (x_frac * cols as f32) as usize;
    let y_px = (y_frac * rows as f32) as usize;
    let row = rows.saturating_sub(1).saturating_sub(y_px);
    (col.min(cols - 1), row.min(rows - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::ThemeMode;

    #[test]
    fn test_build_color_image_orientation_and_colors() {
        let palette = Palette::of(ThemeMode::Light);
        let spectral = SpectralHeatmap {
            cols: 2,
            rows: 2,
            start_idx: 1,
            end_idx: 20,
            k: 1,
            log_scale: false,
            even_only: true,
            gap_values: vec![2, 4],
            col_ranges: vec![(1, 10), (11, 20)],
            // row 0 (gap=2): [1.0, 0.0]; row 1 (gap=4): [0.0, 1.0]
            intensities: vec![1.0, 0.0, 0.0, 1.0],
            counts: vec![10, 0, 0, 10],
            col_totals: vec![10, 10],
        };

        let img = build_color_image(&spectral, &palette);
        assert_eq!(img.size, [2, 2]);
        // Top row (py=0 -> gap_row=1 -> gap=4): col 0 is zero_bg, col 1 is peak viridis
        assert_eq!(img.pixels[1], theme::viridis(1.0));
        // Bottom row (py=1 -> gap_row=0 -> gap=2): col 0 is peak viridis, col 1 is zero_bg
        assert_eq!(img.pixels[2], theme::viridis(1.0));
        assert_eq!(img.pixels[0], img.pixels[3]);
    }
}
