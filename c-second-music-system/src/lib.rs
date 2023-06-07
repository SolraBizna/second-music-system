use second_music_system::*;

use std::{
    mem::transmute,
    ptr::null_mut,
};
use libc::{
    c_char, c_int, size_t,
    malloc, strlen
};

fn input(src: *const c_char, src_len: size_t) -> Result<&'static str, String> {
    let slice = unsafe { std::slice::from_raw_parts(transmute(src), src_len as usize) };
    std::str::from_utf8(slice).map_err(|x| {
        format!("source code contains invalid UTF-8 at byte {}", x.valid_up_to())
    })
}

fn input_str(src: *const c_char) -> Result<&'static str, String> {
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

#[no_mangle]
pub extern "C" fn SMS_new_soundtrack() -> *mut Soundtrack {
    Box::into_raw(Box::new(Soundtrack::new()))
}

#[no_mangle]
pub extern "C" fn SMS_parse_new_soundtrack(
    src: *const c_char, src_len: size_t,
    error_out: *mut *mut c_char, error_out_len: *mut size_t
) -> *mut Soundtrack {
    match input(src, src_len).and_then(Soundtrack::from_source) {
        Ok(x) => Box::into_raw(Box::new(x)),
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_parse_new_soundtrack_str(
    src: *const c_char,
    error_out: *mut *mut c_char, error_out_len: *mut size_t
) -> *mut Soundtrack {
    match input_str(src).and_then(Soundtrack::from_source) {
        Ok(x) => Box::into_raw(Box::new(x)),
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_parse_soundtrack(
    soundtrack: *mut Soundtrack,
    src: *const c_char, src_len: size_t,
    error_out: *mut *mut c_char, error_out_len: *mut size_t
) -> c_int {
    let soundtrack = unsafe { soundtrack.as_mut() }.unwrap();
    match input(src, src_len).and_then(|x| {
        soundtrack.clone().parse_source(x)
    }).map(|x| *soundtrack = x) {
        Ok(_) => 1,
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_parse_soundtrack_str(
    soundtrack: *mut Soundtrack,
    src: *const c_char,
    error_out: *mut *mut c_char, error_out_len: *mut size_t
) -> c_int {
    let soundtrack = unsafe { soundtrack.as_mut() }.unwrap();
    match input_str(src).and_then(|x| {
        soundtrack.clone().parse_source(x)
    }).map(|x| *soundtrack = x) {
        Ok(_) => 1,
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_free_soundtrack(p: *mut Soundtrack) {
    drop(unsafe { Box::from_raw(p) })
}