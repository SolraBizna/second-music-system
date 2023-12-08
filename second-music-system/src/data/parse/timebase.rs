
use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::{PosFloat, din::DinNode};

// I hate this name but we couldn't find a better one quickly
#[derive(Debug,PartialEq)]
struct TimebaseStage {
    one_based: bool,
    multiplier: PosFloat,
}

#[derive(Debug,PartialEq)]
pub struct Timebase {
    stages: Vec<TimebaseStage>,
}

static DEFAULT_TIMEBASE: Lazy<Timebase> = Lazy::new(|| {
    Timebase {
        stages: vec![
            TimebaseStage { one_based: false, multiplier: PosFloat::ONE },
        ],
    }
});

#[derive(Debug,Copy,Clone)]
pub enum TimebaseSuffix {
    Seconds, Milliseconds, Microseconds, Nanoseconds,
    Minutes, Hours, Days,
}

impl TimebaseSuffix {
    /// How many seconds per tick, given `x` ticks per `self`?
    fn num_per(&self, x: PosFloat) -> PosFloat {
        use TimebaseSuffix::*;
        match self {
            Seconds => PosFloat::ONE / x,
            Milliseconds => PosFloat::ONE / (x * PosFloat::THOUSAND),
            Microseconds => PosFloat::ONE / (x * PosFloat::MILLION),
            Nanoseconds => PosFloat::ONE / (x * PosFloat::BILLION),
            Minutes => PosFloat::SECONDS_PER_MINUTE / x,
            Hours => PosFloat::SECONDS_PER_HOUR / x,
            Days => PosFloat::SECONDS_PER_DAY / x,
        }
    }
    /// How many seconds per tick, given that each tick is `x` `self`s?
    fn num_times(&self, x: PosFloat) -> PosFloat {
        use TimebaseSuffix::*;
        match self {
            Seconds => x,
            Milliseconds => x / PosFloat::THOUSAND,
            Microseconds => x / PosFloat::MILLION,
            Nanoseconds => x / PosFloat::BILLION,
            Minutes => PosFloat::SECONDS_PER_MINUTE * x,
            Hours => PosFloat::SECONDS_PER_HOUR * x,
            Days => PosFloat::SECONDS_PER_DAY * x,
        }
    }
}

static TIMEBASE_SUFFIX_MAP: Lazy<HashMap<&'static str, TimebaseSuffix>>
= Lazy::new(|| {
    use TimebaseSuffix::*;
    [
        ("s", Seconds), ("sec", Seconds), ("second", Seconds),
        ("ms", Milliseconds), ("msec", Milliseconds),
        ("msecond", Milliseconds), ("millis", Milliseconds),
        ("millisec", Milliseconds), ("millisecond", Milliseconds),
        ("us", Microseconds), ("usec", Microseconds),
        ("usecond", Microseconds), ("µs", Microseconds),
        ("µsec", Microseconds), ("µsecond", Microseconds),
        ("micros", Microseconds), ("microsec", Microseconds),
        ("microsecond", Microseconds),
        ("ns", Nanoseconds), ("nsec", Nanoseconds),
        ("nsecond", Nanoseconds), ("nanos", Nanoseconds),
        ("nanosec", Nanoseconds), ("nanosecond", Nanoseconds),
        ("m", Minutes), ("min", Minutes), ("minute", Minutes),
        ("h", Hours), ("hr", Hours), ("hour", Hours),
        ("d", Days), ("day", Days),
    ].into_iter().collect()
});

pub enum TimeSpec {
    Basic,
    PerSuffix(TimebaseSuffix),
    TimesSuffix(TimebaseSuffix),
}

