use arrow_array::UInt64Array;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs::File;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Argument 1: Step size k (default: 2)
    let k: usize = if args.len() > 1 {
        args[1].parse().unwrap_or_else(|_| {
            eprintln!("Error: Step size k must be a positive integer (>= 1)");
            std::process::exit(1);
        })
    } else {
        2
    };

    if k == 0 {
        eprintln!("Error: Step size k must be at least 1");
        std::process::exit(1);
    }

    // Argument 2: Parquet file path (default: "primes.parquet")
    let file_path = if args.len() > 2 {
        &args[2]
    } else {
        "primes.parquet"
    };

    println!(
        "Analyzing prime gaps (p_{{n+{}}} - p_n) from: {}\n",
        k, file_path
    );
    let start_time = Instant::now();

    // 1. Open Parquet Reader
    let file = File::open(file_path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let mut reader = builder.build()?;

    // 2. Sliding window buffer of length k + 1
    let mut window: VecDeque<u64> = VecDeque::with_capacity(k + 1);
    let mut freq_map: BTreeMap<u64, u64> = BTreeMap::new();
    let mut total_pairs = 0u64;

    // 3. Stream record batches from Parquet
    while let Some(Ok(batch)) = reader.next() {
        let column = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Expected UInt64Array in first column");

        for &p in column.values() {
            window.push_back(p);

            // Once we have k + 1 primes, calculate difference between front and back
            if window.len() == k + 1 {
                let p_n = window.pop_front().unwrap();
                let diff = p - p_n;
                *freq_map.entry(diff).or_insert(0) += 1;
                total_pairs += 1;
            }
        }
    }

    let duration = start_time.elapsed();

    // 4. Sort results by frequency (descending)
    let mut sorted_freq: Vec<(u64, u64)> = freq_map.into_iter().collect();
    sorted_freq.sort_by(|a, b| b.1.cmp(&a.1));

    // 5. Display top 20 differences
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