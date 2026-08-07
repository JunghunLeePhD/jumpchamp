// ============================================================================
// Pure Functional Iterator Combinators for Prime Gap Analysis
// ============================================================================

use arrow_array::{UInt16Array, UInt64Array};
use parquet::arrow::arrow_reader::ParquetRecordBatchReader;
use std::collections::{BTreeMap, VecDeque};

// ============================================================================
// Slow path — operates on a raw prime stream from primes.parquet
// ============================================================================

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

/// Transforms a sequence of primes into a sequence of k-step gaps (`p_{n+k} − p_n`).
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

// ============================================================================
// Fast path — operates on pre-computed (prime, gap) pairs from gaps.parquet
// ============================================================================

/// Converts a gaps.parquet reader into a lazy stream of `(prime, Δ_1(n))` pairs.
pub fn stream_gap_pairs(reader: ParquetRecordBatchReader) -> impl Iterator<Item = (u64, u16)> {
    reader.filter_map(Result::ok).flat_map(|batch| {
        let primes = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Expected UInt64Array for prime column")
            .clone();
        let gaps = batch
            .column(1)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .expect("Expected UInt16Array for gap column")
            .clone();

        (0..primes.len())
            .map(move |i| (primes.value(i), gaps.value(i)))
            .collect::<Vec<_>>()
            .into_iter()
    })
}

/// Filters `(prime, gap)` pairs to `[min, max]` with early exit on the upper bound.
pub fn apply_gap_interval(
    pairs: impl Iterator<Item = (u64, u16)>,
    min: u64,
    max: u64,
) -> impl Iterator<Item = (u64, u16)> {
    pairs
        .take_while(move |&(p, _)| p <= max) // Early exit when prime > max
        .filter(move |&(p, _)| p >= min)     // Lower bound filter
}

/// Computes k-step gaps from a stream of 1-step `(prime, gap)` pairs via a sliding sum.
///
/// Mathematical identity: Δ_k(n) = Δ_1(n) + Δ_1(n+1) + ... + Δ_1(n+k−1)
///
/// Results are identical to `k_step_gaps` applied to the equivalent prime stream.
pub fn k_step_gaps_from_pairs(
    pairs: impl Iterator<Item = (u64, u16)>,
    k: usize,
) -> impl Iterator<Item = u64> {
    let mut window: VecDeque<u64> = VecDeque::with_capacity(k);
    let mut window_sum: u64 = 0;

    pairs.filter_map(move |(_, gap)| {
        let g = gap as u64;
        window.push_back(g);
        window_sum += g;

        if window.len() == k {
            // Window is full: emit the sum, then slide forward by removing the oldest gap
            let result = window_sum;
            window_sum -= window.pop_front().unwrap();
            Some(result)
        } else {
            None // Window not yet full
        }
    })
}
