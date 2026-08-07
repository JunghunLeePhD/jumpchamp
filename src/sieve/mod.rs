// ============================================================================
// Sieve Layer — re-exports basic, parallel, and stream modules
// ============================================================================

pub mod basic;
pub mod parallel;
pub mod stream;

// Convenience re-exports so callers can write `primes::sieve::small_primes` etc.
pub use basic::{sieve_segment, small_primes};
pub use parallel::sieve_range_parallel;
pub use stream::stream_prime_blocks_range;
