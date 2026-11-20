//! `DistributionEngine` — zero-copy memory-mapped Tier-2 block histogram query engine.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

use crate::cancel::CancelToken;
use crate::engine::{frame_step, histogram, FrameSet, Histogram, Progress};

use super::format::{BlockHistogram, StorageError, BLOCK_SIZE, HISTOGRAM_BINS};
use super::raw::RawGaps;

const RAW_SLICE_POLL_CHUNK: usize = 250_000;
const BLOCK_POLL_STRIDE: usize = 1_024;

/// Sub-millisecond gap distribution engine backed by a memory-mapped `block_index.bin`.
#[derive(Debug)]
pub struct DistributionEngine {
    mmap: Mmap,
    block_size: u64,
}

impl DistributionEngine {
    /// Memory-maps `block_index.bin` using the standard `BLOCK_SIZE = 10_000_000`.
    ///
    /// Performs zero parsing or decompression on startup.
    pub fn load(index_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Self::load_with_block_size(index_path, BLOCK_SIZE)
    }

    /// Memory-maps an index built with a custom `block_size` (primarily for testing).
    pub fn load_with_block_size(
        index_path: impl AsRef<Path>,
        block_size: u64,
    ) -> Result<Self, StorageError> {
        if block_size == 0 {
            return Err(StorageError::InvalidArgument("block_size must be >= 1"));
        }
        let file = File::open(index_path)?;
        let len = file.metadata()?.len();
        if len == 0 {
            return Err(StorageError::EmptyFile {
                kind: "block_index.bin",
            });
        }
        let elem_size = std::mem::size_of::<BlockHistogram>();
        if !len.is_multiple_of(elem_size as u64) {
            return Err(StorageError::InvalidFileSize {
                kind: "block_index.bin",
                len,
                element_size: elem_size,
            });
        }

        // SAFETY: File length is validated as a non-zero multiple of `BlockHistogram` (2,400 bytes)
        // and callers treat the index file as immutable while mapped.
        let mmap = unsafe { Mmap::map(&file)? };
        let _: &[BlockHistogram] = bytemuck::try_cast_slice(&mmap)?;
        Ok(Self { mmap, block_size })
    }

    /// Zero-copy slice of all pre-aggregated [`BlockHistogram`] entries.
    pub fn blocks(&self) -> &[BlockHistogram] {
        bytemuck::cast_slice(&self.mmap)
    }

    /// Number of indexed blocks in `block_index.bin`.
    pub fn block_count(&self) -> usize {
        self.blocks().len()
    }

    /// Number of prime gaps covered by each full block (`L`).
    pub fn block_size(&self) -> u64 {
        self.block_size
    }

    /// Total number of gaps represented across all indexed blocks.
    pub fn max_indexed_primes(&self) -> u64 {
        let blocks = self.blocks();
        if blocks.is_empty() {
            return 0;
        }
        ((blocks.len() - 1) as u64) * self.block_size + blocks.last().unwrap().total()
    }

    /// Aggregates gap counts across blocks spanning 0-based indices `[start_idx, end_idx]`.
    ///
    /// Implements `P_[A, B](g) = sum_{m = floor(A/L)}^{floor(B/L)} H_m(g)`.
    pub fn query_distribution(&self, start_idx: u64, end_idx: u64) -> [u64; HISTOGRAM_BINS] {
        self.query_distribution_inner(start_idx, end_idx, None)
            .unwrap_or([0u64; HISTOGRAM_BINS])
    }

    fn query_distribution_inner(
        &self,
        start_idx: u64,
        end_idx: u64,
        cancel: Option<&CancelToken>,
    ) -> Option<[u64; HISTOGRAM_BINS]> {
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return None;
        }
        let mut totals = [0u64; HISTOGRAM_BINS];
        let blocks = self.blocks();
        if blocks.is_empty() || start_idx > end_idx {
            return Some(totals);
        }

