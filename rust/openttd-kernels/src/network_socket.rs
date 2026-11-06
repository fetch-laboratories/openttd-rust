/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical game-socket protocol fields. Native shells install stable identity and resource
//! links at construction. They live until deferred destruction; raw field access ends before
//! callbacks or deletion. The game-state mutex serializes visits and pool mutation.
#![allow(unsafe_code, missing_docs)]
use crate::network_command::CommandQueue;
use crate::network_frame::SocketSync;
use crate::network_tcp::NetworkTCPSocketHandler;
use crate::packet::NetworkSocketHandler;
use crate::raw_field::{read_field, write_field};
use std::ffi::c_void;
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDNetworkGameSocket" ["probe"] => pub struct GameSocket {
        pub base: *mut NetworkSocketHandler,
        pub client_id: u32,
        pub game: *mut c_void,
        pub incoming: CommandQueue,
        pub info: *mut c_void,
        pub last_packet_ns: i64,
        pub map_reader: *mut crate::network_map::MapReader,
        pub map_writer: *mut crate::network_map::MapWriter,
        pub outgoing: CommandQueue,
        pub pending: bool,
        /// Captured only in the initialized pooled server constructor body; MAX is nonpooled.
        pub pool_index: usize,
        pub role: u8,
        pub server: *mut c_void,
        pub status: u8,
        pub sync: SocketSync,
        pub tcp: *mut NetworkTCPSocketHandler,
    }
}
pub(crate) fn shell(socket: *mut GameSocket) -> *mut c_void {
    read_field!(socket, game)
}
pub(crate) fn server(socket: *mut GameSocket) -> *mut c_void {
    read_field!(socket, server)
}
pub(crate) fn base(socket: *mut GameSocket) -> *mut NetworkSocketHandler {
    read_field!(socket, base)
}
pub(crate) fn tcp(socket: *mut GameSocket) -> *mut NetworkTCPSocketHandler {
    read_field!(socket, tcp)
}
pub(crate) fn sync(socket: *mut GameSocket) -> *mut SocketSync {
    unsafe { &raw mut (*socket).sync }
}
pub(crate) fn incoming(socket: *mut GameSocket) -> *mut CommandQueue {
    unsafe { &raw mut (*socket).incoming }
}
pub(crate) fn outgoing(socket: *mut GameSocket) -> *mut CommandQueue {
    unsafe { &raw mut (*socket).outgoing }
}
pub(crate) fn client_id(socket: *mut GameSocket) -> u32 {
    read_field!(socket, client_id)
}
pub(crate) fn info(socket: *mut GameSocket) -> *mut c_void {
    read_field!(socket, info)
}
pub(crate) fn status(socket: *mut GameSocket) -> u8 {
    read_field!(socket, status)
}
pub(crate) fn pending(socket: *mut GameSocket) -> bool {
    read_field!(socket, pending)
}
pub(crate) fn last_packet(socket: *mut GameSocket) -> i64 {
    read_field!(socket, last_packet_ns)
}
pub(crate) fn set_last_packet(socket: *mut GameSocket, time: i64) {
    write_field!(socket, last_packet_ns, time);
}

/// Native MI base construction, before any derived pooled coordinate is available.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_game_new(
    base: *mut NetworkSocketHandler,
    tcp: *mut NetworkTCPSocketHandler,
    game: *mut c_void,
    role: u8,
    frame: u32,
    time: i64,
) -> *mut GameSocket {
    Box::into_raw(Box::new(GameSocket {
        base,
        client_id: 0,
        game,
        incoming: CommandQueue::EMPTY,
        info: std::ptr::null_mut(),
        last_packet_ns: time,
        map_reader: std::ptr::null_mut(),
        map_writer: std::ptr::null_mut(),
        outgoing: CommandQueue::EMPTY,
        pending: false,
        pool_index: usize::MAX,
        role,
        server: std::ptr::null_mut(),
        status: 0,
        sync: SocketSync {
            receive_limit: 0,
            last_frame: frame,
            last_frame_server: frame,
            last_token_frame: 0,
            lag_test: 0,
            last_token: 0,
            token: 0,
        },
        tcp,
    }))
}

/// Original server outgoing member destruction, before native peer-key/auth members.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_server_queue_drop(socket: *mut GameSocket) {
    unsafe { (*socket).outgoing = CommandQueue::EMPTY };
}

