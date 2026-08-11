use crossbeam_channel::unbounded;
use eframe::App;

use crate::gui::panels::{chart, settings, sidebar, sidebar::SidebarAction};
use crate::gui::state::{AppState, WorkerCommand, WorkerResult};
use crate::gui::theme::apply_theme;
use crate::gui::worker::spawn_worker;

pub struct JumpChampApp {
    pub state: AppState,
}

impl JumpChampApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (res_tx, res_rx) = unbounded();

        spawn_worker(cmd_rx, res_tx, cc.egui_ctx.clone());

        let app = Self {
            state: AppState::new(cmd_tx, res_rx),
        };
        apply_theme(&cc.egui_ctx, app.state.theme_mode);
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
            top_min: self.state.top_min,
            top_max: self.state.top_max,
            sort_by: self.state.sort_by.clone(),
        };
        self.state.cmd_tx.send(cmd).ok();
    }

    fn dispatch_anim_frame(&mut self) {
        self.state.is_loading = true;
        self.state.progress = 0.0;
        self.state.error_msg = None;

        let min_range = self.state.min_val;
        let max_range = self.state.anim_current_val.max(self.state.min_val);

        let cmd = WorkerCommand::ComputeGaps {
            min_val: min_range,
            max_val: max_range,
            k: self.state.k,
            top_min: self.state.top_min,
            top_max: self.state.top_max,
            sort_by: self.state.sort_by.clone(),
        };
        self.state.cmd_tx.send(cmd).ok();
    }

    fn dispatch_step_animation(&mut self) {
        if self.state.anim_current_val >= self.state.max_val {
            self.state.anim_current_val = self.state.min_val;
        } else {
            self.state.anim_current_val =
                (self.state.anim_current_val + self.state.anim_step_size).min(self.state.max_val);
        }
        self.dispatch_anim_frame();
    }

    fn dispatch_cancel(&mut self) {
        self.state.cmd_tx.send(WorkerCommand::Cancel).ok();
        self.state.is_loading = false;
        self.state.is_animating = false;
    }
}

impl App for JumpChampApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.state.theme_mode);

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
                    self.state.is_animating = false;
                }
            }
        }

        // Animation Timer Tick
        if self.state.is_animating && !self.state.is_loading {
            let now = std::time::Instant::now();
            let fps = self.state.anim_speed_fps.max(1.0);
            let target_delay = std::time::Duration::from_secs_f32(1.0 / fps);

            let should_step = match self.state.last_frame_instant {
                Some(last) => now.duration_since(last) >= target_delay,
                None => true,
            };

            if should_step {
                self.state.last_frame_instant = Some(now);
                if self.state.anim_current_val >= self.state.max_val {
                    self.state.is_animating = false;
                } else {
                    self.state.anim_current_val =
                        (self.state.anim_current_val + self.state.anim_step_size).min(self.state.max_val);
                    self.dispatch_anim_frame();
                }
            }
            ctx.request_repaint_after(target_delay);
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("⚙ Engine: In-Memory Parallel Segmented Sieve");
                ui.separator();
                if self.state.is_animating {
                    ui.label(format!(
                        "🎬 ANIMATING: {} ~ {} (Bound: {})",
                        sidebar::format_compact_num(self.state.min_val),
                        sidebar::format_compact_num(self.state.max_val),
                        sidebar::format_compact_num(self.state.anim_current_val)
                    ));
                } else {
                    ui.label(format!(
                        "📊 Range: {} ~ {} (k={}, Rank={}~{})",
                        sidebar::format_compact_num(self.state.min_val),
                        sidebar::format_compact_num(self.state.max_val),
                        self.state.k,
                        self.state.top_min,
                        self.state.top_max
                    ));
                }
                ui.separator();
                let latency_str = self
                    .state
                    .query_latency_ms
                    .map(|ms| format!("{:.1} ms", ms))
                    .unwrap_or_else(|| "-- ms".to_string());
                ui.label(format!("⚡ Latency: {}", latency_str));
            });
        });

        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| match sidebar::render(ui, &mut self.state) {
                SidebarAction::Compute => self.dispatch_compute(),
                SidebarAction::Cancel => self.dispatch_cancel(),
                SidebarAction::StepAnimation => self.dispatch_step_animation(),
                SidebarAction::None => {}
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            chart::render(ui, &self.state);
        });

        settings::render(ctx, &mut self.state);

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
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("JumpChamp — Prime Gap Explorer 🦀")
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([900.0, 600.0]);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/128x128.png")) {
        viewport = viewport.with_icon(icon);
    }

    let opts = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "JumpChamp",
        opts,
        Box::new(|cc| Ok(Box::new(JumpChampApp::new(cc)))),
    )
}

