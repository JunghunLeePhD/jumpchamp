//! Offline indexing CLI: scans raw `gaps.bin` (`u16`) and builds `block_index.bin`.
//!
//! Usage:
//!   generate_index [gaps.bin] [block_index.bin] [--block-size N]

use std::env;
use std::process;
use std::time::Instant;

use jumpchamp_core::storage::{build_block_index, BlockHistogram, BLOCK_SIZE};

fn print_help() {
    println!(
        "\
generate_index — Build two-tier BlockHistogram index from raw u16 gaps.bin

USAGE:
    generate_index [GAPS_PATH] [INDEX_PATH] [--block-size N]

ARGS:
    GAPS_PATH        Input raw gaps binary file (default: gaps.bin)
    INDEX_PATH       Output block index file (default: block_index.bin)

OPTIONS:
    --block-size N   Primes per indexed block L (default: {BLOCK_SIZE})
    -h, --help       Print this help message"
    );
}

struct CliArgs {
    gaps_path: String,
    index_path: String,
    block_size: u64,
}

fn parse_u64(raw: &str, flag: &str) -> Result<u64, String> {
    let cleaned = raw.replace('_', "");
    let val: u64 = cleaned
        .parse()
        .map_err(|_| format!("Invalid {flag} value '{raw}': expected positive integer"))?;
    if val == 0 {
        return Err(format!("Invalid {flag} value '{raw}': must be >= 1"));
    }
    Ok(val)
}

fn parse_args() -> Result<Option<CliArgs>, String> {
    let mut positional = Vec::new();
    let mut block_size = BLOCK_SIZE;
    let mut iter = env::args().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--block-size" => {
                let val = iter
                    .next()
                    .ok_or_else(|| "Missing value after --block-size".to_string())?;
                block_size = parse_u64(&val, "--block-size")?;
            }
            _ if arg.starts_with("--block-size=") => {
                let val = &arg["--block-size=".len()..];
                block_size = parse_u64(val, "--block-size")?;
            }
            _ if arg.starts_with('-') => return Err(format!("Unknown option: {arg}")),
            _ => positional.push(arg),
        }
    }

    if positional.len() > 2 {
        return Err(format!(
            "Expected at most 2 positional arguments, found {}",
            positional.len()
        ));
    }

    let gaps_path = positional
        .first()
        .cloned()
        .unwrap_or_else(|| "gaps.bin".to_string());
    let index_path = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| "block_index.bin".to_string());

    Ok(Some(CliArgs {
        gaps_path,
        index_path,
        block_size,
    }))
}

fn main() {
    let args = match parse_args() {
        Ok(Some(args)) => args,
        Ok(None) => {
            print_help();
            return;
        }
        Err(err) => {
            eprintln!("Error: {err}");
            eprintln!("Run with --help for usage.");
            process::exit(1);
        }
    };

    println!(
        "Indexing '{}' -> '{}' (block_size = {}, {} bytes/block)...",
        args.gaps_path,
        args.index_path,
        args.block_size,
        std::mem::size_of::<BlockHistogram>()
    );

    let start = Instant::now();
    let mut last_pct = usize::MAX;
    let summary = match build_block_index(
        &args.gaps_path,
        &args.index_path,
        args.block_size,
        |done, total| {
            let pct = (done * 100).checked_div(total).unwrap_or(100);
            if pct != last_pct && (pct.is_multiple_of(10) || done == total) {
                last_pct = pct;
                println!("  Progress: {done}/{total} blocks ({pct}%)");
            }
        },
    ) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Indexing failed: {err}");
            process::exit(1);
        }
    };

    let elapsed = start.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        summary.total_gaps as f64 / elapsed
    } else {
        summary.total_gaps as f64
    };
    let mb = summary.output_bytes as f64 / (1024.0 * 1024.0);

    println!(
        "Done in {elapsed:.2}s ({rate:.0} gaps/s): {} blocks, {} gaps, max gap = {}, overflow = {}, output = {} bytes ({mb:.2} MiB)",
        summary.block_count,
        summary.total_gaps,
        summary.max_gap_seen,
        summary.overflow_gaps,
        summary.output_bytes
    );
}
