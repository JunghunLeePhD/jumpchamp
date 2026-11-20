//! Zero-copy binary storage and two-tier indexing for prime gaps up to 10¹².
//!
//! Tier 1 (`gaps.bin`): Memory-mapped `u16` 1-step gaps (`RawGaps`) via `memmap2` + `bytemuck`.
//! Tier 2 (`block_index.bin`): Memory-mapped array of 2.4 KB `BlockHistogram` records (`DistributionEngine`).

pub mod builder;
pub mod engine;
pub mod format;
pub mod raw;
pub mod spectral;

pub use builder::{build_block_index, generate_gaps_file, GapsBuildSummary, IndexBuildSummary};
pub use engine::DistributionEngine;
pub use format::{BlockHistogram, StorageError, BLOCK_SIZE, HISTOGRAM_BINS, MAX_DATASET_PRIMES};
pub use raw::RawGaps;
pub use spectral::SpectralHeatmap;
