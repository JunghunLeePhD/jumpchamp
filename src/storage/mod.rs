// ============================================================================
// Storage Layer — re-exports Parquet I/O modules
// ============================================================================

pub mod gaps_parquet;
pub mod parquet;

pub use gaps_parquet::GapsSink;
pub use parquet::{copy_existing_parquet, get_existing_max_prime, ParquetPrimeSink};
