use super::*;

#[no_mangle]
pub extern "C" fn SMS_SpeakerLayout_get_num_channels(layout: c_int) -> c_int {
    speaker_layout_from_int(layout).map(|x| x.get_num_channels() as c_int).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn SMS_get_version_string() -> *const c_char {
    unsafe {
        std::mem::transmute(concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr())
    }
}

#[no_mangle]
pub extern "C" fn SMS_get_version_number() -> u32 {
    let major = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
    let minor = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
    let patch = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();
    u32::from_be_bytes([0, major, minor, patch])
}