impl Timebase {
    pub fn parse_stage(mut source: &str) -> Result<(bool, PosFloat, TimeSpec), String> {
        let one_based = if source.starts_with('@') {
            source = &source[1..];
            true
        } else { false };
        let end = source.find(|x: char| !x.is_ascii_digit() && x != '.');
        let timespec = match end {
            Some(x) => {
                let suffix = &source[x..];
                source = &source[..x];
                let res = if let Some(x) = suffix.strip_prefix('/') {
                    if source.is_empty() {
                        return Err("Missing number".to_string())
                    }
                    TIMEBASE_SUFFIX_MAP.get(x)
                        .map(|x| TimeSpec::PerSuffix(*x))
                }
                else {
                    if source.is_empty() {
                        source = "1";
                    }
                    TIMEBASE_SUFFIX_MAP.get(&suffix)
                        .map(|x| TimeSpec::TimesSuffix(*x))
                };
                match res {
                    Some(x) => x,
                    None => return Err(format!("Unknown suffix: {:?}", suffix))
                }
            },
            None => TimeSpec::Basic,
        };
        let number = match source.parse::<PosFloat>() {
            Ok(x) => x,
            Err(_) => return Err("Invalid number".to_string()),
        };
        Ok((one_based, number, timespec))
    }
    pub fn parse_source(source: &[String]) -> Result<Timebase, String> {
        let mut stages: Vec<(bool, PosFloat)> = Vec::with_capacity(source.len());
        let mut basis = None;
        for (n, stage) in source.iter().enumerate() {
            match Timebase::parse_stage(stage) {
                Ok((one_based, number, timespec)) => {
                    match timespec {
                        TimeSpec::Basic => (),
                        x => {
                            if basis.is_none() {
                                basis = Some((n, x));
                            }
                            else {
                                return Err(format!("Resolution #{} contains a second basis. Only one basis is allowed.", n+1))
                            }
                        }
                    }
                    stages.push((one_based, number));
                },
                Err(x) => {
                    return Err(format!("Error parsing resolution #{}: {}", n+1, x))
                },
            }
        }
        let (basis_index, basis_spec) = match basis {
            Some(x) => x,
            None => {
                return Err("This timebase doesn't specify a basis (e.g. \"/minute\")".to_string())
            },
        };
        let mut ret: Vec<TimebaseStage> = Vec::with_capacity(stages.len());
        let mut iter = stages.into_iter().enumerate();
        for (n, (one_based, mut multiplier)) in &mut iter {
            if n == basis_index {
                match basis_spec {
                    TimeSpec::Basic => unreachable!(),
                    TimeSpec::PerSuffix(x) => multiplier = x.num_per(multiplier),
                    TimeSpec::TimesSuffix(x) => multiplier = x.num_times(multiplier),
                }
            }
            for x in ret.iter_mut() {
                x.multiplier = x.multiplier * multiplier;
            }
            ret.push(TimebaseStage { one_based, multiplier });
            if n == basis_index { break }
        }
        let mut multiplier = ret.last().unwrap().multiplier;
        for (_, (one_based, number)) in &mut iter {
            multiplier = multiplier / number;
            ret.push(TimebaseStage { one_based, multiplier });
        }
        Ok(Timebase { stages: ret })
    }
    fn eval(&self, mut specifier: &str, be_one_based: bool) -> Result<PosFloat, String> {
        // TODO: it's more numerically stable if we sum from smallest to
        // largest... but if your Segment is long enough for that to matter
        // you should rethink your life choices
        let mut ret = PosFloat::ZERO;
        for (i, stage) in self.stages.iter().enumerate() {
            let last = i+1 == self.stages.len();
            let raw = if last {
                // Parse the rest of the specifier as a f32
                match specifier.parse::<PosFloat>() {
                    Ok(x) => x,
                    Err(_) => {
                        return Err(format!("Invalid timecode"))
                    },
                }
            }
            else {
                // Parse up to the next `.` as an i32
                let period_pos = specifier.find('.').unwrap_or(specifier.len());
                let interesting = &specifier[..period_pos];
                specifier = &specifier[(period_pos+1).min(specifier.len())..];
                match interesting.parse::<i32>() {
                    Ok(x) => PosFloat::new(x as f32)?,
                    Err(_) => {
                        return Err(format!("Invalid timecode"))
                    },
                }
            };
            if be_one_based && stage.one_based {
                raw.saturating_sub(PosFloat::ONE);
            }
            ret = ret + raw * stage.multiplier;
        }
        Ok(ret)
    }
}

