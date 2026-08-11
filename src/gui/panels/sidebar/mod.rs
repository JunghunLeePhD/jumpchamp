// ============================================================================
// Top Control Bar — User Controls & Single-Track Dual-Thumb Range Slider
// ============================================================================

pub mod anim_toolbar;
pub mod dual_slider;
pub mod main_toolbar;

use crate::gui::state::{AppState, ViewMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    None,
    Compute,
    Cancel,
    StartAnimation,
    StartReverseAnimation,
    StepAnimation,
    StepBackAnimation,
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let main_action = main_toolbar::render_main_toolbar(ui, state);

    if state.view_mode == ViewMode::Animation {
        let anim_action = anim_toolbar::render_anim_toolbar(ui, state);
        match main_action {
            SidebarAction::None => anim_action,
            action => action,
        }
    } else {
        main_action
    }
}
