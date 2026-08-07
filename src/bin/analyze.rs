use arrow_array::UInt64Array;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs::File;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // 1. Step size k (default: 2)
    let k: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);

    // 2. Minimum prime bound A (default: 0)
    let min_prime: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    // 3. Maximum prime bound B (default: u64::MAX)
    let max_prime: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX);

    // 4. Parquet file path
    let file_path = args.get(4).map(|s| s.as_str()).unwrap_or("primes.parquet");

    if k == 0 {
        eprintln!("Error: Step size k must be >= 1");
        std::process::exit(1);
    }

    println!("Analyzing prime gaps (p_{{n+{}}} - p_n)", k);
    println!("Interval:  [{}, {}]", min_prime, if max_prime == u64::MAX { "MAX".to_string() } else { max_prime.to_string() });
    println!("File:      {}\n", file_path);

    let start_time = Instant::now();

    let file = File::open(file_path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let mut reader = builder.build()?;

    let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);
    let mut freq_map: BTreeMap<u64, u64> = BTreeMap::new();
    let mut total_pairs = 0u64;
    let mut finished = false;

    // Stream record batches from Parquet
    while let Some(Ok(batch)) = reader.next() {
        if finished {
            break;
        }

        let column = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Expected UInt64Array in first column");

        for &p in column.values() {
            // Early exit as soon as we cross the upper bound
            if p > max_prime {
                finished = true;
                break;
            }

            // Only include primes within the specified interval
            if p >= min_prime {
                window.push_back(p);

                if window.len() == k + 1 {
                    let p_n = window.pop_front().unwrap();
                    let diff = p - p_n;
                    *freq_map.entry(diff).or_insert(0) += 1;
                    total_pairs += 1;
                }
            }
        }
    }

    let duration = start_time.elapsed();

    // Sort results by frequency (descending)
    let mut sorted_freq: Vec<(u64, u64)> = freq_map.into_iter().collect();
    sorted_freq.sort_by(|a, b| b.1.cmp(&a.1));

    // Print summary table
    println!("{:<12} {:<15} {:<12}", "Diff", "Frequency", "Percentage");
    println!("{}", "-".repeat(42));

    for (diff, count) in sorted_freq.iter().take(20) {
        let percentage = (*count as f64 / total_pairs as f64) * 100.0;
        println!("{:<12} {:<15} {:.2}%", diff, count, percentage);
    }

    println!("{}", "-".repeat(42));
    println!("Processed {} prime pairs in {:.2?}\n", total_pairs, duration);

    Ok(())
}