//! The "hot" parts of the Second Music System. Live, moving parts.

use super::*;

use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    fmt::{Debug, Formatter, Result as FmtResult},
    mem::{MaybeUninit, swap},
    num::NonZeroUsize,
};

mod mixer;
use mixer::*;
mod soundman;
use soundman::*;
mod adapter;
use adapter::*;
mod interpreter;
use interpreter::*;

#[cfg(feature="debug-channels")]
pub use mixer::MIX_CHANNELS;

/// The name of the default channel. The default channel is at volume 1.0 by
/// default, while all other channels are at 0.0. Additionally, the default
/// channel is exempted from the "all except main" channel commands.
pub const DEFAULT_CHANNEL: &str = "main";

mod privacy_hack {
    use super::*;
    #[derive(Debug)]
    pub enum EngineCommand {
        Transaction { commands: Vec<EngineCommand> },
        ReplaceSoundtrack { new_soundtrack: Soundtrack },
        Precache { flow_name: String },
        Unprecache { flow_name: String },
        UnprecacheAll {},
        // TODO: IsReady cannot fit here????
        SetFlowControl { control_name: String, new_value: StringOrNumber },
        ClearFlowControl { control_name: String },
        ClearPrefixedFlowControls { control_prefix: String },
        ClearAllFlowControls {},
        FadeMixControlTo { control_name: String, fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadePrefixedMixControlsTo { control_prefix: String, fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadeAllMixControlsTo { fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadeAllMixControlsExceptMainTo { fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadeMixControlOut { control_name: String, fade_type: FadeType, fade_length: PosFloat },
        FadePrefixedMixControlsOut { control_prefix: String, fade_type: FadeType, fade_length: PosFloat },
        FadeAllMixControlsOut { fade_type: FadeType, fade_length: PosFloat },
        FadeAllMixControlsExceptMainOut { fade_type: FadeType, fade_length: PosFloat },
        KillMixControl { control_name: String },
        KillPrefixedMixControls { control_prefix: String },
        KillAllMixControls { },
        KillAllMixControlsExceptMain { },
        StartFlow { flow_name: String, fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadeFlowTo { flow_name: String, fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadePrefixedFlowsTo { flow_prefix: String, fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadeAllFlowsTo { fade_type: FadeType, target_volume: PosFloat, fade_length: PosFloat },
        FadeFlowOut { flow_name: String, fade_type: FadeType, fade_length: PosFloat },
        FadePrefixedFlowsOut { flow_prefix: String, fade_type: FadeType, fade_length: PosFloat },
        FadeAllFlowsOut { fade_type: FadeType, fade_length: PosFloat },
        KillFlow { flow_name: String },
        KillPrefixedFlows { flow_prefix: String },
        KillAllFlows { },
    }
    pub trait EngineCommandIssuer {
        /// Issues an engine command, either directly or by batching.
        fn issue(&mut self, command: EngineCommand);
    }
}
use privacy_hack::EngineCommand;

#[cfg(feature="ffi-expose-issuer")]
pub use privacy_hack::EngineCommandIssuer;
#[cfg(not(feature="ffi-expose-issuer"))]
use privacy_hack::EngineCommandIssuer;

impl EngineCommands for dyn EngineCommandIssuer {}

pub trait EngineCommands : EngineCommandIssuer {
    /// Starts a new transaction. Commands that are issued will be batched
    /// together into one transaction, and delivered and processed all at once
    /// when the transaction is complete.
    ///
    /// - `length`: Your best guess as to the number of commands that will be
    ///   sent during this transaction. This is an optimization hint only.
    fn begin_transaction(&mut self, length: Option<usize>) -> Transaction<'_, Self> {
        Transaction {
            parent: self,
            commands: match length {
                None => Vec::new(),
                Some(x) => Vec::with_capacity(x),
            },
        }
    }
    /// Replace the active soundtrack with the given one. Currently-active
    /// nodes, sequences, and sounds will do their best to play to their
    /// conclusion.
    ///
    /// If you're replacing one soundtrack with an entirely different one, you
    /// probably want to fade or stop all flows first. If you're replacing it
    /// with a variation of the current soundtrack, such as one that contains
    /// additional flows, this replacement is seamless.
    fn replace_soundtrack(&mut self, new_soundtrack: Soundtrack) {
        self.issue(EngineCommand::ReplaceSoundtrack { new_soundtrack });
    }
    /// Requests that the given flow be precached for playback. The engine
    /// will attempt to load/preroll all requested sounds and streams in the
    /// background. Use TODO to determine when the loading is complete.
    ///
    /// This is *not* recursive. If you call `precache` twice, then call
    /// `unprecache` once, the flow will no longer be precached.
    fn precache(&mut self, flow_name: String) {
        self.issue(EngineCommand::Precache { flow_name });
    }
    /// Undoes a previous request that the given flow be precached for
    /// playback. This will lead the relevant sounds and streams to be purged
    /// once the flow stops playing (or immediately, if the flow is
    /// not currently playing).
    ///
    /// Commands sent from a given thread are always received in order, so it
    /// is completely reasonable to call `start_flow` immediately followed
    /// by `unprecache` for the same flow.
    ///
    /// This is *not* recursive. If you call `precache` twice, then call
    /// `unprecache` once, the flow will no longer be precached.
    fn unprecache(&mut self, flow_name: String) {
        self.issue(EngineCommand::Unprecache { flow_name });
    }
    /// Undoes all previous requests for precaching of flows. Flows that are
    /// currently in use will still remain in memory.
    ///
    /// Commands sent from a given thread are always received in order, so it
    /// is completely reasonable to call `start_flow` immediately followed
    /// by `unprecache_all`.
    fn unprecache_all(&mut self) {
        self.issue(EngineCommand::UnprecacheAll {});
    }
    /// Sets a given FlowControl to the given value.
    fn set_flow_control(&mut self, control_name: String, new_value: StringOrNumber) {
        self.issue(EngineCommand::SetFlowControl { control_name, new_value })
    }
    /// Clears a given FlowControl, removing any previous value.
    fn clear_flow_control(&mut self, control_name: String) {
        self.issue(EngineCommand::ClearFlowControl { control_name });
    }
    /// Clears all FlowControls whose names strictly start with the given
    /// prefix.
    fn clear_prefixed_flow_controls(&mut self, control_prefix: String) {
        self.issue(EngineCommand::ClearPrefixedFlowControls { control_prefix });
    }
    /// Clears all FlowControls.
    fn clear_all_flow_controls(&mut self) {
        self.issue(EngineCommand::ClearAllFlowControls { });
    }
    /// Fades a given MixControl to the given volume (0.0 to 1.0), using the
    /// given fading curve, over the given time period (in seconds).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_mix_control_to(&mut self, control_name: String, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeMixControlTo { control_name, fade_type, target_volume, fade_length });
    }
    /// Fades all *currently existing* mix controls whose names strictly
    /// start with the given prefix to the given volume (0.0 to 1.0), using the
    /// given fading curve, over the given time period (in seconds).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_prefixed_mix_controls_to(&mut self, control_prefix: String, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadePrefixedMixControlsTo { control_prefix, fade_type, target_volume, fade_length });
    }
    /// Fades all *currently existing* mix controls, *including* `main`, to
    /// the given volume (0.0 to 1.0), using the given fading curve, over the
    /// given time period (in seconds).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_all_mix_controls_to(&mut self, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeAllMixControlsTo { fade_type, target_volume, fade_length });
    }
    /// Fades all *currently existing* mix controls, *except* `main`, to the
    /// given volume (0.0 to 1.0), using the given fading curve, over the given
    /// time period (in seconds).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_all_mix_controls_except_main_to(&mut self, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeAllMixControlsExceptMainTo { fade_type, target_volume, fade_length });
    }
    /// Fades a given MixControl to zero volume, using the given fading curve,
    /// over the given time period (in seconds). When the fade is complete, the
    /// MixControl will be removed from existence rather than simply zeroed;
    /// future commands to "prefixed" and "all" will not resuscitate it
    /// (unless it is the target of a future, specific command).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_mix_control_out(&mut self, control_name: String, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeMixControlOut { control_name, fade_type, fade_length });
    }
    /// Fades all *currently existing* mix controls whose names strictly
    /// start with the given prefix to zero volume, using the given fading
    /// curve, over the given time period (in seconds). When the fade is
    /// complete, the MixControl will be removed from existence rather than
    /// simply zeroed; future commands to "prefixed" and "all" will not
    /// resuscitate it (unless it is the target of a future, specific
    /// command).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_prefixed_mix_controls_out(&mut self, control_prefix: String, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadePrefixedMixControlsOut { control_prefix, fade_type, fade_length });
    }
    /// Fades all *currently existing* mix controls, *including* `main`,
    /// to zero volume, using the given fading curve, over the given time
    /// period (in seconds). When the fade is complete, the MixControl will be
    /// removed from existence rather than simply zeroed; future commands to
    /// "prefixed" and "all" will not resuscitate it (unless it is the target
    /// of a future, specific command).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_all_mix_controls_out(&mut self, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeAllMixControlsOut { fade_type, fade_length });
    }
    /// Fades all *currently existing* mix controls, *except* `main`,
    /// to zero volume, using the given fading curve, over the given time
    /// period (in seconds). When the fade is complete, the MixControl will be
    /// removed from existence rather than simply zeroed; future commands to
    /// "prefixed" and "all" will not resuscitate it (unless it is the target
    /// of a future, specific command).
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_all_mix_controls_except_main_out(&mut self, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeAllMixControlsExceptMainOut { fade_type, fade_length });
    }
    /// Kills a given MixControl instantly, as if you yanked an audio cable.
    ///
    /// This is similar to fading that MixControl out over zero seconds, except
    /// that the MixControl in question is immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands), instead of only being
    /// removed the next time mixing takes place.
    fn kill_mix_control(&mut self, control_name: String) {
        self.issue(EngineCommand::KillMixControl { control_name });
    }
    /// Kills all MixControls whose names strictly start with the given prefix,
    /// as if you yanked an audio cable.
    ///
    /// This is similar to fading that MixControl out over zero seconds, except
    /// that the MixControl in question is immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands), instead of only being
    /// removed the next time mixing takes place.
    fn kill_prefixed_mix_controls(&mut self, control_prefix: String) {
        self.issue(EngineCommand::KillPrefixedMixControls { control_prefix });
    }
    /// Kills all MixControls, *including* `main`, as if you yanked an audio
    /// cable.
    ///
    /// This is similar to fading that MixControl out over zero seconds, except
    /// that the MixControl in question is immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands), instead of only being
    /// removed the next time mixing takes place.
    fn kill_all_mix_controls(&mut self) {
        self.issue(EngineCommand::KillAllMixControls { });
    }
    /// Kills all MixControls, *except* `main`, as if you yanked an audio
    /// cable.
    ///
    /// This is similar to fading that MixControl out over zero seconds, except
    /// that the MixControl in question is immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands), instead of only being
    /// removed the next time mixing takes place.
    fn kill_all_mix_controls_except_main(&mut self) {
        self.issue(EngineCommand::KillAllMixControlsExceptMain { });
    }
    /// Starts a given flow if it's not already playing. If the flow
    /// is being newly started, it will be faded up from zero volume to the
    /// target volume, with the given fade curve. If the flow was
    /// already playing, acts just like `fade_flow_to`.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn start_flow(&mut self, flow_name: String, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::StartFlow { flow_name, fade_type, target_volume, fade_length });
    }
    /// Fades a given flow to the given volume (0.0 to 1.0), using the
    /// given fading curve, over the given time period (in seconds). Does
    /// nothing if the flow is not currently playing.
    ///
    /// Flows with zero volume will continue silently "playing", waiting to
    /// be faded back up to non-zero volume. If this isn't what you want, use
    /// `fade_flow_out` instead.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_flow_to(&mut self, flow_name: String, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeFlowTo { flow_name, fade_type, target_volume, fade_length});
    }
    /// Fades all *currently playing* flows whose names strictly start with
    /// the given prefix to the given volume (0.0 to 1.0), using the given
    /// fading curve, over the given time period (in seconds). Does nothing to
    /// flows that haven't been started, or that have finished fading out.
    ///
    /// Flows with zero volume will continue silently "playing", waiting to
    /// be faded back up to non-zero volume. If this isn't what you want, use
    /// `fade_prefixed_flows_out` instead.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_prefixed_flows_to(&mut self, flow_prefix: String, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadePrefixedFlowsTo { flow_prefix, fade_type, target_volume, fade_length});
    }
    /// Fades all *currently playing* flows to the given volume (0.0 to
    /// 1.0), using the given fading curve, over the given time period (in
    /// seconds). Does nothing to flows that haven't been started, or that
    /// have finished fading out.
    ///
    /// Flows with zero volume will continue silently "playing", waiting to
    /// be faded back up to non-zero volume. If this isn't what you want, use
    /// `fade_prefixed_flows_out` instead.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals. Don't give a volume above 1.0 unless you are sure
    /// it won't cause clipping. Don't give negative volumes.
    fn fade_all_flows_to(&mut self, target_volume: PosFloat, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeAllFlowsTo { fade_type, target_volume, fade_length});
    }
    /// Fades a given flow to zero volume, using the given fading curve,
    /// over the given time period (in seconds). Does nothing if the flow
    /// is not currently playing, or has already faded out. When the fade is
    /// complete, the flow will be stopped.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_flow_out(&mut self, flow_name: String, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeFlowOut { flow_name, fade_type, fade_length});
    }
    /// Fades all *currently playing* flows whose names strictly start with
    /// the given prefix to zero volume, using the given fading curve, over the
    /// given time period (in seconds). Does nothing to flows that haven't
    /// been started, or that have already finished fading out.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_prefixed_flows_out(&mut self, flow_prefix: String, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadePrefixedFlowsOut { flow_prefix, fade_type, fade_length });
    }
    /// Fades all *currently playing* flows to zero volume, using the given
    /// fading curve, over the given time period (in seconds). Does nothing to
    /// flows that haven't been started, or that have already finished
    /// fading out.
    ///
    /// Use `FadeType::Exponential` unless you are doing intermixing of
    /// correlated signals.
    fn fade_all_flows_out(&mut self, fade_length: PosFloat, fade_type: FadeType) {
        self.issue(EngineCommand::FadeAllFlowsOut { fade_type, fade_length });
    }
    /// Kills a given flow instantly.
    ///
    /// This is similar to fading that flow out over zero seconds, except
    /// that the flow in question is immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands, and able to be started
    /// from the beginning), instead of only being removed the next time mixing
    /// takes place.
    fn kill_flow(&mut self, flow_name: String) {
        self.issue(EngineCommand::KillFlow { flow_name });
    }
    /// Kills all *currently playing* flows whose names strictly start with
    /// the given prefix instantly.
    ///
    /// This is similar to fading that flow out over zero seconds, except
    /// that the flow in question is immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands, and able to be started
    /// from the beginning), instead of only being removed the next time mixing
    /// takes place.
    fn kill_prefixed_flows(&mut self, flow_prefix: String) {
        self.issue(EngineCommand::KillPrefixedFlows { flow_prefix });
    }
    /// Kills all *currently playing* flows instantly.
    ///
    /// This is similar to fading those flows out over zero seconds, except
    /// that the flows in question are immediately removed (and therefore
    /// ineligible for `prefixed` or `all` commands, and able to be started
    /// from the beginning), instead of only being removed the next time mixing
    /// takes place.
    fn kill_all_flows(&mut self) {
        self.issue(EngineCommand::KillAllFlows { });
    }
}

