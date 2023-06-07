use std::{
    num::NonZeroUsize,
    time::Duration,
};

use super::*;

use tokio::runtime::Runtime;

mod buffer;
pub use buffer::*;
//mod stream;
//pub use stream::*;

pub(crate) trait SoundManImpl {
    fn new(delegate: Arc<dyn SoundDelegate>) -> BufferMan;
    /// Load the given sound. Recursive; call `load` N times, and you have to
    /// call `unload` N times before it will take effect.
    fn load(&mut self, sound: &str, start: f32, loading_rt: &Option<Arc<Runtime>>);
    /// Unload the given sound. The sound will actually stick around if it's
    /// currently being referenced by a decoder. Return true if the sound's
    /// live reference count becomes zero.
    fn unload(&mut self, sound: &str, start: f32) -> bool;
    /// Unload all sounds. As if `unload` were called on every currently loaded
    /// sound.
    fn unload_all(&mut self);
    /// Returns whether the given sound is *ready*, i.e. currently loaded and
    /// not awaiting.
    fn is_ready(&mut self, sound: &str, start: f32) -> bool;
    /// Request an instance of the given sound. If it's preloaded, this simply
    /// returns a reference to the preloaded sound. If it's streamed, this
    /// returns the decoder state for the given sound, and will (if background
    /// loading is in effect) queue up another instance of that decoder to be
    /// ready for next time.
    /// 
    /// Returns `None` if the sound isn't loaded yet, or (sometimes) if it
    /// hasn't been loaded with the given `start` or `loop_start`.
    /// 
    /// You *must* have previously requested a load of this sound with the
    /// given `start` *and* `loop_start`.
    /// 
    /// The returned stream will always be able to perfectly seek, iff the
    /// target position corresponds to `loop_start`. `SoundManager` always
    /// returns perfectly seekable streams, and `StreamManager` will handle the
    /// idiosynchrocies of individual decoding backends. Remember that the
    /// correct way to convert from a floating point second number to an
    /// integer sample frame number is to multiply by the sample rate and round
    /// down!
    fn get_sound(
        &mut self,
        sound: &str,
        start: f32,
        loop_start: f32
    ) -> Option<FormattedSoundStream>;
}

#[derive(Debug, PartialEq)]
enum SoundType { Buffered, Streamed }

struct SoundInfo {
    load_count: NonZeroUsize,
    sound_type: SoundType,
}

pub(crate) struct SoundMan {
    bufferman: BufferMan,
    streamman: BufferMan, // TODO :)
    delegate: Arc<dyn SoundDelegate>,
    sound_infos: HashMap<String, SoundInfo>,
    loading_rt: Option<Arc<Runtime>>,
}

