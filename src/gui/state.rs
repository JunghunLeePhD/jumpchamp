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

#[derive(Debug, Clone)]
pub struct PrecomputedAnimData {
    pub min_val: u64,
    pub max_val: u64,
    pub k: usize,
    pub total_frames: usize,
    pub step_size: u64,
    pub prefix_sums: Vec<Vec<u64>>,
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
    PrecacheAnimation {
        min_val: u64,
        max_val: u64,
        k: usize,
        total_frames: usize,
    },
    Cancel,
    ClearCache,
}

pub enum WorkerResult {
    Metadata(DatasetMetadata),
    FrequencyData(Vec<(u64, u64)>),
    PrecomputedAnimation(PrecomputedAnimData),
    QueryLatency(f64),
    Progress {
        progress: f32,
        current_block: usize,
        total_blocks: usize,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Static,
    Animation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayDirection {
    Forward,
    Reverse,
}

pub struct AppState {
    // View Mode
    pub view_mode: ViewMode,

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
    pub is_frame_in_flight: bool,
    pub anim_direction: PlayDirection,
    pub anim_current_val: u64,
    pub anim_step_size: u64,
    pub anim_speed_fps: f32,
    pub last_frame_instant: Option<std::time::Instant>,
    pub anim_precomputed: Option<PrecomputedAnimData>,

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
        let default_step = (max_v.saturating_sub(min_v) / 300).max(1);

        Self {
            view_mode: ViewMode::Static,

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
            is_frame_in_flight: false,
            anim_direction: PlayDirection::Forward,
            anim_current_val: min_v,
            anim_step_size: default_step,
            anim_speed_fps: 30.0,
            last_frame_instant: None,
            anim_precomputed: None,

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

    /// Resets the application state and clears all in-memory precomputations and worker caches back to launch defaults.
    pub fn reset(&mut self) {
        let min_v = 1u64;
        let max_v = 1_000_000u64;
        let default_step = (max_v.saturating_sub(min_v) / 300).max(1);

        self.view_mode = ViewMode::Static;
        self.k = 2;
        self.min_val = min_v;
        self.max_val = max_v;
        self.top_min = 1;
        self.top_max = 20;
        self.sort_by = SortOrder::ByGapSize;

        self.is_animating = false;
        self.is_precaching = false;
        self.is_frame_in_flight = false;
        self.anim_direction = PlayDirection::Forward;
        self.anim_current_val = min_v;
        self.anim_step_size = default_step;
        self.anim_speed_fps = 30.0;
        self.last_frame_instant = None;
        self.anim_precomputed = None;

        self.metadata = None;
        self.freq_data.clear();
        self.query_latency_ms = None;

        self.is_loading = false;
        self.progress = 0.0;
        self.current_block = 0;
        self.total_blocks = 0;
        self.error_msg = None;

        self.cmd_tx.send(WorkerCommand::ClearCache).ok();
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

    pub fn update_freq_from_precomputed(&mut self) -> bool {
        if let Some(ref data) = self.anim_precomputed {
            if data.min_val == self.min_val && data.max_val == self.max_val && data.k == self.k {
                let frame_idx = if data.step_size > 0 {
                    ((self.anim_current_val.saturating_sub(data.min_val)) / data.step_size) as usize
                } else {
                    0
                };
                let frame_idx = frame_idx.min(data.total_frames.saturating_sub(1));

                let hist = &data.prefix_sums[frame_idx];
                let mut freq_vec: Vec<(u64, u64)> = hist
                    .iter()
                    .enumerate()
                    .filter_map(|(gap, &count)| if count > 0 { Some((gap as u64, count)) } else { None })
                    .collect();

                let limit_top_n = self.top_max.max(1000);
                match self.sort_by {
                    SortOrder::ByFrequency => {
                        freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
                        freq_vec.truncate(limit_top_n);
                    }
                    SortOrder::ByGapSize => {
                        freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
                        freq_vec.truncate(limit_top_n);
                        freq_vec.sort_by_key(|&(g, _)| g);
                    }
                }

                self.freq_data = freq_vec;
                self.top_min = 1;
                self.top_max = self.freq_data.len().max(1);
                return true;
            }
        }
        false
    }

    /// Recalculates animation step size dynamically based on prime index range (targeting ~300 frames).
    pub fn recalculate_anim_step(&mut self) {
        let range = self.max_val.saturating_sub(self.min_val);
        self.anim_step_size = (range / 300).max(1);
    }

    pub fn recalculate_dynamic_step(&mut self) {
        self.recalculate_anim_step();
    }

    pub fn recalculate_anim_300_frames(&mut self) {
        self.recalculate_anim_step();
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        if mode == ViewMode::Animation {
            self.recalculate_anim_step();
        }
    }
}
