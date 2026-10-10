//! Prime generation: wheel-30 bit-packed segmented sieve, parallel and streaming.
//!
//! Pure; depends only on `rayon`.

pub mod parallel;
pub mod stream;
pub mod wheel;

pub use parallel::sieve_range_parallel;
pub use stream::{nth_prime_upper_bound, prime_blocks_up_to, stream_prime_blocks_range};
pub use wheel::{sieve_segment, small_primes};
