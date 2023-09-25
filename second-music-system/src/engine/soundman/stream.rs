use super::*;

use std::{
    borrow::BorrowMut,
    collections::{HashMap, hash_map::Entry as HashMapEntry},
    hash::{Hash, Hasher},
    ops::Deref,
    sync::{Arc, Weak},
};

use vecmap::{VecMap, map::Entry as VecMapEntry};
use parking_lot::Mutex;
use tokio::sync::oneshot;

struct EmptyStream;
impl SoundReader<u8> for EmptyStream {
    fn attempt_clone(&self, sample_rate: f32, speaker_layout: SpeakerLayout) -> FormattedSoundStream {
        FormattedSoundStream {
            sample_rate,
            speaker_layout,
            reader: (Box::new(EmptyStream) as Box<dyn SoundReader<u8>>).into(),
        }
    }
    fn can_be_cloned(&self) -> bool {
        true
    }
    fn estimate_len(&mut self) -> Option<u64> {
        Some(0)
    }
    fn read(&mut self, _buf: &mut [MaybeUninit<u8>]) -> usize {
        0
    }
    fn seek(&mut self, pos: u64) -> Option<u64> {
        // Tell a lie, that we can go anywhere
        Some(pos)
    }
    fn skip_coarse(&mut self, count: u64, _buf: &mut [MaybeUninit<u8>]) -> u64 {
        // Tell a lie, that we can skip anything
        count
    }
    fn skip_precise(&mut self, _count: u64, _buf: &mut [MaybeUninit<u8>]) -> bool {
        // Tell the truth, that we reached the end
        false
    }
}

fn empty_stream() -> FormattedSoundStream {
    FormattedSoundStream {
        sample_rate: 44100.0,
        speaker_layout: SpeakerLayout::Mono,
        reader: FormattedSoundReader::U8(Box::new(EmptyStream)),
    }
}

fn load_stream(delegate: &dyn SoundDelegate, name: &str, start_point: f32) -> (FormattedSoundStream, bool) {
    eprintln!("called load_stream({name:?})");
    match delegate.open_file(name) {
        None => {
            delegate.warning(&format!("Unable to open sound file: {:?}", name));
            (empty_stream(), true)
        },
        Some(mut stream) => {
            let start_point = (start_point * stream.sample_rate).floor() as u64;
            let can_seek = match stream.reader.seek(start_point) {
                None => {
                    false
                },
                Some(x) => {
                    let residual = match start_point.checked_sub(x) {
                        None => {
                            panic!("tried to seek to {}, ended up at {}. overshooting is not allowed!", start_point, x);
                        },
                        Some(x) => x,
                    };
                    if residual > 0 {
                        stream.reader.skip(residual * stream.speaker_layout.get_num_channels() as u64);
                    }
                    true
                },
            };
            if !can_seek {
                stream.reader.skip(start_point * stream.speaker_layout.get_num_channels() as u64);
            }
            (stream, can_seek)
        },
    }
}

/// A simple SoundMan implementation that loads streams when `get` is called
/// and otherwise performs no bookkeeping. Used for StreamMan when background
/// loading is not in effect.
pub(crate) struct ForegroundStreamMan {
    delegate: Arc<dyn SoundDelegate>,
}

impl ForegroundStreamMan {
    pub(crate) fn new(delegate: Arc<dyn SoundDelegate>) -> ForegroundStreamMan {
        ForegroundStreamMan { delegate }
    }
}

impl SoundManImpl for ForegroundStreamMan {
    fn load(&mut self, _sound: &str, _start: f32, loading_rt: &Option<Arc<Runtime>>) {
        assert!(loading_rt.is_none(), "ForegroundStreamMan is being used, but there is a background loading runtime!");
    }

    fn unload(&mut self, _sound: &str, _start: f32) -> bool {
        true
    }

    fn unload_all(&mut self) {
    }

    fn is_ready(&mut self, _sound: &str, _start: f32) -> bool {
        true
    }
    fn get_sound(
        &mut self,
        sound: &str,
        start: f32,
        end: f32,
    ) -> Option<FormattedSoundStream> {
        Some(load_stream(&*self.delegate, sound, start).0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A non-NaN, positive f32.
// TODO: infect BufferMan with this, make it part of SoundMan
struct StartTime(f32);
impl StartTime {
    fn new(start: f32, delegate: &dyn SoundDelegate) -> StartTime {
        if !start.is_finite() {
            delegate.warning(&format!("reference to a non-finite start time"));
            StartTime(0.0)
        } else if !start.is_sign_positive() {
            delegate.warning(&format!("reference to a negative start time"));
            StartTime(0.0)
        } else { StartTime(start) }
    }
    fn target_sample_frame_count(&self, sample_rate: f32) -> u64 {
        (self.0 * sample_rate).floor() as u64
    }
}
impl Eq for StartTime {}
impl PartialOrd for StartTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.to_bits().partial_cmp(&other.0.to_bits())
    }
}
impl Ord for StartTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bits().cmp(&other.0.to_bits())
    }
}
impl Hash for StartTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(self.0.is_finite() && self.0.is_sign_positive());
        self.0.to_bits().hash(state);
    }
}
impl Deref for StartTime {
    type Target = f32;
    fn deref(&self) -> &f32 {
        &self.0
    }
}

