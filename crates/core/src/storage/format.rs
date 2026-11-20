//! Binary storage format definitions: constants, `BlockHistogram`, and `StorageError`.

use std::fmt;

/// Number of primes/gaps per indexed block (`L = 10^7`).
pub const BLOCK_SIZE: u64 = 10_000_000;

/// Number of histogram bins per block (`g in [0, 600)`), covering `g_n <= 540` up to `10^12`.
pub const HISTOGRAM_BINS: usize = 600;

/// Number of primes below `10^12` (`pi(10^12) = 37,607,912,018`).
pub const MAX_DATASET_PRIMES: u64 = 37_607_912_018;

/// Fixed-size block gap histogram (`2,400` bytes, zero-copy mappable).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockHistogram {
    pub counts: [u32; HISTOGRAM_BINS],
}

const _: () = assert!(std::mem::size_of::<BlockHistogram>() == 2_400);

impl Default for BlockHistogram {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl BlockHistogram {
    /// Returns an all-zero `BlockHistogram`.
    pub fn zeroed() -> Self {
        Self {
            counts: [0u32; HISTOGRAM_BINS],
        }
    }

    /// Records a single 1-step gap if `gap < HISTOGRAM_BINS`. Returns `false` if out of range.
    pub fn record(&mut self, gap: u16) -> bool {
        if let Some(slot) = self.counts.get_mut(gap as usize) {
            *slot += 1;
            true
        } else {
            false
        }
    }

    /// Total number of recorded gaps in this block.
    pub fn total(&self) -> u64 {
        self.counts.iter().map(|&c| c as u64).sum()
    }
}

/// Errors returned while opening, validating, or building binary gap storage files.
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    EmptyFile {
        kind: &'static str,
    },
    InvalidFileSize {
        kind: &'static str,
        len: u64,
        element_size: usize,
    },
    InvalidArgument(&'static str),
    Alignment(bytemuck::PodCastError),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "storage I/O error: {err}"),
            Self::EmptyFile { kind } => write!(f, "{kind} file is empty"),
            Self::InvalidFileSize {
                kind,
                len,
                element_size,
            } => write!(
                f,
                "{kind} file size ({len} bytes) is not a multiple of {element_size} bytes"
            ),
            Self::InvalidArgument(msg) => write!(f, "invalid storage argument: {msg}"),
            Self::Alignment(err) => write!(f, "zero-copy alignment cast failed: {err:?}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<bytemuck::PodCastError> for StorageError {
    fn from(err: bytemuck::PodCastError) -> Self {
        Self::Alignment(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_histogram_layout_and_record() {
        assert_eq!(std::mem::size_of::<BlockHistogram>(), 2_400);
        let mut h = BlockHistogram::zeroed();
        assert_eq!(h.total(), 0);
        assert!(h.record(2));
        assert!(h.record(2));
        assert!(h.record(599));
        assert!(!h.record(600));
        assert_eq!(h.counts[2], 2);
        assert_eq!(h.counts[599], 1);
        assert_eq!(h.total(), 3);
    }
}