/// An in-progress transaction. Create one by calling `begin_transaction` on
/// any type that can receive commands.
///
/// You can send commands to a transaction, exactly like you can to an
/// `Engine`. When you `commit` a transaction, all of the commands will be sent
/// at once, atomically, with neither a gap nor any interleaving with any other
/// commands. You can instead `abort` a transaction, in which case none of the
/// commands will be sent.
///
/// It is perfectly legal to call `begin_transaction` on a transaction. You can
/// go as deep as you like. If you do `B = A.begin_transaction()`, queue a
/// bunch of commands, and then do `B.commit()`, all of those commands will be
/// sent to `A` at once. If `A` is a transaction, then you still need to do
/// `A.commit()` if you want the commands to be carried out, or you can do
/// `A.abort()` to make it so that none of them happened at all.
pub struct Transaction<'a, T: EngineCommandIssuer + ?Sized> {
    parent: &'a mut T,
    commands: Vec<EngineCommand>,
}

impl<'a, T: EngineCommandIssuer + ?Sized> Transaction<'a, T> {
    /// Commits an in-progress transaction. All commands will arrive at the
    /// `Engine` at the same time, in the correct order.
    pub fn commit(self) {
        self.parent.issue(EngineCommand::Transaction { commands: self.commands })
    }
    /// Aborts an in-progress transaction. None of the commands put into it
    /// will be issued.
    ///
    /// Done implicitly if the transaction is not explicitly `commit`ted.
    pub fn abort(self) {}
}

