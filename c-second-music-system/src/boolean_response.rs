use super::*;

#[no_mangle]
unsafe extern "C" fn SMS_BooleanResponse_free(p: *mut query::Response<bool>) {
    drop(unsafe { Box::from_raw(p) })
}

#[no_mangle]
unsafe extern "C" fn SMS_BooleanResponse_poll(
    p: *mut query::Response<bool>,
) -> c_int {
    if p.is_null() {
        panic!("SMS_BooleanResponse_poll: instance cannot be NULL!");
    }
    let responder = unsafe { p.as_ref().unwrap() };
    responder.poll() as libc::c_int
}

#[no_mangle]
unsafe extern "C" fn SMS_BooleanResponse_get(
    p: *mut query::Response<bool>,
) -> c_int {
    if p.is_null() {
        panic!("SMS_BooleanResponse_poll: instance cannot be NULL!");
    }
    let responder = unsafe { p.as_mut().unwrap() };
    match responder.try_get() {
        Ok(response) => *response as libc::c_int,
        _ => -1,
    }
}
