/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Stable authentication owners. Cryptographic algorithms remain bundled host primitives.
//!
//! Every exported call has the common pointer/ownership contract in `auth_ffi.h`:
//! live initialized fixed buffers, exclusive owner mutation, synchronous nonthrowing
//! callbacks, disjoint MAC/message regions, and borrowed lengths <= `isize::MAX`.
//! Owners and secret buffers are initialized directly in their final allocation;
//! cloning copies heap-to-heap, never through a secret-bearing Rust aggregate.
//! No Packet, RNG, application logger or C++ allocation callback runs under Rust.
#![allow(unsafe_code)] // This module implements the documented raw-pointer owner ABI.

use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::ffi::c_void;
use std::ptr::{self, NonNull};

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct Primitives {
    version: u32,
    reserved: u32,
    aead_size: usize,
    aead_alignment: usize,
    hash_size: usize,
    hash_alignment: usize,
    wipe: unsafe extern "C" fn(*mut c_void, usize),
    x25519: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    public_key: unsafe extern "C" fn(*mut u8, *const u8),
    hash_init: unsafe extern "C" fn(*mut c_void, usize),
    hash_update: unsafe extern "C" fn(*mut c_void, *const u8, usize),
    hash_final: unsafe extern "C" fn(*mut c_void, *mut u8),
    aead_init: unsafe extern "C" fn(*mut c_void, *const u8, *const u8),
    aead_copy: unsafe extern "C" fn(*mut c_void, *const c_void),
    aead_write:
        unsafe extern "C" fn(*mut c_void, *mut u8, *mut u8, *const u8, usize, *const u8, usize),
    aead_read: unsafe extern "C" fn(
        *mut c_void,
        *mut u8,
        *const u8,
        *const u8,
        usize,
        *const u8,
        usize,
    ) -> i32,
    lock: unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        *const u8,
        *const u8,
        usize,
        *const u8,
        usize,
    ),
    unlock: unsafe extern "C" fn(
        *mut u8,
        *const u8,
        *const u8,
        *const u8,
        *const u8,
        usize,
        *const u8,
        usize,
    ) -> i32,
}

const _: () = assert!(size_of::<Primitives>() == 8 + 16 * size_of::<usize>());

impl Primitives {
    unsafe fn read(src: *const Self) -> Self {
        // SAFETY: The host supplies a complete valid function table for each owner.
        let result = unsafe { *src };
        assert_eq!(result.version, 1);
        assert_eq!(result.reserved, 0);
        assert!(result.aead_size > 0 && result.hash_size > 0);
        result
    }
}

struct Buffer {
    data: NonNull<c_void>,
    layout: Layout,
    wipe: Option<unsafe extern "C" fn(*mut c_void, usize)>,
}

