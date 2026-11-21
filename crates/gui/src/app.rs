use std::path::{Path, PathBuf};

use eframe::App;

use crate::actions;
use crate::panels::{chart, heatmap, playback_bar, settings, status_bar, toolbar};
use crate::prefs::{ChartView, ThemeMode};
use crate::state::AppState;
use crate::theme::{self, Palette};
use crate::worker::Worker;

pub struct JumpChampApp {
    pub state: AppState,
    pub worker: Worker,
    heatmap_texture: Option<(u64, ThemeMode, egui::TextureHandle)>,
}

impl JumpChampApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let worker = Worker::spawn(move || ctx.request_repaint());
        let mut state = AppState::default();
        theme::apply(&cc.egui_ctx, state.prefs.theme);

        let (index_candidate, gaps_candidate) = discover_dataset_paths();
        if Path::new(&index_candidate).is_file() {
            state.prefs.index_path = index_candidate.clone();
            let gaps_opt = if Path::new(&gaps_candidate).is_file() {
                state.prefs.gaps_path = gaps_candidate.clone();
                Some(PathBuf::from(gaps_candidate))
            } else {
                None
            };
            worker.load_dataset(PathBuf::from(index_candidate), gaps_opt);
        }

        Self {
            state,
            worker,
            heatmap_texture: None,
        }
    }

    fn sync_heatmap_texture(&mut self, ctx: &egui::Context, palette: &Palette) {
        let Some(ref spectral) = self.state.spectral else {
            self.heatmap_texture = None;
            return;
        };

        let gen = self.state.spectral_generation;
        let theme = self.state.prefs.theme;
        let needs_upload = self
            .heatmap_texture
            .as_ref()
            .is_none_or(|(g, t, _)| *g != gen || *t != theme);

        if needs_upload {
            let img = heatmap::build_color_image(spectral, palette);
            let tex = ctx.load_texture("spectral_heatmap", img, egui::TextureOptions::NEAREST);
            self.heatmap_texture = Some((gen, theme, tex));
        }
    }
}

fn discover_dataset_paths() -> (String, String) {
    let mut index_path = std::env::var("JUMPCHAMP_INDEX")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "block_index.bin".to_string());
    let mut gaps_path = std::env::var("JUMPCHAMP_GAPS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gaps.bin".to_string());

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--index" => {
                if let Some(val) = args.next() {
                    index_path = val;
                }
            }
            "--gaps" => {
                if let Some(val) = args.next() {
                    gaps_path = val;
                }
            }
            _ => {}
        }
    }

    (index_path, gaps_path)
}

impl App for JumpChampApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = Palette::of(self.state.prefs.theme);
        theme::apply(ctx, self.state.prefs.theme);

        actions::receive(&mut self.state, &self.worker);
        if let Some(delay) = actions::tick(&mut self.state, &self.worker, std::time::Instant::now())
        {
            ctx.request_repaint_after(delay);
        }

        self.sync_heatmap_texture(ctx, &palette);

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            status_bar::render(ui, &self.state);
        });

        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| {
                if let Some(act) = toolbar::render(ui, &mut self.state, &palette) {
                    actions::apply(act, &mut self.state, &self.worker);
                }
                if let Some(act) = playback_bar::render(ui, &mut self.state) {
                    actions::apply(act, &mut self.state, &self.worker);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.state.prefs.view_mode {
            ChartView::BarChart => {
                chart::render(ui, &mut self.state, &palette);
            }
            ChartView::SpectralHeatmap => {
                let tex = self.heatmap_texture.as_ref().map(|(_, _, t)| t);
                if let Some(act) = heatmap::render(ui, &mut self.state, tex, &palette) {
                    actions::apply(act, &mut self.state, &self.worker);
                }
            }
            ChartView::Split => {
                let total_h = ui.available_height();
                let top_h = (total_h * 0.52).max(180.0);
                let width = ui.available_width();
                ui.allocate_ui_with_layout(
                    egui::vec2(width, top_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        chart::render(ui, &mut self.state, &palette);
                    },
                );
                ui.separator();
                let tex = self.heatmap_texture.as_ref().map(|(_, _, t)| t);
                if let Some(act) = heatmap::render(ui, &mut self.state, tex, &palette) {
                    actions::apply(act, &mut self.state, &self.worker);
                }
            }
        });

        if let Some(act) = settings::render(ctx, &mut self.state, &palette) {
            actions::apply(act, &mut self.state, &self.worker);
        }
    }
}

pub fn run() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!(
            "JumpChamp {} — Prime Gap Explorer 🦀",
            crate::APP_VERSION
        ))
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([900.0, 600.0]);

    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/128x128.png"))
    {
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
