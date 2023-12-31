use super::*;

use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use crossbeam::channel;

#[derive(Debug, Clone)]
struct Format {
    sample_rate: PosFloat,
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
            WeakFormattedVec::U8(x) => x.upgrade().map(FormattedVec::U8),
            WeakFormattedVec::U16(x) => x.upgrade().map(FormattedVec::U16),
            WeakFormattedVec::I8(x) => x.upgrade().map(FormattedVec::I8),
            WeakFormattedVec::I16(x) => x.upgrade().map(FormattedVec::I16),
            WeakFormattedVec::F32(x) => x.upgrade().map(FormattedVec::F32),
        }
    }
}

impl Format {
    fn default() -> Format {
        Format {
            sample_rate: PosFloat::new_clamped(44100.0),
            speaker_layout: SpeakerLayout::Mono,
        }
    }
}

enum CachedSound {
    /// A sound that hasn't been loaded yet, but whose loading has been
    /// requested.
    Loading {
        rx: channel::Receiver<(Format, FormattedVec)>,
        load_count: u32,
    },
    /// A sound that has been loaded, and is currently being actively cached.
    Loaded {
        format: Format,
        vec: FormattedVec,
        load_count: u32,
    },
    /// A sound that was previously loaded, but whose load count has gone to
    /// zero, and which will get purged if no active playback requires it.
    Unloaded {
        format: Format,
        vec: WeakFormattedVec,
    },
}

impl CachedSound {
    /// If we are a `LoadingSound`, check if we should actually become a
    /// `LoadedSound` instead. If so, mutate.
    fn check_loading(
        &mut self,
        delegate: &Arc<dyn SoundDelegate>,
        name: &str,
    ) {
        if let CachedSound::Loading { load_count, rx } = self {
            match rx.try_recv() {
                Ok((format, vec)) if *load_count > 0 => {
                    *self = CachedSound::Loaded {
                        load_count: *load_count,
                        format,
                        vec,
                    };
                }
                Ok(_) if *load_count == 0 => {
                    // Great, thanks for loading! But we don't want you anymore
                    // (it makes no sense to put it straight in as a weak vec,
                    // because it would immediately be freed)
                    *self = CachedSound::Unloaded {
                        format: Format::default(),
                        vec: WeakFormattedVec::default(),
                    }
                }
                Err(channel::TryRecvError::Empty) => {
                    // nothing to do right now
                }
                _ => {
                    delegate.warning(&format!(
                        "Background loading sound {:?} failed",
                        name
                    ));
                    *self = CachedSound::Loaded {
                        load_count: *load_count,
                        format: Format::default(),
                        vec: FormattedVec::default(),
                    };
                }
            }
        }
    }
}

/// Manages cached *sounds*. We load them completely, ahead of time. We use a
/// lot of memory, but our returned streams are very simple to decode.
pub struct BufferMan<Runtime: TaskRuntime> {
    delegate: Arc<dyn SoundDelegate>,
    sounds: HashMap<String, CachedSound>,
    _marker: PhantomData<Runtime>,
}