pub struct TimebaseCollection<'a> {
    parent: Option<&'a TimebaseCollection<'a>>,
    timebases: HashMap<String, Timebase>,
    active_timebase: Option<String>,
}

impl<'a> TimebaseCollection<'a> {
    pub fn new() -> TimebaseCollection<'static> {
        TimebaseCollection {
            parent: None,
            timebases: HashMap::new(),
            active_timebase: None,
        }
    }
    pub fn make_child(&self) -> TimebaseCollection<'_> {
        TimebaseCollection {
            parent: Some(self),
            timebases: HashMap::new(),
            active_timebase: self.active_timebase.clone(),
        }
    }
    pub fn get_timebase(&self, name: &str) -> Option<&Timebase> {
        self.timebases.get(name)
        .or_else(|| {
            if let Some(parent) = self.parent { parent.get_timebase(name) }
            else { None }
        })
    }
    pub fn get_active_timebase(&self) -> Option<&Timebase> {
        self.active_timebase.as_ref().and_then(|x| self.get_timebase(x))
    }
    pub fn parse_timebase_node(&mut self, node: &DinNode) -> Result<(), String> {
        match || -> Result<(), String> {
            assert_eq!(node.items[0], "timebase");
            if !node.children.is_empty() {
                return Err("timebase elements must have no children (check indentation)".to_string())
            }
            if node.items.len() < 2 {
                return Err("not enough items in timebase spec".to_string())
            }
            let (name, stages) = if node.items[1].starts_with(|x: char| x == '.' || x == '@' || x.is_ascii_digit()) {
                // Doesn't look like a timebase name. Parse the whole thing.
                ("default".to_owned(), &node.items[1..])
            }
            else {
                // Looks like the first thing is the timebase name.
                (node.items[1].clone(), &node.items[2..])
            };
            if stages.is_empty() {
                if self.get_timebase(&name).is_none() {
                    return Err(format!("can't set timebase {:?} as active because it doesn't exist", name))
                }
                self.active_timebase = Some(name);
            }
            else {
                let timebase = Timebase::parse_source(stages)?;
                self.timebases.insert(name.clone(), timebase);
                if self.active_timebase.is_none() {
                    self.active_timebase = Some(name);
                }
            }
            Ok(())
        }() {
            Err(x) => Err(format!("line {}: {}", node.lineno, x)),
            x => x,
        }
    }
    pub fn parse_time(&self, items: &[String]) -> Result<PosFloat, String> {
        let (timebase, time) = match items.len() {
            2 => {
                let timebase = match self.get_active_timebase() {
                    Some(x) => x,
                    None => &DEFAULT_TIMEBASE,
                };
                (timebase, &items[1])
            },
            3 => {
                let timebase = match self.get_timebase(&items[1]) {
                    Some(x) => x,
                    None => {
                        return Err(format!("no known timebase named {:?}", items[1]))
                    },
                };
                (timebase, &items[2])
            },
            _ => return Err("either specify a time in the default timebase, or the name of a timebase followed by a time in that timebase".to_string())
        };
        match timebase.eval(time, !(items[0].ends_with("length") || items[0].starts_with("fade") || items[0].starts_with("over"))) {
            Ok(x) => Ok(x),
            Err(x) => Err(x),
        }
    }
    pub fn parse_time_node(&self, node: &DinNode) -> Result<PosFloat, String> {
        if !node.children.is_empty() {
            return Err(format!("{:?} elements must have no children (check indentation)", node.items[0]))
        }
        match self.parse_time(&node.items) {
            Ok(x) => Ok(x),
            Err(x) => Err(format!("line {}: {}", node.lineno, x)),
        }
    }
}

#[test] fn timebase_parse() {
    assert_eq!(
        Timebase::parse_source(&[
            "@4".to_string(),
            "120/m".to_string(),
            "32".to_string(),
        ]),
        Ok(Timebase {
            stages: vec![
                TimebaseStage { one_based: true, multiplier: PosFloat::new_clamped(2.0) },
                TimebaseStage { one_based: false, multiplier: PosFloat::new_clamped(0.5) },
                TimebaseStage { one_based: false, multiplier: PosFloat::new_clamped(1.0 / 64.0) }
            ]
        })
    );
}
