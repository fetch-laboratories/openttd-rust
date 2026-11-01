/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `network_command.cpp` command queues with their send, distribute, sync and execute decisions.
//! Command arguments stay the encoded bytes of the native `CommandTraits` templates, which alone
//! decode them through typed leaves. Queues and records have C layout: the local wait and
//! execution queues are process-wide state, and each socket shell holds its own queues in place.
//! No queue borrow survives a native leaf; entries address queued packets by raw pointer.
#![allow(unsafe_code, missing_docs)]

use crate::game_state::GameState;
use crate::services::ByteSpan;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr::null_mut;

/// `NetworkClientSocket::STATUS_MAP`; `network_command.cpp` asserts the value.
pub const STATUS_MAP: u8 = 6;

/// `CommandDataBuffer` bytes as the raw parts of a Rust `Vec<u8>`; null when never allocated.
#[repr(C)]
pub struct CommandData {
    pub data: *mut u8,
    pub size: usize,
    pub capacity: usize,
}

impl CommandData {
    const EMPTY: Self = Self {
        data: null_mut(),
        size: 0,
        capacity: 0,
    };
    fn from_vec(bytes: Vec<u8>) -> Self {
        let mut bytes = ManuallyDrop::new(bytes);
        Self {
            data: bytes.as_mut_ptr(),
            size: bytes.len(),
            capacity: bytes.capacity(),
        }
    }
    fn take(&mut self) -> Vec<u8> {
        let parts = std::mem::replace(self, Self::EMPTY);
        let parts = ManuallyDrop::new(parts);
        if parts.data.is_null() {
            Vec::new()
        } else {
            // SAFETY: Non-null parts always come from `from_vec`.
            unsafe { Vec::from_raw_parts(parts.data, parts.size, parts.capacity) }
        }
    }
    fn as_slice(&self) -> &[u8] {
        if self.data.is_null() {
            &[]
        } else {
            // SAFETY: Non-null parts always come from `from_vec`.
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
}

impl Drop for CommandData {
    fn drop(&mut self) {
        drop(self.take());
    }
}

impl Clone for CommandData {
    /// `std::vector` copy: exactly `size` bytes, no allocation when empty.
    fn clone(&self) -> Self {
        Self::from_vec(self.as_slice().to_vec())
    }
}

/// `CommandPacket`.
#[repr(C)]
#[derive(Clone)]
pub struct CommandPacket {
    pub data: CommandData,
    /// `CommandCallback *`, compared and called only by native leaves.
    pub callback: *const c_void,
    pub frame: u32,
    pub err_msg: u32,
    pub company: u8,
    pub cmd: u8,
    pub my_cmd: bool,
}

/// `CommandQueue` as the raw parts of a Rust `Vec<CommandPacket>`; null when never allocated.
#[repr(C)]
pub struct CommandQueue {
    pub data: *mut CommandPacket,
    pub size: usize,
    pub capacity: usize,
}

impl CommandQueue {
    const EMPTY: Self = Self {
        data: null_mut(),
        size: 0,
        capacity: 0,
    };
    /// Edits the queue as a `Vec`; panics abort, so the parts are always written back.
    ///
    /// # Safety
    /// `queue` is live and holds parts from this function or `EMPTY`; `edit` calls no leaf.
    unsafe fn edit<R>(queue: *mut Self, edit: impl FnOnce(&mut Vec<CommandPacket>) -> R) -> R {
        unsafe {
            let mut packets = if (*queue).data.is_null() {
                Vec::new()
            } else {
                Vec::from_raw_parts((*queue).data, (*queue).size, (*queue).capacity)
            };
            let result = edit(&mut packets);
            let mut packets = ManuallyDrop::new(packets);
            (*queue).data = packets.as_mut_ptr();
            (*queue).size = packets.len();
            (*queue).capacity = packets.capacity();
            result
        }
    }
}

/// One `NetworkClientSocket` resolved once per visit; queues and the sync record live in the
/// socket shell. `game` is the same socket as a `NetworkGameSocketHandler *`, and `last_packet`
/// its steady-clock time in nanoseconds when visited.
#[repr(C)]
pub struct Client {
    pub socket: *mut c_void,
    pub game: *mut c_void,
    pub index: usize,
    pub incoming: *mut CommandQueue,
    pub outgoing: *mut CommandQueue,
    pub sync: *mut crate::network_frame::SocketSync,
    pub last_packet: i64,
    pub status: u8,
}

/// Native leaves for `network_command.cpp`; see `network_command_ffi.h`.
#[repr(C)]
pub struct Services {
    pub company: &'static crate::company::Services,
    pub admin_log: extern "C" fn(*const c_void, *const CommandPacket),
    pub allowed_while_paused: extern "C" fn(u8) -> bool,
    pub client_queue: extern "C" fn() -> *mut CommandQueue,
    pub client_send: extern "C" fn(*const CommandPacket),
    pub commands_per_frame: extern "C" fn() -> u16,
    pub commands_per_frame_server: extern "C" fn() -> u16,
    pub execute: extern "C" fn(*const CommandPacket),
    pub execute_past: extern "C" fn(),
    pub next_client: extern "C" fn(usize, *mut Client) -> bool,
    pub server_send: extern "C" fn(*mut c_void, *const CommandPacket),
}

static WAIT: GameState<CommandQueue> = GameState::new(CommandQueue::EMPTY);
static EXECUTION: GameState<CommandQueue> = GameState::new(CommandQueue::EMPTY);

pub(crate) fn next_client(s: &Services, cursor: &mut usize) -> Option<Client> {
    let mut client = Client {
        socket: null_mut(),
        game: null_mut(),
        index: 0,
        incoming: null_mut(),
        outgoing: null_mut(),
        sync: null_mut(),
        last_packet: 0,
        status: 0,
    };
    (s.next_client)(*cursor, &raw mut client).then(|| {
        *cursor = client.index + 1;
        client
    })
}

/// `NetworkSendCommand`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_command(
    s: &Services,
    cmd: u8,
    err_msg: u32,
    callback: *const c_void,
    company: u8,
    data: ByteSpan,
) {
    let bytes = if data.size == 0 {
        Vec::new()
    } else {
        // SAFETY: The caller's buffer is live for this entry.
        unsafe { std::slice::from_raw_parts(data.data, data.size) }.to_vec()
    };
    let mut c = CommandPacket {
        data: CommandData::from_vec(bytes),
        callback,
        frame: 0,
        err_msg,
        company,
        cmd,
        my_cmd: false,
    };
    if (s.company.network_server)() {
        // Delay the server's own commands one tick, as clients' commands are.
        c.frame = crate::network_frame::frame_counter_max().wrapping_add(1);
        c.my_cmd = true;
        unsafe { CommandQueue::edit(WAIT.get(), |queue| queue.push(c)) };
        return;
    }
    // The client cannot tell which frame; it sends and forgets the packet.
    c.frame = 0;
    (s.client_send)(&raw const c);
}

/// `NetworkSyncCommandQueue`: a joining client's save omits commands queued for execution.
///
/// # Safety
/// `outgoing` is the live queue of a client socket.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_sync_command_queue(outgoing: *mut CommandQueue) {
    unsafe {
        let execution = EXECUTION.get();
        for index in 0..(*execution).size {
            let mut c = (*(*execution).data.add(index)).clone();
            c.callback = std::ptr::null();
            CommandQueue::edit(outgoing, |queue| queue.push(c));
        }
    }
}