impl Buffer {
    fn new(
        size: usize,
        alignment: usize,
        wipe: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    ) -> Self {
        let layout = Layout::from_size_align(size, alignment).expect("valid host context layout");
        // SAFETY: Nonzero validated layout. Allocation never crosses allocator domains.
        let raw = unsafe { alloc_zeroed(layout) };
        let data = NonNull::new(raw.cast()).unwrap_or_else(|| handle_alloc_error(layout));
        Self { data, layout, wipe }
    }
    fn ptr(&self) -> *mut c_void {
        self.data.as_ptr()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: This uniquely owned live allocation has the supplied host layout.
        unsafe {
            if let Some(wipe) = self.wipe {
                wipe(self.ptr(), self.layout.size());
            }
            dealloc(self.ptr().cast(), self.layout);
        }
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

#[allow(clippy::too_many_arguments)] // Match the fixed primitive/KDF inputs without copying secrets.
unsafe fn derive(
    p: Primitives,
    keys: *mut u8,
    peer: *const u8,
    side: u8,
    secret: *const u8,
    public: *const u8,
    extra: *const u8,
    length: usize,
) -> u8 {
    assert!(side <= 1 && isize::try_from(length).is_ok());
    let shared = Buffer::new(32, 1, Some(p.wipe));
    // SAFETY: All fixed inputs/output and primitive context lifetimes follow the host contract.
    unsafe {
        (p.x25519)(shared.ptr().cast(), secret, peer);
        if std::slice::from_raw_parts(shared.ptr().cast::<u8>(), 32)
            .iter()
            .all(|v| *v == 0)
        {
            return 0;
        }
        // Vendor final wipes the hash context itself. No duplicate secret-bearing value exists.
        let hash = Buffer::new(p.hash_size, p.hash_alignment, None);
        (p.hash_init)(hash.ptr(), 64);
        (p.hash_update)(hash.ptr(), shared.ptr().cast(), 32);
        let (server, client) = if side == 1 {
            (public, peer)
        } else {
            (peer, public)
        };
        (p.hash_update)(hash.ptr(), server, 32);
        (p.hash_update)(hash.ptr(), client, 32);
        (p.hash_update)(hash.ptr(), extra, length);
        (p.hash_final)(hash.ptr(), keys);
    }
    1
}

pub(crate) struct Keys {
    primitives: Primitives,
    keys: [u8; 64],
}
pub(crate) struct Session {
    primitives: Primitives,
    secret: [u8; 32],
    public: [u8; 32],
    exchange_nonce: [u8; 24],
    keys: [u8; 64],
    peer: [u8; 32],
    encryption_nonce: [u8; 24],
}
pub(crate) struct Stream {
    primitives: Primitives,
    context: Buffer,
}

impl Drop for Keys {
    fn drop(&mut self) {
        // SAFETY: Live uniquely owned key storage; vendor wipe uses volatile writes.
        unsafe {
            (self.primitives.wipe)(self.keys.as_mut_ptr().cast(), 64);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: Wipe fields in original reverse member-destruction order.
        unsafe {
            let wipe = self.primitives.wipe;
            wipe(self.encryption_nonce.as_mut_ptr().cast(), 24);
            wipe(self.peer.as_mut_ptr().cast(), 32);
            wipe(self.keys.as_mut_ptr().cast(), 64);
            wipe(self.exchange_nonce.as_mut_ptr().cast(), 24);
            wipe(self.public.as_mut_ptr().cast(), 32);
            wipe(self.secret.as_mut_ptr().cast(), 32);
        }
    }
}

// All C exports below share the module/header safety contract. Their visibility
// is crate-local; the exported symbols, not Rust types, are the public interface.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_keys_new(p: *const Primitives) -> *mut Keys {
    // SAFETY: Zeroed byte arrays plus initialized function metadata form a valid Keys.
    unsafe {
        let owner = allocate::<Keys>();
        ptr::addr_of_mut!((*owner).primitives).write(Primitives::read(p));
        owner
    }
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
            (*owner).primitives,
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
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_new(
    p: *const Primitives,
    secret: *const u8,
) -> *mut Session {
    unsafe {
        let owner = allocate::<Session>();
        let primitives = Primitives::read(p);
        ptr::addr_of_mut!((*owner).primitives).write(primitives);
        ptr::copy_nonoverlapping(secret, ptr::addr_of_mut!((*owner).secret).cast(), 32);
        (primitives.public_key)(
            ptr::addr_of_mut!((*owner).public).cast(),
            ptr::addr_of!((*owner).secret).cast(),
        );
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
            (*owner).primitives,
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
        ((*owner).primitives.lock)(
            message,
            mac,
            ptr::addr_of!((*owner).keys).cast(),
            ptr::addr_of!((*owner).exchange_nonce).cast(),
            ptr::addr_of!((*owner).public).cast(),
            32,
            message,
            8,
        );
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_session_decrypt_response(
    owner: *const Session,
    message: *mut u8,
    mac: *const u8,
) -> u8 {
    unsafe {
        u8::from(
            ((*owner).primitives.unlock)(
                message,
                mac,
                ptr::addr_of!((*owner).keys).cast(),
                ptr::addr_of!((*owner).exchange_nonce).cast(),
                ptr::addr_of!((*owner).peer).cast(),
                32,
                message,
                8,
            ) == 0,
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
        let p = (*owner).primitives;
        let dst = allocate::<Stream>();
        ptr::addr_of_mut!((*dst).primitives).write(p);
        let context = Buffer::new(p.aead_size, p.aead_alignment, Some(p.wipe));
        (p.aead_init)(
            context.ptr(),
            ptr::addr_of!((*owner).keys)
                .cast::<u8>()
                .add(usize::from(side) * 32),
            ptr::addr_of!((*owner).encryption_nonce).cast(),
        );
        ptr::addr_of_mut!((*dst).context).write(context);
        dst
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_clone(src: *const Stream) -> *mut Stream {
    unsafe {
        let p = (*src).primitives;
        let dst = allocate::<Stream>();
        ptr::addr_of_mut!((*dst).primitives).write(p);
        let context = Buffer::new(p.aead_size, p.aead_alignment, Some(p.wipe));
        (p.aead_copy)(context.ptr(), (*src).context.ptr());
        ptr::addr_of_mut!((*dst).context).write(context);
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
            // Hosts always share one immutable primitive table/layout. Keep receiver storage stable.
            assert_eq!((*dst).context.layout, (*src).context.layout);
            ((*dst).primitives.aead_copy)((*dst).context.ptr(), (*src).context.ptr());
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
        ((*owner).primitives.aead_write)(
            (*owner).context.ptr(),
            message,
            mac,
            ptr::null(),
            0,
            message,
            length,
        );
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
            ((*owner).primitives.aead_read)(
                (*owner).context.ptr(),
                message,
                mac,
                ptr::null(),
                0,
                message,
                length,
            ) == 0,
        )
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAuthPrimitives" ["probe"] => Primitives {
        version,
        reserved,
        aead_size,
        aead_alignment,
        hash_size,
        hash_alignment,
        wipe,
        x25519,
        public_key,
        hash_init,
        hash_update,
        hash_final,
        aead_init,
        aead_copy,
        aead_write,
        aead_read,
        lock,
        unlock,
    };
}
