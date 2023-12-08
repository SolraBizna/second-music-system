use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    num::ParseFloatError,
    ops::{Add, Deref, Div, Mul},
    str::FromStr,
};

use super::SpeakerLayout;

#[derive(Clone, Copy, PartialEq)]
/// A finite, positive f32.
///
/// This is used in many places in SMS where a negative or infinite f32 would
/// break things: timestamps, sample rates, etc. Zero would break some of these
/// too, but less Interestingly.
pub struct PosFloat(f32);

impl PosFloat {
    pub const HALF: PosFloat = PosFloat(0.5);
    pub const ZERO: PosFloat = PosFloat(0.0);
    pub const ONE: PosFloat = PosFloat(1.0);
    pub const THOUSAND: PosFloat = PosFloat(1000.0);
    pub const MILLION: PosFloat = PosFloat(1000000.0);
    pub const BILLION: PosFloat = PosFloat(1000000.0);
    pub const SECONDS_PER_MINUTE: PosFloat = PosFloat(60.0);
    pub const SECONDS_PER_HOUR: PosFloat = PosFloat(3600.0);
    pub const SECONDS_PER_DAY: PosFloat = PosFloat(86400.0);
    /// Try to create a new PosFloat from an f32.
    pub fn new(x: f32) -> Result<PosFloat, &'static str> {
        if !x.is_finite() {
            Err("PosFloat must be finite")
        } else if !x.is_sign_positive() {
            Err("PosFloat must be positive")
        } else {
            Ok(PosFloat(x))
        }
    }
    /// Create a new PosFloat from an f32, which you *promise* is positive and
    /// finite.
    ///
    /// # Safety
    ///
    /// `x` must be positive (sign bit of zero), and finite (exponent < max).
    pub const unsafe fn new_unchecked(x: f32) -> PosFloat {
        /* not available as const fn… */
        /*
        debug_assert!(x.is_finite(), "it wasn't finite");
        debug_assert!(x.is_sign_positive(), "it wasn't positive");
        */
        PosFloat(x)
    }
    /// Create a new PosFloat from an f32. If it is non-finite or negative,
    /// return zero.
    pub fn new_clamped(x: f32) -> PosFloat {
        if x.is_finite() && x.is_sign_positive() {
            PosFloat(x)
        } else {
            PosFloat::ZERO
        }
    }
    /// Interprets this `PosFloat` as a time in seconds, and converts it to an
    /// integer number of sample frames at the given sample rate.
    pub fn seconds_to_frames(&self, sample_rate: PosFloat) -> u64 {
        (self.0 * sample_rate.0).floor() as u64
    }
    /// Interprets this `PosFloat` as a time in seconds, and converts it to an
    /// potentially-non-whole-number of sample frames at the given sample rate.
    pub fn seconds_to_frac_frames(&self, sample_rate: PosFloat) -> PosFloat {
        PosFloat((self.0 * sample_rate.0).floor())
    }
    /// Interprets this `PosFloat` as a time in seconds, and converts it to an
    /// integer number of samples for the given sample rate and speaker layout.
    pub fn seconds_to_samples(
        &self,
        sample_rate: PosFloat,
        speaker_layout: SpeakerLayout,
    ) -> u64 {
        self.seconds_to_frames(sample_rate)
            * speaker_layout.get_num_channels() as u64
    }
    /// Subtract `differend` from ourselves and return the result. If the
    /// result would have been zero (because `different` is greater than
    /// `self`), return zero.
    pub fn saturating_sub(&self, differend: PosFloat) -> PosFloat {
        let ret = self.0 - differend.0;
        if ret.is_sign_negative() || !ret.is_finite() {
            PosFloat::ZERO
        } else {
            PosFloat(ret)
        }
    }
}

impl Display for PosFloat {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&**self, f)
    }
}

impl Debug for PosFloat {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&**self, f)
    }
}

impl Eq for PosFloat {}

impl PartialOrd for PosFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Same-sign finite f32s have a total ordering.
impl Ord for PosFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bits().cmp(&other.0.to_bits())
    }
}

impl Hash for PosFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(self.0.is_finite() && self.0.is_sign_positive());
        self.0.to_bits().hash(state);
    }
}

impl Deref for PosFloat {
    type Target = f32;
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl From<u8> for PosFloat {
    fn from(value: u8) -> PosFloat {
        PosFloat(value as f32)
    }
}

impl From<u16> for PosFloat {
    fn from(value: u16) -> PosFloat {
        PosFloat(value as f32)
    }
}

impl From<u32> for PosFloat {
    fn from(value: u32) -> PosFloat {
        PosFloat(value as f32)
    }
}

impl From<u64> for PosFloat {
    fn from(value: u64) -> PosFloat {
        PosFloat(value as f32)
    }
}

impl From<usize> for PosFloat {
    fn from(value: usize) -> PosFloat {
        PosFloat(value as f32)
    }
}

pub enum TimePointFromStrError {
    ParseFloatError(ParseFloatError),
    NewTimePointError(&'static str),
}

impl From<ParseFloatError> for TimePointFromStrError {
    fn from(e: ParseFloatError) -> Self {
        Self::ParseFloatError(e)
    }
}

impl From<&'static str> for TimePointFromStrError {
    fn from(e: &'static str) -> Self {
        Self::NewTimePointError(e)
    }
}

impl FromStr for PosFloat {
    type Err = TimePointFromStrError;
    fn from_str(s: &str) -> Result<PosFloat, TimePointFromStrError> {
        Ok(PosFloat::new(s.parse::<f32>()?)?)
    }
}

impl Add<PosFloat> for PosFloat {
    type Output = PosFloat;
    fn add(self, rhs: PosFloat) -> PosFloat {
        PosFloat(self.0 + rhs.0)
    }
}

impl Div<PosFloat> for PosFloat {
    type Output = PosFloat;
    fn div(self, rhs: PosFloat) -> PosFloat {
        PosFloat(self.0 / rhs.0)
    }
}

impl Mul<PosFloat> for PosFloat {
    type Output = PosFloat;
    fn mul(self, rhs: PosFloat) -> PosFloat {
        PosFloat(self.0 * rhs.0)
    }
}