/// `NetworkExecuteLocalCommandQueue`; the caller asserts `IsLocalCompany()` first.
pub(crate) fn execute_local_command_queue(s: &Services) {
    let company = s.company;
    let queue = if (company.network_server)() {
        EXECUTION.get()
    } else {
        (s.client_queue)()
    };
    let mut executed = 0;
    // SAFETY: Execution never adds to or frees the queue it executes.
    unsafe {
        while executed < (*queue).size {
            let cp = (*queue).data.add(executed);
            // The queue is in frame order.
            if crate::network_frame::frame_counter() < (*cp).frame {
                break;
            }
            if crate::network_frame::frame_counter() > (*cp).frame {
                (s.execute_past)();
            }
            (company.finance.set_current_company)((*cp).company);
            (s.execute)(cp);
            executed += 1;
        }
        CommandQueue::edit(queue, |queue| drop(queue.drain(..executed)));
    }
    // The local company may have changed; do not restore the previous current company.
    (company.finance.set_current_company)((company.local_company)());
}

/// `NetworkFreeLocalCommandQueue`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_free_local_command_queue() {
    unsafe {
        CommandQueue::edit(WAIT.get(), Vec::clear);
        CommandQueue::edit(EXECUTION.get(), Vec::clear);
    }
}

/// `DistributeCommandPacket`: callbacks go back only to the owning client.
unsafe fn distribute_packet(s: &Services, cp: *mut CommandPacket, owner: *const c_void) {
    unsafe {
        let callback = (*cp).callback;
        (*cp).frame = crate::network_frame::frame_counter_max().wrapping_add(1);
        let mut cursor = 0;
        while let Some(client) = next_client(s, &mut cursor) {
            if client.status >= STATUS_MAP {
                let own = client.socket.cast_const() == owner;
                (*cp).callback = if own { callback } else { std::ptr::null() };
                (*cp).my_cmd = own;
                let c = (*cp).clone();
                CommandQueue::edit(client.outgoing, |queue| queue.push(c));
            }
        }
        (*cp).callback = if owner.is_null() {
            callback
        } else {
            std::ptr::null()
        };
        (*cp).my_cmd = owner.is_null();
        let c = (*cp).clone();
        CommandQueue::edit(EXECUTION.get(), |queue| queue.push(c));
    }
}

