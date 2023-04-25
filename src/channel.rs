use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum Speaker {
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
    pub const MAX_COUNT: usize = 11;

    pub const fn as_u16(&self) -> u16 {
        *self as u16
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

impl From<u16> for Speaker {
    fn from(value: u16) -> Self {
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

pub struct ChannelMapping(
    [Option<Speaker>; Speaker::MAX_COUNT],
    [Option<Speaker>; Speaker::MAX_COUNT],
);

impl ChannelMapping {
    pub fn default(channel_count: u16) -> ChannelMapping {
        let sps: Vec<_> = (0..Speaker::MAX_COUNT as u16)
            .map(|f| {
                if f > channel_count {
                    None
                } else {
                    Some(Speaker::from_ind(f + 1))
                }
            })
            .collect();

        ChannelMapping(sps.clone().try_into().unwrap(), sps.try_into().unwrap())
    }

    pub fn empty() -> ChannelMapping {
        ChannelMapping([None; Speaker::MAX_COUNT], [None; Speaker::MAX_COUNT])
    }

    pub fn get_output_index(&self, input_index: u16) -> Speaker {
        self.0
            .get(input_index as usize)
            .map_or(Speaker::from(input_index), |o| {
                o.unwrap_or(Speaker::from(input_index))
            })
    }

    pub fn get_input_index(&self, output_index: u16) -> Speaker {
        println!("{:?}", self.1);
        self.1
            .get(output_index as usize)
            .map_or(Speaker::from(output_index), |o| {
                o.unwrap_or(Speaker::from(output_index))
            })
    }

    pub fn in_out(&self) -> ChannelMappingAccess {
        ChannelMappingAccess(&self.0)
    }

    pub fn out_in(&self) -> ChannelMappingAccess {
        ChannelMappingAccess(&self.1)
    }

    pub fn in_out_mut(&mut self) -> ChannelMappingAccessMut {
        ChannelMappingAccessMut(&mut self.0)
    }

    pub fn out_in_mut(&mut self) -> ChannelMappingAccessMut {
        ChannelMappingAccessMut(&mut self.1)
    }
}

pub struct ChannelMappingAccess<'a>(&'a [Option<Speaker>; Speaker::MAX_COUNT]);
pub struct ChannelMappingAccessMut<'a>(&'a mut [Option<Speaker>; Speaker::MAX_COUNT]);

impl<'a> Index<&Speaker> for ChannelMappingAccess<'a> {
    type Output = Option<Speaker>;

    fn index(&self, index: &Speaker) -> &Self::Output {
        &self.0[index.as_u16() as usize]
    }
}

impl<'a> Index<&Speaker> for ChannelMappingAccessMut<'a> {
    type Output = Option<Speaker>;

    fn index(&self, index: &Speaker) -> &Self::Output {
        &self.0[index.as_u16() as usize]
    }
}

impl<'a> IndexMut<&Speaker> for ChannelMappingAccessMut<'a> {
    fn index_mut(&mut self, index: &Speaker) -> &mut Self::Output {
        &mut self.0[index.as_u16() as usize]
    }
}

impl<'a> Index<u16> for ChannelMappingAccess<'a> {
    type Output = Option<Speaker>;

    fn index(&self, index: u16) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl<'a> Index<u16> for ChannelMappingAccessMut<'a> {
    type Output = Option<Speaker>;

    fn index(&self, index: u16) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl<'a> IndexMut<u16> for ChannelMappingAccessMut<'a> {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl<'a> Index<usize> for ChannelMappingAccess<'a> {
    type Output = Option<Speaker>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'a> Index<usize> for ChannelMappingAccessMut<'a> {
    type Output = Option<Speaker>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<'a> IndexMut<usize> for ChannelMappingAccessMut<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl ChannelMapping {
    pub fn from_array_mapping<const N: usize>(
        value: [(Speaker, Speaker); N],
        channel_count: u16,
        keep_default: bool,
    ) -> Self {
        let mut default = if keep_default {
            ChannelMapping::default(channel_count)
        } else {
            ChannelMapping::empty()
        };

        for (i, o) in value.iter() {
            default.in_out_mut()[i] = Some(*o);
        }

        for (i, o) in value.iter() {
            default.out_in_mut()[o] = Some(*i);
        }

        default
    }
}

pub fn channel_router<'a>(
    input_channels: u16,
    output_channels: u16,
    input: &[f32],
    output: &mut [f32],
    input_offset: usize,
) {
    match (
        Speaker::from_ind(input_channels),
        Speaker::from_ind(output_channels),
    ) {
        (Speaker::FrontLeft, _) => output
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

pub fn channel_router_split_input<'a>(
    input_channels: u16,
    output_channels: u16,
    input: &[impl AsRef<[f32]>],
    output: &mut [f32],
    input_offset: usize,
    channel_mapping: &ChannelMapping,
) {
    let get_channel = |channel: u16| {
        let new_index = channel_mapping.get_input_index(channel);
        let ni = new_index.as_u16() as usize;

        println!("{} -> {}", channel, ni);

        ni
    };
    match (
        Speaker::from_ind(input_channels),
        Speaker::from_ind(output_channels),
    ) {
        (Speaker::FrontLeft, _) => output
            .chunks_exact_mut(output_channels as _)
            .zip(input[0].as_ref()[input_offset..].iter())
            .for_each(|(o, i)| {
                o.iter_mut().enumerate().for_each(|(ind, o)| {
                    if let Some(Speaker::FrontLeft) = channel_mapping.out_in()[ind] {
                        *o += *i;
                    }
                })
            }),
        (i, o) if i >= o => {
            (0..output_channels).for_each(|c| {
                output
                    .iter_mut()
                    .skip(c as usize)
                    .step_by(output_channels as usize)
                    .enumerate()
                    .for_each(|(i, o)| *o += input[get_channel(c)].as_ref()[i + input_offset])
            });
        }
        (_, _) => output
            .chunks_exact_mut(output_channels as _)
            .enumerate()
            .for_each(|(i, o)| {
                // o is each channel, i is the offset
                o[..2].iter_mut().enumerate().for_each(|(c, o)| {
                    *o += input[get_channel(c as u16)].as_ref()[i + input_offset]
                });
                o[2..4.max(input_channels as usize)]
                    .iter_mut()
                    .enumerate()
                    .for_each(|(c, o)| {
                        *o += input[get_channel(c as u16 + 2)].as_ref()[i + input_offset]
                    });

                o[4..].chunks_exact_mut(2).enumerate().for_each(|(c, o)| {
                    o[0] += input[get_channel(c as u16 + 4)].as_ref()[i + input_offset];
                    o[1] += input[get_channel(c as u16 + 5)].as_ref()[i + input_offset];
                });
            }),
    }
}
