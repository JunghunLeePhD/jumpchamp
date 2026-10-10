//! Background worker thread: runs engine jobs off the UI thread.
//!
//! Cancellation is epoch-based: every job is tagged with the epoch it was sent in, and
//! `cancel()` bumps the epoch. A running job stops at its next sieve block, and any
//! events it already produced are dropped by `poll()`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};

use jumpchamp_core::engine::{range_histogram, ChunkCache, FrameSet, Histogram, Progress};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// Gap histogram for prime indices `min..=max`.
    Histogram { min: u64, max: u64, k: usize },
    /// `count` cumulative animation frames over `min..=max`.
    Frames {
        min: u64,
        max: u64,
        k: usize,
        count: usize,
    },
}

pub enum Event {
    Progress(Progress),
    Histogram { hist: Histogram, ms: f64 },
    Frames { frames: FrameSet, ms: f64 },
}

enum Command {
    Run(u64, Job),
    ClearCache,
}

pub struct Worker {
    tx: Sender<Command>,
    rx: Receiver<(u64, Event)>,
    epoch: Arc<AtomicU64>,
}

impl Worker {
    /// Spawns the thread. `notify` is called after every event (e.g. to request a repaint).
    pub fn spawn(notify: impl Fn() + Send + 'static) -> Self {
        let (tx, commands) = unbounded();
        let (events, rx) = unbounded();
        let epoch = Arc::new(AtomicU64::new(0));
        let shared = Arc::clone(&epoch);
        std::thread::spawn(move || serve(commands, events, shared, notify));
        Self { tx, rx, epoch }
    }

    pub fn run(&self, job: Job) {
        self.tx.send(Command::Run(self.epoch(), job)).ok();
    }

    /// Abandons all queued and running jobs.
    pub fn cancel(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }

    /// Cancels everything and frees the chunk cache.
    pub fn clear_cache(&self) {
        self.cancel();
        self.tx.send(Command::ClearCache).ok();
    }

    /// Events from current (non-cancelled) jobs.
    pub fn poll(&self) -> Vec<Event> {
        let now = self.epoch();
        self.rx
            .try_iter()
            .filter(|(tag, _)| *tag == now)
            .map(|(_, e)| e)
            .collect()
    }

    fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }
}

fn serve(
    commands: Receiver<Command>,
    events: Sender<(u64, Event)>,
    epoch: Arc<AtomicU64>,
    notify: impl Fn(),
) {
    let mut cache = ChunkCache::default();
    let emit = |tag: u64, event: Event| {
        events.send((tag, event)).ok();
        notify();
    };

    for command in commands {
        let (tag, job) = match command {
            Command::ClearCache => {
                cache.clear();
                continue;
            }
            Command::Run(tag, job) => (tag, job),
        };

        let live = || epoch.load(Ordering::SeqCst) == tag;
        if !live() {
            continue;
        }

        let mut last_report: Option<Instant> = None;
        let on_block = |p: Progress| {
            if p.done == p.total || last_report.is_none_or(|t| t.elapsed() >= PROGRESS_INTERVAL) {
                last_report = Some(Instant::now());
                emit(tag, Event::Progress(p));
            }
            live()
        };

        let start = Instant::now();
        let ms = || start.elapsed().as_secs_f64() * 1000.0;
        let result = match job {
            Job::Histogram { min, max, k } => range_histogram(min, max, k, &mut cache, on_block)
                .map(|hist| Event::Histogram { hist, ms: ms() }),
            Job::Frames { min, max, k, count } => FrameSet::build(min, max, k, count, on_block)
                .map(|frames| Event::Frames { frames, ms: ms() }),
        };
        if let Some(event) = result {
            emit(tag, event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for_result(worker: &Worker) -> Event {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            for event in worker.poll() {
                if !matches!(event, Event::Progress(_)) {
                    return event;
                }
            }
            assert!(Instant::now() < deadline, "worker timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn test_histogram_job() {
        let worker = Worker::spawn(|| {});
        worker.run(Job::Histogram {
            min: 1,
            max: 9,
            k: 1,
        });
        // primes 2..=29: gaps 1,2,2,4,2,4,2,4,6
        match wait_for_result(&worker) {
            Event::Histogram { hist, .. } => assert_eq!(&hist[..7], &[0, 1, 4, 0, 3, 0, 1]),
            _ => panic!("expected histogram"),
        }
    }

    #[test]
    fn test_frames_job() {
        let worker = Worker::spawn(|| {});
        worker.run(Job::Frames {
            min: 1,
            max: 1_000,
            k: 2,
            count: 10,
        });
        match wait_for_result(&worker) {
            Event::Frames { frames, .. } => assert!(frames.matches(1, 1_000, 2)),
            _ => panic!("expected frames"),
        }
    }

    #[test]
    fn test_cancelled_job_events_are_dropped() {
        let worker = Worker::spawn(|| {});
        worker.run(Job::Histogram {
            min: 1,
            max: 1_000,
            k: 1,
        });
        worker.cancel();
        worker.run(Job::Histogram {
            min: 1,
            max: 3,
            k: 1,
        });
        match wait_for_result(&worker) {
            Event::Histogram { hist, .. } => assert_eq!(hist.iter().sum::<u64>(), 3),
            _ => panic!("expected histogram"),
        }
    }
}
