use std::mem::MaybeUninit;

use crate::Sample;

/// Describes the format of sound samples stored in a file. SMS uses floats
/// internally, so floats are the preferred format. Using other datatypes will
/// save some memory if cached samples are used, since they will be cached in
/// "native format".
pub enum FormattedSoundReader {
    /// Unsigned 8-bit sound. Zero point is 128, extremes are 1 and 255
    U8(Box<dyn SoundReader<u8>>),
    /// Unsigned 16-bit sound. Zero point is 32768, extremes are 1 and 65535
    U16(Box<dyn SoundReader<u16>>),
    /// Signed 8-bit sound. Zero point is 0, extremes are -127 and +127
    I8(Box<dyn SoundReader<i8>>),
    /// Signed 16-bit sound. Zero point is 0, extremes are -32767 and +32767
    I16(Box<dyn SoundReader<i16>>),
    /// IEEE 754 32-bit float sound. Zero point is 0, extremes are -1 and +1
    F32(Box<dyn SoundReader<f32>>),
}

/// Describes the number of channels and the speaker layout of sound sample
/// frames. This is used both for formats on disk, and also for the output
/// sound. SMS will interconvert them as needed.
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
#[repr(u32)]
#[non_exhaustive] // Might add layouts in the future
pub enum SpeakerLayout {
    // If you add more SpeakerLayouts, make sure to sync the definitions in the
    // C API as well.
    /// One channel, one speaker.
    Mono = 0,
    /// Two channels, two speakers. (FL, FR)
    Stereo = 1,
    /// Two channels, headphones. (L, R)
    Headphones = 2,
    /// Four channels, speakers in each corner. (FL, FR, RL, RR)
    Quadraphonic = 3,
    /// Six channels. Speakers in each corner, one in front, and one subwoofer.
    /// (FL, FR, C, LFE, RL, RR)
    Surround51 = 4,
    /// Eight channels. Speakers in each corner, one in front, one on each
    /// side, and one subwoofer. (FL, FR, C, LFE, RL, RR, SL, SR)
    Surround71 = 5,
}

impl SpeakerLayout {
    pub fn get_num_channels(&self) -> usize {
        match self {
            SpeakerLayout::Mono => 1,
            SpeakerLayout::Stereo | SpeakerLayout::Headphones => 2,
            SpeakerLayout::Quadraphonic => 4,
            SpeakerLayout::Surround51 => 6,
            SpeakerLayout::Surround71 => 8,
        }
    }
}

/// Describes an sound stream actively being decoded from the game data. It has
/// a particular sample rate (which we will convert), a particular speaker
/// layout (which we may also convert), and a callback that will return decoded
/// samples as needed. SMS will either cache this or stream it directly...
/// because of the latter case, mind your thread safety!
pub struct FormattedSoundStream {
    pub sample_rate: f32,
    pub speaker_layout: SpeakerLayout,
    pub reader: FormattedSoundReader,
}

/// This is an object that SMS will hang onto, and will call upon to open sound
/// files. It must be thread safe.
pub trait SoundDelegate: Send + Sync {
    /// Attempt to open an sound file with the given name. If it doesn't exist,
    /// an IO error occurs, you can't identify the format, or whatever, you
    /// should display or log an error message using an application-specific
    /// mechanism, then return `None`.
    fn open_file(&self, name: &str) -> Option<FormattedSoundStream>;
    /// Present and/or log a warning.
    fn warning(&self, message: &str) {
        eprintln!("SMS warning: {}", message);
    }
}

