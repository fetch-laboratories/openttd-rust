/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common TCP transfer loops. Native leaves retain socket I/O, error objects and queue
//! allocation. A requested virtual close or native decryption runs after the entry returns.
//! Packet pointers are live until removal; field/byte borrows end before every leaf.
#![allow(unsafe_code, missing_docs)]
use crate::debug::{Bytes, debug};
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
        pub words: [u64; 8] => "std::array<uint64_t, 8>",
    }
}
#[repr(C)]
pub struct Services {
    pub base_socket:
        extern "C" fn(*mut NetworkTCPSocketHandler) -> *mut packet::NetworkSocketHandler,
    pub can_send_receive: extern "C" fn(*mut NetworkTCPSocketHandler) -> bool,
    pub error_begin: extern "C" fn(*mut TcpErrorStorage) -> *mut NetworkError,
    pub error_end: extern "C" fn(*mut NetworkError),
    pub error_reset: extern "C" fn(*const NetworkError) -> bool,
    pub error_text: extern "C" fn(*const NetworkError) -> crate::services::ByteSpan,
    pub error_would_block: extern "C" fn(*const NetworkError) -> bool,
    /// Native steady-clock count, shared by transport and protocol timeouts.
    pub now: extern "C" fn() -> i64,
    pub packet: &'static packet::Services,
    pub packet_delete_shell: extern "C" fn(*mut Packet),
    /// Allocate the native packet shell and append it to this socket's send queue.
    pub packet_enqueue: extern "C" fn(*mut NetworkTCPSocketHandler, *mut State),
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
/// Original `tcp.h` packet sending state, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SendPacketsState(pub u8);
impl SendPacketsState {
    /// The connection got closed.
    pub const SPS_CLOSED: Self = Self(0);
    /// The buffer is full; no packet part could be sent.
    pub const SPS_NONE_SENT: Self = Self(1);
    /// Packets were partly sent; the queue still has data.
    pub const SPS_PARTLY_SENT: Self = Self(2);
    /// All queued packets were sent.
    pub const SPS_ALL_SENT: Self = Self(3);
}

/// Live native socket and its existing shared leaves.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub services: &'a Services,
    pub socket: *mut NetworkTCPSocketHandler,
}

/// Existing boundary protocol: original send states plus a deferred virtual close.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Send {
    Closed = SendPacketsState::SPS_CLOSED.0,
    None = SendPacketsState::SPS_NONE_SENT.0,
    Partly = SendPacketsState::SPS_PARTLY_SENT.0,
    All = SendPacketsState::SPS_ALL_SENT.0,
    Close = 4,
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
pub unsafe fn send_packets(
    context: Context<'_>,
    closing: bool,
    check: extern "C" fn(bool),
) -> Send {
    let Context {
        services: s,
        socket,
    } = context;
    /* We cannot write to this socket! */
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
            /* Something went wrong: request client close unless already closing. */
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
            /* Client/server has left us. */
            return if closing { Send::Closed } else { Send::Close };
        }
        /* Is this packet sent? */
        if unsafe { (*p).remaining() } == 0 {
            /* Go to the next packet. */
            unsafe {
                std::ptr::drop_in_place(p);
            }
            (s.send_pop)(socket);
        } else {
            return Send::Partly;
        }
    }
}

/// Native positional shim; the module lifetime contract applies.
/// # Safety
/// The socket and all shared leaves remain live through the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_send(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    closing: bool,
    check: extern "C" fn(bool),
) -> Send {
    unsafe {
        send_packets(
            Context {
                services: s,
                socket,
            },
            closing,
            check,
        )
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
            /* Something went wrong; connection reset needs no error message. */
            if !blocked && !(s.error_reset)(error) {
                debug!(
                    net,
                    0,
                    "Recv failed: {}",
                    Bytes(unsafe { (s.error_text)(error).bytes() })
                );
            }
            (s.error_end)(error);
            /* Connection would block, so stop for now. */
            return if blocked {
                Receive::None
            } else {
                Receive::Close
            };
        }
        if result == 0 {
            /* Client/server has left. */
            return Receive::Close;
        }
    }
    Receive::Ready
}

/// `ReceivePacket` through completion of the bytes, before decryption/virtual close.
/// # Safety
/// Same live socket/record contract as `openttd_rust_tcp_send`.
pub unsafe fn receive_packet(context: Context<'_>, check: extern "C" fn(bool)) -> Receive {
    let Context {
        services: s,
        socket,
    } = context;
    if !(s.valid)(socket) {
        return Receive::None;
    }
    let mut p = (s.receive_packet)(socket);
    if p.is_null() {
        let mut value = State::new_receive(
            crate::network_config::TCP_MTU,
            packet::encoded_length_of_packet_size(),
            (s.base_socket)(socket),
            check,
        );
        p = (s.receive_create)(socket, &raw mut value);
    }
    /* Read packet size. */
    if !unsafe { (*p).has_packet_size_data() } {
        let result = unsafe { read_bytes(s, socket, p, check) };
        if result != Receive::Ready {
            return result;
        }
        /* Parse the received size; invalid size requests connection close. */
        if !unsafe { &mut *p }.parse_size() {
            return Receive::Close;
        }
    }
    /* Read the rest of the packet. */
    unsafe { read_bytes(s, socket, p, check) }
}

