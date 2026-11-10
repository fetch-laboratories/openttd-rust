/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Basic functions to create, fill and read packets (`network/core/packet.h`/`.cpp`).
//! All network communication uses packets with a constructor-supplied byte limit.
//! Exceeding the limit while sending asserts; reading past the size returns zero
//! values or an empty string. Multibyte integers are little endian unless specified
//! otherwise: 0x01234567 is sent as 67 45 23 01. Strings have variable length and a
//! NUL terminator; their length is not sent. The original packet header also notes
//! that leap years in days-to-date calculations satisfy `(year % 4 == 0)` and
//! `((year % 100 != 0) || (year % 400 == 0))`.
//!
//! Shared packet ownership and direct typed packet operations.
//!
//! Calls are serialized. Native pointers identify live records; data spans expire before growth,
//! destruction or a callback that may mutate the packet. No record or byte borrow spans a service.
//! Assertion leaves follow native build policy and cannot mutate packet state.
//! Panics and allocation failures abort; no C++ exception may cross a Rust frame.
#![allow(unsafe_code)]

use crate::bitmath_func::{gb, gb_u64, gb_usize};
use crate::string_type::StringValidationSettings;
use std::mem::ManuallyDrop;
use std::ptr;

/// Size of the whole packet (`packet.h`).
pub type PacketSize = u16;
/// Identifier for the packet (`packet.h`).
pub type PacketType = u8;

/// `Packet::EncodedLengthOfPacketSize`.
pub const fn encoded_length_of_packet_size() -> usize {
    size_of::<PacketSize>()
}

/// `Packet::EncodedLengthOfPacketType`.
pub const fn encoded_length_of_packet_type() -> usize {
    size_of::<PacketType>()
}

/// Native socket identity; Rust accesses it only through the shared services.
pub struct NetworkSocketHandler {
    _private: [u8; 0],
}
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
        /// Packet byte count: bytes written so far when sending, allocated read size
        /// before PrepareToRead, and the full packet size after PrepareToRead.
        pub size: usize,
        /// Rust allocation capacity.
        pub capacity: usize,
        /// Native-width packet limit.
        pub limit: usize,
        /// Persistently narrowed read/transfer cursor.
        pub position: u16,
        /// Associated native socket; sending permits null.
        pub socket: *mut NetworkSocketHandler => class,
    }
}

/// Shared native operations for records and virtual implementations still stored in C++.
#[repr(C)]
pub struct Services {
    /// Query connection state before checking packet bounds.
    pub has_quit: unsafe extern "C" fn(*const NetworkSocketHandler) -> bool,
    /// Query the original virtual MAC size.
    pub mac_size: unsafe extern "C" fn(*const NetworkEncryptionHandler) -> usize,
    /// Call the base socket close operation.
    pub mark_closed: unsafe extern "C" fn(*mut NetworkSocketHandler),
    /// Resolve the current receive handler without invoking its codec.
    pub receive_handler:
        unsafe extern "C" fn(*const NetworkSocketHandler) -> *mut NetworkEncryptionHandler,
    /// Resolve the current send handler without invoking its codec.
    pub send_handler:
        unsafe extern "C" fn(*const NetworkSocketHandler) -> *mut NetworkEncryptionHandler,
    /// Resolve the Rust stream owner, null for a native virtual implementation.
    pub stream: unsafe extern "C" fn(*const NetworkEncryptionHandler) -> *mut OpenTTDAuthStream,
}

