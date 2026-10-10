//! Rayon-parallel sieve over L1-cache-sized segments.

use rayon::prelude::*;

use super::wheel::sieve_segment;

/// Numbers per segment: 30 × 8192 → an 8 KB wheel-30 bitmask (fits in L1 cache).
const SEGMENT_SPAN: usize = 245_760;

/// Sieves `[start, end]` in parallel; returns the primes in ascending order.
///
/// `base_primes` must contain every prime up to `√end`.
pub fn sieve_range_parallel(start: usize, end: usize, base_primes: &[usize]) -> Vec<u64> {
    if start > end {
        return vec![];
    }
    let segments = (end - start) / SEGMENT_SPAN + 1;

    (0..segments)
        .into_par_iter()
        .flat_map(|i| {
            let low = start + i * SEGMENT_SPAN;
            let high = (low + SEGMENT_SPAN - 1).min(end);
            sieve_segment(low, high, base_primes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sieve::wheel::small_primes;

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
