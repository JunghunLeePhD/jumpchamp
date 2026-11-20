//! Rayon-parallel sieve over L1-cache-sized segments.

use rayon::prelude::*;

use crate::cancel::CancelToken;

use super::wheel::{sieve_segment, sieve_segment_cancellable};

/// Numbers per segment: 30 × 8192 → an 8 KB wheel-30 bitmask (fits in L1 cache).
pub(super) const SEGMENT_SPAN: usize = 245_760;

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

/// Cancellable variant of [`sieve_range_parallel`].
///
/// Checks `cancel` at segment boundaries and stops assembling results immediately on abort.
pub fn sieve_range_parallel_cancellable(
    start: usize,
    end: usize,
    base_primes: &[usize],
    cancel: &CancelToken,
) -> Option<Vec<u64>> {
    if cancel.is_cancelled() {
        return None;
    }
    if start > end {
        return Some(vec![]);
    }
    let segments = (end - start) / SEGMENT_SPAN + 1;

    let parts: Option<Vec<Vec<u64>>> = (0..segments)
        .into_par_iter()
        .map(|i| {
            if cancel.is_cancelled() {
                return None;
            }
            let low = start + i * SEGMENT_SPAN;
            let high = (low + SEGMENT_SPAN - 1).min(end);
            sieve_segment_cancellable(low, high, base_primes, cancel)
        })
        .collect();

    let parts = parts?;
    if cancel.is_cancelled() {
        return None;
    }
    let total_len: usize = parts.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(total_len);
    for mut part in parts {
        if cancel.is_cancelled() {
            return None;
        }
        out.append(&mut part);
    }
    Some(out)
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
        let expected = small_primes(limit)
            .into_iter()
            .map(|p| p as u64)
            .collect::<Vec<_>>();

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

    #[test]
    fn test_parallel_sieve_cancellable_matches_sequential() {
        let cancel = CancelToken::new();

        let limit = 500_000;
        let base_primes = small_primes((limit as f64).sqrt() as usize);
        let parallel_primes =
            sieve_range_parallel_cancellable(1, limit, &base_primes, &cancel).unwrap();
        let expected: Vec<u64> = small_primes(limit).into_iter().map(|p| p as u64).collect();
        assert_eq!(parallel_primes, expected);

        let base_window = small_primes((2_000_000_f64).sqrt() as usize);
        let window_primes =
            sieve_range_parallel_cancellable(1_000_000, 2_000_000, &base_window, &cancel).unwrap();
        assert_eq!(
            window_primes,
            sieve_range_parallel(1_000_000, 2_000_000, &base_window)
        );
    }

    #[test]
    fn test_parallel_sieve_cancellable_aborts_when_cancelled() {
        let pre_cancel = CancelToken::new();
        pre_cancel.cancel();
        let base_small = small_primes(1_000);
        assert_eq!(
            sieve_range_parallel_cancellable(1, 1_000_000, &base_small, &pre_cancel),
            None
        );

        let cancel = CancelToken::new();
        let trigger = cancel.clone();
        let base_large = small_primes(10_000);
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1));
            trigger.cancel();
        });
        let result = sieve_range_parallel_cancellable(1, 100_000_000, &base_large, &cancel);
        handle.join().unwrap();
        assert!(result.is_none());
    }
}
