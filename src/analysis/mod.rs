// ============================================================================
// Analysis Layer — re-exports gaps and report modules
// ============================================================================

pub mod gaps;
pub mod report;

pub use gaps::{
    // Slow path (primes.parquet)
    apply_interval, count_frequencies, k_step_gaps, stream_primes,
    // Fast path (gaps.parquet)
    apply_gap_interval, k_step_gaps_from_pairs, stream_gap_pairs,
};
pub use report::format_report;
