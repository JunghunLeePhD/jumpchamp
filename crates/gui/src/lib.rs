//! JumpChamp GUI — native desktop application for prime gap exploration.

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
pub use crate as gui;
