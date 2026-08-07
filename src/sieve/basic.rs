// ============================================================================
// Bitpacked Odd-Only Sieve of Eratosthenes — pure, high-performance
// ============================================================================

/// Finds base prime numbers up to `limit` using a bitpacked odd-only Sieve of Eratosthenes.
///
/// Returns a sorted `Vec<usize>` of all primes in `[2, limit]`.
pub fn small_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }
    if limit == 2 {
        return vec![2];
    }

    let num_odds = (limit - 3) / 2 + 1;
    let mut is_prime = vec![true; num_odds];

    let sqrt_limit = (limit as f64).sqrt() as usize;
    let max_i = if sqrt_limit >= 3 { (sqrt_limit - 3) / 2 } else { 0 };

    for i in 0..=max_i {
        if is_prime[i] {
            let p = 2 * i + 3;
            let start_val = p * p;
            if start_val <= limit {
                let start_idx = (start_val - 3) / 2;
                let step = p;
                let mut j = start_idx;
                while j < num_odds {
                    is_prime[j] = false;
                    j += step;
                }
            }
        }
    }

    let mut primes = Vec::with_capacity(num_odds + 1);
    primes.push(2);
    for i in 0..num_odds {
        if is_prime[i] {
            primes.push(2 * i + 3);
        }
    }
    primes
}

/// Sieves a single segment `[seg_low, seg_high]` using precomputed base primes
/// and 64-bit bitmask word alignment for 8x cache efficiency.
///
/// All prime numbers in the window that survive the sieve are returned as `u64`.
pub fn sieve_segment(seg_low: usize, seg_high: usize, base_primes: &[usize]) -> Vec<u64> {
    if seg_low > seg_high {
        return vec![];
    }

    let mut primes = Vec::new();

    // 1. Include 2 if in segment
    if seg_low <= 2 && 2 <= seg_high {
        primes.push(2);
    }

    // 2. Determine odd range bounds [seg_low_odd, seg_high_odd]
    let seg_low_odd = if seg_low % 2 == 0 { seg_low + 1 } else { seg_low }.max(3);
    let seg_high_odd = if seg_high % 2 == 0 { seg_high.saturating_sub(1) } else { seg_high };

    if seg_low_odd > seg_high_odd {
        return primes;
    }

    let num_odds = (seg_high_odd - seg_low_odd) / 2 + 1;
    let num_words = (num_odds + 63) / 64;
    let mut bits = vec![!0u64; num_words];

    // Clear out-of-bounds padding bits in the final 64-bit word
    let remainder = num_odds % 64;
    if remainder != 0 {
        bits[num_words - 1] = (1u64 << remainder) - 1;
    }

    // 3. Cross out composite odd numbers for each base prime
    for &p in base_primes {
        if p == 2 {
            continue;
        }

        let min_multiple = p.saturating_mul(p).max(seg_low_odd);
        let start = if min_multiple % p == 0 {
            min_multiple
        } else {
            min_multiple + (p - (min_multiple % p))
        };

        let start_odd = if start % 2 == 0 { start + p } else { start };

        if start_odd <= seg_high_odd {
            let start_bit = (start_odd - seg_low_odd) / 2;
            let step = p;
            let mut bit_idx = start_bit;
            while bit_idx < num_odds {
                bits[bit_idx / 64] &= !(1u64 << (bit_idx % 64));
                bit_idx += step;
            }
        }
    }

    // 4. Fast bit-scan extraction using trailing_zeros (tzcnt intrinsic)
    for (word_idx, &word) in bits.iter().enumerate() {
        let mut val = word;
        while val != 0 {
            let bit = val.trailing_zeros() as usize;
            let bit_idx = word_idx * 64 + bit;
            if bit_idx < num_odds {
                let p_val = seg_low_odd + 2 * bit_idx;
                primes.push(p_val as u64);
            }
            val &= val - 1;
        }
    }

    primes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_primes_known_counts() {
        // Known prime counting function pi(x) values:
        // pi(10) = 4 [2, 3, 5, 7]
        // pi(100) = 25
        // pi(1,000) = 168
        // pi(10,000) = 1229
        // pi(100,000) = 9592
        assert_eq!(small_primes(10), vec![2, 3, 5, 7]);
        assert_eq!(small_primes(100).len(), 25);
        assert_eq!(small_primes(1000).len(), 168);
        assert_eq!(small_primes(10000).len(), 1229);
        assert_eq!(small_primes(100000).len(), 9592);
    }

    #[test]
    fn test_small_primes_edge_cases() {
        assert_eq!(small_primes(0), Vec::<usize>::new());
        assert_eq!(small_primes(1), Vec::<usize>::new());
        assert_eq!(small_primes(2), vec![2]);
        assert_eq!(small_primes(3), vec![2, 3]);
    }

    #[test]
    fn test_sieve_segment_matches_small_primes() {
        let limit = 10_000;
        let base_primes = small_primes((limit as f64).sqrt() as usize);
        let segment_primes = sieve_segment(1, limit, &base_primes);
        let expected = small_primes(limit).into_iter().map(|p| p as u64).collect::<Vec<_>>();

        assert_eq!(segment_primes, expected);
    }

    #[test]
    fn test_sieve_segment_offset_window() {
        let base_primes = small_primes(100);
        let window_primes = sieve_segment(100, 200, &base_primes);

        // First prime > 100 is 101, last prime <= 200 is 199
        assert_eq!(window_primes.first(), Some(&101));
        assert_eq!(window_primes.last(), Some(&199));
        assert_eq!(window_primes.len(), 21); // 21 primes in [100, 200]
    }
}

