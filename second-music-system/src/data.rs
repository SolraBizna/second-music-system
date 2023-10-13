//! The "cold" parts of the Second Music System. Dead, immutable data.

use super::*;

use std::{
    borrow::Cow,
    collections::HashSet,
    str::FromStr,
};

mod parse;

// ASCII printable non-digit non-letter characters (excluding underscore and
// space and quote marks), and also throw in the funky inequality chars
pub const EXPRESSION_SPLIT_CHARS: &str = r##"!#$%&()*+,-./:;<=>?[\]^{|}~`@≤≥≠"##;

/// A string, or a number.
#[derive(Debug, Clone, PartialEq)]
pub enum StringOrNumber {
    String(CompactString),
    Number(f32),
}

impl StringOrNumber {
    /// When interpreting this as a boolean, it is true if:
    /// - String: not empty, not equal to "0", not equal to "false"
    /// - Number: not equal to zero (this means NaN is true)
    pub fn is_truthy(&self) -> bool {
        match self {
            StringOrNumber::String(s) => !s.is_empty() && s.as_str() != "0" && s.as_str() != "false",
            StringOrNumber::Number(n) => *n != 0.0,
        }
    }
    /// When interpreting this is a number:
    /// - Empty string: zero
    /// - String that is a valid number: that number
    /// - String that is an invalid number: NaN
    /// - Any number: that number
    pub fn as_number(&self) -> f32 {
        match self {
            StringOrNumber::String(s) => {
                if s.is_empty() { 0.0 }
                else { s.parse().unwrap_or(std::f32::NAN) }
            },
            StringOrNumber::Number(n) => *n,
        }
    }
    /// When interpreting this as a string:
    /// - Any string: that string
    /// - Any number: that number, rendered with default formatting, as a string
    pub fn as_string(&self) -> Cow<str> {
        match self {
            StringOrNumber::String(s) => Cow::from(s),
            StringOrNumber::Number(n) => Cow::from(format!("{}", n)),
        }
    }
}

impl Default for StringOrNumber {
    fn default() -> StringOrNumber { StringOrNumber::String(CompactString::new("")) }
}

impl From<String> for StringOrNumber {
    fn from(string: String) -> StringOrNumber { StringOrNumber::String(string.into()) }
}

impl From<CompactString> for StringOrNumber {
    fn from(string: CompactString) -> StringOrNumber { StringOrNumber::String(string) }
}

impl From<f32> for StringOrNumber {
    fn from(f: f32) -> StringOrNumber { StringOrNumber::Number(f) }
}

impl From<bool> for StringOrNumber {
    fn from(b: bool) -> StringOrNumber { StringOrNumber::Number(if b { 1.0 } else { 0.0 }) }
}

impl FromStr for StringOrNumber {
    type Err = String;
    fn from_str(i: &str) -> Result<StringOrNumber, String> {
        if let Ok(x) = i.parse() {
            Ok(StringOrNumber::Number(x))
        }
        else if let Some(x) = i.find(|x| EXPRESSION_SPLIT_CHARS.contains(x)) {
            Err(format!("character {:?} is not allowed in a flow control string", x))
        }
        else {
            Ok(StringOrNumber::String(i.to_compact_string()))
        }
    }
}

impl PartialOrd<StringOrNumber> for StringOrNumber {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self, rhs) {
            (StringOrNumber::String(lhs), StringOrNumber::String(rhs)) =>
                lhs.partial_cmp(rhs),
            (StringOrNumber::Number(lhs), StringOrNumber::Number(rhs)) =>
                lhs.partial_cmp(rhs),
            _ => None,
        }
    }
}

#[derive(Debug,PartialEq)]
pub(crate) struct Sound {
    // All times are in seconds
    // unique within a soundtrack
    pub(crate) name: CompactString,
    pub(crate) path: CompactString,
    pub(crate) start: PosFloat,
    pub(crate) end: PosFloat,
    /// If true, the underlying audio file should be streamed, rather than
    /// cached. (If some sounds request that it be streamed and others request
    /// that it be cached, whether it is streamed or cached is undefined.)
    pub(crate) stream: bool,
}

#[derive(Debug,PartialEq)]
pub(crate) enum SequenceElement {
    PlaySound {
        sound: CompactString,
        channel: CompactString, // default is `main`
        /// How many seconds of fade-in between starting and becoming full
        /// volume
        fade_in: PosFloat,
        /// How long, including the fade in, that it should play at full volume
        length: Option<PosFloat>,
        /// How many seconds of fade-out to have after `length`
        /// (whereas in the format, this is how long before `end` that the fade
        /// will *start*)
        fade_out: PosFloat,
    },
    PlaySequence {
        sequence: CompactString
    },
}

