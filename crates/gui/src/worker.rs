//! Background worker thread: runs engine and mmap storage jobs off the UI thread.
//!
//! Cancellation is epoch-based: every job is tagged with the epoch it was sent in, and
//! `cancel()` bumps the epoch. A running job stops at its next block, and any
//! events it already produced are dropped by `poll()`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};

use jumpchamp_core::engine::{
    frame_step, histogram, range_histogram_cancellable, ChunkCache, FrameSet, Histogram, Progress,
};
use jumpchamp_core::storage::{DistributionEngine, RawGaps, SpectralHeatmap};
use jumpchamp_core::CancelToken;

use crate::state::{DatasetMeta, SpectralKey};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(33);
const EXACT_QUERY_LIMIT: u64 = 1_000_000;
const K_STEP_SAMPLE_BUDGET: usize = 2_000_000;
const K_STEP_FRAME_SAMPLE_BUDGET: usize = 50_000;
const SPECTRAL_COL_SAMPLES: usize = 20_000;
const SUBBLOCK_FRAME_THRESHOLD: u64 = 32;

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
    /// 2D spectral density heatmap across `[min, max]`.
    Spectral {
        min: u64,
        max: u64,
        k: usize,
        cols: usize,
        max_gap: u64,
        even_only: bool,
        log_scale: bool,
    },
}

pub enum Event {
    Progress(Progress),
    Histogram {
        hist: Histogram,
        ms: f64,
        sampled: bool,
    },
    Frames {
        frames: FrameSet,
        ms: f64,
    },
    Spectral {
        heatmap: SpectralHeatmap,
        key: SpectralKey,
        ms: f64,
    },
    DatasetLoaded {
        meta: DatasetMeta,
        ms: f64,
    },
    DatasetUnloaded,
    DatasetError(String),
}

enum Command {
    Run(u64, CancelToken, Job),
    ClearCache,
    LoadDataset {
        index_path: PathBuf,
        gaps_path: Option<PathBuf>,
        block_size: Option<u64>,
    },
    UnloadDataset,
}

pub struct Worker {
    tx: Sender<Command>,
    rx: Receiver<(u64, Event)>,
    epoch: Arc<AtomicU64>,
    active_cancel: Mutex<CancelToken>,
    busy: Arc<AtomicBool>,
}

impl Worker {
    /// Spawns the thread. `notify` is called after every event (e.g. to request a repaint).
    pub fn spawn(notify: impl Fn() + Send + 'static) -> Self {
        let (tx, commands) = unbounded();
        let (events, rx) = unbounded();
        let epoch = Arc::new(AtomicU64::new(0));
        let shared = Arc::clone(&epoch);
        let busy = Arc::new(AtomicBool::new(false));
        let worker_busy = Arc::clone(&busy);
        std::thread::spawn(move || serve(commands, events, shared, worker_busy, notify));
        Self {
            tx,
            rx,
            epoch,
            active_cancel: Mutex::new(CancelToken::new()),
            busy,
        }
    }

    pub fn run(&self, job: Job) {
        let guard = self
            .active_cancel
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let tag = self.epoch();
        let cancel = guard.clone();
        drop(guard);
        self.tx.send(Command::Run(tag, cancel, job)).ok();
    }

    /// Cancels prior jobs and immediately submits `job` under the new epoch.
    pub fn run_latest(&self, job: Job) {
        self.cancel();
        self.run(job);
    }

    /// Abandons all queued and running jobs immediately via epoch bump and `CancelToken`.
    pub fn cancel(&self) {
        let mut guard = self
            .active_cancel
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.epoch.fetch_add(1, Ordering::SeqCst);
        let old = std::mem::replace(&mut *guard, CancelToken::new());
        drop(guard);
        old.cancel();
    }

    /// Returns `true` while the worker thread is actively executing a job.
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    /// Cancels everything and frees the chunk cache.
    pub fn clear_cache(&self) {
        self.cancel();
        self.tx.send(Command::ClearCache).ok();
    }

