use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use jumpchamp_core::sieve::small_primes;
use jumpchamp_core::storage::{BlockHistogram, DistributionEngine, RawGaps, HISTOGRAM_BINS};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_path(name: &str) -> PathBuf {
    let id = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("jc_cli_{}_{}_{}", std::process::id(), id, name))
}

#[test]
fn test_generate_gaps_and_generate_index_cli_pipeline() {
    let gaps_path = temp_path("gaps.bin");
    let index_path = temp_path("block_index.bin");

    let gaps_out = Command::new(env!("CARGO_BIN_EXE_generate_gaps"))
        .arg("200_000")
        .arg(&gaps_path)
        .output()
        .expect("run generate_gaps");
    assert!(
        gaps_out.status.success(),
        "generate_gaps failed: {}",
        String::from_utf8_lossy(&gaps_out.stderr)
    );

    let index_out = Command::new(env!("CARGO_BIN_EXE_generate_index"))
        .arg(&gaps_path)
        .arg(&index_path)
        .arg("--block-size")
        .arg("5000")
        .output()
        .expect("run generate_index");
    assert!(
        index_out.status.success(),
        "generate_index failed: {}",
        String::from_utf8_lossy(&index_out.stderr)
    );

    let expected_primes = small_primes(200_000);
    let expected_gaps: Vec<u16> = expected_primes
        .windows(2)
        .map(|w| (w[1] - w[0]) as u16)
        .collect();
    let expected_blocks = expected_gaps.len().div_ceil(5_000);

    assert_eq!(
        std::fs::metadata(&gaps_path).unwrap().len(),
        (expected_gaps.len() * std::mem::size_of::<u16>()) as u64
    );
    assert_eq!(
        std::fs::metadata(&index_path).unwrap().len(),
        (expected_blocks * std::mem::size_of::<BlockHistogram>()) as u64
    );

    let raw = RawGaps::open(&gaps_path).unwrap();
    assert_eq!(raw.as_slice(), expected_gaps.as_slice());

    let engine = DistributionEngine::load_with_block_size(&index_path, 5_000).unwrap();
    assert_eq!(engine.block_count(), expected_blocks);
    assert_eq!(engine.max_indexed_primes(), expected_gaps.len() as u64);

    let dist = engine.query_distribution(0, expected_gaps.len() as u64 - 1);
    let mut naive = [0u64; HISTOGRAM_BINS];
    for &g in &expected_gaps {
        naive[g as usize] += 1;
    }
    assert_eq!(dist, naive);

    // Verify dynamic k=2 sliding window against direct primes
    let k2 = raw.k_step_gaps_slice(0, expected_gaps.len(), 2);
    let expected_k2: Vec<u64> = expected_primes
        .windows(3)
        .map(|w| (w[2] - w[0]) as u64)
        .collect();
    assert_eq!(k2, expected_k2);

    let _ = std::fs::remove_file(&gaps_path);
    let _ = std::fs::remove_file(&index_path);
}
