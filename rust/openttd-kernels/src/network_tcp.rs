/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical TCP queue/receive ownership and transfer loops. Native leaves retain socket
//! I/O, error objects and native compatibility-shell allocation. A requested virtual close or native decryption runs after the entry returns.
//! Packet pointers are live until removal; field/byte borrows end before every leaf. Raw
//! entries require live unique records under the game-state mutex; panics abort. Native
//! virtual close/decryption frames cannot unwind through Rust.
#![allow(unsafe_code, missing_docs)]
#![allow(clippy::missing_safety_doc)] // Raw lifecycle entries share the module contract.
use crate::debug::{Bytes, debug};
use crate::industrytype::NativeVec;
use crate::os_abstraction::{INVALID_SOCKET, SocketHandle};
use crate::packet::{self, State};
use std::ptr::null_mut;

pub struct Packet {
    _private: [u8; 0],
}
pub struct NetworkError {
    _private: [u8; 0],
}
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDNetworkTcpSocket" ["network_tcp"] => pub struct NetworkTCPSocketHandler {
        pub base: *mut packet::NetworkSocketHandler,
        pub socket: SocketHandle => "SOCKET",
        pub writable: bool,
        pub packets: NativeVec<*mut Packet> => "RustVector<Packet *>",
        pub send_index: usize,
        pub receive: *mut Packet => "Packet *",
    }
    "OpenTTDTcpErrorStorage" ["probe"] => pub struct TcpErrorStorage {
        pub words: [u64; 8] => "std::array<uint64_t, 8>",
    }
}
#[repr(C)]
pub struct Services {
    pub error_begin: extern "C" fn(*mut TcpErrorStorage) -> *mut NetworkError,
    pub error_end: extern "C" fn(*mut NetworkError),
    pub error_reset: extern "C" fn(*const NetworkError) -> bool,
    pub error_text: extern "C" fn(*const NetworkError) -> crate::services::ByteSpan,
    pub error_would_block: extern "C" fn(*const NetworkError) -> bool,
    /// Native steady-clock count, shared by transport and protocol timeouts.
    pub now: extern "C" fn() -> i64,
    pub packet: &'static packet::Services,
    pub packet_delete_shell: extern "C" fn(*mut Packet),
    /// Native allocation/adoption only; Rust retains queue and packet lifetime policy.
    pub packet_new_shell: extern "C" fn(*mut State) -> *mut Packet,
    /// Select result: negative on error, otherwise read bit 0 and write bit 1.
    pub poll: extern "C" fn(*mut NetworkTCPSocketHandler) -> i32,
    pub receive_bytes: extern "C" fn(*mut NetworkTCPSocketHandler, *mut u8, usize) -> isize,
    pub send_bytes: extern "C" fn(*mut NetworkTCPSocketHandler, *const u8, usize) -> isize,
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
    if !unsafe { (*socket).writable } {
        return Send::None;
    }
    if unsafe { (*socket).socket } == INVALID_SOCKET {
        return Send::Closed;
    }
    loop {
        let p = unsafe {
            if (*socket).send_index == (*socket).packets.len() {
                null_mut()
            } else {
                record(*(*socket).packets.as_mut_ptr().add((*socket).send_index))
            }
        };
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
                debug!(
                    net,
                    0,
                    "Send failed: {}",
                    Bytes(unsafe { (s.error_text)(error).bytes() })
                );
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
            let shell = unsafe { *(*socket).packets.as_mut_ptr().add((*socket).send_index) };
            destroy_packet(s, shell);
            unsafe {
                (*socket).send_index += 1;
                if (*socket).send_index == (*socket).packets.len() {
                    (*socket).packets.edit(Vec::clear);
                    (*socket).send_index = 0;
                } else if (*socket).send_index >= (*socket).packets.len() / 2 {
                    let consumed = (*socket).send_index;
                    (*socket).packets.edit(|packets| {
                        packets.drain(..consumed);
                    });
                    (*socket).send_index = 0;
                }
            }
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
                debug!(
                    net,
                    0,
                    "Recv failed: {}",
                    Bytes(unsafe { (s.error_text)(error).bytes() })
                );
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
    if unsafe { (*socket).socket } == INVALID_SOCKET {
        return Receive::None;
    }
    let mut p = record(unsafe { (*socket).receive });
    if p.is_null() {
        let mut value = State::new_receive(
            crate::network_config::TCP_MTU,
            2,
            unsafe { (*socket).base },
            check,
        );
        let shell = (s.packet_new_shell)(&raw mut value);
        unsafe {
            (*socket).receive = shell;
        }
        p = record(shell);
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

/// Complete `ReceivePacket` after the codec returns on its owning frame.
/// Native virtual codecs can throw, so their caller supplies the result after Rust returns.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tcp_receive_prepared(valid: bool) -> Receive {
    if valid {
        Receive::Ready
    } else {
        debug!(
            net,
            0, "Invalid packet received (too small / decryption error)"
        );
        Receive::Close
    }
}

/// Common receive/decrypt/virtual-close tail for direct Rust protocol consumers.
/// # Safety
/// The socket and pending packet are live; `close` may invalidate both after aliases end.
pub(crate) unsafe fn receive_packet(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    check: extern "C" fn(bool),
    close: impl FnOnce(),
) -> *mut Packet {
    let mut result = unsafe { openttd_rust_tcp_receive(s, socket, check) };
    if result == Receive::Ready {
        let p = record(unsafe { (*socket).receive });
        result = openttd_rust_tcp_receive_prepared(unsafe { packet::prepare_to_read(p, s.packet) });
    }
    match result {
        Receive::None => std::ptr::null_mut(),
        Receive::Close => {
            close();
            std::ptr::null_mut()
        }
        Receive::Ready => unsafe { std::mem::replace(&mut (*socket).receive, null_mut()) },
    }
}

/// Common send tail; close runs after all packet and transport aliases have ended.
/// # Safety
/// Same live transport contract as `openttd_rust_tcp_send`.
pub(crate) unsafe fn send_packets(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    closing: bool,
    check: extern "C" fn(bool),
    close: impl FnOnce(),
) -> Send {
    let result = unsafe { openttd_rust_tcp_send(s, socket, closing, check) };
    if result == Send::Close {
        close();
        Send::Closed
    } else {
        result
    }
}

/// Consume a native shell after dropping its canonical Rust packet allocation.
pub(crate) fn destroy_packet(s: &Services, packet: *mut Packet) {
    let record = record(packet);
    unsafe {
        std::ptr::drop_in_place(record);
    }
    (s.packet_delete_shell)(packet);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkTcpServices" ["probe"] => Services {
        error_begin, error_end, error_reset, error_text, error_would_block, now, packet, packet_delete_shell, packet_new_shell, poll, receive_bytes, send_bytes,
    };
}

/// Packet is a standard-layout facade with the generated State as its sole data member.
/// Native static assertions establish equal size/alignment and the zero-offset first member.
pub(crate) fn record(packet: *mut Packet) -> *mut State {
    packet.cast()
}

/// `CanSendReceive`: failed select leaves writable unchanged.
pub(crate) unsafe fn can_receive(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    check: extern "C" fn(bool),
) -> bool {
    check(unsafe { is_connected(socket) });
    let flags = (s.poll)(socket);
    if flags < 0 {
        return false;
    }
    unsafe {
        (*socket).writable = flags & 2 != 0;
    }
    flags & 1 != 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_can_receive(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    check: extern "C" fn(bool),
) -> u8 {
    u8::from(unsafe { can_receive(s, socket, check) })
}

pub(crate) unsafe fn is_connected(socket: *const NetworkTCPSocketHandler) -> bool {
    unsafe { (*socket).socket != INVALID_SOCKET }
}

impl NetworkTCPSocketHandler {
    fn new(base: *mut packet::NetworkSocketHandler, socket: SocketHandle) -> Self {
        Self {
            base,
            socket,
            writable: false,
            packets: NativeVec::new(),
            send_index: 0,
            receive: null_mut(),
        }
    }
}

/// Append an already prepared native packet shell, without reframing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_enqueue(
    socket: *mut NetworkTCPSocketHandler,
    packet: *mut Packet,
) {
    unsafe {
        (*socket).packets.push(packet);
    }
}

/// Native Packet construction for nonthrowing Rust stream consumers.
pub(crate) unsafe fn new_outgoing_shell(
    s: &Services,
    socket: *mut packet::NetworkSocketHandler,
    kind: u8,
    limit: usize,
    check: extern "C" fn(bool),
) -> *mut Packet {
    let mut state = unsafe { packet::new_outgoing(socket, s.packet, check, kind, limit) };
    (s.packet_new_shell)(&raw mut state)
}

/// Adopt the prepared bytes into a native compatibility shell and publish queue ownership.
pub(crate) unsafe fn enqueue_prepared(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    state: *mut State,
) {
    let packet = (s.packet_new_shell)(state);
    unsafe {
        openttd_rust_tcp_enqueue(socket, packet);
    }
}

unsafe fn clear_send(s: &Services, socket: *mut NetworkTCPSocketHandler) {
    let end = unsafe { (*socket).packets.len() };
    for i in unsafe { (*socket).send_index }..end {
        let packet = unsafe { *(*socket).packets.as_mut_ptr().add(i) };
        destroy_packet(s, packet);
    }
    unsafe {
        (*socket).packets.edit(Vec::clear);
        (*socket).send_index = 0;
    }
}

/// Logical close: mark quit, clear writable, discard send packets, then partial receive.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_close(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
) {
    unsafe {
        packet::mark_closed((*socket).base);
        (*socket).writable = false;
        clear_send(s, socket);
        let receive = std::mem::replace(&mut (*socket).receive, null_mut());
        if !receive.is_null() {
            destroy_packet(s, receive);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tcp_new(
    base: *mut packet::NetworkSocketHandler,
    socket: SocketHandle,
) -> *mut NetworkTCPSocketHandler {
    Box::into_raw(Box::new(NetworkTCPSocketHandler::new(base, socket)))
}

/// Native TCP destruction has closed the OS handle; member destruction releases receive
/// before send packets, then buffers. The common socket/encryption record still lives.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_drop(s: &Services, socket: *mut NetworkTCPSocketHandler) {
    unsafe {
        let receive = std::mem::replace(&mut (*socket).receive, null_mut());
        if !receive.is_null() {
            destroy_packet(s, receive);
        }
        clear_send(s, socket);
        drop(Box::from_raw(socket));
    }
}

/// Packet allocation leaves for payload unit tests that perform no transport I/O.
#[cfg(test)]
pub(crate) fn packet_only_services() -> Services {
    extern "C" fn unused0<R>() -> R {
        panic!("unexpected transport operation")
    }
    extern "C" fn unused1<A, R>(_: A) -> R {
        panic!("unexpected transport operation")
    }
    extern "C" fn unused3<A, B, C, R>(_: A, _: B, _: C) -> R {
        panic!("unexpected transport operation")
    }
    extern "C" fn new_shell(state: *mut State) -> *mut Packet {
        Box::into_raw(Box::new(unsafe {
            std::ptr::replace(state, State::new(0, null_mut()))
        }))
        .cast()
    }
    extern "C" fn delete_shell(packet: *mut Packet) {
        unsafe {
            drop(Box::from_raw(
                packet.cast::<std::mem::ManuallyDrop<State>>(),
            ));
        }
    }
    Services {
        error_begin: unused1,
        error_end: unused1,
        error_reset: unused1,
        error_text: unused1,
        error_would_block: unused1,
        now: unused0,
        packet: packet::test_services(),
        packet_delete_shell: delete_shell,
        packet_new_shell: new_shell,
        poll: unused1,
        receive_bytes: unused3,
        send_bytes: unused3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::replace;
    use std::ptr::null_mut;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fixture {
        result: isize,
        blocked: bool,
        calls: usize,
        logs: usize,
        pops: usize,
        drop_order: Vec<usize>,
        output: Vec<u8>,
        input: Vec<u8>,
        read_position: usize,
        read_limit: usize,
        read_end: isize,
    }
    // Error callbacks have no socket argument; serialize the two fixture entries.
    static ENTRY: Mutex<()> = Mutex::new(());
    static FIXTURE: Mutex<Option<Fixture>> = Mutex::new(None);
    fn with<R>(f: impl FnOnce(&mut Fixture) -> R) -> R {
        f(FIXTURE.lock().unwrap().as_mut().unwrap())
    }
    extern "C" fn check(value: bool) {
        assert!(value);
    }
    extern "C" fn blocked(_: *const NetworkError) -> bool {
        with(|f| f.blocked)
    }
    extern "C" fn error_begin(_: *mut TcpErrorStorage) -> *mut NetworkError {
        null_mut()
    }
    extern "C" fn error_end(_: *mut NetworkError) {}
    /// The send failure message renders the error text; each render is one log.
    extern "C" fn error_text(_: *const NetworkError) -> crate::services::ByteSpan {
        with(|f| f.logs += 1);
        crate::services::ByteSpan {
            data: std::ptr::null(),
            size: 0,
        }
    }
    extern "C" fn send_bytes(
        _: *mut NetworkTCPSocketHandler,
        data: *const u8,
        size: usize,
    ) -> isize {
        with(|f| {
            f.calls += 1;
            let result = if f.result == -2 {
                isize::try_from(size).unwrap()
            } else {
                f.result
            };
            if result > 0 {
                f.output.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(data, usize::try_from(result).unwrap())
                });
            }
            result
        })
    }
    extern "C" fn delete_shell(packet: *mut Packet) {
        let limit = unsafe { (*record(packet)).limit };
        (packet_only_services().packet_delete_shell)(packet);
        with(|f| {
            f.pops += 1;
            f.drop_order.push(limit);
        });
    }
    extern "C" fn receive_bytes(
        _: *mut NetworkTCPSocketHandler,
        data: *mut u8,
        size: usize,
    ) -> isize {
        with(|f| {
            if f.read_position == f.read_limit {
                return f.read_end;
            }
            let count = size.min(f.read_limit - f.read_position);
            unsafe {
                std::ptr::copy_nonoverlapping(f.input.as_ptr().add(f.read_position), data, count);
            }
            f.read_position += count;
            isize::try_from(count).unwrap()
        })
    }
    fn services() -> Services {
        let mut s = packet_only_services();
        s.error_begin = error_begin;
        s.error_end = error_end;
        s.error_text = error_text;
        s.error_would_block = blocked;
        s.packet_delete_shell = delete_shell;
        s.receive_bytes = receive_bytes;
        s.send_bytes = send_bytes;
        s
    }

    #[test]
    fn partial_writes_and_close_requests_preserve_cursor_and_closing_policy() {
        let _entry = ENTRY.lock().unwrap();
        let cases = [
            (-2, false, false, true, true, Send::All, 0, 1, 0, 1),
            (2, false, false, true, true, Send::Partly, 2, 1, 0, 0),
            (-1, false, true, true, true, Send::Partly, 0, 1, 0, 0),
            (-1, false, false, true, true, Send::Close, 0, 1, 1, 0),
            (-1, true, false, true, true, Send::Closed, 0, 1, 0, 0),
            (0, false, false, true, true, Send::Close, 0, 1, 0, 0),
            (0, true, false, true, true, Send::Closed, 0, 1, 0, 0),
            (-2, false, false, false, true, Send::Closed, 0, 0, 0, 0),
            (-2, false, false, true, false, Send::None, 0, 0, 0, 0),
        ];
        for (result, closing, blocked, valid, writable, expected, cursor, calls, logs, pops) in
            cases
        {
            let mut p = State::new(100, null_mut());
            p.send_bytes(&[7, 0, 9, 1, 2, 3, 4]);
            *FIXTURE.lock().unwrap() = Some(Fixture {
                result,
                blocked,
                ..Fixture::default()
            });
            let mut base = packet::NetworkSocketHandler::new();
            let mut socket =
                NetworkTCPSocketHandler::new(&raw mut base, if valid { 0 } else { INVALID_SOCKET });
            socket.writable = writable;
            socket.packets.push(Box::into_raw(Box::new(p)).cast());
            let sent =
                unsafe { openttd_rust_tcp_send(&services(), &raw mut socket, closing, check) };
            assert_eq!(sent as u8, expected as u8);
            with(|f| {
                assert_eq!((f.calls, f.logs, f.pops), (calls, logs, pops));
                if !socket.packets.is_empty() {
                    assert_eq!(
                        unsafe { (*record(socket.packets[socket.send_index])).position },
                        cursor
                    );
                }
                assert_eq!(f.output, [7, 0, 9, 1, 2, 3, 4][..f.output.len()]);
            });
            unsafe {
                clear_send(&services(), &raw mut socket);
            }
        }
        *FIXTURE.lock().unwrap() = None;
    }
    #[test]
    fn fragmented_headers_resume_and_invalid_sizes_request_close() {
        let _entry = ENTRY.lock().unwrap();
        *FIXTURE.lock().unwrap() = Some(Fixture {
            blocked: true,
            input: vec![7, 0, 9, 1, 2, 3, 4],
            read_limit: 1,
            read_end: -1,
            ..Fixture::default()
        });
        let mut base = packet::NetworkSocketHandler::new();
        let mut socket = NetworkTCPSocketHandler::new(&raw mut base, 0);
        let s = services();
        assert_eq!(
            unsafe { openttd_rust_tcp_receive(&s, &raw mut socket, check) } as u8,
            Receive::None as u8
        );
        with(|f| {
            assert_eq!(unsafe { (*record(socket.receive)).position }, 1);
            f.read_limit = f.input.len();
        });
        assert_eq!(
            unsafe { openttd_rust_tcp_receive(&s, &raw mut socket, check) } as u8,
            Receive::Ready as u8
        );
        with(|f| {
            let p = unsafe { &*record(socket.receive) };
            assert_eq!(p.position, 7);
            assert_eq!(
                unsafe { std::slice::from_raw_parts(p.data, p.size) },
                f.input
            );
        });
        for input in [vec![1, 0], vec![0, 128], vec![7, 0]] {
            destroy_packet(&s, replace(&mut socket.receive, null_mut()));
            *FIXTURE.lock().unwrap() = Some(Fixture {
                input: input.clone(),
                read_limit: input.len(),
                ..Fixture::default()
            });
            assert_eq!(
                unsafe { openttd_rust_tcp_receive(&s, &raw mut socket, check) } as u8,
                Receive::Close as u8
            );
        }
        destroy_packet(&s, socket.receive);
        *FIXTURE.lock().unwrap() = None;
    }

    #[test]
    fn logical_close_and_native_member_destruction_keep_distinct_packet_order() {
        let _entry = ENTRY.lock().unwrap();
        let s = services();
        for close in [true, false] {
            *FIXTURE.lock().unwrap() = Some(Fixture::default());
            let mut base = packet::NetworkSocketHandler::new();
            let socket = openttd_rust_tcp_new(&raw mut base, INVALID_SOCKET);
            unsafe {
                (*socket).writable = true;
                for limit in [41, 42] {
                    let mut p = State::new_send(limit, 9, 0, &raw mut base, check);
                    packet::prepare_to_send(&raw mut p, s.packet, check);
                    enqueue_prepared(&s, socket, &raw mut p);
                }
                let mut receive = State::new_receive(43, 2, &raw mut base, check);
                (*socket).receive = (s.packet_new_shell)(&raw mut receive);
                if close {
                    openttd_rust_tcp_close(&s, socket);
                    assert!(base.has_quit);
                    assert!(!(*socket).writable);
                    assert!((*socket).packets.is_empty());
                    assert!((*socket).receive.is_null());
                }
                openttd_rust_tcp_drop(&s, socket);
            }
            assert_eq!(base.has_quit, close);
            with(|f| {
                assert_eq!(f.pops, 3);
                assert_eq!(
                    f.drop_order,
                    if close { [41, 42, 43] } else { [43, 41, 42] }
                );
            });
        }
        *FIXTURE.lock().unwrap() = None;
    }
}
