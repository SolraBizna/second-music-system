use crate::PosFloat;

/// Specifies what kind of curve to use in a fade.
///
/// Logarithmic fades will have (roughly) the same perceived volume change per
/// unit time. Linear fades will seem to speed up or slow down over the course
/// of the fade, and should be used when "intermixing" related tracks.
/// Exponential fades will have the variable-speed "problem" even worse, but
/// may sound the best of the three.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FadeType {
    /// Fades between the given volumes on a logarithmic curve, such that any
    /// given timespan within the fade will have the same perceived volume
    /// change as any other.
    Logarithmic,
    /// Fades linearly between the given amplification factors. You only want
    /// this when you're crossfading between partly correlated samples.
    Linear,
    /// Fades between the given volumes on an exponential curve, resulting in
    /// a fade that "hangs out" at the louder side. Arguably more
    /// aesthetically pleasing than a logarithmic fade.
    #[default]
    Exponential,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FadeCurve {
    Logarithmic { pos: f32, step: f32 },
    Exponential { pos: f32, step: f32 },
    Linear { pos: f32, step: f32 },
}

/// Natural logarithm of the quietest amplitude we consider audible.
/// This value is equivalent to a volume level of about -96.3dB, and also the
/// ratio of the smallest non-zero voltage to the largest non-zero voltage that
/// a 16-bit DAC will output.
const SILENT_LOG: f32 = -11.1;

/// Natural exponent of the quietest amplitude we consider audible.
/// This value is equivalent to a volume level of about -96.3dB, and also the
/// ratio of the smallest non-zero voltage to the largest non-zero voltage that
/// a 16-bit DAC will output.
const SILENT_EXP: f32 = 1.0000152;

impl FadeCurve {
    pub fn from(
        typ: FadeType,
        from: PosFloat,
        to: PosFloat,
        length: PosFloat,
    ) -> FadeCurve {
        match typ {
            FadeType::Exponential => {
                let from = from.exp().max(SILENT_EXP);
                let to = to.exp().max(SILENT_EXP);
                let step = (to - from) / (*length + 1.0);
                FadeCurve::Exponential { pos: from, step }
            }
            FadeType::Logarithmic => {
                let from = from.ln().max(SILENT_LOG);
                let to = to.ln().max(SILENT_LOG);
                let step = (to - from) / (*length + 1.0);
                FadeCurve::Logarithmic { pos: from, step }
            }
            FadeType::Linear => {
                let step = (*to - *from) / (*length + 1.0);
                FadeCurve::Linear { pos: *from, step }
            }
        }
    }
    /// Evaluate the current state of the fader.
    fn evaluate(&self) -> PosFloat {
        PosFloat::new_clamped(match self {
            Self::Exponential { pos, .. } => pos.ln(),
            Self::Logarithmic { pos, .. } => pos.exp(),
            Self::Linear { pos, .. } => *pos,
        })
    }
    /// Evaluate the state of the fader t steps into the future.
    fn evaluate_t(&self, t: PosFloat) -> PosFloat {
        PosFloat::new_clamped(match self {
            Self::Exponential { pos, step } => (pos + step * *t).ln(),
            Self::Logarithmic { pos, step } => (pos + step * *t).exp(),
            Self::Linear { pos, step } => *pos + step * *t,
        })
    }
    /// Step by a single sample frame
    fn step_by_one(&mut self) {
        match self {
            Self::Logarithmic { pos, step }
            | Self::Exponential { pos, step }
            | Self::Linear { pos, step } => *pos += *step,
        }
    }
    /// Step by a given number of sample frames
    fn step_by(&mut self, count: PosFloat) {
        match self {
            Self::Logarithmic { pos, step }
            | Self::Exponential { pos, step }
            | Self::Linear { pos, step } => *pos += *step * *count,
        }
    }
}

/// Represents a fade, in or out, currently in progress.
#[derive(Debug, Clone)]
pub struct Fader {
    curve: FadeCurve,
    to: PosFloat,
    length: PosFloat, // given in SAMPLE FRAMES
    pos: PosFloat,    // given in SAMPLE FRAMES
}

impl Fader {
    /// Blank fader. Just has volume at the given level.
    pub fn new(volume: PosFloat) -> Fader {
        Fader {
            curve: FadeCurve::Linear {
                pos: *volume,
                step: 0.0,
            },
            to: volume,
            length: PosFloat::ZERO,
            pos: PosFloat::ONE,
        }
    }
    /// Start a fade.
    /// - `type`: A [`FadeType`](enum.FadeType.html) denoting which curve to
    ///   use.
    /// - `from`: The starting volume of the fade.
    /// - `to`: The ending volume of the fade.
    /// - `length`: How long, in **sample frames**, the fade should take to
    ///   complete.
    pub fn start(
        typ: FadeType,
        from: PosFloat,
        to: PosFloat,
        length: PosFloat,
    ) -> Fader {
        Fader {
            curve: FadeCurve::from(typ, from, to, length),
            to,
            length,
            pos: PosFloat::ZERO,
        }
    }
    /// As `start`, but returns `None` if the given length is infinite, zero,
    /// or negative.
    pub fn maybe_start(
        typ: FadeType,
        from: PosFloat,
        to: PosFloat,
        length: PosFloat,
    ) -> Option<Fader> {
        if length > PosFloat::ZERO {
            Some(Fader::start(typ, from, to, length))
        } else {
            None
        }
    }
    /// Returns true if the fade has run its course.
    pub fn complete(&self) -> bool {
        self.pos >= self.length
    }
    /// Evaluate the current volume.
    pub fn evaluate(&self) -> PosFloat {
        if self.complete() {
            self.to
        } else {
            self.curve.evaluate()
        }
    }
    /// Evaluate the volume `t` steps into the future.
    pub fn evaluate_t(&self, t: PosFloat) -> PosFloat {
        let new_pos = self.pos + t;
        if new_pos >= self.length {
            self.to
        } else {
            self.curve.evaluate_t(t)
        }
    }
    /// Step by a single sample frame
    pub fn step_by_one(&mut self) {
        if !self.complete() {
            self.curve.step_by_one();
            self.pos = self.pos + PosFloat::ONE;
        }
    }
    /// Step by a give number of sample frames
    pub fn step_by(&mut self, count: PosFloat) {
        if !self.complete() {
            self.curve.step_by(count);
            self.pos = self.pos + count;
        }
    }
}

impl Iterator for Fader {
    type Item = PosFloat;
    fn next(&mut self) -> Option<PosFloat> {
        if self.complete() {
            None
        } else {
            let ret = self.evaluate();
            self.step_by_one();
            Some(ret)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = (self.length.ceil() - *self.pos) as usize;
        (size, Some(size))
    }
    fn count(self) -> usize {
        (self.length.ceil() - *self.pos) as usize
    }
}
