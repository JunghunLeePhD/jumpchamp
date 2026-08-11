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
    Progress {
        progress: f32,
        current_block: usize,
        total_blocks: usize,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayDirection {
    Forward,
    Reverse,
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
    pub show_grid_lines: bool,
    pub show_pct_labels: bool,
    pub show_heatmap_meter: bool,

    // Animation Controls (Cumulative Linear Growth)
    pub is_animating: bool,
    pub is_precaching: bool,
    pub anim_direction: PlayDirection,
    pub anim_current_val: u64,
    pub anim_step_size: u64,
    pub anim_speed_fps: f32,
    pub last_frame_instant: Option<std::time::Instant>,

    // Data
    pub metadata: Option<DatasetMetadata>,
    pub freq_data: Vec<(u64, u64)>,
    pub query_latency_ms: Option<f64>,

    // Worker Status
    pub is_loading: bool,
    pub progress: f32,
    pub current_block: usize,
    pub total_blocks: usize,
    pub error_msg: Option<String>,

    // Channels
    pub cmd_tx: Sender<WorkerCommand>,
    pub res_rx: Receiver<WorkerResult>,
}

impl AppState {
    pub fn new(cmd_tx: Sender<WorkerCommand>, res_rx: Receiver<WorkerResult>) -> Self {
        let min_v = 1u64;
        let max_v = 1_000_000u64;
        let default_step = (max_v.saturating_sub(min_v) / 50).max(1);

        Self {
            k: 2,
            min_val: min_v,
            max_val: max_v, // Default 1 Million Primes (n = 1 ~ 1,000,000)
            top_min: 1,
            top_max: 20,
            sort_by: SortOrder::ByGapSize, // Default to Gap Mode (Fixed Numerical Order)

            show_settings: false,
            theme_mode: ThemeMode::Dark,
            max_prime_limit: 10_000_000, // Default 10 Million Primes Limit
            show_grid_lines: true,
            show_pct_labels: true,
            show_heatmap_meter: true,

            is_animating: false,
            is_precaching: false,
            anim_direction: PlayDirection::Forward,
            anim_current_val: default_step,
            anim_step_size: default_step,
            anim_speed_fps: 5.0,
            last_frame_instant: None,

            metadata: None,
            freq_data: Vec::new(),
            query_latency_ms: None,

            is_loading: false,
            progress: 0.0,
            current_block: 0,
            total_blocks: 0,
            error_msg: None,

            cmd_tx,
            res_rx,
        }
    }

    pub fn animation_progress(&self) -> f32 {
        let range = self.max_val.saturating_sub(self.min_val).max(1) as f32;
        let cur = self.anim_current_val.saturating_sub(self.min_val) as f32;
        (cur / range).clamp(0.0, 1.0)
    }

    pub fn update_freq_data(&mut self, new_freq: Vec<(u64, u64)>) {
        self.freq_data = new_freq;
        self.top_min = 1;
        self.top_max = self.freq_data.len().max(1);
    }

    pub fn recalculate_dynamic_step(&mut self) {
        let range = self.max_val.saturating_sub(self.min_val);
        self.anim_step_size = (range / 50).max(1);
    }
}



