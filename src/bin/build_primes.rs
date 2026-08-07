// ============================================================================
// Execution Pipeline — Prime Database Builder
// ============================================================================
//
// Usage:
//   cargo run --release --bin build_primes
//   cargo run --release --bin build_primes -- [limit]
//
// Thin orchestration shell. All algorithm and I/O logic lives in the library:
//   jumpchamp::config       — Config struct & CLI parsing
//   jumpchamp::sieve        — small_primes, stream_prime_blocks_range
//   jumpchamp::storage      — ParquetPrimeSink, get_existing_max_prime, copy_existing_parquet

use jumpchamp::{
    config::Config,
    sieve::{small_primes, stream_prime_blocks_range},
    storage::{copy_existing_parquet, get_existing_max_prime, ParquetPrimeSink},
};
use std::{env, fs, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = Config::from_args(&args);
    let tmp_path = format!("{}.tmp", config.output_path);

    let start_time = Instant::now();
    let existing_max = get_existing_max_prime(&config.output_path);

    // 1. Check if existing computation already covers the target limit
    if let Some(max_p) = existing_max {
        if (config.limit as u64) <= max_p {
            println!(
                "File '{}' already contains primes up to {} (>= requested {}). Nothing to do!",
                config.output_path, max_p, config.limit
            );
            return Ok(());
        }
    }

    let mut sink = ParquetPrimeSink::create(&tmp_path)?;
    let mut total_primes = 0;
    let start_val;

    // 2. Handle Initial vs. Incremental Setup
    if let Some(max_p) = existing_max {
        println!(
            "Found existing database up to {}. Resuming computation up to {}...",
            max_p, config.limit
        );
        total_primes += copy_existing_parquet(&config.output_path, &mut sink)?;
        start_val = (max_p + 1) as usize;
    } else {
        println!("Creating new prime database up to {}...", config.limit);
        let sqrt_limit = (config.limit as f64).sqrt() as usize;
        let base_primes = small_primes(sqrt_limit);
        let base_u64: Vec<u64> = base_primes.iter().map(|&p| p as u64).collect();

        sink.write_batch(&base_u64)?;
        total_primes += base_primes.len();
        start_val = sqrt_limit + 1;
    }

    // 3. Compute and append the new range
    if start_val <= config.limit {
        let sqrt_limit = (config.limit as f64).sqrt() as usize;
        let base_primes = small_primes(sqrt_limit);

        for block in stream_prime_blocks_range(start_val, config.limit, config.block_size, &base_primes) {
            total_primes += block.len();
            sink.write_batch(&block)?;
        }
    }

    // 4. Finalize sink and atomically swap tmp → output
    sink.finish()?;
    fs::rename(&tmp_path, &config.output_path)?;

    // 5. Output summary
    let duration = start_time.elapsed();
    let file_size_bytes = fs::metadata(&config.output_path)?.len();
    let bytes_per_prime = file_size_bytes as f64 / total_primes as f64;

    println!("\n----------------------------------------");
    println!("Total Primes in DB: {}", total_primes);
    println!("Time Elapsed:       {:.2?}", duration);
    println!("Parquet File Size:  {:.2} MB", file_size_bytes as f64 / 1_048_576.0);
    println!("Compression Ratio:  {:.2} bytes/prime", bytes_per_prime);
    println!("----------------------------------------");

    Ok(())
}
