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

/// Applies lazy bounds checking on prime values: stops reading when `p > max`, skips `p < min`.
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
// Advanced Mathematical Analytics — Record Gaps, Residues, & Transitions
// ============================================================================

/// Represents a record-breaking prime gap and its Cramér Ratio.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordGap {
    pub prime_index: u64,  // 1-based index n of prime p_n
    pub prime: u64,        // Prime value p_n
    pub gap: u64,          // Record gap size Δ(n) = p_{n+1} - p_n
    pub cramer_ratio: f64, // C(n) = Δ(n) / (ln p_n)^2
}

/// Identifies record-breaking maximal prime gaps from a stream of (prime_index, prime_val, gap_size).
pub fn record_gaps(gaps: impl Iterator<Item = (u64, u64, u64)>) -> Vec<RecordGap> {
    let mut max_gap = 0u64;
    let mut records = Vec::new();

    for (n, prime, gap) in gaps {
        if gap > max_gap {
            max_gap = gap;
            let ln_p = (prime as f64).ln();
            let cramer_ratio = if ln_p > 0.0 { gap as f64 / (ln_p * ln_p) } else { 0.0 };
            records.push(RecordGap {
                prime_index: n,
                prime,
                gap,
                cramer_ratio,
            });
        }
    }
    records
}

/// Computes the residue class frequencies of prime gaps (`gap % modulus`).
pub fn count_residues(stream: impl Iterator<Item = u64>, modulus: u64) -> BTreeMap<u64, u64> {
    stream.fold(BTreeMap::new(), |mut acc, val| {
        *acc.entry(val % modulus).or_insert(0) += 1;
        acc
    })
}

/// Computes consecutive prime gap 2-step Markov transition counts `(g_n -> g_{n+1})`.
pub fn gap_transition_matrix(gaps: impl Iterator<Item = u64>) -> BTreeMap<(u64, u64), u64> {
    let mut transitions = BTreeMap::new();
    let mut prev_gap: Option<u64> = None;

    for g in gaps {
        if let Some(prev) = prev_gap {
            *transitions.entry((prev, g)).or_insert(0) += 1;
        }
        prev_gap = Some(g);
    }
    transitions
}

// ============================================================================
// Fast path — operates on single-column (gap: u16) from gaps.parquet
// ============================================================================

/// Converts a single-column gaps.parquet reader into a lazy stream of 16-bit gap values.
pub fn stream_gaps(reader: ParquetRecordBatchReader) -> impl Iterator<Item = u16> {
    reader.filter_map(Result::ok).flat_map(|batch| {
        let gaps = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .expect("Expected UInt16Array for gap column")
            .clone();

        (0..gaps.len()).map(move |i| gaps.value(i)).collect::<Vec<_>>().into_iter()
    })
}

/// Applies 1-based index range bounds [min_idx, max_idx] using iterator skip and take.
pub fn apply_offset_interval(
    gaps: impl Iterator<Item = u16>,
    min_idx: u64,
    max_idx: u64,
) -> impl Iterator<Item = u16> {
    let skip_count = min_idx.saturating_sub(1) as usize;
    let take_count = if max_idx >= min_idx {
        (max_idx - min_idx + 1) as usize
    } else {
        0
    };

    gaps.skip(skip_count).take(take_count)
}