impl<'a, T: EngineCommandIssuer + ?Sized> EngineCommandIssuer for Transaction<'a, T> {
    fn issue(&mut self, command: EngineCommand) {
        self.commands.push(command);
    }
}

impl<'a, T: EngineCommandIssuer + ?Sized> EngineCommands for Transaction<'a, T> {}

/// This exists to send commands to an `Engine` that belongs to some other
/// thread. If you're operating entirely in a single thread, you can also just
/// call any of these methods on an `Engine` directly.
#[derive(Clone)]
pub struct Commander {
    command_tx: Sender<EngineCommand>,
}

impl Commander {
    /// Makes another, independent `Commander` that sends commands to the
    /// same underlying `Engine`.
    ///
    /// Equivalent to `clone()`, but can also be called on an `Engine`.
    pub fn clone_commander(&self) -> Commander { self.clone() }
}

impl EngineCommandIssuer for Commander {
    fn issue(&mut self, command: EngineCommand) {
        let _ = self.command_tx.send(command);
    }
}

impl EngineCommands for Commander {}

/// This is the main moving part of the Second Music System. You create one of
/// these, give it a delegate to handle music decoding, and "turn the handle"
/// in your sound output code to make music come out.
pub struct Engine {
    live_soundtrack: Soundtrack,
    mixer: Mixer<PlayingSoundID>,
    command_rx: Receiver<EngineCommand>,
    // for cloning senders
    command_tx: Sender<EngineCommand>,
    flow_controls: HashMap<String, StringOrNumber>,
    mix_controls: HashMap<String, Fader>,
    flow_volumes: HashMap<String, Fader>,
    node_volumes: HashMap<StringAndAHalf, Fader>,
    /// Set of flows that are waiting to start.
    starting_flows: HashSet<String>,
    /// Set of flows that are fading out. Flows are added to this list
    /// when they are requested to fade *out*.
    flows_fading_out: HashSet<String>,
    /// Set of MixControls that are fading out. Controls are added to this list
    /// when they are requested to fade *out*.
    mix_controls_fading_out: HashSet<String>,
    deferred_kill: bool,
    // Will be updated on a best-effort basis. If the sound thread attempts to
    // lock it, and the lock is already held, the values will not be updated.
    flow_control_readout: Arc<RwLock<HashMap<String, StringOrNumber>>>,
    readout_needs_update: bool,
    sound_delegate: Arc<dyn SoundDelegate>,
    soundman: Box<dyn GenericSoundMan>,
    flow_loads: HashMap<String, FlowLoadStatus>,
    speaker_layout: SpeakerLayout,
    sample_rate: PosFloat,
    /// Temporary buffer for mixing
    mix_buf: Vec<MaybeUninit<f32>>,
    active_flow_nodes: Vec<ActiveNode>,
    queued_sounds: BinaryHeap<QueuedSound>,
}

impl EngineCommands for Engine {}

struct VolumeGetWrapper<'a, 'b> {
    mix_controls: &'a mut HashMap<String, Fader>,
    flow_volumes: &'a mut HashMap<String, Fader>,
    node_volumes: &'a mut HashMap<StringAndAHalf, Fader>,
    flows_fading_out: &'a HashSet<String>,
    starting_flows: &'a HashSet<String>,
    seen_flows: &'b mut HashSet<String>,
    seen_nodes: &'b mut HashSet<StringAndAHalf>,
}

/// A Node from a Flow, queued to execute.
#[derive(Debug)]
struct ActiveNode {
    /// The *name* of the flow this node is part of.
    flow_name: String,
    /// The *actual node* this node is.
    node: Arc<Node>,
    /// The time at which execution will resume.
    next_instruction_time: u64,
    /// The index of the next instruction we will execute
    next_instruction_index: usize,
}

/// A Sound that is going to play
struct QueuedSound {
    /// When?
    when: u64,
    /// Who?
    who: PlayingSoundID,
    /// What?
    sound: Arc<Sound>,
    fade_in: PosFloat,
    length: Option<PosFloat>,
    fade_out: PosFloat,
}

impl PartialEq for QueuedSound {
    fn eq(&self, other: &QueuedSound) -> bool {
        self.when == other.when
    }
}

impl Eq for QueuedSound {}

impl Ord for QueuedSound {
    fn cmp(&self, other: &QueuedSound) -> Ordering {
        // return a reversed comparison
        other.when.cmp(&self.when)
    }
}

impl PartialOrd for QueuedSound {
    fn partial_cmp(&self, other: &QueuedSound) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// oh boy
#[derive(Eq,PartialEq,Ord,PartialOrd,Hash,Clone,Debug)]
struct StringAndAHalf(String, Option<String>);

struct PlayingSoundID {
    // holy cow seriously, TODO: we need to intern strings!
    flow_and_node_name: StringAndAHalf,
    channel: String,
}

impl PlayingSoundID {
    fn flow_name(&self) -> &str { &self.flow_and_node_name.0 }
    fn node_name(&self) -> Option<&str> { self.flow_and_node_name.1.as_ref().map(String::as_str) }
}

impl Debug for PlayingSoundID {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        match self.flow_and_node_name.1.as_ref() {
            Some(x) => write!(fmt, "channel {:?}, flow {:?}/{:?}", self.channel, self.flow_and_node_name.0, x),
            None => write!(fmt, "channel {:?}, flow {:?}", self.channel, self.flow_and_node_name.0),
        }
    }
}

