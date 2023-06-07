//! The loop adapter serves two purposes:
//! 
//! 1. Applying fade in, loop length, and fade out to a Sound
//! 2. Possibly, converting the stream from its native format to f32

use super::*;

/// note: except one field, this struct deals in samples, NOT sample frames!
struct LoopAdapter<T: Sample> {
    source_stream: Box<dyn SoundReader<T>>,
    speaker_layout: SpeakerLayout,
    /// Number of samples left to output before the next restarting of the
    /// inner loop. If `None`, we won't loop again.
    samples_till_next_inner_loop: Option<u64>,
    /// Number of samples left to output before the requested length is over.
    /// If `None`, the requested length is over, and the fade out (if any) has
    /// begun.
    samples_till_end_of_outer_loop: Option<u64>,
    /// Number of samples left to output before we're done.
    samples_left: u64,
    /// How many sample frames to seek to within the looped stream to restart
    /// the inner loop.
    inner_loop_start_sampleframes: u64,
    /// Number of samples to output every time through the inner loop. The
    /// value that `samples_till_next_inner_loop` will get set to when an inner
    /// loop happens.
    inner_loop_length_samples: Option<u64>,
    /// If `Some`, fade in is taking place. If `None`, fade in is complete.
    fade_in: Option<Fader>,
    /// If `Some`, fade out will begin once `samples_till_end_of_outer_loop`
    /// becomes `None`, and the stream will end when fade out is complete. If
    /// `None`, fade out will not occur, and the stream will end when it ends.
    fade_out: Option<Fader>,
    /// - Release ON: we stop the "inner loop" after completing the outer loop.
    ///   A fade out <= 0.0 means we *do not* fade out. (fade_out will be None)
    /// - Release OFF: we DO NOT stop the "inner loop", even after completing
    ///   the outer loop. A fade out <= 0.0 means we *instantly* fade out.
    ///   (fade_out will be some instant fade-out fader)
    release: bool,
    buf: Vec<MaybeUninit<T>>,
}

impl<T: Sample> LoopAdapter<T> {
    fn new(
        sound: &Sound,
        fade_in: f32, key_down_length: Option<f32>, fade_out: f32, release: bool,
        sample_rate: f32,
        speaker_layout: SpeakerLayout,
        source_stream: Box<dyn SoundReader<T>>,
    ) -> Box<dyn SoundReader<f32>> {
        let num_channels = speaker_layout.get_num_channels() as u64;
        let samples_till_next_inner_loop;
        let inner_loop_start_sampleframes;
        let inner_loop_length_samples;
        let samples_till_end_of_outer_loop;
        let samples_left;
        if let Some(how_long_to_loop_it_for) = key_down_length {
            if let Some(loop_end) = sound.loop_end {
                let start_sampleframes = (sound.start * sample_rate).floor() as u64;
                let loop_start_sampleframes = (sound.loop_start * sample_rate).floor() as u64;
                let loop_end_sampleframes = (loop_end * sample_rate).ceil() as u64;
                inner_loop_start_sampleframes = loop_start_sampleframes;
                inner_loop_length_samples = Some((loop_end_sampleframes - loop_start_sampleframes) * num_channels);
                samples_till_next_inner_loop = Some((loop_end_sampleframes - start_sampleframes) * num_channels);
            } else {
                (samples_till_next_inner_loop, inner_loop_start_sampleframes, inner_loop_length_samples) = (None, 0, None)
            }
            samples_till_end_of_outer_loop = Some((how_long_to_loop_it_for * sample_rate).floor() as u64 * num_channels);
            samples_left = ((how_long_to_loop_it_for + fade_out.max(0.0)) * sample_rate).ceil() as u64 * num_channels;
        }
        else {
            // If `length` is specified as `None`, then we don't even have an
            // inner loop at all.
            inner_loop_start_sampleframes = 0;
            inner_loop_length_samples = None;
            samples_till_next_inner_loop = None;
            // but we might have a fade out, and we do still need to cut off
            // playback at sound.end
            samples_till_end_of_outer_loop = Some(((sound.end - sound.start - fade_out.max(0.0)) * sample_rate).ceil() as u64 * num_channels);
            samples_left = ((sound.end - sound.start) * sample_rate).ceil() as u64 * num_channels;
        }
        let fade_out = if release {
            Fader::maybe_start(FadeType::Linear, 1.0, 0.0, fade_out)
        }
        else {
            Some(Fader::start(FadeType::Linear, 1.0, 0.0, fade_out))
        };
        Box::new(LoopAdapter {
            source_stream,
            speaker_layout,
            samples_till_next_inner_loop,
            samples_till_end_of_outer_loop,
            samples_left,
            inner_loop_start_sampleframes,
            inner_loop_length_samples,
            fade_in: Fader::maybe_start(FadeType::Linear, 0.0, 1.0, fade_in),
            fade_out,
            release,
            buf: vec![MaybeUninit::uninit(); 64],
        })
    }
}

