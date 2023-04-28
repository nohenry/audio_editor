/// Represents a range of pixels on the screen.
/// This is meant for a single axis but doesn't matter which one
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PixelRange {
    pub min: f32,
    pub max: f32,
}

impl PixelRange {
    /// Get the difference between the two endpoints
    pub fn len(&self) -> f32 {
        self.max - self.min
    }
}

/// Represents a range of sample indicies in a sample
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SampleRange {
    pub min: u64,
    pub max: u64,
}

impl SampleRange {
    /// Get the difference between the two endpoints
    pub fn len(&self) -> u64 {
        self.max - self.min
    }
}

/// This function breaks down an audio signal with channels packed after
/// another into a vector of iterators with the vector indicies representing the channels.
///
/// `data` is the audio signal which would look like:
/// ```
/// [
/// 0.0, // Channel 1
/// 0.5, // Channel 2
/// 0.0, // Channel 1
/// 0.5, // Channel 2
/// 0.0, // Channel 1
/// ]
/// ```
///
/// `channels` is the number of channels the input signal contains
///
pub fn strip_samples_iter(data: &[f32], channels: usize) -> Vec<impl Iterator<Item = f32> + '_> {
    (0..channels)
        .map(|c| data.iter().copied().skip(c).step_by(channels))
        .collect()
}

/// This function breaks down an audio signal with channels packed after
/// another into a vector of vectors with the vector indicies representing the channels.
///
/// `data` is the audio signal which would look like:
/// ```
/// [
/// 0.0, // Channel 1
/// 0.5, // Channel 2
/// 0.0, // Channel 1
/// 0.5, // Channel 2
/// 0.0, // Channel 1
/// 0.5, // Channel 1
/// ...
/// ]
/// ```
///
/// `channels` is the number of channels the input signal contains
///
/// The return value would look like this:
/// ```
/// [
///     [0.0, 0.0, 0.0], // Channel 1
///     [0.5, 0.5, 0.5], // Channel 2
/// ]
/// ```
pub fn strip_samples(data: &[f32], channels: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| data.iter().copied().skip(c).step_by(channels).collect())
        .collect()
}
