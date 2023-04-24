use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use cpal::traits::HostTrait;
use id::Id;
use playback::start_audio;
use sample::WaveViewSampleState;
use track::Track;
use wave_view::WaveViewState;

use crate::state::State;

mod id;
mod playback;
mod sample;
mod state;
mod track;
mod util;
mod wave_view;

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1920.0, 1080.0)),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Audio Editor",
        options,
        Box::new(|cc| {
            let frame = cc.egui_ctx.clone();

            let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();

            let wave_view_state = Arc::new(WaveViewState::new(&wgpu_render_state));

            wgpu_render_state
                .renderer
                .write()
                .paint_callback_resources
                .insert(HashMap::<Id, Arc<WaveViewSampleState>>::new());

            // Create application state
            let state = Arc::new(RwLock::new(State {
                playing: true,
                current_time: None,
                play_time: None,

                egui_ctx: frame,
                wgpu_ctx: wgpu_render_state.clone(),

                wave_view_state,
            }));
            let track_state = state.clone();

            // Load test sample
            let sample = Arc::new(
                sample::Sample::load_from_file("sample_short.wav", Some("Sample 1"), &state)
                    .unwrap(),
            );
            let track_sample = sample.clone();

            let sample2 = Arc::new(
                sample::Sample::load_from_file("sample12.wav", Some("Sample 2"), &state).unwrap(),
            );
            let track_sample2 = sample2.clone();

            let sample3 = Arc::new(
                sample::Sample::load_from_file("sine440.wav", Some("Sample 2"), &state).unwrap(),
            );
            let track_sample3 = sample3.clone();

            let track = Arc::new(RwLock::new(Track::new(
                "Track",
                vec![track_sample.clone(), track_sample3.clone()],
                track_state.clone(),
            )));

            let track2 = Arc::new(RwLock::new(Track::new(
                "Track2",
                vec![track_sample3],
                track_state,
            )));

            let stream = start_audio(device, vec![track.clone(), track2.clone()], state);

            Box::new(Application::new(
                cc,
                vec![track.clone(), track2.clone()],
                vec![stream],
            ))
        }),
    )
    .unwrap();
}

#[derive(Default)]
struct Application {
    _streams: Vec<cpal::Stream>,
    tracks: Vec<Arc<RwLock<Track>>>,
}

impl Application {
    fn new(
        cc: &eframe::CreationContext<'_>,
        tracks: Vec<Arc<RwLock<Track>>>,
        streams: Vec<cpal::Stream>,
    ) -> Self {
        // Set open sans regular as the default font family
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "open_sans".to_string(),
            egui::FontData::from_static(include_bytes!(
                "../res/fonts/OpenSans/OpenSans-Regular.ttf"
            )),
        );
        fonts.families.insert(
            egui::FontFamily::Proportional,
            vec!["open_sans".to_string()],
        );
        fonts
            .families
            .insert(egui::FontFamily::Monospace, vec!["open_sans".to_string()]);

        cc.egui_ctx.set_fonts(fonts);

        let mut visuals = cc.egui_ctx.style().visuals.clone();
        visuals.override_text_color = Some(egui::Color32::from_rgb(255, 255, 255));
        cc.egui_ctx.set_visuals(visuals);

        Application {
            tracks,
            _streams: streams,
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("editor-main-heading").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Audio Editor");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 10.0);
                for track in &mut self.tracks {
                    let mut track = track.write().unwrap();
                    track.ui(ui);
                }
            })
        });
    }
}
