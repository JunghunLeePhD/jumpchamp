// ============================================================================
// Execution Pipeline — Prime Gap Analyzer Binary
// ============================================================================
//
// Thin orchestration shell. All algorithm and formatting logic lives in the library:
//   primes::analysis  — stream_primes, apply_interval, k_step_gaps, count_frequencies, format_report

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use primes::analysis::{apply_interval, count_frequencies, format_report, k_step_gaps, stream_primes};
use std::{env, fs::File, time::Instant};

// ============================================================================
// Analyzer Configuration
// ============================================================================

#[derive(Debug, Clone)]
struct AnalyzeConfig {
    k: usize,
    min_prime: u64,
    max_prime: u64,
    file_path: String,
}

impl AnalyzeConfig {
    fn from_args(args: &[String]) -> Self {
        Self {
            k: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2),
            min_prime: args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
            max_prime: args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX),
            file_path: args.get(4).cloned().unwrap_or_else(|| "primes.parquet".into()),
        }
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

    println!("Analyzing prime gaps (p_{{n+{}}} - p_n)", config.k);
    println!("Interval:  [{}, {}]", config.min_prime, config.max_prime);
    println!("File:      {}\n", config.file_path);

    let start_time = Instant::now();

    // Open I/O stream
    let file = File::open(&config.file_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;

    // Declarative FP pipeline
    let frequencies = count_frequencies(k_step_gaps(
        apply_interval(stream_primes(reader), config.min_prime, config.max_prime),
        config.k,
    ));

    let duration = start_time.elapsed();

    print!("{}", format_report(&frequencies, 20));
    println!("Time Elapsed: {:.2?}\n", duration);

    Ok(())
}