//! The one prime walk every computation is built on.

use std::collections::VecDeque;

use crate::cancel::CancelToken;
use crate::sieve::wheel::isqrt;
use crate::sieve::{
    nth_prime_upper_bound, sieve_range_parallel_cancellable, small_primes_cancellable,
};

const LARGE_COUNT: u64 = 100_000_000;
const LARGE_BLOCK: usize = 5_000_000;
const SMALL_BLOCK: usize = 1_000_000;
const VISIT_POLL_STRIDE: usize = 4_096;

/// How many sieve blocks have been started out of the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

/// Visits every k-step gap `p_{n+k} − p_n` for `n = 1..=last_index`, in order.
///
/// * `on_block` runs before each sieve block; return `false` to cancel.
/// * `visit(n, gap)` receives the 1-based starting prime index and its gap.
///
/// Returns `true` if the walk completed, `false` if it was cancelled.
pub fn scan_k_gaps(
    last_index: u64,
    k: usize,
    on_block: impl FnMut(Progress) -> bool,
    visit: impl FnMut(u64, u64),
) -> bool {
    scan_k_gaps_cancellable(last_index, k, &CancelToken::new(), on_block, visit)
}

/// Cancellable variant of [`scan_k_gaps`].
///
/// Checks `cancel` and `on_block` before sieving each block, inside segment sieving, and
/// periodically during gap emission.
pub fn scan_k_gaps_cancellable(
    last_index: u64,
    k: usize,
    cancel: &CancelToken,
    mut on_block: impl FnMut(Progress) -> bool,
    mut visit: impl FnMut(u64, u64),
) -> bool {
    if cancel.is_cancelled() {
        return false;
    }
    if last_index == 0 || k == 0 {
        return true;
    }

    let prime_count = last_index.saturating_add(k as u64);
    let block = if prime_count >= LARGE_COUNT {
        LARGE_BLOCK
    } else {
        SMALL_BLOCK
    };
    let limit = nth_prime_upper_bound(prime_count) as usize;
    let Some(base_primes) = small_primes_cancellable(isqrt(limit).max(2), cancel) else {
        return false;
    };
    let total = limit.saturating_sub(2) / block + 1;

    let mut window = VecDeque::with_capacity(k + 1);
    let mut primes_seen = 0u64;
    let mut gap_index = 0u64;

    for (i, low) in (2..=limit).step_by(block).enumerate() {
        if cancel.is_cancelled() || !on_block(Progress { done: i + 1, total }) {
            return false;
        }
        let high = (low + block - 1).min(limit);
        let Some(block_primes) = sieve_range_parallel_cancellable(low, high, &base_primes, cancel)
        else {
            return false;
        };

        for (p_idx, p) in block_primes.into_iter().enumerate() {
            if p_idx.is_multiple_of(VISIT_POLL_STRIDE) && cancel.is_cancelled() {
                return false;
            }
            window.push_back(p);
            if window.len() == k + 1 {
                let p_n = window.pop_front().unwrap();
                gap_index += 1;
                visit(gap_index, p - p_n);
            }
            primes_seen += 1;
            if primes_seen == prime_count {
                return !cancel.is_cancelled();
            }
        }
    }

    !cancel.is_cancelled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visits_each_k_gap_with_its_index() {
        let mut seen = vec![];
        let done = scan_k_gaps(5, 2, |_| true, |n, g| seen.push((n, g)));
        // primes: 2 3 5 7 11 13 17 → 2-step gaps from p_1..p_5
        assert!(done);
        assert_eq!(seen, vec![(1, 3), (2, 4), (3, 6), (4, 6), (5, 6)]);
    }

    #[test]
    fn test_cancel_stops_before_any_visit() {
        let mut visits = 0;
        let done = scan_k_gaps(1_000, 1, |_| false, |_, _| visits += 1);
        assert!(!done);
        assert_eq!(visits, 0);
    }

    #[test]
    fn test_reports_progress_per_block() {
        let mut reports = vec![];
        scan_k_gaps(
            10,
            1,
            |p| {
                reports.push(p);
                true
            },
            |_, _| {},
        );
        assert_eq!(reports, vec![Progress { done: 1, total: 1 }]);
    }

    #[test]
    fn test_scan_k_gaps_cancellable_matches_and_aborts() {
        let mut expected = vec![];
        assert!(scan_k_gaps(200, 3, |_| true, |n, g| expected.push((n, g))));

        let cancel = CancelToken::new();
        let mut actual = vec![];
        assert!(scan_k_gaps_cancellable(
            200,
            3,
            &cancel,
            |_| true,
            |n, g| actual.push((n, g))
        ));
        assert_eq!(actual, expected);

        cancel.cancel();
        let mut visits = 0;
        assert!(!scan_k_gaps_cancellable(
            1_000,
            1,
            &cancel,
            |_| true,
            |_, _| visits += 1
        ));
        assert_eq!(visits, 0);

        let mid_cancel = CancelToken::new();
        let trigger = mid_cancel.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(2));
            trigger.cancel();
        });
        let completed = scan_k_gaps_cancellable(5_000_000, 2, &mid_cancel, |_| true, |_, _| {});
        handle.join().unwrap();
        assert!(!completed);
    }
}
