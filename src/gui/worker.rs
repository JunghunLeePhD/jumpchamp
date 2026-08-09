use std::collections::VecDeque;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use arrow_array::UInt16Array;
use crossbeam_channel::{Receiver, Sender};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::analysis::stream_primes;
use crate::config::{default_gaps_path, default_primes_path};
use crate::gui::state::{DatasetMetadata, SortOrder, WorkerCommand, WorkerResult};
use crate::sieve::{small_primes, stream_prime_blocks_range};
use crate::storage::{gaps_parquet::GapsSink, ParquetPrimeSink};

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
                Ok(WorkerCommand::GenerateDatabase { limit, k }) => {
                    cancel_flag.store(false, Ordering::SeqCst);
                    if let Err(err) = run_generate_database(
                        limit,
                        k,
                        &res_tx,
                        &ctx,
                        &cancel_flag,
                    ) {
                        res_tx.send(WorkerResult::Error(err.to_string())).ok();
                        ctx.request_repaint();
                    } else {
                        let path = default_gaps_path(k).to_string_lossy().into_owned();
                        let _ = run_load(
                            &path,
                            1,
                            1_000_000,
                            k,
                            20,
                            SortOrder::ByFrequency,
                            &res_tx,
                            &ctx,
                            &cancel_flag,
                        );
                    }
                }
                Ok(WorkerCommand::LoadParquet {
                    path,
                    min_idx,
                    max_idx,
                    k,
                    top_n,
                    sort_by,
                }) => {
                    cancel_flag.store(false, Ordering::SeqCst);
                    if let Err(err) = run_load(
                        &path,
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

fn run_generate_database(
    limit: usize,
    k: usize,
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let primes_path = default_primes_path();
    let gaps_path = default_gaps_path(k);
    let block_size = 10_000_000;

    let tmp_primes = primes_path.with_extension("parquet.tmp");
    let tmp_gaps = gaps_path.with_extension("parquet.tmp");
    let tmp_primes_str = tmp_primes.to_str().unwrap_or("primes.parquet.tmp");
    let tmp_gaps_str = tmp_gaps.to_str().unwrap_or("gaps2.parquet.tmp");

    // Phase 1: Generate Primes (Progress 0.0 -> 0.5)
    let sqrt_limit = (limit as f64).sqrt() as usize;
    let base_primes = small_primes(sqrt_limit);
    let base_u64: Vec<u64> = base_primes.iter().map(|&p| p as u64).collect();

    let mut prime_sink = ParquetPrimeSink::create(tmp_primes_str).map_err(|e| e.to_string())?;
    prime_sink.write_batch(&base_u64).map_err(|e| e.to_string())?;
    let mut total_primes_written = base_primes.len();

    let start_val = sqrt_limit + 1;
    if start_val <= limit {
        for block in stream_prime_blocks_range(start_val, limit, block_size, &base_primes) {
            if cancel_flag.load(Ordering::SeqCst) {
                return Ok(());
            }
            total_primes_written += block.len();
            prime_sink.write_batch(&block).map_err(|e| e.to_string())?;

            let prog = 0.5 * (total_primes_written as f32 / (limit as f32 / (limit as f32).ln()));
            res_tx.send(WorkerResult::Progress(prog.min(0.48))).ok();
            ctx.request_repaint();
        }
    }
    prime_sink.finish().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_primes, &primes_path)?;

    res_tx.send(WorkerResult::Progress(0.5)).ok();
    ctx.request_repaint();

    // Phase 2: Compute Gaps (Progress 0.5 -> 1.0)
    let file = File::open(&primes_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

    let mut gaps_sink = GapsSink::create(tmp_gaps_str).map_err(|e| e.to_string())?;
    let primes_iter = stream_primes(reader);

    let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);
    let mut batch: Vec<u16> = Vec::with_capacity(1_000_000);
    let mut gap_count = 0u64;

    for p in primes_iter {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(());
        }
        window.push_back(p);
        if window.len() == k + 1 {
            let p_n = window.pop_front().unwrap();
            let deltak = (p - p_n) as u16;
            batch.push(deltak);
            gap_count += 1;

            if batch.len() == 1_000_000 {
                gaps_sink.write_batch(&batch).map_err(|e| e.to_string())?;
                batch.clear();

                let prog = 0.5 + 0.5 * (gap_count as f32 / total_primes_written as f32);
                res_tx.send(WorkerResult::Progress(prog.min(0.98))).ok();
                ctx.request_repaint();
            }
        }
    }

    if !batch.is_empty() {
        gaps_sink.write_batch(&batch).map_err(|e| e.to_string())?;
    }

    gaps_sink.finish().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_gaps, &gaps_path)?;

    res_tx.send(WorkerResult::Progress(1.0)).ok();
    ctx.request_repaint();

    Ok(())
}

fn run_load(
    path: &str,
    min_idx: u64,
    max_idx: u64,
    _k: usize,
    top_n: usize,
    sort_by: SortOrder,
    res_tx: &Sender<WorkerResult>,
    ctx: &egui::Context,
    cancel_flag: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let metadata = builder.metadata().clone();
    let total_rows = metadata.file_metadata().num_rows() as u64;

    res_tx
        .send(WorkerResult::Metadata(DatasetMetadata {
            total_rows,
            unique_gaps: 0,
            min_gap: 0,
            max_gap: 0,
        }))
        .ok();

    let reader = builder.build()?;

    let total_to_read = if max_idx >= min_idx {
        (max_idx - min_idx + 1).min(total_rows.saturating_sub(min_idx - 1))
    } else {
        0
    };

    // Stack L1 primitive array for 1-cycle counts (fits in 512KB L1/L2 CPU cache)
    let mut counts = vec![0u64; 65536];

    let start_idx = min_idx.saturating_sub(1);
    let end_idx = start_idx + total_to_read;

    let mut current_offset = 0u64;
    let mut read_count = 0u64;

    let start_time = std::time::Instant::now();

    // Zero-copy direct Arrow buffer slice processing (SIMD vectorizable)
    for batch_result in reader {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        let batch = batch_result?;
        let batch_len = batch.num_rows() as u64;
        let batch_start = current_offset;
        let batch_end = current_offset + batch_len;

        // Check if current batch overlaps with [start_idx, end_idx)
        if batch_end > start_idx && batch_start < end_idx {
            let slice_start = start_idx.saturating_sub(batch_start) as usize;
            let slice_end = ((end_idx.saturating_sub(batch_start)) as usize).min(batch.num_rows());

            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<UInt16Array>()
                .ok_or("Expected UInt16Array")?;

            let values = col.values();
            let slice = &values[slice_start..slice_end];

            for &gap in slice {
                // 1-cycle L1 primitive array count
                counts[gap as usize] += 1;
                read_count += 1;

                if read_count % 500_000 == 0 && total_to_read > 0 {
                    let progress = (read_count as f32 / total_to_read as f32).min(1.0);
                    res_tx.send(WorkerResult::Progress(progress)).ok();
                    ctx.request_repaint();
                }
            }
        }

        current_offset += batch_len;
        if current_offset >= end_idx {
            break;
        }
    }

    let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    // Convert non-zero counts from L1 array into frequency vector
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
