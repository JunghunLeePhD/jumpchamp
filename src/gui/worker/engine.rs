// ============================================================================
// Worker Sieve Engine & Segment Cache
// ============================================================================

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use crossbeam_channel::Sender;

use crate::gui::state::WorkerResult;
use crate::sieve::{small_primes, stream_prime_blocks_range};

pub const HIST_SIZE: usize = 65_536;
pub const LARGE_RANGE_THRESHOLD: u64 = 100_000_000;
pub const LARGE_BLOCK_SIZE: usize = 5_000_000;
pub const SMALL_BLOCK_SIZE: usize = 1_000_000;
pub const EXACT_THRESHOLD: u64 = 100_000;
pub const PROGRESS_INTERVAL_MS: u128 = 33;
pub const CHUNK_SIZE: u64 = 10_000;

/// Computes upper bound for the n-th prime number using prime number theorem bounds.
pub fn nth_prime_upper_bound(n: u64) -> u64 {
    if n <= 5 {
        match n {
            0 | 1 => 3,
            2 => 5,
            3 => 7,
            4 => 11,
            _ => 13,
        }
    } else {
        let nf = n as f64;
        let ln_n = nf.ln();
        let ln_ln_n = ln_n.ln().max(0.1);
        (nf * (ln_n + ln_ln_n)).ceil() as u64 + 1000
    }
}

#[derive(Default)]
pub struct SegmentCache {
    // Key: (chunk_index, k) -> Value: 65,536-entry gap frequency histogram
    pub chunks: HashMap<(u64, usize), Vec<u64>>,
}

/// Computes prime gaps and populates the segment histogram cache.
pub fn sieve_and_cache(
    min_val: u64,
    max_val: u64,
    k: usize,
    chunk_size: u64,
    cache: &mut SegmentCache,
    counts: &mut [u64],
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let target_prime_count = max_val.saturating_add(k as u64);
    let sieve_high = nth_prime_upper_bound(target_prime_count) as usize;
    let sqrt_limit = (sieve_high as f64).sqrt() as usize;
    let base_primes = small_primes(sqrt_limit.max(2));

    let block_size = if target_prime_count >= LARGE_RANGE_THRESHOLD {
        LARGE_BLOCK_SIZE
    } else {
        SMALL_BLOCK_SIZE
    };
    let total_blocks = ((sieve_high - 2) / block_size).max(1);
    let mut current_block = 0usize;

    let ring_capacity = (k + 1).max(16);
    let mut ring_buf = vec![(0u64, 0u64); ring_capacity];
    let mut head = 0usize;
    let mut count_in_buf = 0usize;
    let mut prime_idx = 0u64;

    let mut current_chunk_idx = 0u64;
    let mut chunk_hist = vec![0u64; HIST_SIZE];
    let mut last_progress_time = std::time::Instant::now();

    for block in stream_prime_blocks_range(2, sieve_high, block_size, &base_primes) {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(false);
        }

        current_block += 1;
        if last_progress_time.elapsed().as_millis() >= PROGRESS_INTERVAL_MS || current_block == total_blocks {
            last_progress_time = std::time::Instant::now();
            let prog = (current_block as f32 / total_blocks as f32).clamp(0.0, 0.99);
            res_tx
                .send(WorkerResult::Progress {
                    progress: prog,
                    current_block,
                    total_blocks,
                })
                .ok();
            ctx.request_repaint();
        }

        for p in block {
            prime_idx += 1;
            if prime_idx > target_prime_count {
                break;
            }

            ring_buf[head] = (prime_idx, p);
            head = (head + 1) % ring_capacity;
            if count_in_buf < k + 1 {
                count_in_buf += 1;
            }

            if count_in_buf == k + 1 {
                let tail = (head + ring_capacity - (k + 1)) % ring_capacity;
                let (idx_start, p_start) = ring_buf[tail];
                let p_chunk = idx_start / chunk_size;

                if p_chunk != current_chunk_idx {
                    cache.chunks.insert((current_chunk_idx, k), chunk_hist.clone());
                    chunk_hist.fill(0);
                    current_chunk_idx = p_chunk;
                }

                let deltak = (p - p_start) as usize;
                if deltak < HIST_SIZE {
                    chunk_hist[deltak] += 1;
                    if idx_start >= min_val && idx_start <= max_val {
                        counts[deltak] += 1;
                    }
                }
            }
        }
        if prime_idx > target_prime_count {
            break;
        }
    }
    cache.chunks.insert((current_chunk_idx, k), chunk_hist);
    Ok(true)
}
