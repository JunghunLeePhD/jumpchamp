use arrow_array::{RecordBatch, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use parquet::basic::{Compression, Encoding};
use parquet::file::properties::WriterProperties;
use rayon::prelude::*;
use std::env;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// 1. Domain Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct Config {
    pub limit: usize,
    pub output_path: String,
    pub block_size: usize,
}

impl Config {
    pub fn from_args(args: &[String]) -> Self {
        Self {
            limit: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000_000),
            output_path: "primes.parquet".into(),
            block_size: 10_000_000,
        }
    }
}

// ============================================================================
// 2. Pure Mathematical Sieves
// ============================================================================

/// Pure Function: Finds base prime numbers up to `limit` sequentially.
pub fn small_primes(limit: usize) -> Vec<usize> {
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

/// Pure Function: Sieves a single segment `[seg_low, seg_high]` using precomputed base primes.
pub fn sieve_segment(seg_low: usize, seg_high: usize, base_primes: &[usize]) -> Vec<u64> {
    if seg_low > seg_high {
        return vec![];
    }

    let range_len = seg_high - seg_low + 1;
    let mut is_prime = vec![true; range_len];

    for &p in base_primes {
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
        .filter_map(|i| if is_prime[i] { Some((seg_low + i) as u64) } else { None })
        .collect()
}

/// Sieves range `[start, end]` in parallel using Rayon across L1-cache-aligned 32KB segments.
pub fn sieve_range_parallel(start: usize, end: usize, base_primes: &[usize]) -> Vec<u64> {
    let segment_size = 32_768; // 32 KB aligned with CPU L1 cache
    let num_segments = (end - start) / segment_size + 1;

    (0..num_segments)
        .into_par_iter()
        .flat_map(|idx| {
            let seg_low = start + idx * segment_size;
            let seg_high = (seg_low + segment_size - 1).min(end);
            sieve_segment(seg_low, seg_high, base_primes)
        })
        .collect()
}

/// Returns a lazy iterator that yields sieved blocks for any arbitrary range [start_val, limit].
pub fn stream_prime_blocks_range(
    start_val: usize,
    limit: usize,
    block_size: usize,
    base_primes: &[usize],
) -> impl Iterator<Item = Vec<u64>> + '_ {
    (start_val..=limit)
        .step_by(block_size)
        .map(move |block_start| {
            let block_end = (block_start + block_size - 1).min(limit);
            sieve_range_parallel(block_start, block_end, base_primes)
        })
}

// ============================================================================
// 3. Parquet Metadata & I/O Helpers
// ============================================================================

/// Reads maximum prime value from existing Parquet metadata in O(1) time.
pub fn get_existing_max_prime(path: &str) -> Option<u64> {
    if !Path::new(path).exists() {
        return None;
    }

    let file = File::open(path).ok()?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
    let metadata = builder.metadata();

    // 1. Try reading statistics from row group metadata (O(1))
    let mut max_val: Option<u64> = None;
    for rg in metadata.row_groups() {
        if let Some(stats) = rg.column(0).statistics() {
            if let parquet::file::statistics::Statistics::Int64(s) = stats {
                if let Some(&max) = s.max_opt() {
                    let max_u64 = max as u64;
                    max_val = Some(max_val.map_or(max_u64, |m| m.max(max_u64)));
                }
            }
        }
    }

    if max_val.is_some() {
        return max_val;
    }

    // 2. Fallback: stream batches to read last value
    let mut reader = builder.build().ok()?;
    let mut last_p = None;
    while let Some(Ok(batch)) = reader.next() {
        let col = batch.column(0).as_any().downcast_ref::<UInt64Array>()?;
        if !col.is_empty() {
            last_p = Some(col.value(col.len() - 1));
        }
    }
    last_p
}

pub struct ParquetPrimeSink {
    schema: Arc<Schema>,
    writer: ArrowWriter<File>,
}

impl ParquetPrimeSink {
    pub fn create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "prime",
            DataType::UInt64,
            false,
        )]));

        let props = WriterProperties::builder()
            .set_column_encoding("prime".into(), Encoding::DELTA_BINARY_PACKED)
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let file = File::create(path)?;
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

        Ok(Self { schema, writer })
    }

    pub fn write_record_batch(&mut self, batch: &RecordBatch) -> Result<(), Box<dyn std::error::Error>> {
        self.writer.write(batch)?;
        Ok(())
    }

    pub fn write_batch(&mut self, primes: &[u64]) -> Result<(), Box<dyn std::error::Error>> {
        let array = Arc::new(UInt64Array::from(primes.to_vec()));
        let batch = RecordBatch::try_new(self.schema.clone(), vec![array])?;
        self.writer.write(&batch)?;
        Ok(())
    }

    pub fn finish(self) -> Result<(), Box<dyn std::error::Error>> {
        self.writer.close()?;
        Ok(())
    }
}

/// Copies existing batches from an old Parquet file into a new sink.
pub fn copy_existing_parquet(
    input_path: &str,
    sink: &mut ParquetPrimeSink,
) -> Result<usize, Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;
    let mut count = 0;

    for batch in reader {
        let batch = batch?;
        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or("Invalid schema in existing parquet file")?;

        count += col.len();
        sink.write_record_batch(&batch)?;
    }

    Ok(count)
}

// ============================================================================
// 4. Execution Pipeline
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = Config::from_args(&args);
    let tmp_path = format!("{}.tmp", config.output_path);

    let start_time = Instant::now();
    let existing_max = get_existing_max_prime(&config.output_path);

    // 1. Check if existing computation already covers target limit
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

    // 3. Compute and append new range
    if start_val <= config.limit {
        let sqrt_limit = (config.limit as f64).sqrt() as usize;
        let base_primes = small_primes(sqrt_limit);

        for block in stream_prime_blocks_range(start_val, config.limit, config.block_size, &base_primes) {
            total_primes += block.len();
            sink.write_batch(&block)?;
        }
    }

    // 4. Finalize Sink and Atomic Swap
    sink.finish()?;
    fs::rename(&tmp_path, &config.output_path)?;

    // 5. Output Summary
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