    /// Memory-maps `block_index.bin` (and optional `gaps.bin`) off the UI thread.
    pub fn load_dataset(&self, index_path: PathBuf, gaps_path: Option<PathBuf>) {
        self.cancel();
        self.tx
            .send(Command::LoadDataset {
                index_path,
                gaps_path,
                block_size: None,
            })
            .ok();
    }

    /// Memory-maps a dataset built with a custom block size (useful in tests).
    pub fn load_dataset_with_block_size(
        &self,
        index_path: PathBuf,
        gaps_path: Option<PathBuf>,
        block_size: u64,
    ) {
        self.cancel();
        self.tx
            .send(Command::LoadDataset {
                index_path,
                gaps_path,
                block_size: Some(block_size),
            })
            .ok();
    }

    /// Unmounts any active memory-mapped dataset and falls back to the in-memory sieve.
    pub fn unload_dataset(&self) {
        self.cancel();
        self.tx.send(Command::UnloadDataset).ok();
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

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}

struct BusyGuard<'a>(&'a AtomicBool);

impl<'a> BusyGuard<'a> {
    fn new(flag: &'a AtomicBool) -> Self {
        flag.store(true, Ordering::Release);
        Self(flag)
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn serve(
    commands: Receiver<Command>,
    events: Sender<(u64, Event)>,
    epoch: Arc<AtomicU64>,
    busy: Arc<AtomicBool>,
    notify: impl Fn(),
) {
    let mut cache = ChunkCache::default();
    let mut index: Option<DistributionEngine> = None;
    let mut raw_gaps: Option<RawGaps> = None;

    let emit = |tag: u64, event: Event| {
        events.send((tag, event)).ok();
        notify();
    };

    for command in commands {
        let (tag, cancel, job) = match command {
            Command::ClearCache => {
                cache.clear();
                continue;
            }
            Command::UnloadDataset => {
                index = None;
                raw_gaps = None;
                emit(epoch.load(Ordering::SeqCst), Event::DatasetUnloaded);
                continue;
            }
            Command::LoadDataset {
                index_path,
                gaps_path,
                block_size,
            } => {
                let tag = epoch.load(Ordering::SeqCst);
                let start = Instant::now();
                let loaded_idx = match block_size {
                    Some(bs) => DistributionEngine::load_with_block_size(&index_path, bs),
                    None => DistributionEngine::load(&index_path),
                };
                let new_idx = match loaded_idx {
                    Ok(eng) => eng,
                    Err(err) => {
                        emit(
                            tag,
                            Event::DatasetError(format!(
                                "Failed to open index '{}': {err}",
                                index_path.display()
                            )),
                        );
                        continue;
                    }
                };

                let new_raw = match gaps_path {
                    Some(ref gp) => match RawGaps::open(gp) {
                        Ok(raw) => Some(raw),
                        Err(err) => {
                            emit(
                                tag,
                                Event::DatasetError(format!(
                                    "Failed to open gaps '{}': {err}",
                                    gp.display()
                                )),
                            );
                            continue;
                        }
                    },
                    None => None,
                };

                let meta = DatasetMeta {
                    index_path: index_path.display().to_string(),
                    gaps_path: gaps_path.map(|p| p.display().to_string()),
                    block_count: new_idx.block_count(),
                    block_size: new_idx.block_size(),
                    max_primes: new_idx.max_indexed_primes(),
                    has_raw_gaps: new_raw.is_some(),
                    raw_gap_count: new_raw.as_ref().map_or(0, |r| r.len() as u64),
                };
                index = Some(new_idx);
                raw_gaps = new_raw;
                emit(
                    tag,
                    Event::DatasetLoaded {
                        meta,
                        ms: start.elapsed().as_secs_f64() * 1000.0,
                    },
                );
                continue;
            }
            Command::Run(tag, cancel, job) => (tag, cancel, job),
        };

        let live = || !cancel.is_cancelled() && epoch.load(Ordering::SeqCst) == tag;
        if !live() {
            continue;
        }
        let _busy_guard = BusyGuard::new(&busy);

        let mut last_report: Option<Instant> = None;
        let on_block = |p: Progress| {
            if !live() {
                return false;
            }
            if p.done == p.total || last_report.is_none_or(|t| t.elapsed() >= PROGRESS_INTERVAL) {
                last_report = Some(Instant::now());
                emit(tag, Event::Progress(p));
            }
            true
        };

        let start = Instant::now();
        let ms = || start.elapsed().as_secs_f64() * 1000.0;
        let result = run_job(
            job,
            index.as_ref(),
            raw_gaps.as_ref(),
            &mut cache,
            &cancel,
            on_block,
        );
        if !live() {
            continue;
        }
        match result {
            Ok(Some(ev_fn)) => emit(tag, ev_fn(ms())),
            Ok(None) => {}
            Err(err) => emit(tag, Event::DatasetError(err)),
        }
    }
}

type JobOutput = Result<Option<Box<dyn FnOnce(f64) -> Event>>, String>;

fn run_job(
    job: Job,
    index: Option<&DistributionEngine>,
    raw_gaps: Option<&RawGaps>,
    cache: &mut ChunkCache,
    cancel: &CancelToken,
    mut on_block: impl FnMut(Progress) -> bool,
) -> JobOutput {
    if cancel.is_cancelled() {
        return Ok(None);
    }

    match job {
        Job::Histogram { min, max, k } => {
            let start0 = min.saturating_sub(1);
            let end0 = max.saturating_sub(1);

            if k == 1 {
                if let Some(idx) = index {
                    let raw_opt = (raw_gaps.is_some()
                        && max.saturating_sub(min) < EXACT_QUERY_LIMIT)
                        .then_some(raw_gaps)
                        .flatten();
                    let Some(dist) =
                        idx.query_distribution_exact_cancellable(start0, end0, raw_opt, cancel)
                    else {
                        return Ok(None);
                    };
                    let hist = dist.to_vec();
                    return Ok(Some(Box::new(move |ms| Event::Histogram {
                        hist,
                        ms,
                        sampled: false,
                    })));
                }
            }

            if let Some(raw) = raw_gaps {
                let Some((hist, sampled)) = raw.k_step_histogram_sampled_cancellable(
                    start0 as usize,
                    max as usize,
                    k,
                    K_STEP_SAMPLE_BUDGET,
                    cancel,
                ) else {
                    return Ok(None);
                };
                return Ok(Some(Box::new(move |ms| Event::Histogram {
                    hist,
                    ms,
                    sampled,
                })));
            }

            if index.is_some() && k > 1 && max > 10_000_000 {
                return Err("Load raw gaps.bin to query k > 1 gaps beyond 10M primes".to_string());
            }

            Ok(
                range_histogram_cancellable(min, max, k, cache, cancel, on_block).map(|hist| {
                    Box::new(move |ms| Event::Histogram {
                        hist,
                        ms,
                        sampled: false,
                    }) as Box<dyn FnOnce(f64) -> Event>
                }),
            )
        }
        Job::Frames { min, max, k, count } => {
            if k == 1 {
                if let Some(idx) = index {
                    let start0 = min.saturating_sub(1);
                    let end0 = max.saturating_sub(1);
                    let blocks_spanned =
                        (end0 / idx.block_size()).saturating_sub(start0 / idx.block_size()) + 1;
                    let raw_opt = (raw_gaps.is_some() && blocks_spanned < SUBBLOCK_FRAME_THRESHOLD)
                        .then_some(raw_gaps)
                        .flatten();
                    let Some(frames) = idx
                        .build_frames_exact_cancellable(min, max, count, raw_opt, cancel, on_block)
                    else {
                        return Ok(None);
                    };
                    return Ok(Some(Box::new(move |ms| Event::Frames { frames, ms })));
                }
            }

            if let Some(raw) = raw_gaps {
                let count = count.max(1);
                let step = frame_step(min, max, count);
                let mut deltas = Vec::with_capacity(count);
                for i in 0..count {
                    if cancel.is_cancelled()
                        || !on_block(Progress {
                            done: i + 1,
                            total: count,
                        })
                    {
                        return Ok(None);
                    }
                    let f_start = min + i as u64 * step;
                    let f_end = if i + 1 == count {
                        max
                    } else {
                        (f_start + step - 1).min(max)
                    };
                    if f_start <= f_end && f_start <= max {
                        let Some((hist, _)) = raw.k_step_histogram_sampled_cancellable(
                            f_start.saturating_sub(1) as usize,
                            f_end as usize,
                            k,
                            K_STEP_FRAME_SAMPLE_BUDGET,
                            cancel,
                        ) else {
                            return Ok(None);
                        };
                        deltas.push(histogram::trimmed(&hist));
                    } else {
                        deltas.push(Vec::new());
                    }
                }
                let frames = FrameSet::from_deltas(min, max, k, &deltas);
                return Ok(Some(Box::new(move |ms| Event::Frames { frames, ms })));
            }

            if index.is_some() && k > 1 && max > 10_000_000 {
                return Err("Load raw gaps.bin to animate k > 1 beyond 10M primes".to_string());
            }

            Ok(
                FrameSet::build_cancellable(min, max, k, count, cancel, on_block)
                    .map(|frames| Box::new(move |ms| Event::Frames { frames, ms }) as _),
            )
        }
        Job::Spectral {
            min,
            max,
            k,
            cols,
            max_gap,
            even_only,
            log_scale,
        } => {
            let key = SpectralKey {
                min,
                max,
                k,
                max_gap,
                even_only,
                log_scale,
            };

            if k == 1 {
                if let Some(idx) = index {
                    let start0 = min.saturating_sub(1);
                    let end0 = max.saturating_sub(1);
                    let blocks_spanned =
                        (end0 / idx.block_size()).saturating_sub(start0 / idx.block_size()) + 1;
                    let heatmap = if let (true, Some(raw)) =
                        (blocks_spanned < SUBBLOCK_FRAME_THRESHOLD, raw_gaps)
                    {
                        SpectralHeatmap::from_raw_gaps_cancellable(
                            raw,
                            min,
                            max,
                            1,
                            cols,
                            max_gap,
                            even_only,
                            log_scale,
                            SPECTRAL_COL_SAMPLES,
                            cancel,
                            on_block,
                        )
                    } else {
                        SpectralHeatmap::from_index_cancellable(
                            idx, min, max, cols, max_gap, even_only, log_scale, cancel,
                        )
                    };
                    let Some(heatmap) = heatmap else {
                        return Ok(None);
                    };
                    return Ok(Some(Box::new(move |ms| Event::Spectral {
                        heatmap,
                        key,
                        ms,
                    })));
                }
            }

            if let Some(raw) = raw_gaps {
                let Some(heatmap) = SpectralHeatmap::from_raw_gaps_cancellable(
                    raw,
                    min,
                    max,
                    k,
                    cols,
                    max_gap,
                    even_only,
                    log_scale,
                    SPECTRAL_COL_SAMPLES,
                    cancel,
                    on_block,
                ) else {
                    return Ok(None);
                };
                return Ok(Some(Box::new(move |ms| Event::Spectral {
                    heatmap,
                    key,
                    ms,
                })));
            }

            if index.is_some() && k > 1 && max > 10_000_000 {
                return Err(
                    "Load raw gaps.bin to inspect k > 1 spectral heatmap beyond 10M primes"
                        .to_string(),
                );
            }

            let frame_count = cols.clamp(16, 120);
            Ok(
                FrameSet::build_cancellable(min, max, k, frame_count, cancel, on_block).map(
                    |frames| {
                        let heatmap =
                            SpectralHeatmap::from_frame_set(&frames, max_gap, even_only, log_scale);
                        Box::new(move |ms| Event::Spectral { heatmap, key, ms }) as _
                    },
                ),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    use jumpchamp_core::storage::build_block_index;

    use super::*;

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jc_wrk_{}_{}_{}", std::process::id(), id, name))
    }

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
            Event::Histogram { hist, sampled, .. } => {
                assert!(!sampled);
                assert_eq!(&hist[..7], &[0, 1, 4, 0, 3, 0, 1]);
            }
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

    #[test]
    fn test_dataset_load_query_k_step_and_spectral() {
        let gaps_path = temp_path("gaps.bin");
        let index_path = temp_path("index.bin");

        let gaps: Vec<u16> = (0..100).map(|i| if i % 2 == 0 { 2 } else { 4 }).collect();
        let mut f = File::create(&gaps_path).unwrap();
        for g in &gaps {
            f.write_all(&g.to_le_bytes()).unwrap();
        }
        f.flush().unwrap();
        build_block_index(&gaps_path, &index_path, 25, |_, _| {}).unwrap();

        let worker = Worker::spawn(|| {});
        worker.load_dataset_with_block_size(index_path.clone(), Some(gaps_path.clone()), 25);
        match wait_for_result(&worker) {
            Event::DatasetLoaded { meta, .. } => {
                assert_eq!(meta.block_count, 4);
                assert_eq!(meta.max_primes, 100);
                assert!(meta.has_raw_gaps);
            }
            _ => panic!("expected DatasetLoaded"),
        }

        worker.run(Job::Histogram {
            min: 1,
            max: 100,
            k: 1,
        });
        match wait_for_result(&worker) {
            Event::Histogram { hist, sampled, .. } => {
                assert!(!sampled);
                assert_eq!(hist[2], 50);
                assert_eq!(hist[4], 50);
            }
            _ => panic!("expected k=1 histogram"),
        }

        // Dynamic k=2 over raw gaps: every 2+4 or 4+2 sums to 6
        worker.run(Job::Histogram {
            min: 1,
            max: 99,
            k: 2,
        });
        match wait_for_result(&worker) {
            Event::Histogram { hist, sampled, .. } => {
                assert!(!sampled);
                assert_eq!(hist[6], 99);
            }
            _ => panic!("expected k=2 histogram"),
        }

        worker.run(Job::Spectral {
            min: 1,
            max: 100,
            k: 1,
            cols: 4,
            max_gap: 10,
            even_only: true,
            log_scale: false,
        });
        match wait_for_result(&worker) {
            Event::Spectral { heatmap, key, .. } => {
                assert_eq!(key.k, 1);
                assert_eq!(heatmap.cols, 4);
                assert_eq!(heatmap.col_totals.iter().sum::<u64>(), 100);
            }
            _ => panic!("expected Spectral"),
        }

        let _ = std::fs::remove_file(&gaps_path);
        let _ = std::fs::remove_file(&index_path);
    }

    #[test]
    fn test_clear_cache_aborts_large_job_mid_computation() {
        let worker = Worker::spawn(|| {});
        worker.run(Job::Frames {
            min: 1,
            max: 10_000_000,
            k: 2,
            count: 300,
        });

        let wait_busy_deadline = Instant::now() + Duration::from_secs(5);
        while !worker.is_busy() {
            assert!(
                Instant::now() < wait_busy_deadline,
                "worker never became busy"
            );
            std::thread::sleep(Duration::from_millis(1));
        }

        let abort_start = Instant::now();
        worker.clear_cache();
        worker.run(Job::Histogram {
            min: 1,
            max: 9,
            k: 1,
        });

        match wait_for_result(&worker) {
            Event::Histogram { hist, .. } => {
                let elapsed = abort_start.elapsed();
                assert_eq!(&hist[..7], &[0, 1, 4, 0, 3, 0, 1]);
                assert!(
                    elapsed < Duration::from_millis(1000),
                    "abort + follow-up took too long: {elapsed:?}"
                );
            }
            _ => panic!("expected histogram after abort"),
        }
    }
}