        let m_start = (start_idx / self.block_size) as usize;
        if m_start >= blocks.len() {
            return Some(totals);
        }
        let m_end = ((end_idx / self.block_size) as usize).min(blocks.len() - 1);

        for (i, block) in blocks[m_start..=m_end].iter().enumerate() {
            if i.is_multiple_of(BLOCK_POLL_STRIDE) && cancel.is_some_and(CancelToken::is_cancelled)
            {
                return None;
            }
            add_block(&mut totals, block);
        }
        Some(totals)
    }

    /// Computes exact 1-step distribution over `[start_idx, end_idx]` (0-based inclusive).
    ///
    /// When `raw` is provided and covers `[start_idx, end_idx]`, partial head/tail blocks are
    /// evaluated directly from `raw` while complete interior blocks use the O(1) index.
    pub fn query_distribution_exact(
        &self,
        start_idx: u64,
        end_idx: u64,
        raw: Option<&RawGaps>,
    ) -> [u64; HISTOGRAM_BINS] {
        self.query_distribution_exact_inner(start_idx, end_idx, raw, None)
            .unwrap_or([0u64; HISTOGRAM_BINS])
    }

    /// Cancellable variant of [`DistributionEngine::query_distribution_exact`].
    pub fn query_distribution_exact_cancellable(
        &self,
        start_idx: u64,
        end_idx: u64,
        raw: Option<&RawGaps>,
        cancel: &CancelToken,
    ) -> Option<[u64; HISTOGRAM_BINS]> {
        self.query_distribution_exact_inner(start_idx, end_idx, raw, Some(cancel))
    }

    fn query_distribution_exact_inner(
        &self,
        start_idx: u64,
        end_idx: u64,
        raw: Option<&RawGaps>,
        cancel: Option<&CancelToken>,
    ) -> Option<[u64; HISTOGRAM_BINS]> {
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return None;
        }
        let Some(raw) = raw else {
            return self.query_distribution_inner(start_idx, end_idx, cancel);
        };
        let gaps = raw.as_slice();
        if gaps.is_empty() || start_idx > end_idx {
            return Some([0u64; HISTOGRAM_BINS]);
        }
        let start = start_idx as usize;
        let end = (end_idx as usize).min(gaps.len() - 1);
        if start > end {
            return Some([0u64; HISTOGRAM_BINS]);
        }

        let blocks = self.blocks();
        let bsize = self.block_size as usize;
        let m_start = start / bsize;
        let m_end = end / bsize;

        let mut totals = [0u64; HISTOGRAM_BINS];
        if m_start >= blocks.len() || m_end >= blocks.len() {
            if !add_raw_slice_cancellable(&mut totals, &gaps[start..=end], cancel) {
                return None;
            }
            return Some(totals);
        }

        if m_start == m_end {
            let block_start = m_start * bsize;
            let block_end = ((m_start + 1) * bsize - 1).min(gaps.len() - 1);
            if start == block_start && end == block_end {
                add_block(&mut totals, &blocks[m_start]);
            } else if !add_raw_slice_cancellable(&mut totals, &gaps[start..=end], cancel) {
                return None;
            }
            return Some(totals);
        }

        let head_end = (m_start + 1) * bsize - 1;
        if start == m_start * bsize {
            add_block(&mut totals, &blocks[m_start]);
        } else if !add_raw_slice_cancellable(&mut totals, &gaps[start..=head_end], cancel) {
            return None;
        }

        for (i, block) in blocks[(m_start + 1)..m_end].iter().enumerate() {
            if i.is_multiple_of(BLOCK_POLL_STRIDE) && cancel.is_some_and(CancelToken::is_cancelled)
            {
                return None;
            }
            add_block(&mut totals, block);
        }

        let tail_start = m_end * bsize;
        let tail_full_end = ((m_end + 1) * bsize - 1).min(gaps.len() - 1);
        if end == tail_full_end {
            add_block(&mut totals, &blocks[m_end]);
        } else if !add_raw_slice_cancellable(&mut totals, &gaps[tail_start..=end], cancel) {
            return None;
        }

        Some(totals)
    }

    /// Convenience wrapper returning `query_distribution` as a `Histogram` (`Vec<u64>`).
    pub fn query_histogram(&self, start_idx: u64, end_idx: u64) -> Histogram {
        self.query_distribution(start_idx, end_idx).to_vec()
    }

    /// Builds `count` cumulative 1-step animation frames over 1-based `[min_1based, max_1based]`.
    ///
    /// Assigns each block in the spanned range to a single frame so no block is double-counted.
    pub fn build_frames(&self, min_1based: u64, max_1based: u64, count: usize) -> FrameSet {
        self.build_frames_inner(min_1based, max_1based, count, None)
            .expect("uncancellable frame build cannot abort")
    }

    fn build_frames_inner(
        &self,
        min_1based: u64,
        max_1based: u64,
        count: usize,
        cancel: Option<&CancelToken>,
    ) -> Option<FrameSet> {
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return None;
        }
        let count = count.max(1);
        let step = frame_step(min_1based, max_1based, count);
        let mut deltas: Vec<Histogram> = vec![vec![0u64; HISTOGRAM_BINS]; count];

        let blocks = self.blocks();
        if !blocks.is_empty() && min_1based <= max_1based {
            let start0 = min_1based.saturating_sub(1);
            let end0 = max_1based.saturating_sub(1);
            let m_start = (start0 / self.block_size) as usize;
            let m_end = ((end0 / self.block_size) as usize).min(blocks.len() - 1);

            if m_start < blocks.len() && m_start <= m_end {
                for (i, (m, block)) in blocks
                    .iter()
                    .enumerate()
                    .take(m_end + 1)
                    .skip(m_start)
                    .enumerate()
                {
                    if i.is_multiple_of(BLOCK_POLL_STRIDE)
                        && cancel.is_some_and(CancelToken::is_cancelled)
                    {
                        return None;
                    }
                    let b_start_1 = (m as u64) * self.block_size + 1;
                    let frame = ((b_start_1.saturating_sub(min_1based)) / step) as usize;
                    let target = &mut deltas[frame.min(count - 1)];
                    for (acc, &cnt) in target.iter_mut().zip(block.counts.iter()) {
                        *acc += cnt as u64;
                    }
                }
            }
        }

        Some(FrameSet::from_deltas(min_1based, max_1based, 1, &deltas))
    }

    /// Builds `count` exact cumulative frames using `raw` when available for sub-block precision.
    pub fn build_frames_exact(
        &self,
        min_1based: u64,
        max_1based: u64,
        count: usize,
        raw: Option<&RawGaps>,
    ) -> FrameSet {
        self.build_frames_exact_cancellable(
            min_1based,
            max_1based,
            count,
            raw,
            &CancelToken::new(),
            |_| true,
        )
        .expect("uncancellable frame build cannot abort")
    }

    /// Cancellable variant of [`DistributionEngine::build_frames_exact`].
    pub fn build_frames_exact_cancellable(
        &self,
        min_1based: u64,
        max_1based: u64,
        count: usize,
        raw: Option<&RawGaps>,
        cancel: &CancelToken,
        mut on_frame: impl FnMut(Progress) -> bool,
    ) -> Option<FrameSet> {
        if cancel.is_cancelled() {
            return None;
        }
        let Some(raw) = raw else {
            return self.build_frames_inner(min_1based, max_1based, count, Some(cancel));
        };
        let count = count.max(1);
        let step = frame_step(min_1based, max_1based, count);
        let mut deltas = Vec::with_capacity(count);
        for i in 0..count {
            if cancel.is_cancelled()
                || !on_frame(Progress {
                    done: i + 1,
                    total: count,
                })
            {
                return None;
            }
            let f_start = min_1based + i as u64 * step;
            let f_end = if i + 1 == count {
                max_1based
            } else {
                (f_start + step - 1).min(max_1based)
            };
            if f_start <= f_end && f_start <= max_1based {
                let dist = self.query_distribution_exact_cancellable(
                    f_start.saturating_sub(1),
                    f_end.saturating_sub(1),
                    Some(raw),
                    cancel,
                )?;
                deltas.push(histogram::trimmed(&dist));
            } else {
                deltas.push(Vec::new());
            }
        }
        Some(FrameSet::from_deltas(min_1based, max_1based, 1, &deltas))
    }
}

