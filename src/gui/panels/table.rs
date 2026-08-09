// ============================================================================
// Virtualized Table View — egui_extras::TableBuilder Row Rendering
// ============================================================================

use egui_extras::{Column, TableBuilder};
use crate::gui::state::{AppState, TableRow};

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label("🔍 Filter gap =");
        ui.text_edit_singleline(&mut state.gap_filter);
        ui.label(format!("Loaded Rows: {}", state.table_rows.len()));
    });

    let filtered: Vec<&TableRow> = if state.gap_filter.trim().is_empty() {
        state.table_rows.iter().collect()
    } else if let Ok(target) = state.gap_filter.trim().parse::<u16>() {
        state.table_rows.iter().filter(|r| r.gap == target).collect()
    } else {
        state.table_rows.iter().collect()
    };

    let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 4.0;

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::auto().at_least(100.0).resizable(true))
        .column(Column::auto().at_least(100.0).resizable(true))
        .header(text_height + 4.0, |mut header| {
            header.col(|ui| {
                ui.strong("Index n");
            });
            header.col(|ui| {
                ui.strong(format!("Gap Δ_{}", state.k));
            });
        })
        .body(|body| {
            body.rows(text_height, filtered.len(), |mut row| {
                let r = filtered[row.index()];
                row.col(|ui| {
                    ui.label(r.n.to_string());
                });
                row.col(|ui| {
                    ui.label(r.gap.to_string());
                });
            });
        });
}
