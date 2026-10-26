/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `LoadTranslationTable` policy for cargo, rail, road and tram labels.
//! Native vector leaves publish each completed read; the native caller rethrows
//! read failure only after Rust has returned. No whole-property prevalidation.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader, Services as ReaderServices};
use crate::newgrf_internal::{CIR_INVALID_ID, CIR_SUCCESS};
use std::ffi::{c_char, c_void};
#[repr(C)]
pub struct Services {
    reset: extern "C" fn(*mut c_void, u32),
    append: extern "C" fn(*mut c_void, u32),
    copy: extern "C" fn(*mut c_void, *mut c_void),
    override_file: extern "C" fn() -> *mut c_void,
    invalid_first: extern "C" fn(*const c_char, usize),
    log_override: extern "C" fn(*const c_char, usize, *mut c_void),
}
fn load(
    s: &Services,
    reader: &mut Reader<'_>,
    file: *mut c_void,
    first: u32,
    last: u32,
    name: *const c_char,
    len: usize,
) -> Result<i32, ReadError> {
    if first != 0 {
        (s.invalid_first)(name, len);
        return Ok(CIR_INVALID_ID);
    }
    (s.reset)(file, last);
    for _ in first..last {
        let label = reader.dword()?.swap_bytes();
        (s.append)(file, label);
    }
    let target = (s.override_file)();
    if !target.is_null() {
        (s.log_override)(name, len, target);
        (s.copy)(target, file);
    }
    Ok(CIR_SUCCESS)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_load_translation(
    s: *const Services,
    reads: *const ReaderServices,
    file: *mut c_void,
    first: u32,
    last: u32,
    native: *mut ByteReader,
    name: *const c_char,
    len: usize,
) -> i32 {
    let mut reader = unsafe { Reader::new(native, &*reads) };
    load(unsafe { &*s }, &mut reader, file, first, last, name, len).unwrap_or(-1)
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTranslationServices" ["engine"] => Services { reset, append, copy, override_file, invalid_first, log_override };
}
