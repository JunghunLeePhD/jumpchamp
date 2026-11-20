//! Zero-copy memory-mapped `gaps.bin` (`[u16]`) reader and on-the-fly k-step evaluator.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;
use rayon::prelude::*;

use crate::cancel::CancelToken;
use crate::engine::{histogram, Histogram, Progress};

use super::format::StorageError;

const PARALLEL_THRESHOLD: usize = 250_000;
const CANCELLABLE_CHUNK: usize = 1_000_000;

/// Memory-mapped raw 1-step prime gaps (`u16`) backed by `memmap2` and `bytemuck`.
#[derive(Debug)]
pub struct RawGaps {
    mmap: Mmap,
}

impl RawGaps {
    /// Opens and memory-maps a raw `gaps.bin` file without copying or parsing.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        if len == 0 {
            return Err(StorageError::EmptyFile { kind: "gaps.bin" });
        }
        if !len.is_multiple_of(std::mem::size_of::<u16>() as u64) {
            return Err(StorageError::InvalidFileSize {
                kind: "gaps.bin",
                len,
                element_size: std::mem::size_of::<u16>(),
            });
        }

        // SAFETY: The file size is validated as a non-zero multiple of `u16` and callers treat
        // the backing `gaps.bin` file as immutable while mapped.
        let mmap = unsafe { Mmap::map(&file)? };
        let _: &[u16] = bytemuck::try_cast_slice(&mmap)?;
        Ok(Self { mmap })
    }

    /// Zero-copy slice view over all 1-step gaps (`g_n = p_{n+1} - p_n`).
    pub fn as_slice(&self) -> &[u16] {
        bytemuck::cast_slice(&self.mmap)
    }

    /// Total number of 1-step gaps stored in the file.
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Returns `true` if the file holds zero gaps.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// Safely clamped slice over 0-based gap indices `[start_idx, end_idx_exclusive)`.
    pub fn slice(&self, start_idx: usize, end_idx_exclusive: usize) -> &[u16] {
        let gaps = self.as_slice();
        let start = start_idx.min(gaps.len());
        let end = end_idx_exclusive.min(gaps.len()).max(start);
        &gaps[start..end]
    }

    /// Computes on-the-fly `k`-step gaps `g_n^(k) = sum_{i=0}^{k-1} g_{n+i}` over `[start_idx, end_idx_exclusive)`.
    pub fn k_step_gaps_slice(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
    ) -> Vec<u64> {
        let Some((start, valid_end)) = self.valid_k_bounds(start_idx, end_idx_exclusive, k) else {
            return Vec::new();
        };
        let gaps = self.as_slice();
        let window = &gaps[start..valid_end + k - 1];
        let mut out = Vec::with_capacity(valid_end - start);
        visit_k_step_window(window, k, |gap| out.push(gap));
        out
    }

    /// Computes an exact `k`-step gap histogram over 0-based indices `[start_idx, end_idx_exclusive)`.
    pub fn k_step_histogram(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
    ) -> Histogram {
        self.k_step_histogram_par_cancellable(start_idx, end_idx_exclusive, k, &CancelToken::new())
            .unwrap_or_else(histogram::empty)
    }

    fn k_step_histogram_par_cancellable(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
        cancel: &CancelToken,
    ) -> Option<Histogram> {
        if cancel.is_cancelled() {
            return None;
        }
        let Some((start, valid_end)) = self.valid_k_bounds(start_idx, end_idx_exclusive, k) else {
            return Some(histogram::empty());
        };
        let gaps = self.as_slice();
        let count = valid_end - start;
        if count < PARALLEL_THRESHOLD {
            let mut hist = histogram::empty();
            visit_k_step_window(&gaps[start..valid_end + k - 1], k, |gap| {
                histogram::record(&mut hist, gap);
            });
            return (!cancel.is_cancelled()).then_some(hist);
        }

        let chunks = count.div_ceil(PARALLEL_THRESHOLD);
        (0..chunks)
            .into_par_iter()
            .map(|c| {
                if cancel.is_cancelled() {
                    return Err(());
                }
                let c_start = start + c * PARALLEL_THRESHOLD;
                let c_end = (c_start + PARALLEL_THRESHOLD).min(valid_end);
                let mut local = histogram::empty();
                visit_k_step_window(&gaps[c_start..c_end + k - 1], k, |gap| {
                    histogram::record(&mut local, gap);
                });
                if cancel.is_cancelled() {
                    Err(())
                } else {
                    Ok(local)
                }
            })
            .try_reduce(histogram::empty, |mut acc, local| {
                if cancel.is_cancelled() {
                    return Err(());
                }
                histogram::add(&mut acc, &local);
                Ok(acc)
            })
            .ok()
    }

    /// Cancellable exact `k`-step gap histogram over `[start_idx, end_idx_exclusive)`.
    pub fn k_step_histogram_cancellable(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
        mut on_chunk: impl FnMut(Progress) -> bool,
    ) -> Option<Histogram> {
        let Some((start, valid_end)) = self.valid_k_bounds(start_idx, end_idx_exclusive, k) else {
            return on_chunk(Progress { done: 1, total: 1 }).then(histogram::empty);
        };
        let gaps = self.as_slice();
        let count = valid_end - start;
        let total = count.div_ceil(CANCELLABLE_CHUNK);
        let mut hist = histogram::empty();

        for chunk_idx in 0..total {
            if !on_chunk(Progress {
                done: chunk_idx + 1,
                total,
            }) {
                return None;
            }
            let c_start = start + chunk_idx * CANCELLABLE_CHUNK;
            let c_end = (c_start + CANCELLABLE_CHUNK).min(valid_end);
            visit_k_step_window(&gaps[c_start..c_end + k - 1], k, |gap| {
                histogram::record(&mut hist, gap);
            });
        }
        Some(hist)
    }

    /// Evaluates a `k`-step histogram, using evenly spaced parallel windows if the range exceeds `max_samples`.
    ///
    /// Returns `(histogram, was_sampled)`.
    pub fn k_step_histogram_sampled(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
        max_samples: usize,
    ) -> (Histogram, bool) {
        self.k_step_histogram_sampled_cancellable(
            start_idx,
            end_idx_exclusive,
            k,
            max_samples,
            &CancelToken::new(),
        )
        .unwrap_or_else(|| (histogram::empty(), false))
    }

    /// Cancellable variant of [`RawGaps::k_step_histogram_sampled`].
    ///
    /// Polls `cancel` at parallel chunk/window boundaries and returns `None` if aborted.
    pub fn k_step_histogram_sampled_cancellable(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
        max_samples: usize,
        cancel: &CancelToken,
    ) -> Option<(Histogram, bool)> {
        if cancel.is_cancelled() {
            return None;
        }
        let Some((start, valid_end)) = self.valid_k_bounds(start_idx, end_idx_exclusive, k) else {
            return Some((histogram::empty(), false));
        };
        let count = valid_end - start;
        let budget = max_samples.max(1);
        if count <= budget {
            return self
                .k_step_histogram_par_cancellable(start, valid_end, k, cancel)
                .map(|hist| (hist, false));
        }

        let windows = 200usize.min(budget).max(1);
        let per_window = (budget / windows).max(1);
        let gaps = self.as_slice();

        let hist = (0..windows)
            .into_par_iter()
            .map(|w| {
                if cancel.is_cancelled() {
                    return Err(());
                }
                let w_start = start + (w * count) / windows;
                let w_end = (w_start + per_window).min(valid_end);
                let mut local = histogram::empty();
                if w_start < w_end {
                    visit_k_step_window(&gaps[w_start..w_end + k - 1], k, |gap| {
                        histogram::record(&mut local, gap);
                    });
                }
                if cancel.is_cancelled() {
                    Err(())
                } else {
                    Ok(local)
                }
            })
            .try_reduce(histogram::empty, |mut acc, local| {
                if cancel.is_cancelled() {
                    return Err(());
                }
                histogram::add(&mut acc, &local);
                Ok(acc)
            })
            .ok()?;

        Some((hist, true))
    }

    fn valid_k_bounds(
        &self,
        start_idx: usize,
        end_idx_exclusive: usize,
        k: usize,
    ) -> Option<(usize, usize)> {
        if k == 0 {
            return None;
        }
        let len = self.len();
        if len < k {
            return None;
        }
        let max_valid_end = len - (k - 1);
        let start = start_idx.min(max_valid_end);
        let valid_end = end_idx_exclusive.min(max_valid_end);
        (start < valid_end).then_some((start, valid_end))
    }
}

