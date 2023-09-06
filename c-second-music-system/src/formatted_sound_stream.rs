use super::*;

use std::marker::PhantomData;

struct ForeignSoundReader<T: Sample> {
    callback_data: *mut c_void,
    read_handler: unsafe extern "C" fn(*mut c_void, *mut c_void, size_t) -> size_t,
    free_handler: Option<unsafe extern "C" fn(*mut c_void)>,
    seek_handler: Option<unsafe extern "C" fn(*mut c_void, u64) -> u64>,
    skip_precise_handler: Option<unsafe extern "C" fn(*mut c_void, u64, *mut c_void, size_t) -> c_int>,
    skip_coarse_handler: Option<unsafe extern "C" fn(*mut c_void, u64, *mut c_void, size_t) -> u64>,
    clone_handler: Option<unsafe extern "C" fn(*mut c_void, f32, c_int) -> *mut FormattedSoundStream>,
    estimate_len_handler: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
    _phantom_data: PhantomData<T>,
}

impl<T: Sample> Drop for ForeignSoundReader<T> {
    fn drop(&mut self) {
        if let Some(free_handler) = self.free_handler {
            unsafe { free_handler(self.callback_data); }
        }
    }
}

unsafe impl<T: Sample> Send for ForeignSoundReader<T> {}

impl<T: Sample> SoundReader<T> for ForeignSoundReader<T> {
    fn read(&mut self, buf: &mut [std::mem::MaybeUninit<T>]) -> usize {
        unsafe { (self.read_handler)(self.callback_data, buf.as_mut_ptr() as *mut c_void, buf.len()) }
    }
    fn seek(&mut self, pos: u64) -> Option<u64> {
        self.seek_handler.and_then(|x| {
            let ret = unsafe { x(self.callback_data, pos) };
            if ret == u64::MAX { Some(ret) }
            else { None }
        })
    }
    fn skip_precise(&mut self, count: u64, buf: &mut [std::mem::MaybeUninit<T>]) -> bool {
        match self.skip_precise_handler {
            None => {
                // this is the default implementation of this method
                let mut rem = count.checked_sub(self.skip_coarse(count, buf))
                    .expect("bug in program's sound delegate: skip_coarse skipped too many samples!");
                while rem > 0 {
                    let amt = (buf.len() as u64).min(rem) as usize;
                    let red = self.read(&mut buf[..amt]);
                    if red == 0 {
                        // premature end? uh oh
                        return false
                    }
                    rem -= red as u64;
                }
                true
            }
            Some(x) => unsafe { x(self.callback_data, count, transmute(buf.as_mut_ptr()), (buf.len() * size_of::<T>()) as size_t) != 0 },
        }
    }
    fn skip_coarse(&mut self, count: u64, buf: &mut [std::mem::MaybeUninit<T>]) -> u64 {
        match self.skip_coarse_handler {
            None => 0,
            Some(x) => unsafe { x(self.callback_data, count, transmute(buf.as_mut_ptr()), (buf.len() * size_of::<T>()) as size_t) },
        }
    }
    fn can_be_cloned(&self) -> bool {
        self.clone_handler.is_some()
    }
    fn attempt_clone(&self, sample_rate: f32, speaker_layout: SpeakerLayout) -> FormattedSoundStream {
        match self.clone_handler {
            None => panic!("attempted to clone a non-cloneable SoundReader"),
            Some(x) => {
                let ret = unsafe { x(self.callback_data, sample_rate, speaker_layout_to_int(speaker_layout)) };
                if ret.is_null() {
                    panic!("PROGRAM BUG: clone_handler cannot return NULL. If your stream is not cloneable, use NULL as your clone_handler instead.");
                }
                unsafe { *Box::from_raw(ret) }
            }
        }
    }
    fn estimate_len(&mut self) -> Option<u64> {
        match self.estimate_len_handler.map(|x| unsafe { x(self.callback_data) }) {
            None => None,
            Some(u64::MAX) => None,
            Some(x) => Some(x)
        }
    }
}

#[no_mangle]
unsafe extern "C" fn SMS_FormattedSoundStream_new(
    callback_data: *mut c_void,
    sample_rate: f32,
    speaker_layout: c_int,
    format: c_int,
    read_handler: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, size_t) -> size_t>,
    free_handler: Option<unsafe extern "C" fn(*mut c_void)>,
    seek_handler: Option<unsafe extern "C" fn(*mut c_void, u64) -> u64>,
    skip_precise_handler: Option<unsafe extern "C" fn(*mut c_void, u64, *mut c_void, size_t) -> c_int>,
    skip_coarse_handler: Option<unsafe extern "C" fn(*mut c_void, u64, *mut c_void, size_t) -> u64>,
    clone_handler: Option<unsafe extern "C" fn(*mut c_void, f32, c_int) -> *mut FormattedSoundStream>,
    estimate_len_handler: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
) -> *mut FormattedSoundStream {
    let read_handler = read_handler.expect("SMS_FormattedSoundStream_new: read_handler was NULL!");
    let speaker_layout = speaker_layout_from_int(speaker_layout)
        .expect("SMS_FormattedSoundStream_new: speaker_layout was not a valid \
                 SMS_SPEAKER_LAYOUT_* constant");
    macro_rules! reader {
        ($enum:ident, $type:ident) => {
            FormattedSoundReader::$enum(Box::new(ForeignSoundReader::<$type> {
                callback_data,
                read_handler,
                free_handler,
                seek_handler,
                skip_precise_handler,
                skip_coarse_handler,
                clone_handler,
                estimate_len_handler,
                _phantom_data: PhantomData,
            }))
        }
    }
    Box::into_raw(Box::new(FormattedSoundStream {
        speaker_layout,
        sample_rate,
        reader: match format {
            SMS_SOUND_FORMAT_UNSIGNED_8 => reader!(U8, u8),
            SMS_SOUND_FORMAT_UNSIGNED_16 => reader!(U16, u16),
            SMS_SOUND_FORMAT_SIGNED_8 => reader!(I8, i8),
            SMS_SOUND_FORMAT_SIGNED_16 => reader!(I16, i16),
            SMS_SOUND_FORMAT_FLOAT_32 => reader!(F32, f32),
            _ => panic!("SMS_FormattedSoundStream_new: format was not a \
                         valid SMS_SOUND_FORMAT_* constant!"),
        },
    }))
}

#[no_mangle]
unsafe extern "C" fn SMS_FormattedSoundStream_free(p: *mut FormattedSoundStream) {
    drop(unsafe { Box::from_raw(p) })
}

