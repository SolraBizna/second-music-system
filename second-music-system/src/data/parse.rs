use std::collections::HashMap;
use once_cell::sync::Lazy;

use super::*;

use din::*;

mod expression;
use expression::{parse_condition, parse_expression};

#[cfg(test)]
mod test;

// I hate this name but we couldn't find a better one quickly
#[derive(Debug,PartialEq)]
struct TimebaseStage {
    one_based: bool,
    multiplier: PosFloat,
}

#[derive(Debug,PartialEq)]
struct Timebase {
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
enum TimebaseSuffix {
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

enum TimeSpec {
    Basic,
    PerSuffix(TimebaseSuffix),
    TimesSuffix(TimebaseSuffix),
}

impl Timebase {
    fn parse_stage(mut source: &str) -> Result<(bool, PosFloat, TimeSpec), String> {
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
    fn parse_source(source: &[String]) -> Result<Timebase, String> {
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

struct TimebaseCollection<'a> {
    parent: Option<&'a TimebaseCollection<'a>>,
    timebases: HashMap<String, Timebase>,
    active_timebase: Option<String>,
}

impl<'a> TimebaseCollection<'a> {
    fn new() -> TimebaseCollection<'static> {
        TimebaseCollection {
            parent: None,
            timebases: HashMap::new(),
            active_timebase: None,
        }
    }
    fn make_child(&self) -> TimebaseCollection<'_> {
        TimebaseCollection {
            parent: Some(self),
            timebases: HashMap::new(),
            active_timebase: self.active_timebase.clone(),
        }
    }
    fn get_timebase(&self, name: &str) -> Option<&Timebase> {
        self.timebases.get(name)
        .or_else(|| {
            if let Some(parent) = self.parent { parent.get_timebase(name) }
            else { None }
        })
    }
    fn get_active_timebase(&self) -> Option<&Timebase> {
        self.active_timebase.as_ref().and_then(|x| self.get_timebase(x))
    }
    fn parse_timebase_node(&mut self, node: &DinNode) -> Result<(), String> {
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
    fn parse_time(&self, items: &[String]) -> Result<PosFloat, String> {
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
    fn parse_time_node(&self, node: &DinNode) -> Result<PosFloat, String> {
        if !node.children.is_empty() {
            return Err(format!("{} elements must have no children (check indentation)", node.items[0]))
        }
        match self.parse_time(&node.items) {
            Ok(x) => Ok(x),
            Err(x) => Err(format!("line {}: {}", node.lineno, x)),
        }
    }
}

const SOUND_TIME_KEYWORDS: &[&str] = &[
    "start", "end", "length",
];

impl Sound {
    fn parse_din_node(node: &DinNode, timebases: &TimebaseCollection) -> Result<Sound, String> {
        assert_eq!(node.items[0], "sound");
        if node.items.len() != 2 {
            return Err(format!("line {}: sound element must have a name", node.lineno))
        }
        let mut timebases = timebases.make_child();
        let name = node.items[1].to_compact_string();
        let mut path = None;
        let mut stream = None;
        let mut data = HashMap::new();
        let mut offset = None;
        for child in node.children.iter() {
            if !child.children.is_empty() {
                return Err(format!("line {}: this element must have no children", child.lineno))
            }
            debug_assert!(!child.items.is_empty());
            if child.items[0] == "stream" {
                if stream.is_some() {
                    return Err(format!("line {}: only one {:?} element allowed", child.lineno, child.items[0]))
                }
                else if child.items.len() > 1 {
                    return Err(format!("line {}: \"stream\" must not have any items", child.lineno))
                }
                else {
                    stream = Some(true);
                }
            }
            else if child.items[0] == "file" {
                if path.is_some() {
                    return Err(format!("line {}: only one {:?} element allowed", child.lineno, child.items[0]))
                }
                else if child.items.len() > 2 {
                    return Err(format!("line {}: this element should have a single item (try adding quotes)", child.lineno))
                }
                else if let Some(index) = child.items[1].find(['\0']) {
                    return Err(format!("line {}: this element's path contains a null character at position {}", child.lineno, index))
                }
                else {
                    path = Some(child.items[1].to_compact_string());
                }
            }
            else if child.items[0] == "timebase" {
                timebases.parse_timebase_node(child)?;
            }
            else if SOUND_TIME_KEYWORDS.contains(&child.items[0].as_str()) {
                if data.contains_key(child.items[0].as_str()) {
                    return Err(format!("line {}: only one {:?} element allowed", child.lineno, child.items[0]))
                }
                else {
                    let time = timebases.parse_time_node(child)?;
                    data.insert(child.items[0].as_str(), time);
                }
            }
            else if child.items[0] == "offset" {
                if offset.is_some() {
                    return Err(format!("line {}: only one {:?} element allowed", child.lineno, child.items[0]))
                }
                else if child.items.len() != 2 {
                    return Err(format!("line {}: this element should have a single item", child.lineno))
                }
                else if let Ok(value) = child.items[1].parse() {
                    offset = Some(PosFloat::new(value)?);
                }
                else {
                    return Err(format!("line {}: that doesn't appear to be a valid number", child.lineno))
                }
            }
            else {
                return Err(format!("line {}: unknown sound element {:?}", child.lineno, child.items[0]))
            }
        }
        let offset = offset.unwrap_or(PosFloat::ZERO);
        let start = match data.get("start") {
            Some(x) => *x + offset,
            None => PosFloat::ZERO,
        };
        let end = match (data.get("end"), data.get("length")) {
            (Some(_), Some(_)) => {
                return Err(format!("line {}: only one of \"end\" and \"
                length\" may be specified, not both", node.lineno))
            },
            (Some(x), None) => *x + offset,
            (None, Some(x)) => start + *x,
            (None, None) => {
                return Err(format!("line {}: one of \"end\" or \"length\" must be specified", node.lineno))
            },
        };
        // TODO: fade out requires length
        let path = match path {
            Some(path) => path,
            None => {
                if let Some(index) = name.find(['\0']) {
                    return Err(format!("Sound {name:?} has a null character in its name at position {index} and no explicit path. If there is no explicit path, the name is used as the path, and the path is not allowed to have null characters in it. Either remove the null character from the name or add an explicit path."));
                }
                name.to_compact_string()
            }
        };
        let stream = stream.unwrap_or(false);
        Ok(Sound {
            name, path, start, end, stream,
        })
    }
}

impl Sequence {
    fn parse_din_node(node: &DinNode, timebases: &TimebaseCollection) -> Result<Sequence, String> {
        assert_eq!(node.items[0], "sequence");
        if node.items.len() != 2 {
            return Err(format!("line {}: sequence element must have a name", node.lineno))
        }
        let mut timebases = timebases.make_child();
        let name = node.items[1].to_compact_string();
        let mut length = None;
        let mut elements = Vec::new();
        for child in node.children.iter() {
            debug_assert!(!child.items.is_empty());
            if child.items[0] == "length" {
                if !child.children.is_empty() {
                    return Err(format!("line {}: this element must have no children", child.lineno))
                }
                else if length.is_some() {
                    return Err(format!("line {}: only one {:?} element allowed", child.lineno, child.items[0]))
                }
                else {
                    length = Some(timebases.parse_time_node(child)?);
                }
            }
            else if child.items[0] == "play" {
                let (start, element) = SequenceElement::parse_din_node(child, &timebases)?;
                elements.push((start, element));
            }
            else if child.items[0] == "timebase" {
                timebases.parse_timebase_node(child)?;
            }
            else {
                return Err(format!("line {}: unknown sequence element {:?}", child.lineno, child.items[0]))
            }
        }
        let length = match length {
            Some(x) => x,
            None => return Err(format!("line {}: \"length\" must be specified", node.lineno))
        };
        elements.sort_by(|a,b| a.0.cmp(&b.0));
        Ok(Sequence {
            name, length, elements,
        })
    }
}

const SOUND_ELEMENT_TIME_KEYWORDS: &[&str] = &[
    "at", "for", "until", "fade_in", "fade_out",
];

const SEQUENCE_ELEMENT_TIME_KEYWORDS: &[&str] = &[
    "at"
];

impl SequenceElement {
    fn parse_din_node(node: &DinNode, timebases: &TimebaseCollection) -> Result<(PosFloat, SequenceElement), String> {
        assert_eq!(node.items[0], "play");
        // TODO: Better (combinatorial?) error messages.
        if node.items.len() == 1 {
            return Err(format!("line {}: \"play\" must specify an element type of either \"sound\" or \"sequence\" and the name of an element of the specified type.", node.lineno))
        }
        let element_type = match node.items[1].as_str() {
            e @ "sound" | e @ "sequence" => e,
            x => return Err(format!("line {}: invalid element type \"{x}\". Element type must be either \"sound\" or \"sequence\".", node.lineno))
        };
        if node.items.len() < 3 {
            return Err(format!("line {}: \"play\" must specify the name of the element to be played", node.lineno))
        }
        let name = &node.items[2];
        if node.items.len() > 3 {
            return Err(format!("line {}: \"play\" must only include the element type and the name of the element on its own line.", node.lineno))
        }
        let mut timebases = timebases.make_child();
        // TODO: remove explicit type
        let mut data = HashMap::new();
        let mut channel = None;
        for child in node.children.iter() {
            if !child.children.is_empty() {
                return Err(format!("line {}: this element must have no children", child.lineno))
            }
            debug_assert!(!child.items.is_empty());
            if child.items[0] == "channel" {
                if element_type == "sequence" {
                    return Err(format!("line {}: \"channel\" is not allowed in a sequence element", child.lineno))
                }
                if channel.is_some() {
                    return Err(format!("line {}: only one {:?} parameter allowed", child.lineno, child.items[0]))
                }
                else if child.items.len() == 1 {
                    return Err(format!("line {}: \"channel\" must specify the name of a channel that will control this region", child.lineno))
                }
                else if child.items.len() > 2 {
                    return Err(format!("line {}: \"channel\" must have only one item (do you need quotes?)", child.lineno))
                }
                else {
                    channel = Some(child.items[1].clone());
                }
            }
            else if child.items[0] == "timebase" {
                timebases.parse_timebase_node(child)?;
            }
            else if match element_type {
                "sequence" => SEQUENCE_ELEMENT_TIME_KEYWORDS.contains(&child.items[0].as_str()),
                "sound" => SOUND_ELEMENT_TIME_KEYWORDS.contains(&child.items[0].as_str()),
                _ => unreachable!()
            } {
                if data.contains_key(child.items[0].as_str()) {
                    return Err(format!("line {}: only one {:?} parameter allowed", child.lineno, child.items[0]))
                }
                else {
                    let time = timebases.parse_time_node(child)?;
                    data.insert(child.items[0].as_str(), time);
                }
            }
            else {
                return Err(format!("line {}: unknown element parameter {:?}", child.lineno, child.items[0]))
            }
        }
        let channel = match channel {
            Some(x) => x.to_compact_string(),
            None => "main".to_compact_string(),
        };
        let start = match data.get("at") {
            Some(x) => *x,
            None => PosFloat::ZERO,
        };
        let fade_in = match data.get("fade_in") {
            Some(x) => *x,
            None => PosFloat::ZERO,
        };
        let length = match (data.get("for"), data.get("until")) {
            (Some(_), Some(_)) => {
                return Err(format!("line {}: only one of \"for\" and \"until\" may be specified, not both", node.lineno))
            },
            (None, None) => {
                None
            },
            (Some(length), None) => {
                Some(*length)
            },
            (None, Some(end)) => {
                Some(end.saturating_sub(start))
            },
        };
        let (length, fade_out) = match data.get("fade_out") {
            Some(fade_out) => (length.map(|x| x.saturating_sub(*fade_out)), *fade_out),
            None => (length, PosFloat::ZERO),
        };
        match element_type {
            "sound" => Ok((start, SequenceElement::PlaySound { sound: name.to_compact_string(), channel, fade_in, length, fade_out })),
            "sequence" => Ok((start, SequenceElement::PlaySequence { sequence: name.to_compact_string() })),
            _ => unreachable!()
        }
    }
}

fn parse_flow_command_tokens(tokens: &[String], timebases: &TimebaseCollection) -> Result<Option<Command>, String> {
    if tokens.is_empty() { return Ok(None) }
    match tokens[0].as_str() {
        "done" => {
            if tokens.len() != 1  {
                return Err("nothing is allowed after \"done\"".to_string())
            }
            Ok(Some(Command::Done))
        },
        "wait" => {
            let how_long = timebases.parse_time(tokens)?;
            Ok(Some(Command::Wait(how_long)))
        },
        "play" => {
            let token = tokens.get(1).map(String::as_str);
            match token {
                Some("sequence") | Some("sound") => (),
                _ => return Err("next element after \"play\" must be \"sequence\" or \"sound\"".to_string())
            }
            let target = match tokens.get(2) {
                Some(x) => x,
                None => return Err(format!("next element after \"{}\" must be the name of the {} to play", token.unwrap(), token.unwrap())),
            }.to_compact_string();
            let and_wait = if tokens.len() == 3 {
                false
            }
            else if tokens.len() == 5 && tokens[3] == "and" && tokens[4] == "wait" {
                true
            }
            else {
                return Err("the only thing allowed after the name of the sequence or sound to play is the elements \"and wait\" (do you need quotation marks?)".to_string())
            };
            Ok(Some(match (token.unwrap(), and_wait) {
                ("sequence", false) => Command::PlaySequence(target),
                ("sequence", true) => Command::PlaySequenceAndWait(target),
                ("sound", false) => Command::PlaySound(target),
                ("sound", true) => Command::PlaySoundAndWait(target),
                _ => unreachable!(),
            }))
        },
        "start" | "restart" | "stop" => {
            match tokens.get(1).map(String::as_str) {
                Some("node") => {
                    let target = match tokens.get(2) {
                        Some(x) => x,
                        None => return Err(format!("next element after \"node\" must be the name of the node to {}", tokens[0])),
                    }.to_compact_string();
                    if tokens.len() != 3 {
                        return Err("nothing is allowed after the node name (do you need quotation marks?)".to_string())
                    };
                    match tokens[0].as_str() {
                        "start" => Ok(Some(Command::StartNode(target))),
                        "restart" => Ok(Some(Command::RestartNode(target))),
                        "stop" => {
                            Err("stop is not allowed because it will sound bad (if you really want an abrupt cutoff, try `fade NodeName over 0`)".to_string())
                        }
                        _ => unreachable!(),
                    }
                },
                Some("starting") => {
                    if tokens.get(0).map(String::as_str) != Some("restart") {
                        return Err(format!("next element after \"restart\" must be \"node\" or \"starting\""))
                    }
                    if tokens.get(2).map(String::as_str) != Some("node") {
                        return Err(format!("next element after \"starting\" must be \"node\""))
                    }
                    if tokens.len() != 3 {
                        return Err(format!("nothing is allowed after \"restart starting node\""))
                    }
                    Ok(Some(Command::RestartFlow))
                },
                Some(x) => Err(format!("invalid element \"{}\" next element after {:?} must be \"node\" or \"starting\"", x, tokens[0])),
                None => Err(format!("\"{:?}\" must be followed by \"node\" or \"starting\"", tokens[0]))
            }
        },
        "fade" => {
            if tokens.get(1).map(String::as_str) != Some("node") {
                return Err("next element after \"fade\" must be \"node\"".to_string())
            }
            let target = match tokens.get(2) {
                Some(x) => x,
                None => return Err("next element after \"node\" must be the name of the node to fade".to_string()),
            }.to_compact_string();
            if tokens.get(3).map(String::as_str) != Some("over") {
                return Err("next element after node name must be \"over\"".to_string())
            }
            let length = timebases.parse_time(&tokens[3..])?;
            Ok(Some(Command::FadeNodeOut(target, length)))
        },
        "set" => {
            let target = match tokens.get(1) {
                Some(x) => x,
                None => return Err("next element after \"set\" must be the name of the flow control to set".to_string()),
            }.to_compact_string();
            if tokens.get(2).map(String::as_str) != Some("to") {
                return Err("next element after node name must be \"to\"".to_string())
            }
            Ok(Some(Command::Set(target, parse_expression(&tokens[3..])?)))
        },
        "if" => {
            // If we get here, we're an inline if. No children.
            let (condition, rest) = parse_condition(&tokens[1..])?;
            let command = match parse_flow_command_tokens(rest, timebases)? {
                Some(x) => x,
                None => return Err("there needs to be a command after the \"then\"".to_string()),
            };
            Ok(Some(Command::If {
                branches: vec![
                    (condition, vec![command]),
                ], fallback_branch: vec![],
            }))
        },
        "else" => Err("else is not allowed here (try breaking it onto its own line)".to_string()),
        "elseif" => Err("elseif is not allowed here (try breaking it onto its own line)".to_string()),
        _ => Ok(None),
    }
}

fn parse_node_child_code(node: &DinNode, timebases: &TimebaseCollection) -> Result<Vec<Command>, String> {
    let mut timebases = timebases.make_child();
    let mut commands = vec![];
    for child in node.children.iter() {
        debug_assert!(!child.items.is_empty());
        if child.items[0] == "timebase" {
            timebases.parse_timebase_node(child)?;
        }
        else if child.items[0] == "node" {
            return Err(format!("line {}: nodes cannot be nested", child.lineno))
        }
        else if let Some(command) = parse_flow_command_node(child, &timebases, commands.last_mut())? {
            if let Some(command) = command {
                // it was a command to add
                commands.push(command);
            }
            else {
                // it was an `else` or `elseif`, and we have nothing to do
            }
        }
        else {
            return Err(format!("line {}: unknown node element {:?}", child.lineno, child.items[0]))
        }
    }
    Ok(commands)
}

fn parse_if_body(node: &DinNode, rest: &[String], timebases: &TimebaseCollection) -> Result<Vec<Command>, String> {
    if !rest.is_empty() {
        if !node.children.is_empty() {
            return Err(format!("{} can have an inline body (right after the \"then\") or children (indented lines afterward) but not both", node.items[0]))
        }
        let command = match parse_flow_command_tokens(rest, timebases)? {
            Some(x) => x,
            None => return Err("unknown command after \"then\"".to_string()),
        };
        Ok(vec![command])
    }
    else {
        // not an error if no children
        parse_node_child_code(node, timebases)
    }
}

/// Tentatively parse a `DinNode` that corresponds to a single command within
/// a `Node`.
///
/// - `Err(x)`: A parse error
/// - `Ok(None)`: Unknown command
/// - `Ok(Some(None))`: `else` or `else if` that successfully got folded into
///   a preceding `If`
/// - `Ok(Some(Some(command)))`: A command you must now append to the list
fn parse_flow_command_node(node: &DinNode, timebases: &TimebaseCollection, last_command: Option<&mut Command>) -> Result<Option<Option<Command>>, String> {
    if node.items[0] == "if" {
        // If we get here, we might either be an inline if or an expanded one.
        let (condition, rest) = parse_condition(&node.items[1..])?;
        let commands = parse_if_body(node, rest, timebases)?;
        Ok(Some(Some(Command::If {
            branches: vec![
                (condition, commands),
            ], fallback_branch: vec![],
        })))
    }
    else if node.items[0] == "else" {
        let (last_branches, last_fallback_branch) = match last_command {
            Some(Command::If { branches, fallback_branch }) => (branches, fallback_branch),
            _ => {
                return Err(format!("line {}: \"else\" without matching \"if\" (check indentation)", node.lineno))
            },
        };
        if node.items.get(1).map(String::as_str) == Some("if") {
            // We are an else if
            let (condition, rest) = parse_condition(&node.items[2..])?;
            let commands = parse_if_body(node, rest, timebases)?;
            last_branches.push((condition, commands));
        }
        else {
            // We are an else
            let commands = parse_if_body(node, &[], timebases)?;
            if !last_fallback_branch.is_empty() {
                return Err(format!("line {}: only one \"else\" is allowed for a given \"if\" chain (check indentation)", node.lineno))
            }
            if commands.is_empty() {
                return Err(format!("line {}: \"else\" must contain at least oen command (check indentation or delete this line)", node.lineno))
            }
            *last_fallback_branch = commands;
        }
        Ok(Some(None))
    }
    else if node.items[0] == "elseif" {
        let last_branches = match last_command {
            Some(Command::If { branches, .. }) => branches,
            _ => {
                return Err(format!("line {}: \"elseif\" without matching \"if\" (check indentation)", node.lineno))
            },
        };
        let (condition, rest) = parse_condition(&node.items[1..])?;
        let commands = parse_if_body(node, rest, timebases)?;
        last_branches.push((condition, commands));
        Ok(Some(None))
    }
    else {
        let parsed = parse_flow_command_tokens(&node.items, timebases);
        match parsed {
            Ok(Some(x)) => {
                if !node.children.is_empty() {
                    return Err(format!("line {}: this element must have no children", node.lineno))
                }
                Ok(Some(Some(x)))
            },
            Ok(None) => Ok(None),
            Err(x) => Err(format!("line {}: {}", node.lineno, x)),
        }
    }
}

impl Node {
    fn parse_node(din_node: &DinNode, timebases: &TimebaseCollection) -> Result<Node, String> {
        assert_eq!(din_node.items[0], "node");
        if din_node.items.len() != 2 {
            return Err(format!("line {}: node element must have a name", din_node.lineno))
        }
        let name = din_node.items[1].to_compact_string();
        let commands = parse_node_child_code(din_node, timebases)?;
        Ok(Node { name: Some(name), commands })
    }
}

impl Flow {
    fn parse_din_node(node: &DinNode, timebases: &TimebaseCollection, flows: &HashMap<CompactString, Arc<Flow>>) -> Result<Flow, String> {
        assert_eq!(node.items[0], "flow");
        if node.items.len() != 2 {
            return Err(format!("line {}: flow element must have a name", node.lineno))
        }
        let mut timebases = timebases.make_child();
        let name = node.items[1].to_compact_string();
        let mut nodes = flows.get(&name).map(|x| x.nodes.clone()).unwrap_or_else(|| { HashMap::new() });
        let mut start_node = Node::new();
        for child in node.children.iter() {
            debug_assert!(!child.items.is_empty());
            if child.items[0] == "timebase" {
                timebases.parse_timebase_node(child)?;
            }
            else if child.items[0] == "node" {
                let mut node = Node::parse_node(child, &timebases)?;
                Command::flatten_commands(&mut node.commands);
                nodes.insert(node.name.clone().unwrap(), Arc::new(node));
            }
            else if let Some(command) = parse_flow_command_node(child, &timebases, start_node.commands.last_mut())? {
                if let Some(command) = command {
                    // it was a command to add
                    start_node.commands.push(command);
                }
                else {
                    // it was an `else` or `elseif`, and we have nothing to do
                }
            }
            else {
                return Err(format!("line {}: unknown flow element {:?}", child.lineno, child.items[0]))
            }
        }
        Command::flatten_commands(&mut start_node.commands);
        let new_flow = Flow {
            name,
            start_node: Arc::new(start_node),
            nodes,
        };
        Ok(new_flow)
    }
}

impl Soundtrack {
    pub fn parse_source(mut self, source: &str) -> Result<Soundtrack, String> {
        let document = parse_din(source)?;
        let mut timebases = TimebaseCollection::new();
        for node in document.into_iter() {
            assert!(!node.items.is_empty());
            match node.items[0].as_str() {
                "timebase" => timebases.parse_timebase_node(&node)?,
                "sound" => {
                    let sound = Sound::parse_din_node(&node, &timebases)?;
                    self.sounds.insert(sound.name.clone(), Arc::new(sound));
                },
                "sequence" => {
                    let sequence = Sequence::parse_din_node(&node, &timebases)?;
                    self.sequences.insert(sequence.name.clone(), Arc::new(sequence));
                },
                "flow" => {
                    let flow = Flow::parse_din_node(&node, &timebases, &self.flows)?;
                    self.flows.insert(flow.name.clone(), Arc::new(flow));
                },
                "region" => return Err(format!("line {}: regions may only exist inside sequences (check indentation)", node.lineno)),
                "node" => return Err(format!("line {}: nodes may only exist inside flows (check indentation)", node.lineno)),
                x => return Err(format!("line {}: unknown top-level element {:?}", node.lineno, x)),
            }
        }
        Ok(self)
    }
}

impl Command {
    pub fn flatten_commands(commands: &mut Vec<Command>) {
        let mut n = 0;
        while n < commands.len() {
            if let Command::If { .. } = &commands[n] {
                let eye_eff = commands.remove(n);
                let (branches, fallback_branch) = if let Command::If { branches, fallback_branch } = eye_eff { (branches, fallback_branch) } else { unreachable!() };
                // Well that was ugly.
                Command::insert_flattened_if(commands, n, branches, fallback_branch);
            }
            else {
                n += 1;
            }
        }
    }
    /// Performs one level of flattening. You'll still need to run the
    /// steamroller over the commands we insert.
    fn insert_flattened_if(commands: &mut Vec<Command>, insertion_point: usize, branches: Vec<(Vec<PredicateOp>, Vec<Command>)>, mut fallback_branch: Vec<Command>) {
        #[allow(clippy::identity_op)]
        let buffer_size = 0
            // Two gotos per branch
            + branches.len() * 2
            // One command per... command
            + branches.iter().fold(0, |a,x| a + x.1.len())
            + fallback_branch.len();
        let mut to_insert = Vec::with_capacity(buffer_size);
        let mut exit_goto_positions = Vec::with_capacity(branches.len());
        for (predicate, mut subcommands) in branches.into_iter() {
            let conditional_goto_position = to_insert.len();
            to_insert.push(Command::Placeholder);
            for subcommand in subcommands.iter_mut() {
                if let Command::Goto(_, _, pos) = subcommand {
                    *pos += insertion_point + to_insert.len();
                }
            }
            to_insert.append(&mut subcommands);
            exit_goto_positions.push(to_insert.len());
            to_insert.push(Command::Placeholder);
            to_insert[conditional_goto_position]
                = Command::Goto(predicate, false, to_insert.len() + insertion_point);
        }
        for subcommand in fallback_branch.iter_mut() {
            if let Command::Goto(_, _, pos) = subcommand {
                *pos += insertion_point + to_insert.len();
            }
        }
        to_insert.append(&mut fallback_branch);
        let exit_position = to_insert.len() + insertion_point;
        for pos in exit_goto_positions.into_iter() {
            to_insert[pos]
                = Command::Goto(vec![], true, exit_position);
        }
        for command in commands.iter_mut() {
            if let Command::Goto(_, _, target) = command {
                if *target > insertion_point {
                    *target = *target + to_insert.len() - 1;
                }
            }
        }
        commands.splice(insertion_point .. insertion_point, to_insert.into_iter());
    }
}
