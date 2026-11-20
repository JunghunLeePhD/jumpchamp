//! Offline builders for `gaps.bin` and `block_index.bin`.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::engine::Progress;
use crate::sieve::prime_blocks_up_to;

use super::format::{BlockHistogram, StorageError};
use super::raw::RawGaps;

const PARALLEL_BATCH_BLOCKS: usize = 64;
const SIEVE_BLOCK_SIZE: usize = 1_000_000;

/// Summary metadata produced by [`build_block_index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexBuildSummary {
    pub total_gaps: u64,
    pub block_count: usize,
    pub max_gap_seen: u16,
    pub overflow_gaps: u64,
    pub output_bytes: u64,
}

/// Summary metadata produced by [`generate_gaps_file`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapsBuildSummary {
    pub prime_limit: usize,
    pub total_primes: u64,
    pub total_gaps: u64,
    pub max_gap_seen: u16,
    pub output_bytes: u64,
}

/// Scans memory-mapped `gaps_path` (`u16` gaps) in parallel batches and atomically writes `index_path`.
pub fn build_block_index(
    gaps_path: impl AsRef<Path>,
    index_path: impl AsRef<Path>,
    block_size: u64,
    mut on_block: impl FnMut(usize, usize),
) -> Result<IndexBuildSummary, StorageError> {
    if block_size == 0 {
        return Err(StorageError::InvalidArgument("block_size must be >= 1"));
    }

    let raw = RawGaps::open(gaps_path)?;
    let gaps = raw.as_slice();
    let chunk_len = usize::try_from(block_size)
        .map_err(|_| StorageError::InvalidArgument("block_size exceeds usize"))?;
    let total_blocks = gaps.len().div_ceil(chunk_len);

    let index_path = index_path.as_ref();
    let tmp_path = temp_sibling(index_path);

    let mut all_blocks = Vec::with_capacity(total_blocks);
    let mut max_gap_seen = 0u16;
    let mut overflow_gaps = 0u64;
    let mut done = 0usize;

    let batch_elems = chunk_len.saturating_mul(PARALLEL_BATCH_BLOCKS);
    for batch in gaps.chunks(batch_elems.max(chunk_len)) {
        let chunk_results: Vec<(BlockHistogram, u16, u64)> = batch
            .par_chunks(chunk_len)
            .map(|slice| {
                let mut hist = BlockHistogram::zeroed();
                let mut local_max = 0u16;
                let mut local_overflow = 0u64;
                for &g in slice {
                    local_max = local_max.max(g);
                    if !hist.record(g) {
                        local_overflow += 1;
                    }
                }
                (hist, local_max, local_overflow)
            })
            .collect();

        for (hist, local_max, local_overflow) in chunk_results {
            all_blocks.push(hist);
            max_gap_seen = max_gap_seen.max(local_max);
            overflow_gaps += local_overflow;
            done += 1;
        }
        on_block(done, total_blocks);
    }

    let bytes: &[u8] = bytemuck::cast_slice(&all_blocks);
    {
        let mut writer = BufWriter::new(File::create(&tmp_path)?);
        writer.write_all(bytes)?;
        writer.flush()?;
    }
    fs::rename(&tmp_path, index_path)?;

    Ok(IndexBuildSummary {
        total_gaps: gaps.len() as u64,
        block_count: all_blocks.len(),
        max_gap_seen,
        overflow_gaps,
        output_bytes: bytes.len() as u64,
    })
}

/// Sieves all primes `<= prime_limit` and atomically writes consecutive 1-step `u16` gaps to `output_path`.
pub fn generate_gaps_file(
    prime_limit: usize,
    output_path: impl AsRef<Path>,
    mut on_block: impl FnMut(Progress),
) -> Result<GapsBuildSummary, StorageError> {
    if prime_limit < 3 {
        return Err(StorageError::InvalidArgument(
            "prime_limit must be >= 3 to produce at least one prime gap",
        ));
    }

    let output_path = output_path.as_ref();
    let tmp_path = temp_sibling(output_path);
    let (total_blocks, stream) = prime_blocks_up_to(prime_limit, SIEVE_BLOCK_SIZE);

    let mut writer = BufWriter::with_capacity(1 << 20, File::create(&tmp_path)?);
    let mut prev_prime: Option<u64> = None;
    let mut total_primes = 0u64;
    let mut total_gaps = 0u64;
    let mut max_gap_seen = 0u16;
    let mut buf: Vec<u16> = Vec::with_capacity(128 * 1024);

    for (idx, block) in stream.enumerate() {
        buf.clear();
        for p in block {
            total_primes += 1;
            if let Some(prev) = prev_prime {
                let gap = u16::try_from(p - prev)
                    .map_err(|_| StorageError::InvalidArgument("prime gap exceeds u16::MAX"))?;
                max_gap_seen = max_gap_seen.max(gap);
                buf.push(gap);
                total_gaps += 1;
            }
            prev_prime = Some(p);
        }
        if !buf.is_empty() {
            writer.write_all(bytemuck::cast_slice(&buf))?;
        }
        on_block(Progress {
            done: idx + 1,
            total: total_blocks,
        });
    }

    writer.flush()?;
    drop(writer);
    fs::rename(&tmp_path, output_path)?;

    Ok(GapsBuildSummary {
        prime_limit,
        total_primes,
        total_gaps,
        max_gap_seen,
        output_bytes: total_gaps * std::mem::size_of::<u16>() as u64,
    })
}

fn temp_sibling(target: &Path) -> PathBuf {
    let mut os = target.as_os_str().to_os_string();
    os.push(".tmp");
    PathBuf::from(os)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::sieve::small_primes;

    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jc_bld_{}_{}_{}", std::process::id(), id, name))
    }

    #[test]
    fn test_generate_gaps_file_matches_small_primes() {
        let gaps_path = temp_path("gaps.bin");
        let summary = generate_gaps_file(50_000, &gaps_path, |_| {}).unwrap();

        let primes = small_primes(50_000);
        assert_eq!(summary.total_primes, primes.len() as u64);
        assert_eq!(summary.total_gaps, primes.len() as u64 - 1);

        let raw = RawGaps::open(&gaps_path).unwrap();
        let expected: Vec<u16> = primes.windows(2).map(|w| (w[1] - w[0]) as u16).collect();
        assert_eq!(raw.as_slice(), expected.as_slice());

        let _ = std::fs::remove_file(&gaps_path);
    }
}
