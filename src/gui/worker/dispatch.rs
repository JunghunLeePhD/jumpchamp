// ============================================================================
// Background Worker Thread Dispatcher
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};

use crate::gui::state::{
    DatasetMetadata, PrecomputedAnimData, SortOrder, WorkerCommand, WorkerResult,
};
use crate::sieve::{small_primes, stream_prime_blocks_range};

use super::engine::{
    nth_prime_upper_bound, sieve_and_cache, SegmentCache, CHUNK_SIZE, EXACT_THRESHOLD, HIST_SIZE,
    LARGE_BLOCK_SIZE, LARGE_RANGE_THRESHOLD, PROGRESS_INTERVAL_MS, SMALL_BLOCK_SIZE,
};

/// Spawns the background worker thread that receives commands from the GUI.
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
                Ok(WorkerCommand::PrecacheAnimation {
                    min_val,
                    max_val,
                    k,
                    total_frames,
                }) => {
                    cancel_flag.store(false, Ordering::SeqCst);
                    if let Err(err) = run_precache_animation(
                        min_val,
                        max_val,
                        k,
                        total_frames,
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

pub fn run_compute_with_cache(
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
    let mut counts = vec![0u64; HIST_SIZE];

    let total_elements = max_val.saturating_sub(min_val).saturating_add(1);
    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: total_elements,
            unique_gaps: 0,
            min_gap: 0,
            max_gap: 0,
        }))
        .ok();

    let chunk_size = CHUNK_SIZE;
    let start_chunk = min_val / chunk_size;
    let end_chunk = max_val / chunk_size;

    let mut all_cached = max_val > EXACT_THRESHOLD;
    if all_cached {
        for c_idx in start_chunk..=end_chunk {
            if !cache.chunks.contains_key(&(c_idx, k)) {
                all_cached = false;
                break;
            }
        }
    }

    if all_cached && start_chunk <= end_chunk {
        // Fast Cache-Hit Accumulation Path: Optimized bounded slice iteration
        for c_idx in start_chunk..=end_chunk {
            if let Some(hist) = cache.chunks.get(&(c_idx, k)) {
                let max_len = hist.iter().rposition(|&cnt| cnt > 0).map(|i| i + 1).unwrap_or(0);
                for (g, &cnt) in hist[..max_len].iter().enumerate() {
                    if cnt > 0 {
                        counts[g] += cnt;
                    }
                }
            }
        }
    } else {
        let completed = sieve_and_cache(
            min_val, max_val, k, chunk_size, cache, &mut counts, res_tx, ctx, cancel_flag,
        )?;
        if !completed {
            return Ok(());
        }
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
            total_rows: total_elements,
            unique_gaps: unique_gaps_count,
            min_gap: min_gap_val,
            max_gap: max_gap_val,
        }))
        .ok();

    res_tx.send(WorkerResult::FrequencyData(freq_vec)).ok();
    res_tx.send(WorkerResult::QueryLatency(elapsed_ms)).ok();
    res_tx
        .send(WorkerResult::Progress {
            progress: 1.0,
            current_block: 1,
            total_blocks: 1,
        })
        .ok();
    ctx.request_repaint();

    Ok(())
}

pub fn run_precache_animation(
    min_val: u64,
    max_val: u64,
    k: usize,
    total_frames: usize,
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();
    let total_frames = total_frames.max(1);
    let range = max_val.saturating_sub(min_val);
    let step_size = (range / total_frames as u64).max(1);

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

    let mut frame_chunks = vec![vec![0u64; HIST_SIZE]; total_frames];
    let mut last_progress_time = std::time::Instant::now();

    for block in stream_prime_blocks_range(2, sieve_high, block_size, &base_primes) {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(());
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

                if idx_start >= min_val && idx_start <= max_val {
                    let chunk_idx = ((idx_start - min_val) / step_size) as usize;
                    let target_frame = chunk_idx.min(total_frames - 1);
                    let deltak = (p - p_start) as usize;
                    if deltak < HIST_SIZE {
                        frame_chunks[target_frame][deltak] += 1;
                    }
                }
            }
        }
        if prime_idx > target_prime_count {
            break;
        }
    }

    if cancel_flag.load(Ordering::SeqCst) {
        return Ok(());
    }

    // Build Prefix Sums
    let mut prefix_sums = vec![vec![0u64; HIST_SIZE]; total_frames];
    let mut running = vec![0u64; HIST_SIZE];
    for (f, chunk) in frame_chunks.iter().enumerate() {
        for g in 0..HIST_SIZE {
            running[g] += chunk[g];
        }
        prefix_sums[f] = running.clone();
    }

    let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    let final_counts = &prefix_sums[total_frames - 1];
    let mut freq_vec: Vec<(u64, u64)> = final_counts
        .iter()
        .enumerate()
        .filter_map(|(gap, &count)| if count > 0 { Some((gap as u64, count)) } else { None })
        .collect();
    freq_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let unique_gaps_count = freq_vec.len() as u64;
    let min_gap_val = freq_vec.iter().map(|&(g, _)| g).min().unwrap_or(0) as u16;
    let max_gap_val = freq_vec.iter().map(|&(g, _)| g).max().unwrap_or(0) as u16;

    res_tx
        .send(WorkerResult::PrecomputedAnimation(PrecomputedAnimData {
            min_val,
            max_val,
            k,
            total_frames,
            step_size,
            prefix_sums,
        }))
        .ok();

    let total_elements = max_val.saturating_sub(min_val).saturating_add(1);
    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: total_elements,
            unique_gaps: unique_gaps_count,
            min_gap: min_gap_val,
            max_gap: max_gap_val,
        }))
        .ok();

    res_tx.send(WorkerResult::QueryLatency(elapsed_ms)).ok();
    res_tx
        .send(WorkerResult::Progress {
            progress: 1.0,
            current_block: 1,
            total_blocks: 1,
        })
        .ok();
    ctx.request_repaint();

    Ok(())
}
