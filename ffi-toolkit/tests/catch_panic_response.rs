use std::ffi::CString;
use std::ptr;

use drop_struct_macro_derive::DropStructMacro;
// `free_c_str` is needed by `DropStructMacro`
// `CodeAndMessage` is the trait implemented by `code_and_message_impl`
use ffi_toolkit::{
    catch_panic_response, code_and_message_impl, free_c_str, raw_ptr, CodeAndMessage,
    FCPResponseStatus,
};

#[repr(C)]
#[derive(DropStructMacro)]
pub struct BasicResponse {
    pub status_code: FCPResponseStatus,
    pub error_msg: *mut libc::c_char,
    pub is_valid: bool,
}

impl Default for BasicResponse {
    fn default() -> Self {
        BasicResponse {
            status_code: FCPResponseStatus::FCPNoError,
            error_msg: ptr::null_mut(),
            is_valid: false,
        }
    }
}

code_and_message_impl!(BasicResponse);

unsafe extern "C" fn fn_does_not_panic() -> *mut BasicResponse {
    let mut response = BasicResponse::default();
    response.is_valid = true;
    raw_ptr(response)
}

unsafe extern "C" fn fn_does_not_panic_with_catch_panic() -> *mut BasicResponse {
    catch_panic_response(|| {
        let mut response = BasicResponse::default();
        response.is_valid = true;
        raw_ptr(response)
    })
}

unsafe extern "C" fn fn_does_panic_with_catch_panic() -> *mut BasicResponse {
    catch_panic_response(|| panic!("I do panic"))
}

/// Nothing special in this test, this is just there to make sure things work the same with
/// or without a `catch_panic()` closure.
#[test]
fn does_not_panic() {
    unsafe {
        let response = fn_does_not_panic();
        assert!((*response).is_valid);
        assert_eq!((*response).status_code, FCPResponseStatus::FCPNoError);
        assert!((*response).error_msg.is_null());
    }
}

/// This test should return the same result as the `does_not_panic()` test.
#[test]
fn does_not_panic_with_catch_panic_response() {
    unsafe {
        let response = fn_does_not_panic_with_catch_panic();
        assert!((*response).is_valid);
        assert_eq!((*response).status_code, FCPResponseStatus::FCPNoError);
        assert!((*response).error_msg.is_null());
    }
}

// `fn does_panic` isn't a test case as it would abort the test suite with a
// `(signal: 4, SIGILL: illegal instruction)`

/// Testing if catching panics actually works.
#[test]
fn does_panic_with_catch_panic_response() {
    unsafe {
        let response = fn_does_panic_with_catch_panic();
        assert!(!(*response).is_valid);
        assert_eq!(
            (*response).status_code,
            FCPResponseStatus::FCPUnclassifiedError
        );
        let error_message = CString::from_raw((*response).error_msg as *mut _)
            .into_string()
            .unwrap();
        assert_eq!(error_message, "Rust panic: I do panic");
    }
}
