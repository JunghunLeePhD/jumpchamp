// ============================================================================
// Storage Layer — re-exports Parquet I/O modules
// ============================================================================

pub mod gaps2_parquet;
pub mod gaps_parquet;
pub mod parquet;

pub use gaps2_parquet::GapsSink2;
pub use gaps_parquet::GapsSink;
pub use parquet::{copy_existing_parquet, get_existing_max_prime, ParquetPrimeSink};
