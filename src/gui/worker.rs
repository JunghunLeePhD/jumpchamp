use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};

use crate::gui::state::{DatasetMetadata, SortOrder, WorkerCommand, WorkerResult};
use crate::sieve::{small_primes, stream_prime_blocks_range};

const SEGMENT_SIZE: u64 = 5_000_000;

#[derive(Default)]
struct SegmentCache {
    // Key: (segment_index, k_step) -> Value: 65,536-entry gap frequency histogram
    store: HashMap<(u64, usize), Vec<u64>>,
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

    let start_seg = min_val / SEGMENT_SIZE;
    let end_seg = max_val / SEGMENT_SIZE;
    let total_segs = (end_seg - start_seg + 1) as f32;

    let mut counts = vec![0u64; 65536];

    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: max_val,
            unique_gaps: 0,
            min_gap: 0,
            max_gap: 0,
        }))
        .ok();

    for (seg_step, seg_idx) in (start_seg..=end_seg).enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        let key = (seg_idx, k);
        if !cache.store.contains_key(&key) {
            let seg_low = (seg_idx * SEGMENT_SIZE).max(2) as usize;
            let seg_high = ((seg_idx + 1) * SEGMENT_SIZE) as usize;

            let sqrt_limit = (seg_high as f64).sqrt() as usize;
            let base_primes = small_primes(sqrt_limit.max(2));

            let mut seg_counts = vec![0u64; 65536];
            let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);

            for block in stream_prime_blocks_range(seg_low, seg_high, 1_000_000, &base_primes) {
                for p in block {
                    if cancel_flag.load(Ordering::SeqCst) {
                        return Ok(());
                    }
                    window.push_back(p);
                    if window.len() == k + 1 {
                        let p_start = window.pop_front().unwrap();
                        let deltak = (p - p_start) as usize;
                        if deltak < 65536 {
                            seg_counts[deltak] += 1;
                        }
                    }
                }
            }
            cache.store.insert(key, seg_counts);
        }

        // Fast histogram accumulation from cache
        if let Some(cached_vec) = cache.store.get(&key) {
            for (g, &cnt) in cached_vec.iter().enumerate() {
                counts[g] += cnt;
            }
        }

        if total_segs > 0.0 {
            let prog = ((seg_step + 1) as f32 / total_segs).clamp(0.0, 0.99);
            res_tx.send(WorkerResult::Progress(prog)).ok();
            ctx.request_repaint();
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
