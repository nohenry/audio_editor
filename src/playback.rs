use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, StreamTrait},
    SampleFormat,
};
use tracing::{info, warn};

use crate::{resampler::Resampler, state::State, track::Track};

const MAX_RESAMPLING_BUFFER: usize = 8096;

pub fn start_audio(
    device: cpal::Device,
    tracks: Vec<Arc<RwLock<Track>>>,
    state: Arc<RwLock<State>>,
) -> cpal::Stream {
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    // supported_configs_range.next();
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_max_sample_rate();

    info!("Channel Count: {}", supported_config.channels());
    info!("Sample Rate: {}", supported_config.sample_rate().0);
    info!("Buffer Size: {:?}", supported_config.buffer_size());
    info!("Config: {:?}", supported_config.config());

    let target_sample_rate = supported_config.sample_rate();
    let target_sample_count = supported_config.channels();

    // (current_index, absolute_index)
    let mut output_indicies = vec![(0, 0); tracks.len()];

    let buffer_size = match supported_config.buffer_size() {
        cpal::SupportedBufferSize::Range { max, .. } => *max,
        _ => 0,
    };

    // let resample_buffer_size = (buffer_size as usize).min(MAX_RESAMPLING_BUFFER);
    let resample_buffer_size = MAX_RESAMPLING_BUFFER;

    let mut output_buffer = [Vec::<f32>::with_capacity(resample_buffer_size)];

    let resamplers: Vec<_> = tracks
        .iter()
        .map(|track| {
            let track = track.read().unwrap();
            let track: Vec<_> = track
                .samples
                .iter()
                .map(|sample| {
                    if sample.header.sampling_rate != target_sample_rate.0 {
                        let params = rubato::InterpolationParameters {
                            sinc_len: 256,
                            f_cutoff: 0.95,
                            interpolation: rubato::InterpolationType::Cubic,
                            oversampling_factor: 256,
                            window: rubato::WindowFunction::Blackman,
                        };

                        let resampler = rubato::SincFixedOut::<f32>::new(
                            target_sample_rate.0 as f64 / sample.header.sampling_rate as f64,
                            2.0,
                            params,
                            resample_buffer_size,
                            sample.header.channel_count as _,
                        )
                        .unwrap();

                        let buffer = vec![
                            Vec::with_capacity(sample.len());
                            sample.header.channel_count as usize
                        ];
                        let index = 0usize;

                        Some((sample.clone(), resampler, buffer, index))
                    } else {
                        None
                    }
                })
                .collect();

            (
                Resampler::from(track),
                [Vec::<f32>::with_capacity(resample_buffer_size)],
            )
        })
        .collect();

    let thread_resamplers = resamplers.clone();

    let write_data_f32 = move |sample_data: &mut [f32], info: &cpal::OutputCallbackInfo| {
        {
            let mut state = state.write().unwrap();
            state.play_time.get_or_insert(info.timestamp().playback);
            state.current_time = Some(info.timestamp().playback);
        }

        sample_data.fill(0.0);

        for ((track, indicies), resampler) in tracks
            .iter()
            .zip(output_indicies.iter_mut())
            .zip(resamplers.iter())
        {
            write_track(
                track,
                &mut indicies.0,
                &mut indicies.1,
                target_sample_count,
                target_sample_rate.0 as u64,
                sample_data,
                &resampler.0,
            )
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

    // spawn one thread per track for resampling
    for (resampler, mut output_buffer) in thread_resamplers {
        std::thread::spawn(move || 'outer: loop {
            for (resampler, complete) in resampler.iter() {
                if complete.load(Ordering::SeqCst) {
                    continue;
                }

                if resampler.resample(&mut output_buffer) {
                    info!(
                        "{} - complete - {}",
                        resampler.sample().name,
                        resampler.buffer().read().unwrap()[0].len()
                    );
                    complete.store(true, Ordering::SeqCst);

                    break 'outer;
                }

                break;
            }
        });
    }

    stream
}

fn write_track(
    track: &Arc<RwLock<Track>>,
    current_index: &mut usize,
    absolute_index: &mut usize,
    target_sample_count: u16,
    target_sample_rate: u64,
    sample_data: &mut [f32],
    resampler: &Resampler,
) {
    let track = track.read().unwrap();
    let Some((sample, sample_index)) = track.sample_at_sample_index(*absolute_index, target_sample_rate as f64) else {
        warn!("Sample at index {} not found! (rate: {})", *absolute_index, target_sample_rate);
        return;
    };

    let data = sample.data.as_thirty_two_float().unwrap();

    let adjusted_len = sample_data.len() / target_sample_count as usize;

    if let Some(Some((resampler, _))) = resampler.iter_all().nth(sample_index) {
        // if !complete {
        let buffer = resampler.buffer().read().unwrap();
        channel_router_split_input(
            sample.header.channel_count,
            target_sample_count,
            &buffer,
            sample_data,
            *current_index,
        );
        // }

        *absolute_index += adjusted_len;
        *current_index += adjusted_len;
    }
    // let resampler =
    // if let Some((resampler, buffer, index)) = &mut resamplers[sample_index] {
    //     if *index >= data.len() {
    //         // *absolute_index += adjusted_len;
    //         // *output_index = 0;
    //         absolute_index.fetch_add(adjusted_len, Ordering::SeqCst);
    //         output_index.store(0, Ordering::SeqCst);

    //         return;
    //     } else if *index + resampler.input_frames_next() >= data.len() {
    //         let data = vec![data[*index..].to_vec(), vec![0.0f32; data.len() - *index]].concat();

    //         let oindex = output_index.load(Ordering::Acquire);
    //         *index += resample(
    //             &data,
    //             resampling_buffer,
    //             buffer,
    //             0,
    //             sample.header.channel_count,
    //             oindex,
    //             sample_data.len() / (target_sample_count as usize),
    //             resampler,
    //         );

    //         channel_router_split_input(
    //             sample.header.channel_count,
    //             target_sample_count,
    //             &buffer,
    //             sample_data,
    //             oindex,
    //         );

    //         // *absolute_index += adjusted_len;
    //         // *output_index += adjusted_len;
    //         absolute_index.fetch_add(adjusted_len, Ordering::SeqCst);
    //         output_index.fetch_add(adjusted_len, Ordering::Release);

    //         return;
    //     }

    //     let oindex = output_index.load(Ordering::Acquire);
    //     *index += resample(
    //         &data,
    //         resampling_buffer,
    //         buffer,
    //         *index,
    //         sample.header.channel_count,
    //         oindex,
    //         sample_data.len() / (target_sample_count as usize),
    //         resampler,
    //     );

    //     channel_router_split_input(
    //         sample.header.channel_count,
    //         target_sample_count,
    //         &buffer,
    //         sample_data,
    //         oindex,
    //     );

    //     absolute_index.fetch_add(adjusted_len, Ordering::SeqCst);
    //     output_index.fetch_add(adjusted_len, Ordering::Release);
    // }
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
            .for_each(|(o, i)| o.iter_mut().for_each(|o| *o += *i)),
        (i, o) if i >= o => {
            (0..output_channels as usize).for_each(|c| {
                output
                    .iter_mut()
                    .skip(c)
                    .step_by(output_channels as usize)
                    .enumerate()
                    .for_each(|(i, o)| *o += input[c].as_ref()[i + input_offset])
            });
        }
        (_, _) => output
            .chunks_exact_mut(output_channels as _)
            .enumerate()
            .for_each(|(i, o)| {
                let ins = input[..input_offset].iter().map(|f| f.as_ref()[i]);
                o[..2].iter_mut().zip(ins).for_each(|o| *o.0 += o.1);
            }),
    }
}
