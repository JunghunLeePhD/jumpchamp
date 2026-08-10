// ============================================================================
// AppState & Data Structures for GUI
// ============================================================================

use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    ByFrequency,
    ByGapSize,
}

#[derive(Debug, Clone)]

pub struct DatasetMetadata {
    pub total_rows: u64,
    pub unique_gaps: u64,
    pub min_gap: u16,
    pub max_gap: u16,
}

pub enum WorkerCommand {
    ComputeGaps {
        min_val: u64,
        max_val: u64,
        k: usize,
        top_min: usize,
        top_max: usize,
        sort_by: SortOrder,
    },
    Cancel,
}

pub enum WorkerResult {
    Metadata(DatasetMetadata),
    FrequencyData(Vec<(u64, u64)>),
    QueryLatency(f64),
    Progress(f32),
    Error(String),
}

pub struct AppState {
    // Controls
    pub k: usize,
    pub min_val: u64,
    pub max_val: u64,
    pub top_min: usize,
    pub top_max: usize,
    pub sort_by: SortOrder,

    // Settings & Limits
    pub show_settings: bool,
    pub theme_mode: ThemeMode,
    pub max_prime_limit: u64,
    pub max_k_limit: usize,
    pub show_grid_lines: bool,
    pub show_pct_labels: bool,
    pub show_heatmap_meter: bool,

    // Data
    pub metadata: Option<DatasetMetadata>,
    pub freq_data: Vec<(u64, u64)>,
    pub query_latency_ms: Option<f64>,

    // Worker Status
    pub is_loading: bool,
    pub progress: f32,
    pub error_msg: Option<String>,

    // Channels
    pub cmd_tx: Sender<WorkerCommand>,
    pub res_rx: Receiver<WorkerResult>,
}

impl AppState {
    pub fn new(cmd_tx: Sender<WorkerCommand>, res_rx: Receiver<WorkerResult>) -> Self {
        Self {
            k: 2,
            min_val: 1,
            max_val: 10_000_000_000, // Default 10B (10 Billion)
            top_min: 1,
            top_max: 20,
            sort_by: SortOrder::ByFrequency,

            show_settings: false,
            theme_mode: ThemeMode::Dark,
            max_prime_limit: 10_000_000_000, // Default 10B (10 Billion)
            max_k_limit: 3,                  // Default max k limit = 3
            show_grid_lines: true,
            show_pct_labels: true,
            show_heatmap_meter: true,

            metadata: None,
            freq_data: Vec::new(),
            query_latency_ms: None,

            is_loading: false,
            progress: 0.0,
            error_msg: None,

            cmd_tx,
            res_rx,
        }
    }
}