impl SoundMan {
    pub fn new(
        delegate: Arc<dyn SoundDelegate>,
        num_threads: usize,
        background_loading: bool
    ) -> SoundMan {
        let loading_rt = if background_loading {
            use tokio::runtime::Builder;
            let runtime = if num_threads > 1 {
                Builder::new_multi_thread()
                    .worker_threads(num_threads)
                    .thread_name("SMS decoder")
                    .thread_stack_size(1 * 1024 * 1024)
                    .enable_time()
                    .build()
                    .expect("unable to create multithreaded Tokio runtime")
            }
            else {
                Builder::new_current_thread()
                    .enable_time()
                    .build()
                    .expect("unable to create unihreaded Tokio runtime")
            };
            let runtime_ref = Arc::new(runtime);
            spin_off_tokio(&runtime_ref);
            Some(runtime_ref)
        } else { None };
        SoundMan {
            bufferman: BufferMan::new(delegate.clone()),
            streamman: BufferMan::new(delegate.clone()),
            delegate,
            sound_infos: HashMap::new(),
            loading_rt,
        }
    }
    pub fn load(&mut self, sound: &Sound) {
        if let Some(info) = self.sound_infos.get_mut(&sound.path) {
            let target_type = if sound.stream { SoundType::Streamed } else { SoundType::Buffered };
            if target_type != info.sound_type {
                self.delegate.warning(&format!("sound file {:?} is both streamed and buffered", sound.path));
            }
            // already loaded
            match info.sound_type {
                SoundType::Streamed => {
                    self.streamman.load(&sound.path, sound.start, &self.loading_rt);
                    info.load_count = info.load_count.checked_add(1).unwrap();
                    if sound.loop_start != sound.start {
                        self.streamman.load(&sound.path, sound.start, &self.loading_rt);
                        info.load_count = info.load_count.checked_add(1).unwrap();
                    }
                },
                SoundType::Buffered => {
                    self.bufferman.load(&sound.path, sound.start, &self.loading_rt);
                    info.load_count = info.load_count.checked_add(1).unwrap();
                },
            }
        }
        else {
            // not yet loaded
            let load_count;
            let sound_type = if sound.stream {
                // load it as a streaming sound
                self.streamman.load(&sound.path, sound.start, &self.loading_rt);
                if sound.loop_start != sound.start {
                    self.streamman.load(&sound.path, sound.start, &self.loading_rt);
                    load_count = 2;
                } else { load_count = 1 }
                SoundType::Streamed
            }
            else {
                self.bufferman.load(&sound.path, sound.start, &self.loading_rt);
                load_count = 1;
                SoundType::Buffered
            };
            self.sound_infos.insert(sound.path.clone(), SoundInfo {
                sound_type,
                load_count: NonZeroUsize::new(load_count).unwrap(),
            });
        }
    }
    pub fn unload(&mut self, sound: &Sound) {
        match self.sound_infos.get_mut(&sound.path) {
            None => {
                self.delegate.warning(&format!("unbalanced unload of sound file {:?} (THIS IS A BUG IN SMS)", sound.path));
            },
            Some(sound_info) => {
                let new_load_count = match sound_info.sound_type {
                    SoundType::Streamed => {
                        self.streamman.unload(&sound.path, sound.start);
                        if sound.loop_start != sound.start {
                            self.streamman.unload(&sound.path, sound.loop_start);
                            sound_info.load_count.get().checked_sub(2)
                        }
                        else {
                            sound_info.load_count.get().checked_sub(1)
                        }
                    },
                    SoundType::Buffered => {
                        self.bufferman.unload(&sound.path, sound.start);
                        sound_info.load_count.get().checked_sub(1)
                    }
                };
                match new_load_count.and_then(NonZeroUsize::new) {
                    None => { self.sound_infos.remove(&sound.path); },
                    Some(x) => sound_info.load_count = x,
                }
            },
        }
    }
    pub fn unload_all(&mut self) {
        self.sound_infos.clear();
        self.bufferman.unload_all();
        self.streamman.unload_all();
    }
    pub fn is_ready(&mut self, sound: &Sound) -> bool {
        match self.sound_infos.get(&sound.path) {
            None => false, // not being loaded, therefore not ready
            Some(SoundInfo { sound_type: SoundType::Buffered, .. }) => {
                self.bufferman.is_ready(&sound.path, sound.start)
            },
            Some(SoundInfo { sound_type: SoundType::Streamed, .. }) => {
                self.streamman.is_ready(&sound.path, sound.start)
                && (sound.loop_start == sound.start || self.streamman.is_ready(&sound.path, sound.loop_start))
            },
        }
    }
    pub fn get_sound(&mut self, sound: &Sound) -> Option<FormattedSoundStream> {
        match self.sound_infos.get(&sound.path) {
            None => None, // not being loaded, therefore not ready
            Some(SoundInfo { sound_type: SoundType::Buffered, .. }) => {
                self.bufferman.get_sound(&sound.path, sound.start, sound.loop_start)
            },
            Some(SoundInfo { sound_type: SoundType::Streamed, .. }) => {
                self.streamman.get_sound(&sound.path, sound.start, sound.loop_start)
            },
        }
    }
}

fn spin_off_tokio(runtime: &Arc<Runtime>) {
    let runtime = runtime.clone();
    std::thread::Builder::new()
        .name(format!("SMS decoder runtime"))
        .spawn(move || {
            runtime.block_on(async {
                loop {
                    tokio::time::sleep(Duration::new(456, 0)).await;
                }    
            });
        })
        .expect("Unable to spawn background decoding master thread");
}