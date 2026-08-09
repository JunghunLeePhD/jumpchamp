#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// ============================================================================
// JumpChamp GUI Binary Entry Point
// ============================================================================

fn main() -> eframe::Result<()> {
    jumpchamp::gui::app::run()
}

