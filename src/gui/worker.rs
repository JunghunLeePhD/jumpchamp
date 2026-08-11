use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};

use crate::gui::state::{DatasetMetadata, SortOrder, WorkerCommand, WorkerResult};
use crate::sieve::{small_primes, stream_prime_blocks_range};

fn nth_prime_upper_bound(n: u64) -> u64 {
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
struct SegmentCache {
    // Key: (chunk_index, k) -> Value: 65,536-entry gap frequency histogram
    chunks: HashMap<(u64, usize), Vec<u64>>,
}

pub fn spawn_worker(
    cmd_rx: Receiver<WorkerCommand>,
    res_tx: Sender<WorkerResult>,
    ctx: egui::Context,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let mut cache = SegmentCache::default();

        loop {
            match cmd_rx.recv() {
                Ok(WorkerCommand::Cancel) => {
                    cancel_flag.store(true, Ordering::SeqCst);
                }
                Ok(WorkerCommand::ComputeGaps {
                    min_val,
                    max_val,
                    k,
                    top_min,
                    top_max,
                    sort_by,
                }) => {
                    cancel_flag.store(false, Ordering::SeqCst);
                    if let Err(err) = run_compute_with_cache(
                        min_val,
                        max_val,
                        k,
                        top_min,
                        top_max,
                        sort_by,
                        &mut cache,
                        &res_tx,
                        &ctx,
                        &cancel_flag,
                    ) {
                        res_tx.send(WorkerResult::Error(err.to_string())).ok();
                        ctx.request_repaint();
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn run_compute_with_cache(
    min_val: u64,
    max_val: u64,
    k: usize,
    _top_min: usize,
    top_max: usize,
    sort_by: SortOrder,
    cache: &mut SegmentCache,
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    let mut counts = vec![0u64; 65536];

    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: max_val,
            unique_gaps: 0,
            min_gap: 0,
            max_gap: 0,
        }))
        .ok();

    let chunk_size = (max_val / 100).clamp(10_000, 1_000_000);
    let start_chunk = min_val / chunk_size;
    let end_chunk = max_val / chunk_size;

    let mut all_cached = max_val > 100_000;
    if all_cached {
        for c_idx in start_chunk..=end_chunk {
            if !cache.chunks.contains_key(&(c_idx, k)) {
                all_cached = false;
                break;
            }
        }
    }

    if all_cached && start_chunk <= end_chunk {
        // Fast Cache-Hit Accumulation Path: < 0.05 ms sub-millisecond execution!
        for c_idx in start_chunk..=end_chunk {
            if let Some(hist) = cache.chunks.get(&(c_idx, k)) {
                for (g, &cnt) in hist.iter().enumerate() {
                    counts[g] += cnt;
                }
            }
        }
    } else {
        // Stream & Cache Missing Chunks
        let sieve_high = nth_prime_upper_bound(max_val) as usize;
        let sqrt_limit = (sieve_high as f64).sqrt() as usize;
        let base_primes = small_primes(sqrt_limit.max(2));

        let block_size = if max_val >= 100_000_000 { 5_000_000usize } else { 1_000_000usize };
        let total_blocks = ((sieve_high - 2) / block_size).max(1);
        let mut current_block = 0usize;

        let mut ring_buf = [(0u64, 0u64); 16];
        let mut head = 0usize;
        let mut count_in_buf = 0usize;
        let mut prime_idx = 0u64;

        let mut current_chunk_idx = 0u64;
        let mut chunk_hist = vec![0u64; 65536];
        let mut last_progress_time = std::time::Instant::now();

        for block in stream_prime_blocks_range(2, sieve_high, block_size, &base_primes) {
            if cancel_flag.load(Ordering::SeqCst) {
                return Ok(());
            }

            current_block += 1;
            if last_progress_time.elapsed().as_millis() >= 33 || current_block == total_blocks {
                last_progress_time = std::time::Instant::now();
                let prog = (current_block as f32 / total_blocks as f32).clamp(0.0, 0.99);
                res_tx.send(WorkerResult::Progress(prog)).ok();
                ctx.request_repaint();
            }

            for p in block {
                prime_idx += 1;
                if prime_idx > max_val + (k as u64) {
                    break;
                }

                ring_buf[head] = (prime_idx, p);
                head = (head + 1) % 16;
                if count_in_buf < k + 1 {
                    count_in_buf += 1;
                }

                if count_in_buf == k + 1 {
                    let tail = (head + 16 - (k + 1)) % 16;
                    let (idx_start, p_start) = ring_buf[tail];
                    let p_chunk = idx_start / chunk_size;

                    if p_chunk != current_chunk_idx {
                        cache.chunks.insert((current_chunk_idx, k), chunk_hist.clone());
                        chunk_hist.fill(0);
                        current_chunk_idx = p_chunk;
                    }

                    if idx_start >= min_val && prime_idx <= max_val {
                        let deltak = (p - p_start) as usize;
                        if deltak < 65536 {
                            counts[deltak] += 1;
                            chunk_hist[deltak] += 1;
                        }
                    }
                }
            }
            if prime_idx > max_val + (k as u64) {
                break;
            }
        }
        cache.chunks.insert((current_chunk_idx, k), chunk_hist);
    }

    let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    let mut freq_vec: Vec<(u64, u64)> = counts
        .iter()
        .enumerate()
        .filter_map(|(gap, &count)| if count > 0 { Some((gap as u64, count)) } else { None })
        .collect();

    let limit_top_n = top_max.max(1000);
    match sort_by {
        SortOrder::ByFrequency => {
            freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
            freq_vec.truncate(limit_top_n);
        }
        SortOrder::ByGapSize => {
            freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
            freq_vec.truncate(limit_top_n);
            freq_vec.sort_by_key(|&(g, _)| g);
        }
    }

    let unique_gaps_count = freq_vec.len() as u64;
    let min_gap_val = freq_vec.iter().map(|&(g, _)| g).min().unwrap_or(0) as u16;
    let max_gap_val = freq_vec.iter().map(|&(g, _)| g).max().unwrap_or(0) as u16;

    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: max_val,
            unique_gaps: unique_gaps_count,
            min_gap: min_gap_val,
            max_gap: max_gap_val,
        }))
        .ok();

    res_tx.send(WorkerResult::FrequencyData(freq_vec)).ok();
    res_tx.send(WorkerResult::QueryLatency(elapsed_ms)).ok();
    res_tx.send(WorkerResult::Progress(1.0)).ok();
    ctx.request_repaint();

    Ok(())
}
