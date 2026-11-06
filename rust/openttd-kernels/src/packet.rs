/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared packet ownership and direct typed packet operations.
//!
//! Calls are serialized. Native pointers identify live records; data spans expire before growth,
//! destruction or a callback that may mutate the packet. No record or byte borrow spans a service.
//! Assertion leaves follow native build policy and cannot mutate packet state.
//! Panics and allocation failures abort; no C++ exception may cross a Rust frame.
#![allow(unsafe_code)]

use std::mem::ManuallyDrop;
use std::ptr;

/// Native encryption-handler identity.
pub struct NetworkEncryptionHandler {
    _private: [u8; 0],
}
/// Rust authentication stream identity, as exposed by the native shell.
pub struct OpenTTDAuthStream {
    _private: [u8; 0],
}

crate::abi::shared_records! {
    fn abi_record;
    /// Common socket state. Native virtual shells own the encryption resources in these slots.
    "OpenTTDNetworkSocket" ["packet"] => pub struct NetworkSocketHandler {
        /// Logical close state; independent of the native OS handle.
        pub has_quit: bool,
        /// Receive encryption resource, released after the send resource.
        pub receive_handler: *mut NetworkEncryptionHandler => class,
        /// Send encryption resource, released first during native base destruction.
        pub send_handler: *mut NetworkEncryptionHandler => class,
    }
    /// Packet-specific framing offsets, with native unsigned arithmetic.
    "OpenTTDPacketFrame" ["packet"] => pub struct Frame {
        /// Offset after the MAC.
        pub message: usize,
        /// Number of message bytes.
        pub payload: usize,
    }
    /// Rust-owned allocation, accessed in place by the native Packet facade.
    "OpenTTDPacketState" ["packet"] => pub struct State {
        /// Initialized bytes, null only when capacity is zero.
        pub data: *mut u8,
        /// Initialized byte count.
        pub size: usize,
        /// Rust allocation capacity.
        pub capacity: usize,
        /// Native-width packet limit.
        pub limit: usize,
        /// Persistently narrowed read/transfer cursor.
        pub position: u16,
        /// Associated common socket record; sending permits null.
        pub socket: *mut NetworkSocketHandler,
    }
}

/// Shared native operations for records and virtual implementations still stored in C++.
#[repr(C)]
pub struct Services {
    /// Query the original virtual MAC size.
    pub mac_size: unsafe extern "C" fn(*const NetworkEncryptionHandler) -> usize,
    /// Resolve the Rust stream owner, null for a native virtual implementation.
    pub stream: unsafe extern "C" fn(*const NetworkEncryptionHandler) -> *mut OpenTTDAuthStream,
}

impl State {
    /// Empty packet allocation with the original limit and cursor defaults.
    #[must_use]
    pub fn new(limit: usize, socket: *mut NetworkSocketHandler) -> Self {
        Self {
            data: ptr::null_mut(),
            size: 0,
            capacity: 0,
            limit,
            position: 0,
            socket,
        }
    }

    /// Receive storage for a stream header or an entire datagram.
    #[must_use]
    pub fn new_receive(
        limit: usize,
        initial: usize,
        socket: *mut NetworkSocketHandler,
        check: extern "C" fn(bool),
    ) -> Self {
        check(!socket.is_null());
        let mut result = Self::new(limit, socket);
        result.resize(initial);
        result
    }

    /// Outgoing framing after querying the send handler's MAC size.
    #[must_use]
    pub fn new_send(
        limit: usize,
        packet_type: u8,
        mac: usize,
        socket: *mut NetworkSocketHandler,
        check: extern "C" fn(bool),
    ) -> Self {
        let mut result = Self::new(limit, socket);
        let initial = 2_usize.wrapping_add(mac);
        check(result.can_write(initial));
        result.resize(initial);
        result.send_uint8(packet_type, check);
        result
    }

    /// Native unsigned capacity decision.
    #[must_use]
    pub fn can_write(&self, amount: usize) -> bool {
        self.size.wrapping_add(amount) <= self.limit
    }

