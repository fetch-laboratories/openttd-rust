/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native string storage operation from `string_services.h`.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};

type Append = extern "C" fn(*mut c_void, *const u8, usize);
extern "C" fn unbound(_: *mut c_void, _: *const u8, _: usize) {
    std::process::abort();
}
static APPEND: AtomicPtr<()> = AtomicPtr::new(unbound as *mut ());

/// Bind `RustStringAppend` during native startup, before any string append.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_string_append_bind(append: Append) {
    APPEND.store(append as *mut (), Ordering::Release);
}

/// # Safety
/// `text` is a live native string, serialized for this synchronous append.
/// The input slice remains readable for the call; no pointer is retained.
pub unsafe fn append(text: *mut c_void, data: &[u8]) {
    // SAFETY: Both the initial function and every bound function have this signature.
    let append: Append = unsafe { std::mem::transmute(APPEND.load(Ordering::Acquire)) };
    append(text, data.as_ptr(), data.len());
}
