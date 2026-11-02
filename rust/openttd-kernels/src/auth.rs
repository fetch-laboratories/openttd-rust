/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Authentication session and encryption-stream owners. The pointer contract is
//! in `auth_ffi.h`. Calls are serialized; no pointer is retained past its owner.
//! Panics abort, and no C++ exception may cross an entry.
#![allow(unsafe_code)]

use crate::crypto::{self, Aead};
use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::ptr;
use zeroize::{Zeroize, Zeroizing};

unsafe fn bytes<'a>(data: *const u8, length: usize) -> &'a [u8] {
    if length == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, length) }
    }
}

unsafe fn bytes_mut<'a>(data: *mut u8, length: usize) -> &'a mut [u8] {
    if length == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(data, length) }
    }
}

unsafe fn allocate<T>() -> *mut T {
    let layout = Layout::new::<T>();
    // SAFETY: Only raw storage is returned; caller initializes fields before references.
    let result: *mut T = unsafe { alloc_zeroed(layout) }.cast();
    if result.is_null() {
        handle_alloc_error(layout);
    }
    result
}

unsafe fn free<T>(owner: *mut T) {
    // SAFETY: Matching Rust allocation, dropped once by its C++ owner.
    unsafe {
        ptr::drop_in_place(owner);
        dealloc(owner.cast(), Layout::new::<T>());
    }
}

#[allow(clippy::too_many_arguments)] // Preserve the fixed KDF inputs without a carrier.
unsafe fn derive(
    keys: *mut u8,
    peer: *const u8,
    side: u8,
    secret: *const u8,
    public: *const u8,
    extra: *const u8,
    length: usize,
) -> u8 {
    let mut output = Zeroizing::new([0; 64]);
    // Extra may alias old keys, so finish all reads before replacing that storage.
    let valid = unsafe {
        crypto::derive(
            &mut output,
            &*secret.cast::<[u8; 32]>(),
            &*public.cast::<[u8; 32]>(),
            &*peer.cast::<[u8; 32]>(),
            side == 1,
            bytes(extra, length),
        )
    };
    if valid {
        unsafe {
            ptr::copy_nonoverlapping(output.as_ptr(), keys, 64);
        }
    }
    u8::from(valid)
}

pub(crate) struct Keys {
    keys: [u8; 64],
}
pub(crate) struct Session {
    secret: [u8; 32],
    public: [u8; 32],
    exchange_nonce: [u8; 24],
    keys: [u8; 64],
    peer: [u8; 32],
    encryption_nonce: [u8; 24],
}
pub(crate) struct Stream {
    context: Aead,
}

impl Drop for Keys {
    fn drop(&mut self) {
        self.keys.zeroize();
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.encryption_nonce.zeroize();
        self.peer.zeroize();
        self.keys.zeroize();
        self.exchange_nonce.zeroize();
        self.public.zeroize();
        self.secret.zeroize();
    }
}

