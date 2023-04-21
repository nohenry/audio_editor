use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use biquad::{Coefficients, ToHertz, Type, Q_BUTTERWORTH_F32};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use id::Id;
use sample::WaveViewSampleState;
use track::Track;
use wave_view::WaveViewState;

use crate::state::State;

mod id;
mod sample;
mod state;
mod track;
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
            // frame.set_debug_on_hover(true);

            let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();

            let wave_view_state = Arc::new(WaveViewState::new(&wgpu_render_state));

            // let wave_view_state = init_wave_view_wgpu(&wgpu_render_state);
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
            let sample =
                Arc::new(sample::Sample::load_from_file("sample.wav", Some("S1"), &state).unwrap());
            let track_sample = sample.clone();

            let sample2 =
                Arc::new(sample::Sample::load_from_file("sample.wav", Some("S2"), &state).unwrap());
            let track_sample2 = sample2.clone();

            let stream = start_audio(device, sample, state);

            Box::new(Application::new(
                cc,
                vec![Track::new(
                    "Track",
                    vec![track_sample, track_sample2],
                    track_state,
                )],
                vec![stream],
            ))
        }),
    )
    .unwrap();
}

fn start_audio(
    device: cpal::Device,
    sample: Arc<sample::Sample>,
    state: Arc<RwLock<State>>,
) -> cpal::Stream {
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_max_sample_rate();

    println!("Channels: {}", supported_config.channels());
    println!("Rate: {:?}", supported_config.sample_rate());
    println!("Rate: {:?}", supported_config.buffer_size());
    println!("Config: {:?}", supported_config.config());

    let target_sample_rate = supported_config.sample_rate();
    let target_sample_count = supported_config.channels();

    let f0 = 10.hz();
    let fs = 1.khz();

    // Create coefficients for the biquads
    let _coeffs =
        Coefficients::<f32>::from_params(Type::BandPass, fs, f0, Q_BUTTERWORTH_F32).unwrap();

    let mut index = 0.0;
    let write_data_f32 = move |sample_data: &mut [f32], info: &cpal::OutputCallbackInfo| {
        let mut state = state.write().unwrap();
        state.play_time.get_or_insert(info.timestamp().playback);
        state.current_time = Some(info.timestamp().playback);

        let data = sample.data.as_thirty_two_float().unwrap();

        if sample.header.channel_count == target_sample_count {
            for sd in sample_data.chunks_mut(target_sample_count as usize) {
                // for (i, s) in sd.iter_mut().enumerate() {
                // *s = data[index as usize + i] * 0.2;
                // }
                sd.copy_from_slice(
                    &data[index as usize..index as usize + target_sample_count as usize],
                );

                index += (sample.header.channel_count as f64)
                    * (sample.header.sampling_rate as f64 / target_sample_rate.0 as f64);
            }
        } else if sample.header.channel_count == 1 {
            for sd in sample_data.chunks_mut(target_sample_count as usize) {
                let _value = data[index as usize] * 0.2;
                // let value = biquad1.run(value);
                let value = 0.0;

                for s in sd {
                    *s = value;
                }

                index += (sample.header.channel_count as f64)
                    * (sample.header.sampling_rate as f64 / target_sample_rate.0 as f64);
            }
        }

        state.egui_ctx.request_repaint();
    };

    let write_data_i16 = move |_: &mut [i16], _: &cpal::OutputCallbackInfo| println!("i16");

    let write_data_u16 = move |_: &mut [u16], _: &cpal::OutputCallbackInfo| println!("u16");

    let stream = match supported_config.sample_format() {
        SampleFormat::F32 => {
            device.build_output_stream(&supported_config.config(), write_data_f32, |_| {}, None)
        }
        SampleFormat::I16 => {
            device.build_output_stream(&supported_config.config(), write_data_i16, |_| {}, None)
        }
        SampleFormat::U16 => {
            device.build_output_stream(&supported_config.config(), write_data_u16, |_| {}, None)
        }
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .unwrap();

    stream.play().unwrap();

    stream
}

#[derive(Default)]
struct Application {
    _streams: Vec<cpal::Stream>,
    tracks: Vec<Track>,
}

impl Application {
    fn new(
        cc: &eframe::CreationContext<'_>,
        tracks: Vec<Track>,
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
                for track in &mut self.tracks {
                    track.ui(ui);
                }
            })
        });
    }
}
