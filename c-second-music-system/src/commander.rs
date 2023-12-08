use super::*;

#[no_mangle]
unsafe extern "C" fn SMS_Commander_free(p: *mut Commander) {
    drop(unsafe { Box::from_raw(p) })
}

#[no_mangle]
unsafe extern "C" fn SMS_Commander_clone_commander(
    commander: *mut Commander,
) -> *mut Commander {
    if commander.is_null() {
        panic!("SMS_Commander_clone_commander: engine cannot be NULL!");
    }
    let commander = unsafe { commander.as_ref().unwrap() };
    Box::into_raw(Box::new(commander.clone_commander()))
}
