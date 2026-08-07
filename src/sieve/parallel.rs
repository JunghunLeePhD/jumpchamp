// ============================================================================
// Rayon-Parallel Sieve Dispatcher — L1-cache-aligned 32KB segments
// ============================================================================

use rayon::prelude::*;
use super::basic::sieve_segment;

/// Sieves the range `[start, end]` in parallel using Rayon.
///
/// The range is split into 32 KB windows that align with typical CPU L1 cache size,
/// each window sieved independently by `sieve_segment` across all available threads.
pub fn sieve_range_parallel(start: usize, end: usize, base_primes: &[usize]) -> Vec<u64> {
    let segment_size = 32_768; // 32 KB — aligned with CPU L1 cache
    let num_segments = (end - start) / segment_size + 1;

    (0..num_segments)
        .into_par_iter()
        .flat_map(|idx| {
            let seg_low = start + idx * segment_size;
            let seg_high = (seg_low + segment_size - 1).min(end);
            sieve_segment(seg_low, seg_high, base_primes)
        })
        .collect()
}
