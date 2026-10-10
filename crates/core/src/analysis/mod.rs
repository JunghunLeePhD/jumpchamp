// ============================================================================
// Analysis Layer — re-exports gaps and report modules
// ============================================================================

pub mod gaps;
pub mod report;

pub use gaps::{
    apply_interval, apply_offset_interval, count_frequencies, count_residues,
    gap_transition_matrix, k_step_gaps, k_step_gaps_from_gaps, record_gaps, RecordGap,
};
pub use report::{format_record_gaps_report, format_report, format_residue_report};
