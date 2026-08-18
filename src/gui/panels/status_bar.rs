// ============================================================================
// Status Bar Panel Component
// ============================================================================

use crate::gui::state::{AppState, PlayDirection};
use crate::gui::utils::format_compact_num;

/// Renders the bottom telemetry status bar showing engine state, index range, latency, and progress.
pub fn render(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal(|ui| {
        ui.label("⚙ Engine: In-Memory Parallel Segmented Sieve");
        ui.separator();

        if state.is_precaching {
            let pct = (state.progress * 100.0) as u32;
            let block_info = if state.total_blocks > 0 {
                format!(" [Block {}/{}]", state.current_block, state.total_blocks)
            } else {
                String::new()
            };
            ui.label(format!(
                "⚡ PRE-CACHING ({pct}%{}): n = {} ~ {} for 0-delay playback...",
                block_info,
                format_compact_num(state.min_val),
                format_compact_num(state.max_val)
            ));
        } else if state.is_animating {
            let dir_str = match state.anim_direction {
                PlayDirection::Forward => "▶ FORWARD",
                PlayDirection::Reverse => "◀ REVERSE",
            };
            ui.label(format!(
                "🎬 ANIMATING ({}): n = {} ~ {} (Bound: n = {})",
                dir_str,
                format_compact_num(state.min_val),
                format_compact_num(state.max_val),
                format_compact_num(state.anim_current_val)
            ));
        } else {
            ui.label(format!(
                "📊 Prime Index Range: n = {} ~ {} (k={}, Rank={}~{})",
                format_compact_num(state.min_val),
                format_compact_num(state.max_val),
                state.k,
                state.top_min,
                state.top_max
            ));
        }

        ui.separator();
        let latency_str = state
            .query_latency_ms
            .map(|ms| format!("{:.1} ms", ms))
            .unwrap_or_else(|| "-- ms".to_string());
        ui.label(format!("⚡ Latency: {}", latency_str));

        ui.separator();
        if state.is_animating {
            let anim_prog = state.animation_progress();
            ui.add_sized(
                [90.0_f32, 16.0_f32],
                egui::ProgressBar::new(anim_prog).show_percentage(),
            );
        } else if state.is_loading || state.is_precaching {
            ui.add_sized(
                [90.0_f32, 16.0_f32],
                egui::ProgressBar::new(state.progress).show_percentage(),
            );
        }
    });
}