    /// Untransferred bytes, preserving native unsigned subtraction.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.size.wrapping_sub(usize::from(self.position))
    }

    /// Plan a transfer before a callback; commit only after its normal return.
    #[must_use]
    pub fn transfer_amount(&self, limit: usize) -> usize {
        self.remaining().min(limit)
    }

    /// Commit a positive result to the live post-callback cursor.
    pub fn transfer_commit(&mut self, amount: isize) {
        if amount > 0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let narrowed = amount as u16;
            self.position = self.position.wrapping_add(narrowed);
        }
    }

    fn edit_buffer(&mut self, edit: impl FnOnce(&mut Vec<u8>)) {
        let data = if self.capacity == 0 {
            ptr::NonNull::<u8>::dangling().as_ptr()
        } else {
            self.data
        };
        // SAFETY: Only Rust allocates this unique buffer. No callback runs during an edit.
        let mut bytes =
            ManuallyDrop::new(unsafe { Vec::from_raw_parts(data, self.size, self.capacity) });
        edit(&mut bytes);
        self.data = bytes.as_mut_ptr();
        self.size = bytes.len();
        self.capacity = bytes.capacity();
    }

    fn resize(&mut self, size: usize) {
        self.edit_buffer(|bytes| bytes.resize(size, 0));
    }
    fn append(&mut self, input: &[u8]) {
        self.edit_buffer(|bytes| bytes.extend_from_slice(input));
    }

    fn send_integer<const N: usize>(&mut self, data: [u8; N], check: extern "C" fn(bool)) {
        check(self.can_write(N));
        self.append(&data);
    }

    /// Send a canonical boolean byte.
    pub fn send_bool(&mut self, value: bool, check: extern "C" fn(bool)) {
        self.send_uint8(u8::from(value), check);
    }
    /// Send an unsigned byte.
    pub fn send_uint8(&mut self, value: u8, check: extern "C" fn(bool)) {
        self.send_integer(value.to_le_bytes(), check);
    }
    /// Send a little-endian 16-bit integer.
    pub fn send_uint16(&mut self, value: u16, check: extern "C" fn(bool)) {
        self.send_integer(value.to_le_bytes(), check);
    }
    /// Send a little-endian 32-bit integer.
    pub fn send_uint32(&mut self, value: u32, check: extern "C" fn(bool)) {
        self.send_integer(value.to_le_bytes(), check);
    }
    /// Send a little-endian 64-bit integer.
    pub fn send_uint64(&mut self, value: u64, check: extern "C" fn(bool)) {
        self.send_integer(value.to_le_bytes(), check);
    }

    /// Send the original bytes and a NUL terminator.
    pub fn send_string(&mut self, value: &[u8], check: extern "C" fn(bool)) {
        check(self.can_write(value.len().wrapping_add(1)));
        self.append(value);
        self.append(&[0]);
    }

    /// Send a narrowed 16-bit size prefix followed by all input bytes.
    pub fn send_buffer(&mut self, value: &[u8], check: extern "C" fn(bool)) {
        check(self.can_write(value.len().wrapping_add(2)));
        self.send_uint16(prefix(value.len()), check);
        self.append(value);
    }

    /// Append as much input as fits and return the unwritten suffix.
    pub fn send_bytes<'a>(&mut self, value: &'a [u8]) -> &'a [u8] {
        let amount = value.len().min(self.limit.wrapping_sub(self.size));
        self.append(&value[..amount]);
        &value[amount..]
    }

    /// Validate the wire size, resize and commit the read cursor.
    pub fn parse_size(&mut self) -> bool {
        // SAFETY: The original requires both size-header bytes to be initialized.
        let size = unsafe { usize::from(*self.data) | (usize::from(*self.data.add(1)) << 8) };
        if size < 3 || size > self.limit {
            return false;
        }
        self.resize(size);
        self.position = 2;
        true
    }

    /// Write the low 16 bits of size before any encryption-handler call.
    pub fn begin_send(&mut self, check: extern "C" fn(bool)) {
        // SAFETY: Sending requires the original two reserved header bytes.
        unsafe {
            check(*self.data == 0 && *self.data.add(1) == 0);
            let header = prefix(self.size).to_le_bytes();
            ptr::copy_nonoverlapping(header.as_ptr(), self.data, 2);
        }
    }

    /// Reset the cursor and release spare capacity after normal encryption return.
    pub fn finish_send(&mut self) {
        self.position = 0;
        self.edit_buffer(Vec::shrink_to_fit);
    }

    /// Reset before the original receive-handler lookup.
    pub fn begin_read(&mut self) {
        self.position = 2;
    }

    /// Consume the MAC after normal decryption return, including false.
    pub fn finish_read(&mut self, mac: usize) {
        self.position = self.position.wrapping_add(prefix(mac));
    }

    /// Read the type using the send handler's MAC size, as in packet.cpp.
    #[must_use]
    pub fn packet_type(&self, mac: usize) -> u8 {
        // SAFETY: Original packet-type and handler-offset preconditions apply.
        unsafe { *self.data.add(2_usize.wrapping_add(mac)) }
    }

    fn read_integer<const N: usize>(&mut self) -> [u8; N] {
        let mut value = [0; N];
        for byte in &mut value {
            // SAFETY: Bounds were checked for the whole scalar; narrowing occurs per byte.
            *byte = unsafe { *self.data.add(usize::from(self.position)) };
            self.position = self.position.wrapping_add(1);
        }
        value
    }

    /// Copy into a result whose prefixed size has already been accepted.
    /// # Safety
    /// The original bounds check succeeded; destination is live, exclusive and disjoint.
    pub unsafe fn read_buffer_data(&mut self, destination: &mut [u8]) {
        for byte in destination {
            *byte = self.read_integer::<1>()[0];
        }
    }

    /// Copy the requested bytes with #41's original short-source precondition.
    /// # Safety
    /// When source is nonempty, `destination.len()` must not exceed `remaining()`.
    /// Destination is exclusive, live and disjoint from packet storage.
    pub unsafe fn recv_bytes(&mut self, destination: &mut [u8]) -> usize {
        if self.remaining() == 0 {
            return 0;
        }
        let amount = destination.len();
        // SAFETY: The caller preserves packet.cpp's original span precondition (#41).
        unsafe {
            ptr::copy_nonoverlapping(
                self.data.add(usize::from(self.position)),
                destination.as_mut_ptr(),
                amount,
            );
        }
        self.position = self.position.wrapping_add(prefix(amount));
        amount
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        let mut result = Self::new(self.limit, self.socket);
        result.position = self.position;
        // SAFETY: The initialized byte interval remains live throughout the copy.
        result.append(unsafe { crate::utf8::borrow(self.data, self.size) });
        result
    }
    fn clone_from(&mut self, source: &Self) {
        self.limit = source.limit;
        self.position = source.position;
        self.socket = source.socket;
        // SAFETY: The source interval stays live, and clone_from requires distinct borrows.
        let input = unsafe { crate::utf8::borrow(source.data, source.size) };
        self.edit_buffer(|bytes| {
            bytes.clear();
            bytes.extend_from_slice(input);
        });
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

fn prefix(size: usize) -> u16 {
    #[allow(clippy::cast_possible_truncation)]
    {
        size as u16
    }
}

/// Packet framing with original native unsigned arithmetic.
#[must_use]
pub fn frame(position: u16, size: usize, mac: usize) -> Frame {
    let message = usize::from(position).wrapping_add(mac);
    Frame {
        message,
        payload: size.wrapping_sub(message),
    }
}

/// Check quit state first, then fresh packet bounds; optionally call the base close operation.
/// # Safety
/// State and socket are live. Services are valid and nonthrowing; no borrowed state spans them.
pub unsafe fn can_read(
    state: *mut State,
    _services: &Services,
    amount: usize,
    close: bool,
) -> bool {
    unsafe {
        if (*(*state).socket).has_quit {
            return false;
        }
        if usize::from((*state).position).wrapping_add(amount) > (*state).size {
            if close {
                mark_closed((*state).socket);
            }
            return false;
        }
    }
    true
}

/// Direct Rust receive operations; failed mandatory reads close the socket and return zero.
/// # Safety
/// Same live record, native service and byte-interval contract as `can_read`.
pub unsafe fn recv_uint8(state: *mut State, services: &Services) -> u8 {
    if !unsafe { can_read(state, services, 1, true) } {
        return 0;
    }
    unsafe { &mut *state }.read_integer::<1>()[0]
}
/// Receive a little-endian 16-bit integer.
/// # Safety
/// Same contract as `recv_uint8`.
pub unsafe fn recv_uint16(state: *mut State, services: &Services) -> u16 {
    if !unsafe { can_read(state, services, 2, true) } {
        return 0;
    }
    u16::from_le_bytes(unsafe { &mut *state }.read_integer())
}
/// Receive a little-endian 32-bit integer.
/// # Safety
/// Same contract as `recv_uint8`.
pub unsafe fn recv_uint32(state: *mut State, services: &Services) -> u32 {
    if !unsafe { can_read(state, services, 4, true) } {
        return 0;
    }
    u32::from_le_bytes(unsafe { &mut *state }.read_integer())
}
/// Receive a little-endian 64-bit integer.
/// # Safety
/// Same contract as `recv_uint8`.
pub unsafe fn recv_uint64(state: *mut State, services: &Services) -> u64 {
    if !unsafe { can_read(state, services, 8, true) } {
        return 0;
    }
    u64::from_le_bytes(unsafe { &mut *state }.read_integer())
}
/// Receive any nonzero byte as true.
/// # Safety
/// Same contract as `recv_uint8`.
pub unsafe fn recv_bool(state: *mut State, services: &Services) -> bool {
    unsafe { recv_uint8(state, services) != 0 }
}

/// Consume and validate a sized-buffer prefix before allocating result storage.
/// # Safety
/// Same contract as `recv_uint8`.
pub unsafe fn recv_buffer_size(state: *mut State, services: &Services) -> usize {
    let size = usize::from(unsafe { recv_uint16(state, services) });
    if size == 0 || !unsafe { can_read(state, services, size, true) } {
        return 0;
    }
    size
}

fn validate_string(input: &[u8], settings: u8, mut append: impl FnMut(&[u8])) {
    let mut position = 0;
    while position < input.len() {
        let step = crate::string_validation::step(&input[position..], settings);
        position += step.consumed;
        if step.stopped != 0 {
            break;
        }
        if step.output.length != 0 {
            append(&step.output.bytes[..step.output.length]);
        }
    }
}

/// Collect up to length - 1 bytes, consume through NUL, then use the shared sanitation owner.
/// # Safety
/// Same contract as `recv_uint8`. The output callback is nonthrowing and retains no byte pointer.
pub unsafe fn recv_string_into(
    state: *mut State,
    services: &Services,
    check: extern "C" fn(bool),
    mut length: usize,
    settings: u8,
    append: impl FnMut(&[u8]),
) {
    check(length > 1);
    // Packet::Recv_string uses native small-string storage for short input.
    let mut inline = [0; 64];
    let mut inline_size = 0;
    let mut overflow = Vec::new();
    loop {
        length = length.wrapping_sub(1);
        if length == 0 {
            break;
        }
        let byte = unsafe { recv_uint8(state, services) };
        if byte == 0 {
            break;
        }
        if inline_size < inline.len() {
            inline[inline_size] = byte;
            inline_size += 1;
        } else {
            if overflow.is_empty() {
                overflow.reserve(inline.len() * 2);
                overflow.extend_from_slice(&inline);
            }
            overflow.push(byte);
        }
    }
    if length == 0 {
        while unsafe { recv_uint8(state, services) } != 0 {}
    }
    let raw = if overflow.is_empty() {
        &inline[..inline_size]
    } else {
        &overflow
    };
    validate_string(raw, settings, append);
}

/// Construct an outgoing packet after resolving the live send handler.
/// # Safety
/// Socket may be null; otherwise it and the nonthrowing services are live.
pub unsafe fn new_outgoing(
    socket: *mut NetworkSocketHandler,
    services: &Services,
    check: extern "C" fn(bool),
    packet_type: u8,
    limit: usize,
) -> State {
    let handler = if socket.is_null() {
        ptr::null_mut()
    } else {
        unsafe { (*socket).send_handler }
    };
    let mac = if handler.is_null() {
        0
    } else {
        unsafe { (services.mac_size)(handler) }
    };
    State::new_send(limit, packet_type, mac, socket, check)
}

unsafe fn crypt(
    state: *mut State,
    services: &Services,
    handler: *mut NetworkEncryptionHandler,
    mac_size: usize,
    sending: bool,
) -> bool {
    unsafe {
        let position = if sending { 2 } else { (*state).position };
        let frame = frame(position, (*state).size, mac_size);
        if !sending && (*state).size <= frame.message {
            return false;
        }
        let mac = (*state).data.add(usize::from(position));
        let message = (*state).data.add(frame.message);
        let stream = (services.stream)(handler);
        // Native virtual implementations execute on the caller's C++ frame.
        let valid = if sending {
            crate::auth::openttd_rust_auth_stream_encrypt(
                stream.cast(),
                mac,
                message,
                frame.payload,
            );
            true
        } else {
            crate::auth::openttd_rust_auth_stream_decrypt(
                stream.cast(),
                mac,
                message,
                frame.payload,
            ) != 0
        };
        if !sending {
            (&mut *state).finish_read(mac_size);
        }
        valid
    }
}

/// Read the packet type after the original size assertion and send-handler query.
/// # Safety
/// Same live record and service contract as `prepare_to_send`.
pub unsafe fn get_type(state: *const State, services: &Services, check: extern "C" fn(bool)) -> u8 {
    unsafe {
        check((*state).size >= 3);
        let socket = (*state).socket;
        let handler = if socket.is_null() {
            ptr::null_mut()
        } else {
            (*socket).send_handler
        };
        let mac = if handler.is_null() {
            0
        } else {
            (services.mac_size)(handler)
        };
        (&*state).packet_type(mac)
    }
}

/// Write framing, call the Rust encryption owner directly, then reset and shrink.
/// # Safety
/// State, socket and nonthrowing services are live. Handlers expose a live Rust stream.
/// No state borrow spans a service call.
pub unsafe fn prepare_to_send(state: *mut State, services: &Services, check: extern "C" fn(bool)) {
    unsafe {
        (&mut *state).begin_send(check);
        let socket = (*state).socket;
        let handler = if socket.is_null() {
            ptr::null_mut()
        } else {
            (*socket).send_handler
        };
        if !handler.is_null() {
            let mac = (services.mac_size)(handler);
            crypt(state, services, handler, mac, true);
        }
        (&mut *state).finish_send();
    }
}

/// Reset and decrypt, advancing past the MAC after every normal decrypt return.
/// # Safety
/// Same contract as `prepare_to_send`.
pub unsafe fn prepare_to_read(state: *mut State, services: &Services) -> bool {
    unsafe {
        (&mut *state).begin_read();
        let socket = (*state).socket;
        let handler = if socket.is_null() {
            ptr::null_mut()
        } else {
            (*socket).receive_handler
        };
        if handler.is_null() {
            return true;
        }
        let mac = (services.mac_size)(handler);
        crypt(state, services, handler, mac, false)
    }
}

/// Native facade for the direct type lookup.
/// # Safety
/// Pointers follow this module's shared-record contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_get_type(
    state: *const State,
    services: *const Services,
    check: extern "C" fn(bool),
) -> u8 {
    unsafe { get_type(state, &*services, check) }
}

