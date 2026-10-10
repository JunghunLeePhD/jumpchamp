//! Animation playback: a pure state machine over a prime-index position.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Stopped,
    /// Waiting for the worker to build animation frames.
    Precaching,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

#[derive(Debug, Clone)]
pub struct Playback {
    pub mode: Mode,
    pub direction: Direction,
    /// Current prime index `n` (upper bound of the displayed range).
    pub position: u64,
    pub fps: f32,
    last_tick: Option<Instant>,
}

impl Playback {
    pub fn new(position: u64) -> Self {
        Self {
            mode: Mode::Stopped,
            direction: Direction::Forward,
            position,
            fps: 30.0,
            last_tick: None,
        }
    }

    /// Playing or precaching — i.e. the range controls must stay locked.
    pub fn is_busy(&self) -> bool {
        self.mode != Mode::Stopped
    }

    /// Starts playing on the next tick.
    pub fn play(&mut self) {
        self.mode = Mode::Playing;
        self.last_tick = None;
    }

    /// Time between frames at the current speed.
    pub fn interval(&self) -> Duration {
        Duration::from_secs_f64(1.0 / (self.fps.max(1.0) as f64))
    }

    /// True (and records `now`) if a new frame is due.
    pub fn due(&mut self, now: Instant) -> bool {
        let due = self
            .last_tick
            .is_none_or(|last| now.duration_since(last) >= self.interval());
        if due {
            self.last_tick = Some(now);
        }
        due
    }

    /// Moves one `step` in the current direction within `[min, max]`.
    /// At the boundary, stops playback and returns `false`.
    pub fn advance(&mut self, min: u64, max: u64, step: u64) -> bool {
        let next = match self.direction {
            Direction::Forward if self.position < max => (self.position + step).min(max),
            Direction::Reverse if self.position > min => {
                self.position.saturating_sub(step).max(min)
            }
            _ => {
                if self.mode == Mode::Playing {
                    self.mode = Mode::Stopped;
                }
                return false;
            }
        };
        self.position = next;
        true
    }

    /// If already at the end for the current direction, jumps back to its start.
    pub fn rewind_if_finished(&mut self, min: u64, max: u64) -> bool {
        let (end, start) = match self.direction {
            Direction::Forward => (self.position >= max, min),
            Direction::Reverse => (self.position <= min, max),
        };
        if end {
            self.position = start;
        }
        end
    }

    /// Position as a fraction of `[min, max]`.
    pub fn progress(&self, min: u64, max: u64) -> f32 {
        let range = max.saturating_sub(min).max(1) as f32;
        (self.position.saturating_sub(min) as f32 / range).clamp(0.0, 1.0)
    }

    /// 1-based frame number for display, capped at `frames + 1`.
    pub fn frame_number(&self, min: u64, step: u64, frames: u64) -> u64 {
        (self.position.saturating_sub(min) / step.max(1)).min(frames) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance_forward_stops_at_end() {
        let mut p = Playback::new(90);
        p.play();
        assert!(p.advance(0, 100, 7));
        assert_eq!(p.position, 97);
        assert!(p.advance(0, 100, 7));
        assert_eq!(p.position, 100);
        assert!(!p.advance(0, 100, 7));
        assert_eq!(p.mode, Mode::Stopped);
    }

    #[test]
    fn test_advance_reverse_clamps_to_min() {
        let mut p = Playback::new(12);
        p.direction = Direction::Reverse;
        assert!(p.advance(10, 100, 5));
        assert_eq!(p.position, 10);
        assert!(!p.advance(10, 100, 5));
    }

    #[test]
    fn test_rewind_if_finished() {
        let mut p = Playback::new(100);
        assert!(p.rewind_if_finished(1, 100));
        assert_eq!(p.position, 1);
        assert!(!p.rewind_if_finished(1, 100));

        p.direction = Direction::Reverse;
        assert!(p.rewind_if_finished(1, 100));
        assert_eq!(p.position, 100);
    }

    #[test]
    fn test_progress_and_frame_number() {
        let mut p = Playback::new(100);
        assert!((p.progress(100, 200) - 0.0).abs() < 1e-5);
        p.position = 150;
        assert!((p.progress(100, 200) - 0.5).abs() < 1e-5);
        p.position = 200;
        assert!((p.progress(100, 200) - 1.0).abs() < 1e-5);
        assert_eq!(p.frame_number(100, 10, 300), 11);
    }

    #[test]
    fn test_due_respects_fps() {
        let mut p = Playback::new(0);
        p.fps = 10.0;
        let t0 = Instant::now();
        assert!(p.due(t0));
        assert!(!p.due(t0 + Duration::from_millis(50)));
        assert!(p.due(t0 + Duration::from_millis(101)));
    }

    #[test]
    fn test_is_busy() {
        let mut p = Playback::new(0);
        assert!(!p.is_busy());
        p.mode = Mode::Precaching;
        assert!(p.is_busy());
        p.mode = Mode::Playing;
        assert!(p.is_busy());
    }
}
