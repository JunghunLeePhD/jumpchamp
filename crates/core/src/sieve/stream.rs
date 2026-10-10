//! Lazy, block-wise prime streams.

use super::parallel::sieve_range_parallel;
use super::wheel::{isqrt, small_primes};

/// Lazily yields sieved prime blocks covering `[start, limit]`.
///
/// Each block spans `block_size` integers and is computed on demand, so memory stays
/// bounded to one block of primes at a time.
pub fn stream_prime_blocks_range<'a>(
    start: usize,
    limit: usize,
    block_size: usize,
    base_primes: &'a [usize],
) -> impl Iterator<Item = Vec<u64>> + 'a {
    (start..=limit).step_by(block_size).map(move |low| {
        let high = (low + block_size - 1).min(limit);
        sieve_range_parallel(low, high, base_primes)
    })
}

/// Like [`stream_prime_blocks_range`] starting at 2, but owns its base primes.
///
/// Returns the number of blocks alongside the stream (useful for progress reporting).
pub fn prime_blocks_up_to(
    limit: usize,
    block_size: usize,
) -> (usize, impl Iterator<Item = Vec<u64>>) {
    let base_primes = small_primes(isqrt(limit).max(2));
    let blocks = limit.saturating_sub(2) / block_size + 1;
    let stream = (2..=limit).step_by(block_size).map(move |low| {
        let high = (low + block_size - 1).min(limit);
        sieve_range_parallel(low, high, &base_primes)
    });
    (blocks, stream)
}

/// An upper bound for the `n`-th prime: `n (ln n + ln ln n)`, padded by 1000.
pub fn nth_prime_upper_bound(n: u64) -> u64 {
    match n {
        0 | 1 => 3,
        2 => 5,
        3 => 7,
        4 => 11,
        5 => 13,
        _ => {
            let nf = n as f64;
            let ln_n = nf.ln();
            (nf * (ln_n + ln_n.ln().max(0.1))).ceil() as u64 + 1000
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_prime_blocks_range() {
        let limit = 100_000;
        let block_size = 25_000;
        let base_primes = small_primes((limit as f64).sqrt() as usize);

        let blocks: Vec<Vec<u64>> =
            stream_prime_blocks_range(1, limit, block_size, &base_primes).collect();

        // Should yield 4 blocks of size 25,000 span each
        assert_eq!(blocks.len(), 4);
        let total_primes: usize = blocks.iter().map(|b| b.len()).sum();
        assert_eq!(total_primes, 9592); // pi(100,000) = 9592
    }

    #[test]
    fn test_prime_blocks_up_to_reports_exact_block_count() {
        let (blocks, stream) = prime_blocks_up_to(100_000, 30_000);
        let all: Vec<Vec<u64>> = stream.collect();
        assert_eq!(blocks, all.len());
        assert_eq!(all.iter().map(Vec::len).sum::<usize>(), 9592);
    }

    #[test]
    fn test_nth_prime_upper_bound_holds() {
        let primes = small_primes(2_000_000);
        for n in [1u64, 2, 5, 6, 10, 100, 1_000, 10_000, 100_000] {
            assert!(
                nth_prime_upper_bound(n) >= primes[n as usize - 1] as u64,
                "n = {n}"
            );
        }
    }
}
