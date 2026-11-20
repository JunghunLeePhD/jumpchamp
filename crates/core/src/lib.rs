//! JumpChamp Core — high-performance prime gap computation engine.
//!
//! Modules:
//! - `sieve`: Bitpacked wheel-of-30 sieve, parallel range sieving, chunk streaming.
//! - `analysis`: Pure gap combinators, transition matrices, residue counts, and text reports.
//! - `engine`: Gap histograms, chunk caching, range aggregation, and animation frames.
//! - `storage`: Zero-copy memory-mapped raw gaps, block histogram indexing, and spectral heatmaps.

pub mod analysis;
pub mod cancel;
pub mod engine;
pub mod sieve;
pub mod storage;

pub use analysis::{count_residues, gap_transition_matrix, k_step_gaps, record_gaps};
pub use cancel::CancelToken;
pub use engine::{
    frame_step, range_histogram, top_gaps, ChunkCache, FrameSet, Histogram, Progress,
};
pub use sieve::{
    nth_prime_upper_bound, prime_blocks_up_to, prime_blocks_up_to_cancellable,
    sieve_range_parallel, sieve_range_parallel_cancellable, sieve_segment,
    sieve_segment_cancellable, small_primes, small_primes_cancellable, stream_prime_blocks_range,
    stream_prime_blocks_range_cancellable,
};
pub use storage::{
    build_block_index, generate_gaps_file, BlockHistogram, DistributionEngine, RawGaps,
    SpectralHeatmap, StorageError, BLOCK_SIZE, HISTOGRAM_BINS, MAX_DATASET_PRIMES,
};