/// Native constructor using the same direct Rust entry as network callers.
/// # Safety
/// Socket and service pointers follow this module's shared-record contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_init_outgoing(
    state: *mut State,
    socket: *mut NetworkSocketHandler,
    services: *const Services,
    check: extern "C" fn(bool),
    packet_type: u8,
    limit: usize,
) {
    unsafe {
        state.write(new_outgoing(socket, &*services, check, packet_type, limit));
    }
}

/// Native facade for `init_receive`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_init_receive(
    state: *mut State,
    check: extern "C" fn(bool),
    socket: *mut NetworkSocketHandler,
    limit: usize,
    initial: usize,
) {
    unsafe {
        state.write(State::new_receive(limit, initial, socket, check));
    }
}

/// Native facade for `init_send`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_init_send(
    state: *mut State,
    check: extern "C" fn(bool),
    socket: *mut NetworkSocketHandler,
    packet_type: u8,
    limit: usize,
    mac: usize,
) {
    unsafe {
        state.write(State::new_send(limit, packet_type, mac, socket, check));
    }
}

/// Native facade for `clone`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_clone(destination: *mut State, source: *const State) {
    unsafe {
        destination.write((&*source).clone());
    }
}

/// Native facade for `assign`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_assign(destination: *mut State, source: *const State) {
    unsafe {
        if !ptr::eq(destination, source) {
            (&mut *destination).clone_from(&*source);
        }
    }
}

