use std::{
    fmt::Debug,
    mem::MaybeUninit,
};

use crate::{PosFloat, SoundReader};

/// Something that has opinions on how loud a particular mixer channel should
/// be.
pub(crate) trait VolumeGetter<ID: Debug> {
    /// Called after every output buffer. You should step all your faders by
    /// the given number of sample frames.
    fn step_faders_by(&mut self, #[allow(unused)] n: PosFloat) {}
    /// If the sound should stop playing, returns `None`. If the sound should
    /// play at a specific volume, returns `Some(volume)` instead. `t` is the
    /// number of steps in the future to get the value for, where `0.0` is the
    /// first sample frame of a buffer, `1.0` is the second, etc.
    ///
    /// `Some(ZERO)` is a perfectly valid volume, and we still need to consume
    /// audio in that case.
    ///
    /// We assume that if `Some(0.0)` is returned, any further calls (in, say,
    /// a tight loop) will not result in a non-zero return. If you allow phase
    /// inversion (which... why?) then this assumption will not hold.
    fn get_volume(&mut self, identity: &ID, t: PosFloat) -> Option<PosFloat>;
    /// Return `None` if the sound should stop, `Some(false)` if the sound
    /// is currently playing at a fixed volume, or `Some(true)` if the sound
    /// is currently playing at a varying volume.
    ///
    /// Guaranteed to be called exactly once per playing sample per output mix
    /// buffer, before any calls to `get_volume`. You can use this to, for
    /// example, mark each components of the given identity as individually
    /// being still relevant.
    fn is_varying(&mut self, identity: &ID) -> Option<bool>;
}

impl<'a, T: VolumeGetter<ID>, ID: Debug> VolumeGetter<ID> for &'a mut T {
    fn step_faders_by(&mut self, n: PosFloat) {
        (*self).step_faders_by(n)
    }
    fn get_volume(&mut self, identity: &ID, t: PosFloat) -> Option<PosFloat> {
        (*self).get_volume(identity, t)
    }
    fn is_varying(&mut self, identity: &ID) -> Option<bool> {
        (*self).is_varying(identity)
    }
}

struct Channel<ID: Debug> {
    stream: Box<dyn SoundReader<f32>>,
    identity: ID,
}

pub(crate) struct Mixer<ID: Debug> {
    channels: Vec<Channel<ID>>,
    /// Number of *samples* per *sample frame*. Number of *channels* of output
    /// audio.
    samples_per_frame: usize,
    next_output_sample_frame_number: u64,
}

impl<ID: Debug> Mixer<ID> {
    pub fn new(samples_per_frame: usize) -> Mixer<ID> {
        Mixer {
            channels: vec![],
            samples_per_frame,
            next_output_sample_frame_number: 0,
        }
    }
    pub fn play(&mut self, stream: Box<dyn SoundReader<f32>>, identity: ID) {
        self.channels.push(Channel {
            stream,
            identity,
        });
    }
    /// Returns true if the channel lived, false if the channel died.
    fn mix_channel<T: VolumeGetter<ID>>(channel: &mut Channel<ID>, mut out: &mut[f32], mix_buf: &mut[MaybeUninit<f32>], mut volume_getter: T, samples_per_frame: usize) -> bool {
        let mut accum_len = 0;
        while !out.is_empty() {
            debug_assert!(out.len() % samples_per_frame == 0);
            debug_assert!(out.len() <= mix_buf.len());
            let stream = &mut channel.stream;
            let identity = &channel.identity;
            let is_varying = volume_getter.is_varying(identity);
            let len = match is_varying {
                None => {
                    return false;
                },
                Some(false) => {
                    // Time to mix!
                    let out_frames = out.len() / samples_per_frame;
                    // (use the volume at the halfway point)
                    let t = PosFloat::from(out_frames) * PosFloat::HALF;
                    // Cache the volume given by the VolumeGetter, and use it
                    // for the whole buffer. We can do this because the volume
                    // is not currently varying.
                    let volume = volume_getter.get_volume(identity, t);
                    match volume {
                        // we're done here
                        None => {
                            return false
                        },
                        Some(volume) => {
                            if volume == PosFloat::ZERO {
                                if !stream.skip_precise(out.len() as u64, mix_buf) {
                                    return false
                                }
                                out.len()
                            }
                            else if volume == PosFloat::ONE {
                                // easy mode
                                let len = stream.read(&mut mix_buf[..out.len()]);
                                assert!(len % samples_per_frame == 0);
                                for x in 0 .. len {
                                    out[x] += unsafe { *mix_buf[x].assume_init_ref() };
                                }
                                len
                            }
                            else {
                                // hard mode
                                let len = stream.read(&mut mix_buf[..out.len()]);
                                assert!(len % samples_per_frame == 0);
                                for x in 0 .. len {
                                    out[x] += unsafe { *mix_buf[x].assume_init_ref() }
                                        * *volume;
                                }
                                len
                            }
                        },
                    }
                },
                Some(true) => {
                    // Time to bix!
                    // We will have to call GetVolume every sample frame,
                    // because the volume is currently varying.
                    let mut time_accumulator = PosFloat::HALF;
                    let len = stream.read(&mut mix_buf[..out.len()]);
                    assert!(len % samples_per_frame == 0);
                    for x in (0 .. len).step_by(samples_per_frame) {
                        let volume = volume_getter.get_volume(identity, time_accumulator);
                        time_accumulator = time_accumulator + PosFloat::ONE;
                        match volume {
                            // we're done here
                            None => {
                                return false
                            },
                            Some(volume) => {
                                if volume == PosFloat::ZERO {
                                    // we have nothing to mix, and we assume
                                    // we won't for the rest of the buffer
                                    break
                                }
                                else if volume == PosFloat::ONE {
                                    // easy mode
                                    for x in x .. x + samples_per_frame {
                                        out[x] += unsafe { *mix_buf[x].assume_init_ref() };
                                    }
                                }
                                else {
                                    // hard mode
                                    for x in x .. x + samples_per_frame {
                                        out[x] += unsafe { *mix_buf[x].assume_init_ref() }
                                            * *volume;
                                    }
                                }
                            },
                        }
                    }
                    len
                },
            };
            if len == 0 {
                // (Maybe) done outputting forever
                return accum_len != 0
            }
            else if len < out.len() {
                // Need to mix a little bit more
                out = &mut out[len..];
                accum_len += len;
                continue
            }
            else {
                debug_assert_eq!(len, out.len());
                // All done for now
                return true
            }
        }
        true
    }
    /// Adds the active sounds to `out`. Unless you're combining more than one
    /// `Mixer`, you definitely *definitely* want to zero `out`.
    pub fn mix<T: VolumeGetter<ID>>(&mut self, out: &mut[f32], mix_buf: &mut [MaybeUninit<f32>], mut volume_getter: T) {
        debug_assert!(out.len() % self.samples_per_frame == 0);
        debug_assert_eq!(out.len(), mix_buf.len());
        self.channels.retain_mut(|channel| {
            Self::mix_channel(channel, out, mix_buf, &mut volume_getter, self.samples_per_frame)
        });
        #[cfg(feature="debug-channels")]
        {
            if let Some(mut target) = MIX_CHANNELS.try_lock() {
                let report = self.channels.iter().map(|x| {
                    let volume = volume_getter.get_volume(&x.identity, PosFloat::ZERO);
                    let blah = format!("{:?}", x.identity);
                    (volume, blah)
                }).collect();
                *target = report;
            }
        }
        let out_frames = out.len() / self.samples_per_frame;
        volume_getter.step_faders_by(out_frames.into());
        self.next_output_sample_frame_number = self.next_output_sample_frame_number.wrapping_add(out_frames as u64);
    }
    /// Similar to `mix` with empty buffers. Use this if you desperately need
    /// the mixer to notice that some sounds have died.
    pub fn bump<T: VolumeGetter<ID>>(&mut self, mut volume_getter: T) {
        self.channels.retain(|channel| {
            volume_getter.is_varying(&channel.identity).is_some()
        });
    }
    /// Returns the sample *frame* number of the next output sample frame.
    /// Every time you call `mix`, this will increase by the number of sample
    /// *frames* you mix.
    pub fn get_next_output_sample_frame_number(&self) -> u64 {
        self.next_output_sample_frame_number
    }
}

#[cfg(feature="debug-channels")]
pub static MIX_CHANNELS: parking_lot::Mutex<Vec<(Option<PosFloat>, String)>> = parking_lot::Mutex::new(vec![]);
