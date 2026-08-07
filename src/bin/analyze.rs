use arrow_array::UInt64Array;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs::File;
use std::time::Instant;

// ============================================================================
// 1. Domain Configuration & Input Parsing
// ============================================================================

#[derive(Debug, Clone)]
pub struct Config {
    pub k: usize,
    pub min_prime: u64,
    pub max_prime: u64,
    pub file_path: String,
}

impl Config {
    pub fn from_args(args: &[String]) -> Self {
        Self {
            k: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2),
            min_prime: args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
            max_prime: args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX),
            file_path: args.get(4).cloned().unwrap_or_else(|| "primes.parquet".into()),
        }
    }
}

// ============================================================================
// 2. Pure Functional Iterator Combinators
// ============================================================================

/// Converts a Parquet reader into a lazy stream of prime numbers (u64).
pub fn stream_primes(reader: ParquetRecordBatchReader) -> impl Iterator<Item = u64> {
    reader.filter_map(Result::ok).flat_map(|batch| {
        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Expected UInt64Array")
            .clone();

        (0..col.len()).map(move |i| col.value(i)).collect::<Vec<_>>().into_iter()
    })
}

/// Applies lazy bounds checking: stops reading when p > max, skips p < min.
pub fn apply_interval(
    primes: impl Iterator<Item = u64>,
    min: u64,
    max: u64,
) -> impl Iterator<Item = u64> {
    primes
        .take_while(move |&p| p <= max) // Early exit when p > max_prime
        .filter(move |&p| p >= min)     // Lower bound filter
}

/// Transforms a sequence of primes into a sequence of k-step gaps (p_{n+k} - p_n).
pub fn k_step_gaps(
    primes: impl Iterator<Item = u64>,
    k: usize,
) -> impl Iterator<Item = u64> {
    let mut window = VecDeque::with_capacity(k + 1);

    primes.filter_map(move |p| {
        window.push_back(p);
        if window.len() == k + 1 {
            let p_n = window.pop_front().unwrap();
            Some(p - p_n)
        } else {
            None
        }
    })
}

/// Folds a stream of numbers into a frequency map (u64 -> count).
pub fn count_frequencies(stream: impl Iterator<Item = u64>) -> BTreeMap<u64, u64> {
    stream.fold(BTreeMap::new(), |mut acc, val| {
        *acc.entry(val).or_insert(0) += 1;
        acc
    })
}

// ============================================================================
// 3. Pure Reporting & Formatting
// ============================================================================

pub fn format_report(freq_map: &BTreeMap<u64, u64>, top_n: usize) -> String {
    let total_pairs: u64 = freq_map.values().sum();
    let mut sorted: Vec<_> = freq_map.iter().map(|(&diff, &count)| (diff, count)).collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = String::new();
    out.push_str(&format!("{:<12} {:<15} {:<12}\n", "Diff", "Frequency", "Percentage"));
    out.push_str(&format!("{}\n", "-".repeat(42)));

    for (diff, count) in sorted.into_iter().take(top_n) {
        let pct = (count as f64 / total_pairs as f64) * 100.0;
        out.push_str(&format!("{:<12} {:<15} {:.2}%\n", diff, count, pct));
    }

    out.push_str(&format!("{}\n", "-".repeat(42)));
    out.push_str(&format!("Total Analyzed Pairs: {}\n", total_pairs));
    out
}

// ============================================================================
// 4. Main Entry Point (Impure Shell)
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = Config::from_args(&args);

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

    // Declarative FP Pipeline
    let frequencies = count_frequencies(
        k_step_gaps(
            apply_interval(
                stream_primes(reader),
                config.min_prime,
                config.max_prime,
            ),
            config.k,
        )
    );

    let duration = start_time.elapsed();

    // Print output
    print!("{}", format_report(&frequencies, 20));
    println!("Time Elapsed: {:.2?}\n", duration);

    Ok(())
}