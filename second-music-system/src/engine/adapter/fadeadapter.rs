use super::*;

/// The loop adapter serves two purposes:
/// 
/// 1. Applying fade in, playback length, and fade out to a Sound
/// 2. Possibly, converting the stream from its native format to f32
/// 
/// note: this struct deals in samples, NOT sample frames!
struct FadeAdapter<T: Sample> {
    source_stream: Box<dyn SoundReader<T>>,
    speaker_layout: SpeakerLayout,
    /// Number of samples left to output before the requested length is over.
    /// If zero, the requested length is over, and the fade out (if any) has
    /// begun.
    samples_till_fade_out: u64,
    /// Number of samples left to output before we're done.
    samples_left: u64,
    /// If `Some`, fade in is taking place. If `None`, fade in is complete.
    fade_in: Option<Fader>,
    /// If `Some`, fade out will begin once `samples_till_end_of_outer_loop`
    /// becomes `None`, and the stream will end when fade out is complete. If
    /// `None`, fade out will not occur, and the stream will end when it ends.
    fade_out: Option<Fader>,
    buf: Vec<MaybeUninit<T>>,
}

impl<T: Sample> FadeAdapter<T> {
    fn new(
        sound: &Sound,
        fade_in: PosFloat,
        how_long_to_play_before_fade: Option<PosFloat>,
        fade_out: PosFloat,
        sample_rate: PosFloat,
        speaker_layout: SpeakerLayout,
        source_stream: Box<dyn SoundReader<T>>,
    ) -> Box<dyn SoundReader<f32>> {
        let num_channels = speaker_layout.get_num_channels() as u64;
        let samples_in_sound = ((sound.end.saturating_sub(sound.start)) * sample_rate).ceil() as u64 * num_channels;
        let samples_till_fade_out;
        let samples_left;
        if let Some(how_long_to_play_before_fade) = how_long_to_play_before_fade {
            samples_till_fade_out = (how_long_to_play_before_fade * sample_rate).floor() as u64 * num_channels;
            samples_left = (((how_long_to_play_before_fade + fade_out.max(PosFloat::ZERO)) * sample_rate).ceil() as u64 * num_channels)
                .min(samples_in_sound);
        }
        else {
            samples_left = samples_in_sound;
            samples_till_fade_out = samples_left;
        }
        Box::new(FadeAdapter {
            source_stream,
            speaker_layout,
            samples_till_fade_out,
            samples_left,
            fade_in: Fader::maybe_start(FadeType::Linear, PosFloat::ZERO, PosFloat::ONE, fade_in),
            fade_out: Fader::maybe_start(FadeType::Linear, PosFloat::ONE, PosFloat::ZERO, fade_out),
            buf: vec![MaybeUninit::uninit(); 64],
        })
    }
}

