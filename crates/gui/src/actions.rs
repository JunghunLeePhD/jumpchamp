//! User intents and how they drive state + worker. The only place the two meet.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use jumpchamp_core::storage::SpectralHeatmap;

use super::playback::{Direction, Mode};
use super::prefs::ChartView;
use super::state::{AppState, FRAMES};
use super::worker::{Event, Job, Worker};

const SPECTRAL_MAX_COLS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Abort precaching / playback.
    Cancel,
    /// Back to launch defaults and drop all caches.
    Reset,
    /// Continuous playback (precaching frames first if needed).
    Play(Direction),
    /// Move one frame.
    Step(Direction),
    /// Redraw the current position (after scrubbing, stopping, changing k…).
    Show,
    /// Mount `block_index.bin` (and optional `gaps.bin`) from `state.prefs`.
    LoadDataset,
    /// Unmount the memory-mapped dataset and return to the in-memory sieve.
    UnloadDataset,
    /// Compute or refresh the 2D spectral heatmap for the current query + preferences.
    RefreshSpectral,
}

pub fn apply(action: Action, state: &mut AppState, worker: &Worker) {
    match action {
        Action::Cancel => {
            worker.cancel();
            state.playback.mode = Mode::Stopped;
        }
        Action::Reset => {
            worker.clear_cache();
            state.reset();
            if state.dataset.is_some() {
                show(state, worker);
            }
        }
        Action::Play(direction) => play(direction, state, worker),
        Action::Step(direction) => {
            let q = state.query;
            state.playback.direction = direction;
            if !state.playback.rewind_if_finished(q.min, q.max) {
                state.playback.advance(q.min, q.max, q.step());
            }
            show(state, worker);
        }
        Action::Show => show(state, worker),
        Action::LoadDataset => load_dataset(state, worker),
        Action::UnloadDataset => {
            worker.unload_dataset();
            state.dataset = None;
            state.dataset_error = None;
            state.frames = None;
            state.spectral = None;
            state.spectral_key = None;
            state.last_query_sampled = false;
        }
        Action::RefreshSpectral => refresh_spectral(state, worker),
    }
}

fn load_dataset(state: &mut AppState, worker: &Worker) {
    let idx = state.prefs.index_path.trim();
    if idx.is_empty() {
        state.dataset_error = Some("Please provide a path to block_index.bin".to_string());
        return;
    }

    let raw_trimmed = state.prefs.gaps_path.trim();
    let gaps_path = if raw_trimmed.is_empty() {
        None
    } else {
        let p = Path::new(raw_trimmed);
        if p.exists() {
            Some(p.to_path_buf())
        } else if raw_trimmed == "gaps.bin" {
            None
        } else {
            state.dataset_error = Some(format!("Raw gaps file not found: {raw_trimmed}"));
            return;
        }
    };

    state.dataset_error = None;
    worker.load_dataset(PathBuf::from(idx), gaps_path);
}

fn refresh_spectral(state: &mut AppState, worker: &Worker) {
    let key = state.current_spectral_key();
    if state.spectral_key == Some(key) {
        return;
    }

    let q = state.query;
    if state.dataset.is_none() {
        if let Some(frames) = state
            .frames
            .as_ref()
            .filter(|f| f.matches(q.min, q.max, q.k))
        {
            state.spectral = Some(SpectralHeatmap::from_frame_set(
                frames,
                key.max_gap,
                key.even_only,
                key.log_scale,
            ));
            state.spectral_key = Some(key);
            state.spectral_generation += 1;
            return;
        }
    }

    worker.run(Job::Spectral {
        min: q.min,
        max: q.max,
        k: q.k,
        cols: SPECTRAL_MAX_COLS,
        max_gap: key.max_gap,
        even_only: key.even_only,
        log_scale: key.log_scale,
    });
}

/// Displays the current position — from cached frames if possible, else via the worker.
fn show(state: &mut AppState, worker: &Worker) {
    let need_spectral = state.prefs.view_mode != ChartView::BarChart && state.dataset.is_some();
    let spectral_stale = need_spectral && state.spectral_key != Some(state.current_spectral_key());

    let from_cache = state.show_cached_frame();
    if !from_cache || spectral_stale {
        if state.dataset.is_some() {
            worker.cancel();
        }
        let q = state.query;
        if !from_cache {
            worker.run(Job::Histogram {
                min: q.min,
                max: state.playback.position.max(q.min),
                k: q.k,
            });
        }
        if spectral_stale {
            let key = state.current_spectral_key();
            worker.run(Job::Spectral {
                min: q.min,
                max: q.max,
                k: q.k,
                cols: SPECTRAL_MAX_COLS,
                max_gap: key.max_gap,
                even_only: key.even_only,
                log_scale: key.log_scale,
            });
        }
    }
}

fn play(direction: Direction, state: &mut AppState, worker: &Worker) {
    let q = state.query;
    state.playback.direction = direction;
    state.playback.rewind_if_finished(q.min, q.max);

    if state
        .frames
        .as_ref()
        .is_some_and(|f| f.matches(q.min, q.max, q.k))
    {
        state.playback.play();
        show(state, worker);
    } else {
        state.playback.mode = Mode::Precaching;
        state.progress = None;
        worker.run(Job::Frames {
            min: q.min,
            max: q.max,
            k: q.k,
            count: FRAMES,
        });
    }
}