/// Native facade for `destroy`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_destroy(state: *mut State) {
    unsafe {
        ptr::drop_in_place(state);
    }
}

/// Native facade for `can_write`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_can_write_abi(
    state: *const State,
    amount: usize,
) -> u8 {
    u8::from(unsafe { (&*state).can_write(amount) })
}

/// Native facade for `can_read`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_can_read_abi(
    state: *mut State,
    services: *const Services,
    amount: usize,
    close: bool,
) -> u8 {
    u8::from(unsafe { can_read(state, &*services, amount, close) })
}

/// Native facade for `parse_size`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_parse_size_abi(state: *mut State) -> u8 {
    u8::from(unsafe { (&mut *state).parse_size() })
}

/// Native facade for `begin_send`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_begin_send(
    state: *mut State,
    check: extern "C" fn(bool),
) {
    unsafe {
        (&mut *state).begin_send(check);
    }
}

/// Native facade for `finish_send`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_finish_send(state: *mut State) {
    unsafe {
        (&mut *state).finish_send();
    }
}

/// Native facade for `begin_read`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_begin_read(state: *mut State) {
    unsafe {
        (&mut *state).begin_read();
    }
}

/// Native facade for `finish_read`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_finish_read(state: *mut State, mac: usize) {
    unsafe {
        (&mut *state).finish_read(mac);
    }
}

