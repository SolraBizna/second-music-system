use second_music_system::*;

use std::{
    ffi::CString,
    mem::{size_of, transmute},
    ptr::null_mut,
};
use libc::{
    c_char, c_int, c_void, size_t,
    malloc, strlen
};
use compact_str::{CompactString, ToCompactString};

mod commander;
mod commands;
mod engine;
mod formatted_sound_stream;
mod sound_delegate;
mod soundtrack;
mod utilities;

const SMS_SPEAKER_LAYOUT_MONO: c_int = 0;
const SMS_SPEAKER_LAYOUT_STEREO: c_int = 1;
const SMS_SPEAKER_LAYOUT_HEADPHONES: c_int = 2;
const SMS_SPEAKER_LAYOUT_QUADRAPHONIC: c_int = 3;
const SMS_SPEAKER_LAYOUT_SURROUND51: c_int = 4;
const SMS_SPEAKER_LAYOUT_SURROUND71: c_int = 5;

const SMS_SOUND_FORMAT_UNSIGNED_8: c_int = 0;
const SMS_SOUND_FORMAT_UNSIGNED_16: c_int = 1;
const SMS_SOUND_FORMAT_SIGNED_8: c_int = 2;
const SMS_SOUND_FORMAT_SIGNED_16: c_int = 3;
const SMS_SOUND_FORMAT_FLOAT_32: c_int = 4;

const SMS_FADE_TYPE_LOGARITHMIC: c_int = 1;
const SMS_FADE_TYPE_LINEAR: c_int = 2;
const SMS_FADE_TYPE_EXPONENTIAL: c_int = 0;

fn source_input(src: *const c_char, src_len: size_t) -> Result<&'static str, String> {
    let slice = unsafe { std::slice::from_raw_parts(transmute(src), src_len) };
    std::str::from_utf8(slice).map_err(|x| {
        format!("soundtrack source code contains invalid UTF-8 at byte {}", x.valid_up_to())
    })
}

fn source_input_cstr(src: *const c_char) -> Result<&'static str, String> {
    let len = unsafe { strlen(src) };
    source_input(src, len)
}

fn input(src: *const c_char, src_len: size_t) -> Result<CompactString, String> {
    let slice = unsafe { std::slice::from_raw_parts(transmute(src), src_len) };
    Ok(String::from_utf8_lossy(slice).to_compact_string())
}

fn input_cstr(src: *const c_char) -> Result<CompactString, String> {
    let len = unsafe { strlen(src) };
    input(src, len)
}

fn output_error(text: &str, error_out: *mut *mut c_char, error_out_len: *mut size_t) {
    unsafe {
        if let Some(error_out_len) = error_out_len.as_mut() {
            *error_out_len = text.len() as size_t;
        }
        if let Some(error_out) = error_out.as_mut() {
            let ptr = malloc(text.len() + 1);
            *error_out = transmute(ptr);
            if !ptr.is_null() {
                let slice = std::slice::from_raw_parts_mut(transmute(ptr), text.len() + 1);
                slice[text.len()] = 0u8;
                slice[..text.len()].copy_from_slice(text.as_bytes());
            }
        }
    }
}

/// Return a PosFloat-clamped float. If clamping was needed, print a warning
/// to stderr. (Ew.)
fn positive(x: f32) -> PosFloat {
    PosFloat::new(x).unwrap_or_else(|e| {
        eprintln!("THIS IS A BUG IN THE PROGRAM USING SMS: {e}");
        PosFloat::ZERO
    })
}

fn speaker_layout_from_int(int: c_int) -> Option<SpeakerLayout> {
    Some(match int {
        SMS_SPEAKER_LAYOUT_MONO => SpeakerLayout::Mono,
        SMS_SPEAKER_LAYOUT_STEREO => SpeakerLayout::Stereo,
        SMS_SPEAKER_LAYOUT_HEADPHONES => SpeakerLayout::Headphones,
        SMS_SPEAKER_LAYOUT_QUADRAPHONIC => SpeakerLayout::Quadraphonic,
        SMS_SPEAKER_LAYOUT_SURROUND51 => SpeakerLayout::Surround51,
        SMS_SPEAKER_LAYOUT_SURROUND71 => SpeakerLayout::Surround71,
        _ => return None,
    })
}

fn speaker_layout_to_int(layout: SpeakerLayout) -> c_int {
    match layout {
        SpeakerLayout::Mono => SMS_SPEAKER_LAYOUT_MONO,
        SpeakerLayout::Stereo => SMS_SPEAKER_LAYOUT_STEREO,
        SpeakerLayout::Headphones => SMS_SPEAKER_LAYOUT_HEADPHONES,
        SpeakerLayout::Quadraphonic => SMS_SPEAKER_LAYOUT_QUADRAPHONIC,
        SpeakerLayout::Surround51 => SMS_SPEAKER_LAYOUT_SURROUND51,
        SpeakerLayout::Surround71 => SMS_SPEAKER_LAYOUT_SURROUND71,
        _ => panic!("SpeakerLayout was expanded, but speaker_layout_to_int was not!"),
    }
}

fn fade_type_from_int(int: c_int) -> Option<FadeType> {
    Some(match int {
        SMS_FADE_TYPE_LOGARITHMIC => FadeType::Logarithmic,
        SMS_FADE_TYPE_LINEAR => FadeType::Linear,
        SMS_FADE_TYPE_EXPONENTIAL => FadeType::Exponential,
        _ => return None,
    })
}
