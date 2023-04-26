use std::ops::{Index, IndexMut};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Speakers: u16  {
        const FrontLeft    = 0b1;
        const FrontRight   = 0b10;
        const Center       = 0b100;
        const Subwoofer    = 0b1000;
        const SideLeft     = 0b10000;
        const SideRight    = 0b100000;
        const RearLeft     = 0b1000000;
        const RearRight    = 0b10000000;
        const HeightLeft1  = 0b100000000;
        const HeightRight1 = 0b1000000000;
        const HeightLeft2  = 0b10000000000;
        const HeightRight2 = 0b100000000000;
    }
}

impl Speakers {
    pub const MAX_COUNT: usize = 11;

    pub fn as_u16(&self) -> u16 {
        let zeros = self.bits().trailing_zeros() as u16;
        if zeros >= Speakers::MAX_COUNT as u16 {
            0
        } else {
            zeros
        }
    }

    pub const fn all_to(channel: u16) -> Speakers {
        Speakers::from_bits_truncate(u16::MAX >> (16 - channel))
    }
}

impl From<u16> for Speakers {
    fn from(value: u16) -> Self {
        match value {
            0 => Speakers::FrontLeft,
            1 => Speakers::FrontRight,
            2 => Speakers::Center,
            3 => Speakers::Subwoofer,
            4 => Speakers::SideLeft,
            5 => Speakers::SideRight,
            6 => Speakers::RearLeft,
            7 => Speakers::RearRight,
            8 => Speakers::HeightLeft1,
            9 => Speakers::HeightRight1,
            10 => Speakers::HeightLeft2,
            11 => Speakers::HeightRight2,
            _ => panic!("Unsupported speaker index!"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelMapping([Option<Speakers>; Speakers::MAX_COUNT]);

impl ChannelMapping {
    pub fn identity(channels: u16) -> ChannelMapping {
        let sps: Vec<_> = (0..Speakers::MAX_COUNT as u16)
            .map(|f| {
                if f > channels {
                    None
                } else {
                    Some(Speakers::from(f))
                }
            })
            .collect();

        ChannelMapping(sps.try_into().unwrap())
    }

    pub fn default(input_channels: u16, output_channels: u16) -> ChannelMapping {
        if input_channels == 1 {
            // Map all outputs to first channel
            let mut v = vec![None; Speakers::MAX_COUNT];
            v[0] = Some(Speakers::all());
            ChannelMapping(v.try_into().unwrap())
        } else if input_channels >= output_channels {
            // Map channels 1:1
            ChannelMapping::identity(output_channels)
        } else {
            let mut map = ChannelMapping::empty();

            let upto = Speakers::all_to(output_channels) & !Speakers::Subwoofer & !Speakers::Center;
            map[0u16] = Some(upto);
            for c in 2..output_channels {
                map[c] = Some(Speakers::from(c));
            }

            map
        }
    }

    pub fn empty() -> ChannelMapping {
        ChannelMapping([None; Speakers::MAX_COUNT])
    }
}

impl Index<&Speakers> for ChannelMapping {
    type Output = Option<Speakers>;

    fn index(&self, index: &Speakers) -> &Self::Output {
        &self.0[index.as_u16() as usize]
    }
}

impl IndexMut<&Speakers> for ChannelMapping {
    fn index_mut(&mut self, index: &Speakers) -> &mut Self::Output {
        &mut self.0[index.as_u16() as usize]
    }
}

impl Index<u16> for ChannelMapping {
    type Output = Option<Speakers>;

    fn index(&self, index: u16) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<u16> for ChannelMapping {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl Index<usize> for ChannelMapping {
    type Output = Option<Speakers>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for ChannelMapping {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl ChannelMapping {
    pub fn from_array_mapping<const N: usize>(
        value: [(Speakers, Speakers); N],
        channel_count: u16,
        keep_default: bool,
    ) -> Self {
        let mut default = if keep_default {
            ChannelMapping::default(channel_count, channel_count)
        } else {
            ChannelMapping::empty()
        };

        for (i, o) in value.iter() {
            default[i] = Some(*o);
        }

        default
    }
}

pub fn channel_router(
    input_channels: u16,
    output_channels: u16,
    input: &[f32],
    output: &mut [f32],
    input_offset: usize,
) {
    match (
        Speakers::from(input_channels - 1),
        Speakers::from(output_channels - 1),
    ) {
        (Speakers::FrontLeft, _) => output
            .chunks_exact_mut(output_channels as _)
            .zip(input[input_offset * input_channels as usize..].iter())
            .for_each(|(o, i)| o.fill(*i)),
        (i, o) if i >= o => output.copy_from_slice(
            &input[input_offset * input_channels as usize..output_channels as usize],
        ),
        (_, _) => output
            .chunks_exact_mut(output_channels as _)
            .zip(input[input_offset * input_channels as usize..].chunks(input_channels as _))
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

pub fn channel_router_split_input(
    input_channels: u16,
    output_channels: u16,
    input: &[impl AsRef<[f32]>],
    output: &mut [f32],
    input_offset: usize,
    channel_mapping: &Option<ChannelMapping>,
) {
    let default_mapping = ChannelMapping::default(input_channels, output_channels);
    let available = Speakers::all_to(output_channels);
    let channel_mapping = channel_mapping.as_ref().unwrap_or(&default_mapping);

    for (input_index, channel) in input.iter().enumerate() {
        let input = &channel.as_ref()[input_offset..];

        if let Some(speakers) = channel_mapping[input_index] {
            speakers.intersection(available).iter().for_each(|s| {
                let output_index = s.as_u16();

                output
                    .iter_mut()
                    .skip(output_index as usize)
                    .step_by(output_channels as usize)
                    .enumerate()
                    .for_each(|(i, o)| *o += input[i]);
            });
        }
    }
}