/// Native facade for `packet_type`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_packet_type(state: *const State, mac: usize) -> u8 {
    unsafe { (&*state).packet_type(mac) }
}

/// Native facade for `transfer_amount`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_transfer_amount(
    state: *const State,
    limit: usize,
) -> usize {
    unsafe { (&*state).transfer_amount(limit) }
}

/// Native facade for `transfer_commit`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_transfer_commit(state: *mut State, amount: isize) {
    unsafe {
        (&mut *state).transfer_commit(amount);
    }
}

/// Native facade for `frame`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_frame_abi(
    position: u16,
    size: usize,
    mac: usize,
    output: *mut Frame,
) -> u8 {
    u8::from(unsafe {
        let frame = frame(position, size, mac);
        let accepted = size > frame.message;
        output.write(frame);
        accepted
    })
}

/// Native facade for `prepare_to_send`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_prepare_to_send(
    state: *mut State,
    services: *const Services,
    check: extern "C" fn(bool),
) {
    unsafe {
        prepare_to_send(state, &*services, check);
    }
}

/// Native facade for `prepare_to_read`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_prepare_to_read_abi(
    state: *mut State,
    services: *const Services,
) -> u8 {
    u8::from(unsafe { prepare_to_read(state, &*services) })
}

/// Native facade for `recv_buffer_size`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_buffer_size(
    state: *mut State,
    services: *const Services,
) -> usize {
    unsafe { recv_buffer_size(state, &*services) }
}

