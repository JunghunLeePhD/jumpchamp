//! 2D spectral gap density heatmap matrix across prime-index scales.

use crate::engine::FrameSet;

use super::engine::DistributionEngine;
use super::format::HISTOGRAM_BINS;
use super::raw::RawGaps;

/// Pure 2D matrix of gap densities across prime-index columns (X) and gap values (Y).
///
/// Cells are stored in row-major order (`row * cols + col`), where `row 0` corresponds
/// to the smallest gap in [`SpectralHeatmap::gap_values`].
#[derive(Debug, Clone)]
pub struct SpectralHeatmap {
    pub cols: usize,
    pub rows: usize,
    pub start_idx: u64,
    pub end_idx: u64,
    pub k: usize,
    pub log_scale: bool,
    pub even_only: bool,
    pub gap_values: Vec<u64>,
    pub col_ranges: Vec<(u64, u64)>,
    /// Normalized per-column intensity in `[0.0, 1.0]`.
    pub intensities: Vec<f32>,
    /// Raw gap counts per cell (`row * cols + col`).
    pub counts: Vec<u64>,
    /// Total gaps in each column.
    pub col_totals: Vec<u64>,
}

impl SpectralHeatmap {
    /// Returns `(count, intensity)` at `(row, col)` if in bounds.
    pub fn cell(&self, row: usize, col: usize) -> Option<(u64, f32)> {
        if row < self.rows && col < self.cols {
            let idx = row * self.cols + col;
            Some((self.counts[idx], self.intensities[idx]))
        } else {
            None
        }
    }

    /// Builds a 1-step spectral heatmap directly from memory-mapped block histograms.
    pub fn from_index(
        engine: &DistributionEngine,
        start_idx_1based: u64,
        end_idx_1based: u64,
        max_cols: usize,
        max_gap: u64,
        even_only: bool,
        log_scale: bool,
    ) -> Self {
        let gap_values = build_gap_values(max_gap, even_only);
        let rows = gap_values.len();
        let blocks = engine.blocks();

        if blocks.is_empty() || start_idx_1based > end_idx_1based {
            return Self::empty(
                start_idx_1based,
                end_idx_1based,
                1,
                log_scale,
                even_only,
                gap_values,
            );
        }

        let bsize = engine.block_size();
        let start0 = start_idx_1based.saturating_sub(1);
        let end0 = end_idx_1based.saturating_sub(1);
        let m_start = (start0 / bsize) as usize;
        if m_start >= blocks.len() {
            return Self::empty(
                start_idx_1based,
                end_idx_1based,
                1,
                log_scale,
                even_only,
                gap_values,
            );
        }
        let m_end = ((end0 / bsize) as usize).min(blocks.len() - 1);
        let num_blocks = m_end - m_start + 1;
        let cols = num_blocks.min(max_cols.max(1));
        let max_primes = engine.max_indexed_primes();

        let mut col_ranges = Vec::with_capacity(cols);
        let mut counts = vec![0u64; rows * cols];
        let mut col_totals = vec![0u64; cols];

        for col in 0..cols {
            let b0 = m_start + (col * num_blocks) / cols;
            let b1 = (m_start + ((col + 1) * num_blocks) / cols).saturating_sub(1);
            let b1 = b1.max(b0).min(m_end);

            let range_lo = (b0 as u64) * bsize + 1;
            let range_hi = (((b1 as u64) + 1) * bsize).min(max_primes);
            col_ranges.push((range_lo, range_hi.max(range_lo)));

            let mut agg = [0u64; HISTOGRAM_BINS];
            for block in &blocks[b0..=b1] {
                for (acc, &c) in agg.iter_mut().zip(block.counts.iter()) {
                    *acc += c as u64;
                }
            }
            col_totals[col] = agg.iter().sum();

            for (row, &gap) in gap_values.iter().enumerate() {
                counts[row * cols + col] = agg.get(gap as usize).copied().unwrap_or(0);
            }
        }

        let intensities = normalize_columns(cols, rows, &counts, log_scale);
        Self {
            cols,
            rows,
            start_idx: start_idx_1based,
            end_idx: end_idx_1based,
            k: 1,
            log_scale,
            even_only,
            gap_values,
            col_ranges,
            intensities,
            counts,
            col_totals,
        }
    }