/// Visits every `k`-step sliding sum in `window` (`len >= k`).
pub(crate) fn visit_k_step_window(window: &[u16], k: usize, mut visit: impl FnMut(u64)) {
    if k == 0 || window.len() < k {
        return;
    }
    let mut sum: u64 = window[..k].iter().map(|&g| g as u64).sum();
    visit(sum);
    for i in k..window.len() {
        sum = sum + window[i] as u64 - window[i - k] as u64;
        visit(sum);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jc_raw_{}_{}_{}", std::process::id(), id, name))
    }

    fn write_u16_file(path: &Path, values: &[u16]) {
        let mut f = File::create(path).unwrap();
        f.write_all(bytemuck::cast_slice(values)).unwrap();
        f.flush().unwrap();
    }

    #[test]
    fn test_open_empty_and_odd_size_errors() {
        let empty = temp_path("empty.bin");
        File::create(&empty).unwrap();
        assert!(matches!(
            RawGaps::open(&empty),
            Err(StorageError::EmptyFile { kind: "gaps.bin" })
        ));
        let _ = std::fs::remove_file(&empty);

        let odd = temp_path("odd.bin");
        std::fs::write(&odd, [1u8, 2, 3]).unwrap();
        assert!(matches!(
            RawGaps::open(&odd),
            Err(StorageError::InvalidFileSize {
                kind: "gaps.bin",
                len: 3,
                element_size: 2
            })
        ));
        let _ = std::fs::remove_file(&odd);
    }

    #[test]
    fn test_k_step_gaps_and_histogram_exact() {
        let path = temp_path("gaps.bin");
        // gaps from primes [2, 3, 5, 7, 11, 13, 17, 19]
        let gaps = [1u16, 2, 2, 4, 2, 4, 2];
        write_u16_file(&path, &gaps);

        let raw = RawGaps::open(&path).unwrap();
        assert_eq!(raw.len(), 7);
        assert_eq!(raw.slice(1, 4), &[2, 2, 4]);
        assert_eq!(raw.k_step_gaps_slice(0, 7, 1), vec![1, 2, 2, 4, 2, 4, 2]);
        assert_eq!(raw.k_step_gaps_slice(0, 7, 2), vec![3, 4, 6, 6, 6, 6]);
        assert_eq!(raw.k_step_gaps_slice(1, 5, 3), vec![8, 8, 10, 8]);

        let hist = raw.k_step_histogram(0, 7, 2);
        assert_eq!(hist[3], 1);
        assert_eq!(hist[4], 1);
        assert_eq!(hist[6], 4);
        assert_eq!(hist.iter().sum::<u64>(), 6);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_parallel_and_sampled_k_step_histogram() {
        let path = temp_path("large.bin");
        let gaps = vec![2u16; 300_000];
        write_u16_file(&path, &gaps);

        let raw = RawGaps::open(&path).unwrap();
        let exact = raw.k_step_histogram(0, gaps.len(), 3);
        assert_eq!(exact[6], 300_000 - 2);
        assert_eq!(exact.iter().sum::<u64>(), 299_998);

        let (sampled_hist, sampled) = raw.k_step_histogram_sampled(0, gaps.len(), 3, 20_000);
        assert!(sampled);
        assert_eq!(sampled_hist[6], sampled_hist.iter().sum::<u64>());
        assert!(sampled_hist[6] > 0);

        let cancelled = raw.k_step_histogram_cancellable(0, gaps.len(), 2, |_| false);
        assert!(cancelled.is_none());

        let token = CancelToken::new();
        assert_eq!(
            raw.k_step_histogram_sampled_cancellable(0, gaps.len(), 3, 20_000, &token),
            Some((sampled_hist, true))
        );
        assert_eq!(
            raw.k_step_histogram_sampled_cancellable(0, gaps.len(), 3, 500_000, &token),
            Some((exact, false))
        );
        token.cancel();
        assert!(raw
            .k_step_histogram_sampled_cancellable(0, gaps.len(), 3, 20_000, &token)
            .is_none());
        assert!(raw
            .k_step_histogram_sampled_cancellable(0, gaps.len(), 3, 500_000, &token)
            .is_none());

        let _ = std::fs::remove_file(&path);
    }
}
