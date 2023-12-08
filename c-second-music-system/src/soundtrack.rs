use super::*;

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_new() -> *mut Soundtrack {
    Box::into_raw(Box::new(Soundtrack::new()))
}

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_parse_new(
    src: *const c_char,
    src_len: size_t,
    error_out: *mut *mut c_char,
    error_out_len: *mut size_t,
) -> *mut Soundtrack {
    match source_input(src, src_len).and_then(Soundtrack::from_source) {
        Ok(x) => Box::into_raw(Box::new(x)),
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_parse_new_str(
    src: *const c_char,
    error_out: *mut *mut c_char,
    error_out_len: *mut size_t,
) -> *mut Soundtrack {
    match source_input_cstr(src).and_then(Soundtrack::from_source) {
        Ok(x) => Box::into_raw(Box::new(x)),
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_parse(
    soundtrack: *mut Soundtrack,
    src: *const c_char,
    src_len: size_t,
    error_out: *mut *mut c_char,
    error_out_len: *mut size_t,
) -> c_int {
    let soundtrack = unsafe { soundtrack.as_mut() }.unwrap();
    match source_input(src, src_len)
        .and_then(|x| soundtrack.clone().parse_source(x))
        .map(|x| *soundtrack = x)
    {
        Ok(_) => 1,
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_parse_str(
    soundtrack: *mut Soundtrack,
    src: *const c_char,
    error_out: *mut *mut c_char,
    error_out_len: *mut size_t,
) -> c_int {
    let soundtrack = unsafe { soundtrack.as_mut() }.unwrap();
    match source_input_cstr(src)
        .and_then(|x| soundtrack.clone().parse_source(x))
        .map(|x| *soundtrack = x)
    {
        Ok(_) => 1,
        Err(x) => {
            output_error(&x, error_out, error_out_len);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_free(p: *mut Soundtrack) {
    drop(unsafe { Box::from_raw(p) })
}

#[no_mangle]
pub extern "C" fn SMS_Soundtrack_clone(p: *mut Soundtrack) -> *mut Soundtrack {
    let soundtrack = unsafe { p.as_ref() }.unwrap();
    Box::into_raw(Box::new(soundtrack.clone()))
}
