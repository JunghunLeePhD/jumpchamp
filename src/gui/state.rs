//! Application state: plain data, no channels or threads.

use crate::engine::{frame_step, top_gaps, FrameSet, Progress};

use super::playback::Playback;
use super::prefs::{Prefs, MAX_LIMIT, MIN_LIMIT};

/// Frames per full animation.
pub const FRAMES: usize = 300;
/// At most this many (most frequent) gaps are kept for display.
pub const MAX_BARS: usize = 1000;
pub const DEFAULT_K: usize = 2;
pub const MAX_K: usize = 1000;

/// Which k-step gaps, over which prime-index range `n = min..=max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query {
    pub k: usize,
    pub min: u64,
    pub max: u64,
}

impl Query {
    pub fn full(limit: u64) -> Self {
        Self { k: DEFAULT_K, min: 1, max: limit }
    }

    /// Prime-index distance between animation frames.
    pub fn step(&self) -> u64 {
        frame_step(self.min, self.max, FRAMES)
    }
}

/// Visible slice of bars by frequency rank, 1-based inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank {
    pub min: usize,
    pub max: usize,
}

pub struct AppState {
    pub query: Query,
    pub rank: Rank,
    pub prefs: Prefs,
    pub playback: Playback,
    /// Pinned bar (by gap size).
    pub selected_gap: Option<u64>,

    /// `(gap, count)` in ascending gap order.
    pub bars: Vec<(u64, u64)>,
    pub frames: Option<FrameSet>,
    pub latency_ms: Option<f64>,
    pub progress: Option<Progress>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::with_prefs(Prefs::default())
    }
}

impl AppState {
    fn with_prefs(prefs: Prefs) -> Self {
        let query = Query::full(prefs.max_prime_limit);
        Self {
            query,
            rank: Rank { min: 1, max: 20 },
            prefs,
            playback: Playback::new(query.min),
            selected_gap: None,
            bars: Vec::new(),
            frames: None,
            latency_ms: None,
            progress: None,
        }
    }

    /// Back to launch defaults, keeping preferences.
    pub fn reset(&mut self) {
        *self = Self::with_prefs(self.prefs.clone());
    }

    /// Sets the global index limit and widens the range to `[1, limit]`.
    pub fn set_max_prime_limit(&mut self, limit: u64) {
        let limit = limit.clamp(MIN_LIMIT, MAX_LIMIT);
        self.prefs.max_prime_limit = limit;
        self.query.max = limit;
        if self.query.min > limit {
            self.query.min = 1;
        }
        self.frames = None;
    }

    /// Replaces the bars with the most frequent gaps of `hist` and shows all ranks.
    pub fn show_histogram(&mut self, hist: &[u64]) {
        self.bars = top_gaps(hist, MAX_BARS);
        self.rank = Rank { min: 1, max: self.bars.len().max(1) };
    }

    /// Shows the precomputed frame for the current position. False if no frames match the query.
    pub fn show_cached_frame(&mut self) -> bool {
        let Some(frames) = self.frames.take() else { return false };
        let hit = frames.matches(self.query.min, self.query.max, self.query.k);
        if hit {
            self.show_histogram(frames.at(self.playback.position));
        }
        self.frames = Some(frames);
        hit
    }

    /// Bars within the selected rank range.
    pub fn visible_bars(&self) -> &[(u64, u64)] {
        let len = self.bars.len();
        let start = self.rank.min.saturating_sub(1).min(len);
        let end = self.rank.max.min(len).max(start);
        &self.bars[start..end]
    }

    pub fn animation_progress(&self) -> f32 {
        self.playback.progress(self.query.min, self.query.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::prefs::ThemeMode;

    #[test]
    fn test_app_state_initializes_in_light_mode() {
        assert_eq!(AppState::default().prefs.theme, ThemeMode::Light);
    }

    #[test]
    fn test_default_range_matches_max_prime_limit() {
        let state = AppState::default();
        assert_eq!(state.query.min, 1);
        assert_eq!(state.query.max, state.prefs.max_prime_limit);
        assert_eq!(state.query.max, 10_000_000);
    }

    #[test]
    fn test_reset_preserves_prefs_and_restores_range() {
        let mut state = AppState::default();
        state.prefs.theme = ThemeMode::Dark;
        state.prefs.show_bar_tooltip = true;
        state.query.max = 500_000;
        state.selected_gap = Some(6);
        state.reset();
        assert_eq!(state.prefs.theme, ThemeMode::Dark);
        assert!(state.prefs.show_bar_tooltip);
        assert_eq!(state.query.max, 10_000_000);
        assert_eq!(state.selected_gap, None);
    }

    #[test]
    fn test_set_max_prime_limit_updates_range_and_steps() {
        let mut state = AppState::default();
        state.set_max_prime_limit(100_000_000);
        assert_eq!(state.prefs.max_prime_limit, 100_000_000);
        assert_eq!(state.query.max, 100_000_000);
        assert_eq!(state.query.step(), (100_000_000 - 1) / 300);
        state.set_max_prime_limit(5);
        assert_eq!(state.query.max, MIN_LIMIT);
    }

    #[test]
    fn test_show_histogram_and_visible_bars() {
        let mut state = AppState::default();
        state.show_histogram(&[0, 1, 5, 0, 3]);
        assert_eq!(state.bars, vec![(1, 1), (2, 5), (4, 3)]);
        assert_eq!(state.rank, Rank { min: 1, max: 3 });
        state.rank = Rank { min: 2, max: 9 };
        assert_eq!(state.visible_bars(), &[(2, 5), (4, 3)]);
    }

    #[test]
    fn test_show_cached_frame_requires_matching_query() {
        let mut state = AppState::default();
        assert!(!state.show_cached_frame());
        state.query = Query { k: 1, min: 1, max: 1_000 };
        state.frames = FrameSet::build(1, 1_000, 1, 10, |_| true);
        state.playback.position = 1_000;
        assert!(state.show_cached_frame());
        assert_eq!(state.bars.iter().map(|b| b.1).sum::<u64>(), 1_000);
        state.query.k = 2;
        assert!(!state.show_cached_frame());
    }
}