impl Engine {
    /// Creates a new Engine with the given properties, which will perform
    /// loading in the background using a purely internal `Switchyard`. Only
    /// available if you haven't disabled the `"switchyard"` feature.
    ///
    /// Once these properties are set, they cannot be changed without creating
    /// a new Engine.
    ///
    /// - `speaker_layout`: What kind of speaker layout your listener has. When
    ///   in doubt, use `Stereo`.
    /// - `sample_rate`: Number of samples per second you will be outputting.
    /// - `num_threads`: Number of threads to use for decoding and streaming.
    ///   If `None` or 0, will use a reasonable default based on the number of
    ///   available hardware threads and whether background loading is
    ///   requested.
    /// - `num_threads`: If `None`, will use a reasonable default based on the
    ///   number of available hardware threads. If `Some(x)`, will use that
    ///   exact number of hardware threads.
    /// - `affinity`: Offset added to core affinity of threads. When in doubt,
    ///   use `0`.
    pub fn new(
        sound_delegate: Arc<dyn SoundDelegate>,
        speaker_layout: SpeakerLayout,
        sample_rate: PosFloat,
        num_threads: Option<NonZeroUsize>,
        affinity: usize,
    ) -> Engine {
        let num_logical_cores = num_cpus::get();
        let num_threads = num_threads.map(NonZeroUsize::get)
        .unwrap_or_else(|| num_cpus::get() / 3).max(1);
        use ::switchyard::{Switchyard, threads::ThreadAllocationOutput};
        let runtime = Switchyard::new(
            (0..num_threads).map(|i| ThreadAllocationOutput {
                name: Some(format!("SMSworker{i}")),
                ident: i,
                stack_size: Some(1 * 1024 * 1024),
                affinity: Some((i+affinity)%num_logical_cores),
            }),
            || (),
        ).expect("Unable to create Switchyard runtime");
        Self::new_with_runtime(sound_delegate, speaker_layout, sample_rate, Arc::new(runtime))
    }
    /// Creates a new Engine with the given properties using a particular custom
    /// [`TaskRuntime`](trait.TaskRuntime.html) for loading tasks. If you want
    /// to perform offline rendering, or another task where you just want all
    /// loading to happen synchronously, pass `Arc::new(ForegroundTaskRuntime)`
    /// as the loading runtime. If you don't care, and haven't disabled the
    /// `switchyard` feature, just use `new` instead.
    ///
    /// Once these properties are set, they cannot be changed without creating
    /// a new Engine.
    ///
    /// - `speaker_layout`: What kind of speaker layout your listener has. When
    ///   in doubt, use `Stereo`.
    /// - `sample_rate`: Number of samples per second you will be outputting.
    /// - `num_threads`: Number of threads to use for decoding and streaming.
    ///   If `None` or 0, will use a reasonable default based on the number of
    ///   available hardware threads and whether background loading is
    ///   requested.
    pub fn new_with_runtime<Runtime: TaskRuntime>(
        sound_delegate: Arc<dyn SoundDelegate>,
        speaker_layout: SpeakerLayout,
        sample_rate: PosFloat,
        loading_rt: Arc<Runtime>,
    ) -> Engine {
        let (command_tx, command_rx) = unbounded();
        Engine {
            mixer: Mixer::new(speaker_layout.get_num_channels()),
            soundman: Box::new(SoundMan::new(sound_delegate.clone(), loading_rt)),
            sound_delegate, speaker_layout, sample_rate,
            command_tx, command_rx,
            live_soundtrack: Soundtrack::new(),
            flow_controls: HashMap::new(),
            mix_controls: [(DEFAULT_CHANNEL.to_string(), Fader::new(PosFloat::ONE))].into_iter().collect(),
            flow_volumes: HashMap::new(),
            node_volumes: HashMap::new(),
            flow_control_readout: Arc::new(RwLock::new(HashMap::new())),
            readout_needs_update: false,
            active_flow_nodes: vec![],
            queued_sounds: BinaryHeap::new(),
            mix_buf: vec![],
            flows_fading_out: HashSet::new(),
            mix_controls_fading_out: HashSet::new(),
            flow_loads: HashMap::new(),
            deferred_kill: false,
            starting_flows: HashSet::new(),
        }
    }
    /// Makes an independent `Commander` that can send commands to this
    /// `Engine` from another thread.
    pub fn clone_commander(&self) -> Commander {
        Commander {
            command_tx: self.command_tx.clone()
        }
    }
    /// Gets a copy of the Soundtrack that is currently live.
    pub fn copy_live_soundtrack(&self) -> Soundtrack {
        self.live_soundtrack.clone()
    }
    /// Gets a copy of all the FlowControls.
    pub fn copy_all_flow_controls(&self) -> HashMap<String, StringOrNumber> {
        self.flow_controls.clone()
    }
    /// Returns the `SpeakerLayout` this `Engine` was initialized for.
    pub fn get_speaker_layout(&self) -> SpeakerLayout { self.speaker_layout }
    /// Returns the sample rate this `Engine` was initialized for.
    pub fn get_sample_rate(&self) -> PosFloat { self.sample_rate }
    /// Mix some audio, advance time! `out` must have a number of elements
    /// divisible by the number of speaker channels. Any existing data in `out`
    /// is mixed with the active music data. You may or may not want to zero
    /// `out` before this call.
    pub fn turn_handle(&mut self, mut out: &mut [f32]) {
        assert_eq!(out.len() % self.speaker_layout.get_num_channels(), 0);
        let mut mix_buf = Vec::new();
        swap(&mut mix_buf, &mut self.mix_buf);
        // TODO: slim this, Bloom filter?
        let mut seen_flows = HashSet::with_capacity(self.active_flow_nodes.len()*2);
        let mut seen_nodes = HashSet::with_capacity(self.active_flow_nodes.len()*2);
        while out.len() > 0 {
            let now = self.mixer.get_next_output_sample_frame_number();
            // Here, at this command boundary, evaluate any commands we might
            // have received.
            while let Ok(cmd) = self.command_rx.try_recv() {
                self.issue(cmd);
            }
            // See if any newly-started flows are ready to start
            self.starting_flows.retain(|flow_name| {
                let load_status = self.flow_loads.get_mut(flow_name).unwrap();
                if load_status.is_ready(self.soundman.as_mut()) {
                    // oh boy! start the start node!
                    let flow =
                    self.live_soundtrack.flows.get(flow_name).unwrap();
                    self.active_flow_nodes.push(ActiveNode {
                        flow_name: flow_name.to_string(),
                        node: flow.start_node.clone(),
                        next_instruction_time: now,
                        next_instruction_index: 0,
                    });
                    false
                }
                else { true } // still waiting
            });
            // Process every active node
            let mut nodes_to_start: HashSet<StringAndAHalf> = HashSet::with_capacity(16);
            let mut nodes_to_restart: HashSet<StringAndAHalf> = HashSet::with_capacity(16);
            let flow_controls = &mut self.flow_controls;
            self.active_flow_nodes.retain_mut(|active_node| {
                if active_node.next_instruction_time > now { return true }
                let mut n = active_node.next_instruction_index;
                while n < active_node.node.commands.len() {
                    let next_command = &active_node.node.commands[n];
                    n += 1;
                    match next_command {
                        Command::Done => {
                            return false;
                        },
                        Command::Wait(sleep_time) => {
                            active_node.next_instruction_time = now + sleep_time.seconds_to_frames(self.sample_rate);
                            break;
                        },
                        Command::PlaySound(sound_name) => {
                            Self::execute_sound(&self.live_soundtrack, self.sample_rate, now, &active_node.flow_name, active_node.node.name.as_ref().map(String::as_str), &sound_name, &mut self.sound_delegate, &mut self.queued_sounds, DEFAULT_CHANNEL, PosFloat::ZERO, None, PosFloat::ZERO);
                        },
                        Command::PlaySoundAndWait(sound_name) => {
                            let sleep_time = Self::execute_sound(&self.live_soundtrack, self.sample_rate, now, &active_node.flow_name, active_node.node.name.as_ref().map(String::as_str), &sound_name, &mut self.sound_delegate, &mut self.queued_sounds, DEFAULT_CHANNEL, PosFloat::ZERO, None, PosFloat::ZERO);
                            active_node.next_instruction_time = now + sleep_time;
                            break;
                        },
                        Command::PlaySequence(seqname) => {
                            Self::execute_sequence(&self.live_soundtrack, self.sample_rate, now, &active_node.flow_name, active_node.node.name.as_ref().map(String::as_str), seqname, &mut self.sound_delegate, &mut self.queued_sounds);
                        },
                        Command::PlaySequenceAndWait(seqname) => {
                            let sleep_time = Self::execute_sequence(&self.live_soundtrack, self.sample_rate, now, &active_node.flow_name, active_node.node.name.as_ref().map(String::as_str), seqname, &mut self.sound_delegate, &mut self.queued_sounds);
                            active_node.next_instruction_time = now + sleep_time;
                            break;
                        },
                        Command::StartNode(node_name) => {
                            nodes_to_start.insert(StringAndAHalf(active_node.flow_name.clone(), Some(node_name.clone())));
                        },
                        Command::RestartNode(node_name) => {
                            nodes_to_restart.insert(StringAndAHalf(active_node.flow_name.clone(), Some(node_name.clone())));
                        },
                        Command::RestartFlow => {
                            nodes_to_restart.insert(StringAndAHalf(active_node.flow_name.clone(), None));
                        },
                        Command::FadeNodeOut(node_name, fade_length) => {
                            match self.node_volumes.get_mut(&StringAndAHalf(active_node.flow_name.clone(), Some(node_name.clone()))) {
                                Some(fader) => {
                                    let old_volume = fader.evaluate();
                                    *fader = Fader::start(FadeType::Linear, old_volume, PosFloat::ONE, fade_length.seconds_to_frac_frames(self.sample_rate));
                                },
                                None => self.sound_delegate.warning(&format!("missing node: {:?}::{:?}", active_node.flow_name, node_name))
                            }
                        },
                        Command::Set(control_name, ops) => {
                            flow_controls.insert(control_name.clone(), evaluate(flow_controls, ops));
                        },
                        Command::Goto(ops, cond, index) => {
                            if evaluate(flow_controls, ops).is_truthy() == *cond {
                                n = *index;
                            }
                        },
                        Command::If { .. } | Command::Placeholder => {
                            unreachable!("`If` and `Placeholder` commands should not survive long enough to be evaluated.");
                        }
                    }
                }
                active_node.next_instruction_index = n;
                active_node.next_instruction_index < active_node.node.commands.len()
            });
            for StringAndAHalf(flow_name, node_name) in nodes_to_start.into_iter() {
                let node_name = node_name.expect("SMS internal bug: a node with no name was put into nodes_to_start, which should not be possible");
                // Unwrapping this then wrapping it back in a Some feels wrong,
                // but it avoids iteration if node_name is `None`. Of course,
                // since it panics, does it really matter if we do a few
                // iterations before crashing? I don't know, but you know:
                // premature optimization and all that. Probably need to
                // re-evaluate this later. -n
                match self.active_flow_nodes.iter_mut().find(|x| x.flow_name == flow_name && x.node.name == Some(node_name.clone())) {
                    Some(_active_flow_node) => {
                        // Node is already playing. Do nothing.
                        self.sound_delegate.warning(&format!("attempt to start node {:?}, which was already playing", node_name));
                    },
                    None => {
                        // Node is not already playing. Start it.
                        let flow = match self.live_soundtrack.flows.get(&flow_name) {
                            None => {
                                // No such flow. (This should only happen
                                // when soundtrack shenanigans are happening.)
                                self.sound_delegate.warning(&format!("missing flow {:?} for node \"{:?}\"", flow_name, node_name));
                                continue;
                            },
                            Some(flow) => flow,
                        };
                        let node = match flow.nodes.get(&node_name) {
                            None => {
                                self.sound_delegate.warning(&format!("can't start missing node: {:?}::{:?}", flow_name, node_name));
                                continue;
                            },
                            Some(node) => node.clone(),
                        };
                        self.active_flow_nodes.push(ActiveNode {
                            flow_name, node,
                            next_instruction_time: now,
                            next_instruction_index: 0,
                        });
                    },
                }
            }
            for StringAndAHalf(flow_name, node_name) in nodes_to_restart.into_iter() {
                match self.active_flow_nodes.iter_mut().find(|x| x.flow_name == flow_name && x.node.name == node_name) {
                    Some(afn) => {
                        // Node is already playing. Restart it.
                        // TODO: Should this stop all sounds and sequences that
                        // it has caused to play?
                        afn.next_instruction_index = 0;
                        afn.next_instruction_time = now;
                    },
                    None => {
                        // Node is not already playing. Start it.
                        let flow = match self.live_soundtrack.flows.get(&flow_name) {
                            None => {
                                // No such flow. (This should only happen
                                // when soundtrack shenanigans are happening.)
                                self.sound_delegate.warning(&format!("can't restart missing flow: {:?}", flow_name));
                                continue;
                            },
                            Some(flow) => flow,
                        };
                        let node = match node_name {
                            None => flow.start_node.clone(),
                            Some(node_name) => {
                                match flow.nodes.get(&node_name) {
                                    None => {
                                        self.sound_delegate.warning(&format!("can't restart missing flow: {:?}::{:?}", flow_name, node_name));
                                        continue;
                                    },
                                    Some(node) => node.clone(),
                                }
                            },
                        };
                        self.active_flow_nodes.push(ActiveNode {
                            flow_name, node,
                            next_instruction_time: now,
                            next_instruction_index: 0,
                        });
                    },
                }
            }
            // TODO: Process every queued sequence, too
            // Consume queued sounds whose times have come
            while self.queued_sounds.peek().map(|x| x.when <= now).unwrap_or(false) {
                let queued_sound = self.queued_sounds.pop().unwrap();
                if let Some(adapter) = adaptify(&self.sound_delegate, self.soundman.as_mut(), &*queued_sound.sound, queued_sound.fade_in, queued_sound.length, queued_sound.fade_out, self.sample_rate, self.speaker_layout) {
                    self.mixer.play(adapter, queued_sound.who);
                }
            }
            let max_wait = self.get_num_sample_frames_until_next_exec();
            let buf_len = max_wait.map(|x| (x * self.speaker_layout.get_num_channels() as u64).min(out.len() as u64) as usize).unwrap_or(out.len());
            if buf_len > 0 {
                let buf = &mut out[..buf_len];
                buf.fill(0.0);
                if mix_buf.len() < buf.len() {
                    mix_buf.resize(buf.len(), MaybeUninit::uninit());
                }
                self.mixer.mix(buf, &mut mix_buf[..buf.len()],
                    VolumeGetWrapper {
                        mix_controls: &mut self.mix_controls,
                        flow_volumes: &mut self.flow_volumes,
                        node_volumes: &mut self.node_volumes,
                        flows_fading_out: &self.flows_fading_out,
                        starting_flows: &self.starting_flows,
                        seen_flows: &mut seen_flows,
                        seen_nodes: &mut seen_nodes,
                    });
                drop(buf);
                out = &mut out[buf_len..];
            }
        }
        self.mix_buf = mix_buf;
        if self.readout_needs_update {
            if let Some(mut flow_control_readout) = self.flow_control_readout.try_write() {
                *flow_control_readout = self.flow_controls.clone();
                self.readout_needs_update = false;
            }
        }
        #[cfg(feature="debug-flows")]
        {
            if let Some(mut target) = ACTIVE_FLOWS.try_lock() {
                let report = self.active_flow_nodes.iter().map(|x| {
                    let blah = format!("{:?}::{:?} next=[{}]@{}", x.flow_name, x.node.name, x.next_instruction_index, x.next_instruction_time);
                    blah
                }).collect();
                *target = report;
            }
        }
        self.kill_the_unseen(seen_flows, seen_nodes);
    }
    /// Returns the number of sample frames left to output before the next
    /// scheduled `Node` command or `Region` start, or none if the schedule is
    /// empty.
    fn get_num_sample_frames_until_next_exec(&self) -> Option<u64> {
        let now = self.mixer.get_next_output_sample_frame_number();
        let mut ret = None;
        for node in self.active_flow_nodes.iter() {
            let sooner = match ret {
                None => true,
                Some(time) => node.next_instruction_time < time,
            };
            if sooner {
                debug_assert!(node.next_instruction_time >= now);
                ret = Some(node.next_instruction_time);
            }
        }
        if let Some(x) = self.queued_sounds.peek() {
            let sooner = match ret {
                None => true,
                Some(time) => x.when < time,
            };
            if sooner {
                debug_assert!(x.when >= now);
                ret = Some(x.when);
            }
        }
        ret.map(|x| x - now)
    }
    fn perform_deferred_kill(&mut self) {
        if !self.deferred_kill { return }
        self.deferred_kill = false;
        let mut seen_flows = HashSet::with_capacity(self.active_flow_nodes.len()*2);
        let mut seen_nodes = HashSet::with_capacity(self.active_flow_nodes.len()*2);
        self.mixer.bump(
            VolumeGetWrapper {
                mix_controls: &mut self.mix_controls,
                flow_volumes: &mut self.flow_volumes,
                node_volumes: &mut self.node_volumes,
                flows_fading_out: &self.flows_fading_out,
                starting_flows: &self.starting_flows,
                seen_flows: &mut seen_flows,
                seen_nodes: &mut seen_nodes,
            });
        self.kill_the_unseen(seen_flows, seen_nodes);
    }
    /// Make nodes, flows, and mix controls that were not processed and (if
    /// relevant) have zero current volume stop existing.
    fn kill_the_unseen(&mut self, seen_flows: HashSet<String>, seen_nodes: HashSet<StringAndAHalf>) {
        self.flow_volumes.retain(|k, _| {
            if seen_flows.contains(k)
            || !self.flows_fading_out.contains(k)
            || self.starting_flows.contains(k) {
                true
            }
            else {
                let load_status = self.flow_loads.get_mut(k).unwrap();
                load_status.active_loading = false;
                load_status.maybe_unload(&self.live_soundtrack, self.soundman.as_mut());
                self.node_volumes.retain(|node_id, _| {
                    &node_id.0 != k
                });
                self.active_flow_nodes.retain(|afn| {
                    &afn.flow_name != k
                });
                false
            }
        });
        self.node_volumes.retain(|k, _| {
            if seen_nodes.contains(k) { true }
            else if self.starting_flows.contains(&k.0) { true }
            else if self.active_flow_nodes.iter().any(|afn| afn.flow_name == k.0 && afn.node.name.as_ref() == k.1.as_ref()) {
                true
            }
            else { false }
        });
        self.mix_controls.retain(|k, fader| {
            fader.evaluate() != PosFloat::ONE || !self.mix_controls_fading_out.contains(k)
        });
    }
    fn replace_soundtrack(&mut self, new_soundtrack: Soundtrack) {
        self.live_soundtrack = new_soundtrack;
        let mut new_flow_loads = HashMap::with_capacity(self.live_soundtrack.flows.len());
        for (flow_name, flow) in self.live_soundtrack.flows.iter() {
            let (active_loading, precaching) = match self.flow_loads.get(flow_name) {
                Some(x) => (x.active_loading, x.precaching),
                None => (false, false),
            };
            let mut new_load_status = FlowLoadStatus {
                active_loading, precaching,
                load_requested: false,
                known_all_ready: false,
                known_sounds: flow.find_all_sounds(
                    &self.live_soundtrack,
                    |name| self.sound_delegate.warning(&format!("missing sound: {:?}", name)),
                    |name| self.sound_delegate.warning(&format!("missing sequence: {:?}", name))
                ),
            };
            new_load_status.maybe_load(&self.live_soundtrack, self.soundman.as_mut());
            new_flow_loads.insert(flow_name.clone(), new_load_status);
        }
        // unload the old ones AFTER loading the new ones, that way anything
        // that's still in common will remain loaded
        for load_status in self.flow_loads.values_mut() {
            load_status.force_unload(&self.live_soundtrack, self.soundman.as_mut());
        }
        self.flow_loads = new_flow_loads;
    }
    /// Start a sequence being played. Returns the number of *sample frames*
    /// this sequence will last.
    fn execute_sequence(
        soundtrack: &Soundtrack,
        sample_rate: PosFloat,
        now: u64,
        flow_name: &str,
        node_name: Option<&str>,
        seqname: &str,
        sound_delegate: &mut Arc<dyn SoundDelegate>,
        queued_sounds: &mut BinaryHeap<QueuedSound>
    ) -> u64 {
        match soundtrack.sequences.get(seqname) {
            None => {
                sound_delegate.warning(&format!("can't play missing sequence: {:?}", seqname));
                0
            },
            Some(sequence) => {
                let sequence = sequence.clone();
                let len = sequence.length.seconds_to_frames(sample_rate);
                for (when, what) in sequence.elements.iter() {
                    let when = now + when.seconds_to_frames(sample_rate);
                    match what {
                        SequenceElement::PlaySequence { sequence } => {
                            assert_ne!(sequence, seqname);
                            Engine::execute_sequence(soundtrack, sample_rate, when, flow_name, node_name, seqname, sound_delegate, queued_sounds);
                        },
                        SequenceElement::PlaySound {
                            sound, channel, fade_in, length, fade_out,
                        } => {
                            Engine::execute_sound(soundtrack, sample_rate, when, flow_name, node_name, &sound, sound_delegate, queued_sounds, channel, *fade_in, *length, *fade_out);
                        }
                    }
                }
                len
            },
        }
    }
    /// Queues a sound to be played. Returns the number of *sample frames* this
    /// sound will last.
    fn execute_sound(
        soundtrack: &Soundtrack,
        sample_rate: PosFloat,
        when: u64,
        flow_name: &str,
        node_name: Option<&str>,
        sound_name: &str,
        sound_delegate: &mut Arc<dyn SoundDelegate>,
        queued_sounds: &mut BinaryHeap<QueuedSound>,
        channel: &str,
        fade_in: PosFloat,
        length: Option<PosFloat>,
        fade_out: PosFloat,
    ) -> u64 {
        let sound = match soundtrack.sounds.get(sound_name) {
            Some(x) => x.clone(),
            None => {
                sound_delegate.warning(&format!("can't play missing sound: {:?}", sound_name));
                return 0
            },
        };
        let ret = length.unwrap_or_else(|| sound.end.saturating_sub(sound.start)).seconds_to_frames(sample_rate);
        queued_sounds.push(QueuedSound {
            when,
            who: PlayingSoundID {
                flow_and_node_name: StringAndAHalf(
                    flow_name.to_string(),
                    node_name.map(str::to_string),
                ),
                channel: channel.to_string(),
            },
            sound,
            fade_in,
            length,
            fade_out,
        });
        ret
    }
}

