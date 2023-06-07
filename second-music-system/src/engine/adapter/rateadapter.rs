//! The rate adapter ingests a stream at one sample rate, and produces a stream
//! at another sample rate.

use crate::*;

use std::{
    mem::MaybeUninit,
};

use libsoxr::*;

use derivative::Derivative;

#[derive(Derivative)]
#[derivative(Debug)]
struct RateAdapter {
    #[derivative(Debug="ignore")]
    inner: Box<dyn SoundReader<f32>>,
    #[derivative(Debug="ignore")]
    in_buf: Vec<MaybeUninit<f32>>,
    in_buf_pos: usize,
    fini: bool,
    soxr: Soxr,
    buffer_numerator: u8,
    buffer_denominator: u8,
    num_channels: u32,
}

pub(crate) fn new_rate_adapter(delegate: &Arc<dyn SoundDelegate>, in_stream: Box<dyn SoundReader<f32>>, num_channels: u32, in_sample_rate: f32, out_sample_rate: f32) -> Box<dyn SoundReader<f32>> {
    let io_spec = IOSpec::new(Datatype::Float32I, Datatype::Float32I);
    let quality_spec = QualitySpec::new(&QualityRecipe::Medium, QualityFlags::HI_PREC_CLOCK);
    let runtime_spec = RuntimeSpec::new(1); // no multithreading
    // NOTE! NOTE NOTE NOTE!
    // We send this Soxr instance between threads. This is *probably* safe to
    // do as long as we're only using one thread for resampling (as selected
    // above). EVERYTHING WILL BREAK IF YOU CHANGE THAT!
    let soxr = match Soxr::create(in_sample_rate as f64, out_sample_rate as f64, num_channels, Some(&io_spec), Some(&quality_spec), Some(&runtime_spec)) {
        Ok(x) => x,
        Err(x) => {
            delegate.warning(&format!("Unable to initialize resampler for {} -> {} Hz: {}", in_sample_rate, out_sample_rate, x));
            return in_stream
        }
    };
    let (buffer_numerator, buffer_denominator);
    if in_sample_rate < out_sample_rate {
        buffer_numerator = (in_sample_rate * 32.0 / out_sample_rate).ceil().max(1.0) as u8;
        buffer_denominator = 32;
    }
    else {
        buffer_numerator = 255;
        buffer_denominator = (out_sample_rate * 255.0 / in_sample_rate).ceil().max(1.0) as u8;
    }
    Box::new(RateAdapter {
        inner: in_stream,
        in_buf: vec![], in_buf_pos: 0,
        buffer_numerator, buffer_denominator,
        soxr, fini: false, num_channels,
    })
}

impl SoundReader<f32> for RateAdapter {
    fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
        if self.in_buf_pos >= self.in_buf.len() && !self.fini {
            let in_len = (out.len() / self.num_channels as usize) * self.buffer_numerator as usize / self.buffer_denominator as usize * self.num_channels as usize;
            // will reallocate if growing, will not reallocate if shrinking
            self.in_buf.resize(in_len, MaybeUninit::uninit());
            debug_assert_eq!(out.len() % self.num_channels as usize, 0);
            debug_assert_eq!(in_len % self.num_channels as usize, 0);
            let amount_read = self.inner.read(&mut self.in_buf[..in_len]);
            debug_assert_eq!(amount_read % self.num_channels as usize, 0);
            self.in_buf.truncate(amount_read);
            self.in_buf_pos = 0;
            self.fini = amount_read == 0;
        }
        let in_slice = &self.in_buf[self.in_buf_pos..];
        // EVIL! and not currently sound?!
        // TODO: maybe_uninit_slice
        let in_slice: Option<&[f32]> = if self.fini && in_slice.is_empty() { None }
        else { unsafe { std::mem::transmute(in_slice) } };
        // this is unsound even with maybe_uninit_slice, but it will always be
        // okay as long as the soxr crate is a wrapper around the native
        // libsoxr library
        let out_slice: &mut [f32] = unsafe { std::mem::transmute(out) };
        let (in_consumed, out_produced) = self.soxr.process(in_slice, out_slice).expect("internal libsoxr error");
        self.in_buf_pos += in_consumed as usize * self.num_channels as usize;
        if self.in_buf_pos >= self.in_buf.len() {
            self.in_buf.truncate(0);
            self.in_buf_pos = 0;
        }
        if out_produced > 0 || self.fini {
            return out_produced as usize * self.num_channels as usize
        }
        else {
            return self.read(unsafe { std::mem::transmute(out_slice) })
        }
    }
    fn seek(&mut self, _pos: u64) -> Option<u64> {
        panic!("SMS logic error: attempt to seek a rate adapter");
    }
    fn estimate_len(&mut self) -> Option<u64> {
        panic!("SMS logic error: attempt to estimate length of a rate adapter");
    }
}

// Sound because of note above about thread counts
unsafe impl Send for RateAdapter {}
