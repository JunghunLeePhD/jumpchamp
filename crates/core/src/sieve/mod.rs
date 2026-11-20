//! Prime generation: wheel-30 bit-packed segmented sieve, parallel and streaming.
//!
//! Pure; depends only on `rayon`.

pub mod parallel;
pub mod stream;
pub mod wheel;

pub use parallel::{sieve_range_parallel, sieve_range_parallel_cancellable};
pub use stream::{
    nth_prime_upper_bound, prime_blocks_up_to, prime_blocks_up_to_cancellable,
    stream_prime_blocks_range, stream_prime_blocks_range_cancellable,
};
pub use wheel::{sieve_segment, sieve_segment_cancellable, small_primes, small_primes_cancellable};
