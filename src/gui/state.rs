// ============================================================================
// AppState & Data Structures for GUI
// ============================================================================

use crossbeam_channel::{Receiver, Sender};

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

use crate::config::default_gaps_path;

pub enum WorkerCommand {
    LoadParquet {
        path: String,
        min_idx: u64,
        max_idx: u64,
        k: usize,
        top_n: usize,
        sort_by: SortOrder,
    },
    GenerateDatabase {
        limit: usize,
        k: usize,
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
    pub file_path: String,
    pub k: usize,
    pub min_idx: u64,
    pub max_idx: u64,
    pub top_n: usize,
    pub sort_by: SortOrder,

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
        let default_app_db = default_gaps_path(2);
        let initial_path = if default_app_db.exists() {
            default_app_db.to_string_lossy().into_owned()
        } else if std::path::Path::new("gaps2.parquet").exists() {
            "gaps2.parquet".to_string()
        } else {
            default_app_db.to_string_lossy().into_owned()
        };

        Self {
            file_path: initial_path,
            k: 2,
            min_idx: 1,
            max_idx: 1_000_000,
            top_n: 20,
            sort_by: SortOrder::ByFrequency,

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



