// ============================================================================
// Execution Pipeline — Single-Column Gap Database Builder Binary
// ============================================================================
//
// Reads primes.parquet, computes Δ_1(n) = p_{n+1} − p_n for every consecutive
// prime pair, and writes single-column gap rows (gap: u16) to gaps.parquet.
//
// Usage:
//   cargo run --release --bin build_gaps
//   cargo run --release --bin build_gaps -- [primes_file] [gaps_file]

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use jumpchamp::{analysis::stream_primes, storage::GapsSink};
use std::{env, fs, fs::File, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let primes_path = args.get(1).cloned().unwrap_or_else(|| "primes.parquet".into());
    let gaps_path   = args.get(2).cloned().unwrap_or_else(|| "gaps.parquet".into());
    let tmp_path    = format!("{}.tmp", gaps_path);

    println!("Building single-column gap database: {} → {}", primes_path, gaps_path);
    let start = Instant::now();

    // Open prime stream
    let file   = File::open(&primes_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

    let mut sink        = GapsSink::create(&tmp_path)?;
    let mut primes_iter = stream_primes(reader).peekable();
    let mut batch: Vec<u16> = Vec::with_capacity(1_000_000);
    let mut count       = 0u64;

    // Slide a 2-element window: emit p_{n+1} - p_n for every consecutive pair
    while let Some(p_n) = primes_iter.next() {
        if let Some(&p_next) = primes_iter.peek() {
            let gap = (p_next - p_n) as u16;
            batch.push(gap);
            count += 1;

            // Flush every 1M gaps to keep memory bounded
            if batch.len() == 1_000_000 {
                sink.write_batch(&batch)?;
                batch.clear();
            }
        }
    }

    if !batch.is_empty() {
        sink.write_batch(&batch)?;
    }

    sink.finish()?;
    fs::rename(&tmp_path, &gaps_path)?;

    let size = fs::metadata(&gaps_path)?.len();

    println!("\n----------------------------------------");
    println!("Gap values written: {}", count);
    println!("Time Elapsed:       {:.2?}", start.elapsed());
    println!("Gap DB File Size:   {:.2} MB", size as f64 / 1_048_576.0);
    if count > 0 {
        println!("Compression Ratio:  {:.3} bytes/gap", size as f64 / count as f64);
    }
    println!("----------------------------------------");

    Ok(())
}
