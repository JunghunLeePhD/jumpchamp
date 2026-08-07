use rayon::prelude::*;
use std::time::Instant;

/// Sequential Sieve to find small primes up to sqrt(N)
fn find_small_primes(limit: usize) -> Vec<usize> {
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

/// Parallel Segmented Sieve using Rayon
fn find_primes_parallel(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }

    let sqrt_limit = (limit as f64).sqrt() as usize;

    // 1. Calculate small primes up to sqrt(limit) sequentially
    let small_primes = find_small_primes(sqrt_limit);

    if limit <= sqrt_limit {
        return small_primes;
    }

    // 2. Define segment size (32 KB aligns well with CPU L1 cache)
    let segment_size = 32_768;
    let start_val = sqrt_limit + 1;
    let num_segments = (limit - start_val) / segment_size + 1;

    // 3. Process segments across available CPU threads in parallel
    let mut segmented_primes: Vec<usize> = (0..num_segments)
        .into_par_iter()
        .flat_map(|seg_idx| {
            let seg_low = start_val + seg_idx * segment_size;
            let seg_high = (seg_low + segment_size - 1).min(limit);

            if seg_low > seg_high {
                return vec![];
            }

            let range_len = seg_high - seg_low + 1;
            let mut is_prime = vec![true; range_len];

            // Mark non-primes within this segment using the pre-computed small primes
            for &p in &small_primes {
                // Find smallest multiple of p that is >= seg_low
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

            // Collect prime numbers in this segment
            (0..range_len)
                .filter_map(|i| if is_prime[i] { Some(seg_low + i) } else { None })
                .collect::<Vec<usize>>()
        })
        .collect();

    // 4. Combine initial small primes with segmented results
    let mut all_primes = small_primes;
    all_primes.append(&mut segmented_primes);
    all_primes
}

fn main() {
    let limit = 100_000_000; // 100 Million

    println!("Calculating primes up to {} using Rayon...", limit);
    let start = Instant::now();

    let primes = find_primes_parallel(limit);

    let duration = start.elapsed();

    println!("Found {} primes in {:.2?}", primes.len(), duration);
}