/// Tea, predicated on a question.
#[derive(Debug)]
enum Predicated<T,F=(),U=()> {
    /// We don't yet know whether tea is available.
    Unknown(U),
    /// We know that tea is unavailable.
    Unavailable(F),
    /// We know that tea is available, here it is.
    Available(T),
}

impl<T, F, U> Predicated<T, F, U> {
    pub fn as_ref(&self) -> Predicated<&T,&F,&U> {
        match self {
            Predicated::Unknown(x) => Predicated::Unknown(x),
            Predicated::Unavailable(x) => Predicated::Unavailable(x),
            Predicated::Available(x) => Predicated::Available(x),
        }
    }
    pub fn as_mut(&mut self) -> Predicated<&mut T, &mut F, &mut U> {
        match self {
            Predicated::Unknown(x) => Predicated::Unknown(x),
            Predicated::Unavailable(x) => Predicated::Unavailable(x),
            Predicated::Available(x) => Predicated::Available(x),
        }
    }
}

impl<T, F, U: Default> Default for Predicated<T,F,U> {
    fn default() -> Self {
        Self::Unknown(U::default())
    }
}

/// A single decoder for a particular Sound at a particular start point, which
/// may or may not have become available yet.
enum CachedStream {
    /// A stream that hasn't been loaded yet, but whose loading has been
    /// requested.
    LoadingStream(oneshot::Receiver<(FormattedSoundStream, bool)>),
    /// A stream that has been loaded, and is currently ready.
    LoadedStream(FormattedSoundStream, bool),
}

impl CachedStream {
    fn begin_loading(delegate: Arc<dyn SoundDelegate>, name: String, start_point: StartTime, loading_runtime: &Arc<Runtime>) -> CachedStream {
        let (tx, rx) = oneshot::channel();
        loading_runtime.spawn(async move {
            let _ = tx.send(load_stream(&*delegate, &name, *start_point));
        });
        CachedStream::LoadingStream(rx)
    }
    /// If we are a `LoadingStream`, check if we should actually become a
    /// `LoadedStream` instead. If so, mutate.
    fn check_loading(&mut self, delegate: &dyn SoundDelegate, name: &str) {
        if let CachedStream::LoadingStream(rx) = self {
            match rx.try_recv() {
                Ok((stream, can_seek)) => *self = CachedStream::LoadedStream(stream, can_seek),
                Err(oneshot::error::TryRecvError::Empty) => {
                    // nothing we can do right now
                }
                _ => {
                    delegate.warning(&format!("Background loading stream {:?} failed", name));
                    *self = CachedStream::LoadedStream(empty_stream(), true);
                },
            }
        }
    }
    fn is_ready(&self) -> bool {
        match self {
            CachedStream::LoadedStream(_, _) => true,
            _ => false,
        }
    }
    fn needs_check(&self) -> bool {
        match self {
            CachedStream::LoadingStream(_) => true,
            _ => false,
        }
    }
}

/// Refers to only the requested loadings of a particular sound at a
/// particular start point.
struct AtStartPoint {
    loads: u32,
    // If cloneable, just one copy of the stream, ready to clone.
    // If not cloneable, a whole array of CachedStreams, with length kept
    // equal to `loads`.
    // If not loaded yet, a single CachedStream of the first attempt to load.
    cloneable: Predicated<FormattedSoundStream, Vec<CachedStream>, CachedStream>,
}

