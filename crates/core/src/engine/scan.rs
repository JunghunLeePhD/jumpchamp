//! The one prime walk every computation is built on.

use crate::analysis::k_step_gaps;
use crate::sieve::{nth_prime_upper_bound, prime_blocks_up_to};

const LARGE_COUNT: u64 = 100_000_000;
const LARGE_BLOCK: usize = 5_000_000;
const SMALL_BLOCK: usize = 1_000_000;

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
    mut on_block: impl FnMut(Progress) -> bool,
    mut visit: impl FnMut(u64, u64),
) -> bool {
    let prime_count = last_index.saturating_add(k as u64);
    let block = if prime_count >= LARGE_COUNT {
        LARGE_BLOCK
    } else {
        SMALL_BLOCK
    };
    let (total, blocks) = prime_blocks_up_to(nth_prime_upper_bound(prime_count) as usize, block);

    let mut cancelled = false;
    let primes = blocks
        .enumerate()
        .take_while(|&(i, _)| {
            cancelled = !on_block(Progress { done: i + 1, total });
            !cancelled
        })
        .flat_map(|(_, primes)| primes)
        .take(prime_count as usize);

    for (i, gap) in k_step_gaps(primes, k).enumerate() {
        visit(i as u64 + 1, gap);
    }
    !cancelled
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
}
