/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Native mutation of Rust-owned name and encoded-string bytes. Views expire on
//! assignment; accesses are serialized by the game-state mutex.
#![allow(unsafe_code)]
use crate::industrytype::NativeVec;
use crate::services::ByteSpan;

/// Replace the bytes, permitting the input to alias the existing allocation.
///
/// # Safety
/// `bytes` is live Rust-owned storage and `text` is readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_owned_bytes_assign(
    bytes: *mut NativeVec<u8>,
    text: ByteSpan,
) {
    let value = if text.size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(text.data, text.size).to_vec() }
    };
    unsafe { *bytes = NativeVec::from_vec(value) };
}