/// `DistributeQueue`.
unsafe fn distribute_queue(s: &Services, queue: *mut CommandQueue, owner: *const c_void) {
    let mut to_go = i32::from((s.commands_per_frame)());
    if owner.is_null() {
        // The server uses `commands_per_frame_server` if higher.
        to_go = to_go.max(i32::from((s.commands_per_frame_server)()));
    }
    let mut index = 0;
    unsafe {
        while index < (*queue).size {
            let cp = (*queue).data.add(index);
            if crate::session::pause() != 0 && !(s.allowed_while_paused)((*cp).cmd) {
                index += 1;
                continue;
            }
            to_go -= 1;
            if to_go < 0 {
                break;
            }
            distribute_packet(s, cp, owner);
            (s.admin_log)(owner, cp);
            CommandQueue::edit(queue, |queue| drop(queue.remove(index)));
        }
    }
}

/// `NetworkDistributeCommands`: the server's commands first, then each client's.
pub(crate) fn distribute_commands(s: &Services) {
    unsafe { distribute_queue(s, WAIT.get(), std::ptr::null()) };
    let mut cursor = 0;
    while let Some(client) = next_client(s, &mut cursor) {
        unsafe { distribute_queue(s, client.incoming, client.socket) };
    }
}

/// `NetworkHandleCommandQueue`.
///
/// # Safety
/// `outgoing` is the live queue of `socket`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_handle_command_queue(
    s: &Services,
    socket: *mut c_void,
    outgoing: *mut CommandQueue,
) {
    unsafe {
        for index in 0..(*outgoing).size {
            (s.server_send)(socket, (*outgoing).data.add(index));
        }
        CommandQueue::edit(outgoing, Vec::clear);
    }
}

/// Grows `data` by at least one byte, as `std::vector::push_back`.
///
/// # Safety
/// `data` is live and holds parts from this module or zeroes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_data_grow(data: *mut CommandData) {
    unsafe {
        let mut bytes = (*data).take();
        bytes.reserve(1);
        std::ptr::write(data, CommandData::from_vec(bytes));
    }
}

