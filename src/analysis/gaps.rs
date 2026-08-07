// ============================================================================
// Pure Functional Iterator Combinators for Prime Gap Analysis
// ============================================================================

use arrow_array::UInt64Array;
use parquet::arrow::arrow_reader::ParquetRecordBatchReader;
use std::collections::{BTreeMap, VecDeque};

/// Converts a Parquet reader into a lazy stream of prime numbers (`u64`).
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

/// Applies lazy bounds checking: stops reading when `p > max`, skips `p < min`.
pub fn apply_interval(
    primes: impl Iterator<Item = u64>,
    min: u64,
    max: u64,
) -> impl Iterator<Item = u64> {
    primes
        .take_while(move |&p| p <= max) // Early exit when p > max_prime
        .filter(move |&p| p >= min)     // Lower bound filter
}

/// Transforms a sequence of primes into a sequence of k-step gaps (`p_{n+k} - p_n`).
pub fn k_step_gaps(primes: impl Iterator<Item = u64>, k: usize) -> impl Iterator<Item = u64> {
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

/// Folds a stream of numbers into a sorted frequency map (`gap → count`).
pub fn count_frequencies(stream: impl Iterator<Item = u64>) -> BTreeMap<u64, u64> {
    stream.fold(BTreeMap::new(), |mut acc, val| {
        *acc.entry(val).or_insert(0) += 1;
        acc
    })
}
