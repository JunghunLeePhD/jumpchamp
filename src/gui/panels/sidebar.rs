// ============================================================================
// Top Control Bar — User Controls & Data Selection
// ============================================================================

use crate::gui::state::{AppState, SortOrder};

pub enum SidebarAction {
    None,
    Load,
    Cancel,
    OpenFilePicker,
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        // Group 1: Dataset & k-step
        ui.label("📂 File:");
        ui.add(egui::TextEdit::singleline(&mut state.file_path).desired_width(180.0));
        if ui.button("…").on_hover_text("Browse for .parquet file").clicked() {
            action = SidebarAction::OpenFilePicker;
        }

        ui.separator();
        ui.label("k =");
        ui.add(egui::DragValue::new(&mut state.k).range(1..=20));

        ui.separator();
        // Group 2: Index Range
        ui.label("min n:");
        ui.add(egui::DragValue::new(&mut state.min_idx).speed(100_000));
        ui.label("max n:");
        ui.add(egui::DragValue::new(&mut state.max_idx).speed(100_000));

        ui.separator();
        // Group 3: Display Settings
        ui.add(egui::Slider::new(&mut state.top_n, 5..=200).text("Top N"));
        ui.radio_value(&mut state.sort_by, SortOrder::ByFrequency, "Freq");
        ui.radio_value(&mut state.sort_by, SortOrder::ByGapSize, "Gap");

        ui.separator();
        // Action Button / Progress
        if state.is_loading {
            ui.add(egui::ProgressBar::new(state.progress).show_percentage());
            if ui.button("⬛ Cancel").clicked() {
                action = SidebarAction::Cancel;
            }
        } else if ui.button("▶ Load / Query").clicked() {
            action = SidebarAction::Load;
        }
    });
    ui.add_space(4.0);

    action
}
