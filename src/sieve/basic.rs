const WHEEL_OFFSETS: [usize; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
const WHEEL_INDEX: [u8; 30] = [
    255, 0, 255, 255, 255, 255, 255, 1, 255, 255,
    255, 2, 255, 3, 255, 255, 255, 4, 255, 5,
    255, 255, 255, 6, 255, 255, 255, 255, 255, 7,
];

/// Finds base prime numbers up to `limit` using a bitpacked Wheel-of-30 Sieve of Eratosthenes.
///
/// Returns a sorted `Vec<usize>` of all primes in `[2, limit]`.
pub fn small_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }
    let mut primes = Vec::new();
    if limit >= 2 { primes.push(2); }
    if limit >= 3 { primes.push(3); }
    if limit >= 5 { primes.push(5); }
    if limit < 7 {
        return primes;
    }

    let num_blocks = (limit / 30) + 1;
    let num_bits = num_blocks * 8;
    let num_words = (num_bits + 63) / 64;
    let mut bits = vec![!0u64; num_words];

    let sqrt_limit = (limit as f64).sqrt() as usize;

    for word_idx in 0..num_words {
        let mut word = bits[word_idx];
        while word != 0 {
            let bit = word.trailing_zeros() as usize;
            let bit_idx = word_idx * 64 + bit;
            let block_i = bit_idx / 8;
            let res_i = bit_idx % 8;
            let p = block_i * 30 + WHEEL_OFFSETS[res_i];

            if p > sqrt_limit {
                break;
            }

            if p > 1 {
                let start_val = p * p;
                if start_val <= limit {
                    let step = 2 * p; // Skip even multiples
                    let mut m = start_val;
                    while m <= limit {
                        let rem = m % 30;
                        let r_idx = WHEEL_INDEX[rem];
                        if r_idx != 255 {
                            let m_bit = (m / 30) * 8 + r_idx as usize;
                            let w_idx = m_bit / 64;
                            if w_idx < num_words {
                                bits[w_idx] &= !(1u64 << (m_bit % 64));
                            }
                        }
                        m += step;
                    }
                }
            }
            word &= word - 1;
        }
    }

    for word_idx in 0..num_words {
        let mut word = bits[word_idx];
        while word != 0 {
            let bit = word.trailing_zeros() as usize;
            let bit_idx = word_idx * 64 + bit;
            let block_i = bit_idx / 8;
            let res_i = bit_idx % 8;
            let p = block_i * 30 + WHEEL_OFFSETS[res_i];
            if p > 1 && p <= limit {
                primes.push(p);
            }
            word &= word - 1;
        }
    }

    primes
}

/// Sieves a single segment `[seg_low, seg_high]` using precomputed base primes
/// and Wheel-of-30 64-bit word bitmask alignment.
pub fn sieve_segment(seg_low: usize, seg_high: usize, base_primes: &[usize]) -> Vec<u64> {
    if seg_low > seg_high {
        return vec![];
    }

    let mut primes = Vec::new();

    // 1. Include base primes 2, 3, 5 if in segment
    for &p in &[2, 3, 5] {
        if seg_low <= p && p <= seg_high {
            primes.push(p as u64);
        }
    }

    let block_low = (seg_low / 30) * 30;
    let block_high = ((seg_high + 29) / 30) * 30;
    let num_blocks = (block_high - block_low) / 30 + 1;
    let num_bits = num_blocks * 8;
    let num_words = (num_bits + 63) / 64;

    let mut bits = vec![!0u64; num_words];

    // 2. Mark composites for base_primes > 5
    for &p in base_primes {
        if p <= 5 {
            continue;
        }

        let start_mult = p.saturating_mul(p).max(block_low);
        let mut start = if start_mult % p == 0 {
            start_mult
        } else {
            start_mult + (p - (start_mult % p))
        };

        if start % 2 == 0 {
            start += p;
        }

        let step = 2 * p; // Skip even multiples
        let mut m = start;
        while m <= seg_high + 30 {
            let rem = m % 30;
            let r_idx = WHEEL_INDEX[rem];
            if r_idx != 255 && m >= block_low {
                let m_offset = m - block_low;
                let m_bit = (m_offset / 30) * 8 + r_idx as usize;
                let word_i = m_bit / 64;
                if word_i < num_words {
                    bits[word_i] &= !(1u64 << (m_bit % 64));
                }
            }
            m += step;
        }
    }

    // 3. Extract surviving primes
    for word_idx in 0..num_words {
        let mut word = bits[word_idx];
        while word != 0 {
            let bit = word.trailing_zeros() as usize;
            let bit_idx = word_idx * 64 + bit;
            let block_i = bit_idx / 8;
            let res_i = bit_idx % 8;
            let p_val = block_low + block_i * 30 + WHEEL_OFFSETS[res_i];

            if p_val >= seg_low && p_val <= seg_high && p_val > 1 {
                primes.push(p_val as u64);
            }
            word &= word - 1;
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

