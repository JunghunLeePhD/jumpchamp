// ============================================================================
// Execution Pipeline — Gap Analyzer (default binary)
// ============================================================================
//
// Usage:
//   cargo run --release -- [k] [min_idx] [max_idx] [primes_file] [--force]
//
// Requires pre-computed gaps database (gaps{k}.parquet or gaps.parquet).
// Pass '--force' or '-f' to run slow path on primes.parquet directly if gaps database is missing.
//
// Fast path  (gaps{k}.parquet / gaps.parquet present — single-column u16 file):
//   stream_gaps → apply_offset_interval → k_step_gaps_from_gaps → count_frequencies
//
// Slow path  (primes.parquet with --force):
//   stream_primes → apply_interval → k_step_gaps → count_frequencies

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use jumpchamp::analysis::{
    apply_interval, apply_offset_interval, count_frequencies, format_report,
    k_step_gaps, k_step_gaps_from_gaps, stream_gaps, stream_primes,
};
use jumpchamp::config::AnalyzeConfig;
use std::{env, fs::File, path::Path, time::Instant};

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

    if !use_gaps && !config.force {
        eprintln!("❌ Error: Pre-computed gaps database '{}' not found.\n", gaps_path);
        eprintln!("Please prepare the gaps database first by running:");
        eprintln!("  cargo run --release --bin build_gaps -- {}\n", config.k);
        eprintln!("Or pass '--force' (or '-f') to compute gaps directly from '{}' (slow path):", config.file_path);
        eprintln!("  cargo run --release -- {} {} {} --force\n", config.k, config.min_idx, config.max_idx);
        std::process::exit(1);
    }

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
        println!("Source:          {} (prime database — slow path via --force)\n", config.file_path);

        let file   = File::open(&config.file_path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

        frequencies = count_frequencies(k_step_gaps(
            apply_interval(stream_primes(reader), config.min_idx, config.max_idx),
            config.k,
        ));
    }

    let duration = start_time.elapsed();

    print!("{}", format_report(&frequencies, 1, 20));
    println!("Time Elapsed: {:.2?}\n", duration);

    Ok(())
}