/// Computes k-step gaps from a stream of 1-step `u16` gaps via a sliding sum.
///
/// Mathematical identity: Δ_k(n) = Δ_1(n) + Δ_1(n+1) + ... + Δ_1(n+k−1)
pub fn k_step_gaps_from_gaps(
    gaps: impl Iterator<Item = u16>,
    k: usize,
) -> impl Iterator<Item = u64> {
    let mut window: VecDeque<u64> = VecDeque::with_capacity(k);
    let mut window_sum: u64 = 0;

    gaps.filter_map(move |gap| {
        let g = gap as u64;
        window.push_back(g);
        window_sum += g;

        if window.len() == k {
            let result = window_sum;
            window_sum -= window.pop_front().unwrap();
            Some(result)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k_step_gaps_k1_and_k2() {
        let primes = vec![2, 3, 5, 7, 11, 13];
        let gaps1: Vec<u64> = k_step_gaps(primes.clone().into_iter(), 1).collect();
        assert_eq!(gaps1, vec![1, 2, 2, 4, 2]);

        let gaps2: Vec<u64> = k_step_gaps(primes.into_iter(), 2).collect();
        assert_eq!(gaps2, vec![3, 4, 6, 6]);
    }

    #[test]
    fn test_k_step_gaps_from_1step_gaps_identity() {
        let step1_gaps = vec![1u16, 2, 2, 4, 2];
        let gaps2: Vec<u64> = k_step_gaps_from_gaps(step1_gaps.into_iter(), 2).collect();

        // 1+2=3, 2+2=4, 2+4=6, 4+2=6
        assert_eq!(gaps2, vec![3, 4, 6, 6]);
    }

    #[test]
    fn test_record_gaps_and_cramer_ratio() {
        // Primes: (1, 2), (2, 3), (3, 5), (4, 7), (5, 11), (6, 13), (7, 17), (8, 19), (9, 23), (10, 29), (11, 31)
        // Gaps: (1, 2, 1), (2, 3, 2), (3, 5, 2), (4, 7, 4), (5, 11, 2), (6, 13, 4), (7, 17, 2), (8, 19, 4), (9, 23, 6)
        let gap_data = vec![
            (1, 2, 1),
            (2, 3, 2),
            (3, 5, 2),
            (4, 7, 4),
            (5, 11, 2),
            (9, 23, 6),
        ];

        let records = record_gaps(gap_data.into_iter());
        assert_eq!(records.len(), 4); // record gaps: 1 (p=2), 2 (p=3), 4 (p=7), 6 (p=23)
        assert_eq!(records[0].gap, 1);
        assert_eq!(records[1].gap, 2);
        assert_eq!(records[2].gap, 4);
        assert_eq!(records[3].gap, 6);
        assert!(records[3].cramer_ratio > 0.0);
    }

    #[test]
    fn test_count_residues() {
        let gaps = vec![2u64, 4, 6, 2, 6, 8, 12];
        let residues = count_residues(gaps.into_iter(), 6);

        // 2 % 6 = 2 (cnt 2)
        // 4 % 6 = 4 (cnt 1)
        // 6 % 6 = 0 (cnt 2)
        // 8 % 6 = 2 -> cnt 2+1=3
        // 12 % 6 = 0 -> cnt 2+1=3
        assert_eq!(residues.get(&0), Some(&3));
        assert_eq!(residues.get(&2), Some(&3));
        assert_eq!(residues.get(&4), Some(&1));
    }

    #[test]
    fn test_gap_transition_matrix() {
        let gaps = vec![2u64, 4, 2, 4, 6, 2];
        let trans = gap_transition_matrix(gaps.into_iter());

        // Transitions: (2->4): 2, (4->2): 1, (4->6): 1, (6->2): 1
        assert_eq!(trans.get(&(2, 4)), Some(&2));
        assert_eq!(trans.get(&(4, 2)), Some(&1));
        assert_eq!(trans.get(&(4, 6)), Some(&1));
        assert_eq!(trans.get(&(6, 2)), Some(&1));
    }

    #[test]
    fn test_apply_interval() {
        let primes = vec![2, 3, 5, 7, 11, 13, 17, 19];
        let filtered: Vec<u64> = apply_interval(primes.into_iter(), 5, 13).collect();
        assert_eq!(filtered, vec![5, 7, 11, 13]);
    }

    #[test]
    fn test_apply_offset_interval() {
        let gaps = vec![1u16, 2, 2, 4, 2];
        let sliced: Vec<u16> = apply_offset_interval(gaps.into_iter(), 2, 4).collect();
        assert_eq!(sliced, vec![2, 2, 4]);
    }

    #[test]
    fn test_count_frequencies() {
        let stream = vec![2, 4, 2, 6, 2, 4].into_iter();
        let freq = count_frequencies(stream);

        assert_eq!(freq.get(&2), Some(&3));
        assert_eq!(freq.get(&4), Some(&2));
        assert_eq!(freq.get(&6), Some(&1));
        assert_eq!(freq.get(&8), None);
    }
}


