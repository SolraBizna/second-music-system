use super::*;

use std::{
    sync::{Arc, Weak},
};

use tokio::sync::oneshot;

#[derive(Debug, Clone)]
struct Format {
    sample_rate: f32,
    speaker_layout: SpeakerLayout,
}

#[derive(Clone)]
enum FormattedVec {
    U8(Arc<Vec<u8>>),
    U16(Arc<Vec<u16>>),
    I8(Arc<Vec<i8>>),
    I16(Arc<Vec<i16>>),
    F32(Arc<Vec<f32>>),
}

#[derive(Clone)]
enum WeakFormattedVec {
    U8(Weak<Vec<u8>>),
    U16(Weak<Vec<u16>>),
    I8(Weak<Vec<i8>>),
    I16(Weak<Vec<i16>>),
    F32(Weak<Vec<f32>>),
}

impl FormattedVec {
    fn default() -> FormattedVec {
        FormattedVec::U8(Arc::new(vec![0u8; 128]))
    }
    fn len(&self) -> usize {
        match self {
            FormattedVec::U8(x) => x.len(),
            FormattedVec::U16(x) => x.len(),
            FormattedVec::I8(x) => x.len(),
            FormattedVec::I16(x) => x.len(),
            FormattedVec::F32(x) => x.len(),
        }
    }
    fn downgrade(&self) -> WeakFormattedVec {
        match self {
            FormattedVec::U8(x) => WeakFormattedVec::U8(Arc::downgrade(x)),
            FormattedVec::U16(x) => WeakFormattedVec::U16(Arc::downgrade(x)),
            FormattedVec::I8(x) => WeakFormattedVec::I8(Arc::downgrade(x)),
            FormattedVec::I16(x) => WeakFormattedVec::I16(Arc::downgrade(x)),
            FormattedVec::F32(x) => WeakFormattedVec::F32(Arc::downgrade(x)),
        }
    } 
}

impl WeakFormattedVec {
    fn default() -> WeakFormattedVec {
        WeakFormattedVec::U8(Weak::default())
    }
    fn upgrade(&self) -> Option<FormattedVec> {
        match self {
            WeakFormattedVec::U8(x) => x.upgrade().map(|x| FormattedVec::U8(x)),
            WeakFormattedVec::U16(x) => x.upgrade().map(|x| FormattedVec::U16(x)),
            WeakFormattedVec::I8(x) => x.upgrade().map(|x| FormattedVec::I8(x)),
            WeakFormattedVec::I16(x) => x.upgrade().map(|x| FormattedVec::I16(x)),
            WeakFormattedVec::F32(x) => x.upgrade().map(|x| FormattedVec::F32(x)),
        }
    }
}

impl Format {
    fn default() -> Format {
        Format {
            sample_rate: 44100.0,
            speaker_layout: SpeakerLayout::Mono,
        }
    }
}

enum CachedSound {
    /// A sound that hasn't been loaded yet, but whose loading has been
    /// requested.
    LoadingSound { rx: oneshot::Receiver<(Format, FormattedVec)>, load_count: u32 },
    /// A sound that has been loaded, and is currently being actively cached.
    LoadedSound { format: Format, vec: FormattedVec, load_count: u32 },
    /// A sound that was previously loaded, but whose load count has gone to
    /// zero, and which will get purged if no active playback requires it.
    UnloadedSound { format: Format, vec: WeakFormattedVec },
}

impl CachedSound {
    /// If we are a `LoadingSound`, check if we should actually become a
    /// `LoadedSound` instead. If so, mutate.
    fn check_loading(&mut self, delegate: &Arc<dyn SoundDelegate>, name: &str) {
        if let CachedSound::LoadingSound { load_count, rx } = self {
            match rx.try_recv() {
                Ok((format, vec)) if *load_count > 0 => {
                    *self = CachedSound::LoadedSound { load_count: *load_count, format, vec };
                },
                Ok(_) if *load_count == 0 => {
                    // Great, thanks for loading! But we don't want you anymore
                    // (it makes no sense to put it straight in as a weak vec,
                    // because it would immediately be freed)
                    *self = CachedSound::UnloadedSound { format: Format::default(), vec: WeakFormattedVec::default() }
                },
                Err(oneshot::error::TryRecvError::Empty) => {
                    // nothing to do right now
                }
                _ => {
                    delegate.warning(&format!("Background loading sound {:?} failed", name));
                    *self = CachedSound::LoadedSound { load_count: *load_count, format: Format::default(), vec: FormattedVec::default() };
                },
            }
        }
    }
}

/// Manages cached *sounds*. We load them completely, ahead of time. We use a
/// lot of memory, but our returned streams are very simple to decode.
pub struct BufferMan {
    delegate: Arc<dyn SoundDelegate>,
    sounds: HashMap<String, CachedSound>,
}