impl VolumeGetter<PlayingSoundID> for VolumeGetWrapper<'_, '_> {
    fn step_faders_by(&mut self, n: PosFloat) {
        for (flow_name, fader) in self.flow_volumes.iter_mut() {
            if !self.starting_flows.contains(flow_name) {
                fader.step_by(n);
            }
        }
        for fader in self.node_volumes.values_mut() {
            fader.step_by(n);
        }
        for fader in self.mix_controls.values_mut() {
            fader.step_by(n);
        }
    }
    fn get_volume(&mut self, id: &PlayingSoundID, t: PosFloat) -> Option<PosFloat> {
        // the seen_* fields will be updated by `is_silent`
        let flow_fader = match self.flow_volumes.get_mut(id.flow_name()) {
            None => return None,
            Some(x) => x,
        };
        let flow_volume = flow_fader.evaluate_t(t);
        if flow_volume == PosFloat::ZERO && self.flows_fading_out.contains(id.flow_name()) {
            return None
        }
        let node_fader = match self.node_volumes.get_mut(&id.flow_and_node_name) {
            None => return None,
            Some(x) => x,
        };
        let node_volume = node_fader.evaluate();
        // Nodes cannot reach zero volume unless they are being faded out.
        if node_volume == PosFloat::ZERO { return None }
        let channel_fader = self.mix_controls.get_mut(&id.channel);
        let channel_volume = channel_fader.as_ref().map(|x| x.evaluate()).unwrap_or(PosFloat::ZERO);
        Some(flow_volume * node_volume * channel_volume)
    }
    fn is_varying(&mut self, id: &PlayingSoundID) -> Option<bool> {
        // stop if the flow has stopped
        let flow_fader = self.flow_volumes.get_mut(id.flow_name())?;
        // stop if the node has stopped
        let node_fader = self.node_volumes.get_mut(&id.flow_and_node_name)?;
        // DO NOT stop if the channel is silenced, UNLESS it's also fading
        if flow_fader.complete() && flow_fader.evaluate() == PosFloat::ONE && self.flows_fading_out.contains(id.flow_name()) {
            return None
        }
        if !self.seen_flows.contains(id.flow_name()) {
            self.seen_flows.insert(id.flow_name().to_string());
        }
        if !self.seen_nodes.contains(&id.flow_and_node_name) {
            self.seen_nodes.insert(StringAndAHalf(id.flow_name().to_string(), id.node_name().map(str::to_string)));
        }
        // TODO: "fader quality" setting
        Some(!flow_fader.complete() || !node_fader.complete())
    }
}

