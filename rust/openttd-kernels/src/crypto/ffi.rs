/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native facade for key buffers, UID hashes and plugin signature checks.
//! Initialized fixed buffers hold 32-byte keys and 64-byte signatures; writable
//! outputs are disjoint from inputs. Byte spans may be null only when empty.
//! File callbacks are synchronous, nonthrowing, and retain no pointers.
//! Panics abort; no exception may cross a Rust frame.
#![allow(unsafe_code)]

use super::{Hash, ed25519_check, public_key};
use std::{ffi::c_void, ptr, slice};
use zeroize::Zeroize;

unsafe fn bytes<'a>(data: *const u8, length: usize) -> &'a [u8] {
    if length == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(data, length) }
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_crypto_public_key(out: *mut u8, secret: *const u8) {
    unsafe {
        ptr::copy_nonoverlapping(public_key(&*secret.cast::<[u8; 32]>()).as_ptr(), out, 32);
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_crypto_wipe(data: *mut u8, length: usize) {
    if length != 0 {
        unsafe {
            slice::from_raw_parts_mut(data, length).zeroize();
        }
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_crypto_check(
    signature: *const u8,
    public: *const u8,
    message: *const u8,
    length: usize,
) -> u8 {
    unsafe {
        u8::from(ed25519_check(
            &*signature.cast::<[u8; 64]>(),
            &*public.cast::<[u8; 32]>(),
            bytes(message, length),
        ))
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_crypto_hash_uid(
    out: *mut u8,
    random: *const u8,
    text: *const u8,
    length: usize,
) {
    let mut hash = Hash::new(16);
    unsafe {
        hash.update(slice::from_raw_parts(random, 32));
        hash.update(bytes(text, length));
        hash.finish(slice::from_raw_parts_mut(out, 16));
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_crypto_hash_file(
    out: *mut u8,
    file: *mut c_void,
    eof: unsafe extern "C" fn(*mut c_void) -> u8,
    read: unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> usize,
) {
    let mut hash = Hash::new(32);
    unsafe {
        while eof(file) == 0 {
            let mut buffer = [0; 1024];
            let size = read(file, buffer.as_mut_ptr(), buffer.len());
            hash.update(&buffer[..size]);
        }
        hash.finish(slice::from_raw_parts_mut(out, 32));
    }
}
