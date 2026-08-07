use arrow_array::{RecordBatch, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, Encoding};
use parquet::file::properties::WriterProperties;
use rayon::prelude::*;
use std::env;
use std::fs::File;
use std::sync::Arc;
use std::time::Instant;

fn find_small_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }
    let mut is_prime = vec![true; limit + 1];
    is_prime[0] = false;
    is_prime[1] = false;

    let mut p = 2;
    while p * p <= limit {
        if is_prime[p] {
            let mut i = p * p;
            while i <= limit {
                is_prime[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    (2..=limit).filter(|&x| is_prime[x]).collect()
}

/// Sieve a specific range [range_start, range_end] in parallel with Rayon
fn sieve_range(range_start: usize, range_end: usize, small_primes: &[usize]) -> Vec<usize> {
    let segment_size = 32_768; // 32 KB aligns with CPU L1 cache
    let num_segments = (range_end - range_start) / segment_size + 1;

    (0..num_segments)
        .into_par_iter()
        .flat_map(|seg_idx| {
            let seg_low = range_start + seg_idx * segment_size;
            let seg_high = (seg_low + segment_size - 1).min(range_end);

            if seg_low > seg_high {
                return vec![];
            }

            let range_len = seg_high - seg_low + 1;
            let mut is_prime = vec![true; range_len];

            for &p in small_primes {
                let mut start = (seg_low + p - 1) / p * p;
                if start == p {
                    start += p;
                }

                let mut i = start;
                while i <= seg_high {
                    is_prime[i - seg_low] = false;
                    i += p;
                }
            }

            (0..range_len)
                .filter_map(|i| if is_prime[i] { Some(seg_low + i) } else { None })
                .collect::<Vec<usize>>()
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let limit: usize = if args.len() > 1 {
        args[1].parse().unwrap_or(10_000_000)
    } else {
        10_000_000
    };

    let output_path = "primes.parquet";
    println!("Generating primes up to {} -> {}", limit, output_path);
    let start_time = Instant::now();

    // 1. Prepare Arrow Schema (Single UInt64 column 'prime')
    let schema = Arc::new(Schema::new(vec![Field::new(
        "prime",
        DataType::UInt64,
        false,
    )]));

    // 2. Configure Parquet Properties: Delta Encoding + ZSTD Compression
    let writer_props = WriterProperties::builder()
        .set_column_encoding("prime".into(), Encoding::DELTA_BINARY_PACKED)
        .set_compression(Compression::ZSTD(Default::default()))
        .build();

    let file = File::create(output_path)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(writer_props))?;

    // 3. Precompute base small primes
    let sqrt_limit = (limit as f64).sqrt() as usize;
    let small_primes = find_small_primes(sqrt_limit);

    let small_u64: Vec<u64> = small_primes.iter().map(|&p| p as u64).collect();
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(UInt64Array::from(small_u64))],
    )?;
    writer.write(&batch)?;

    let mut total_primes = small_primes.len();

    // 4. Stream rest of range in 10,000,000 blocks to bound memory
    let block_size = 10_000_000;
    let mut current_start = sqrt_limit + 1;

    while current_start <= limit {
        let current_end = (current_start + block_size - 1).min(limit);

        let chunk_primes = sieve_range(current_start, current_end, &small_primes);
        total_primes += chunk_primes.len();

        let chunk_u64: Vec<u64> = chunk_primes.into_iter().map(|p| p as u64).collect();
        let chunk_array = Arc::new(UInt64Array::from(chunk_u64));
        let batch = RecordBatch::try_new(schema.clone(), vec![chunk_array])?;

        writer.write(&batch)?;

        current_start = current_end + 1;
    }

    // 5. Finalize Parquet File
    writer.close()?;

    let duration = start_time.elapsed();
    let file_size_bytes = std::fs::metadata(output_path)?.len();
    let bytes_per_prime = file_size_bytes as f64 / total_primes as f64;

    println!("\n----------------------------------------");
    println!("Total Primes:      {}", total_primes);
    println!("Time Elapsed:      {:.2?}", duration);
    println!("Parquet File Size: {:.2} MB", file_size_bytes as f64 / 1_048_576.0);
    println!("Compression Ratio: {:.2} bytes/prime", bytes_per_prime);
    println!("----------------------------------------");

    Ok(())
}