/// Native facade for `recv_buffer_data`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_buffer_data(
    state: *mut State,
    output: *mut u8,
    length: usize,
) {
    unsafe {
        (&mut *state).read_buffer_data(std::slice::from_raw_parts_mut(
            if length == 0 {
                ptr::NonNull::<u8>::dangling().as_ptr()
            } else {
                output
            },
            length,
        ));
    }
}

/// Native facade for `recv_bytes`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_bytes(
    state: *mut State,
    output: *mut u8,
    length: usize,
) -> usize {
    unsafe {
        (&mut *state).recv_bytes(std::slice::from_raw_parts_mut(
            if length == 0 {
                ptr::NonNull::<u8>::dangling().as_ptr()
            } else {
                output
            },
            length,
        ))
    }
}

/// Native facade for `send_string`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_string(
    state: *mut State,
    check: extern "C" fn(bool),
    input: *const u8,
    length: usize,
) {
    unsafe {
        (&mut *state).send_string(crate::utf8::borrow(input, length), check);
    }
}

/// Native facade for `send_buffer`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_buffer(
    state: *mut State,
    check: extern "C" fn(bool),
    input: *const u8,
    length: usize,
) {
    unsafe {
        (&mut *state).send_buffer(crate::utf8::borrow(input, length), check);
    }
}

