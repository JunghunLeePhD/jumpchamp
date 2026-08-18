use crossbeam_channel::unbounded;
use eframe::App;

use crate::gui::animation::{
    advance_anim_backward, advance_anim_forward, dispatch_anim_frame, dispatch_cancel,
    dispatch_compute, dispatch_start_animation, dispatch_step_animation,
    dispatch_step_back_animation,
};
use crate::gui::panels::{chart, settings, sidebar, sidebar::SidebarAction, status_bar};
use crate::gui::state::{AppState, PlayDirection, WorkerResult};
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

    fn handle_worker_results(&mut self) {
        while let Ok(result) = self.state.res_rx.try_recv() {
            match result {
                WorkerResult::Metadata(m) => self.state.metadata = Some(m),
                WorkerResult::FrequencyData(f) => {
                    self.state.update_freq_data(f);
                    self.state.is_frame_in_flight = false;
                }
                WorkerResult::PrecomputedAnimation(anim_data) => {
                    self.state.anim_precomputed = Some(anim_data);
                    self.state.update_freq_from_precomputed();
                }
                WorkerResult::QueryLatency(ms) => self.state.query_latency_ms = Some(ms),

                WorkerResult::Progress {
                    progress,
                    current_block,
                    total_blocks,
                } => {
                    self.state.progress = progress;
                    self.state.current_block = current_block;
                    self.state.total_blocks = total_blocks;
                    if progress >= 1.0 {
                        self.state.is_loading = false;
                        if !self.state.freq_data.is_empty() {
                            self.state.top_min = 1;
                            self.state.top_max = self.state.freq_data.len().max(1);
                        }
                        if self.state.is_precaching {
                            self.state.is_precaching = false;
                            self.state.is_animating = true;
                            if self.state.anim_direction == PlayDirection::Forward {
                                self.state.anim_current_val = self.state.min_val;
                            } else {
                                self.state.anim_current_val = self.state.max_val;
                            }
                            self.state.last_frame_instant = None;
                            dispatch_anim_frame(&mut self.state);
                        }
                    }
                }
                WorkerResult::Error(e) => {
                    self.state.error_msg = Some(e);
                    self.state.is_loading = false;
                    self.state.is_animating = false;
                    self.state.is_precaching = false;
                    self.state.is_frame_in_flight = false;
                }
            }
        }
    }

    fn tick_animation(&mut self, ctx: &egui::Context) {
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
                let stepped = match self.state.anim_direction {
                    PlayDirection::Forward => advance_anim_forward(&mut self.state),
                    PlayDirection::Reverse => advance_anim_backward(&mut self.state),
                };
                if stepped {
                    dispatch_anim_frame(&mut self.state);
                }
            }
            ctx.request_repaint_after(target_delay);
        }
    }
}

impl App for JumpChampApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.state.theme_mode);

        self.handle_worker_results();
        self.tick_animation(ctx);

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            status_bar::render(ui, &self.state);
        });

        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| match sidebar::render(ui, &mut self.state) {
                SidebarAction::Compute => dispatch_compute(&mut self.state),
                SidebarAction::Cancel => dispatch_cancel(&mut self.state),
                SidebarAction::StartAnimation => {
                    dispatch_start_animation(&mut self.state, PlayDirection::Forward)
                }
                SidebarAction::StartReverseAnimation => {
                    dispatch_start_animation(&mut self.state, PlayDirection::Reverse)
                }
                SidebarAction::StepAnimation => dispatch_step_animation(&mut self.state),
                SidebarAction::StepBackAnimation => dispatch_step_back_animation(&mut self.state),
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
