//! JumpChamp GUI — native desktop application for prime gap exploration.

/// Application version resolved at compile time from the active git tag (or crate version fallback).
pub const APP_VERSION: &str = env!("JUMPCHAMP_APP_VERSION");

pub mod actions;
pub mod app;
pub mod format;
pub mod panels;
pub mod playback;
pub mod prefs;
pub mod state;
pub mod theme;
pub mod widgets;
pub mod worker;

pub use app::run;