impl SoundManImpl for BufferMan {
    fn new(delegate: Arc<dyn SoundDelegate>) -> BufferMan {
        BufferMan {
            delegate,
            sounds: HashMap::new(),
        }
    }
    fn load(&mut self, sound: &str, _start: f32, loading_rt: &Option<Arc<Runtime>>) {
        if let Some(ent) = self.sounds.get_mut(sound) {
            ent.check_loading(&self.delegate, sound);
            match ent {
                CachedSound::LoadingSound { load_count, .. } | CachedSound::LoadedSound { load_count, .. } => {
                    *load_count += 1;
                    return;
                },
                CachedSound::UnloadedSound { format, vec } => {
                    if let Some(vec) = vec.upgrade() {
                        *ent = CachedSound::LoadedSound { load_count: 1, format: format.clone(), vec };
                        return;
                    }
                },
            }
        }
        // if we got here, the sound has never been loaded, or it was unloaded
        // and collected
        match loading_rt.as_ref() {
            Some(loading_rt) => {
                let (result_tx, result_rx) = oneshot::channel();
                let delegate = self.delegate.clone();
                let sound = sound.to_string();
                let sound_clone = sound.clone();
                loading_rt.spawn(async move {
                    let _ = result_tx.send(load_whole_sound(&delegate, &sound_clone));
                });
                self.sounds.insert(sound.to_string(),
                    CachedSound::LoadingSound {
                        load_count: 1,
                        rx: result_rx,
                });
            },
            None => {
                let (format, vec) = load_whole_sound(&self.delegate, sound);
                // if we got here, background loading isn't enabled, or wasn't possible
                self.sounds.insert(sound.to_string(), CachedSound::LoadedSound { load_count: 1, format, vec });
            },
        }
    }
    fn unload(&mut self, sound: &str, _start: f32) -> bool {
        match self.sounds.get_mut(sound) {
            None | Some(CachedSound::UnloadedSound { .. }) => {
                self.delegate.warning(&format!("unbalanced unload of sound {:?} (THIS IS A BUG IN SMS!)", sound));
                true
            },
            Some(x) => match x {
                CachedSound::LoadingSound { load_count, .. } => {
                    if *load_count > 0 {
                        *load_count -= 1;
                    }
                    else {
                        self.delegate.warning(&format!("unbalanced unload of sound {:?} (THIS IS A BUG IN SMS!)", sound));
                    }
                    *load_count == 0
                },
                CachedSound::LoadedSound { load_count, format, vec } => {
                    if *load_count > 1 {
                        *load_count -= 1;
                        false
                    }
                    else {
                        *x = CachedSound::UnloadedSound {
                            format: format.clone(),
                            vec: vec.downgrade(),
                        };
                        true
                    }
                },
                CachedSound::UnloadedSound { .. } => unreachable!(),
            },
        }
    }
    fn unload_all(&mut self) {
        for sound in self.sounds.values_mut() {
            match sound {
                CachedSound::LoadingSound { load_count, .. }
                    => *load_count = 0,
                CachedSound::LoadedSound { vec, format, .. }
                    => *sound = CachedSound::UnloadedSound {
                        format: format.clone(),
                        vec: vec.downgrade(),
                    },
                CachedSound::UnloadedSound { .. } => (),
            }
        }
    }
    fn is_ready(&mut self, sound: &str, _start: f32) -> bool {
        if let Some(x) = self.sounds.get_mut(sound) {
            x.check_loading(&self.delegate, sound);
            if let CachedSound::LoadedSound { .. } = x {
                return true
            }
        }
        false
    }
    fn get_sound(
        &mut self,
        sound: &str,
        start: f32,
        _loop_start: f32
    ) -> Option<FormattedSoundStream> {
        self.sounds.get_mut(sound).and_then(|s| {
            s.check_loading(&self.delegate, sound);
            match s {
                CachedSound::LoadedSound { format, vec, .. } => {
                    Some(new_buffer_stream(format, vec.clone(), start))
                },
                CachedSound::UnloadedSound { format, vec } => {
                    match vec.upgrade() {
                        None => None,
                        Some(vec) => {
                            Some(new_buffer_stream(format, vec.clone(), start))
                        },
                    }
                },
                CachedSound::LoadingSound { .. } => None,
            }
        })
    }
}

#[derive(Clone)]
struct BufferStream<T: Sample> {
    vec: Arc<Vec<T>>,
    cursor: usize,
    num_channels: usize,
}