#[derive(Debug,PartialEq)]
pub(crate) struct Sequence {
    // unique within a soundtrack
    pub(crate) name: CompactString,
    pub(crate) length: PosFloat,
    pub(crate) elements: Vec<(PosFloat, SequenceElement)>,
}

impl Sequence {
    /// Call the given handlers at least once with every sound or sequence
    /// directly used by this Sequence.
    pub fn find_all_direct_dependencies<A,B>(&self, mut found_sound: A, mut found_sequence: B)
    where A: FnMut(&str), B: FnMut(&str) {
        for (_time, element) in self.elements.iter() {
            match element {
                SequenceElement::PlaySound { sound, .. } => {
                    found_sound(sound)
                },
                SequenceElement::PlaySequence { sequence } => {
                    found_sequence(sequence)
                },
            }
        }
    }
}

#[derive(Debug,PartialEq)]
pub(crate) enum Command {
    /// Conclude the current node without running any more commands.
    Done,
    /// Wait a certain number of seconds.
    Wait(PosFloat),
    /// Start a Sound playing (even if another instance of that sound is
    /// already playing)
    PlaySound(CompactString),
    /// Acts like `PlaySound` followed by `Wait`, but the amount of waiting
    /// depends on the length of the named sound (information about which may
    /// not be available at parse time).
    PlaySoundAndWait(CompactString),
    /// Start a Sequence playing (even if another instance of that sequence
    /// is already playing)
    PlaySequence(CompactString),
    /// Acts like `PlaySequence` followed by `Wait`, but the amount of waiting
    /// depends on the length of the named sequence (information about which
    /// may not be available at parse time).
    PlaySequenceAndWait(CompactString),
    /// Cause another Node to start in parallel (iff not already playing)
    StartNode(CompactString),
    /// Cause another Node to start in parallel (iff not already playing), or
    /// suddenly restart from the beginning (iff already playing)
    RestartNode(CompactString),
    /// As `RestartNode(the starting node)`
    RestartFlow,
    /// Cause another Node to fade out and go away (iff already playing)
    FadeNodeOut(CompactString, PosFloat),
    /// Change a FlowControl to a new value.
    Set(CompactString, Vec<PredicateOp>),
    /// If/else chain. **INTERMEDIATE PARSING STEP ONLY, MUST NOT OCCUR IN THE
    /// FINAL DATA**
    If {
        /// Conditions to check, and commands to run if they're true.
        branches: Vec<(Vec<PredicateOp>, Vec<Command>)>,
        /// The commands to run if the condition evaluates to false.
        fallback_branch: Vec<Command>,
    },
    /// Goto. If condition matches bool, jump to index. (Empty condition is
    /// always true.)
    Goto(Vec<PredicateOp>, bool, usize),
    /// Placeholder where a Goto is about to go. **INTERMEDIATE PARSING STEP
    /// ONLY, MUST NOT OCCUR IN THE FINAL DATA**
    Placeholder,
}

#[derive(Debug,PartialEq)]
pub(crate) struct Node {
    pub(crate) name: Option<CompactString>,
    pub(crate) commands: Vec<Command>,
}

impl Node {
    pub fn new() -> Node {
        Node { name: None, commands: vec![] }
    }
}

#[derive(Debug,PartialEq)]
pub(crate) struct Flow {
    // unique within a soundtrack
    pub(crate) name: CompactString,
    pub(crate) start_node: Arc<Node>,
    pub(crate) nodes: HashMap<CompactString, Arc<Node>>,
}

