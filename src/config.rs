use std::path::{Path, PathBuf};

// ============================================================================
// Prime Generator Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct Config {
    pub limit: usize,
    pub output_path: String,
    pub block_size: usize,
}

impl Config {
    pub fn from_args(args: &[String]) -> Self {
        Self {
            limit: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000_000),
            output_path: default_primes_path().to_string_lossy().into_owned(),
            block_size: 10_000_000,
        }
    }
}

// ============================================================================
// Analyzer CLI Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct AnalyzeConfig {
    pub k: usize,
    pub min_idx: u64,
    pub max_idx: u64,
    pub file_path: String,
    pub force: bool,
}

impl AnalyzeConfig {
    pub fn from_args(args: &[String]) -> Self {
        let force = args.iter().any(|a| a == "--force" || a == "-f");
        let positional: Vec<String> = args
            .iter()
            .skip(1)
            .filter(|a| *a != "--force" && *a != "-f")
            .cloned()
            .collect();

        Self {
            k: positional.get(0).and_then(|s| s.parse().ok()).unwrap_or(2),
            min_idx: positional.get(1).and_then(|s| s.parse().ok()).unwrap_or(1),
            max_idx: positional.get(2).and_then(|s| s.parse().ok()).unwrap_or(u64::MAX),
            file_path: positional.get(3).cloned().unwrap_or_else(|| "primes.parquet".into()),
            force,
        }
    }

    /// Derives the expected gaps database path: `gaps{k}.parquet` or `gaps.parquet`.
    pub fn gaps_path(&self) -> String {
        let filename = format!("gaps{}.parquet", self.k);
        let path = Path::new(&self.file_path);
        if let Some(parent) = path.parent() {
            if parent != Path::new("") {
                let target = parent.join(&filename);
                if target.exists() {
                    return target.to_string_lossy().into_owned();
                }
            }
        }
        if Path::new(&filename).exists() {
            filename
        } else if Path::new("gaps.parquet").exists() {
            "gaps.parquet".into()
        } else {
            filename
        }
    }
}

// ============================================================================
// File Path Utilities
// ============================================================================

pub fn app_data_dir() -> PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jumpchamp");
    let _ = std::fs::create_dir_all(&base);
    base
}

pub fn default_gaps_path(k: usize) -> PathBuf {
    app_data_dir().join(format!("gaps{}.parquet", k))
}

pub fn default_primes_path() -> PathBuf {
    app_data_dir().join("primes.parquet")
}