fn new_buffer_stream(format: &Format, vec: FormattedVec, start: f32) -> FormattedSoundStream {
    let cursor = (start.seconds_to_index(format.sample_rate) * format.speaker_layout.get_num_channels() as u64).min(vec.len() as u64) as usize;
    let sample_rate = format.sample_rate;
    let speaker_layout = format.speaker_layout;
    let reader = match vec {
        FormattedVec::U8(vec) => FormattedSoundReader::U8(Box::new(BufferStream { vec, cursor, num_channels: speaker_layout.get_num_channels() })),
        FormattedVec::U16(vec) => FormattedSoundReader::U16(Box::new(BufferStream { vec, cursor, num_channels: speaker_layout.get_num_channels() })),
        FormattedVec::I8(vec) => FormattedSoundReader::I8(Box::new(BufferStream { vec, cursor, num_channels: speaker_layout.get_num_channels() })),
        FormattedVec::I16(vec) => FormattedSoundReader::I16(Box::new(BufferStream { vec, cursor, num_channels: speaker_layout.get_num_channels() })),
        FormattedVec::F32(vec) => FormattedSoundReader::F32(Box::new(BufferStream { vec, cursor, num_channels: speaker_layout.get_num_channels() })),
    };
    FormattedSoundStream { sample_rate, speaker_layout, reader }
}

impl<T: Sample> BufferStream<T> {
    fn read_whole_sound(stream: &mut Box<dyn SoundReader<T>>) -> Vec<T> {
        let mut ret = Vec::new();
        if let Some(len) = stream.estimate_len() {
            if let Ok(len) = len.try_into() {
                ret.resize_with(len, MaybeUninit::uninit);
            }
        }
        let mut amount_read = 0;
        loop {
            let rem_capacity = ret.len() - amount_read;
            let wanted_capacity = 32768; // why not
            if rem_capacity < wanted_capacity {
                let grew = ret.len().checked_mul(2).unwrap().max(32768);
                ret.resize_with(grew, MaybeUninit::uninit);
            }
            let len = stream.read(&mut ret[amount_read..]);
            if len == 0 { break }
            amount_read += len;
        }
        unsafe {
            ret.set_len(amount_read);
            std::mem::transmute(ret)
        }
    }
}

fn read_whole_sound(reader: &mut FormattedSoundReader) -> FormattedVec {
    match reader {
        FormattedSoundReader::U8(x) => FormattedVec::U8(Arc::new(BufferStream::read_whole_sound(x))),
        FormattedSoundReader::U16(x) => FormattedVec::U16(Arc::new(BufferStream::read_whole_sound(x))),
        FormattedSoundReader::I8(x) => FormattedVec::I8(Arc::new(BufferStream::read_whole_sound(x))),
        FormattedSoundReader::I16(x) => FormattedVec::I16(Arc::new(BufferStream::read_whole_sound(x))),
        FormattedSoundReader::F32(x) => FormattedVec::F32(Arc::new(BufferStream::read_whole_sound(x))),
    }
}

impl<T: Sample> SoundReader<T> for BufferStream<T> {
    fn read(&mut self, buf: &mut [MaybeUninit<T>]) -> usize {
        let start = self.cursor;
        let len = (self.vec.len() - self.cursor).min(buf.len());
        let end = start + len;
        // sure hope the compiler figures out that this is a memmove
        buf[..len].iter_mut().zip(&self.vec[start..end]).for_each(|(dst, src)| { dst.write(src.clone()); });
        // https://github.com/rust-lang/rust/issue/79995
        //MaybeUninit::write_slice(&mut buf[..len], &self.vec[start..end]);
        self.cursor += len;
        len
    }
    fn seek(&mut self, in_pos: u64) -> Option<u64> {
        if in_pos > usize::MAX as u64 { return None }
        let pos = (in_pos as usize).checked_mul(self.num_channels)?;
        if pos > self.vec.len() { return None }
        self.cursor = pos;
        Some(in_pos)
    }
    fn attempt_clone(&mut self) -> Option<Box<dyn SoundReader<T>>> {
        Some(Box::new(self.clone()))
    }
    fn estimate_len(&mut self) -> Option<u64> {
        Some((self.vec.len() / self.num_channels) as u64)
    }
    fn skip_coarse(&mut self, count: u64, _buf: &mut [MaybeUninit<T>]) -> u64 {
        let count = count.min(usize::MAX as u64) as usize;
        let old_cursor = self.cursor;
        self.cursor = self.cursor.saturating_add(count).min(self.vec.len());
        (self.cursor - old_cursor) as u64
    }
}

fn load_whole_sound(delegate: &Arc<dyn SoundDelegate>, name: &str) -> (Format, FormattedVec) {
    match delegate.open_file(name) {
        None => {
            delegate.warning(&format!("Unable to open sound file: {:?}", name));
            (Format::default(), FormattedVec::default())
        },
        Some(mut stream) => {
            let format = Format { sample_rate: stream.sample_rate, speaker_layout: stream.speaker_layout };
            let buf = read_whole_sound(&mut stream.reader);
            (format, buf)
        },
    }
}