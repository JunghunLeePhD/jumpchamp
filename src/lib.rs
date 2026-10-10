//! JumpChamp — prime gap explorer.
//!
//! Layers, each depending only on the ones above it:
//!
//! | Module     | Role                                             | Depends on          |
//! |------------|--------------------------------------------------|---------------------|
//! | `sieve`    | Prime generation (wheel-30, parallel, streaming) | `rayon`             |
//! | `analysis` | Pure gap combinators and text reports            | —                   |
//! | `engine`   | Gap histograms, chunk cache, animation frames    | `sieve`, `analysis` |
//! | `gui`      | egui desktop app (state, worker thread, panels)  | `engine`, `egui`    |

pub mod analysis;
pub mod engine;
pub mod gui;
pub mod sieve;
