// ============================================================================
// Worker Module — Background Processing Engine & Channels
// ============================================================================

pub mod dispatch;
pub mod engine;

pub use dispatch::spawn_worker;
