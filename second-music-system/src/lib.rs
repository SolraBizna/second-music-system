use std::{
    collections::HashMap,
    cmp::{Ordering, PartialOrd},
    sync::Arc,
};

use arcow::Arcow;
use crossbeam::channel::{Sender, Receiver, unbounded};
use parking_lot::RwLock;

#[macro_use]
mod din;
mod data;
mod engine;
mod fader;
mod stream;

use data::*;
pub use data::StringOrNumber;
pub use engine::*;
pub use fader::*;
pub use stream::*;

/// Encapsulates all the information about a soundtrack: what files to play,
/// how to play them, etc. This is purely inert data. It can be built up
/// incrementally, or replaced entirely, cleanly and efficiently.
#[derive(Clone, Debug)]
pub struct Soundtrack {
    flows: Arcow<HashMap<String, Arc<Flow>>>,
    sequences: Arcow<HashMap<String, Arc<Sequence>>>,
    sounds: Arcow<HashMap<String, Arc<Sound>>>,
}

impl Soundtrack {
    pub fn new() -> Soundtrack {
        Soundtrack {
            flows: Arcow::new(HashMap::new()),
            sequences: Arcow::new(HashMap::new()),
            sounds: Arcow::new(HashMap::new()),
        }
    }
    pub fn from_source(source: &str) -> Result<Soundtrack, String> {
        Soundtrack::new().parse_source(source)
    }
}

trait SecondsToIndexHack {
    fn seconds_to_index(&self, sample_rate: f32) -> u64;
}

impl SecondsToIndexHack for f32 {
    fn seconds_to_index(&self, sample_rate: f32) -> u64 {
        (self * sample_rate).floor() as u64
    }
}

mod private { pub trait Sealed {} }

pub trait Sample: private::Sealed + Send + Sync + Copy + Clone + 'static {
    fn to_float_sample(&self) -> f32;
    fn make_formatted_sound_reader_from(value: Box<dyn SoundReader<Self>>) -> FormattedSoundReader;
}

impl private::Sealed for u8 {}
impl Sample for u8 {
    fn to_float_sample(&self) -> f32 { (*self - 128) as f32 * (1.0 / 128.0) }
    fn make_formatted_sound_reader_from(value: Box<dyn SoundReader<u8>>) -> FormattedSoundReader { FormattedSoundReader::U8(value) }
}

impl private::Sealed for u16 {}
impl Sample for u16 {
    fn to_float_sample(&self) -> f32 { (*self - 32768) as f32 * (1.0 / 32768.0) }
    fn make_formatted_sound_reader_from(value: Box<dyn SoundReader<u16>>) -> FormattedSoundReader { FormattedSoundReader::U16(value) }
}

impl private::Sealed for i8 {}
impl Sample for i8 {
    fn to_float_sample(&self) -> f32 { *self as f32 * (1.0 / 128.0) }
    fn make_formatted_sound_reader_from(value: Box<dyn SoundReader<i8>>) -> FormattedSoundReader { FormattedSoundReader::I8(value) }
}

impl private::Sealed for i16 {}
impl Sample for i16 {
    fn to_float_sample(&self) -> f32 { *self as f32 * (1.0 / 32768.0) }
    fn make_formatted_sound_reader_from(value: Box<dyn SoundReader<i16>>) -> FormattedSoundReader { FormattedSoundReader::I16(value) }
}

impl private::Sealed for f32 {}
impl Sample for f32 {
    fn to_float_sample(&self) -> f32 { *self }
    fn make_formatted_sound_reader_from(value: Box<dyn SoundReader<f32>>) -> FormattedSoundReader { FormattedSoundReader::F32(value) }
}