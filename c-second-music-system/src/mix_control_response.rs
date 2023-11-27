use super::*;

#[no_mangle]
unsafe extern "C" fn SMS_MixControlResponse_free(p: *mut query::Response<Option<PosFloat>>) {
    drop(unsafe { Box::from_raw(p) })
}

#[no_mangle]
unsafe extern "C" fn SMS_MixControlResponse_poll(p: *mut query::Response<Option<PosFloat>>) -> c_int {
    if p.is_null() {
        panic!("SMS_MixControlResponse_poll: instance cannot be NULL!");
    }
    let responder = unsafe { p.as_ref().unwrap() };
    responder.poll() as libc::c_int
}

#[no_mangle]
unsafe extern "C" fn SMS_MixControlResponse_get(p: *mut query::Response<Option<PosFloat>>) -> c_float {
    if p.is_null() {
        panic!("SMS_MixControlResponse_poll: instance cannot be NULL!");
    }
    let responder = unsafe { p.as_mut().unwrap() };
    match responder.try_get() {
        Ok(Some(response)) => **response,
        Ok(None) => -1.0,
        _ => c_float::NEG_INFINITY,
    }
}
