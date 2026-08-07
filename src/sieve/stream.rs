// ============================================================================
// Lazy Block Streaming over Sieve Output
// ============================================================================

use super::parallel::sieve_range_parallel;

/// Returns a lazy iterator that yields sieved prime blocks for the range `[start_val, limit]`.
///
/// Each block is computed on demand by `sieve_range_parallel`, so memory usage stays
/// bounded to `block_size` primes at a time regardless of total output size.
pub fn stream_prime_blocks_range<'a>(
    start_val: usize,
    limit: usize,
    block_size: usize,
    base_primes: &'a [usize],
) -> impl Iterator<Item = Vec<u64>> + 'a {
    (start_val..=limit)
        .step_by(block_size)
        .map(move |block_start| {
            let block_end = (block_start + block_size - 1).min(limit);
            sieve_range_parallel(block_start, block_end, base_primes)
        })
}
