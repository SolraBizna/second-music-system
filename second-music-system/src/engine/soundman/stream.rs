use super::*;

use std::{
    borrow::BorrowMut,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

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

#[cfg(never)] mod die {
/// A single decoder for a given Sound, which may or may not have become
/// available yet.
enum CachedStream {
    /// A stream that hasn't been loaded yet, but whose loading has been
    /// requested.
    LoadingStream(oneshot::Receiver<(FormattedSoundStream, bool)>),
    /// A stream that has been loaded, and is currently ready.
    LoadedStream(FormattedSoundStream, bool),
}

impl CachedStream {
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

/// Information used to create new stream decoders. Only needed if the stream
/// can't be cloned.
struct StreamCacheReintake {
    delegate: Weak<dyn SoundDelegate>,
    // TODO: please intern strings
    name: String,
    start_point: f32,
    loading_runtime: Option<Weak<Runtime>>,
}

/// One or more decoders for a given sound AND start point. Some or all of the
/// decoders may not be available yet.
struct CachedStreams {
    reintake: Option<StreamCacheReintake>,
    cached: Vec<CachedStream>,
    desired_count: usize,
    can_clone: Option<bool>,
    can_seek: Option<bool>,
}

impl CachedStreams {
    fn new(reintake: StreamCacheReintake) -> CachedStreams {
        CachedStreams {
            reintake: Some(reintake),
            cached: vec![],
            desired_count: 0,
            can_clone: None,
            can_seek: None,
        }
    }
    /// Transform into an empty husk of our previous self. Any future streams
    /// that come from us will be empty. No further attempts to load will be
    /// made.
    fn nullify(&mut self) {
        self.reintake = None;
        self.cached = vec![CachedStream::LoadedStream(empty_stream(), true)];
        self.can_clone = Some(true);
        self.can_seek = Some(true);
    }
    fn load(&mut self) {
        self.desired_count += 1;
        if self.reintake.is_none() { return }
        match self.can_clone {
            Some(true) => assert!(self.cached.len() > 0),
            Some(false) => while self.cached.len() < self.desired_count && self.load_one_more() {},
            None => if self.cached.is_empty() { self.load_one_more(); },
        }
    }
    fn load_one_more(&mut self) -> bool {
        let reintake = self.reintake.as_ref().expect("SMS logic error: reintake was destroyed but it was still needed!");
        match reintake.loading_runtime.as_ref() {
            None => {
                // We will initiate the load on demand.
                false
            },
            Some(loading_runtime) => {
                let (loading_runtime, delegate) = match (loading_runtime.upgrade(), reintake.delegate.upgrade()) {
                    (Some(a), Some(b)) => (a, b),
                    _ => {
                        // something went away, which means we are closing
                        // down. accept our fate quietly
                        self.nullify();
                        return false
                    }
                };
                let (tx, rx) = oneshot::channel();
                let name = reintake.name.clone();
                let start_point = reintake.start_point;
                loading_runtime.spawn(async move {
                    let _ = tx.send(load_stream(&*delegate, &name, start_point));
                });
                self.cached.push(CachedStream::LoadingStream(rx));
                true
            },
        }
    }
    /// A balanced unload. Returns true if this cache should go away.
    fn unload(&mut self) -> bool {
        if self.desired_count == 0 {
            panic!("SMS logic error: CachedStreams' unload called too many times");
        }
        else {
            self.desired_count -= 1;
            self.desired_count == 0
        }
    }
    /// Return true if all requisite instances of this stream are ready to be
    /// dispensed.
    fn is_ready(&mut self) -> bool {
        match self.reintake.as_ref() {
            Some(reintake) if self.cached.iter().any(CachedStream::needs_check) => {
                // not loading in background, always ready
                if reintake.loading_runtime.is_none() {
                    eprintln!("No runtime");
                    return true
                }
                let delegate = match reintake.delegate.upgrade() {
                    None => {
                        self.nullify();
                        return true
                    }
                    Some(x) => x,
                };
                for x in self.cached.iter_mut() {
                    x.check_loading(&*delegate, &reintake.name);
                }
            }
            _ => return true, // ???
        }
        if self.can_clone == Some(true) {
            self.cached.iter().any(CachedStream::is_ready)
        }
        else {
            self.cached.iter().filter(|x| x.is_ready()).count()
                >= self.desired_count
        }
    }
    /// Take a stream out of the cache. If background loading is in effect,
    /// prepare a background task to load the stream again.
    fn get_sound(&mut self) -> Option<FormattedSoundStream> {
        if let Some(reintake) = self.reintake.as_ref() {
            if reintake.loading_runtime.is_none() {
                // We are not doing background loading. Load it now.
                let delegate = match reintake.delegate.upgrade() {
                    None => {
                        self.nullify();
                        return Some(empty_stream())
                    }
                    Some(x) => x,
                };
                let (stream, _can_seek) = load_stream(&*delegate, &reintake.name, reintake.start_point);
                return Some(stream);
            }
        }
        match self.can_clone {
            Some(true) => {
                // We've already determined that we can be cloned.
                debug_assert!(self.cached.len() == 1);
                match &self.cached[0] {
                    CachedStream::LoadedStream(x, _) => {
                        return Some(x.attempt_clone())
                    },
                    _ => unreachable!(),
                }
            },
            Some(false) => {
                // We cannot be cloned, and we know it.
                for n in 0 .. self.cached.len() {
                    if self.cached[n].is_ready() {
                        let ret = self.cached.remove(n);
                        if let CachedStream::LoadedStream(ret, _) = ret {
                            self.load_one_more();
                            return Some(ret)
                        }
                        else { unreachable!() }
                    }
                }
                return None
            },
            None => {
                // Find out if we can be cloned and seeked.
                assert!(self.cached.len() == 1);
                if !self.cached[0].is_ready() {
                    return None
                }
                let stream = self.cached.remove(0);
                let (stream, can_seek) = match stream {
                    CachedStream::LoadedStream(a,b) => (a,b),
                    _ => unreachable!(),
                };
                self.can_seek = Some(can_seek);
                if stream.can_be_cloned() {
                    // We can!
                    self.can_clone = Some(true);
                    self.cached.push(CachedStream::LoadedStream(stream.attempt_clone(), can_seek));
                }
                else {
                    // We can't be cloned.
                    self.can_clone = Some(false);
                    while self.cached.len() < self.desired_count && self.load_one_more() {}
                }
                return Some(stream)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A non-NaN f32.
struct StartTime(f32);
impl Eq for StartTime {}
impl Hash for StartTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Manages cached *streams*. We initialize decoders for them, prerolled to
/// the requested start positions, and dish them out on request. We use a lot
/// more CPU time, but a lot less memory.
pub struct StreamMan {
    delegate: Arc<dyn SoundDelegate>,
    streams: HashMap<String, HashMap<StartTime, Arc<Mutex<CachedStreams>>>>,
}

fn safe_and_positive(start: f32, delegate: &Arc<dyn SoundDelegate>) -> f32 {
    if start.is_nan() {
        delegate.warning(&format!("INTERNAL SMS BUG: StreamMan tried to load at a NaN start time, replacing with 0"));
        0.0
    } else if start.is_infinite() {
        delegate.warning(&format!("INTERNAL SMS BUG: StreamMan tried to load at an infinite start time, replacing with 0"));
        0.0
    } else if start < 0.0 {
        delegate.warning(&format!("INTERNAL SMS BUG: StreamMan tried to load at a negative start time ({:?}), replacing with 0", start));
        0.0
    } else if start == 0.0 { 0.0 } // make negative zero into positive zero
    else { start }
}

impl SoundManImpl for StreamMan {
    fn load(&mut self, sound: &str, start: f32, loading_rt: &Option<Arc<Runtime>>) {
        let start = safe_and_positive(start, &self.delegate);
        let sound_cache = match self.streams.get_mut(sound) {
            Some(x) => x,
            None => {
                self.streams.insert(sound.to_string(), HashMap::new());
                self.streams.get_mut(sound).unwrap()
            },
        };
        let point_cache = sound_cache.entry(StartTime(start)).or_insert_with(||{
             Arc::new(Mutex::new(CachedStreams::new(StreamCacheReintake {
                delegate: Arc::downgrade(&self.delegate),
                name: sound.to_string(),
                start_point: start,
                loading_runtime: loading_rt.as_ref().map(Arc::downgrade),
            })))
        }).borrow_mut();
        let mut point_cache = point_cache.lock();
        point_cache.load();
    }
    fn unload(&mut self, sound: &str, start: f32) -> bool {
        let start = safe_and_positive(start, &self.delegate);
        let sound_cache = match self.streams.get_mut(sound) {
            Some(x) => x,
            None => {
                self.delegate.warning(&format!("INTERNAL SMS BUG: unload({:?},{:?}) called but the sound was already unloaded!", sound, start));
                return true
            },
        };
        let point_cache = match sound_cache.get_mut(&StartTime(start)) {
            Some(x) => x,
            None => {
                self.delegate.warning(&format!("INTERNAL SMS BUG: unload({:?},{:?}) called but that sound was not loaded at that start time!", sound, start));
                return true
            },
        };
        let mut point_cache = point_cache.lock();
        let should_remove = point_cache.unload();
        if should_remove {
            drop(point_cache);
            sound_cache.remove(&StartTime(start));
            if sound_cache.is_empty() {
                self.streams.remove(sound);
            }
        }
        should_remove
    }
    fn unload_all(&mut self) {
        self.streams.clear();
    }
    fn is_ready(&mut self, sound: &str, start: f32) -> bool {
        let start = safe_and_positive(start, &self.delegate);
        let sound_cache = match self.streams.get_mut(sound) {
            Some(x) => x,
            None => return false,
        };
        let point_cache = match sound_cache.get_mut(&StartTime(start)) {
            Some(x) => x,
            None => return false,
        };
        point_cache.lock().is_ready()
    }
    fn get_sound(
        &mut self,
        sound: &str,
        start: f32,
        end: f32,
    ) -> Option<FormattedSoundStream> {
        let start = safe_and_positive(start, &self.delegate);
        let end = safe_and_positive(end, &self.delegate);
        if end <= start {
            self.delegate.warning(&format!("attempted to stream a sound of zero or negative length"));
            return Some(empty_stream())
        }
        let sound_cache = match self.streams.get_mut(sound) {
            Some(x) => x,
            None => return None,
        };
        let point_cache = match sound_cache.get_mut(&StartTime(start)) {
            Some(x) => x,
            None => return None,
        };
        point_cache.lock().get_sound()
    }
}

impl StreamMan {
    pub fn new(delegate: Arc<dyn SoundDelegate>) -> StreamMan {
        StreamMan {
            delegate,
            streams: HashMap::new(),
        }
    }
}

}