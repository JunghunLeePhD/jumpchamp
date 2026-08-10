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
                    min_val,
                    max_val,
                    k,
                    top_n,
                    sort_by,
                }) => {
                    cancel_flag.store(false, Ordering::SeqCst);
                    if let Err(err) = run_compute_in_memory(
                        min_val,
                        max_val,
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

fn run_compute_in_memory(
    min_val: u64,
    max_val: u64,
    k: usize,
    top_n: usize,
    sort_by: SortOrder,
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = std::time::Instant::now();

    let start_val = min_val.max(2) as usize;
    let limit_val = max_val.max(start_val as u64) as usize;
    let range_span = (limit_val - start_val + 1) as f32;

    let sqrt_limit = (limit_val as f64).sqrt() as usize;
    let base_primes = small_primes(sqrt_limit.max(2));
    let block_size = 1_000_000;

    let mut counts = vec![0u64; 65536];
    let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);

    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows: max_val,
            unique_gaps: 0,
            min_gap: 0,
            max_gap: 0,
        }))
        .ok();

    for block in stream_prime_blocks_range(start_val, limit_val, block_size, &base_primes) {
        for p in block {
            if cancel_flag.load(Ordering::SeqCst) {
                return Ok(());
            }

            window.push_back(p);

            if window.len() == k + 1 {
                let p_start = window.pop_front().unwrap();
                let deltak = (p - p_start) as usize;
                if deltak < 65536 {
                    counts[deltak] += 1;
                }
            }

            if p % 500_000 < 2 && range_span > 0.0 {
                let prog = ((p as f32 - start_val as f32) / range_span).clamp(0.0, 0.99);
                res_tx.send(WorkerResult::Progress(prog)).ok();
                ctx.request_repaint();
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
