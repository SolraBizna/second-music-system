//! The channel adapter takes in a stream with one speaker layout, and outputs
//! a stream with a different speaker layout.

// some of the macros check buffer length against channel count, this prevents
// Clippy from getting mad when the channel count is 1
#![allow(clippy::modulo_one)]

use super::*;

#[macro_use] mod macros;

// Important note: Order of outputs and inputs matters.

///////////////////////////////////////////////////////////////////////////////
// --- Mono source ---
///////////////////////////////////////////////////////////////////////////////

make_upmixer!(MonoToStereo(c) -> {
    fl = c;
    fr = c;
});

type MonoToHeadphones = MonoToStereo;

make_upmixer!(MonoToQuadraphonic(c) -> {
    fl = c;
    fr = c;
    rl = 0.0;
    rr = 0.0;
});

make_upmixer!(MonoToSurround51(c) -> {
    fl = 0.0;
    fr = 0.0;
    c = c;
    lfe = 0.0;
    rl = 0.0;
    rr = 0.0;
});

make_upmixer!(MonoToSurround71(c) -> {
    fl = 0.0;
    fr = 0.0;
    c = c;
    lfe = 0.0;
    rl = 0.0;
    rr = 0.0;
    sl = 0.0;
    sr = 0.0;
});

///////////////////////////////////////////////////////////////////////////////
// --- Stereo source ---
///////////////////////////////////////////////////////////////////////////////

make_downmixer!(StereoToMono(fl, fr) -> {
    c = (fl+fr) * (1.0 / 2.0);
});

make_upmixer!(StereoToQuadraphonic(fl, fr) -> {
    fl = fl;
    fr = fr;
    rl = 0.0;
    rr = 0.0;
});

make_upmixer!(StereoToSurround51(fl, fr) -> {
    fl = fl;
    fr = fr;
    c = 0.0;
    lfe = 0.0;
    rl = 0.0;
    rr = 0.0;
});

make_upmixer!(StereoToSurround71(fl, fr) -> {
    fl = fl;
    fr = fr;
    c = 0.0;
    lfe = 0.0;
    rl = 0.0;
    rr = 0.0;
    sl = 0.0;
    sr = 0.0;
});

///////////////////////////////////////////////////////////////////////////////
// --- Headphones source ---
///////////////////////////////////////////////////////////////////////////////

type HeadphonesToMono = StereoToMono;
type HeadphonesToQuadraphonic = StereoToQuadraphonic;
type HeadphonesToSurround51 = StereoToSurround51;
type HeadphonesToSurround71 = StereoToSurround71;

///////////////////////////////////////////////////////////////////////////////
// --- Quadraphonic source ---
///////////////////////////////////////////////////////////////////////////////

make_downmixer!(QuadraphonicToMono(fl, fr, rl, rr) -> {
    c = (fl+fr+rl+rr) * (1.0 / 4.0);
});

make_downmixer!(QuadraphonicToStereo(fl, fr, rl, rr) -> {
    fl = (fl+rl) * (1.0 / 2.0);
    fr = (fr+rr) * (1.0 / 2.0);
});

type QuadraphonicToHeadphones = QuadraphonicToStereo;

make_upmixer!(QuadraphonicToSurround51(fl, fr, rl, rr) -> {
    fl = fl;
    fr = fr;
    c = 0.0;
    lfe = 0.0;
    rl = rl;
    rr = rr;
});

make_upmixer!(QuadraphonicToSurround71(fl, fr, rl, rr) -> {
    fl = fl;
    fr = fr;
    c = 0.0;
    lfe = 0.0;
    rl = rl;
    rr = rr;
    sl = 0.0;
    sr = 0.0;
});

///////////////////////////////////////////////////////////////////////////////
// --- Surround 5.1 source ---
///////////////////////////////////////////////////////////////////////////////

make_downmixer!(Surround51ToMono(fl, fr, c, _lfe, rl, rr) -> {
    c = (fl+fr+c+rl+rr) * (1.0 / 5.0);
});

