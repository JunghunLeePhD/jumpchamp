//! Second row: playback buttons, position scrubber, speed, progress and frame counter.

use egui::{vec2, Button, ProgressBar, RichText, Ui};

use crate::actions::Action;
use crate::format::compact;
use crate::playback::{Direction, Mode};
use crate::state::{AppState, FRAMES};
use crate::theme;
use crate::widgets::{hint, index_input};

const BUTTON: egui::Vec2 = vec2(26.0, 20.0);
const SPEED_PRESETS: [f32; 4] = [15.0, 30.0, 60.0, 120.0];

pub fn render(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        action = transport(ui, state);
        ui.add_enabled_ui(!state.playback.is_busy(), |ui| {
            action = action.or(stepping(ui, state)).or(scrubber(ui, state));
        });
        speed(ui, state);
        progress(ui, state);
        frame_counter(ui, state);
    });
    ui.add_space(2.0);
    action
}

fn button(ui: &mut Ui, label: &str, tip: &str) -> bool {
    ui.add_sized(BUTTON, Button::new(label))
        .on_hover_text(tip)
        .clicked()
}

/// Play / pause / stop / cancel, depending on the mode.
fn transport(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    let p = &mut state.playback;
    let mut action = None;
    match p.mode {
        Mode::Precaching => {
            let size = vec2(BUTTON.x * 2.0 + ui.spacing().item_spacing.x, BUTTON.y);
            if ui
                .add_sized(size, Button::new("⏹ Cancel"))
                .on_hover_text("Cancel in-progress pre-caching computation")
                .clicked()
            {
                action = Some(Action::Cancel);
            }
        }
        Mode::Playing => {
            if button(ui, "⏸", "Pause Growth Chart Animation at current frame") {
                p.mode = Mode::Stopped;
            }
            if button(ui, "⏹", "Stop Animation and reset frame to start (n_min)") {
                p.mode = Mode::Stopped;
                p.position = state.query.min;
                action = Some(Action::Show);
            }
        }
        Mode::Stopped => {
            if button(ui, "◀", "Play Reverse Animation") {
                action = Some(Action::Play(Direction::Reverse));
            }
            if button(ui, "▶", "Play Forward Animation") {
                action = Some(Action::Play(Direction::Forward));
            }
        }
    }
    action
}

/// Step back / step forward / back to start.
fn stepping(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    const LOCKED: &str = "disabled while animation is running. Pause or stop first.";
    let mut action = None;
    let mut step = |label: &str, tip: &str| {
        hint(
            ui.add_sized(BUTTON, Button::new(label)),
            tip,
            &format!("{tip} is {LOCKED}"),
        )
        .clicked()
    };
    if step("⏮", "Step Back One Frame") {
        action = Some(Action::Step(Direction::Reverse));
    }
    if step("⏭", "Step Forward One Frame") {
        action = Some(Action::Step(Direction::Forward));
    }
    if step("↺", "Reset to Start") {
        state.playback.position = state.query.min;
        action = Some(Action::Show);
    }
    ui.separator();
    action
}

fn scrubber(ui: &mut Ui, state: &mut AppState) -> Option<Action> {
    let q = state.query;
    let changed = hint(
        index_input(ui, &mut state.playback.position, q.min..=q.max),
        "Current prime index position (n). Drag to scrub frames.",
        "Frame scrubbing is locked while animation is running. Pause or stop first.",
    )
    .changed();
    ui.separator();
    changed.then_some(Action::Show)
}

fn speed(ui: &mut Ui, state: &mut AppState) {
    let fps = &mut state.playback.fps;
    ui.label("Speed:");
    ui.add_sized(
        [110.0, 18.0],
        egui::Slider::new(fps, 1.0..=120.0)
            .step_by(1.0)
            .suffix(" FPS")
            .clamping(egui::SliderClamping::Always),
    );
    for preset in SPEED_PRESETS {
        if ui
            .button(format!("{preset}"))
            .on_hover_text(format!("Set to {preset} FPS"))
            .clicked()
        {
            *fps = preset;
        }
    }
    ui.separator();
}

fn progress(ui: &mut Ui, state: &AppState) {
    let (fraction, text, tip) = match (state.playback.mode, state.progress) {
        (Mode::Precaching, progress) => {
            let (fraction, blocks) = match progress {
                Some(p) => (
                    (p.done as f32 / p.total as f32).clamp(0.0, 0.99),
                    format!(" (Block {}/{})", p.done, p.total),
                ),
                None => (0.0, String::new()),
            };
            let text = format!("Caching: {:.0}%", fraction * 100.0);
            (
                fraction,
                text,
                format!("Pre-caching {FRAMES} animation frames in background{blocks}..."),
            )
        }
        _ => {
            let fraction = state.animation_progress();
            let tip = format!(
                "Animation progress: {:.1}% (Current n = {} / {})",
                fraction * 100.0,
                compact(state.playback.position),
                compact(state.query.max)
            );
            (fraction, format!("{:.0}%", fraction * 100.0), tip)
        }
    };
    ui.add_sized(
        [120.0, 18.0],
        ProgressBar::new(fraction).show_percentage().text(text),
    )
    .on_hover_text(tip);
    ui.separator();
}

fn frame_counter(ui: &mut Ui, state: &AppState) {
    let frame = state
        .playback
        .frame_number(state.query.min, state.query.step(), FRAMES as u64);
    ui.label(
        RichText::new(format!("Frame {frame}/{FRAMES}"))
            .strong()
            .color(theme::PLAYING),
    );
}
