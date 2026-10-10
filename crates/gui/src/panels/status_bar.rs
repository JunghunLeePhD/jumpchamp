//! Bottom status bar: engine, current range/mode, latency, progress.

use egui::{ProgressBar, Ui};

use crate::gui::format::compact;
use crate::gui::playback::{Direction, Mode};
use crate::gui::state::AppState;

pub fn render(ui: &mut Ui, state: &AppState) {
    ui.horizontal(|ui| {
        ui.label("⚙ Engine: In-Memory Parallel Segmented Sieve");
        ui.separator();
        ui.label(summary(state));
        ui.separator();

        let latency = state
            .latency_ms
            .map_or("-- ms".into(), |ms| format!("{ms:.1} ms"));
        ui.label(format!("⚡ Latency: {latency}"));
        ui.separator();

        let fraction = match (state.playback.mode, state.progress) {
            (Mode::Playing, _) => Some(state.animation_progress()),
            (Mode::Precaching, p) => Some(p.map_or(0.0, |p| p.done as f32 / p.total as f32)),
            (Mode::Stopped, _) => None,
        };
        if let Some(fraction) = fraction {
            ui.add_sized([90.0, 16.0], ProgressBar::new(fraction).show_percentage());
        }
    });
}

/// One-line description of what is being shown.
pub fn summary(state: &AppState) -> String {
    let (q, p) = (&state.query, &state.playback);
    let range = format!("n = {} ~ {}", compact(q.min), compact(q.max));
    match p.mode {
        Mode::Precaching => {
            let (pct, blocks) = match state.progress {
                Some(pr) => (
                    pr.done * 100 / pr.total,
                    format!(" [Block {}/{}]", pr.done, pr.total),
                ),
                None => (0, String::new()),
            };
            format!("⚡ PRE-CACHING ({pct}%{blocks}): {range} for 0-delay playback...")
        }
        Mode::Playing => {
            let dir = match p.direction {
                Direction::Forward => "▶ FORWARD",
                Direction::Reverse => "◀ REVERSE",
            };
            format!(
                "🎬 ANIMATING ({dir}): {range} (Bound: n = {})",
                compact(p.position)
            )
        }
        Mode::Stopped => {
            let r = &state.rank;
            format!(
                "📊 Prime Index Range: {range} (k={}, Rank={}~{})",
                q.k, r.min, r.max
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_per_mode() {
        let mut state = AppState::default();
        assert!(summary(&state).starts_with("📊 Prime Index Range: n = 1 ~ 10.00 M (k=2"));
        state.playback.mode = Mode::Playing;
        assert!(summary(&state).contains("▶ FORWARD"));
        state.playback.mode = Mode::Precaching;
        assert!(summary(&state).starts_with("⚡ PRE-CACHING (0%)"));
    }
}