/// This is an object that SMS will hang onto, representing an ongoing decoding
/// of a particular underlying file. SMS will either use this to populate a
/// cache or to stream it directly, depending on the Soundtrack's
/// configuration.
pub trait SoundReader<T: Sample>: Send {
    /// Produce some sound, placing it into the target buffer.
    /// 
    /// Return the number of *samples* (not *sample frames*) that were written
    /// to buf. If this is not *exactly* equal to the size of the buf, then the
    /// stream is assumed to have been ended; either it will be disposed of,
    /// or `seek` will be called.
    fn read(&mut self, buf: &mut [MaybeUninit<T>]) -> usize;
    /// Attempt to seek to the given *sample frame count* from the beginning of
    /// the file. Imprecision is permitted in one direction only: seeking is
    /// permitted to end up earlier than the target, but not later. Returns the
    /// actual *sample frame count*, measured from the beginning of the stream,
    /// that was seeked to.
    /// 
    /// This number must be exact! If you can't provide an exact timestamp,
    /// don't provide seeking! (SMS will work around it.) Again, it's okay if
    /// you can't *seek to an exact timestamp*, but you *do* need to be able to
    /// *know where you've seeked to* and *not seek too late*.
    /// 
    /// Returns None if seeking failed or is impossible, in which case, SMS
    /// will reopen the file instead. Default implementation returns None.
    #[allow(unused_variables)]
    fn seek(&mut self, pos: u64) -> Option<u64> { None }
    /// Attempt to skip exactly the given number of *samples*. Failure is not
    /// an option. Returns true if there is more sound data to come, false if
    /// we have reached the end of the sound.
    ///
    /// The default implementation will try to use `skip_coarse` to skip
    /// ahead, and then repeatedly `read` into the target buffer until the
    /// exact number of target samples are consumed.
    /// 
    /// `buf` is provided as scratch space.
    fn skip_precise(&mut self, count: u64, buf: &mut [MaybeUninit<T>]) -> bool {
        let mut count = count.checked_sub(self.skip_coarse(count, buf))
            .expect("bug in program's sound delegate: skip_coarse skipped too many samples!");
        while count > 0 {
            let amt = (buf.len() as u64).min(count) as usize;
            let red = self.read(&mut buf[..amt]);
            if red == 0 {
                // premature end? uh oh
                return false
            }
            count -= red as u64;
        }
        true
    }
    /// Attempt to efficiently skip *up to* a large number of *samples*, by
    /// discarding partial buffers, skipping packets, seeking in the file,
    /// etc. Return the number of *samples* skipped, possibly including zero.
    /// 
    /// Default implementation just returns 0.
    /// 
    /// `buf` is provided as scratch space.
    #[allow(unused_variables)]
    fn skip_coarse(&mut self, count: u64, buf: &mut [MaybeUninit<T>]) -> u64 {
        0
    }
    /// Returns true if this decoder can be cheaply cloned, false otherwise.
    /// 
    /// Default implementation assumes non-cloneability.
    fn can_be_cloned(&self) -> bool { false }
    /// If this is a cloneable decoder, returns Some(clone). If not, returns
    /// None. If you claim that you are cloneable, you *must* always succeed!
    fn attempt_clone(&mut self) -> Option<Box<dyn SoundReader<T>>> { None }
    /// Attempt to estimate how many *sample frames* are in the entire file,
    /// from beginning to end. This is a BEST GUESS ESTIMATE and may not
    /// reflect the actual value!
    fn estimate_len(&mut self) -> Option<u64> { None }
}

impl FormattedSoundReader {
    /// Attempt to seek to the given *sample frame count* from the beginning of
    /// the file. Imprecision is permitted in one direction only: seeking is
    /// permitted to end up earlier than the target, but not later. Returns the
    /// actual *sample frame count*, measured from the beginning of the stream,
    /// that was seeked to. This number must be exact! If you can't provide an
    /// exact timestamp, don't provide seeking! (SMS will work around it.)
    /// 
    /// Returns None if seeking failed or is impossible, in which case, SMS
    /// will reopen the file instead. Default implementation returns None.
    #[allow(unused_variables)]
    pub fn seek(&mut self, pos: u64) -> Option<u64> {
        match self {
            FormattedSoundReader::U8(x) => x.seek(pos),
            FormattedSoundReader::U16(x) => x.seek(pos),
            FormattedSoundReader::I8(x) => x.seek(pos),
            FormattedSoundReader::I16(x) => x.seek(pos),
            FormattedSoundReader::F32(x) => x.seek(pos),
        }
    }
    /// Attempt to clone the decoder, if the decoder can be cloned cheaply.
    /// Returns None if it's not cheap to decode the decoder, or something goes
    /// wrong in the process. Default implementation returns None.
    pub fn attempt_clone(&mut self) -> Option<FormattedSoundReader> {
        match self {
            FormattedSoundReader::U8(x) => x.attempt_clone().map(|x| FormattedSoundReader::U8(x)),
            FormattedSoundReader::U16(x) => x.attempt_clone().map(|x| FormattedSoundReader::U16(x)),
            FormattedSoundReader::I8(x) => x.attempt_clone().map(|x| FormattedSoundReader::I8(x)),
            FormattedSoundReader::I16(x) => x.attempt_clone().map(|x| FormattedSoundReader::I16(x)),
            FormattedSoundReader::F32(x) => x.attempt_clone().map(|x| FormattedSoundReader::F32(x)),
        }
    }
}