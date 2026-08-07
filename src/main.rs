// ============================================================================
// Execution Pipeline — Gap Analyzer (default binary)
// ============================================================================
//
// Usage:
//   cargo run --release -- [k] [min_idx] [max_idx] [gaps_file]
//
// Auto-detects gaps.parquet (single-column fast path) and falls back to primes.parquet (slow path).
//
// Fast path  (gaps.parquet present — ~95 MB single-column file):
//   stream_gaps → apply_offset_interval → k_step_gaps_from_gaps → count_frequencies
//
// Slow path  (primes.parquet only):
//   stream_primes → apply_interval → k_step_gaps → count_frequencies

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use jumpchamp::analysis::{
    apply_interval, apply_offset_interval, count_frequencies, format_report,
    k_step_gaps, k_step_gaps_from_gaps, stream_gaps, stream_primes,
};
use std::{env, fs::File, path::Path, time::Instant};

// ============================================================================
// Analyzer Configuration
// ============================================================================

#[derive(Debug, Clone)]
struct AnalyzeConfig {
    k: usize,
    min_idx: u64,
    max_idx: u64,
    file_path: String,
}

impl AnalyzeConfig {
    fn from_args(args: &[String]) -> Self {
        Self {
            k: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2),
            min_idx: args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1),
            max_idx: args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX),
            file_path: args.get(4).cloned().unwrap_or_else(|| "primes.parquet".into()),
        }
    }

    /// Derives the expected gaps database path: same directory as the primes file,
    /// named `gaps.parquet`.
    fn gaps_path(&self) -> String {
        Path::new(&self.file_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("gaps.parquet")
            .to_string_lossy()
            .into_owned()
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = AnalyzeConfig::from_args(&args);

    if config.k == 0 {
        eprintln!("Error: Step size k must be >= 1");
        std::process::exit(1);
    }

    let gaps_path = config.gaps_path();
    let use_gaps  = Path::new(&gaps_path).exists();

    println!("Analyzing prime gaps (p_{{n+{}}} - p_n)", config.k);
    println!("Index Interval:  [n={}, m={}]", config.min_idx, config.max_idx);

    let frequencies;
    let start_time = Instant::now();

    if use_gaps {
        // ── Fast path: stream single-column (gap: u16) by row offset ────────────
        println!("Source:          {} (single-column gap database — fast path)\n", gaps_path);

        let file   = File::open(&gaps_path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        frequencies = count_frequencies(k_step_gaps_from_gaps(
            apply_offset_interval(stream_gaps(reader), config.min_idx, config.max_idx),
            config.k,
        ));
    } else {
        // ── Slow path: derive gaps on the fly from primes.parquet ───────────────
        println!("Source:          {} (prime database — slow path)\n", config.file_path);
        println!("  Tip: run `cargo run --release --bin build_gaps` to build gaps.parquet");
        println!("       for faster single-column gap queries.\n");

        let file   = File::open(&config.file_path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        frequencies = count_frequencies(k_step_gaps(
            apply_interval(stream_primes(reader), config.min_idx, config.max_idx),
            config.k,
        ));
    }

    let duration = start_time.elapsed();

    print!("{}", format_report(&frequencies, 20));
    println!("Time Elapsed: {:.2?}\n", duration);

    Ok(())
}