    /// Builds a `k`-step spectral heatmap over `[start_idx_1based, end_idx_1based]` using `RawGaps`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_raw_gaps(
        raw: &RawGaps,
        start_idx_1based: u64,
        end_idx_1based: u64,
        k: usize,
        max_cols: usize,
        max_gap: u64,
        even_only: bool,
        log_scale: bool,
        max_samples_per_col: usize,
    ) -> Self {
        let gap_values = build_gap_values(max_gap, even_only);
        let rows = gap_values.len();
        if k == 0 || start_idx_1based > end_idx_1based || raw.is_empty() {
            return Self::empty(
                start_idx_1based,
                end_idx_1based,
                k,
                log_scale,
                even_only,
                gap_values,
            );
        }

        let start0 = (start_idx_1based.saturating_sub(1) as usize).min(raw.len());
        let end0_excl = (end_idx_1based as usize).min(raw.len());
        let span = end0_excl.saturating_sub(start0);
        if span < k {
            return Self::empty(
                start_idx_1based,
                end_idx_1based,
                k,
                log_scale,
                even_only,
                gap_values,
            );
        }

        let cols = span.min(max_cols.max(1));
        let mut col_ranges = Vec::with_capacity(cols);
        let mut counts = vec![0u64; rows * cols];
        let mut col_totals = vec![0u64; cols];

        for col in 0..cols {
            let c_start = start0 + (col * span) / cols;
            let c_end = start0 + ((col + 1) * span) / cols;
            let c_end = c_end.max(c_start + 1).min(end0_excl);

            let (hist, _) = raw.k_step_histogram_sampled(c_start, c_end, k, max_samples_per_col);
            col_ranges.push((c_start as u64 + 1, c_end as u64));
            col_totals[col] = hist.iter().sum();

            for (row, &gap) in gap_values.iter().enumerate() {
                counts[row * cols + col] = hist.get(gap as usize).copied().unwrap_or(0);
            }
        }

        let intensities = normalize_columns(cols, rows, &counts, log_scale);
        Self {
            cols,
            rows,
            start_idx: start_idx_1based,
            end_idx: end_idx_1based,
            k,
            log_scale,
            even_only,
            gap_values,
            col_ranges,
            intensities,
            counts,
            col_totals,
        }
    }

    /// Builds a 2D spectral heatmap from an in-memory cumulative [`FrameSet`].
    pub fn from_frame_set(
        frames: &FrameSet,
        max_gap: u64,
        even_only: bool,
        log_scale: bool,
    ) -> Self {
        let gap_values = build_gap_values(max_gap, even_only);
        let rows = gap_values.len();
        let slices = frames.frame_slices();
        let cols = slices.len();

        if cols == 0 {
            return Self::empty(
                frames.min, frames.max, frames.k, log_scale, even_only, gap_values,
            );
        }

        let mut col_ranges = Vec::with_capacity(cols);
        let mut counts = vec![0u64; rows * cols];
        let mut col_totals = vec![0u64; cols];

        for col in 0..cols {
            let cur = &slices[col];
            let prev: &[u64] = if col == 0 { &[] } else { &slices[col - 1] };

            let lo = frames.min + col as u64 * frames.step;
            let hi = if col + 1 == cols {
                frames.max
            } else {
                (lo + frames.step - 1).min(frames.max)
            };
            col_ranges.push((lo.min(frames.max), hi.max(lo.min(frames.max))));

            let mut total = 0u64;
            for (g, &c) in cur.iter().enumerate() {
                let p = prev.get(g).copied().unwrap_or(0);
                total += c.saturating_sub(p);
            }
            col_totals[col] = total;

            for (row, &gap) in gap_values.iter().enumerate() {
                let g = gap as usize;
                let cur_val = cur.get(g).copied().unwrap_or(0);
                let prev_val = prev.get(g).copied().unwrap_or(0);
                counts[row * cols + col] = cur_val.saturating_sub(prev_val);
            }
        }

        let intensities = normalize_columns(cols, rows, &counts, log_scale);
        Self {
            cols,
            rows,
            start_idx: frames.min,
            end_idx: frames.max,
            k: frames.k,
            log_scale,
            even_only,
            gap_values,
            col_ranges,
            intensities,
            counts,
            col_totals,
        }
    }

    fn empty(
        start_idx: u64,
        end_idx: u64,
        k: usize,
        log_scale: bool,
        even_only: bool,
        gap_values: Vec<u64>,
    ) -> Self {
        let rows = gap_values.len();
        Self {
            cols: 0,
            rows,
            start_idx,
            end_idx,
            k,
            log_scale,
            even_only,
            gap_values,
            col_ranges: Vec::new(),
            intensities: Vec::new(),
            counts: Vec::new(),
            col_totals: Vec::new(),
        }
    }
}