/// Native leaves shared by packet construction, string receive and send operations.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    /// Live nonthrowing socket/encryption operations.
    pub services: &'a Services,
    /// Assertion callback selected by the native build policy.
    pub check: extern "C" fn(bool),
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

    /// Create a packet used to read from the associated network socket.
    /// `limit` is the maximum accepted packet size. `initial` defaults in the native
    /// facade to the bytes needed to determine the packet size: TCP streams must not
    /// read bytes belonging to the next packet. UDP instead needs the whole datagram
    /// at once, or some data may be lost, so it passes the maximum expected packet size.
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

    /// Create a packet to send; `socket` may be null and `packet_type` identifies it.
    /// `limit` defaults in the native facade to COMPAT_MTU. Changing it requires care:
    /// older clients/servers may not expect substantially larger packets.
    /// The send handler's MAC size is queried before constructing this framing.
    #[must_use]
    pub fn new_send(
        limit: usize,
        packet_type: PacketType,
        mac: usize,
        socket: *mut NetworkSocketHandler,
        check: extern "C" fn(bool),
    ) -> Self {
        let mut result = Self::new(limit, socket);
        /* Allocate space for the size so we can write it just before sending,
         * and space for the message authentication code of the encryption. */
        let initial = encoded_length_of_packet_size().wrapping_add(mac);
        check(result.can_write(initial));
        result.resize(initial);
        result.send_uint8(packet_type, check);
        result
    }

    /// `CanWriteToPacket`: is writing the requested amount safe within the limit?
    #[must_use]
    pub fn can_write(&self, amount: usize) -> bool {
        self.size.wrapping_add(amount) <= self.limit
    }

    /// `RemainingBytesToTransfer`: bytes still available for the transfer functions,
    /// preserving native unsigned subtraction.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.size.wrapping_sub(usize::from(self.position))
    }

    /// `TransferOutWithLimit`: transfer from the last stopped position, up to `limit`.
    /// Transfer functions such as TCP send/recv or UDP sendto/recvfrom access packet
    /// storage directly without an intermediate buffer. They may process less data
    /// than requested, so only their returned positive count advances the position.
    /// Plan before the callback; commit only after its normal return.
    #[must_use]
    pub fn transfer_amount(&self, limit: usize) -> usize {
        self.remaining().min(limit)
    }

    /// Commit a positive result to the live post-callback cursor.
    pub fn transfer_commit(&mut self, amount: isize) {
        if amount > 0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let narrowed = amount as PacketSize;
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

    /* Send uint8_t, uint16_t, uint32_t and uint64_t endian-safe over the network.
     * The least significant bytes go first: 0x01234567 becomes 67 45 23 01.
     * A bool is sent as uint8_t: zero means false and nonzero means true. */

    /// Package a boolean in the packet as its canonical zero or one byte.
    pub fn send_bool(&mut self, value: bool, check: extern "C" fn(bool)) {
        self.send_uint8(u8::from(value), check);
    }
    /// Send an unsigned byte.
    pub fn send_uint8(&mut self, value: u8, check: extern "C" fn(bool)) {
        self.send_integer(value.to_le_bytes(), check);
    }
    /// Send a little-endian 16-bit integer.
    pub fn send_uint16(&mut self, value: u16, check: extern "C" fn(bool)) {
        self.send_integer(
            [
                gb(u32::from(value), 0, 8).to_le_bytes()[0],
                gb(u32::from(value), 8, 8).to_le_bytes()[0],
            ],
            check,
        );
    }
    /// Send a little-endian 32-bit integer.
    pub fn send_uint32(&mut self, value: u32, check: extern "C" fn(bool)) {
        self.send_integer(
            [
                gb(value, 0, 8).to_le_bytes()[0],
                gb(value, 8, 8).to_le_bytes()[0],
                gb(value, 16, 8).to_le_bytes()[0],
                gb(value, 24, 8).to_le_bytes()[0],
            ],
            check,
        );
    }
    /// Send a little-endian 64-bit integer.
    pub fn send_uint64(&mut self, value: u64, check: extern "C" fn(bool)) {
        self.send_integer(
            [
                gb_u64(value, 0, 8).to_le_bytes()[0],
                gb_u64(value, 8, 8).to_le_bytes()[0],
                gb_u64(value, 16, 8).to_le_bytes()[0],
                gb_u64(value, 24, 8).to_le_bytes()[0],
                gb_u64(value, 32, 8).to_le_bytes()[0],
                gb_u64(value, 40, 8).to_le_bytes()[0],
                gb_u64(value, 48, 8).to_le_bytes()[0],
                gb_u64(value, 56, 8).to_le_bytes()[0],
            ],
            check,
        );
    }

    /// Send the original string bytes plus a NUL terminator; no size byte is sent.
    pub fn send_string(&mut self, value: &[u8], check: extern "C" fn(bool)) {
        check(self.can_write(value.len().wrapping_add(1)));
        self.append(value);
        self.append(&[0]);
    }

    /// Send a narrowed 16-bit size prefix followed by all input bytes.
    pub fn send_buffer(&mut self, value: &[u8], check: extern "C" fn(bool)) {
        check(self.can_write(value.len().wrapping_add(size_of::<u16>())));
        self.send_uint16(prefix(value.len()), check);
        self.append(value);
    }

    /// Append as much input as fits and return the unwritten suffix.
    pub fn send_bytes<'a>(&mut self, value: &'a [u8]) -> &'a [u8] {
        let amount = value.len().min(self.limit.wrapping_sub(self.size));
        self.append(&value[..amount]);
        &value[amount..]
    }

    /// `HasPacketSizeData`: has the write position reached enough data for the size?
    #[must_use]
    pub fn has_packet_size_data(&self) -> bool {
        usize::from(self.position) >= encoded_length_of_packet_size()
    }

    /// `ParsePacketSize`: read the wire size and accept only plausible packet sizes.
    pub fn parse_size(&mut self) -> bool {
        // SAFETY: The original requires both size-header bytes to be initialized.
        let size = unsafe { usize::from(*self.data) | (usize::from(*self.data.add(1)) << 8) };
        /* A size smaller than the bytes required for size and type, or greater than
         * the allowed limit, means something is wrong: regard the packet as garbage. */
        if size < encoded_length_of_packet_size() + encoded_length_of_packet_type()
            || size > self.limit
        {
            return false;
        }
        self.resize(size);
        self.position = prefix(encoded_length_of_packet_size());
        true
    }

    /// Write the packet's size into its reserved raw header before encryption.
    pub fn begin_send(&mut self, check: extern "C" fn(bool)) {
        // SAFETY: Sending requires the original two reserved header bytes.
        unsafe {
            /* Prevent this being called twice or for packets that have been received. */
            check(*self.data == 0 && *self.data.add(1) == 0);
            *self.data = gb_usize(self.size, 0, 8).to_le_bytes()[0];
            *self.data.add(1) = gb_usize(self.size, 8, 8).to_le_bytes()[0];
        }
    }

    /// Reset the cursor and release spare capacity after normal encryption return.
    pub fn finish_send(&mut self) {
        self.position = 0; // We start reading from here.
        self.edit_buffer(Vec::shrink_to_fit);
    }

    /// Reset before the original receive-handler lookup.
    pub fn begin_read(&mut self) {
        /* Put the position in the right place. */
        self.position = prefix(encoded_length_of_packet_size());
    }

    /// Consume the MAC after normal decryption return, including false.
    pub fn finish_read(&mut self, mac: usize) {
        self.position = self.position.wrapping_add(prefix(mac));
    }

    /// `GetPacketType`: read the type using the send handler's MAC size.
    #[must_use]
    pub fn packet_type(&self, mac: usize) -> PacketType {
        // SAFETY: Original packet-type and handler-offset preconditions apply.
        unsafe {
            *self
                .data
                .add(encoded_length_of_packet_size().wrapping_add(mac))
        }
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

fn prefix(size: usize) -> PacketSize {
    #[allow(clippy::cast_possible_truncation)]
    {
        size as PacketSize
    }
}

/// Packet framing with original native unsigned arithmetic.
#[must_use]
pub fn frame(position: PacketSize, size: usize, mac: usize) -> Frame {
    let message = usize::from(position).wrapping_add(mac);
    Frame {
        message,
        payload: size.wrapping_sub(message),
    }
}

/// Check quit state first, then fresh packet bounds; optionally call the base close operation.
/// # Safety
/// State and socket are live. Services are valid and nonthrowing; no borrowed state spans them.
pub unsafe fn can_read(state: *mut State, services: &Services, amount: usize, close: bool) -> bool {
    unsafe {
        /* Don't allow reading from a quit client/client who sent bad data. */
        if (services.has_quit)((*state).socket) {
            return false;
        }
        /* Check if the variable is within the packet size. */
        if usize::from((*state).position).wrapping_add(amount) > (*state).size {
            if close {
                (services.mark_closed)((*state).socket);
            }
            return false;
        }
    }
    true
}

/* Receiving commands are endian-safe like the sending functions above. */

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

/// `Recv_string`: read bytes until NUL or at most `length` characters.
/// If no NUL was reached in that range, read and discard through the terminator.
/// Return at most `length - 1` bytes, validated with `settings`.
/// `length` includes the NUL terminator.
/// # Safety
/// Same contract as `recv_uint8`. The output callback is nonthrowing and retains no byte pointer.
pub unsafe fn recv_string_into(
    state: *mut State,
    context: Context<'_>,
    mut length: usize,
    settings: StringValidationSettings,
    append: impl FnMut(&[u8]),
) {
    (context.check)(length > 1);
    // Packet::Recv_string uses native small-string storage for short input.
    let mut inline = [0; 64];
    let mut inline_size = 0;
    let mut overflow = Vec::new();
    /* Both Recv_uint8 loops terminate past the packet end because Recv_uint8
     * then closes the connection and returns zero. */
    loop {
        length = length.wrapping_sub(1);
        if length == 0 {
            break;
        }
        let byte = unsafe { recv_uint8(state, context.services) };
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
        /* The string in the packet was longer. Read until the termination. */
        while unsafe { recv_uint8(state, context.services) } != 0 {}
    }
    let raw = if overflow.is_empty() {
        &inline[..inline_size]
    } else {
        &overflow
    };
    crate::string_validation::make_valid_into(raw, settings, append);
}

/// Construct an outgoing packet after resolving the live send handler.
/// # Safety
/// Socket may be null; otherwise it and the nonthrowing services are live.
pub unsafe fn new_outgoing(
    socket: *mut NetworkSocketHandler,
    context: Context<'_>,
    packet_type: PacketType,
    limit: usize,
) -> State {
    let handler = if socket.is_null() {
        ptr::null_mut()
    } else {
        unsafe { (context.services.send_handler)(socket) }
    };
    let mac = if handler.is_null() {
        0
    } else {
        unsafe { (context.services.mac_size)(handler) }
    };
    State::new_send(limit, packet_type, mac, socket, context.check)
}

unsafe fn crypt(
    state: *mut State,
    services: &Services,
    handler: *mut NetworkEncryptionHandler,
    mac_size: usize,
    sending: bool,
) -> bool {
    unsafe {
        let position = if sending {
            prefix(encoded_length_of_packet_size())
        } else {
            (*state).position
        };
        let frame = frame(position, (*state).size, mac_size);
        if !sending && (*state).size <= frame.message {
            return false;
        }
        let mac = (*state).data.add(usize::from(position));
        let message = (*state).data.add(frame.message);
        let stream = (services.stream)(handler);
        // Native virtual implementations execute on the caller's C++ frame.
        let valid = if sending {
            assert!(isize::try_from(frame.payload).is_ok());
            (*stream.cast::<crate::auth::Stream>())
                .encrypt(mac, std::slice::from_raw_parts_mut(message, frame.payload));
            true
        } else {
            assert!(isize::try_from(frame.payload).is_ok());
            (*stream.cast::<crate::auth::Stream>()).decrypt(
                &*mac.cast::<[u8; 16]>(),
                std::slice::from_raw_parts_mut(message, frame.payload),
            )
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
pub unsafe fn get_type(state: *const State, context: Context<'_>) -> PacketType {
    unsafe {
        (context.check)(
            (*state).size >= encoded_length_of_packet_size() + encoded_length_of_packet_type(),
        );
        let socket = (*state).socket;
        let handler = if socket.is_null() {
            ptr::null_mut()
        } else {
            (context.services.send_handler)(socket)
        };
        let mac = if handler.is_null() {
            0
        } else {
            (context.services.mac_size)(handler)
        };
        (&*state).packet_type(mac)
    }
}

/// Write framing, call the Rust encryption owner directly, then reset and shrink.
/// # Safety
/// State, socket and nonthrowing services are live. Handlers expose a live Rust stream.
/// No state borrow spans a service call.
pub unsafe fn prepare_to_send(state: *mut State, context: Context<'_>) {
    unsafe {
        (&mut *state).begin_send(context.check);
        let socket = (*state).socket;
        let handler = if socket.is_null() {
            ptr::null_mut()
        } else {
            (context.services.send_handler)(socket)
        };
        if !handler.is_null() {
            let mac = (context.services.mac_size)(handler);
            crypt(state, context.services, handler, mac, true);
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
            (services.receive_handler)(socket)
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
    unsafe {
        get_type(
            state,
            Context {
                services: &*services,
                check,
            },
        )
    }
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
        state.write(new_outgoing(
            socket,
            Context {
                services: &*services,
                check,
            },
            packet_type,
            limit,
        ));
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
        prepare_to_send(
            state,
            Context {
                services: &*services,
                check,
            },
        );
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
        recv_string_into(
            state,
            Context {
                services: &*services,
                check,
            },
            length,
            StringValidationSettings(settings),
            |bytes| {
                append(output, bytes.as_ptr(), bytes.len());
            },
        );
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
        has_quit,
        mac_size,
        mark_closed,
        receive_handler,
        send_handler,
        stream,
    };
}

#[cfg(test)]
pub(crate) fn test_services() -> &'static Services {
    unsafe extern "C" fn has_quit(_: *const NetworkSocketHandler) -> bool {
        false
    }
    unsafe extern "C" fn no_handler(
        _: *const NetworkSocketHandler,
    ) -> *mut NetworkEncryptionHandler {
        std::ptr::null_mut()
    }
    unsafe extern "C" fn unused<A, R>(_: A) -> R {
        panic!("unexpected packet operation")
    }
    static SERVICES: Services = Services {
        has_quit,
        mac_size: unused,
        mark_closed: unused,
        receive_handler: no_handler,
        send_handler: no_handler,
        stream: unused,
    };
    &SERVICES
}

/// Observations of the retired `packet-comparison.cpp` probe (#736 P), frozen from the unchanged
/// original only (`packet/comparison-oracle.txt`) and replayed on the production packet exports.
///
/// The `transfer`, `crypto`, `strict-mac` and `type` sections mirror the fork's C++ glue around
/// those exports (packet.h's transfer templates and packet.cpp's native encryption-handler path)
/// with `Err` standing for a C++ exception. The real C++ frames, exceptions, move operations and
/// the base `MarkClosed` dispatch stay in `tools/migration/core-boundary-comparison.cpp`
/// (`CORE_BOUNDARY_PACKET`).
#[cfg(test)]
mod comparison_tests {
    use super::*;
    use crate::oracle;
    use crate::string_type::{StringValidationSetting, StringValidationSettings};
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::fmt::Write as _;
    use std::mem::MaybeUninit;

    const ORACLE: &str = include_str!("packet/comparison-oracle.txt");

    /// The native socket's base state: `has_quit`, set only by the base `MarkClosed`.
    #[derive(Default)]
    struct Socket {
        quit: Cell<bool>,
    }

    impl Socket {
        fn handle(&self) -> *mut NetworkSocketHandler {
            ptr::from_ref(self).cast_mut().cast()
        }
    }

    unsafe extern "C" fn has_quit(socket: *const NetworkSocketHandler) -> bool {
        unsafe { (*socket.cast::<Socket>()).quit.get() }
    }
    unsafe extern "C" fn mark_closed(socket: *mut NetworkSocketHandler) {
        unsafe { (*socket.cast::<Socket>()).quit.set(true) }
    }
    /// The C++ glue reaches native handlers directly; Rust services never see them.
    unsafe extern "C" fn no_handler(
        _: *const NetworkSocketHandler,
    ) -> *mut NetworkEncryptionHandler {
        ptr::null_mut()
    }
    unsafe extern "C" fn unused<A, R>(_: A) -> R {
        panic!("unexpected packet service")
    }
    static SERVICES: Services = Services {
        has_quit,
        mac_size: unused,
        mark_closed,
        receive_handler: no_handler,
        send_handler: no_handler,
        stream: unused,
    };
    fn services() -> *const Services {
        &raw const SERVICES
    }

    thread_local! {
        static FAILED_ASSERTIONS: Cell<u32> = const { Cell::new(0) };
    }
    /// Assertions enabled (the O0 stream): no common probe case fails one.
    extern "C" fn asserts(condition: bool) {
        if !condition {
            FAILED_ASSERTIONS.with(|failed| failed.set(failed.get() + 1));
        }
    }
    /// NDEBUG (the O2 stream's asserts-off cases): the original evaluates no assertion.
    extern "C" fn ndebug(_: bool) {}

    /// A C++ exception leaving a native callback or handler.
    #[derive(Clone, Copy)]
    struct Thrown;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }

    unsafe fn span<'a>(data: *const u8, length: usize) -> &'a [u8] {
        if length == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(data, length) }
        }
    }

    fn outgoing(
        socket: &Socket,
        packet_type: u8,
        limit: usize,
        check: extern "C" fn(bool),
    ) -> State {
        let mut state = MaybeUninit::uninit();
        unsafe {
            openttd_rust_packet_init_outgoing(
                state.as_mut_ptr(),
                socket.handle(),
                services(),
                check,
                packet_type,
                limit,
            );
            state.assume_init()
        }
    }

    fn receive(
        socket: *mut NetworkSocketHandler,
        limit: usize,
        initial: usize,
        check: extern "C" fn(bool),
    ) -> State {
        let mut state = MaybeUninit::uninit();
        unsafe {
            openttd_rust_packet_init_receive(state.as_mut_ptr(), check, socket, limit, initial);
            state.assume_init()
        }
    }

    /// packet.h's `TransferIn`/`TransferOutWithLimit`: plan, call, then commit a normal return to
    /// the live cursor, which a reentrant callback may have moved.
    unsafe fn transfer(
        packet: *mut State,
        limit: usize,
        callback: impl FnOnce(*mut u8, usize) -> Result<isize, Thrown>,
    ) -> Result<isize, Thrown> {
        unsafe {
            let amount = openttd_rust_packet_transfer_amount(packet, limit);
            if amount == 0 {
                return Ok(0);
            }
            let bytes = callback((*packet).data.add(usize::from((*packet).position)), amount)?;
            openttd_rust_packet_transfer_commit(packet, bytes);
            Ok(bytes)
        }
    }

    /// The probe's `State`: size, remaining, has-size, quit, then the bytes one `TransferOut` of a
    /// copy passes to its callback (absent when no byte remains).
    unsafe fn observe(packet: *mut State, socket: &Socket) -> String {
        unsafe {
            let mut text = format!(
                "{} {} {} {}",
                (*packet).size,
                (*packet).remaining(),
                u8::from((*packet).has_packet_size_data()),
                u8::from(socket.quit.get())
            );
            let mut copy = MaybeUninit::uninit();
            openttd_rust_packet_clone(copy.as_mut_ptr(), packet);
            let mut copy = copy.assume_init();
            let _ = transfer(&raw mut copy, usize::MAX, |data, length| {
                text.push(' ');
                text += &hex(span(data, length));
                text.push('/');
                Ok(length.cast_signed())
            });
            text
        }
    }

    /// The probe's `Read`: storage sized to the wire bytes, one `TransferIn`, then `PrepareToRead`.
    fn read(socket: &Socket, wire: &[u8], check: extern "C" fn(bool)) -> State {
        let mut packet = receive(socket.handle(), 70000, wire.len(), check);
        unsafe {
            let p = &raw mut packet;
            let _ = transfer(p, usize::MAX, |data, _| {
                ptr::copy_nonoverlapping(wire.as_ptr(), data, wire.len());
                Ok(wire.len().cast_signed())
            });
            openttd_rust_packet_prepare_to_read_abi(p, services());
        }
        packet
    }

    /// `Packet::Recv_buffer`: the size, then that many bytes, as the command reader reads
    /// a buffer.
    unsafe fn recv_buffer(packet: *mut State) -> Vec<u8> {
        unsafe {
            let mut result = vec![0; recv_buffer_size(packet, &*services())];
            (*packet).read_buffer_data(&mut result);
            result
        }
    }

    unsafe extern "C" fn append(output: *mut c_void, bytes: *const u8, length: usize) {
        unsafe { (*output.cast::<Vec<u8>>()).extend_from_slice(span(bytes, length)) }
    }

    unsafe fn recv_string(
        packet: *mut State,
        check: extern "C" fn(bool),
        length: usize,
        settings: u8,
    ) -> Vec<u8> {
        let mut result = Vec::<u8>::new();
        unsafe {
            openttd_rust_packet_recv_string(
                packet,
                services(),
                check,
                length,
                settings,
                (&raw mut result).cast(),
                append,
            );
        }
        result
    }

    fn setting(element: StringValidationSetting) -> u8 {
        StringValidationSettings::from_element(element).0
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the probe's static_cast<uint8_t/uint16_t/uint32_t>(value)"
    )]
    fn binary_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        for value in [0, 1, 0xFEDC_BA98_7654_3210, u64::MAX] {
            let socket = Socket::default();
            let mut packet = outgoing(&socket, 7, 100, asserts);
            let mut assigned = outgoing(&socket, 0, 100, asserts);
            let mut copy = MaybeUninit::uninit();
            let (p, a) = (&raw mut packet, &raw mut assigned);
            let output = unsafe {
                openttd_rust_packet_send_bool(p, asserts, false);
                openttd_rust_packet_send_bool(p, asserts, true);
                openttd_rust_packet_send_uint8(p, asserts, 128);
                openttd_rust_packet_send_uint8(p, asserts, value as u8);
                openttd_rust_packet_send_uint16(p, asserts, value as u16);
                openttd_rust_packet_send_uint32(p, asserts, value as u32);
                openttd_rust_packet_send_uint64(p, asserts, value);
                openttd_rust_packet_prepare_to_send(p, services(), asserts);
                let wire = observe(p, &socket);
                openttd_rust_packet_clone(copy.as_mut_ptr(), p);
                let mut copy = copy.assume_init();
                let c = &raw mut copy;
                openttd_rust_packet_assign(a, p);
                openttd_rust_packet_prepare_to_read_abi(c, services());
                openttd_rust_packet_prepare_to_read_abi(a, services());
                let values = format!(
                    "{} {} {} {} {} {} {} {}",
                    openttd_rust_packet_recv_uint8(c, services()),
                    openttd_rust_packet_recv_bool_abi(c, services()),
                    openttd_rust_packet_recv_bool_abi(c, services()),
                    openttd_rust_packet_recv_bool_abi(c, services()),
                    openttd_rust_packet_recv_uint8(c, services()),
                    openttd_rust_packet_recv_uint16(c, services()),
                    openttd_rust_packet_recv_uint32(c, services()),
                    openttd_rust_packet_recv_uint64(c, services())
                );
                [
                    wire,
                    values,
                    observe(p, &socket),
                    observe(c, &socket),
                    observe(a, &socket),
                ]
                .join(" | ")
            };
            cases.push((format!("value={value:#x}"), output));
        }
        cases
    }

    fn buffer_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        for length in [0, 1, 8, 257] {
            let socket = Socket::default();
            let bytes = vec![0xA5; length];
            let mut packet = outgoing(&socket, 3, 1000, asserts);
            let p = &raw mut packet;
            let output = unsafe {
                openttd_rust_packet_send_buffer(p, asserts, bytes.as_ptr(), bytes.len());
                openttd_rust_packet_prepare_to_send(p, services(), asserts);
                openttd_rust_packet_prepare_to_read_abi(p, services());
                openttd_rust_packet_recv_uint8(p, services());
                let result = recv_buffer(p);
                format!("{}/ | {}", hex(&result), observe(p, &socket))
            };
            cases.push((format!("length={length}"), output));
        }
        cases
    }

    fn truncated_cases() -> Vec<(String, String)> {
        let bytes = [6, 0, 3, 2, 0, 0xAB];
        let mut cases = Vec::new();
        for size in [3, 4, 5, 6] {
            let socket = Socket::default();
            let mut packet = read(&socket, &bytes[..size], asserts);
            let p = &raw mut packet;
            let output = unsafe {
                openttd_rust_packet_recv_uint8(p, services());
                let result = recv_buffer(p);
                format!("{}/ | {}", hex(&result), observe(p, &socket))
            };
            cases.push((format!("size={size}"), output));
        }
        cases
    }

    fn send_bytes_cases() -> Vec<(String, String)> {
        let bytes = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut cases = Vec::new();
        for limit in [3, 4, 8] {
            let socket = Socket::default();
            let mut packet = outgoing(&socket, 0, limit, asserts);
            let p = &raw mut packet;
            let output = unsafe {
                let written = openttd_rust_packet_send_bytes(p, bytes.as_ptr(), bytes.len());
                let state = observe(p, &socket);
                format!("{} {written} | {state}", bytes.len() - written)
            };
            cases.push((format!("limit={limit}"), output));
        }
        cases
    }

    fn string_cases() -> Vec<(String, String)> {
        let long = vec![b'q'; 80];
        let inputs: [&[u8]; 4] = [b"a\xC3\xA9z", b"a\xFFz", b"a\nb\r\nc\t", &long];
        let settings = [
            StringValidationSetting::ReplaceWithQuestionMark,
            StringValidationSetting::AllowNewline,
            StringValidationSetting::ReplaceTabCrNlWithSpace,
        ];
        let mut cases = Vec::new();
        for (index, input) in inputs.iter().enumerate() {
            for length in [2, 3, 4, 20, 65, 66, 100] {
                for element in settings {
                    let socket = Socket::default();
                    let mut packet = outgoing(&socket, 1, 100, asserts);
                    let p = &raw mut packet;
                    let output = unsafe {
                        openttd_rust_packet_send_string(p, asserts, input.as_ptr(), input.len());
                        openttd_rust_packet_send_uint8(p, asserts, 0xED);
                        openttd_rust_packet_prepare_to_send(p, services(), asserts);
                        openttd_rust_packet_prepare_to_read_abi(p, services());
                        openttd_rust_packet_recv_uint8(p, services());
                        let text = recv_string(p, asserts, length, setting(element));
                        let next = openttd_rust_packet_recv_uint8(p, services());
                        format!(
                            "{}/{next} {} {}",
                            hex(&text),
                            u8::from(socket.quit.get()),
                            (*p).remaining()
                        )
                    };
                    cases.push((
                        format!("input={index} length={length} setting={}", element.0),
                        output,
                    ));
                }
            }
        }
        let socket = Socket::default();
        let mut packet = read(&socket, &[4, 0, b'x', b'y'], asserts);
        let p = &raw mut packet;
        let unterminated = unsafe {
            let text = recv_string(
                p,
                asserts,
                20,
                setting(StringValidationSetting::ReplaceWithQuestionMark),
            );
            format!(
                "{}/{} {}",
                hex(&text),
                u8::from(socket.quit.get()),
                (*p).remaining()
            )
        };
        cases.push((String::from("unterminated"), unterminated));
        cases
    }

    fn parse_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        for initial in [2, 8] {
            for size in [0_u16, 2, 3, 8, 9] {
                let socket = Socket::default();
                let mut packet = receive(socket.handle(), 8, initial, asserts);
                let p = &raw mut packet;
                let output = unsafe {
                    let _ = transfer(p, usize::MAX, |data, length| {
                        ptr::write_bytes(data, 0xAB, length);
                        ptr::copy_nonoverlapping(size.to_le_bytes().as_ptr(), data, 2);
                        Ok(2)
                    });
                    let parsed = openttd_rust_packet_parse_size_abi(p);
                    if parsed != 0 {
                        let _ = transfer(p, usize::MAX, |data, length| {
                            ptr::write_bytes(data, 0xCC, length);
                            Ok(length.cast_signed())
                        });
                    }
                    format!("{parsed} | {}", observe(p, &socket))
                };
                cases.push((format!("initial={initial} size={size}"), output));
            }
        }
        cases
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the probe's static_cast<uint8_t>(i) fill"
    )]
    fn wrap_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        for width in [1, 2, 4, 8] {
            let socket = Socket::default();
            let mut packet = receive(socket.handle(), 70000, 65543, asserts);
            let p = &raw mut packet;
            let output = unsafe {
                let _ = transfer(p, usize::MAX, |data, length| {
                    for index in 0..length {
                        *data.add(index) = index as u8;
                    }
                    Ok(65535)
                });
                let value = match width {
                    1 => u64::from(openttd_rust_packet_recv_uint8(p, services())),
                    2 => u64::from(openttd_rust_packet_recv_uint16(p, services())),
                    4 => u64::from(openttd_rust_packet_recv_uint32(p, services())),
                    _ => openttd_rust_packet_recv_uint64(p, services()),
                };
                format!("{value} {}", (*p).remaining())
            };
            cases.push((format!("width={width}"), output));
        }
        cases
    }

    fn bounds_cases() -> Vec<(String, String)> {
        let socket = Socket::default();
        let mut packet = receive(socket.handle(), 10, 3, asserts);
        let p = &raw mut packet;
        let output = unsafe {
            openttd_rust_packet_prepare_to_read_abi(p, services());
            let wrapped = openttd_rust_packet_can_read_abi(p, services(), usize::MAX, false);
            let first = openttd_rust_packet_can_read_abi(p, services(), 2, false);
            let first_closed = u8::from(socket.quit.get());
            let second = openttd_rust_packet_can_read_abi(p, services(), 2, true);
            let second_closed = u8::from(socket.quit.get());
            let third = openttd_rust_packet_can_read_abi(p, services(), 0, false);
            let state = observe(p, &socket);
            format!("{wrapped} {first} {first_closed} {second} {second_closed} {third} | {state}")
        };
        vec![(String::from("bounds"), output)]
    }

    fn recv_bytes_cases() -> Vec<(String, String)> {
        let wire = [5, 0, 0xA1, 0xA2, 0xA3];
        let mut cases = Vec::new();
        for length in [0, 1, 3] {
            let socket = Socket::default();
            let mut packet = read(&socket, &wire, asserts);
            let p = &raw mut packet;
            let mut result = [0; 3];
            let output = unsafe {
                let count = openttd_rust_packet_recv_bytes(p, result.as_mut_ptr(), length);
                format!("{count} {}/ | {}", hex(&result), observe(p, &socket))
            };
            cases.push((format!("length={length}"), output));
        }
        cases
    }

    fn narrow_cases() -> Vec<(String, String)> {
        let socket = Socket::default();
        let bytes = vec![0xA5; 65537];
        let mut packet = outgoing(&socket, 1, 70000, asserts);
        let p = &raw mut packet;
        let prefix = unsafe {
            openttd_rust_packet_send_buffer(p, asserts, bytes.as_ptr(), bytes.len());
            openttd_rust_packet_prepare_to_read_abi(p, services());
            openttd_rust_packet_recv_uint8(p, services());
            let result = recv_buffer(p);
            format!("{} {} {}", (*p).size, result.len(), (*p).remaining())
        };
        let mut empty = receive(socket.handle(), 10, 0, asserts);
        let mut destination = [0; 1];
        let count = unsafe {
            openttd_rust_packet_recv_bytes(
                &raw mut empty,
                destination.as_mut_ptr(),
                destination.len(),
            )
        };
        vec![
            (String::from("narrow-prefix"), prefix),
            (
                String::from("empty-source"),
                format!("{count} {}", destination[0]),
            ),
        ]
    }

    /// The asserts-off cases, compiled only under NDEBUG in the probe.
    fn asserts_off_cases() -> Vec<(String, String)> {
        let null = receive(ptr::null_mut(), 10, 2, ndebug);
        let mut cases = vec![(String::from("null-socket"), null.size.to_string())];
        let socket = Socket::default();
        let mut packet = outgoing(&socket, 7, 1, ndebug);
        let p = &raw mut packet;
        let writes = unsafe {
            openttd_rust_packet_send_bool(p, ndebug, true);
            openttd_rust_packet_send_uint8(p, ndebug, 0xAB);
            openttd_rust_packet_send_uint16(p, ndebug, 0x1234);
            openttd_rust_packet_send_uint32(p, ndebug, 0x5678_9ABC);
            openttd_rust_packet_send_uint64(p, ndebug, 0xFEDC_BA98_7654_3210);
            openttd_rust_packet_send_string(p, ndebug, b"release".as_ptr(), 7);
            openttd_rust_packet_send_buffer(p, ndebug, [1, 2, 3].as_ptr(), 3);
            openttd_rust_packet_prepare_to_send(p, services(), ndebug);
            openttd_rust_packet_prepare_to_send(p, services(), ndebug);
            observe(p, &socket)
        };
        cases.push((String::from("writes"), writes));
        for length in [0, 1] {
            let socket = Socket::default();
            let mut packet = read(&socket, &[6, 0, b'a', b'b', 0, b'c'], ndebug);
            let p = &raw mut packet;
            let output = unsafe {
                let settings = setting(StringValidationSetting::ReplaceWithQuestionMark);
                let text = recv_string(p, ndebug, length, settings);
                format!(
                    "{} | {}",
                    String::from_utf8_lossy(&text),
                    observe(p, &socket)
                )
            };
            cases.push((format!("string length={length}"), output));
        }
        cases
    }

    /// The probe's native `Encryption` handler: call log, mode, MAC size and reentry target.
    struct Handler {
        mode: u32,
        mac: usize,
        packet: *mut State,
        log: Vec<String>,
    }

    impl Handler {
        fn new(mac: usize) -> Self {
            Self {
                mode: 0,
                mac,
                packet: ptr::null_mut(),
                log: Vec::new(),
            }
        }

        unsafe fn mac_size(&mut self) -> Result<usize, Thrown> {
            self.log.push(format!("mac {}", self.mode));
            if self.mode == 4 {
                return Err(Thrown);
            }
            if self.mode == 5 {
                unsafe { openttd_rust_packet_recv_uint8(self.packet, services()) };
            }
            Ok(self.mac)
        }

        /// `Encrypt`/`Decrypt`: log, mark both spans, optionally reenter, optionally throw.
        unsafe fn crypt(
            &mut self,
            name: &str,
            mac: (*mut u8, usize),
            message: (*mut u8, usize),
            marks: (u8, u8),
        ) -> Result<(), Thrown> {
            unsafe {
                self.log.push(format!(
                    "{name} {} {} {}/",
                    mac.1,
                    message.1,
                    hex(span(message.0, message.1))
                ));
                if mac.1 != 0 {
                    *mac.0 = marks.0;
                }
                if message.1 != 0 {
                    *message.0 = marks.1;
                }
                if self.mode == 2 || self.mode == 3 {
                    openttd_rust_packet_recv_uint8(self.packet, services());
                }
            }
            if self.mode == 1 || self.mode == 3 {
                return Err(Thrown);
            }
            Ok(())
        }
    }

    /// packet.cpp's `Packet(cs, type, limit)` with a native send handler.
    unsafe fn native_outgoing(socket: &Socket, handler: &mut Handler, packet_type: u8) -> State {
        let Ok(mac) = (unsafe { handler.mac_size() }) else {
            panic!("the probe constructs in mode 0");
        };
        let mut state = MaybeUninit::uninit();
        unsafe {
            openttd_rust_packet_init_send(
                state.as_mut_ptr(),
                asserts,
                socket.handle(),
                packet_type,
                100,
                mac,
            );
            state.assume_init()
        }
    }

    /// packet.cpp's `PrepareToSend` with a native send handler.
    unsafe fn native_prepare_to_send(
        packet: *mut State,
        handler: &mut Handler,
    ) -> Result<(), Thrown> {
        unsafe {
            openttd_rust_packet_begin_send(packet, asserts);
            let mac = handler.mac_size()?;
            let mut frame = MaybeUninit::uninit();
            openttd_rust_packet_frame_abi(
                prefix(encoded_length_of_packet_size()),
                (*packet).size,
                mac,
                frame.as_mut_ptr(),
            );
            let frame = frame.assume_init();
            let data = (*packet).data;
            handler.crypt(
                "encrypt",
                (data.add(encoded_length_of_packet_size()), mac),
                (data.add(frame.message), frame.payload),
                (0xA1, 0xB2),
            )?;
            openttd_rust_packet_finish_send(packet);
            Ok(())
        }
    }

    /// packet.cpp's `PrepareToRead` with a native receive handler.
    unsafe fn native_prepare_to_read(
        packet: *mut State,
        handler: &mut Handler,
    ) -> Result<bool, Thrown> {
        unsafe {
            openttd_rust_packet_begin_read(packet);
            let mac = handler.mac_size()?;
            let mut frame = MaybeUninit::uninit();
            if openttd_rust_packet_frame_abi(
                (*packet).position,
                (*packet).size,
                mac,
                frame.as_mut_ptr(),
            ) == 0
            {
                return Ok(false);
            }
            let frame = frame.assume_init();
            let data = (*packet).data;
            let position = usize::from((*packet).position);
            handler.crypt(
                "decrypt",
                (data.add(position), mac),
                (data.add(frame.message), frame.payload),
                (0xC3, 0xD4),
            )?;
            openttd_rust_packet_finish_read(packet, mac);
            Ok(handler.mode != 0)
        }
    }

    /// packet.cpp's `GetPacketType` with a native send handler.
    unsafe fn native_packet_type(packet: *mut State, handler: &mut Handler) -> Result<u8, Thrown> {
        unsafe {
            asserts(
                (*packet).size >= encoded_length_of_packet_size() + encoded_length_of_packet_type(),
            );
            let mac = handler.mac_size()?;
            Ok(openttd_rust_packet_packet_type(packet, mac))
        }
    }

    fn outcome(result: Result<bool, Thrown>) -> String {
        match result {
            Ok(valid) => format!("{} 0", u8::from(valid)),
            Err(Thrown) => String::from("0 1"),
        }
    }

    fn transfer_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        for mode in 0..7 {
            for incoming in [false, true] {
                let socket = Socket::default();
                let mut packet = receive(socket.handle(), 20, 10, asserts);
                let p = &raw mut packet;
                let mut events = Vec::new();
                let mut calls = 0;
                let callback = |data: *mut u8, length: usize| unsafe {
                    calls += 1;
                    events.push(length.to_string());
                    if incoming {
                        *data = 0xEE;
                    }
                    if mode == 3 {
                        return Err(Thrown);
                    }
                    if mode == 4 || mode == 5 {
                        openttd_rust_packet_prepare_to_read_abi(p, services());
                        openttd_rust_packet_recv_uint8(p, services());
                    }
                    if mode == 5 {
                        let mut other = receive(socket.handle(), 20, 4, asserts);
                        let _ = transfer(&raw mut other, usize::MAX, |_, _| Ok(1));
                    }
                    Ok(match mode {
                        0 => 0,
                        1 => -1,
                        _ => 2,
                    })
                };
                let limit = if incoming {
                    usize::MAX
                } else if mode == 6 {
                    0
                } else {
                    4
                };
                let (result, threw) = match unsafe { transfer(p, limit, callback) } {
                    Ok(result) => (result, 0),
                    Err(Thrown) => (99, 1),
                };
                events.push(format!("{result} {calls} {threw}"));
                events.push(unsafe { observe(p, &socket) });
                cases.push((
                    format!("mode={mode} incoming={incoming}"),
                    events.join(" | "),
                ));
            }
        }
        cases
    }

    fn crypto_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        for mode in 0..6 {
            for sending in [false, true] {
                let socket = Socket::default();
                let mut handler = Handler::new(2);
                let mut packet = if sending {
                    unsafe { native_outgoing(&socket, &mut handler, 7) }
                } else {
                    outgoing(&socket, 7, 100, asserts)
                };
                let p = &raw mut packet;
                let result = unsafe {
                    for byte in [9, 10, 11] {
                        openttd_rust_packet_send_uint8(p, asserts, byte);
                    }
                    handler.packet = p;
                    handler.mode = mode;
                    if sending {
                        native_prepare_to_send(p, &mut handler).map(|()| false)
                    } else {
                        native_prepare_to_read(p, &mut handler)
                    }
                };
                handler.log.push(outcome(result));
                handler.log.push(unsafe { observe(p, &socket) });
                cases.push((
                    format!("mode={mode} sending={sending}"),
                    handler.log.join(" | "),
                ));
            }
        }
        for mac in [0, 3, 4] {
            let socket = Socket::default();
            let mut handler = Handler::new(mac);
            let mut packet = outgoing(&socket, 1, 100, asserts);
            let p = &raw mut packet;
            let result = unsafe {
                openttd_rust_packet_send_uint8(p, asserts, 2);
                openttd_rust_packet_send_uint8(p, asserts, 3);
                handler.packet = p;
                native_prepare_to_read(p, &mut handler)
            };
            let Ok(valid) = result else {
                panic!("mode 0 never throws")
            };
            handler.log.push(u8::from(valid).to_string());
            handler.log.push(unsafe { observe(p, &socket) });
            cases.push((format!("strict mac={mac}"), handler.log.join(" | ")));
        }
        cases
    }

    fn type_cases() -> Vec<(String, String)> {
        let socket = Socket::default();
        let mut packet = read(&socket, &[6, 0, 0xAA, 0xBB, 0xCC, 0xDD], asserts);
        let mut handler = Handler::new(1);
        handler.packet = &raw mut packet;
        let packet_type = unsafe { native_packet_type(handler.packet, &mut handler) };
        let Ok(packet_type) = packet_type else {
            panic!("mode 0 never throws")
        };
        handler.log.push(packet_type.to_string());
        handler.mode = 4;
        let threw = unsafe { native_packet_type(handler.packet, &mut handler) }.is_err();
        handler.log.push(u8::from(threw).to_string());
        vec![(String::from("send-handler type"), handler.log.join(" | "))]
    }

    #[test]
    fn packet_bytes_cursors_and_strings_match_original() {
        let oracle = oracle::parse(ORACLE);
        FAILED_ASSERTIONS.with(|failed| failed.set(0));
        assert_eq!(oracle::check(&oracle, "binary", binary_cases()), 4);
        assert_eq!(oracle::check(&oracle, "buffer", buffer_cases()), 4);
        assert_eq!(oracle::check(&oracle, "truncated", truncated_cases()), 4);
        assert_eq!(oracle::check(&oracle, "send-bytes", send_bytes_cases()), 3);
        let mut strings = string_cases();
        let unterminated = strings.pop().into_iter();
        assert_eq!(oracle::check(&oracle, "string", strings), 84);
        assert_eq!(oracle::check(&oracle, "unterminated", unterminated), 1);
        assert_eq!(oracle::check(&oracle, "parse", parse_cases()), 10);
        assert_eq!(oracle::check(&oracle, "wrap", wrap_cases()), 4);
        assert_eq!(oracle::check(&oracle, "bounds", bounds_cases()), 1);
        assert_eq!(oracle::check(&oracle, "recv-bytes", recv_bytes_cases()), 3);
        assert_eq!(oracle::check(&oracle, "narrow", narrow_cases()), 2);
        assert_eq!(
            FAILED_ASSERTIONS.with(Cell::get),
            0,
            "the original O0 stream fails no assertion"
        );
    }

    #[test]
    fn packet_transfer_and_native_handler_glue_match_original() {
        let oracle = oracle::parse(ORACLE);
        FAILED_ASSERTIONS.with(|failed| failed.set(0));
        assert_eq!(oracle::check(&oracle, "transfer", transfer_cases()), 14);
        let mut crypto = crypto_cases();
        let strict = crypto.split_off(12);
        assert_eq!(oracle::check(&oracle, "crypto", crypto), 12);
        assert_eq!(oracle::check(&oracle, "strict-mac", strict), 3);
        assert_eq!(oracle::check(&oracle, "type", type_cases()), 1);
        assert_eq!(
            FAILED_ASSERTIONS.with(Cell::get),
            0,
            "the original O0 stream fails no assertion"
        );
    }

    #[test]
    fn packet_release_assertion_paths_match_original() {
        let oracle = oracle::parse(ORACLE);
        assert_eq!(
            oracle::check(&oracle, "asserts-off", asserts_off_cases()),
            4
        );
    }
}