impl Flow {
    /// Call the given handlers at least once with every Sound or Sequence
    /// directly used by this Flow.
    pub fn find_all_direct_dependencies<A,B>(&self, mut found_sound: A, mut found_sequence: B)
    where A: FnMut(&str), B: FnMut(&str) {
        for node in Some(&self.start_node).into_iter().chain(self.nodes.values()) {
            for command in node.commands.iter() {
                use Command::*;
                match command {
                    PlaySound(x) | PlaySoundAndWait(x) => {
                        found_sound(x)
                    },
                    PlaySequence(x) | PlaySequenceAndWait(x) => {
                        found_sequence(x)
                    },
                    If { .. } => unreachable!("Command::If should not ever be in the final commands array, but was found"),
                    Placeholder => unreachable!("Command::Placeholder should not ever be in the final commands array, but was found"),
                    _ => (),
                }
            }
        }
    }
    /// Return a Vec containing every Sound used by this Flow, directly or
    /// indirectly. Calls the `missing_sound` and `missing_sequence` functions
    /// exactly once for each sound or sequence that is referred to, but not
    /// (currently) present within the Soundtrack.
    pub fn find_all_sounds<A,B>(
        &self,
        soundtrack: &Soundtrack,
        mut missing_sound: A,
        mut missing_sequence: B
    ) -> Vec<Arc<Sound>>
    where A: FnMut(&str), B: FnMut(&str) {
        let mut found_sounds = HashSet::new();
        let mut found_sequences = HashSet::new();
        let mut found_sound;
        let mut found_sequence;
        let mut indirects = Vec::with_capacity(soundtrack.sequences.len());
        found_sound = |sound_name: &str| {
            if !found_sounds.contains(sound_name) {
                found_sounds.insert(sound_name.to_compact_string());
                if !soundtrack.sounds.contains_key(sound_name) {
                    missing_sound(sound_name);
                }
            }
        };
        found_sequence = |sequence_name: &str| {
            if !found_sequences.contains(sequence_name) {
                found_sequences.insert(sequence_name.to_compact_string());
                indirects.push(sequence_name.to_compact_string());
                if !soundtrack.sequences.contains_key(sequence_name) {
                    missing_sequence(sequence_name);
                }
            }
        };
        self.find_all_direct_dependencies(&mut found_sound, &mut found_sequence);
        let mut n = 0;
        while n < indirects.len() {
            if let Some(sequence) = soundtrack.sequences.get(&indirects[n]) {
                let mut found_sequence = |sequence_name: &str| {
                    if !found_sequences.contains(sequence_name) {
                        found_sequences.insert(sequence_name.to_compact_string());
                        indirects.push(sequence_name.to_compact_string());
                        if !soundtrack.sequences.contains_key(sequence_name) {
                            missing_sequence(sequence_name);
                        }
                    }
                };
                sequence.find_all_direct_dependencies(&mut found_sound, &mut found_sequence)
            }
            n += 1;
        }
        found_sounds.into_iter().filter_map(|k| soundtrack.sounds.get(&k).cloned()).collect()
    }
}

#[derive(Debug,PartialEq)]
pub(crate) enum PredicateOp {
    /// Push the value of the given `FlowControl`, empty string if unset.
    PushVar(CompactString),
    /// Push the given value.
    PushConst(StringOrNumber),
    /// Pop two elements, push whether they're equal.
    Eq,
    /// Pop two elements, push whether they're unequal.
    NotEq,
    /// Pop two elements, push whether the second from the top is greater than
    /// the top.
    Greater,
    /// Pop two elements, push whether the second from the top is greater than
    /// or equal to the top.
    GreaterEq,
    /// Pop two elements, push whether the second from the top is lesser than
    /// the top.
    Lesser,
    /// Pop two elements, push whether the second from the top is lesser than
    /// or equal to the top.
    LesserEq,
    /// Pop two elements, push whether they're both truthy.
    And,
    /// Pop two elements, push whether at least one is truthy.
    Or,
    /// Pop two elements, push whether only one is truthy.
    Xor,
    /// Pop one element, push whether it's not truthy.
    Not,
    /// Pop two elements, push their numeric sum.
    Add,
    /// Pop two elements, push their numeric difference.
    Sub,
    /// Pop two elements, push their numeric prodict.
    Mul,
    /// Pop two elements, push their numeric quotient.
    Div,
    /// Pop two elements, push their numeric remainder. (As Lua % operator)
    Rem,
    /// Pop two elements, push the floor of their numeric quotient.
    IDiv,
    /// Pop two elements, push the result of exponentiation.
    Pow,
    /// Pop one element, return its sine (as degrees).
    Sin,
    /// Pop one element, return its cosine (as degrees).
    Cos,
    /// Pop one element, return its tangent (as degrees).
    Tan,
    /// Pop one element, return its arcsine (in degrees).
    ASin,
    /// Pop one element, return its arccosine (in degrees).
    ACos,
    /// Pop one element, return its arctangent (in degrees).
    ATan,
    /// Pop two elements, return atan2 (in degrees).
    ATan2,
    /// Pop one element, return its natural logarithm.
    Log,
    /// Pop one element, return its natural exponent.
    Exp,
    /// Pop one element, return its floor.
    Floor,
    /// Pop one element, return its ceiling.
    Ceil,
    /// Pop two elements, return the one closer to negative infinity.
    Min,
    /// Pop two elements, return the one closer to positive infinity.
    Max,
    /// Pop one element, return its absolute value.
    Abs,
    /// Pop one element, push -1 if it's negative, 1 if it's positive.
    Sign,
    /// Pop one element, push its negation.
    Negate,
}

#[cfg(test)] mod test;
