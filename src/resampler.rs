use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, RwLock,
};

use rubato::SincFixedOut;

use crate::{sample::Sample, util::strip_samples};

pub struct ResamplerInner {
    sample: Arc<Sample>,
    resampler: Mutex<rubato::SincFixedOut<f32>>,
    buffer: RwLock<Vec<Vec<f32>>>,
    index: AtomicUsize,
}

impl ResamplerInner {
    pub fn sample(&self) -> &Arc<Sample> {
        &self.sample
    }

    pub fn buffer(&self) -> &RwLock<Vec<Vec<f32>>> {
        &self.buffer
    }

    fn impl_resample(&self, data: &[f32], input_index: usize, resampling_buffer: &mut [Vec<f32>]) {
        let len = {
            let mut resampler_guard = self.resampler.lock().unwrap();
            let resampler: &mut SincFixedOut<f32> = &mut resampler_guard;

            let len = rubato::Resampler::input_frames_next(resampler);
            let sample_channels = strip_samples(
                &data[input_index..input_index + len],
                self.sample.header.channel_count as usize,
            );

            rubato::Resampler::process_into_buffer(
                resampler,
                &sample_channels,
                resampling_buffer,
                None,
            )
            .unwrap();

            len
        };

        {
            let mut buffer = self.buffer.write().unwrap();
            for i in 0..self.sample.header.channel_count as usize {
                buffer[i].extend_from_slice(&resampling_buffer[i][..]);
            }
        }

        self.index.fetch_add(len, Ordering::SeqCst);
    }

    fn impl_resample_self(&self, resampling_buffer: &mut [Vec<f32>]) {
        let len = {
            let mut resampler_guard = self.resampler.lock().unwrap();
            let resampler: &mut SincFixedOut<f32> = &mut resampler_guard;

            let data = self.sample.data.as_thirty_two_float().unwrap();

            let len = rubato::Resampler::input_frames_next(resampler);
            let index = self.index.load(Ordering::SeqCst);

            let sample_channels = strip_samples(
                &data[index..index + len],
                self.sample.header.channel_count as usize,
            );

            rubato::Resampler::process_into_buffer(
                resampler,
                &sample_channels,
                resampling_buffer,
                None,
            )
            .unwrap();

            len
        };

        {
            let mut buffer = self.buffer.write().unwrap();
            for i in 0..self.sample.header.channel_count as usize {
                buffer[i].extend_from_slice(&resampling_buffer[i][..]);
            }
        }

        self.index.fetch_add(len, Ordering::SeqCst);
    }

    pub fn resample(&self, resampling_buffer: &mut [Vec<f32>]) -> bool {
        let next_len = {
            let resampler_guard = self.resampler.lock().unwrap();
            let resampler: &SincFixedOut<f32> = &resampler_guard;
            rubato::Resampler::input_frames_next(resampler)
        };

        let index = self.index.load(Ordering::SeqCst);
        if index + next_len >= self.sample.len() {
            let data = self.sample.data.as_thirty_two_float().unwrap();
            let data = vec![data[index..].to_vec(), vec![0.0f32; data.len() - index]].concat();

            self.impl_resample(&data, 0, resampling_buffer);
        } else {
            self.impl_resample_self(resampling_buffer);
        }

        self.index.load(Ordering::SeqCst) >= self.sample.data.as_thirty_two_float().unwrap().len()
    }
}

#[derive(Clone)]
pub struct Resampler(Arc<Vec<Option<(ResamplerInner, AtomicBool)>>>);

impl Resampler {
    pub fn iter_all(&self) -> impl Iterator<Item = &Option<(ResamplerInner, AtomicBool)>> + '_ {
        self.0.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(ResamplerInner, AtomicBool)> + '_ {
        self.0.iter().filter_map(|v| v.as_ref())
    }
}

impl From<Vec<Option<(Arc<Sample>, SincFixedOut<f32>, Vec<Vec<f32>>, usize)>>> for Resampler {
    fn from(value: Vec<Option<(Arc<Sample>, SincFixedOut<f32>, Vec<Vec<f32>>, usize)>>) -> Self {
        Resampler(Arc::new(
            value
                .into_iter()
                .map(|v| {
                    v.map(|v| {
                        (
                            ResamplerInner {
                                sample: v.0,
                                resampler: Mutex::new(v.1),
                                buffer: RwLock::new(v.2),
                                index: AtomicUsize::new(v.3),
                            },
                            AtomicBool::new(false),
                        )
                    })
                })
                .collect(),
        ))
    }
}
