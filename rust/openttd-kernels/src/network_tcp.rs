/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common TCP transfer loops. Native leaves retain socket I/O, error objects and queue
//! allocation. A requested virtual close or native decryption runs after the entry returns.
//! Packet pointers are live until removal; field/byte borrows end before every leaf.
#![allow(unsafe_code, missing_docs)]
use crate::packet::{self, State};

pub struct NetworkTCPSocketHandler {
    _private: [u8; 0],
}
pub struct Packet {
    _private: [u8; 0],
}
pub struct NetworkError {
    _private: [u8; 0],
}
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDTcpErrorStorage" ["probe"] => pub struct TcpErrorStorage {
        pub words: [u64; 8],
    }
}
#[repr(C)]
pub struct Services {
    pub base_socket:
        extern "C" fn(*mut NetworkTCPSocketHandler) -> *mut packet::NetworkSocketHandler,
    pub error_begin: extern "C" fn(*mut TcpErrorStorage) -> *mut NetworkError,
    pub error_end: extern "C" fn(*mut NetworkError),
    pub error_receive: extern "C" fn(*const NetworkError),
    pub error_reset: extern "C" fn(*const NetworkError) -> bool,
    pub error_send: extern "C" fn(*const NetworkError),
    pub error_would_block: extern "C" fn(*const NetworkError) -> bool,
    pub invalid_packet: extern "C" fn(),
    pub packet: &'static packet::Services,
    pub packet_delete_shell: extern "C" fn(*mut Packet),
    pub packet_record: extern "C" fn(*mut Packet) -> *mut State,
    pub receive_bytes: extern "C" fn(*mut NetworkTCPSocketHandler, *mut u8, usize) -> isize,
    pub receive_create: extern "C" fn(*mut NetworkTCPSocketHandler, *mut State) -> *mut State,
    pub receive_packet: extern "C" fn(*mut NetworkTCPSocketHandler) -> *mut State,
    pub receive_take: extern "C" fn(*mut NetworkTCPSocketHandler) -> *mut Packet,
    pub send_bytes: extern "C" fn(*mut NetworkTCPSocketHandler, *const u8, usize) -> isize,
    pub send_front: extern "C" fn(*mut NetworkTCPSocketHandler) -> *mut State,
    pub send_pop: extern "C" fn(*mut NetworkTCPSocketHandler),
    pub valid: extern "C" fn(*const NetworkTCPSocketHandler) -> bool,
    pub writable: extern "C" fn(*const NetworkTCPSocketHandler) -> bool,
}
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Send {
    Closed,
    None,
    Partly,
    All,
    Close,
}
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Receive {
    None,
    Close,
    Ready,
}

/// `SendPackets`, returning before a requested virtual close.
/// # Safety
/// The socket, queues, services and initialized packet records are live for the entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_send(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    closing: bool,
    check: extern "C" fn(bool),
) -> Send {
    if !(s.writable)(socket) {
        return Send::None;
    }
    if !(s.valid)(socket) {
        return Send::Closed;
    }
    loop {
        let p = (s.send_front)(socket);
        if p.is_null() {
            return Send::All;
        }
        let amount = unsafe { (*p).remaining() };
        let result = if amount == 0 {
            0
        } else {
            let (position, size, data) = unsafe { ((*p).position, (*p).size, (*p).data) };
            check(usize::from(position) < size);
            check(usize::from(position).wrapping_add(amount) <= size);
            let result = (s.send_bytes)(socket, unsafe { data.add(usize::from(position)) }, amount);
            unsafe {
                (*p).transfer_commit(result);
            }
            result
        };
        if result == -1 {
            let mut storage = std::mem::MaybeUninit::<TcpErrorStorage>::uninit();
            let error = (s.error_begin)(storage.as_mut_ptr());
            let blocked = (s.error_would_block)(error);
            if !blocked && !closing {
                (s.error_send)(error);
            }
            (s.error_end)(error);
            return if blocked {
                Send::Partly
            } else if closing {
                Send::Closed
            } else {
                Send::Close
            };
        }
        if result == 0 {
            return if closing { Send::Closed } else { Send::Close };
        }
        if unsafe { (*p).remaining() } == 0 {
            unsafe {
                std::ptr::drop_in_place(p);
            }
            (s.send_pop)(socket);
        } else {
            return Send::Partly;
        }
    }
}

unsafe fn read_bytes(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    p: *mut State,
    check: extern "C" fn(bool),
) -> Receive {
    while unsafe { (*p).remaining() } != 0 {
        let (position, size, data) = unsafe { ((*p).position, (*p).size, (*p).data) };
        let amount = size.wrapping_sub(usize::from(position));
        check(usize::from(position) < size);
        check(usize::from(position).wrapping_add(amount) <= size);
        let result = (s.receive_bytes)(socket, unsafe { data.add(usize::from(position)) }, amount);
        unsafe {
            (*p).transfer_commit(result);
        }
        if result == -1 {
            let mut storage = std::mem::MaybeUninit::<TcpErrorStorage>::uninit();
            let error = (s.error_begin)(storage.as_mut_ptr());
            let blocked = (s.error_would_block)(error);
            if !blocked && !(s.error_reset)(error) {
                (s.error_receive)(error);
            }
            (s.error_end)(error);
            return if blocked {
                Receive::None
            } else {
                Receive::Close
            };
        }
        if result == 0 {
            return Receive::Close;
        }
    }
    Receive::Ready
}

/// `ReceivePacket` through completion of the bytes, before decryption/virtual close.
/// # Safety
/// Same live socket/record contract as `openttd_rust_tcp_send`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_receive(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    check: extern "C" fn(bool),
) -> Receive {
    if !(s.valid)(socket) {
        return Receive::None;
    }
    let mut p = (s.receive_packet)(socket);
    if p.is_null() {
        let mut value = State::new_receive(
            crate::network_config::TCP_MTU,
            2,
            (s.base_socket)(socket),
            check,
        );
        p = (s.receive_create)(socket, &raw mut value);
    }
    if unsafe { (*p).position } < 2 {
        let result = unsafe { read_bytes(s, socket, p, check) };
        if result != Receive::Ready {
            return result;
        }
        if !unsafe { &mut *p }.parse_size() {
            return Receive::Close;
        }
    }
    unsafe { read_bytes(s, socket, p, check) }
}

/// Consume a native shell after dropping its canonical Rust packet allocation.
pub(crate) fn destroy_packet(s: &Services, packet: *mut Packet) {
    let record = (s.packet_record)(packet);
    unsafe {
        std::ptr::drop_in_place(record);
    }
    (s.packet_delete_shell)(packet);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkTcpServices" ["probe"] => Services {
        base_socket, error_begin, error_end, error_receive, error_reset, error_send,
        error_would_block, invalid_packet, packet, packet_delete_shell, packet_record,
        receive_bytes, receive_create, receive_packet, receive_take, send_bytes, send_front,
        send_pop, valid, writable,
    };
}