fn build_gap_values(max_gap: u64, even_only: bool) -> Vec<u64> {
    if even_only {
        (2..=max_gap.max(2)).step_by(2).collect()
    } else {
        (1..=max_gap.max(1)).collect()
    }
}

fn normalize_columns(cols: usize, rows: usize, counts: &[u64], log_scale: bool) -> Vec<f32> {
    let mut intensities = vec![0.0f32; rows * cols];
    for col in 0..cols {
        let col_max = (0..rows).map(|r| counts[r * cols + col]).max().unwrap_or(0);
        if col_max == 0 {
            continue;
        }
        let denom = if log_scale {
            (1.0 + col_max as f64).ln()
        } else {
            col_max as f64
        };
        for row in 0..rows {
            let count = counts[row * cols + col];
            if count == 0 {
                continue;
            }
            let num = if log_scale {
                (1.0 + count as f64).ln()
            } else {
                count as f64
            };
            intensities[row * cols + col] = (num / denom).clamp(0.0, 1.0) as f32;
        }
    }
    intensities
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::storage::builder::build_block_index;

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jc_spec_{}_{}_{}", std::process::id(), id, name))
    }

    #[test]
    fn test_spectral_heatmap_from_index_and_raw() {
        let gaps_path = temp_path("gaps.bin");
        let idx_path = temp_path("index.bin");

        let mut gaps = Vec::with_capacity(40);
        for b in 0..4 {
            for _ in 0..10 {
                gaps.push(((b + 1) * 2) as u16);
            }
        }
        let mut f = File::create(&gaps_path).unwrap();
        f.write_all(bytemuck::cast_slice(&gaps)).unwrap();
        f.flush().unwrap();

        build_block_index(&gaps_path, &idx_path, 10, |_, _| {}).unwrap();
        let engine = DistributionEngine::load_with_block_size(&idx_path, 10).unwrap();

        let hm = SpectralHeatmap::from_index(&engine, 1, 40, 16, 10, true, false);
        assert_eq!(hm.cols, 4);
        assert_eq!(hm.gap_values, vec![2, 4, 6, 8, 10]);
        assert_eq!(hm.rows, 5);
        for c in 0..4 {
            assert_eq!(hm.cell(c, c), Some((10, 1.0)));
            assert_eq!(hm.col_totals[c], 10);
        }

        let raw = RawGaps::open(&gaps_path).unwrap();
        let raw_hm = SpectralHeatmap::from_raw_gaps(&raw, 1, 40, 2, 4, 20, true, true, 10_000);
        assert_eq!(raw_hm.cols, 4);
        assert_eq!(raw_hm.k, 2);
        assert!(raw_hm.intensities.iter().all(|&v| (0.0..=1.0).contains(&v)));

        let _ = std::fs::remove_file(&gaps_path);
        let _ = std::fs::remove_file(&idx_path);
    }

    #[test]
    fn test_spectral_heatmap_from_frame_set() {
        let frames = FrameSet::build(1, 200, 1, 4, |_| true).unwrap();
        let hm = SpectralHeatmap::from_frame_set(&frames, 20, false, true);
        assert_eq!(hm.cols, 4);
        assert_eq!(hm.rows, 20);
        assert_eq!(hm.col_totals.iter().sum::<u64>(), 200);
        assert!(hm.intensities.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }
}