/// The indexed pool binding is retired; incoming payloads precede TCP base destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_game_drop(socket: *mut GameSocket) {
    unsafe { drop(Box::from_raw(socket)) };
}

use crate::game_state::GameState;
use crate::network_action::Actions;
use crate::network_frame::Services;
use crate::network_type::{NETWORK_RECV_STATUS_CLIENT_QUIT, NETWORK_RECV_STATUS_CONNECTION_LOST};

static DEFERRED: GameState<Vec<*mut GameSocket>> = GameState::new(Vec::new());

/// `DeferDeletion`: queue ownership before publishing the pending flag.
pub(crate) fn defer(socket: *mut GameSocket) {
    unsafe { (*DEFERRED.get()).push(socket) };
    write_field!(socket, pending, true);
}

/// The native deleter may reenter Rust; no vector borrow crosses it. `clear` retains capacity.
fn delete_deferred(delete: extern "C" fn(*mut c_void)) {
    let count = unsafe { (*DEFERRED.get()).len() };
    for index in 0..count {
        let socket = unsafe { *(*DEFERRED.get()).as_ptr().add(index) };
        delete(shell(socket));
    }
    unsafe { (*DEFERRED.get()).clear() };
}

pub(crate) fn process_deferred(s: &Services) {
    delete_deferred(s.socket_delete);
}

pub(crate) fn specialized_close(
    s: &Services,
    socket: *mut GameSocket,
    result: u8,
    work: Actions,
) -> u8 {
    match read_field!(socket, role) {
        0 => crate::network_client::openttd_rust_network_client_close(s.client, s, socket, result),
        1 => crate::network_connection::openttd_rust_network_server_close(
            s.connection,
            s,
            socket,
            result,
            work,
        ),
        _ => (s.query_close)(shell(socket), result),
    }
}

/// `NetworkGameSocketHandler::CloseConnection(bool)`.
pub(crate) fn close(s: &Services, socket: *mut GameSocket, work: Actions) -> u8 {
    work.network(s);
    let result =
        if !(s.command.company.network_server)() && (s.command.company.finance.networking)() {
            (s.client.emergency_save)();
            crate::session::set_pending(crate::openttd::SwitchMode::MENU);
            (s.lifecycle.set_networking)(false);
            (s.show_lost_connection)();
            NETWORK_RECV_STATUS_CLIENT_QUIT
        } else {
            NETWORK_RECV_STATUS_CONNECTION_LOST
        };
    specialized_close(s, socket, result, work)
}

/// Direct common TCP send; virtual game close stays in this owner.
pub(crate) fn send(
    s: &Services,
    socket: *mut GameSocket,
    work: Actions,
) -> crate::network_tcp::Send {
    unsafe {
        crate::network_tcp::send_packets(
            s.tcp,
            tcp(socket),
            false,
            crate::network_frame::packet_check(s),
            || {
                close(s, socket, work);
            },
        )
    }
}

pub(crate) fn send_closing(s: &Services, socket: *mut GameSocket) {
    unsafe {
        crate::network_tcp::openttd_rust_tcp_send(
            s.tcp,
            tcp(socket),
            true,
            crate::network_frame::packet_check(s),
        )
    };
}

