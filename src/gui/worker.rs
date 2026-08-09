// ============================================================================
// Background Worker Thread for Non-Blocking Parquet I/O & Analytics
// ============================================================================

use std::collections::BTreeMap;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::analysis::gaps::{apply_offset_interval, stream_gaps};
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
    ctx.request_repaint();

    let reader = builder.build()?;
    let gap_iter = apply_offset_interval(stream_gaps(reader), min_idx, max_idx);

    let total_to_read = if max_idx >= min_idx {
        (max_idx - min_idx + 1).min(total_rows.saturating_sub(min_idx - 1))
    } else {
        0
    };

    let mut freq: BTreeMap<u64, u64> = BTreeMap::new();
    let mut raw_pairs: Vec<[f64; 2]> = Vec::with_capacity(total_to_read.min(1_000_000) as usize);
    let mut table_rows: Vec<TableRow> = Vec::with_capacity(total_to_read.min(1_000_000) as usize);

    let mut curr_n = min_idx;
    let mut read_count = 0u64;
    let report_interval = 250_000u64;

    for gap in gap_iter {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        *freq.entry(gap as u64).or_insert(0) += 1;
        raw_pairs.push([curr_n as f64, gap as f64]);
        table_rows.push(TableRow { n: curr_n, gap });

        curr_n += 1;
        read_count += 1;

        if read_count % report_interval == 0 && total_to_read > 0 {
            let progress = (read_count as f32 / total_to_read as f32).min(1.0);
            res_tx.send(WorkerResult::Progress(progress)).ok();
            ctx.request_repaint();
        }
    }

    let mut freq_vec: Vec<(u64, u64)> = freq.into_iter().collect();
    match sort_by {
        SortOrder::ByFrequency => freq_vec.sort_by(|a, b| b.1.cmp(&a.1)),
        SortOrder::ByGapSize => freq_vec.sort_by_key(|&(g, _)| g),
    }
    freq_vec.truncate(top_n);

    res_tx.send(WorkerResult::FrequencyData(freq_vec)).ok();
    res_tx.send(WorkerResult::TableData(table_rows)).ok();

    let downsampled = lttb::downsample(&raw_pairs, 2_000);
    res_tx.send(WorkerResult::ScatterData(downsampled)).ok();
    res_tx.send(WorkerResult::Progress(1.0)).ok();
    ctx.request_repaint();

    Ok(())
}