impl<T: Sample> SoundReader<f32> for FadeAdapter<T> {
    fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
        if self.samples_left == 0 { return 0 }
        let mut amount_to_read = out.len() as u64;
        amount_to_read = amount_to_read.min(self.samples_till_fade_out);
        amount_to_read = amount_to_read.min(self.samples_left);
        let amount_to_read = if amount_to_read > usize::MAX as u64 {
            // this is ridiculous, but... try to read as many as possible,
            // WITHOUT reading a partial frame
            usize::MAX - usize::MAX % self.speaker_layout.get_num_channels()
        } else { amount_to_read as usize };
        if amount_to_read % self.speaker_layout.get_num_channels() != 0 {
            panic!("bug in SMS: not reading whole sample frames at a time");
        }
        // TODO: don't use `self.buf` as an intermediary if we've wrapped an
        // f32 stream
        if self.buf.len() < amount_to_read {
            self.buf.resize_with(amount_to_read, MaybeUninit::uninit);
        }
        let amount_read = self.source_stream.read(&mut self.buf[..amount_to_read]);
        if amount_read % self.speaker_layout.get_num_channels() != 0 {
            panic!("bug in program's sound delegate: didn't read a whole sample frame at a time");
        }
        debug_assert!(amount_read <= amount_to_read);
        if amount_read == 0 {
            // we hit the end. prematurely? don't care. nothing left for us here
            self.samples_left = 0;
            return 0
        }
        out[..amount_read].iter_mut().zip(self.buf[..amount_read].iter())
        .for_each(|(o,i)| {
            *o = MaybeUninit::new(unsafe { i.assume_init_ref() }.to_float_sample());
        });
        let out: &mut[f32] = unsafe { std::mem::transmute(&mut out[..]) }; // TODO: this isn't okay, tracking issue 63569
        if let Some(fade_in) = self.fade_in.as_mut() {
            // TODO: factor this logic into a method because DRY
            let mut out_n = 0;
            let mut in_n: usize = 0;
            while out_n < amount_read {
                let eval = fade_in.evaluate_t(in_n.into());
                in_n += 1;
                for _ in 0 .. self.speaker_layout.get_num_channels() {
                    out[out_n] *= *eval;
                    out_n += 1;
                }
            }
            if fade_in.complete() {
                self.fade_in = None;
            }
            else {
                fade_in.step_by(amount_read.into());
            }
        }
        if self.samples_till_fade_out == 0 {
            if let Some(fade_out) = self.fade_out.as_mut() {
                let mut out_n = 0;
                let mut in_n: usize = 0;
                while out_n < amount_read {
                    let eval = fade_out.evaluate_t(in_n.into());
                    in_n += 1;
                    for _ in 0 .. self.speaker_layout.get_num_channels() {
                        out[out_n] *= *eval;
                        out_n += 1;
                    }
                }
                if fade_out.complete() {
                    self.fade_out = None;
                    self.samples_left = 0;
                }
                else {
                    fade_out.step_by(amount_read.into());
                }
            }
        }
        if self.samples_till_fade_out > 0 {
            self.samples_till_fade_out -= amount_read as u64;
        }
        self.samples_left -= amount_read as u64;
        amount_read
    }
    fn skip_coarse(&mut self, count: u64, buf: &mut [MaybeUninit<f32>]) -> u64 {
        // This transmutation is super icky, but since it's a scratch buf that
        // will not be aliased by code that cares about its values, and since
        // f32 has more pessimistic alignment than any other sample type, it
        // should be fine...
        let buf = unsafe { std::mem::transmute(buf) };
        let result = self.source_stream.skip_coarse(count, buf);
        if result > 0 {
            self.samples_till_fade_out = self.samples_till_fade_out.saturating_sub(result);
            self.samples_left = self.samples_left.saturating_sub(result);
        }
        result
    }
    // Ick. This is the same as the trait default implementation, except that
    // we transmute buf and call our inner implementation, then adjust our
    // counts accordingly.
    fn skip_precise(&mut self, count: u64, buf: &mut [MaybeUninit<f32>]) -> bool {
        let buf: &mut [MaybeUninit<T>] = unsafe { std::mem::transmute(buf) };
        let mut rem = count.checked_sub(self.source_stream.skip_coarse(count, buf))
            .expect("bug in program's sound delegate: skip_coarse skipped too many samples!");
        while rem > 0 {
            let amt = (buf.len() as u64).min(rem) as usize;
            let red = self.source_stream.read(&mut buf[..amt]);
            if red == 0 {
                // premature end? uh oh
                self.samples_left = 0;
                return false
            }
            rem -= red as u64;
        }
        self.samples_till_fade_out = self.samples_till_fade_out.saturating_sub(count);
        self.samples_left = self.samples_left.saturating_sub(count);
        true
    }
    fn seek(&mut self, _pos: u64) -> Option<u64> {
        panic!("SMS logic error: attempt to seek a loop adapter");
    }
    fn estimate_len(&mut self) -> Option<u64> {
        panic!("SMS logic error: attempt to estimate length of a loop adapter");
    }
}

pub(crate) fn new_fade_adapter(
    sound: &Sound, stream: FormattedSoundStream,
    fade_in: PosFloat, length: Option<PosFloat>, fade_out: PosFloat,
) -> Box<dyn SoundReader<f32>> {
    let FormattedSoundStream { sample_rate, speaker_layout, reader }
        = stream;
    match reader {
        FormattedSoundReader::U8(x) => FadeAdapter::new(sound, fade_in, length, fade_out, sample_rate, speaker_layout, x),
        FormattedSoundReader::U16(x) => FadeAdapter::new(sound, fade_in, length, fade_out, sample_rate, speaker_layout, x),
        FormattedSoundReader::I8(x) => FadeAdapter::new(sound, fade_in, length, fade_out, sample_rate, speaker_layout, x),
        FormattedSoundReader::I16(x) => FadeAdapter::new(sound, fade_in, length, fade_out, sample_rate, speaker_layout, x),
        FormattedSoundReader::F32(x) => FadeAdapter::new(sound, fade_in, length, fade_out, sample_rate, speaker_layout, x),
    }
}