make_downmixer!(Surround51ToStereo(fl, fr, c, _lfe, rl, rr) -> {
    fl = (fl+rl+c*0.5) * (1.0 / 2.5);
    fr = (fr+rr+c*0.5) * (1.0 / 2.5);
});

type Surround51ToHeadphones = Surround51ToStereo;

make_downmixer!(Surround51ToQuadraphonic(fl, fr, c, _lfe, rl, rr) -> {
    fl = (fl+c*0.5) * (1.0 / 1.5);
    fr = (fr+c*0.5) * (1.0 / 1.5);
    rl = rl * (1.0 / 1.5);
    rr = rr * (1.0 / 1.5);
});

make_upmixer!(Surround51ToSurround71(fl, fr, c, lfe, rl, rr) -> {
    fl = fl;
    fr = fr;
    c = c;
    lfe = lfe;
    rl = rl;
    rr = rr;
    sl = 0.0;
    sr = 0.0;
});

///////////////////////////////////////////////////////////////////////////////
// --- Surround 7.1 source ---
///////////////////////////////////////////////////////////////////////////////

make_downmixer!(Surround71ToMono(fl, fr, c, _lfe, rl, rr, sl, sr) -> {
    c = (fl+fr+c+rl+rr+sl+sr) * (1.0 / 7.0);
});

make_downmixer!(Surround71ToStereo(fl, fr, c, _lfe, rl, rr, sl, sr) -> {
    fl = (fl+sl+rl+c*0.5) * (1.0 / 3.5);
    fr = (fr+sr+rr+c*0.5) * (1.0 / 3.5);
});

type Surround71ToHeadphones = Surround71ToStereo;

make_downmixer!(Surround71ToQuadraphonic(fl, fr, c, _lfe, rl, rr, sl, sr) -> {
    fl = (fl+sl*0.5+c*0.5) * (1.0 / 2.0);
    fr = (fr+sr*0.5+c*0.5) * (1.0 / 2.0);
    rl = (rl+sl*0.5) * (1.0 / 2.0);
    rr = (rr+sr*0.5) * (1.0 / 2.0);
});

make_downmixer!(Surround71ToSurround51(fl, fr, c, lfe, rl, rr, sl, sr) -> {
    fl = (fl+sl*0.5) * (1.0 / 1.5);
    fr = (fr+sr*0.5) * (1.0 / 1.5);
    c = c * (1.0 / 1.5);
    lfe = lfe * (1.0 / 1.5);
    rl = (rl+sl*0.5) * (1.0 / 1.5);
    rr = (rr+sr*0.5) * (1.0 / 1.5);
});

