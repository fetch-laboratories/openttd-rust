/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native string storage operation from `string_services.h`.
#![allow(unsafe_code)]

use std::ffi::c_void;

unsafe extern "C" {
    /// # Safety
    /// `text` is a live native string and `data` holds `size` readable bytes.
    pub fn RustStringAppend(text: *mut c_void, data: *const u8, size: usize);
}