impl EngineCommandIssuer for Engine {
    // Since this is the Engine, we can handle the commands immediately instead
    // of shoving them in a channel or whatnot.
    //
    // Commands sent to us via our channel will end up being processed here.
    fn issue(&mut self, command: EngineCommand) {
        use EngineCommand::*;
        match command {
            Transaction { commands } => {
                for command in commands.into_iter() {
                    self.issue(command)
                }
            },
            ReplaceSoundtrack { new_soundtrack } => {
                self.replace_soundtrack(new_soundtrack);
            },
            Precache { flow_name } => {
                match self.flow_loads.get_mut(&flow_name) {
                    Some(load_status) => {
                        if load_status.precaching {
                            self.sound_delegate.warning(&format!("attempt to precache flow {:?} more than once", flow_name));
                        }
                        else {
                            load_status.precaching = true;
                            load_status.maybe_load(&self.live_soundtrack, self.soundman.as_mut());
                        }
                    },
                    None => {
                        self.sound_delegate.warning(&format!("attempt to precache flow {:?}, which does not exist", flow_name));
                    },
                }
            },
            Unprecache { flow_name } => {
                match self.flow_loads.get_mut(&flow_name) {
                    None => self.sound_delegate.warning(&format!("attempt to unprecache flow {:?}, which does not exist", flow_name)),
                    Some(load_status) => {
                        if load_status.precaching {
                            load_status.precaching = false;
                            load_status.maybe_unload(&self.live_soundtrack, self.soundman.as_mut());
                        }
                        else {
                            self.sound_delegate.warning(&format!("attempt to unprecache flow {:?} that wasn't currently precached", flow_name));
                        }
                    },
                }
            },
            UnprecacheAll {} => {
                for load_status in self.flow_loads.values_mut() {
                    if load_status.precaching {
                        load_status.precaching = false;
                        load_status.maybe_unload(&self.live_soundtrack, self.soundman.as_mut());
                    }
                }
            },
            SetFlowControl { control_name, new_value } => {
                self.flow_controls.insert(control_name, new_value);
            },
            ClearFlowControl { control_name } => {
                self.flow_controls.remove(&control_name);
            },
            ClearPrefixedFlowControls { control_prefix } => {
                self.flow_controls.retain(|k, _| {
                    !k.starts_with(&control_prefix)
                });
            },
            ClearAllFlowControls {} => {
                self.flow_controls.clear();
            },
            FadeMixControlTo { control_name, fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                self.mix_controls_fading_out.remove(&control_name);
                let old_volume = self.mix_controls.get(&control_name).map(Fader::evaluate).unwrap_or(PosFloat::ZERO);
                self.mix_controls.insert(control_name, Fader::start(fade_type, old_volume, target_volume, fade_length * self.sample_rate));
            },
            FadePrefixedMixControlsTo { control_prefix, fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                for (control_name, fader) in self.mix_controls.iter_mut() {
                    if control_name.starts_with(&control_prefix) {
                        self.mix_controls_fading_out.remove(control_name);
                        *fader = Fader::start(fade_type, fader.evaluate(), target_volume, fade_length.seconds_to_frac_frames(self.sample_rate));
                    }
                }
            },
            FadeAllMixControlsTo { fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                for (control_name, fader) in self.mix_controls.iter_mut() {
                    self.mix_controls_fading_out.remove(control_name);
                    *fader = Fader::start(fade_type, fader.evaluate(), target_volume, fade_length * self.sample_rate);
                }
            },
            FadeAllMixControlsExceptMainTo { fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                for (control_name, fader) in self.mix_controls.iter_mut() {
                    if control_name != DEFAULT_CHANNEL {
                        self.mix_controls_fading_out.remove(control_name);
                        *fader = Fader::start(fade_type, fader.evaluate(), target_volume, fade_length * self.sample_rate)
                    }
                }
            },
            FadeMixControlOut { control_name, fade_type, fade_length } => {
                self.perform_deferred_kill();
                if let Some(fader) = self.mix_controls.get_mut(&control_name) {
                    let old_volume = fader.evaluate();
                    *fader = Fader::start(fade_type, old_volume, PosFloat::ZERO, fade_length * self.sample_rate);
                    self.mix_controls_fading_out.insert(control_name);
                }
            },
            FadePrefixedMixControlsOut { control_prefix, fade_type, fade_length } => {
                self.perform_deferred_kill();
                for (control_name, fader) in self.mix_controls.iter_mut() {
                    if control_name.starts_with(&control_prefix) {
                        *fader = Fader::start(fade_type, fader.evaluate(), PosFloat::ZERO, fade_length * self.sample_rate);
                        self.mix_controls_fading_out.insert(control_name.to_string());
                    }
                }
            },
            FadeAllMixControlsOut { fade_type, fade_length } => {
                self.perform_deferred_kill();
                for (control_name, fader) in self.mix_controls.iter_mut() {
                    *fader = Fader::start(fade_type, fader.evaluate(), PosFloat::ZERO, fade_length * self.sample_rate);
                    self.mix_controls_fading_out.insert(control_name.to_string());
                }
            },
            FadeAllMixControlsExceptMainOut { fade_type, fade_length } => {
                self.perform_deferred_kill();
                for (control_name, fader) in self.mix_controls.iter_mut() {
                    if control_name != DEFAULT_CHANNEL {
                        *fader = Fader::start(fade_type, fader.evaluate(), PosFloat::ZERO, fade_length * self.sample_rate);
                        self.mix_controls_fading_out.insert(control_name.to_string());
                    }
                }
            },
            KillMixControl { control_name } => {
                if self.mix_controls.contains_key(&control_name) {
                    self.mix_controls.remove(&control_name);
                    self.mix_controls_fading_out.insert(control_name);
                    self.deferred_kill = true;
                }
            },
            KillPrefixedMixControls { control_prefix } => {
                self.mix_controls.retain(|control_name, _| {
                    if !control_name.starts_with(&control_prefix) { true }
                    else {
                        self.mix_controls_fading_out.insert(control_name.to_string());
                        self.deferred_kill = true;
                        false
                    }
                });
            },
            KillAllMixControls { } => {
                self.mix_controls.retain(|control_name, _| {
                    self.mix_controls_fading_out.insert(control_name.to_string());
                    self.deferred_kill = true;
                    false
                });
            },
            KillAllMixControlsExceptMain { } => {
                self.mix_controls.retain(|control_name, _| {
                    if control_name == DEFAULT_CHANNEL { true }
                    else {
                        self.mix_controls_fading_out.insert(control_name.to_string());
                        self.deferred_kill = true;
                        false
                    }
                });
            },
            StartFlow { flow_name, fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                let load_status = match self.flow_loads.get_mut(&flow_name) {
                    Some(x) => x,
                    None => {
                        self.sound_delegate.warning(&format!("attempt to start non-existent flow {:?}", flow_name));
                        return
                    },
                };
                if let Some(x) = self.flow_volumes.get(&flow_name) {
                    let old_volume = x.evaluate();
                    self.flows_fading_out.remove(&flow_name);
                    self.flow_volumes.insert(flow_name, Fader::start(fade_type, old_volume, target_volume, fade_length * self.sample_rate));
                }
                else {
                    load_status.active_loading = true;
                    load_status.maybe_load(&self.live_soundtrack, self.soundman.as_mut());
                    // we will check if it's loaded the next time the handle turns
                    self.starting_flows.insert(flow_name.clone());
                    self.node_volumes.insert(StringAndAHalf(flow_name.clone(), None), Fader::new(PosFloat::ONE));
                    self.flow_volumes.insert(flow_name, Fader::start(fade_type, PosFloat::ZERO, target_volume, fade_length * self.sample_rate));
                }
            },
            FadeFlowTo { flow_name, fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                self.flows_fading_out.remove(&flow_name);
                let old_volume = self.flow_volumes.get(&flow_name).map(Fader::evaluate).unwrap_or(PosFloat::ZERO);
                self.flow_volumes.insert(flow_name, Fader::start(fade_type, old_volume, target_volume, fade_length * self.sample_rate));
            },
            FadePrefixedFlowsTo { flow_prefix, fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                for (flow_name, fader) in self.flow_volumes.iter_mut() {
                    if flow_name.starts_with(&flow_prefix) {
                        self.flows_fading_out.remove(flow_name);
                        *fader = Fader::start(fade_type, fader.evaluate(), target_volume, fade_length * self.sample_rate);
                    }
                }
            },
            FadeAllFlowsTo { fade_type, target_volume, fade_length } => {
                self.perform_deferred_kill();
                for (flow_name, fader) in self.flow_volumes.iter_mut() {
                    self.flows_fading_out.remove(flow_name);
                    *fader = Fader::start(fade_type, fader.evaluate(), target_volume, fade_length * self.sample_rate);
                }
            },
            FadeFlowOut { flow_name, fade_type, fade_length } => {
                self.perform_deferred_kill();
                if let Some(fader) = self.flow_volumes.get_mut(&flow_name) {
                    let old_volume = fader.evaluate();
                    *fader = Fader::start(fade_type, old_volume, PosFloat::ZERO, fade_length * self.sample_rate);
                    self.flows_fading_out.insert(flow_name);
                }
            },
            FadePrefixedFlowsOut { flow_prefix, fade_type, fade_length } => {
                self.perform_deferred_kill();
                for (flow_name, fader) in self.flow_volumes.iter_mut() {
                    if flow_name.starts_with(&flow_prefix) {
                        *fader = Fader::start(fade_type, fader.evaluate(), PosFloat::ZERO, fade_length * self.sample_rate);
                        self.flows_fading_out.insert(flow_name.to_string());
                    }
                }
            },
            FadeAllFlowsOut { fade_type, fade_length } => {
                self.perform_deferred_kill();
                for (flow_name, fader) in self.flow_volumes.iter_mut() {
                    *fader = Fader::start(fade_type, fader.evaluate(), PosFloat::ZERO, fade_length * self.sample_rate);
                    self.flows_fading_out.insert(flow_name.to_string());
                }
            },
            KillFlow { flow_name } => {
                if self.starting_flows.remove(&flow_name) {
                    debug_assert!(self.flow_volumes.contains_key(&flow_name));
                }
                self.node_volumes.retain(|node_id, _| {
                    node_id.0 != flow_name
                });
                if self.flow_volumes.contains_key(&flow_name) {
                    self.flow_volumes.remove(&flow_name);
                    self.flows_fading_out.insert(flow_name);
                    self.deferred_kill = true;
                }
            },
            KillPrefixedFlows { flow_prefix } => {
                self.starting_flows.retain(|flow_name| {
                    !flow_name.starts_with(&flow_prefix)
                });
                self.node_volumes.retain(|node_id, _| {
                    !node_id.0.starts_with(&flow_prefix)
                });
                self.flow_volumes.retain(|flow_name, _| {
                    if !flow_name.starts_with(&flow_prefix) { true }
                    else {
                        self.flows_fading_out.insert(flow_name.to_string());
                        self.deferred_kill = true;
                        false
                    }
                });
            },
            KillAllFlows { } => {
                self.starting_flows.clear();
                self.node_volumes.clear();
                self.flow_volumes.retain(|flow_name, _| {
                    self.flows_fading_out.insert(flow_name.to_string());
                    self.deferred_kill = true;
                    false
                });
            },
        }
    }
}

