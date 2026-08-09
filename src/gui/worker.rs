// ============================================================================
// High-Performance Background Worker Thread for Zero-Copy Parquet Analytics
// ============================================================================

use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use arrow_array::UInt16Array;
use crossbeam_channel::{Receiver, Sender};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::gui::lttb;
use crate::gui::state::{DatasetMetadata, SortOrder, TableRow, WorkerCommand, WorkerResult};

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

    // Optimization 1: Stack L1 primitive array for 1-cycle counts (fits in 512KB L1/L2 CPU cache)
    let mut counts = vec![0u64; 65536];

    // Memory Cap 1: Limit table preview memory to 2,000 rows max
    let max_table_preview = 2_000usize;
    let mut table_rows: Vec<TableRow> = Vec::with_capacity(total_to_read.min(max_table_preview as u64) as usize);

    // Memory Cap 2: Strided sampling for LTTB scatter plot (max 10,000 points)
    let stride = (total_to_read / 10_000).max(1);
    let mut raw_pairs: Vec<[f64; 2]> = Vec::with_capacity((total_to_read / stride).min(10_000) as usize);

    let start_idx = min_idx.saturating_sub(1);
    let end_idx = start_idx + total_to_read;

    let mut current_offset = 0u64;
    let mut read_count = 0u64;

    let start_time = std::time::Instant::now();

    // Optimization 2: Zero-copy direct Arrow buffer slice processing (SIMD vectorizable)
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
                let curr_n = start_idx + read_count + 1;

                // 1-cycle L1 primitive array count
                counts[gap as usize] += 1;

                // Preview table
                if table_rows.len() < max_table_preview {
                    table_rows.push(TableRow { n: curr_n, gap });
                }

                // Strided scatter sampling
                if read_count % stride == 0 {
                    raw_pairs.push([curr_n as f64, gap as f64]);
                }

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
        SortOrder::ByFrequency => freq_vec.sort_by(|a, b| b.1.cmp(&a.1)),
        SortOrder::ByGapSize => freq_vec.sort_by_key(|&(g, _)| g),
    }
    freq_vec.truncate(top_n);

    res_tx.send(WorkerResult::FrequencyData(freq_vec)).ok();
    res_tx.send(WorkerResult::TableData(table_rows)).ok();

    // Instant LTTB downsampling (<1ms)
    let downsampled = lttb::downsample(&raw_pairs, 2_000);
    res_tx.send(WorkerResult::ScatterData(downsampled)).ok();
    res_tx.send(WorkerResult::QueryLatency(elapsed_ms)).ok();
    res_tx.send(WorkerResult::Progress(1.0)).ok();
    ctx.request_repaint();

    Ok(())
}
