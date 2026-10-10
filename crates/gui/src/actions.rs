//! User intents and how they drive state + worker. The only place the two meet.

use std::time::{Duration, Instant};

use super::playback::{Direction, Mode};
use super::state::{AppState, FRAMES};
use super::worker::{Event, Job, Worker};

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
    }
}

/// Displays the current position — from cached frames if possible, else via the worker.
fn show(state: &mut AppState, worker: &Worker) {
    if !state.show_cached_frame() {
        let q = state.query;
        worker.run(Job::Histogram {
            min: q.min,
            max: state.playback.position.max(q.min),
            k: q.k,
        });
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
            Event::Histogram { hist, ms } => {
                state.show_histogram(&hist);
                state.latency_ms = Some(ms);
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
    use super::*;
    use crate::state::Query;

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
}
