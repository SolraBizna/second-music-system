use super::*;

use std::sync::Arc;

struct ForeignSoundDelegate {
    callback_data: *mut c_void,
    file_open_handler: unsafe extern "C" fn(
        *mut c_void,
        *const c_char,
    ) -> *mut FormattedSoundStream,
    warning_handler: Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
    free_handler: Option<unsafe extern "C" fn(*mut c_void)>,
}
unsafe impl Send for ForeignSoundDelegate {}
unsafe impl Sync for ForeignSoundDelegate {}

impl Drop for ForeignSoundDelegate {
    fn drop(&mut self) {
        if let Some(free_handler) = self.free_handler.take() {
            unsafe { free_handler(self.callback_data) };
        }
    }
}

impl SoundDelegate for ForeignSoundDelegate {
    fn open_file(&self, name: &str) -> Option<FormattedSoundStream> {
        let name = CString::new(name).unwrap();
        unsafe {
            let result =
                (self.file_open_handler)(self.callback_data, name.as_ptr());
            if result.is_null() {
                None
            } else {
                Some(*Box::from_raw(result))
            }
        }
    }
    fn warning(&self, message: &str) {
        match self.warning_handler {
            Some(warning_handler) => {
                let message = CString::new(message).unwrap();
                unsafe {
                    (warning_handler)(self.callback_data, message.as_ptr());
                }
            }
            None => {
                eprintln!("SMS warning: {}", message);
            }
        }
    }
}

#[no_mangle]
extern "C" fn SMS_SoundDelegate_new(
    callback_data: *mut c_void,
    file_open_handler: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
        ) -> *mut FormattedSoundStream,
    >,
    warning_handler: Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
    free_handler: Option<unsafe extern "C" fn(*mut c_void)>,
) -> *mut Arc<dyn SoundDelegate> {
    let file_open_handler = file_open_handler
        .expect("SMS_SoundDelegate_new: file_open_handler cannot be NULL!");
    Box::into_raw(Box::new(Arc::new(ForeignSoundDelegate {
        callback_data,
        file_open_handler,
        warning_handler,
        free_handler,
    })))
}

#[no_mangle]
extern "C" fn SMS_SoundDelegate_free(p: *mut Arc<dyn SoundDelegate>) {
    drop(unsafe { Box::from_raw(p) })
}
