use std::sync::{atomic::Ordering, Arc, RwLock};

use cpal::{
    traits::{DeviceTrait, StreamTrait},
    SampleFormat,
};
use tracing::{info, warn};

use crate::{
    channel::{channel_router, channel_router_split_input},
    resampler::Resampler,
    state::State,
    track::Track,
};

const MAX_RESAMPLING_BUFFER: usize = 16192;

pub fn start_audio(
    device: cpal::Device,
    tracks: Vec<Arc<RwLock<Track>>>,
    state: Arc<RwLock<State>>,
) -> cpal::Stream {
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    supported_configs_range.next();
    supported_configs_range.next();
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

    // indicies in the form of (current_index, absolute_index)
    let mut output_indicies = vec![(0, 0); tracks.len()];

    let buffer_size = match supported_config.buffer_size() {
        cpal::SupportedBufferSize::Range { max, .. } => *max,
        _ => 0,
    };

    let resample_buffer_size = (buffer_size as usize).min(MAX_RESAMPLING_BUFFER);

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

    stream.pause().unwrap();

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
        let buffer = resampler.buffer().read().unwrap();
        channel_router_split_input(
            sample.header.channel_count,
            target_sample_count,
            &buffer,
            sample_data,
            *current_index,
            // &track.channel_mapping,
            &None,
        );
    } else {
        channel_router(
            sample.header.channel_count,
            target_sample_count,
            data,
            sample_data,
            *current_index,
        );
    }
    *absolute_index += adjusted_len;
    *current_index += adjusted_len;
}
