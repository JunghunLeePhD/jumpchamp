//! Offline prime gap binary generator: sieves primes `<= prime_limit` into `gaps.bin` (`u16`).
//!
//! Usage:
//!   generate_gaps [prime_limit] [gaps.bin]

use std::env;
use std::process;
use std::time::Instant;

use jumpchamp_core::storage::generate_gaps_file;

const DEFAULT_PRIME_LIMIT: usize = 10_000_000;

fn print_help() {
    println!(
        "\
generate_gaps — Sieve primes up to PRIME_LIMIT and write raw 1-step u16 gaps.bin

USAGE:
    generate_gaps [PRIME_LIMIT] [OUTPUT_PATH]

ARGS:
    PRIME_LIMIT    Upper prime value bound (default: {DEFAULT_PRIME_LIMIT})
    OUTPUT_PATH    Output binary gaps file (default: gaps.bin)

OPTIONS:
    -h, --help     Print this help message"
    );
}

struct CliArgs {
    prime_limit: usize,
    output_path: String,
}

fn parse_args() -> Result<Option<CliArgs>, String> {
    let mut positional = Vec::new();
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
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

    let prime_limit = match positional.first() {
        Some(raw) => raw
            .replace('_', "")
            .parse::<usize>()
            .map_err(|_| format!("Invalid prime_limit '{raw}': expected integer >= 3"))?,
        None => DEFAULT_PRIME_LIMIT,
    };
    if prime_limit < 3 {
        return Err("prime_limit must be >= 3".to_string());
    }

    let output_path = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| "gaps.bin".to_string());

    Ok(Some(CliArgs {
        prime_limit,
        output_path,
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
        "Generating raw 1-step u16 prime gaps up to {} -> '{}'...",
        args.prime_limit, args.output_path
    );

    let start = Instant::now();
    let mut last_pct = usize::MAX;
    let summary = match generate_gaps_file(args.prime_limit, &args.output_path, |p| {
        let pct = (p.done * 100).checked_div(p.total).unwrap_or(100);
        if pct != last_pct && (pct.is_multiple_of(20) || p.done == p.total) {
            last_pct = pct;
            println!("  Sieving: {}/{} blocks ({pct}%)", p.done, p.total);
        }
    }) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Generation failed: {err}");
            process::exit(1);
        }
    };

    let elapsed = start.elapsed().as_secs_f64();
    let mb = summary.output_bytes as f64 / (1024.0 * 1024.0);
    println!(
        "Done in {elapsed:.2}s: {} primes, {} gaps, max gap = {}, output = {} bytes ({mb:.2} MiB)",
        summary.total_primes, summary.total_gaps, summary.max_gap_seen, summary.output_bytes
    );
}