impl AtStartPoint {
    fn load_one_more(&mut self, delegate: &Arc<dyn SoundDelegate>, name: &str, start_point: StartTime, loading_runtime: &Arc<Runtime>) {
        self.loads += 1;
        if let Predicated::Unavailable(x) = self.cloneable.as_mut() {
            while x.len() < self.loads as usize {
                x.push(CachedStream::begin_loading(delegate.clone(), name.to_string(), start_point, loading_runtime));
            }   
        }
    }
    fn check_load(&mut self, delegate: &Arc<dyn SoundDelegate>, sound: &str, start_point: StartTime, loading_rt: &Weak<Runtime>) -> Option<()> {
        if let Predicated::Unknown(cached) = self.cloneable.as_mut() {
            cached.check_loading(&**delegate, sound);
            match cached {
                CachedStream::LoadingStream(_) => (),
                CachedStream::LoadedStream(stream, can_seek) => {
                    let mut alt = FormattedSoundStream {
                        sample_rate: stream.sample_rate,
                        speaker_layout: stream.speaker_layout,
                        reader: FormattedSoundReader::U8(Box::new(EmptyStream)),
                    };
                    std::mem::swap(stream, &mut alt);
                    let stream = alt;
                    if stream.can_be_cloned() {
                        self.cloneable = Predicated::Available(stream);
                    } else {
                        let mut vec = Vec::with_capacity(self.loads as usize);
                        vec.push(CachedStream::LoadedStream(stream, *can_seek));
                        let loading_rt = loading_rt.upgrade()?;
                        while vec.len() < self.loads as usize {
                            vec.push(CachedStream::begin_loading(delegate.clone(), sound.to_string(), start_point, &loading_rt));
                        }
                        self.cloneable = Predicated::Unavailable(vec);
                    }
                },
            }
        }
        Some(())
    }
}

/// Contains all of the requested loadings of an individual sound, potentially
/// at different starting points.
#[derive(Default)]
struct IndividualSound {
    /// If the stream is cloneable *and* seekable, here's a decoder at the
    /// beginning of this sound.
    cloneable_and_seekable: Predicated<FormattedSoundStream>,
    カンバン: VecMap<StartTime, AtStartPoint>,
}

/// A SoundMan implementation that performs stream loading in the background,
/// but decodes the audio on an as-needed basis in the sound thread. Used as
/// StreamMan when background loading is in effect.
pub(crate) struct BackgroundStreamMan {
    delegate: Arc<dyn SoundDelegate>,
    sounds: HashMap<String, IndividualSound>,
    loading_rt: Weak<Runtime>,
}

impl BackgroundStreamMan {
    pub(crate) fn new(delegate: Arc<dyn SoundDelegate>, loading_rt: &Arc<Runtime>) -> BackgroundStreamMan {
        BackgroundStreamMan {
            delegate,
            sounds: HashMap::new(),
            loading_rt: Arc::downgrade(loading_rt),
        }
    }
}

