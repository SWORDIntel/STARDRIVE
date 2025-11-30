#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ptr;

// Include auto-generated EVDI bindings once here
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Define EVDI_INVALID_HANDLE (bindgen doesn't handle C macros)
pub const EVDI_INVALID_HANDLE: evdi_handle = ptr::null_mut();

// Wrapper to make evdi_handle Send (EVDI is thread-safe in practice)
pub struct SendEvdiHandle(pub evdi_handle);
unsafe impl Send for SendEvdiHandle {}
unsafe impl Sync for SendEvdiHandle {}
