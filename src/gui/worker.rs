use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};

use crate::gui::state::{DatasetMetadata, SortOrder, WorkerCommand, WorkerResult};
use crate::sieve::{small_primes, stream_prime_blocks_range};

pub fn spawn_worker(
    cmd_rx: Receiver<WorkerCommand>,
    res_tx: Sender<WorkerResult>,
    ctx: egui::Context,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let cancel_flag = Arc::new(AtomicBool::new(false));

        loop {
            match cmd_rx.recv() {
                Ok(WorkerCommand::Cancel) => {
                    cancel_flag.store(true, Ordering::SeqCst);
                }
                Ok(WorkerCommand::ComputeGaps {
                    min_idx,
                    max_idx,
                    k,
                    top_n,
                    sort_by,
                }) => {
                    cancel_flag.store(false, Ordering::SeqCst);
                    if let Err(err) = run_compute_in_memory(
                        min_idx,
                        max_idx,
                        k,
                        top_n,
                        sort_by,
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

fn estimate_nth_prime_bound(n: u64) -> usize {
    if n < 6 {
        match n {
            0 => 2,
            1 => 3,
            2 => 5,
            3 => 7,
            4 => 11,
            _ => 13,
        }
    } else {
        let nf = n as f64;
        let ln_n = nf.ln();
        let ln_ln_n = ln_n.ln();
        let bound = nf * (ln_n + ln_ln_n);
        (bound * 1.05) as usize + 20
    }
}

fn run_compute_in_memory(
    min_idx: u64,
    max_idx: u64,
    k: usize,
    top_n: usize,
    sort_by: SortOrder,
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();

    let total_primes_needed = max_idx.saturating_add(k as u64);
    let limit_val = estimate_nth_prime_bound(total_primes_needed);
    let sqrt_limit = (limit_val as f64).sqrt() as usize;

    let base_primes = small_primes(sqrt_limit);
    let block_size = 1_000_000;

    let mut counts = vec![0u64; 65536];
    let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);

    let mut prime_idx = 0u64;
    let mut processed_gaps = 0u64;
    let total_gaps_target = if max_idx >= min_idx {
        max_idx - min_idx + 1
    } else {
        0
    };

    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: max_idx,
            unique_gaps: 0,
            min_gap: 0,
            max_gap: 0,
        }))
        .ok();

    'outer: for block in stream_prime_blocks_range(1, limit_val, block_size, &base_primes) {
        for p in block {
            if cancel_flag.load(Ordering::SeqCst) {
                return Ok(());
            }

            prime_idx += 1;
            window.push_back(p);

            if window.len() == k + 1 {
                let p_n = window.pop_front().unwrap();
                let gap_prime_idx = prime_idx - k as u64;

                if gap_prime_idx >= min_idx && gap_prime_idx <= max_idx {
                    let deltak = (p - p_n) as usize;
                    if deltak < 65536 {
                        counts[deltak] += 1;
                    }
                    processed_gaps += 1;

                    if processed_gaps % 500_000 == 0 && total_gaps_target > 0 {
                        let prog = (processed_gaps as f32 / total_gaps_target as f32).min(0.99);
                        res_tx.send(WorkerResult::Progress(prog)).ok();
                        ctx.request_repaint();
                    }
                }

                if gap_prime_idx >= max_idx {
                    break 'outer;
                }
            }
        }
    }

    let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    let mut freq_vec: Vec<(u64, u64)> = counts
        .iter()
        .enumerate()
        .filter_map(|(gap, &count)| if count > 0 { Some((gap as u64, count)) } else { None })
        .collect();

    match sort_by {
        SortOrder::ByFrequency => {
            freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
            freq_vec.truncate(top_n);
        }
        SortOrder::ByGapSize => {
            freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
            freq_vec.truncate(top_n);
            freq_vec.sort_by_key(|&(g, _)| g);
        }
    }

    res_tx.send(WorkerResult::FrequencyData(freq_vec)).ok();
    res_tx.send(WorkerResult::QueryLatency(elapsed_ms)).ok();
    res_tx.send(WorkerResult::Progress(1.0)).ok();
    ctx.request_repaint();

    Ok(())
}
