// ============================================================================
// Rayon-Parallel Sieve Dispatcher — L1-cache-aligned 32KB bitmask segments
// ============================================================================

use rayon::prelude::*;
use super::basic::sieve_segment;

/// Sieves the range `[start, end]` in parallel using Rayon.
///
/// The range is split into cache-aligned windows, each sieved independently
/// by bitpacked `sieve_segment` across all available threads.
pub fn sieve_range_parallel(start: usize, end: usize, base_primes: &[usize]) -> Vec<u64> {
    if start > end {
        return vec![];
    }
    let segment_span = 245_760; // 30 * 8192 numbers = 8 KB Wheel-30 bitmask, L1 cache fit
    let num_segments = (end - start) / segment_span + 1;

    (0..num_segments)
        .into_par_iter()
        .flat_map(|idx| {
            let seg_low = start + idx * segment_span;
            let seg_high = (seg_low + segment_span - 1).min(end);
            sieve_segment(seg_low, seg_high, base_primes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sieve::basic::small_primes;

    #[test]
    fn test_parallel_sieve_matches_sequential() {
        let limit = 500_000;
        let sqrt_limit = (limit as f64).sqrt() as usize;
        let base_primes = small_primes(sqrt_limit);

        let parallel_primes = sieve_range_parallel(1, limit, &base_primes);
        let expected = small_primes(limit).into_iter().map(|p| p as u64).collect::<Vec<_>>();

        assert_eq!(parallel_primes, expected);
        assert_eq!(parallel_primes.len(), 41538); // pi(500,000) = 41538
    }

    #[test]
    fn test_parallel_sieve_large_interval() {
        let start = 1_000_000;
        let end = 2_000_000;
        let base_primes = small_primes((end as f64).sqrt() as usize);

        let window_primes = sieve_range_parallel(start, end, &base_primes);
        // pi(2,000,000) - pi(1,000,000) = 148,933 - 78,498 = 70,435
        assert_eq!(window_primes.len(), 70435);
        assert_eq!(window_primes.first(), Some(&1000003));
        assert_eq!(window_primes.last(), Some(&1999993));
    }
}