impl<T: Sample> SoundReader<f32> for LoopAdapter<T> {
    fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
        if self.samples_left == 0 { return 0 }
        if let Some(0) = self.samples_till_end_of_outer_loop {
            if self.release {
                self.samples_till_next_inner_loop = None;
            }
            self.samples_till_end_of_outer_loop = None;
        }
        let should_fade_out = self.samples_till_end_of_outer_loop.is_none();
        let did_loop = if let Some(0) = self.samples_till_next_inner_loop {
            let new_pos = self.source_stream.seek(self.inner_loop_start_sampleframes)
                .expect("SMS bug: Loop adapter expects loopable stream, but SoundMan didn't provide one!");
            if new_pos > self.inner_loop_start_sampleframes {
                panic!("bug in program's sound delegate: seek seeked past the requested timestamp!");
            }
            let samples_to_skip = (self.inner_loop_start_sampleframes - new_pos) * self.speaker_layout.get_num_channels() as u64;
            if samples_to_skip > 0 {
                self.source_stream.skip_precise(samples_to_skip, &mut self.buf[..]);
            }
            self.samples_till_next_inner_loop = self.inner_loop_length_samples;
            true
        } else { false };
        let mut amount_to_read = out.len() as u64;
        if let Some(nelo) = self.samples_till_next_inner_loop {
            amount_to_read = amount_to_read.min(nelo);
        }
        if let Some(endo) = self.samples_till_end_of_outer_loop {
            amount_to_read = amount_to_read.min(endo);
        }
        amount_to_read = amount_to_read.min(self.samples_left);
        let amount_to_read = if amount_to_read > usize::MAX as u64 {
            usize::MAX - usize::MAX % self.speaker_layout.get_num_channels()
        } else { amount_to_read as usize };
        if amount_to_read % self.speaker_layout.get_num_channels() != 0 {
            panic!("bug in SMS: not reading a whole number of sample frames at a time");
        }
        // TODO: don't use `self.buf` as an intermediary if we've wrapped an
        // f32 stream
        if self.buf.len() < amount_to_read {
            self.buf.resize_with(amount_to_read, MaybeUninit::uninit);
        }
        let amount_read = self.source_stream.read(&mut self.buf[..amount_to_read]);
        if amount_read % self.speaker_layout.get_num_channels() != 0 {
            panic!("bug in program's sound delegate: didn't read a whole number of sample frames at a time");
        }
        debug_assert!(amount_read <= amount_to_read);
        self.samples_left -= amount_read as u64;
        if let Some(nelo) = self.samples_till_next_inner_loop.as_mut() {
            *nelo -= amount_read as u64;
        }
        if let Some(endo) = self.samples_till_end_of_outer_loop.as_mut() {
            *endo -= amount_read as u64;
        }
        if amount_read == 0 {
            if did_loop {
                // TODO: warning about loop being past end of sample
                return 0
            }
            else if self.samples_till_next_inner_loop.is_some() {
                self.samples_till_next_inner_loop = Some(0);
                // TODO: loop instead of recursing
                return self.read(out)
            }
            else {
                return 0
            }
        }
        out[..amount_read].iter_mut().zip(self.buf[..amount_read].iter())
        .for_each(|(o,i)| {
            *o = MaybeUninit::new(unsafe { i.assume_init_ref() }.to_float_sample());
        });
        let out: &mut[f32] = unsafe { std::mem::transmute(&mut out[..]) }; // TODO: this isn't okay, tracking issue 63569
        if let Some(fade_in) = self.fade_in.as_mut() {
            // TODO: factor this logic into a method because DRY
            let mut out_n = 0;
            let mut in_n = 0;
            while out_n < amount_read {
                let eval = fade_in.evaluate_t(in_n as f32);
                in_n += 1;
                for _ in 0 .. self.speaker_layout.get_num_channels() {
                    out[out_n] *= eval;
                    out_n += 1;
                }
            }
            if fade_in.complete() {
                self.fade_in = None;
            }
            else {
                fade_in.step_by(amount_read as f32);
            }
        }
        if self.samples_till_end_of_outer_loop.is_none() {
            if let Some(fade_out) = self.fade_out.as_mut() {
                let mut out_n = 0;
                let mut in_n = 0;
                while out_n < amount_read {
                    let eval = fade_out.evaluate_t(in_n as f32);
                    in_n += 1;
                    for _ in 0 .. self.speaker_layout.get_num_channels() {
                        out[out_n] *= eval;
                        out_n += 1;
                    }
                }
                if fade_out.complete() {
                    self.fade_out = None;
                    self.samples_left = 0;
                }
                else {
                    fade_out.step_by(amount_read as f32);
                }
            }
        }
        amount_read
    }
    // TODO: implement skip
    fn seek(&mut self, _pos: u64) -> Option<u64> {
        panic!("SMS logic error: attempt to seek a loop adapter");
    }
    fn estimate_len(&mut self) -> Option<u64> {
        panic!("SMS logic error: attempt to estimate length of a loop adapter");
    }
}

pub(crate) fn new_loop_adapter(
    sound: &Sound, stream: FormattedSoundStream,
    fade_in: f32, length: Option<f32>, fade_out: f32, release: bool,
) -> Box<dyn SoundReader<f32>> {
    let FormattedSoundStream { sample_rate, speaker_layout, reader }
        = stream;
    match reader {
        FormattedSoundReader::U8(x) => LoopAdapter::new(sound, fade_in, length, fade_out, release, sample_rate, speaker_layout, x),
        FormattedSoundReader::U16(x) => LoopAdapter::new(sound, fade_in, length, fade_out, release, sample_rate, speaker_layout, x),
        FormattedSoundReader::I8(x) => LoopAdapter::new(sound, fade_in, length, fade_out, release, sample_rate, speaker_layout, x),
        FormattedSoundReader::I16(x) => LoopAdapter::new(sound, fade_in, length, fade_out, release, sample_rate, speaker_layout, x),
        FormattedSoundReader::F32(x) => LoopAdapter::new(sound, fade_in, length, fade_out, release, sample_rate, speaker_layout, x),
    }
}