// All C exports below share the module/header safety contract. Their visibility
// is crate-local; the exported symbols, not Rust types, are the public interface.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_new() -> *mut Keys {
    // SAFETY: Zeroed byte arrays form valid key storage.
    unsafe { allocate::<Keys>() }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_clone(src: *const Keys) -> *mut Keys {
    // SAFETY: Direct initialized heap-to-heap copy; no secret aggregate temporary.
    unsafe {
        let dst = allocate::<Keys>();
        ptr::copy_nonoverlapping(src, dst, 1);
        dst
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_assign(dst: *mut Keys, src: *const Keys) {
    // SAFETY: Independent owners or self-assignment. Original assignment overwrote without wiping first.
    unsafe {
        if !ptr::eq(dst, src) {
            ptr::copy_nonoverlapping(src, dst, 1);
        }
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_destroy(owner: *mut Keys) {
    unsafe {
        free(owner);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_data(
    owner: *const Keys,
    side: u8,
) -> *const u8 {
    assert!(side <= 1);
    unsafe {
        ptr::addr_of!((*owner).keys)
            .cast::<u8>()
            .add(usize::from(side) * 32)
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_exchange(
    owner: *mut Keys,
    peer: *const u8,
    side: u8,
    secret: *const u8,
    public: *const u8,
    extra: *const u8,
    length: usize,
) -> u8 {
    unsafe {
        derive(
            ptr::addr_of_mut!((*owner).keys).cast(),
            peer,
            side,
            secret,
            public,
            extra,
            length,
        )
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_new(secret: *const u8) -> *mut Session {
    unsafe {
        let owner = allocate::<Session>();
        ptr::copy_nonoverlapping(secret, ptr::addr_of_mut!((*owner).secret).cast(), 32);
        ptr::addr_of_mut!((*owner).public).write(crypto::public_key(&(*owner).secret));
        owner
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_clone(
    src: *const Session,
) -> *mut Session {
    unsafe {
        let dst = allocate::<Session>();
        ptr::copy_nonoverlapping(src, dst, 1);
        dst
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_assign(
    dst: *mut Session,
    src: *const Session,
) {
    unsafe {
        if !ptr::eq(dst, src) {
            ptr::copy_nonoverlapping(src, dst, 1);
        }
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_destroy(owner: *mut Session) {
    unsafe {
        free(owner);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_view(
    owner: *const Session,
    field: u8,
) -> *const u8 {
    unsafe {
        match field {
            0 => ptr::addr_of!((*owner).public).cast(),
            1 => ptr::addr_of!((*owner).peer).cast(),
            2 => ptr::addr_of!((*owner).exchange_nonce).cast(),
            3 => ptr::addr_of!((*owner).encryption_nonce).cast(),
            4 => ptr::addr_of!((*owner).keys).cast(),
            5 => ptr::addr_of!((*owner).keys).cast::<u8>().add(32),
            _ => std::process::abort(),
        }
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_mut_view(
    owner: *mut Session,
    field: u8,
) -> *mut u8 {
    assert!((1..=3).contains(&field));
    unsafe { openttd_rust_auth_session_view(owner, field).cast_mut() }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_exchange(
    owner: *mut Session,
    side: u8,
    extra: *const u8,
    length: usize,
) -> u8 {
    unsafe {
        derive(
            ptr::addr_of_mut!((*owner).keys).cast(),
            ptr::addr_of!((*owner).peer).cast(),
            side,
            ptr::addr_of!((*owner).secret).cast(),
            ptr::addr_of!((*owner).public).cast(),
            extra,
            length,
        )
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_encrypt_response(
    owner: *const Session,
    message: *mut u8,
    mac: *mut u8,
) {
    unsafe {
        let session = &*owner;
        let mut context = Aead::new(
            session.keys[..32].try_into().unwrap(),
            &session.exchange_nonce,
        );
        let tag = context.write(std::slice::from_raw_parts_mut(message, 8), &session.public);
        ptr::copy_nonoverlapping(tag.as_ptr(), mac, 16);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_decrypt_response(
    owner: *const Session,
    message: *mut u8,
    mac: *const u8,
) -> u8 {
    unsafe {
        let session = &*owner;
        u8::from(
            Aead::new(
                session.keys[..32].try_into().unwrap(),
                &session.exchange_nonce,
            )
            .read(
                std::slice::from_raw_parts_mut(message, 8),
                &*mac.cast::<[u8; 16]>(),
                &session.peer,
            ),
        )
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_new(
    owner: *const Session,
    side: u8,
) -> *mut Stream {
    assert!(side <= 1);
    unsafe {
        let dst = allocate::<Stream>();
        let session = &*owner;
        ptr::addr_of_mut!((*dst).context).write(Aead::new(
            session.keys[usize::from(side) * 32..usize::from(side) * 32 + 32]
                .try_into()
                .unwrap(),
            &session.encryption_nonce,
        ));
        dst
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_clone(src: *const Stream) -> *mut Stream {
    unsafe {
        let dst = allocate::<Stream>();
        ptr::addr_of_mut!((*dst).context).write((*src).context.clone());
        dst
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_assign(
    dst: *mut Stream,
    src: *const Stream,
) {
    unsafe {
        if !ptr::eq(dst, src) {
            (*dst).context.clone_from(&(*src).context);
        }
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_destroy(owner: *mut Stream) {
    unsafe {
        free(owner);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_encrypt(
    owner: *mut Stream,
    mac: *mut u8,
    message: *mut u8,
    length: usize,
) {
    assert!(isize::try_from(length).is_ok());
    unsafe {
        let tag = (*owner).context.write(bytes_mut(message, length), &[]);
        ptr::copy_nonoverlapping(tag.as_ptr(), mac, 16);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_decrypt(
    owner: *mut Stream,
    mac: *const u8,
    message: *mut u8,
    length: usize,
) -> u8 {
    assert!(isize::try_from(length).is_ok());
    unsafe {
        u8::from(
            (*owner)
                .context
                .read(bytes_mut(message, length), &*mac.cast::<[u8; 16]>(), &[]),
        )
    }
}
