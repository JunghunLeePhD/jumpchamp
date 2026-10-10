//! Prime-gap computation engine: histograms, a chunk cache and animation frames.
//!
//! Pure and GUI-free — composes `sieve` (prime streams) with `analysis` (gap combinators).
//! Long computations take an `on_block` callback for progress reporting and cancellation.

pub mod frames;
pub mod histogram;
pub mod range;
pub mod scan;

pub use frames::{frame_step, FrameSet};
pub use histogram::{top_gaps, Histogram, MAX_GAP};
pub use range::{range_histogram, ChunkCache};
pub use scan::{scan_k_gaps, Progress};
