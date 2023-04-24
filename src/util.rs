#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PixelRange {
    pub min: f32,
    pub max: f32,
}

impl PixelRange {
    pub fn len(&self) -> f32 {
        self.max - self.min
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SampleRange {
    pub min: u64,
    pub max: u64,
}

impl SampleRange {
    pub fn len(&self) -> u64 {
        self.max - self.min
    }
}