/// Native facade for `send_bytes`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_bytes(
    state: *mut State,
    input: *const u8,
    length: usize,
) -> usize {
    unsafe {
        length
            - (&mut *state)
                .send_bytes(crate::utf8::borrow(input, length))
                .len()
    }
}

/// Native facade for `recv_string`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_string(
    state: *mut State,
    services: *const Services,
    check: extern "C" fn(bool),
    length: usize,
    settings: u8,
    output: *mut std::ffi::c_void,
    append: unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize),
) {
    unsafe {
        recv_string_into(state, &*services, check, length, settings, |bytes| {
            append(output, bytes.as_ptr(), bytes.len());
        });
    }
}

/// Native facade for `send_bool`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_bool(
    state: *mut State,
    check: extern "C" fn(bool),
    value: bool,
) {
    unsafe {
        (&mut *state).send_bool(value, check);
    }
}

/// Native facade for `send_uint8`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_uint8(
    state: *mut State,
    check: extern "C" fn(bool),
    value: u8,
) {
    unsafe {
        (&mut *state).send_uint8(value, check);
    }
}

/// Native facade for `send_uint16`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_uint16(
    state: *mut State,
    check: extern "C" fn(bool),
    value: u16,
) {
    unsafe {
        (&mut *state).send_uint16(value, check);
    }
}

/// Native facade for `send_uint32`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_uint32(
    state: *mut State,
    check: extern "C" fn(bool),
    value: u32,
) {
    unsafe {
        (&mut *state).send_uint32(value, check);
    }
}

/// Native facade for `send_uint64`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_uint64(
    state: *mut State,
    check: extern "C" fn(bool),
    value: u64,
) {
    unsafe {
        (&mut *state).send_uint64(value, check);
    }
}

/// Native facade for `recv_bool`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_bool_abi(
    state: *mut State,
    services: *const Services,
) -> u8 {
    u8::from(unsafe { recv_bool(state, &*services) })
}

/// Native facade for `recv_uint8`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_uint8(
    state: *mut State,
    services: *const Services,
) -> u8 {
    unsafe { recv_uint8(state, &*services) }
}

/// Native facade for `recv_uint16`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_uint16(
    state: *mut State,
    services: *const Services,
) -> u16 {
    unsafe { recv_uint16(state, &*services) }
}

/// Native facade for `recv_uint32`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_uint32(
    state: *mut State,
    services: *const Services,
) -> u32 {
    unsafe { recv_uint32(state, &*services) }
}

/// Native facade for `recv_uint64`.
/// # Safety
/// Record and service pointers follow this module's ownership and byte-span contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv_uint64(
    state: *mut State,
    services: *const Services,
) -> u64 {
    unsafe { recv_uint64(state, &*services) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDPacketServices" ["packet"] => Services {
        mac_size,
        stream,
    };
}

#[cfg(test)]
pub(crate) fn test_services() -> &'static Services {
    unsafe extern "C" fn unused<A, R>(_: A) -> R {
        panic!("unexpected packet operation")
    }
    static SERVICES: Services = Services {
        mac_size: unused,
        stream: unused,
    };
    &SERVICES
}

impl NetworkSocketHandler {
    /// `NetworkSocketHandler` constructor defaults, independent of any pool identity.
    pub(crate) const fn new() -> Self {
        Self {
            has_quit: false,
            receive_handler: ptr::null_mut(),
            send_handler: ptr::null_mut(),
        }
    }
}

/// Allocate common state before native derived construction.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_socket_new() -> *mut NetworkSocketHandler {
    Box::into_raw(Box::new(NetworkSocketHandler::new()))
}

/// # Safety
/// Native base destruction has released send then receive encryption resources; the
/// uniquely owned record remains live until this final release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_socket_drop(socket: *mut NetworkSocketHandler) {
    unsafe { drop(Box::from_raw(socket)) };
}

/// `NetworkSocketHandler::MarkClosed`, with no OS-handle closure.
/// # Safety
/// The common record is live and exclusively accessible under the network/mutex contract.
pub(crate) unsafe fn mark_closed(socket: *mut NetworkSocketHandler) {
    unsafe { (*socket).has_quit = true };
}
