//! Gap histogram over a prime-index range, backed by a per-chunk cache.

use std::collections::HashMap;

use super::histogram::{self, Histogram};
use super::scan::{scan_k_gaps, Progress};

/// Prime indices per cached chunk.
pub const CHUNK: u64 = 10_000;

/// Ranges ending at or below this index are always computed exactly.
/// Above it, a fully cached range is answered from whole chunks (chunk-granular).
pub const EXACT_THRESHOLD: u64 = 100_000;

/// Histograms of complete index chunks, keyed by `(chunk, k)`.
#[derive(Default)]
pub struct ChunkCache {
    chunks: HashMap<(u64, usize), Histogram>,
}

impl ChunkCache {
    pub fn clear(&mut self) {
        self.chunks = HashMap::new();
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    fn insert(&mut self, chunk: u64, k: usize, hist: &[u64]) {
        self.chunks.insert((chunk, k), histogram::trimmed(hist));
    }

    /// Sum of chunks `first..=last`, or `None` if any is missing.
    fn sum(&self, first: u64, last: u64, k: usize) -> Option<Histogram> {
        let mut total = histogram::empty();
        for chunk in first..=last {
            histogram::add(&mut total, self.chunks.get(&(chunk, k))?);
        }
        Some(total)
    }
}

/// Histogram of k-step gaps starting at prime indices `min..=max`.
///
/// Returns `None` if cancelled via `on_block`.
pub fn range_histogram(
    min: u64,
    max: u64,
    k: usize,
    cache: &mut ChunkCache,
    on_block: impl FnMut(Progress) -> bool,
) -> Option<Histogram> {
    if max > EXACT_THRESHOLD {
        if let Some(hist) = cache.sum(min / CHUNK, max / CHUNK, k) {
            return Some(hist);
        }
    }
    scan_and_cache(min, max, k, cache, on_block)
}

/// Walks primes up to `max`, filling the cache with every complete chunk along the way.
fn scan_and_cache(
    min: u64,
    max: u64,
    k: usize,
    cache: &mut ChunkCache,
    on_block: impl FnMut(Progress) -> bool,
) -> Option<Histogram> {
    let mut hist = histogram::empty();
    let mut chunk_hist = histogram::empty();
    let mut chunk = 0;

    let completed = scan_k_gaps(max, k, on_block, |n, gap| {
        if n / CHUNK != chunk {
            cache.insert(chunk, k, &chunk_hist);
            chunk_hist.fill(0);
            chunk = n / CHUNK;
        }
        histogram::record(&mut chunk_hist, gap);
        if n >= min {
            histogram::record(&mut hist, gap);
        }
    });

    // The last chunk is only cached if the walk reached its end.
    if completed && (max + 1) % CHUNK == 0 {
        cache.insert(chunk, k, &chunk_hist);
    }
    completed.then_some(hist)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference: direct k-gap histogram from a plain prime list.
    fn naive(min: u64, max: u64, k: usize) -> Histogram {
        let primes = crate::sieve::small_primes(5_000_000);
        let mut h = histogram::empty();
        for n in min..=max {
            let i = n as usize - 1;
            histogram::record(&mut h, (primes[i + k] - primes[i]) as u64);
        }
        h
    }

    #[test]
    fn test_exact_range_matches_naive() {
        let mut cache = ChunkCache::default();
        let h = range_histogram(1_234, 56_789, 3, &mut cache, |_| true).unwrap();
        assert_eq!(h, naive(1_234, 56_789, 3));
    }

    #[test]
    fn test_scan_caches_only_complete_chunks() {
        let mut cache = ChunkCache::default();
        range_histogram(1, 25_000, 1, &mut cache, |_| true).unwrap();
        // chunks 0 (1..9_999) and 1 (10_000..19_999) are complete; chunk 2 is partial.
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_hit_sums_whole_chunks() {
        let mut cache = ChunkCache::default();
        range_histogram(1, 299_999, 2, &mut cache, |_| true).unwrap();
        let cached = range_histogram(10_000, 199_999, 2, &mut cache, |_| false).unwrap();
        assert_eq!(cached, naive(10_000, 199_999, 2));
    }

    #[test]
    fn test_cancel_returns_none() {
        let mut cache = ChunkCache::default();
        assert!(range_histogram(1, 50_000, 1, &mut cache, |_| false).is_none());
    }
}
