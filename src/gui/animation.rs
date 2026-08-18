// ============================================================================
// Animation Controller & Dispatch Logic
// ============================================================================

use crate::gui::state::{AppState, PlayDirection, WorkerCommand};

/// Dispatches a single compute frame for static or animation playback.
pub fn dispatch_compute(state: &mut AppState) {
    state.is_loading = true;
    state.progress = 0.0;
    state.error_msg = None;

    let cmd = WorkerCommand::ComputeGaps {
        min_val: state.min_val,
        max_val: state.max_val,
        k: state.k,
        top_min: state.top_min,
        top_max: state.top_max,
        sort_by: state.sort_by.clone(),
    };
    state.cmd_tx.send(cmd).ok();
}

/// Dispatches the animation frame update (precomputed cache or on-demand worker query).
pub fn dispatch_anim_frame(state: &mut AppState) {
    if !state.update_freq_from_precomputed() {
        state.is_frame_in_flight = true;
        let min_range = state.min_val;
        let max_range = state.anim_current_val.max(state.min_val);

        let cmd = WorkerCommand::ComputeGaps {
            min_val: min_range,
            max_val: max_range,
            k: state.k,
            top_min: state.top_min,
            top_max: state.top_max,
            sort_by: state.sort_by.clone(),
        };
        state.cmd_tx.send(cmd).ok();
    }
}

/// Advances animation one step forward. Returns true if within bounds, false if reached the end.
pub fn advance_anim_forward(state: &mut AppState) -> bool {
    if state.anim_current_val >= state.max_val {
        state.is_animating = false;
        false
    } else {
        state.anim_current_val =
            (state.anim_current_val + state.anim_step_size).min(state.max_val);
        true
    }
}

/// Advances animation one step backward. Returns true if within bounds, false if reached the beginning.
pub fn advance_anim_backward(state: &mut AppState) -> bool {
    if state.anim_current_val <= state.min_val {
        state.is_animating = false;
        false
    } else {
        state.anim_current_val =
            state.anim_current_val.saturating_sub(state.anim_step_size).max(state.min_val);
        true
    }
}

/// Steps animation forward by one frame.
pub fn dispatch_step_animation(state: &mut AppState) {
    state.anim_direction = PlayDirection::Forward;
    if state.anim_current_val >= state.max_val {
        state.anim_current_val = state.min_val;
    } else {
        advance_anim_forward(state);
    }
    dispatch_anim_frame(state);
}

/// Steps animation backward by one frame.
pub fn dispatch_step_back_animation(state: &mut AppState) {
    state.anim_direction = PlayDirection::Reverse;
    if state.anim_current_val <= state.min_val {
        state.anim_current_val = state.max_val;
    } else {
        advance_anim_backward(state);
    }
    dispatch_anim_frame(state);
}

/// Starts continuous animation in the specified direction (Forward or Reverse),
/// pre-caching if needed or starting immediately from existing cache.
pub fn dispatch_start_animation(state: &mut AppState, direction: PlayDirection) {
    state.anim_direction = direction;
    state.recalculate_anim_step();

    match direction {
        PlayDirection::Forward => {
            if state.anim_current_val >= state.max_val {
                state.anim_current_val = state.min_val;
            }
        }
        PlayDirection::Reverse => {
            if state.anim_current_val <= state.min_val {
                state.anim_current_val = state.max_val;
            }
        }
    }

    let needs_precache = match &state.anim_precomputed {
        Some(data) => {
            data.min_val != state.min_val
                || data.max_val != state.max_val
                || data.k != state.k
        }
        None => true,
    };

    if needs_precache {
        state.is_loading = true;
        state.is_precaching = true;
        state.is_animating = false;
        state.progress = 0.0;
        state.error_msg = None;

        let cmd = WorkerCommand::PrecacheAnimation {
            min_val: state.min_val,
            max_val: state.max_val,
            k: state.k,
            total_frames: 300,
        };
        state.cmd_tx.send(cmd).ok();
    } else {
        state.is_animating = true;
        state.last_frame_instant = None;
        dispatch_anim_frame(state);
    }
}

/// Cancels ongoing background worker task and resets processing states.
pub fn dispatch_cancel(state: &mut AppState) {
    state.cmd_tx.send(WorkerCommand::Cancel).ok();
    state.is_loading = false;
    state.is_animating = false;
    state.is_precaching = false;
    state.is_frame_in_flight = false;
}
