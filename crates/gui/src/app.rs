use eframe::App;

use crate::gui::actions;
use crate::gui::panels::{chart, playback_bar, settings, status_bar, toolbar};
use crate::gui::state::AppState;
use crate::gui::theme::{self, Palette};
use crate::gui::worker::Worker;

pub struct JumpChampApp {
    pub state: AppState,
    pub worker: Worker,
}

impl JumpChampApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let worker = Worker::spawn(move || ctx.request_repaint());
        let state = AppState::default();
        theme::apply(&cc.egui_ctx, state.prefs.theme);
        Self { state, worker }
    }
}

impl App for JumpChampApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = Palette::of(self.state.prefs.theme);
        theme::apply(ctx, self.state.prefs.theme);

        actions::receive(&mut self.state, &self.worker);
        if let Some(delay) = actions::tick(&mut self.state, &self.worker, std::time::Instant::now()) {
            ctx.request_repaint_after(delay);
        }

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

        egui::CentralPanel::default().show(ctx, |ui| {
            chart::render(ui, &mut self.state, &palette);
        });

        settings::render(ctx, &mut self.state, &palette);
    }
}

pub fn run() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("JumpChamp — Prime Gap Explorer 🦀")
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([900.0, 600.0]);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/128x128.png")) {
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