impl<Runtime: TaskRuntime> SoundManSubtype<Runtime> for BufferMan<Runtime> {
    fn load(
        &mut self,
        sound: &str,
        _start: PosFloat,
        loading_rt: &Arc<Runtime>,
    ) {
        if let Some(ent) = self.sounds.get_mut(sound) {
            ent.check_loading(&self.delegate, sound);
            match ent {
                CachedSound::Loading { load_count, .. }
                | CachedSound::Loaded { load_count, .. } => {
                    *load_count += 1;
                    return;
                }
                CachedSound::Unloaded { format, vec } => {
                    if let Some(vec) = vec.upgrade() {
                        *ent = CachedSound::Loaded {
                            load_count: 1,
                            format: format.clone(),
                            vec,
                        };
                        return;
                    }
                }
            }
        }
        // if we got here, the sound has never been loaded, or it was unloaded
        // and collected
        let (result_tx, result_rx) = channel::bounded(1);
        let delegate = self.delegate.clone();
        let sound = sound.to_string();
        let sound_clone = sound.clone();
        loading_rt.spawn_task(TaskType::BufferLoad, async move {
            let _ = result_tx.send(load_whole_sound(&delegate, &sound_clone));
        });
        self.sounds.insert(
            sound.to_string(),
            CachedSound::Loading {
                load_count: 1,
                rx: result_rx,
            },
        );
    }
    fn unload(&mut self, sound: &str, _start: PosFloat) -> bool {
        match self.sounds.get_mut(sound) {
            None | Some(CachedSound::Unloaded { .. }) => {
                self.delegate.warning(&format!(
                    "unbalanced unload of sound {:?} (THIS IS A BUG IN SMS!)",
                    sound
                ));
                true
            }
            Some(x) => match x {
                CachedSound::Loading { load_count, .. } => {
                    if *load_count > 0 {
                        *load_count -= 1;
                    } else {
                        self.delegate.warning(&format!("unbalanced unload of sound {:?} (THIS IS A BUG IN SMS!)", sound));
                    }
                    *load_count == 0
                }
                CachedSound::Loaded {
                    load_count,
                    format,
                    vec,
                } => {
                    if *load_count > 1 {
                        *load_count -= 1;
                        false
                    } else {
                        *x = CachedSound::Unloaded {
                            format: format.clone(),
                            vec: vec.downgrade(),
                        };
                        true
                    }
                }
                CachedSound::Unloaded { .. } => unreachable!(),
            },
        }
    }
    fn unload_all(&mut self) {
        self.sounds.clear();
    }
    fn is_ready(&mut self, sound: &str, _start: PosFloat) -> bool {
        if let Some(x) = self.sounds.get_mut(sound) {
            x.check_loading(&self.delegate, sound);
            if let CachedSound::Loaded { .. } = x {
                return true;
            }
        }
        false
    }
    fn get_sound(
        &mut self,
        sound: &str,
        start: PosFloat,
        end: &OnceLock<PosFloat>,
    ) -> Option<FormattedSoundStream> {
        self.sounds.get_mut(sound).and_then(|s| {
            s.check_loading(&self.delegate, sound);
            match s {
                CachedSound::Loaded { format, vec, .. } => {
                    Some(new_buffer_stream(format, vec.clone(), start, end))
                }
                CachedSound::Unloaded { format, vec } => {
                    vec.upgrade().map(|vec| {
                        new_buffer_stream(format, vec.clone(), start, end)
                    })
                }
                CachedSound::Loading { .. } => None,
            }
        })
    }
}

