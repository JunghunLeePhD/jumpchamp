//! Cooperative cancellation token shared across worker and core engine tasks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cooperative, thread-safe cancellation token backed by an `Arc<AtomicBool>`.
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    /// Creates an uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Signals cancellation to all holders of this token.
    #[inline]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Returns `true` if cancellation has been requested.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_token_propagates_across_clones_and_threads() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!token.is_cancelled());
        assert!(!clone.is_cancelled());

        let handle = std::thread::spawn(move || {
            clone.cancel();
        });
        handle.join().unwrap();

        assert!(token.is_cancelled());
    }
}
