// ============================================================================
// Execution Pipeline — 2-Step Gap Database Builder Binary
// ============================================================================
//
// Reads primes.parquet, computes Δ_2(n) = p_{n+2} − p_n for every 2-step
// prime pair, and writes single-column gap rows (delta2: u16) to gaps2.parquet.
//
// Usage:
//   cargo run --release --bin build_gaps2
//   cargo run --release --bin build_gaps2 -- [primes_file] [gaps2_file]

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use jumpchamp::{analysis::stream_primes, storage::gaps2_parquet::GapsSink2};
use std::{env, fs, fs::File, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let primes_path = args.get(1).cloned().unwrap_or_else(|| "primes.parquet".into());
    let gaps2_path  = args.get(2).cloned().unwrap_or_else(|| "gaps2.parquet".into());
    let tmp_path    = format!("{}.tmp", gaps2_path);

    println!("Building single-column 2-step gap database (k=2): {} → {}", primes_path, gaps2_path);
    let start = Instant::now();

    // Open prime stream
    let file   = File::open(&primes_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

    let mut sink        = GapsSink2::create(&tmp_path)?;
    let mut primes_iter = stream_primes(reader);

    let mut p0 = match primes_iter.next() { Some(p) => p, None => return Ok(()) };
    let mut p1 = match primes_iter.next() { Some(p) => p, None => return Ok(()) };

    let mut batch: Vec<u16> = Vec::with_capacity(1_000_000);
    let mut count       = 0u64;

    for p2 in primes_iter {
        let delta2 = (p2 - p0) as u16;
        batch.push(delta2);
        count += 1;

        // Flush every 1M gaps to keep memory bounded
        if batch.len() == 1_000_000 {
            sink.write_batch(&batch)?;
            batch.clear();
        }

        p0 = p1;
        p1 = p2;
    }

    if !batch.is_empty() {
        sink.write_batch(&batch)?;
    }

    sink.finish()?;
    fs::rename(&tmp_path, &gaps2_path)?;

    let size = fs::metadata(&gaps2_path)?.len();

    println!("\n----------------------------------------");
    println!("2-step gap values written: {}", count);
    println!("Time Elapsed:              {:.2?}", start.elapsed());
    println!("File Size:                 {:.2} MB", size as f64 / 1_048_576.0);
    if count > 0 {
        println!("Compression Ratio:         {:.3} bytes/gap", size as f64 / count as f64);
    }
    println!("----------------------------------------");

    Ok(())
}
