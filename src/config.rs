// ============================================================================
// Generator Configuration
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
            output_path: "primes.parquet".into(),
            block_size: 10_000_000,
        }
    }
}