/// Load/precache state of a Flow.
#[derive(Debug)]
struct FlowLoadStatus {
    /// True if we've determined that every Sound that this Flow requires is
    /// ready.
    known_all_ready: bool,
    /// True if this Flow is being precached, as opposed to being loaded
    /// because it's live.
    precaching: bool,
    /// True if this Flow is being loaded for a queued playback, or is
    /// currently active.
    active_loading: bool,
    /// True if this Flow has asked SoundMan to load its sounds.
    load_requested: bool,
    /// List of all Sounds that we knew this Flow requires.
    known_sounds: Vec<Arc<Sound>>,
}

impl FlowLoadStatus {
    fn should_be_loaded(&self) -> bool {
        self.precaching || self.active_loading
    }
    fn is_ready(&mut self, soundman: &mut dyn GenericSoundMan) -> bool {
        if self.known_all_ready {
            return true
        }
        for sound in self.known_sounds.iter() {
            if !soundman.is_ready(&**sound) {
                return false
            }
        }
        self.known_all_ready = true;
        return true
    }
    fn maybe_unload(&mut self, _live_soundtrack: &Soundtrack, soundman: &mut dyn GenericSoundMan) {
        if !self.load_requested || self.should_be_loaded() { return }
        for sound in self.known_sounds.iter() {
            soundman.unload(&*sound);
        }
        self.load_requested = false;
        self.known_all_ready = false;
    }
    fn maybe_load(&mut self, _live_soundtrack: &Soundtrack, soundman: &mut dyn GenericSoundMan) {
        if self.load_requested || !self.should_be_loaded() { return }
        for sound in self.known_sounds.iter() {
            soundman.load(&*sound);
        }
        self.load_requested = true;
    }
    fn force_unload(&mut self, live_soundtrack: &Soundtrack, soundman: &mut dyn GenericSoundMan) {
        self.precaching = false;
        self.active_loading = false;
        self.maybe_unload(live_soundtrack, soundman)
    }
}

#[cfg(feature="debug-flows")]
pub static ACTIVE_FLOWS: parking_lot::Mutex<Vec<String>> = parking_lot::Mutex::new(vec![]);
