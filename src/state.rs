use std::{sync::Arc, time::Duration};

use crate::wave_view::WaveViewState;

pub struct State {
    pub playing: bool,
    pub play_time: Option<cpal::StreamInstant>,
    pub current_time: Option<cpal::StreamInstant>,

    pub egui_ctx: egui::Context,
    pub wgpu_ctx: eframe::egui_wgpu::RenderState,

    // WGPU state
    pub wave_view_state: Arc<WaveViewState>,
}

impl State {
    pub fn duration_played(&self) -> Option<Duration> {
        if !self.playing {
            return None;
        }

        if let (Some(start_time), Some(current_time)) = (&self.play_time, &self.current_time) {
            if let Some(duration) = current_time.duration_since(start_time) {
                return Some(duration);
            }
        }

        None
    }
}
