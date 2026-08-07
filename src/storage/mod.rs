// ============================================================================
// Storage Layer — re-exports Parquet I/O module
// ============================================================================

pub mod parquet;

pub use parquet::{copy_existing_parquet, get_existing_max_prime, ParquetPrimeSink};
