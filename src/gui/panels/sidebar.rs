// ============================================================================
// Sidebar Panel — User Controls & Data Selection
// ============================================================================

use crate::gui::state::{AppState, ChartMode, SortOrder};

pub enum SidebarAction {
    None,
    Load,
    Cancel,
    OpenFilePicker,
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.heading("📂 Dataset");
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut state.file_path);
        if ui.button("…").on_hover_text("Browse for .parquet file").clicked() {
            action = SidebarAction::OpenFilePicker;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Gap step size k:");
        ui.add(egui::DragValue::new(&mut state.k).range(1..=20).prefix("k = "));
    });

    ui.separator();

    ui.heading("🔢 Index Range");
    ui.horizontal(|ui| {
        ui.label("min n:");
        ui.add(egui::DragValue::new(&mut state.min_idx).speed(100_000));
    });
    ui.horizontal(|ui| {
        ui.label("max n:");
        ui.add(egui::DragValue::new(&mut state.max_idx).speed(100_000));
    });

    if let Some(meta) = &state.metadata {
        ui.label(format!("Total rows in DB: {}", meta.total_rows));
    }

    ui.separator();

    ui.heading("📊 Display");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.chart_mode, ChartMode::Histogram, "Histogram");
        ui.selectable_value(&mut state.chart_mode, ChartMode::Scatter, "Scatter");
    });
    ui.add(egui::Slider::new(&mut state.top_n, 5..=200).text("Top N gaps"));
    ui.label("Sort order:");
    ui.radio_value(&mut state.sort_by, SortOrder::ByFrequency, "Frequency");
    ui.radio_value(&mut state.sort_by, SortOrder::ByGapSize, "Gap Size");

    ui.separator();

    if state.is_loading {
        ui.add(egui::ProgressBar::new(state.progress).show_percentage());
        if ui.button("⬛ Cancel").clicked() {
            action = SidebarAction::Cancel;
        }
    } else if ui.button("▶ Load / Query").clicked() {
        action = SidebarAction::Load;
    }

    action
}