/// # Safety
/// `data` is live and holds parts from this module or zeroes; it is left empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_data_free(data: *mut CommandData) {
    unsafe { drop((*data).take()) };
}

/// Moves `packet` to the back of `queue`, leaving its data empty.
///
/// # Safety
/// Both are live; `packet` holds parts from this module or zeroes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_queue_push(
    queue: *mut CommandQueue,
    packet: *mut CommandPacket,
) {
    unsafe {
        let c = CommandPacket {
            data: CommandData::from_vec((*packet).data.take()),
            callback: (*packet).callback,
            frame: (*packet).frame,
            err_msg: (*packet).err_msg,
            company: (*packet).company,
            cmd: (*packet).cmd,
            my_cmd: (*packet).my_cmd,
        };
        CommandQueue::edit(queue, |queue| queue.push(c));
    }
}

/// # Safety
/// `queue` is live and holds parts from this module or zeroes; it is left empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_queue_free(queue: *mut CommandQueue) {
    unsafe {
        drop(CommandQueue::edit(queue, std::mem::take));
        *queue = CommandQueue::EMPTY;
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCommandData" ["probe"] => CommandData {
        capacity,
        data,
        size,
    };
    "OpenTTDCommandPacket" ["probe"] => CommandPacket {
        callback,
        cmd,
        company,
        data,
        err_msg,
        frame,
        my_cmd,
    };
    "OpenTTDCommandQueue" ["probe"] => CommandQueue {
        capacity,
        data,
        size,
    };
    "OpenTTDNetworkCommandClient" ["probe"] => Client {
        game,
        incoming,
        index,
        last_packet,
        outgoing,
        socket,
        status,
        sync,
    };
    "OpenTTDNetworkCommandServices" ["probe"] => Services {
        admin_log,
        allowed_while_paused,
        client_queue,
        client_send,
        commands_per_frame,
        commands_per_frame_server,
        company,
        execute,
        execute_past,
        next_client,
        server_send,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(frame: u32, bytes: &[u8]) -> CommandPacket {
        CommandPacket {
            data: CommandData::from_vec(bytes.to_vec()),
            callback: std::ptr::null(),
            frame,
            err_msg: 0,
            company: 0,
            cmd: 0,
            my_cmd: false,
        }
    }

    #[test]
    fn queue_parts_survive_edits_and_moves() {
        let mut queue = CommandQueue::EMPTY;
        let mut moved = packet(7, &[1, 2, 3]);
        unsafe {
            openttd_rust_command_queue_push(&raw mut queue, &raw mut moved);
            assert!(moved.data.data.is_null() && moved.data.size == 0);
            CommandQueue::edit(&raw mut queue, |q| q.push(packet(8, &[])));
            assert_eq!(queue.size, 2);
            assert_eq!((*queue.data).data.as_slice(), &[1, 2, 3]);
            assert_eq!((*queue.data.add(1)).frame, 8);
            CommandQueue::edit(&raw mut queue, |q| drop(q.remove(0)));
            assert_eq!((*queue.data).frame, 8);
            openttd_rust_command_queue_free(&raw mut queue);
        }
        assert!(queue.data.is_null() && queue.size == 0 && queue.capacity == 0);
    }

    #[test]
    fn data_grows_from_zeroed_parts_like_push_back() {
        let mut data = CommandData::EMPTY;
        for byte in 0..40u8 {
            unsafe {
                if data.size == data.capacity {
                    openttd_rust_command_data_grow(&raw mut data);
                }
                *data.data.add(data.size) = byte;
            }
            data.size += 1;
        }
        let expected: Vec<u8> = (0..40).collect();
        assert_eq!(data.as_slice(), expected.as_slice());
        let copy = data.clone();
        assert_eq!(copy.capacity, 40);
        assert_eq!(copy.as_slice(), data.as_slice());
    }
}
