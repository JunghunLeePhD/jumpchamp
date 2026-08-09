use std::path::PathBuf;

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


