use std::ptr::null;

use super::*;

#[no_mangle]
unsafe extern "C" fn SMS_FlowControlResponse_free(p: *mut query::Response<Option<StringOrNumber>>) {
    drop(unsafe { Box::from_raw(p) })
}

#[no_mangle]
unsafe extern "C" fn SMS_FlowControlResponse_poll(p: *mut query::Response<Option<StringOrNumber>>) -> c_int {
    if p.is_null() {
        panic!("SMS_FlowControlResponse_poll: instance cannot be NULL!");
    }
    let responder = unsafe { p.as_ref().unwrap() };
    responder.poll() as libc::c_int
}

#[no_mangle]
unsafe extern "C" fn SMS_FlowControlResponse_get(
    p: *mut query::Response<Option<StringOrNumber>>,
    context: *mut c_void,
    is_number: Option<extern "C" fn(*mut c_void, c_float)>,
    is_string: Option<extern "C" fn(*mut c_void, *const u8, size_t)>,
    is_unset: Option<extern "C" fn(*mut c_void)>,
    no_response: Option<extern "C" fn(*mut c_void)>,
) {
    if p.is_null() {
        panic!("SMS_FlowControlResponse_poll: instance cannot be NULL!");
    }
    let responder = unsafe { p.as_mut().unwrap() };
    match responder.try_get() {
        Ok(Some(response)) => {
            match response {
                StringOrNumber::Number(num)
                    => is_number.map(|callback| callback(context, *num)),
                StringOrNumber::String(str)
                    => is_string.map(|callback| {
                        let first_char_ptr =
                            if str.is_empty() { null() }
                            else { &str.as_bytes()[0] };
                        callback(context, first_char_ptr, str.len())
                    }),
            }
        },
        Ok(None) => is_unset.map(|callback| callback(context)),
        _ => no_response.map(|callback| callback(context)),
    };
}