pub(crate) fn new_channel_adapter(in_stream: Box<dyn SoundReader<f32>>, sample_rate: PosFloat, in_layout: SpeakerLayout, out_layout: SpeakerLayout) -> Box<dyn SoundReader<f32>> {
    match (in_layout, out_layout) {
        // Mono source
        (SpeakerLayout::Mono, SpeakerLayout::Mono) => in_stream,
        (SpeakerLayout::Mono, SpeakerLayout::Stereo)
            => MonoToStereo::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Mono, SpeakerLayout::Headphones)
            => MonoToHeadphones::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Mono, SpeakerLayout::Quadraphonic)
            => MonoToQuadraphonic::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Mono, SpeakerLayout::Surround51)
            => MonoToSurround51::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Mono, SpeakerLayout::Surround71)
            => MonoToSurround71::new_boxed(sample_rate, in_stream),
        // Stereo source
        (SpeakerLayout::Stereo, SpeakerLayout::Mono)
            => StereoToMono::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Stereo, SpeakerLayout::Stereo) => in_stream,
        (SpeakerLayout::Stereo, SpeakerLayout::Headphones) => in_stream,
        (SpeakerLayout::Stereo, SpeakerLayout::Quadraphonic)
            => StereoToQuadraphonic::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Stereo, SpeakerLayout::Surround51)
            => StereoToSurround51::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Stereo, SpeakerLayout::Surround71)
            => StereoToSurround71::new_boxed(sample_rate, in_stream),
        // Headphone source
        (SpeakerLayout::Headphones, SpeakerLayout::Mono)
            => HeadphonesToMono::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Headphones, SpeakerLayout::Stereo) => in_stream,
        (SpeakerLayout::Headphones, SpeakerLayout::Headphones) => in_stream,
        (SpeakerLayout::Headphones, SpeakerLayout::Quadraphonic)
            => HeadphonesToQuadraphonic::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Headphones, SpeakerLayout::Surround51)
            => HeadphonesToSurround51::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Headphones, SpeakerLayout::Surround71)
            => HeadphonesToSurround71::new_boxed(sample_rate, in_stream),
        // Quadraphonic source
        (SpeakerLayout::Quadraphonic, SpeakerLayout::Mono)
            => QuadraphonicToMono::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Quadraphonic, SpeakerLayout::Stereo)
            => QuadraphonicToStereo::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Quadraphonic, SpeakerLayout::Headphones)
            => QuadraphonicToHeadphones::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Quadraphonic, SpeakerLayout::Quadraphonic) => in_stream,
        (SpeakerLayout::Quadraphonic, SpeakerLayout::Surround51)
            => QuadraphonicToSurround51::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Quadraphonic, SpeakerLayout::Surround71)
            => QuadraphonicToSurround71::new_boxed(sample_rate, in_stream),
        // Surround 5.1 source
        (SpeakerLayout::Surround51, SpeakerLayout::Mono)
            => Surround51ToMono::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround51, SpeakerLayout::Stereo)
            => Surround51ToStereo::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround51, SpeakerLayout::Headphones)
            => Surround51ToHeadphones::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround51, SpeakerLayout::Quadraphonic)
            => Surround51ToQuadraphonic::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround51, SpeakerLayout::Surround51) => in_stream,
        (SpeakerLayout::Surround51, SpeakerLayout::Surround71)
            => Surround51ToSurround71::new_boxed(sample_rate, in_stream),
        // Surround 7.1 source
        (SpeakerLayout::Surround71, SpeakerLayout::Mono)
            => Surround71ToMono::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround71, SpeakerLayout::Stereo)
            => Surround71ToStereo::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround71, SpeakerLayout::Headphones)
            => Surround71ToHeadphones::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround71, SpeakerLayout::Quadraphonic)
            => Surround71ToQuadraphonic::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround71, SpeakerLayout::Surround51)
            => Surround71ToSurround51::new_boxed(sample_rate, in_stream),
        (SpeakerLayout::Surround71, SpeakerLayout::Surround71) => in_stream,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    struct FixedSource {
        src_data: Vec<f32>,
        pos: usize,
    }
    impl SoundReader<f32> for FixedSource {
        fn read(&mut self, mut buf: &mut [MaybeUninit<f32>]) -> usize {
            let src_range = &self.src_data[self.pos..];
            if buf.len() > src_range.len() {
                buf = &mut buf[..src_range.len()];
            }
            for n in 0 .. buf.len() {
                buf[n] = MaybeUninit::new(src_range[n]);
            }
            self.pos += buf.len();
            buf.len()
        }
    }
    #[test]
    fn uninit_ub() {
        let src_data: Vec<f32> = (0 .. 500).map(|x| (x as f32).sin()).collect();
        let src_reader = Box::new(FixedSource { src_data, pos: 0 });
        let mut adapted = new_channel_adapter(src_reader, PosFloat::new_clamped(456.0), SpeakerLayout::Mono, SpeakerLayout::Stereo);
        let mut bawk = [MaybeUninit::uninit(); 1000];
        assert_eq!(adapted.read(&mut bawk[..]), bawk.len());
        let bawk: [f32; 1000] = unsafe { std::mem::transmute(bawk) };
        for (n, e) in bawk.iter().enumerate() {
            assert_eq!(*e, ((n/2) as f32).sin());
        }
    }
}