impl SoundManImpl for BackgroundStreamMan {
    fn load(&mut self, sound: &str, start: f32, loading_rt: &Option<Arc<Runtime>>) {
        let loading_rt = loading_rt.as_ref().unwrap();
        let individual_sound = if let Some(individual_sound) = self.sounds.get_mut(sound) {
            individual_sound
        } else {
            self.sounds.entry(sound.to_string()).or_default().borrow_mut()
        };
        let start = StartTime::new(start, &*self.delegate);
        match individual_sound.カンバン.entry(start) {
            VecMapEntry::Occupied(mut ent) => {
                ent.get_mut().load_one_more(&self.delegate, sound, start, loading_rt);
            },
            VecMapEntry::Vacant(ent) => {
                match individual_sound.cloneable_and_seekable.as_ref() {
                    Predicated::Available(parent) => {
                        let mut child = parent.attempt_clone();
                        let target_point = start.target_sample_frame_count(child.sample_rate);
                        let sought = child.reader.seek(target_point).expect("Bug in delegate: stream stopped being seekable!");
                        if sought < target_point {
                            // TODO: we should like to do this in a background
                            // thread
                            child.reader.skip((target_point - sought) * child.speaker_layout.get_num_channels() as u64);
                        }
                        ent.insert(AtStartPoint {
                            loads: 1,
                            cloneable: Predicated::Available(child),
                        });
                    },
                    _ => {
                        ent.insert(AtStartPoint {
                            loads: 1,
                            cloneable: Predicated::Unknown(CachedStream::begin_loading(self.delegate.clone(), sound.to_string(), start, loading_rt)),
                        });
                    },
                }
            },
        }
    }
    fn unload(&mut self, sound: &str, start: f32) -> bool {
        let individual_sound = if let Some(individual_sound) = self.sounds.get_mut(sound) {
            individual_sound
        } else {
            self.delegate.warning(&format!("SMS bug: unloaded something not loaded"));
            return true;
        };
        let start = StartTime::new(start, &*self.delegate);
        match individual_sound.カンバン.entry(start) {
            VecMapEntry::Occupied(mut ent) => {
                ent.get_mut().loads -= 1;
                if ent.get().loads == 0 {
                    ent.remove();
                    true
                } else {
                    // do not attempt to bring the number of CachedStreams
                    // down, we will use them up eventually
                    false
                }
            },
            VecMapEntry::Vacant(_) => {
                self.delegate.warning(&format!("SMS bug: unloaded something not loaded"));
                true
            },
        }
    }
    fn unload_all(&mut self) {
        self.sounds.clear();
    }
    fn is_ready(&mut self, sound: &str, start: f32) -> bool {
        let individual_sound = if let Some(individual_sound) = self.sounds.get_mut(sound) {
            individual_sound
        } else {
            return false;
        };
        let start = StartTime::new(start, &*self.delegate);
        let カンバン = match individual_sound.カンバン.get_mut(&start) {
            None => return false,
            Some(x) => x,
        };
        カンバン.check_load(&self.delegate, sound, start, &self.loading_rt);
        match カンバン.cloneable.as_mut() {
            Predicated::Unknown(_) => false,
            Predicated::Unavailable(x) => x.iter_mut().any(|x| {
                x.check_loading(&*self.delegate, sound);
                x.is_ready()
            }),
            Predicated::Available(_) => true,
        }
    }
    fn get_sound(
        &mut self,
        sound: &str,
        start: f32,
        _end: f32,
    ) -> Option<FormattedSoundStream> {
        // TODO: end
        let individual_sound = self.sounds.get_mut(sound)?;
        let start = StartTime::new(start, &*self.delegate);
        let カンバン = individual_sound.カンバン.get_mut(&start)?;
        カンバン.check_load(&self.delegate, sound, start, &self.loading_rt);
        match カンバン.cloneable.as_mut() {
            Predicated::Unknown(_) => unreachable!(),
            Predicated::Unavailable(vec) => {
                if let Some(i) = vec.iter_mut().position(|x| {
                    x.check_loading(&*self.delegate, sound);
                    x.is_ready()
                }) {
                    let loading_rt = self.loading_rt.upgrade()?;
                    let ret = vec.remove(i);
                    let ret = match ret {
                        CachedStream::LoadedStream(x, _) => x,
                        _ => unreachable!(),
                    };
                    while vec.len() < カンバン.loads as usize {
                        vec.push(CachedStream::begin_loading(self.delegate.clone(), sound.to_string(), start, &loading_rt));
                    }
                    Some(ret)
                } else { None }
            },
            Predicated::Available(parent) => {
                Some(parent.attempt_clone())
            },
        }
    }
}

/// Either a ForegroundStreamMan or a BackgroundStreamMan, depending on whether
/// background loading is in use.
///
/// Yes, I could have used a `dyn SoundManImpl` instead of making this enum.
/// But arbitrary jumps are predicted less well than a nearly-always-consistent
/// if.
pub(crate) enum StreamMan {
    Foreground(ForegroundStreamMan),
    Background(BackgroundStreamMan),
}

impl StreamMan {
    pub(crate) fn new(delegate: Arc<dyn SoundDelegate>, loading_rt: Option<&Arc<Runtime>>) -> StreamMan {
        match loading_rt {
            None => StreamMan::Foreground(ForegroundStreamMan::new(delegate)),
            Some(loading_rt) => StreamMan::Background(BackgroundStreamMan::new(delegate, loading_rt)),
        }
    }
}

impl SoundManImpl for StreamMan {
    fn load(&mut self, sound: &str, start: f32, loading_rt: &Option<Arc<Runtime>>) {
        match self {
            StreamMan::Foreground(x) => x.load(sound, start, loading_rt),
            StreamMan::Background(x) => x.load(sound, start, loading_rt),
        }
    }
    fn unload(&mut self, sound: &str, start: f32) -> bool {
        match self {
            StreamMan::Foreground(x) => x.unload(sound, start),
            StreamMan::Background(x) => x.unload(sound, start),
        }
    }
    fn unload_all(&mut self) {
        match self {
            StreamMan::Foreground(x) => x.unload_all(),
            StreamMan::Background(x) => x.unload_all(),
        }
    }
    fn is_ready(&mut self, sound: &str, start: f32) -> bool {
        match self {
            StreamMan::Foreground(x) => x.is_ready(sound, start),
            StreamMan::Background(x) => x.is_ready(sound, start),
        }
    }
    fn get_sound(
        &mut self,
        sound: &str,
        start: f32,
        end: f32,
    ) -> Option<FormattedSoundStream> {
        match self {
            StreamMan::Foreground(x) => x.get_sound(sound, start, end),
            StreamMan::Background(x) => x.get_sound(sound, start, end),
        }
    }
}