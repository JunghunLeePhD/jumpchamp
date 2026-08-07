// ============================================================================
// Execution Pipeline — Generalized k-Step Gap Database Builder Binary
// ============================================================================
//
// Reads primes.parquet, computes Δ_k(n) = p_{n+k} − p_n for every k-step
// prime pair, and writes single-column gap rows (deltak: u16) to gaps{k}.parquet.
//
// Usage:
//   cargo run --release --bin build_gaps                     # Default: k=2 -> gaps2.parquet
//   cargo run --release --bin build_gaps -- 3                # k=3 -> gaps3.parquet
//   cargo run --release --bin build_gaps -- 6 primes.parquet custom.parquet

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use jumpchamp::{analysis::stream_primes, storage::gaps_parquet::GapsSink};
use std::{collections::VecDeque, env, fs, fs::File, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let k: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    if k == 0 {
        eprintln!("Error: Step size k must be >= 1");
        std::process::exit(1);
    }

    let primes_path = args.get(2).cloned().unwrap_or_else(|| "primes.parquet".into());
    let default_output = format!("gaps{}.parquet", k);
    let output_path = args.get(3).cloned().unwrap_or(default_output);
    let tmp_path = format!("{}.tmp", output_path);

    println!("Building pre-computed {}-step gap database (k={}): {} → {}", k, k, primes_path, output_path);
    let start = Instant::now();

    // Open prime stream
    let file = File::open(&primes_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

    let mut sink = GapsSink::create(&tmp_path)?;
    let primes_iter = stream_primes(reader);

    let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);
    let mut batch: Vec<u16> = Vec::with_capacity(1_000_000);
    let mut count = 0u64;

    // Slide a (k+1)-element window: emit p_{n+k} - p_n for every k-step pair
    for p in primes_iter {
        window.push_back(p);
        if window.len() == k + 1 {
            let p_n = window.pop_front().unwrap();
            let deltak = (p - p_n) as u16;
            batch.push(deltak);
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
    fs::rename(&tmp_path, &output_path)?;

    let size = fs::metadata(&output_path)?.len();

    println!("\n----------------------------------------");
    println!("{}-step gap values written: {}", k, count);
    println!("Time Elapsed:              {:.2?}", start.elapsed());
    println!("Gap DB File Size:          {:.2} MB", size as f64 / 1_048_576.0);
    if count > 0 {
        println!("Compression Ratio:         {:.3} bytes/gap", size as f64 / count as f64);
    }
    println!("----------------------------------------");

    Ok(())
}
