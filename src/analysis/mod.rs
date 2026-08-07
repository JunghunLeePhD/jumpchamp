// ============================================================================
// Analysis Layer — re-exports gaps and report modules
// ============================================================================

pub mod gaps;
pub mod report;

pub use gaps::{
    // Slow path (primes.parquet)
    apply_interval, count_frequencies, k_step_gaps, stream_primes,
    // Fast path (single-column gaps.parquet)
    apply_offset_interval, k_step_gaps_from_gaps, stream_gaps,
};
pub use report::format_report;
