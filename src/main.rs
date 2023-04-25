use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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
mod resampler;

fn load_channel(n: u32, state: &Arc<RwLock<State>>) -> Arc<RwLock<Track>> {
    let sample = Arc::new(
        sample::Sample::load_from_file(
            format!("res/sounds/channel{}.wav", n),
            Some(format!("Channel {}", n)),
            &state,
        )
        .unwrap(),
    );

    let track = Arc::new(RwLock::new(Track::new(
        format!("Track c{}", n),
        vec![sample],
        state.clone(),
    )));

    track
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let host = cpal::default_host();
    for device in host.output_devices().unwrap() {
        println!("{:?}", device.name());
        for config in device.supported_output_configs().unwrap() {
            println!("    {:?}", config);
        }
    }

    let device = host
        .output_devices()
        .expect("unable to iterate output devices")
        // .find(|device| device.name().unwrap() == "Speakers (Realtek(R) Audio)")
        .find(|device| device.name().unwrap() == "Speakers (Focusrite USB Audio)")
        .or_else(|| host.default_output_device())
        .expect("Unable to find any output devices!");

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

            // Load test sample
            let tracks: Vec<_> = (1..=8).map(|cn| load_channel(cn, &state)).collect();

            let stream = start_audio(device, tracks.clone(), state.clone());

            Box::new(Application::new(cc, tracks, state, vec![stream]))
        }),
    )
    .unwrap();
}

struct Application {
    streams: Vec<cpal::Stream>,
    tracks: Vec<Arc<RwLock<Track>>>,
    state: Arc<RwLock<State>>,
}

impl Application {
    fn new(
        cc: &eframe::CreationContext<'_>,
        tracks: Vec<Arc<RwLock<Track>>>,
        state: Arc<RwLock<State>>,
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
            state,
            streams,
        }
    }

    pub fn play(&self) {
        let mut state = self.state.write().unwrap();
        state.playing = true;

        self.streams.iter().for_each(|s| s.play().unwrap());
    }

    pub fn pause(&self) {
        let mut state = self.state.write().unwrap();
        state.playing = false;

        self.streams.iter().for_each(|s| s.pause().unwrap());
    }

    pub fn stop(&self) {
        let mut state = self.state.write().unwrap();
        state.playing = false;

        self.streams.iter().for_each(|s| s.pause().unwrap());
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("editor-main-heading").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.heading("Audio Editor");

                if ui.button("Play").clicked() {
                    self.play()
                }
                if ui.button("Pause").clicked() {
                    self.pause()
                }
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