/// Folds worker events into the state.
pub fn receive(state: &mut AppState, worker: &Worker) {
    for event in worker.poll() {
        match event {
            Event::Progress(p) => state.progress = Some(p),
            Event::Histogram { hist, ms, sampled } => {
                state.show_histogram(&hist);
                state.latency_ms = Some(ms);
                state.last_query_sampled = sampled;
            }
            Event::Frames { frames, ms } => {
                state.frames = Some(frames);
                state.latency_ms = Some(ms);
                state.progress = None;
                if state.playback.mode == Mode::Precaching {
                    let q = state.query;
                    state.playback.position = match state.playback.direction {
                        Direction::Forward => q.min,
                        Direction::Reverse => q.max,
                    };
                    state.playback.play();
                }
                state.show_cached_frame();
            }
            Event::Spectral { heatmap, key, ms } => {
                state.spectral = Some(heatmap);
                state.spectral_key = Some(key);
                state.spectral_generation += 1;
                state.latency_ms = Some(ms);
            }
            Event::DatasetLoaded { meta, ms } => {
                state.apply_dataset(meta);
                state.latency_ms = Some(ms);
                show(state, worker);
            }
            Event::DatasetUnloaded => {
                state.dataset = None;
                state.dataset_error = None;
                state.frames = None;
                state.spectral = None;
                state.spectral_key = None;
                state.last_query_sampled = false;
            }
            Event::DatasetError(err) => {
                state.dataset_error = Some(err);
            }
        }
    }
}

/// Advances playback if a frame is due. Returns when to wake up next, if playing.
pub fn tick(state: &mut AppState, worker: &Worker, now: Instant) -> Option<Duration> {
    if state.playback.mode != Mode::Playing {
        return None;
    }
    if state.playback.due(now) {
        let q = state.query;
        if state.playback.advance(q.min, q.max, q.step()) {
            show(state, worker);
        }
    }
    Some(state.playback.interval())
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    use jumpchamp_core::storage::build_block_index;

    use super::*;
    use crate::state::Query;

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jc_act_{}_{}_{}", std::process::id(), id, name))
    }

    fn settle(state: &mut AppState, worker: &Worker, until: impl Fn(&AppState) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !until(state) {
            assert!(Instant::now() < deadline, "timed out");
            std::thread::sleep(Duration::from_millis(5));
            receive(state, worker);
        }
    }

    fn small_state() -> AppState {
        let mut state = AppState {
            query: Query {
                k: 1,
                min: 1,
                max: 3_000,
            },
            ..Default::default()
        };
        state.playback.position = 1;
        state
    }

    #[test]
    fn test_play_precaches_then_plays_from_start() {
        let worker = Worker::spawn(|| {});
        let mut state = small_state();
        apply(Action::Play(Direction::Forward), &mut state, &worker);
        assert_eq!(state.playback.mode, Mode::Precaching);

        settle(&mut state, &worker, |s| s.playback.mode == Mode::Playing);
        assert_eq!(state.playback.position, 1);
        assert!(!state.bars.is_empty());

        // Ticks are served from cached frames, without the worker.
        let t0 = Instant::now();
        tick(&mut state, &worker, t0);
        assert_eq!(state.playback.position, 1 + state.query.step());
    }

    #[test]
    fn test_step_without_frames_queries_worker() {
        let worker = Worker::spawn(|| {});
        let mut state = small_state();
        apply(Action::Step(Direction::Forward), &mut state, &worker);
        assert_eq!(state.playback.position, 1 + state.query.step());
        settle(&mut state, &worker, |s| !s.bars.is_empty());
        let total: u64 = state.bars.iter().map(|b| b.1).sum();
        assert_eq!(total, state.playback.position);
    }

    #[test]
    fn test_cancel_and_reset() {
        let worker = Worker::spawn(|| {});
        let mut state = small_state();
        apply(Action::Play(Direction::Reverse), &mut state, &worker);
        apply(Action::Cancel, &mut state, &worker);
        assert_eq!(state.playback.mode, Mode::Stopped);

        state.selected_gap = Some(2);
        apply(Action::Reset, &mut state, &worker);
        assert_eq!(state.selected_gap, None);
        assert_eq!(state.query, Query::full(state.prefs.max_prime_limit));
    }

    #[test]
    fn test_tick_idle_does_nothing() {
        let worker = Worker::spawn(|| {});
        let mut state = small_state();
        assert_eq!(tick(&mut state, &worker, Instant::now()), None);
    }

    #[test]
    fn test_dataset_auto_populates_bars_and_spectral_heatmap() {
        let gaps_path = temp_path("gaps.bin");
        let index_path = temp_path("index.bin");

        let gaps = vec![2u16; 60];
        let mut f = File::create(&gaps_path).unwrap();
        for g in &gaps {
            f.write_all(&g.to_le_bytes()).unwrap();
        }
        f.flush().unwrap();
        build_block_index(&gaps_path, &index_path, 20, |_, _| {}).unwrap();

        let worker = Worker::spawn(|| {});
        let mut state = AppState::default();
        state.prefs.view_mode = ChartView::Split;

        worker.load_dataset_with_block_size(index_path.clone(), Some(gaps_path.clone()), 20);
        settle(&mut state, &worker, |s| {
            s.dataset.is_some() && !s.bars.is_empty() && s.spectral.is_some()
        });

        assert_eq!(state.query.k, 1);
        assert_eq!(state.bars, vec![(2, 60)]);
        assert!(state.spectral_generation >= 1);

        apply(Action::UnloadDataset, &mut state, &worker);
        assert!(state.dataset.is_none());
        assert!(state.spectral.is_none());

        let _ = std::fs::remove_file(&gaps_path);
        let _ = std::fs::remove_file(&index_path);
    }
}
