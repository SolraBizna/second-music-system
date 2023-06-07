// modified version of `count!` from:
// https://stackoverflow.com/questions/34304593/counting-length-of-repetition-in-macro
macro_rules! count {
    () => (0usize);
    ( $x:ident ) => (1usize);
    ( $x:ident, $($rest:ident),+ ) => (1usize + count!($($rest),*));
}

macro_rules! make_downmixer {
    ($name:ident ($($in_channel:ident),+) -> {$($out_channel:ident = $out_expr:expr);+;}) => {

        struct $name {
            // TODO: buffer pools
            buf: Vec<MaybeUninit<f32>>,
            inner: Box<dyn SoundReader<f32>>,
        }

        impl $name {
            const NUM_IN_CHANNELS: usize = count!($($in_channel),*);
            const NUM_OUT_CHANNELS: usize = count!($($out_channel),*);
            pub(crate) fn new(_sample_rate: f32, inner: Box<dyn SoundReader<f32>>) -> Box<dyn SoundReader<f32>> {
                Box::new($name {
                    buf: vec![],
                    inner,
                })
            }
        }

        impl SoundReader<f32> for $name {
            fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
                let in_len = out.len() * $name::NUM_IN_CHANNELS / $name::NUM_OUT_CHANNELS;
                if self.buf.len() < in_len {
                    self.buf.resize(in_len, MaybeUninit::uninit());
                }
                debug_assert_eq!(out.len() % $name::NUM_OUT_CHANNELS, 0,
                    "output buffer not a multiple of output channel count");
                debug_assert_eq!(in_len % $name::NUM_IN_CHANNELS, 0,
                    "input buffer not a multiple of input channel count");
                let amount_read = self.inner.read(&mut self.buf[..in_len]);
                debug_assert_eq!(amount_read % $name::NUM_IN_CHANNELS, 0,
                    "input did not read an exact number of frames");
                let amount_out = amount_read * $name::NUM_OUT_CHANNELS / $name::NUM_IN_CHANNELS;
                for (o, i) in out[..amount_out].chunks_exact_mut($name::NUM_OUT_CHANNELS).zip(self.buf[..amount_read].chunks($name::NUM_IN_CHANNELS)) {
                    let n = 0;
                    $(#[allow(unused)]
                    let $in_channel = unsafe { *i[n].assume_init_ref() };
                    #[allow(unused)]
                    let n = n + 1;)+
                    let n = 0;
                    $(o[n] = MaybeUninit::new($out_expr); #[allow(unused)] let n = n + 1;)+
                }
                amount_out
            }
            fn seek(&mut self, _pos: u64) -> Option<u64> {
                panic!("SMS logic error: attempt to seek a downmixer");
            }
            fn estimate_len(&mut self) -> Option<u64> {
                panic!("SMS logic error: attempt to estimate length of a downmixer");
            }
            fn skip_coarse(&mut self, out_count: u64, buf: &mut [MaybeUninit<f32>]) -> u64 {
                let in_count = out_count * ($name::NUM_IN_CHANNELS as u64) / ($name::NUM_OUT_CHANNELS as u64);
                debug_assert_eq!(out_count % ($name::NUM_OUT_CHANNELS as u64), 0,
                    "output skip not a multiple of output channel count");
                debug_assert_eq!(in_count % ($name::NUM_IN_CHANNELS as u64), 0,
                    "input skip not a multiple of input channel count");
                let in_skipped = self.inner.skip_coarse(out_count, buf);
                let out_skipped = in_skipped * ($name::NUM_OUT_CHANNELS as u64) / ($name::NUM_IN_CHANNELS as u64);
                out_skipped
            }
        }

    }
}

