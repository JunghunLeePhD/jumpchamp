//! Bottom status bar: engine, current range/mode, latency, progress.

use egui::{ProgressBar, Ui};

use crate::format::compact;
use crate::playback::{Direction, Mode};
use crate::state::AppState;

pub fn render(ui: &mut Ui, state: &AppState) {
    ui.horizontal(|ui| {
        ui.label(engine_label(state));
        ui.separator();
        ui.label(summary(state));
        ui.separator();

        ui.label(format!("⚡ Latency: {}", format_latency(state)));
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

pub fn engine_label(state: &AppState) -> String {
    match &state.dataset {
        Some(meta) => {
            let raw = if meta.has_raw_gaps { " + Raw Gaps" } else { "" };
            format!("⚡ Engine: Mmap Index ({} blocks{raw})", meta.block_count)
        }
        None => "⚙ Engine: In-Memory Parallel Segmented Sieve".to_string(),
    }
}

pub fn format_latency(state: &AppState) -> String {
    let base = state.latency_ms.map_or_else(
        || "-- ms".to_string(),
        |ms| {
            if ms < 1.0 {
                format!("{:.0} µs", ms * 1000.0)
            } else {
                format!("{ms:.1} ms")
            }
        },
    );
    if state.last_query_sampled {
        format!("{base} (sampled)")
    } else {
        base
    }
}

/// One-line description of what is being shown.
pub fn summary(state: &AppState) -> String {
    let (q, p) = (&state.query, &state.playback);
    let range = format!("n = {} ~ {}", compact(q.min), compact(q.max));
    match p.mode {
        Mode::Precaching => {
            let (pct, blocks) = match state.progress {
                Some(pr) => (
                    (pr.done * 100).checked_div(pr.total).unwrap_or(0),
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
    use crate::state::DatasetMeta;

    #[test]
    fn test_summary_per_mode() {
        let mut state = AppState::default();
        assert!(summary(&state).starts_with("📊 Prime Index Range: n = 1 ~ 10.00 M (k=2"));
        state.playback.mode = Mode::Playing;
        assert!(summary(&state).contains("▶ FORWARD"));
        state.playback.mode = Mode::Precaching;
        assert!(summary(&state).starts_with("⚡ PRE-CACHING (0%)"));
    }

    #[test]
    fn test_engine_label_and_microsecond_latency() {
        let mut state = AppState::default();
        assert_eq!(
            engine_label(&state),
            "⚙ Engine: In-Memory Parallel Segmented Sieve"
        );
        state.dataset = Some(DatasetMeta {
            index_path: "block_index.bin".to_string(),
            gaps_path: Some("gaps.bin".to_string()),
            block_count: 3761,
            block_size: 10_000_000,
            max_primes: 37_607_912_018,
            has_raw_gaps: true,
            raw_gap_count: 37_607_912_018,
        });
        state.latency_ms = Some(0.125);
        state.last_query_sampled = true;
        assert_eq!(
            engine_label(&state),
            "⚡ Engine: Mmap Index (3761 blocks + Raw Gaps)"
        );
        assert_eq!(format_latency(&state), "125 µs (sampled)");
    }
}