impl<Runtime: TaskRuntime> BufferMan<Runtime> {
    pub fn new(delegate: Arc<dyn SoundDelegate>) -> BufferMan<Runtime> {
        BufferMan {
            delegate,
            sounds: HashMap::new(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
struct BufferStream<T: Sample> {
    vec: Arc<Vec<T>>,
    cursor: usize,
    end: usize,
    num_channels: usize,
}

fn new_buffer_stream(
    format: &Format,
    vec: FormattedVec,
    start: PosFloat,
    end: &OnceLock<PosFloat>,
) -> FormattedSoundStream {
    let cursor = start
        .seconds_to_samples(format.sample_rate, format.speaker_layout)
        .min(vec.len() as u64) as usize;
    let end = match end.get() {
        Some(end) => {
            // End point is specified or cached.
            end.seconds_to_samples(format.sample_rate, format.speaker_layout)
                .min(vec.len() as u64) as usize
        }
        None => {
            // End point is not specified. Fill it in.
            let new_end = PosFloat::new(
                (vec.len() / format.speaker_layout.get_num_channels()) as f32,
            )
            .expect("SMS bug: Buffer too big, couldn't make float!")
                / format.sample_rate;
            end.get_or_init(|| new_end);
            vec.len()
        }
    };
    let sample_rate = format.sample_rate;
    let speaker_layout = format.speaker_layout;
    let reader = match vec {
        FormattedVec::U8(vec) => {
            FormattedSoundReader::U8(Box::new(BufferStream {
                vec,
                cursor,
                end,
                num_channels: speaker_layout.get_num_channels(),
            }))
        }
        FormattedVec::U16(vec) => {
            FormattedSoundReader::U16(Box::new(BufferStream {
                vec,
                cursor,
                end,
                num_channels: speaker_layout.get_num_channels(),
            }))
        }
        FormattedVec::I8(vec) => {
            FormattedSoundReader::I8(Box::new(BufferStream {
                vec,
                cursor,
                end,
                num_channels: speaker_layout.get_num_channels(),
            }))
        }
        FormattedVec::I16(vec) => {
            FormattedSoundReader::I16(Box::new(BufferStream {
                vec,
                cursor,
                end,
                num_channels: speaker_layout.get_num_channels(),
            }))
        }
        FormattedVec::F32(vec) => {
            FormattedSoundReader::F32(Box::new(BufferStream {
                vec,
                cursor,
                end,
                num_channels: speaker_layout.get_num_channels(),
            }))
        }
    };
    FormattedSoundStream {
        sample_rate,
        speaker_layout,
        reader,
    }
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
            if len == 0 {
                break;
            }
            amount_read += len;
        }
        unsafe {
            ret.set_len(amount_read);
            ret.shrink_to_fit();
            std::mem::transmute(ret)
        }
    }
}

fn read_whole_sound(reader: &mut FormattedSoundReader) -> FormattedVec {
    match reader {
        FormattedSoundReader::U8(x) => {
            FormattedVec::U8(Arc::new(BufferStream::read_whole_sound(x)))
        }
        FormattedSoundReader::U16(x) => {
            FormattedVec::U16(Arc::new(BufferStream::read_whole_sound(x)))
        }
        FormattedSoundReader::I8(x) => {
            FormattedVec::I8(Arc::new(BufferStream::read_whole_sound(x)))
        }
        FormattedSoundReader::I16(x) => {
            FormattedVec::I16(Arc::new(BufferStream::read_whole_sound(x)))
        }
        FormattedSoundReader::F32(x) => {
            FormattedVec::F32(Arc::new(BufferStream::read_whole_sound(x)))
        }
    }
}

impl<T: Sample> SoundReader<T> for BufferStream<T> {
    fn read(&mut self, buf: &mut [MaybeUninit<T>]) -> usize {
        debug_assert_eq!(buf.len() % self.num_channels, 0);
        let start = self.cursor;
        let len = (self.end - self.cursor).min(buf.len());
        let end = start + len;
        // sure hope the compiler figures out that this is a memmove
        buf[..len].iter_mut().zip(&self.vec[start..end]).for_each(
            |(dst, src)| {
                dst.write(*src);
            },
        );
        // https://github.com/rust-lang/rust/issue/79995
        //MaybeUninit::write_slice(&mut buf[..len], &self.vec[start..end]);
        self.cursor += len;
        len
    }
    fn seek(&mut self, _in_pos: u64) -> Option<u64> {
        panic!("SMS logic error: seeking a BufferStream");
    }
    fn attempt_clone(
        &self,
        sample_rate: PosFloat,
        speaker_layout: SpeakerLayout,
    ) -> FormattedSoundStream {
        FormattedSoundStream {
            sample_rate,
            speaker_layout,
            reader: (Box::new(self.clone()) as Box<dyn SoundReader<T>>).into(),
        }
    }
    fn estimate_len(&mut self) -> Option<u64> {
        panic!("SMS logic error: len estimation on a BufferStream");
    }
    fn skip_coarse(&mut self, count: u64, _buf: &mut [MaybeUninit<T>]) -> u64 {
        let count = count.min(usize::MAX as u64) as usize;
        let old_cursor = self.cursor;
        self.cursor = self.cursor.saturating_add(count).min(self.vec.len());
        (self.cursor - old_cursor) as u64
    }
}

fn load_whole_sound(
    delegate: &Arc<dyn SoundDelegate>,
    name: &str,
) -> (Format, FormattedVec) {
    match delegate.open_file(name) {
        None => {
            delegate
                .warning(&format!("Unable to open sound file: {:?}", name));
            (Format::default(), FormattedVec::default())
        }
        Some(mut stream) => {
            let format = Format {
                sample_rate: stream.sample_rate,
                speaker_layout: stream.speaker_layout,
            };
            let buf = read_whole_sound(&mut stream.reader);
            (format, buf)
        }
    }
}
