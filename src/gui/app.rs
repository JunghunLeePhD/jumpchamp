// ============================================================================
// Main eframe App Implementation
// ============================================================================

use crossbeam_channel::unbounded;
use eframe::App;

use crate::gui::panels::{chart, sidebar, sidebar::SidebarAction};
use crate::gui::state::{AppState, WorkerCommand, WorkerResult};
use crate::gui::theme::apply_theme;
use crate::gui::worker::spawn_worker;

pub struct JumpChampApp {
    pub state: AppState,
}

impl JumpChampApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);

        let (cmd_tx, cmd_rx) = unbounded();
        let (res_tx, res_rx) = unbounded();

        spawn_worker(cmd_rx, res_tx, cc.egui_ctx.clone());

        let mut app = Self {
            state: AppState::new(cmd_tx, res_rx),
        };

        app.dispatch_compute();

        app
    }

    fn dispatch_compute(&mut self) {
        self.state.is_loading = true;
        self.state.progress = 0.0;
        self.state.error_msg = None;

        let cmd = WorkerCommand::ComputeGaps {
            min_val: self.state.min_val,
            max_val: self.state.max_val,
            k: self.state.k,
            top_n: self.state.top_n,
            sort_by: self.state.sort_by.clone(),
        };
        self.state.cmd_tx.send(cmd).ok();
    }

    fn dispatch_cancel(&mut self) {
        self.state.cmd_tx.send(WorkerCommand::Cancel).ok();
        self.state.is_loading = false;
    }
}

impl App for JumpChampApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(result) = self.state.res_rx.try_recv() {
            match result {
                WorkerResult::Metadata(m) => self.state.metadata = Some(m),
                WorkerResult::FrequencyData(f) => self.state.freq_data = f,
                WorkerResult::QueryLatency(ms) => self.state.query_latency_ms = Some(ms),

                WorkerResult::Progress(p) => {
                    self.state.progress = p;
                    if p >= 1.0 {
                        self.state.is_loading = false;
                    }
                }
                WorkerResult::Error(e) => {
                    self.state.error_msg = Some(e);
                    self.state.is_loading = false;
                }
            }
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("⚡ Engine: In-Memory Parallel Segmented Sieve");
                ui.separator();
                ui.label(format!(
                    "📊 Range: {} ~ {} (k={}, Top N={})",
                    sidebar::format_compact_num(self.state.min_val),
                    sidebar::format_compact_num(self.state.max_val),
                    self.state.k,
                    self.state.top_n
                ));
                ui.separator();
                let latency_str = self
                    .state
                    .query_latency_ms
                    .map(|ms| format!("{:.1} ms", ms))
                    .unwrap_or_else(|| "-- ms".to_string());
                ui.label(format!("⏱️ Query Latency: {}", latency_str));
            });
        });

        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| match sidebar::render(ui, &mut self.state) {
                SidebarAction::Compute => self.dispatch_compute(),
                SidebarAction::Cancel => self.dispatch_cancel(),
                SidebarAction::None => {}
            });


        egui::CentralPanel::default().show(ctx, |ui| {
            chart::render(ui, &self.state);
        });


        if let Some(err) = self.state.error_msg.clone() {
            egui::Window::new("Error").show(ctx, |ui| {
                ui.colored_label(egui::Color32::RED, &err);
                if ui.button("Dismiss").clicked() {
                    self.state.error_msg = None;
                }
            });
        }
    }
}


pub fn run() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("JumpChamp — Prime Gap Explorer 🦀")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "JumpChamp",
        opts,
        Box::new(|cc| Ok(Box::new(JumpChampApp::new(cc)))),
    )
}