/// Native positional shim; the module lifetime contract applies.
/// # Safety
/// The socket and all shared leaves remain live through the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tcp_receive(
    s: &Services,
    socket: *mut NetworkTCPSocketHandler,
    check: extern "C" fn(bool),
) -> Receive {
    unsafe {
        receive_packet(
            Context {
                services: s,
                socket,
            },
            check,
        )
    }
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
        base_socket, can_send_receive, error_begin, error_end, error_reset, error_text, error_would_block, now, packet, packet_delete_shell, packet_enqueue, packet_record,
        receive_bytes, receive_create, receive_packet, receive_take, send_bytes, send_front,
        send_pop, valid, writable,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null_mut, replace};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fixture {
        packet: Option<Box<State>>,
        result: isize,
        valid: bool,
        writable: bool,
        blocked: bool,
        calls: usize,
        logs: usize,
        pops: usize,
        output: Vec<u8>,
        input: Vec<u8>,
        read_position: usize,
        read_limit: usize,
        read_end: isize,
    }
    // SAFETY: The packet owns its allocation; socket identities are never dereferenced.
    unsafe impl std::marker::Send for Fixture {}
    // Error callbacks have no socket argument; serialize the two fixture entries.
    static ENTRY: Mutex<()> = Mutex::new(());
    static FIXTURE: Mutex<Option<Fixture>> = Mutex::new(None);
    fn with<R>(f: impl FnOnce(&mut Fixture) -> R) -> R {
        f(FIXTURE.lock().unwrap().as_mut().unwrap())
    }
    extern "C" fn check(value: bool) {
        assert!(value);
    }
    extern "C" fn unused1<A, R>(_: A) -> R {
        panic!("unexpected TCP operation")
    }
    extern "C" fn unused0<R>() -> R {
        panic!("unexpected TCP operation")
    }
    extern "C" fn unused2<A, B, R>(_: A, _: B) -> R {
        panic!("unexpected TCP operation")
    }
    extern "C" fn packet(_: *mut NetworkTCPSocketHandler) -> *mut State {
        with(|f| f.packet.as_deref_mut().map_or(null_mut(), |p| &raw mut *p))
    }
    extern "C" fn valid(_: *const NetworkTCPSocketHandler) -> bool {
        with(|f| f.valid)
    }
    extern "C" fn writable(_: *const NetworkTCPSocketHandler) -> bool {
        with(|f| f.writable)
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
    extern "C" fn send_pop(_: *mut NetworkTCPSocketHandler) {
        with(|f| {
            let mut p = f.packet.take().unwrap();
            // The send owner already dropped the record before removing its shell.
            unsafe {
                std::ptr::write(&raw mut *p, State::new(0, null_mut()));
            }
            f.pops += 1;
        });
    }
    extern "C" fn base(socket: *mut NetworkTCPSocketHandler) -> *mut packet::NetworkSocketHandler {
        socket.cast()
    }
    extern "C" fn create(_: *mut NetworkTCPSocketHandler, record: *mut State) -> *mut State {
        with(|f| {
            f.packet = Some(Box::new(unsafe {
                replace(record, State::new(0, null_mut()))
            }));
            &raw mut **f.packet.as_mut().unwrap()
        })
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
        Services {
            base_socket: base,
            can_send_receive: unused1,
            error_begin,
            error_end,
            error_reset: unused1,
            error_text,
            error_would_block: blocked,
            now: unused0,
            packet: packet::test_services(),
            packet_delete_shell: unused1,
            packet_enqueue: unused2,
            packet_record: unused1,
            receive_bytes,
            receive_create: create,
            receive_packet: packet,
            receive_take: unused1,
            send_bytes,
            send_front: packet,
            send_pop,
            valid,
            writable,
        }
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
                packet: Some(Box::new(p)),
                result,
                blocked,
                valid,
                writable,
                ..Fixture::default()
            });
            let sent = unsafe { openttd_rust_tcp_send(&services(), null_mut(), closing, check) };
            assert_eq!(sent as u8, expected as u8);
            with(|f| {
                assert_eq!((f.calls, f.logs, f.pops), (calls, logs, pops));
                if let Some(p) = &f.packet {
                    assert_eq!(p.position, cursor);
                }
                assert_eq!(f.output, [7, 0, 9, 1, 2, 3, 4][..f.output.len()]);
            });
        }
        *FIXTURE.lock().unwrap() = None;
    }
    #[test]
    fn fragmented_headers_resume_and_invalid_sizes_request_close() {
        let _entry = ENTRY.lock().unwrap();
        *FIXTURE.lock().unwrap() = Some(Fixture {
            valid: true,
            blocked: true,
            input: vec![7, 0, 9, 1, 2, 3, 4],
            read_limit: 1,
            read_end: -1,
            ..Fixture::default()
        });
        let socket = std::ptr::dangling_mut::<NetworkTCPSocketHandler>();
        let s = services();
        assert_eq!(
            unsafe { openttd_rust_tcp_receive(&s, socket, check) } as u8,
            Receive::None as u8
        );
        with(|f| {
            assert_eq!(f.packet.as_ref().unwrap().position, 1);
            f.read_limit = f.input.len();
        });
        assert_eq!(
            unsafe { openttd_rust_tcp_receive(&s, socket, check) } as u8,
            Receive::Ready as u8
        );
        with(|f| {
            let p = f.packet.as_ref().unwrap();
            assert_eq!(p.position, 7);
            assert_eq!(
                unsafe { std::slice::from_raw_parts(p.data, p.size) },
                f.input
            );
        });
        for input in [vec![1, 0], vec![0, 128], vec![7, 0]] {
            *FIXTURE.lock().unwrap() = Some(Fixture {
                valid: true,
                input: input.clone(),
                read_limit: input.len(),
                ..Fixture::default()
            });
            assert_eq!(
                unsafe { openttd_rust_tcp_receive(&s, socket, check) } as u8,
                Receive::Close as u8
            );
        }
        *FIXTURE.lock().unwrap() = None;
    }
}