/// # Safety
/// `socket` is live through the native action driver's completion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_game_close(
    s: &Services,
    socket: *mut GameSocket,
    work: Actions,
) -> u8 {
    close(s, socket, work)
}
/// # Safety
/// `socket` transfers ownership once, until the next deferred-deletion pass.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_defer(socket: *mut GameSocket) {
    defer(socket);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_process_deferred(s: &Services) {
    process_deferred(s);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_deferred_shutdown(delete: extern "C" fn(*mut c_void)) {
    delete_deferred(delete);
    unsafe { *DEFERRED.get() = Vec::new() };
}

/// `NetworkSend`: admin protocol first, then game sockets and deferred destruction.
pub(crate) fn send_all(
    s: &Services,
    frame: *mut crate::session::Frame,
    work: Actions,
) -> Result<(), ()> {
    if (s.command.company.network_server)() {
        if !(s.admins_send)(frame) {
            return Err(());
        }
        send_clients(s, 0, work);
        if work.pending() {
            return Err(());
        }
    } else {
        send(s, (s.client_game)(), work);
        let client = (s.client_game)();
        if !client.is_null() {
            crate::network_client::check_connection(s.client, s, client, work);
        }
        process_deferred(s);
    }
    Ok(())
}

/// `ServerNetworkGameSocketHandler::Send`, with pool iteration resumed after map saves.
pub(crate) fn send_clients(s: &Services, mut cursor: usize, work: Actions) {
    while let Some(client) = crate::network_command::next_client(s.command, &mut cursor) {
        if !unsafe { (*tcp(client.socket)).writable } {
            continue;
        }
        let sent = send(s, client.socket, work);
        if work.pending() {
            work.push(crate::network_action::Resume::SendClients(cursor));
            return;
        }
        if sent != crate::network_tcp::Send::Closed
            && status(client.socket) == crate::network_frame::server_status::MAP
        {
            crate::network_connection::send_map(s.connection, s, client.socket, work);
            if work.pending() {
                work.push(crate::network_action::Resume::SendClients(cursor));
                return;
            }
        }
    }
    process_deferred(s);
}

#[repr(u8)]
#[expect(dead_code, reason = "Constructed by the native select callback.")]
pub enum Poll {
    Error,
    Ready,
    Exception,
}

/// `NetworkReceive`: admin protocol precedes game sockets, then deferred destruction.
pub(crate) fn receive_all(
    session: &crate::session::Services,
    s: &Services,
    frame: *mut crate::session::Frame,
    work: Actions,
) -> Result<bool, ()> {
    work.bind(s, session, frame);
    if (s.command.company.network_server)() {
        if !(s.admins_receive)(frame) {
            return Err(());
        }
        match (s.server_poll)(frame) {
            Poll::Exception => return Err(()),
            Poll::Error => return Ok(receive_finish(s, frame, true, false, work)),
            Poll::Ready => (),
        }
        loop {
            match (s.server_accept_next)(frame) {
                Poll::Exception => return Err(()),
                Poll::Error => break,
                Poll::Ready => {
                    let id = crate::network_connection::openttd_rust_network_next_client_id();
                    if !(s.server_accept)(frame, id) {
                        return Err(());
                    }
                }
            }
        }
        receive_clients(session, s, frame, 0, work)
    } else {
        let game = (s.client_game)();
        if unsafe {
            crate::network_tcp::can_receive(s.tcp, tcp(game), crate::network_frame::packet_check(s))
        } {
            let result = crate::network_dispatch::receive_packets(session, s, game, frame, work);
            if work.pending() {
                work.push(crate::network_action::Resume::ClientReceived);
                return Err(());
            }
            result?;
            Ok(client_received(s, frame, work))
        } else {
            let networking = (s.command.company.finance.networking)();
            Ok(receive_finish(s, frame, false, networking, work))
        }
    }
}

pub(crate) fn receive_clients(
    session: &crate::session::Services,
    s: &Services,
    frame: *mut crate::session::Frame,
    mut cursor: usize,
    work: Actions,
) -> Result<bool, ()> {
    while let Some(client) = crate::network_command::next_client(s.command, &mut cursor) {
        let ready = (s.server_poll_client)(frame, client.socket);
        write_field!(tcp(client.socket), writable, ready & 2 != 0);
        if ready & 1 == 0 {
            continue;
        }
        let result =
            crate::network_dispatch::receive_packets(session, s, client.socket, frame, work);
        if work.pending() {
            work.push(crate::network_action::Resume::ReceiveClients(cursor));
            return Err(());
        }
        result?;
    }
    let networking = (s.command.company.finance.networking)();
    Ok(receive_finish(s, frame, true, networking, work))
}

pub(crate) fn client_received(
    s: &Services,
    frame: *mut crate::session::Frame,
    work: Actions,
) -> bool {
    let result = work.result();
    if result != crate::network_type::NETWORK_RECV_STATUS_OKAY {
        crate::network_client::client_error(s.client, s, (s.client_game)(), result);
        return receive_finish(s, frame, false, false, work);
    }
    let networking = (s.command.company.finance.networking)();
    receive_finish(s, frame, false, networking, work)
}

fn receive_finish(
    s: &Services,
    frame: *mut crate::session::Frame,
    server: bool,
    result: bool,
    work: Actions,
) -> bool {
    if server {
        (s.server_poll_end)(frame);
    }
    process_deferred(s);
    work.set_result(u8::from(result));
    result
}
