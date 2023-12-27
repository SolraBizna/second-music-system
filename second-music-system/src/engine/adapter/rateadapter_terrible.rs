//! The rate adapter ingests a stream at one sample rate, and produces a stream
//! at another sample rate. This implementation uses nearest-neighbor
//! resampling. It sounds *terrible* but it requires no other dependencies.

use crate::*;

use std::mem::MaybeUninit;

use derivative::Derivative;

#[derive(Derivative)]
#[derivative(Debug)]
struct RateAdapter {
    #[derivative(Debug = "ignore")]
    inner: Box<dyn SoundReader<f32>>,
    #[derivative(Debug = "ignore")]
    in_buf: Vec<MaybeUninit<f32>>,
    in_buf_pos: usize,
    fini: bool,
    buffer_numerator: u8,
    buffer_denominator: u8,
    output_numerator: f32,
    output_denominator: f32,
    output_accumulator: f32,
    num_channels: u32,
}

pub(crate) fn new_rate_adapter(
    _delegate: &Arc<dyn SoundDelegate>,
    in_stream: Box<dyn SoundReader<f32>>,
    num_channels: u32,
    in_sample_rate: PosFloat,
    out_sample_rate: PosFloat,
) -> Box<dyn SoundReader<f32>> {
    let (buffer_numerator, buffer_denominator);
    if in_sample_rate < out_sample_rate {
        buffer_numerator =
            (*in_sample_rate * 32.0 / *out_sample_rate).ceil().max(1.0) as u8;
        buffer_denominator = 32;
    } else {
        buffer_numerator = 255;
        buffer_denominator =
            (*out_sample_rate * 255.0 / *in_sample_rate).ceil().max(1.0) as u8;
    }
    Box::new(RateAdapter {
        inner: in_stream,
        in_buf: vec![],
        in_buf_pos: 0,
        buffer_numerator,
        buffer_denominator,
        output_numerator: *out_sample_rate,
        output_denominator: *in_sample_rate,
        output_accumulator: 0.0,
        fini: false,
        num_channels,
    })
}

impl SoundReader<f32> for RateAdapter {
    fn read(&mut self, out: &mut [MaybeUninit<f32>]) -> usize {
        if self.in_buf_pos >= self.in_buf.len() && !self.fini {
            let in_len = (out.len() / self.num_channels as usize)
                * self.buffer_numerator as usize
                / self.buffer_denominator as usize
                * self.num_channels as usize;
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
        let mut out_produced = 0;
        'outer: while self.in_buf_pos < self.in_buf.len()
            && out_produced < out.len()
        {
            while self.output_accumulator >= self.output_denominator {
                self.in_buf_pos += self.num_channels as usize;
                self.output_accumulator -= self.output_denominator;
                if self.in_buf_pos >= self.in_buf.len() {
                    break 'outer;
                }
            }
            for n in 0..self.num_channels as usize {
                out[out_produced + n].write(unsafe {
                    self.in_buf[self.in_buf_pos + n].assume_init_read()
                });
            }
            out_produced += self.num_channels as usize;
            self.output_accumulator += self.output_numerator;
        }
        if self.in_buf_pos >= self.in_buf.len() {
            self.in_buf.truncate(0);
            self.in_buf_pos = 0;
        }
        if out_produced > 0 || self.fini {
            out_produced * self.num_channels as usize
        } else {
            self.read(out)
        }
    }
    fn seek(&mut self, _pos: u64) -> Option<u64> {
        panic!("SMS logic error: attempt to seek a rate adapter");
    }
    fn estimate_len(&mut self) -> Option<u64> {
        panic!(
            "SMS logic error: attempt to estimate length of a rate adapter"
        );
    }
}

// Sound because of note above about thread counts
unsafe impl Send for RateAdapter {}