fn add_block(totals: &mut [u64; HISTOGRAM_BINS], block: &BlockHistogram) {
    for (acc, &cnt) in totals.iter_mut().zip(block.counts.iter()) {
        *acc += cnt as u64;
    }
}

fn add_raw_slice_cancellable(
    totals: &mut [u64; HISTOGRAM_BINS],
    slice: &[u16],
    cancel: Option<&CancelToken>,
) -> bool {
    for chunk in slice.chunks(RAW_SLICE_POLL_CHUNK) {
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return false;
        }
        add_raw_slice(totals, chunk);
    }
    !cancel.is_some_and(CancelToken::is_cancelled)
}

fn add_raw_slice(totals: &mut [u64; HISTOGRAM_BINS], slice: &[u16]) {
    for &g in slice {
        if let Some(slot) = totals.get_mut(g as usize) {
            *slot += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::storage::builder::build_block_index;

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jc_eng_{}_{}_{}", std::process::id(), id, name))
    }

    #[test]
    fn test_distribution_engine_block_and_exact_queries() {
        let gaps_path = temp_path("gaps.bin");
        let idx_path = temp_path("index.bin");

        // 25 gaps -> with block_size = 10 -> 3 blocks (10, 10, 5)
        let gaps: Vec<u16> = (0..25).map(|i| ((i % 4) as u16 + 1) * 2).collect();
        let mut f = File::create(&gaps_path).unwrap();
        f.write_all(bytemuck::cast_slice(&gaps)).unwrap();
        f.flush().unwrap();

        let summary = build_block_index(&gaps_path, &idx_path, 10, |_, _| {}).unwrap();
        assert_eq!(summary.block_count, 3);
        assert_eq!(summary.total_gaps, 25);

        let engine = DistributionEngine::load_with_block_size(&idx_path, 10).unwrap();
        assert_eq!(engine.block_count(), 3);
        assert_eq!(engine.max_indexed_primes(), 25);

        // Block-level query across blocks 0..=1 (20 elements)
        let dist = engine.query_distribution(3, 14);
        assert_eq!(dist.iter().sum::<u64>(), 20);

        // Exact query using raw + interior block
        let raw = RawGaps::open(&gaps_path).unwrap();
        let exact = engine.query_distribution_exact(3, 22, Some(&raw));
        assert_eq!(exact.iter().sum::<u64>(), 20);
        let mut expected = [0u64; HISTOGRAM_BINS];
        for &g in &gaps[3..=22] {
            expected[g as usize] += 1;
        }
        assert_eq!(exact, expected);

        // Frame building avoids double counting
        let frames = engine.build_frames(1, 25, 5);
        assert_eq!(frames.at(25).iter().sum::<u64>(), 25);

        let exact_frames = engine.build_frames_exact(1, 25, 5, Some(&raw));
        assert_eq!(exact_frames.at(25).iter().sum::<u64>(), 25);

        let cancel = CancelToken::new();
        assert_eq!(
            engine.query_distribution_exact_cancellable(3, 22, Some(&raw), &cancel),
            Some(expected)
        );
        cancel.cancel();
        assert!(engine
            .query_distribution_exact_cancellable(3, 22, Some(&raw), &cancel)
            .is_none());
        assert!(engine
            .build_frames_exact_cancellable(1, 25, 5, Some(&raw), &cancel, |_| true)
            .is_none());

        let _ = std::fs::remove_file(&gaps_path);
        let _ = std::fs::remove_file(&idx_path);
    }
}
