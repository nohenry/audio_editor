use std::{
    collections::HashMap,
    f32::consts::PI,
    fs::File,
    io::{self, Cursor, Seek, Write},
    slice::{Iter, IterMut},
    sync::{Arc, RwLock},
};

use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type, Q_BUTTERWORTH_F32};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use egui::output;
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

    let mut absolute_index = 0;
    let mut current_index = 0.0;
    let mut current_id = Id::NULL;

    let mut output_index = vec![0; tracks.len()];

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
                            window: rubato::WindowFunction::Blackman,
                        };

                        let resampler = rubato::SincFixedOut::<f32>::new(
                            target_sample_rate.0 as f64 / sample.header.sampling_rate as f64,
                            2.0,
                            params,
                            buffer_size as usize / 2,
                            sample.header.channel_count as _,
                        )
                        .unwrap();

                        let buffer = vec![
                            Vec::with_capacity(sample.len());
                            sample.header.channel_count as usize
                        ];
                        let index = 0usize;

                        Some((resampler, buffer, index))
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
                output,
                target_sample_count,
                target_sample_rate.0 as u64,
                sample_data,
                resampler,
                &mut output_buffer,
            )
        }

        for sample_point in sample_data {
            // *sample_point = sample_point.tanh();
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

#[inline(always)]
fn normalize(sample: f32) -> f32 {
    (sample + 1.0) * 0.5
    // sample
}

fn write_track(
    track: &Arc<RwLock<Track>>,
    absolute_index: &mut usize,
    output_index: &mut usize,
    target_sample_count: u16,
    target_sample_rate: u64,
    sample_data: &mut [f32],
    resamplers: &mut Vec<Option<(impl rubato::Resampler<f32>, Vec<Vec<f32>>, usize)>>,
    resampling_buffer: &mut [Vec<f32>],
) {
    let track = track.read().unwrap();
    let Some((sample, sample_index)) = track.sample_at_sample_index(*absolute_index as usize, target_sample_rate as f64) else {
        return;
    };

    let data = sample.data.as_thirty_two_float().unwrap();

    let adjusted_len = sample_data.len() / target_sample_count as usize;
    if let Some((resampler, buffer, index)) = &mut resamplers[sample_index] {
        if *index >= data.len() {
            *absolute_index += adjusted_len;
            *output_index = 0;

            return;
        } else if *index + resampler.input_frames_next() >= data.len() {
            let data = vec![data[*index..].to_vec(), vec![0.0f32; data.len() - *index]].concat();

            *index += resample(
                &data,
                resampling_buffer,
                buffer,
                0,
                sample.header.channel_count,
                *output_index,
                sample_data.len() / (target_sample_count as usize),
                resampler,
            );

            channel_router_split_input(
                sample.header.channel_count,
                target_sample_count,
                &buffer,
                sample_data,
                *output_index,
            );

            *absolute_index += adjusted_len;
            *output_index += adjusted_len;

            return;
        }

        *index += resample(
            &data,
            resampling_buffer,
            buffer,
            *index,
            sample.header.channel_count,
            *output_index,
            sample_data.len() / (target_sample_count as usize),
            resampler,
        );

        channel_router_split_input(
            sample.header.channel_count,
            target_sample_count,
            &buffer,
            sample_data,
            *output_index,
        );

        *absolute_index += adjusted_len;
        *output_index += adjusted_len;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Speaker {
    FrontLeft = 0,
    FrontRight = 1,
    Center = 2,
    Subwoofer = 3,
    SideLeft = 4,
    SideRight = 5,
    RearLeft = 6,
    RearRight = 7,
    HeightLeft1 = 8,
    HeightRight1 = 9,
    HeightLeft2 = 10,
    HeightRight2 = 11,
}

impl Speaker {
    pub const fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub const fn len(&self) -> u16 {
        *self as u16 + 1
    }

    pub const fn ind(self) -> u16 {
        self as u16 + 1
    }

    pub const fn from_ind(ind: u16) -> Speaker {
        match ind - 1 {
            0 => Speaker::FrontLeft,
            1 => Speaker::FrontRight,
            2 => Speaker::Center,
            3 => Speaker::Subwoofer,
            4 => Speaker::SideLeft,
            5 => Speaker::SideRight,
            6 => Speaker::RearLeft,
            7 => Speaker::RearRight,
            8 => Speaker::HeightLeft1,
            9 => Speaker::HeightRight1,
            10 => Speaker::HeightLeft2,
            11 => Speaker::HeightRight2,
            _ => panic!("Unsupported speaker index!"),
        }
    }
}

impl Into<u8> for Speaker {
    fn into(self) -> u8 {
        self as u8
    }
}

impl From<u8> for Speaker {
    fn from(value: u8) -> Self {
        match value {
            0 => Speaker::FrontLeft,
            1 => Speaker::FrontRight,
            2 => Speaker::Center,
            3 => Speaker::Subwoofer,
            4 => Speaker::SideLeft,
            5 => Speaker::SideRight,
            6 => Speaker::RearLeft,
            7 => Speaker::RearRight,
            8 => Speaker::HeightLeft1,
            9 => Speaker::HeightRight1,
            10 => Speaker::HeightLeft2,
            11 => Speaker::HeightRight2,
            _ => panic!("Unsupported speaker index!"),
        }
    }
}

fn channel_router<'a>(
    input_channels: u16,
    output_channels: u16,
    input: &[f32],
    output: &mut [f32],
) {
    match (
        Speaker::from_ind(input_channels),
        Speaker::from_ind(output_channels),
    ) {
        (Speaker::FrontLeft, _) => output
            .chunks_exact_mut(output_channels as _)
            .zip(input.iter())
            .for_each(|(o, i)| o.fill(*i)),
        (i, o) if i >= o => output.copy_from_slice(&input[..output_channels as usize]),
        (_, _) => output
            .chunks_exact_mut(output_channels as _)
            .zip(input.chunks(input_channels as _))
            .for_each(|(o, i)| {
                o[..2].copy_from_slice(&i[..2]);
                o[2..4.min(input_channels as usize)]
                    .copy_from_slice(&i[2..4.min(input_channels as usize)]);
                o[4..]
                    .chunks_exact_mut(2)
                    .for_each(|o| o.copy_from_slice(&i[..2]));
            }),
    }
}

fn channel_router_split_input<'a>(
    input_channels: u16,
    output_channels: u16,
    input: &[impl AsRef<[f32]>],
    output: &mut [f32],
    input_offset: usize,
) {
    match (
        Speaker::from_ind(input_channels),
        Speaker::from_ind(output_channels),
    ) {
        (Speaker::FrontLeft, _) => output
            .chunks_exact_mut(output_channels as _)
            .zip(input[0].as_ref()[input_offset..].iter())
            .for_each(|(o, i)| o.fill(*i)),
        (i, o) if i >= o => {
            (0..output_channels as usize).for_each(|c| {
                output
                    .iter_mut()
                    .skip(c)
                    .step_by(output_channels as usize)
                    .enumerate()
                    .for_each(|(i, o)| *o = input[c].as_ref()[i + input_offset])
            });
        }
        (_, _) => output
            .chunks_exact_mut(output_channels as _)
            .enumerate()
            .for_each(|(i, o)| {
                let ins = input[..input_offset].iter().map(|f| f.as_ref()[i]);
                o[..2].iter_mut().zip(ins).for_each(|o| *o.0 = o.1);
            }),
    }
}

fn strip_samples_iter(data: &[f32], channels: usize) -> Vec<impl Iterator<Item = f32> + '_> {
    (0..channels)
        .map(|c| data.iter().copied().skip(c).step_by(channels))
        .collect()
}

fn strip_samples(data: &[f32], channels: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| data.iter().copied().skip(c).step_by(channels).collect())
        .collect()
}

fn resample(
    data: &[f32],
    resampling_buffer: &mut [Vec<f32>],
    output_buffer: &mut [Vec<f32>],
    input_offset: usize,
    input_channels: u16,
    output_offset: usize,
    output_len: usize,
    resampler: &mut impl rubato::Resampler<f32>,
) -> usize {
    if output_buffer[0].len() > output_offset && output_len < output_buffer[0].len() - output_offset
    {
        0
    } else {
        // Only resample if we've used up the output buffer
        let len = resampler.input_frames_next();
        let sample_channels = strip_samples(
            &data[input_offset..input_offset + len],
            input_channels as usize,
        );

        resampler
            .process_into_buffer(&sample_channels, resampling_buffer, None)
            .unwrap();

        for i in 0..input_channels as usize {
            output_buffer[i].extend_from_slice(&resampling_buffer[i][..]);
        }

        len
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
