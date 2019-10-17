use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::panic;
use std::path::PathBuf;

#[repr(C)]
#[derive(PartialEq, Debug)]
pub enum FCPResponseStatus {
    // Don't use FCPSuccess, since that complicates description of 'successful' verification.
    FCPNoError = 0,
    FCPUnclassifiedError = 1,
    FCPCallerError = 2,
    FCPReceiverError = 3,
}

/// All FFI responses need to implement this trait in order to be able to use `catch_panic()`
pub trait CodeAndMessage {
    /// Set the status code and error message
    fn set_error(&mut self, code_and_message: (FCPResponseStatus, *const libc::c_char));
}

/// A simple macro to create implementations for the `CodeAndMessage` trait
///
/// The only requirement is that the response has an `status_code: FCPResponseStatus` and
/// `error_msg: *const libc::c_char` field.
#[macro_export]
macro_rules! code_and_message_impl {
    { $response:ty } => {
        impl CodeAndMessage for $response {
            fn set_error(&mut self, (code, message): (FCPResponseStatus, *const libc::c_char)) {
                self.status_code = code;
                self.error_msg = message;
            }
        }
    }
}

// produce a C string from a Rust string
pub fn rust_str_to_c_str<T: Into<String>>(s: T) -> *mut libc::c_char {
    CString::new(s.into()).unwrap().into_raw()
}

// consume a C string-pointer and free its memory
pub unsafe fn free_c_str(ptr: *mut libc::c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

// return a forgotten raw pointer to something of type T
pub fn raw_ptr<T>(thing: T) -> *mut T {
    Box::into_raw(Box::new(thing))
}

// transmutes a C string to a copy-on-write Rust string
pub unsafe fn c_str_to_rust_str<'a>(x: *const libc::c_char) -> Cow<'a, str> {
    if x.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(x).to_string_lossy()
    }
}

// cast from mutable to constant reference
pub unsafe fn cast_const<'a, T>(x: *mut T) -> &'a T {
    assert!(!x.is_null(), "Object argument was null");
    (&(*x))
}

// transmutes a C string to a PathBuf
pub unsafe fn c_str_to_pbuf(x: *const libc::c_char) -> PathBuf {
    PathBuf::from(String::from(c_str_to_rust_str(x)))
}

///// Catch panics and return an error response
pub fn catch_panic_response<F, T>(callback: F) -> *mut T
where
    T: Default + CodeAndMessage,
    F: FnOnce() -> *mut T,
{
    // Using AssertUnwindSafe is code smell. Though catching our panics here is really
    // last resort, so it should be OK.
    let maybe_panic = panic::catch_unwind(panic::AssertUnwindSafe(callback));
    match maybe_panic {
        Ok(return_value) => return_value,
        Err(panic) => {
            let error_msg = match panic.downcast_ref::<&'static str>() {
                Some(message) => message,
                _ => "no unwind information",
            };
            let mut response = T::default();
            let message = CString::new(format!("Rust panic: {}", error_msg))
                .unwrap()
                .into_raw();
            response.set_error((FCPResponseStatus::FCPUnclassifiedError, message));
            raw_ptr(response)
        }
    }
}
