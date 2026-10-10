//! JumpChamp Core — high-performance prime gap computation engine.
//!
//! Modules:
//! - `sieve`: Bitpacked wheel-of-30 sieve, parallel range sieving, chunk streaming.
//! - `analysis`: Pure gap combinators, transition matrices, residue counts, and text reports.
//! - `engine`: Gap histograms, chunk caching, range aggregation, and animation frames.

pub mod analysis;
pub mod engine;
pub mod sieve;

pub use analysis::{count_residues, gap_transition_matrix, k_step_gaps, record_gaps};
pub use engine::{
    frame_step, range_histogram, top_gaps, ChunkCache, FrameSet, Histogram, Progress,
};
pub use sieve::{
    nth_prime_upper_bound, prime_blocks_up_to, sieve_range_parallel, sieve_segment, small_primes,
    stream_prime_blocks_range,
};
