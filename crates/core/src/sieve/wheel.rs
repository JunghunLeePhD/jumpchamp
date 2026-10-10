//! Wheel-of-30 bit-packed Sieve of Eratosthenes.
//!
//! Every block of 30 integers is stored as 8 bits — one per residue coprime to 30
//! (1, 7, 11, 13, 17, 19, 23, 29). Multiples of 2, 3 and 5 are never stored.

const OFFSETS: [usize; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
const NO_BIT: u8 = 255;
const INDEX: [u8; 30] = [
    255, 0, 255, 255, 255, 255, 255, 1, 255, 255,
    255, 2, 255, 3, 255, 255, 255, 4, 255, 5,
    255, 255, 255, 6, 255, 255, 255, 255, 255, 7,
];

/// Floor of the square root (float-based; exact enough for sieve bounds).
pub fn isqrt(n: usize) -> usize {
    (n as f64).sqrt() as usize
}

/// Bit position of `n` relative to `base` (a multiple of 30), if `n` is coprime to 30.
fn bit_of(n: usize, base: usize) -> Option<usize> {
    let r = INDEX[n % 30];
    (r != NO_BIT).then(|| ((n - base) / 30) * 8 + r as usize)
}

/// Integer represented by bit position `bit` relative to `base`.
fn value_of(bit: usize, base: usize) -> usize {
    base + (bit / 8) * 30 + OFFSETS[bit % 8]
}

/// Iterates the positions of all set bits, in ascending order.
fn set_bits(words: &[u64]) -> impl Iterator<Item = usize> + '_ {
    words.iter().enumerate().flat_map(|(w, &word)| {
        std::iter::successors((word != 0).then_some(word), |&x| {
            let rest = x & (x - 1);
            (rest != 0).then_some(rest)
        })
        .map(move |x| w * 64 + x.trailing_zeros() as usize)
    })
}

/// First odd multiple of `p` that is `>= max(p², from)`.
fn first_odd_multiple(p: usize, from: usize) -> usize {
    let start = p.saturating_mul(p).max(from).div_ceil(p) * p;
    if start % 2 == 0 { start + p } else { start }
}

/// Sieves `[low, high]` using `base_primes` (all primes up to `√high`).
///
/// Returns the primes in the segment in ascending order.
pub fn sieve_segment(low: usize, high: usize, base_primes: &[usize]) -> Vec<u64> {
    if low > high {
        return vec![];
    }

    let base = (low / 30) * 30;
    let blocks = (high.div_ceil(30) * 30 - base) / 30 + 1;
    let mut bits = vec![!0u64; (blocks * 8).div_ceil(64)];

    for &p in base_primes.iter().filter(|&&p| p > 5) {
        for m in (first_odd_multiple(p, base)..=high).step_by(2 * p) {
            if let Some(bit) = bit_of(m, base) {
                bits[bit / 64] &= !(1u64 << (bit % 64));
            }
        }
    }

    let wheel_primes = [2, 3, 5].into_iter().filter(|p| (low..=high).contains(p));
    let sieved = set_bits(&bits)
        .map(|bit| value_of(bit, base))
        .filter(|&n| n > 1 && (low..=high).contains(&n));

    wheel_primes.chain(sieved).map(|p| p as u64).collect()
}

/// All primes in `[2, limit]`, ascending.
pub fn small_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }
    let base = small_primes(isqrt(limit));
    sieve_segment(1, limit, &base).into_iter().map(|p| p as usize).collect()
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
        assert_eq!(small_primes(49).last(), Some(&47));
    }

    #[test]
    fn test_small_primes_matches_trial_division() {
        let is_prime = |n: usize| n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0);
        let expected: Vec<usize> = (0..5000).filter(|&n| is_prime(n)).collect();
        assert_eq!(small_primes(4999), expected);
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
