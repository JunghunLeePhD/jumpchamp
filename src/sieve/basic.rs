// ============================================================================
// Sequential Sieve of Eratosthenes — pure, no external dependencies
// ============================================================================

/// Finds base prime numbers up to `limit` using a standard Sieve of Eratosthenes.
///
/// Returns a sorted `Vec<usize>` of all primes in `[2, limit]`.
pub fn small_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }
    let mut is_prime = vec![true; limit + 1];
    is_prime[0] = false;
    is_prime[1] = false;

    let mut p = 2;
    while p * p <= limit {
        if is_prime[p] {
            let mut i = p * p;
            while i <= limit {
                is_prime[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    (2..=limit).filter(|&x| is_prime[x]).collect()
}

/// Sieves a single segment `[seg_low, seg_high]` using precomputed base primes.
///
/// All numbers in the window that survive the sieve are returned as `u64`.
pub fn sieve_segment(seg_low: usize, seg_high: usize, base_primes: &[usize]) -> Vec<u64> {
    if seg_low > seg_high {
        return vec![];
    }

    let range_len = seg_high - seg_low + 1;
    let mut is_prime = vec![true; range_len];

    for &p in base_primes {
        let mut start = (seg_low + p - 1) / p * p;
        if start == p {
            start += p;
        }
        let mut i = start;
        while i <= seg_high {
            is_prime[i - seg_low] = false;
            i += p;
        }
    }

    (0..range_len)
        .filter_map(|i| if is_prime[i] { Some((seg_low + i) as u64) } else { None })
        .collect()
}
