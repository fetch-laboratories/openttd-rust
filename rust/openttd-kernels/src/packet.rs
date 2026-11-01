/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Packet storage and byte operations. Native sockets and callbacks borrow the shared record.
#![allow(unsafe_code)]

use std::mem::ManuallyDrop;
use std::ptr;

crate::abi::shared_records! {
    fn abi_record;
    /// Rust-owned allocation, accessed in place by the native Packet facade.
    "OpenTTDPacketState" ["packet"] => pub struct State {
        pub data: *mut u8,
        pub size: usize,
        pub capacity: usize,
        pub limit: usize,
        pub position: u16,
    }
}

/// Packet-specific framing offsets, with native unsigned arithmetic.
#[repr(C)]
pub struct Frame {
    pub message: usize,
    pub payload: usize,
}

impl State {
    /// Temporarily recover the unique allocation; no callback runs during an edit.
    fn edit_buffer(&mut self, edit: impl FnOnce(&mut Vec<u8>)) {
        // SAFETY: Only Rust allocates this storage; an empty native record may be null.
        let data = if self.capacity == 0 {
            ptr::NonNull::<u8>::dangling().as_ptr()
        } else {
            self.data
        };
        let mut bytes =
            ManuallyDrop::new(unsafe { Vec::from_raw_parts(data, self.size, self.capacity) });
        edit(&mut bytes);
        self.data = bytes.as_mut_ptr();
        self.size = bytes.len();
        self.capacity = bytes.capacity();
    }

    pub(crate) fn resize(&mut self, size: usize) {
        self.edit_buffer(|bytes| bytes.resize(size, 0));
    }

    pub(crate) fn append(&mut self, input: &[u8]) {
        self.edit_buffer(|bytes| bytes.extend_from_slice(input));
    }

    pub(crate) fn shrink(&mut self) {
        self.edit_buffer(Vec::shrink_to_fit);
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        let mut result = Self::new(self.limit);
        result.position = self.position;
        // SAFETY: This owner's initialized byte interval remains live during the copy.
        result.append(unsafe { crate::utf8::borrow(self.data, self.size) });
        result
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if self.capacity != 0 {
            // SAFETY: Matching Rust allocation, uniquely owned and freed once.
            unsafe {
                drop(Vec::from_raw_parts(self.data, self.size, self.capacity));
            }
        }
    }
}

/// Resize the shared packet allocation, zero-initializing new bytes.
/// # Safety
/// State is live, aligned and exclusive; no native span is in use during allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_resize(state: *mut State, size: usize) {
    unsafe { &mut *state }.resize(size);
}

/// Append bytes without retaining the source interval.
/// # Safety
/// State is exclusive. Input is disjoint live storage, null only when empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_append(
    state: *mut State,
    input: *const u8,
    length: usize,
) {
    unsafe { &mut *state }.append(unsafe { crate::utf8::borrow(input, length) });
}

/// Release spare packet capacity after resetting the transfer cursor.
/// # Safety
/// State is exclusive and has no live native span.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_shrink(state: *mut State) {
    unsafe { &mut *state }.shrink();
}

/// Deep-copy into uninitialized native record storage.
/// # Safety
/// Source is initialized and shared; destination is disjoint, aligned and writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_clone(destination: *mut State, source: *const State) {
    unsafe {
        destination.write((&*source).clone());
    }
}

/// Assign independent byte storage, including the live cursor and limit.
/// # Safety
/// Destination is initialized and exclusive; source is shared or aliases destination exactly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_assign(destination: *mut State, source: *const State) {
    if !ptr::eq(destination, source) {
        unsafe {
            *destination = (&*source).clone();
        }
    }
}

/// Destroy a native packet's Rust allocation.
/// # Safety
/// State is initialized and exclusive, and is destroyed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_destroy(state: *mut State) {
    unsafe {
        ptr::drop_in_place(state);
    }
}

impl State {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            data: ptr::null_mut(),
            size: 0,
            capacity: 0,
            limit,
            position: 0,
        }
    }
    pub(crate) fn can_write(&self, size: usize, amount: usize) -> bool {
        size.wrapping_add(amount) <= self.limit
    }
    pub(crate) fn can_read(&self, size: usize, amount: usize) -> bool {
        usize::from(self.position).wrapping_add(amount) <= size
    }
    pub(crate) fn remaining(&self, size: usize) -> usize {
        size.wrapping_sub(usize::from(self.position))
    }
    pub(crate) fn transfer_amount(&self, size: usize, limit: usize) -> usize {
        self.remaining(size).min(limit)
    }
    pub(crate) fn transfer_commit(&mut self, amount: isize) {
        if amount > 0 {
            // Read the live post-callback cursor, not a captured planning cursor.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let narrowed = amount as u16;
            self.position = self.position.wrapping_add(narrowed);
        }
    }
    pub(crate) fn send_amount(&self, size: usize, input: usize) -> usize {
        input.min(self.limit.wrapping_sub(size))
    }
    pub(crate) fn read_start(&mut self) {
        self.position = 2;
    }
    pub(crate) fn send_reset(&mut self) {
        self.position = 0;
    }
    pub(crate) fn skip_mac(&mut self, size: usize) {
        self.position = self.position.wrapping_add(prefix(size));
    }
    pub(crate) fn recv(&mut self, bytes: &[u8], width: u8) -> u64 {
        assert!(matches!(width, 1 | 2 | 4 | 8));
        let mut value = 0;
        for byte in 0..width {
            value |= u64::from(bytes[usize::from(self.position)]) << (byte * 8);
            self.position = self.position.wrapping_add(1);
        }
        value
    }
    pub(crate) fn parse_size(&self, bytes: &[u8]) -> u16 {
        let size = u16::from_le_bytes([bytes[0], bytes[1]]);
        if size < 3 || usize::from(size) > self.limit {
            0
        } else {
            size
        }
    }
}

pub(crate) fn frame(position: u16, size: usize, mac: usize) -> Frame {
    let message = usize::from(position).wrapping_add(mac);
    Frame {
        message,
        payload: size.wrapping_sub(message),
    }
}

pub(crate) fn prefix(size: usize) -> u16 {
    // The public constructor has no additional u16-sized limit; preserve narrowing.
    #[allow(clippy::cast_possible_truncation)]
    {
        size as u16
    }
}

pub(crate) fn header(bytes: &mut [u8]) {
    let encoded = crate::builder::little_endian(bytes.len() as u64);
    bytes[..2].copy_from_slice(&encoded.bytes[..2]);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDPacketFrame" ["probe"] => Frame {
        message,
        payload,
    };
}
