// ============================================================================
// Analysis Layer — re-exports gaps and report modules
// ============================================================================

pub mod gaps;
pub mod report;

pub use gaps::{apply_interval, count_frequencies, k_step_gaps, stream_primes};
pub use report::format_report;
