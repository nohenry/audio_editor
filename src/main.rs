use std::{
    collections::HashMap,
    f32::consts::PI,
    sync::{Arc, RwLock},
};

use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type, Q_BUTTERWORTH_F32};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use id::Id;
use rubato::Resampler;
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
                sample::Sample::load_from_file("sample12.wav", Some("Sample 1"), &state).unwrap(),
            );
            let track_sample = sample.clone();

            let sample2 = Arc::new(
                sample::Sample::load_from_file("sample12.wav", Some("Sample 2"), &state).unwrap(),
            );
            let track_sample2 = sample2.clone();

            let sample3 = Arc::new(
                sample::Sample::load_from_file("sine44048.wav", Some("Sample 2"), &state).unwrap(),
            );
            let track_sample3 = sample3.clone();

            let track = Arc::new(RwLock::new(Track::new(
                "Track",
                vec![track_sample.clone()],
                track_state.clone(),
            )));

            let track2 = Arc::new(RwLock::new(Track::new(
                "Track2",
                vec![track_sample3],
                track_state,
            )));

            let stream = start_audio(device, vec![track.clone()], state);

            Box::new(Application::new(
                cc,
                vec![track.clone(), track2.clone()],
                vec![stream],
            ))
        }),
    )
    .unwrap();
}

