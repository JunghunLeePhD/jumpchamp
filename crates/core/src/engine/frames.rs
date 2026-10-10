//! Precomputed animation frames: cumulative gap histograms over a growing index range.

use super::histogram::{self, Histogram};
use super::scan::{scan_k_gaps, Progress};

/// Frame `i` holds the histogram of gaps starting at indices `min .. min + (i+1)·step`
/// (the last frame also absorbs the remainder up to `max`).
#[derive(Debug, Clone)]
pub struct FrameSet {
    pub min: u64,
    pub max: u64,
    pub k: usize,
    pub step: u64,
    frames: Vec<Histogram>,
}

impl FrameSet {
    /// Builds `count` cumulative frames. Returns `None` if cancelled via `on_block`.
    pub fn build(
        min: u64,
        max: u64,
        k: usize,
        count: usize,
        on_block: impl FnMut(Progress) -> bool,
    ) -> Option<Self> {
        let count = count.max(1);
        let step = frame_step(min, max, count);
        let mut deltas = vec![histogram::empty(); count];

        let completed = scan_k_gaps(max, k, on_block, |n, gap| {
            if n >= min {
                let frame = (((n - min) / step) as usize).min(count - 1);
                histogram::record(&mut deltas[frame], gap);
            }
        });
        if !completed {
            return None;
        }

        let width = deltas
            .iter()
            .map(|h| histogram::trimmed_len(h))
            .max()
            .unwrap_or(0)
            .max(1);
        let frames = histogram::running_totals(&deltas, width);
        Some(Self {
            min,
            max,
            k,
            step,
            frames,
        })
    }

    /// Whether these frames were built for this exact query.
    pub fn matches(&self, min: u64, max: u64, k: usize) -> bool {
        (self.min, self.max, self.k) == (min, max, k)
    }

    /// The cumulative histogram shown when the animation is at prime index `position`.
    pub fn at(&self, position: u64) -> &[u64] {
        let i = (position.saturating_sub(self.min) / self.step) as usize;
        &self.frames[i.min(self.frames.len() - 1)]
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Index distance between consecutive frames (at least 1).
pub fn frame_step(min: u64, max: u64, count: usize) -> u64 {
    (max.saturating_sub(min) / count.max(1) as u64).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::range::{range_histogram, ChunkCache};

    #[test]
    fn test_last_frame_equals_full_range_histogram() {
        let frames = FrameSet::build(100, 20_000, 2, 30, |_| true).unwrap();
        let full = range_histogram(100, 20_000, 2, &mut ChunkCache::default(), |_| true).unwrap();
        assert_eq!(frames.len(), 30);
        assert_eq!(frames.at(20_000), &full[..frames.at(20_000).len()]);
        assert_eq!(histogram::trimmed_len(&full), frames.at(20_000).len());
    }

    #[test]
    fn test_frames_are_cumulative() {
        let frames = FrameSet::build(1, 10_000, 1, 10, |_| true).unwrap();
        let totals: Vec<u64> = (0..10)
            .map(|i| frames.at(1 + i * frames.step).iter().sum())
            .collect();
        assert!(totals.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(*totals.last().unwrap(), 10_000);
    }

    #[test]
    fn test_matches_and_cancel() {
        let frames = FrameSet::build(1, 1_000, 1, 5, |_| true).unwrap();
        assert!(frames.matches(1, 1_000, 1));
        assert!(!frames.matches(1, 1_000, 2));
        assert!(FrameSet::build(1, 1_000, 1, 5, |_| false).is_none());
    }
}
