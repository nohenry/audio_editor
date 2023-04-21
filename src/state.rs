use std::{sync::Arc};

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