fn start_audio(
    device: cpal::Device,
    tracks: Vec<Arc<RwLock<Track>>>,
    state: Arc<RwLock<State>>,
) -> cpal::Stream {
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    for f in supported_configs_range {
        println!("{:?}", f);
    }
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    supported_configs_range.next();
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_max_sample_rate();

    // println!("{:?}", supported_configs_range);

    println!("Channels: {}", supported_config.channels());
    println!("Rate: {:?}", supported_config.sample_rate());
    println!("Rate: {:?}", supported_config.buffer_size());
    println!("Config: {:?}", supported_config.config());

    let target_sample_rate = supported_config.sample_rate();
    let target_sample_count = supported_config.channels();

    let f0 = 10.hz();
    let fs = (12000.0 / 100.0).hz();

    // Create coefficients for the biquads
    let coeffs =
        Coefficients::<f32>::from_params(Type::LowPass, fs, f0, Q_BUTTERWORTH_F32).unwrap();

    let mut filter = DirectForm1::<f32>::new(coeffs);

    let mut absolute_index = 0.0;
    let mut current_index = 0.0;
    let mut current_id = Id::NULL;

    let mut output_index = vec![0; tracks.len()];

    for track in &tracks {
        let track = track.read();
    }
    let buffer_size = match supported_config.buffer_size() {
        cpal::SupportedBufferSize::Range { max, .. } => *max,
        _ => 0,
    };

    let mut output_buffer = [Vec::<f32>::with_capacity(buffer_size as _)];

    let mut resamplers: Vec<Vec<_>> = tracks
        .iter()
        .map(|track| {
            let track = track.read().unwrap();
            track
                .samples
                .iter()
                .map(|sample| {
                    if sample.header.sampling_rate != target_sample_rate.0 {
                        let params = rubato::InterpolationParameters {
                            sinc_len: 256,
                            f_cutoff: 0.95,
                            interpolation: rubato::InterpolationType::Linear,
                            oversampling_factor: 256,
                            window: rubato::WindowFunction::BlackmanHarris2,
                        };

                        Some(
                            rubato::SincFixedOut::<f32>::new(
                                target_sample_rate.0 as f64 / sample.header.sampling_rate as f64,
                                2.0,
                                params,
                                buffer_size as _,
                                // 2,
                                sample.header.channel_count as _,
                            )
                            .unwrap(),
                        )
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    let mut currents = vec![(0.0, Id::NULL); tracks.len()];

    let write_data_f32 = move |sample_data: &mut [f32], info: &cpal::OutputCallbackInfo| {
        {
            let mut state = state.write().unwrap();
            state.play_time.get_or_insert(info.timestamp().playback);
            state.current_time = Some(info.timestamp().playback);
        }

        sample_data.fill(0.0);

        for (((track, ind), output), resampler) in tracks
            .iter()
            .zip(currents.iter_mut())
            .zip(output_index.iter_mut())
            .zip(resamplers.iter_mut())
        {
            write_track(
                track,
                &mut absolute_index,
                &mut ind.0,
                output,
                &mut ind.1,
                target_sample_count,
                target_sample_rate.0 as u64,
                sample_data,
                &mut filter,
                resampler,
                &mut output_buffer,
            )
        }

        for sample_point in sample_data {
            *sample_point = sample_point.tanh();
        }

        let state = state.read().unwrap();
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

fn linear_interpolation(x0: f32, x1: f32, t: f32) -> f32 {
    x0 * (1.0 - t) + x1 * t
}

#[inline(always)]
fn normalize(sample: f32) -> f32 {
    // (sample + 1.0) * 0.5
    sample
}

#[inline(always)]
fn cubic_spline_interpolation(x0: f32, x1: f32, x2: f32, x3: f32, t: f32) -> f32 {
    let c0 = x1;
    let c1 = 0.5 * (x2 - x0);
    let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3;
    let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);
    ((c3 * t + c2) * t + c1) * t + c0
}

fn sinc(x: f32) -> f32 {
    if x == 0.0 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

fn windowed_sinc(x: f32, window_size: usize) -> f32 {
    let mut y = x / window_size as f32;
    y *= PI;
    let w = y.sin() / y;
    let window = (0..window_size)
        .map(|i| {
            let n = i as f32 - window_size as f32 / 2.0;
            0.54 - 0.46 * ((2.0 * PI * n) / (window_size as f32 - 1.0)).cos()
        })
        .fold(1.0, |acc, x| acc * x);
    w * window
}

fn write_track(
    track: &Arc<RwLock<Track>>,
    absolute_index: &mut f64,
    current_index: &mut f64,
    output_index: &mut u64,
    current_id: &mut Id,
    target_sample_count: u16,
    target_sample_rate: u64,
    sample_data: &mut [f32],
    filter: &mut DirectForm1<f32>,
    resamplers: &mut Vec<Option<rubato::SincFixedOut<f32>>>,
    resampling_buffer: &mut [Vec<f32>],
) {
    let track = track.read().unwrap();
    let Some((mut sample, mut sample_index)) = track.sample_at_index(*output_index as usize, target_sample_rate as f64) else {
        return;
    };

    if *current_id != sample.id {
        *current_index = 0.0;
        *current_id = sample.id;
    }

    let mut data = sample.data.as_thirty_two_float().unwrap();

    if sample.header.channel_count == target_sample_count {
        for sd in sample_data.chunks_mut(target_sample_count as usize) {
            // for (i, s) in sd.iter_mut().enumerate() {
            // *s = data[index as usize + i] * 0.2;
            // }
            sd.copy_from_slice(
                &data[*current_index as usize
                    ..*current_index as usize + target_sample_count as usize],
            );

            // *current_index += (sample.header.channel_count as f64)
            //     * (sample.header.sampling_rate as f64 / target_sample_rate as f64);

            // *absolute_index += (sample.header.channel_count as f64)
            //     * (sample.header.sampling_rate as f64 / target_sample_rate as f64);
        }
    } else if sample.header.channel_count == 1 {
        // let left = sample_data
        if let Some(resampler) = &mut resamplers[sample_index] {
            let mut data = [&data[*output_index as usize
                ..*output_index as usize + 250]];
            println!("{}", data[0].len());
            let buffer = resampler.process(&data, None).unwrap();
            // resampler
            //     .process_into_buffer(&mut data, resampling_buffer, None)
            //     .unwrap();

            for (i, sd) in sample_data
                .chunks_mut(target_sample_count as usize)
                .enumerate()
            {
                for dst in sd.iter_mut() {
                    *dst = buffer[0][i]
                }
            }

            // sample_data.copy_from_slice(&resampling_buffer[0]);
        }
        *output_index += 120;
        // for sd in sample_data.chunks_mut(target_sample_count as usize) {
        //     if *current_index as usize >= data.len() {
        //         let Some((ssample, index)) = track.sample_at_index(*output_index as usize, target_sample_rate as f64) else {
        //             return;
        //         };

        //         sample = ssample;
        //         sample_index = index;
        //         data = sample.data.as_thirty_two_float().unwrap();

        //         *current_index = 0.0;
        //     }

        //     let value = data[*output_index as usize];

        //     // let original_sample_index = *output_index as f64
        //     //     / (target_sample_rate as f64 / sample.header.sampling_rate as f64);
        //     // let floor_original_sample_index = original_sample_index.floor() as usize;
        //     // let fractional_part = original_sample_index - floor_original_sample_index as f64;

        //     // let value = if floor_original_sample_index + 3 < sample.len() {
        //     //     let value = cubic_spline_interpolation(
        //     //         normalize(data[floor_original_sample_index]),
        //     //         normalize(data[floor_original_sample_index + 1]),
        //     //         normalize(data[floor_original_sample_index + 2]),
        //     //         normalize(data[floor_original_sample_index + 3]),
        //     //         fractional_part as f32,
        //     //     );

        //     //     // let value = filter.run(value);

        //     //     value
        //     // } else {
        //     //     normalize(data[floor_original_sample_index])
        //     // };

        //     for s in sd {
        //         *s += value;
        //     }

        //     // *current_index += (sample.header.channel_count as f64)
        //     //     * (sample.header.sampling_rate as f64 / target_sample_rate as f64);

        //     *output_index += 1;

        //     // *absolute_index += (sample.header.channel_count as f64)
        //     //     * (sample.header.sampling_rate as f64 / target_sample_rate as f64);
        // }
    }
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