macro_rules! make_upmixer {
    ($name:ident ($($in_channel:ident),+) -> {$($out_channel:ident = $out_expr:expr);+;}) => {

        struct $name {
            inner: Box<dyn SoundReader<f32>>,
        }

        impl $name {
            const NUM_IN_CHANNELS: usize = count!($($in_channel),*);
            const NUM_OUT_CHANNELS: usize = count!($($out_channel),*);
            pub(crate) fn new(_sample_rate: f32, inner: Box<dyn SoundReader<f32>>) -> Box<dyn SoundReader<f32>> {
                Box::new($name {
                    inner,
                })
            }
        }

        impl SoundReader<f32> for $name {
            fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
                let out_len = out.len();
                let in_len = out_len * $name::NUM_IN_CHANNELS / $name::NUM_OUT_CHANNELS;
                debug_assert_eq!(out.len() % $name::NUM_OUT_CHANNELS, 0,
                    "output buffer not a multiple of output channel count");
                debug_assert_eq!(in_len % $name::NUM_IN_CHANNELS, 0,
                    "input buffer not a multiple of input channel count");
                let amount_read = self.inner.read(&mut out[out_len-in_len..]);
                debug_assert_eq!(amount_read % $name::NUM_IN_CHANNELS, 0,
                    "input did not read an exact number of frames");
                let amount_out = amount_read * $name::NUM_OUT_CHANNELS / $name::NUM_IN_CHANNELS;
                let mut in_index = out_len - in_len;
                let mut out_index = 0;
                while out_index < amount_out {
                    let n = 0;
                    $(
                        #[allow(unused)]
                        let $in_channel = unsafe {
                            *out[in_index+n].assume_init_ref()
                        };
                        #[allow(unused)]
                        let n = n + 1;
                    )+
                    let n = 0;
                    $(
                        out[out_index+n] = MaybeUninit::new($out_expr);
                        #[allow(unused)]
                        let n = n + 1;
                    )+
                    in_index += $name::NUM_IN_CHANNELS;
                    out_index += $name::NUM_OUT_CHANNELS;
                }
                amount_out
            }
            fn seek(&mut self, _pos: u64) -> Option<u64> {
                panic!("SMS logic error: attempt to seek an upmixer");
            }
            fn estimate_len(&mut self) -> Option<u64> {
                panic!("SMS logic error: attempt to estimate length of an upmixer");
            }
            fn skip_coarse(&mut self, out_count: u64, buf: &mut [MaybeUninit<f32>]) -> u64 {
                let in_count = out_count * ($name::NUM_IN_CHANNELS as u64) / ($name::NUM_OUT_CHANNELS as u64);
                debug_assert_eq!(out_count % ($name::NUM_OUT_CHANNELS as u64), 0,
                    "output skip not a multiple of output channel count");
                debug_assert_eq!(in_count % ($name::NUM_IN_CHANNELS as u64), 0,
                    "input skip not a multiple of input channel count");
                let in_skipped = self.inner.skip_coarse(out_count, buf);
                let out_skipped = in_skipped * ($name::NUM_OUT_CHANNELS as u64) / ($name::NUM_IN_CHANNELS as u64);
                out_skipped
            }
        }

    }
}


macro_rules! make_upmixer_with_lowpass {
    ($name:ident ($($in_channel:ident),+) -> {$lowpass_var:ident = $lowpass_expr:expr; $($out_channel:ident = $out_expr:expr);+;}) => {

        struct $name {
            inner: Box<dyn SoundReader<f32>>,
            lowpass_alpha: f32,
            lowpass_hold: f32,
        }

        impl $name {
            const NUM_IN_CHANNELS: usize = count!($($in_channel),*);
            const NUM_OUT_CHANNELS: usize = count!($($out_channel),*);
            pub(crate) fn new(_sample_rate: f32, inner: Box<dyn SoundReader<f32>>) -> Box<dyn SoundReader<f32>> {
                Box::new($name {
                    inner,
                    lowpass_alpha: todo!(),
                    lowpass_hold: 0.0,
                })
            }
        }

        impl SoundReader<f32> for $name {
            fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
                let out_len = out.len();
                let in_len = out_len * $name::NUM_IN_CHANNELS / $name::NUM_OUT_CHANNELS;
                debug_assert_eq!(out.len() % $name::NUM_OUT_CHANNELS, 0,
                    "output buffer not a multiple of output channel count");
                debug_assert_eq!(in_len % $name::NUM_IN_CHANNELS, 0,
                    "input buffer not a multiple of input channel count");
                let amount_read = self.inner.read(&mut out[out_len-in_len..]);
                debug_assert_eq!(amount_read % $name::NUM_IN_CHANNELS, 0,
                    "input did not read an exact number of frames");
                let amount_out = amount_read * $name::NUM_OUT_CHANNELS / $name::NUM_IN_CHANNELS;
                let mut in_index = out_len - in_len;
                let mut out_index = 0;
                while out_index < amount_out {
                    let n = 0;
                    $(
                        #[allow(unused)]
                        let $in_channel = unsafe {
                            *out[in_index+n].assume_init_ref()
                        };
                        #[allow(unused)]
                        let n = n + 1;
                    )+
                    let $lowpass_var = $lowpass_expr;
                    self.lowpass_hold = self.lowpass_hold + (self.lowpass_hold + $lowpass_var) * self.lowpass_alpha;
                    let $lowpass_var = self.lowpass_hold;
                    let n = 0;
                    $(out[out_index+n] = MaybeUninit::new($out_expr); #[allow(unused)] let n = n + 1;)+
                    in_index += $name::NUM_IN_CHANNELS;
                    out_index += $name::NUM_OUT_CHANNELS;
                }
                amount_out
            }
            fn seek(&mut self, _pos: u64) -> Option<u64> {
                panic!("SMS logic error: attempt to seek an upmixer");
            }
            fn estimate_len(&mut self) -> Option<u64> {
                panic!("SMS logic error: attempt to estimate length of an upmixer");
            }
            fn skip_coarse(&mut self, out_count: u64, buf: &mut [MaybeUninit<f32>]) -> u64 {
                let in_count = out_count * ($name::NUM_IN_CHANNELS as u64) / ($name::NUM_OUT_CHANNELS as u64);
                debug_assert_eq!(out_count % ($name::NUM_OUT_CHANNELS as u64), 0,
                    "output skip not a multiple of output channel count");
                debug_assert_eq!(in_count % ($name::NUM_IN_CHANNELS as u64), 0,
                    "input skip not a multiple of input channel count");
                let in_skipped = self.inner.skip_coarse(out_count, buf);
                let out_skipped = in_skipped * ($name::NUM_OUT_CHANNELS as u64) / ($name::NUM_IN_CHANNELS as u64);
                out_skipped
            }
        }

    }
}
