//! Application state: plain data, no channels or threads.

use jumpchamp_core::engine::{frame_step, top_gaps, FrameSet, Progress};
use jumpchamp_core::storage::SpectralHeatmap;

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
        Self {
            k: DEFAULT_K,
            min: 1,
            max: limit,
        }
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

/// Metadata describing a mounted zero-copy `block_index.bin` (and optional `gaps.bin`) dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetMeta {
    pub index_path: String,
    pub gaps_path: Option<String>,
    pub block_count: usize,
    pub block_size: u64,
    pub max_primes: u64,
    pub has_raw_gaps: bool,
    pub raw_gap_count: u64,
}

/// Cache key identifying the parameters used to compute [`AppState::spectral`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpectralKey {
    pub min: u64,
    pub max: u64,
    pub k: usize,
    pub max_gap: u64,
    pub even_only: bool,
    pub log_scale: bool,
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

    pub dataset: Option<DatasetMeta>,
    pub dataset_error: Option<String>,
    pub last_query_sampled: bool,
    pub spectral: Option<SpectralHeatmap>,
    pub spectral_key: Option<SpectralKey>,
    pub spectral_generation: u64,
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
            dataset: None,
            dataset_error: None,
            last_query_sampled: false,
            spectral: None,
            spectral_key: None,
            spectral_generation: 0,
        }
    }

    /// Back to launch defaults, keeping preferences and any mounted mmap dataset.
    pub fn reset(&mut self) {
        let dataset = self.dataset.take();
        let gen = self.spectral_generation;
        *self = Self::with_prefs(self.prefs.clone());
        self.spectral_generation = gen;
        if let Some(meta) = dataset {
            self.apply_dataset(meta);
        }
    }

    /// Activates a mounted zero-copy dataset and aligns query bounds to its indexed range.
    pub fn apply_dataset(&mut self, meta: DatasetMeta) {
        let max_primes = meta.max_primes.clamp(1, MAX_LIMIT);
        if max_primes > self.prefs.max_prime_limit {
            self.prefs.max_prime_limit = max_primes;
        }
        self.query = Query {
            k: 1,
            min: 1,
            max: max_primes,
        };
        self.playback = Playback::new(self.query.max);
        self.selected_gap = None;
        self.frames = None;
        self.spectral = None;
        self.spectral_key = None;
        self.last_query_sampled = false;
        self.dataset_error = None;
        self.dataset = Some(meta);
    }

    /// Key for the currently configured spectral heatmap query.
    pub fn current_spectral_key(&self) -> SpectralKey {
        SpectralKey {
            min: self.query.min,
            max: self.query.max,
            k: self.query.k,
            max_gap: self.prefs.heatmap_max_gap,
            even_only: self.prefs.heatmap_even_only,
            log_scale: self.prefs.heatmap_log_scale,
        }
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
        self.spectral_key = None;
    }

    /// Replaces the bars with the most frequent gaps of `hist` and shows all ranks.
    pub fn show_histogram(&mut self, hist: &[u64]) {
        self.bars = top_gaps(hist, MAX_BARS);
        self.rank = Rank {
            min: 1,
            max: self.bars.len().max(1),
        };
    }

    /// Shows the precomputed frame for the current position. False if no frames match the query.
    pub fn show_cached_frame(&mut self) -> bool {
        let Some(frames) = self.frames.take() else {
            return false;
        };
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
    use crate::prefs::ThemeMode;

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
        state.query = Query {
            k: 1,
            min: 1,
            max: 1_000,
        };
        state.frames = FrameSet::build(1, 1_000, 1, 10, |_| true);
        state.playback.position = 1_000;
        assert!(state.show_cached_frame());
        assert_eq!(state.bars.iter().map(|b| b.1).sum::<u64>(), 1_000);
        state.query.k = 2;
        assert!(!state.show_cached_frame());
    }

    #[test]
    fn test_apply_dataset_expands_limit_and_sets_k1() {
        let mut state = AppState::default();
        state.apply_dataset(DatasetMeta {
            index_path: "block_index.bin".to_string(),
            gaps_path: Some("gaps.bin".to_string()),
            block_count: 3761,
            block_size: 10_000_000,
            max_primes: 37_607_912_018,
            has_raw_gaps: true,
            raw_gap_count: 37_607_912_018,
        });
        assert_eq!(state.query.k, 1);
        assert_eq!(state.query.min, 1);
        assert_eq!(state.query.max, 37_607_912_018);
        assert_eq!(state.prefs.max_prime_limit, 37_607_912_018);
        assert_eq!(state.playback.position, 37_607_912_018);
    }
}
