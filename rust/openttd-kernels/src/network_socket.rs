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
        pub base: *mut NetworkSocketHandler => class,
        pub game: *mut c_void,
        pub incoming: *mut CommandQueue,
        pub last_packet_ns: i64,
        pub outgoing: *mut CommandQueue,
        pub pending: bool,
        pub role: u8,
        pub server: *mut c_void,
        pub status: u8,
        pub sync: *mut SocketSync,
        pub tcp: *mut NetworkTCPSocketHandler => class,
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
    read_field!(socket, sync)
}
pub(crate) fn incoming(socket: *mut GameSocket) -> *mut CommandQueue {
    read_field!(socket, incoming)
}
pub(crate) fn outgoing(socket: *mut GameSocket) -> *mut CommandQueue {
    read_field!(socket, outgoing)
}
/// The status byte, whose meaning depends on the role: `network_client::status` and
/// `network_server::status` are its typed forms.
pub(crate) fn status(socket: *mut GameSocket) -> u8 {
    read_field!(socket, status)
}
/// Stores the status byte; `network_client::set_status` and `network_server::set_status` are
/// its typed forms.
pub(crate) fn set_status(socket: *mut GameSocket, value: u8) {
    write_field!(socket, status, value);
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

use crate::game_state::GameState;
use crate::network_action::Actions;
use crate::network_frame::Services;
use crate::network_type::NetworkRecvStatus;

/// The `role` byte: which `NetworkGameSocketHandler` subclass the native shell is, standing for
/// the original's virtual `CloseConnection(NetworkRecvStatus)` and receive handlers.
pub(crate) mod role {
    /// `ClientNetworkGameSocketHandler`.
    pub(crate) const CLIENT: u8 = 0;
    /// `ServerNetworkGameSocketHandler`.
    pub(crate) const SERVER: u8 = 1;
    /// `QueryNetworkGameSocketHandler`.
    pub(crate) const QUERY: u8 = 2;
}

/// `_deferred_deletions`: the original's file-static vector of sockets awaiting deletion.
static DEFERRED: GameState<Vec<*mut GameSocket>> = GameState::new(Vec::new());

/// `DeferDeletion`: queue ownership before publishing the pending flag.
pub(crate) fn defer(socket: *mut GameSocket) {
    unsafe { (*DEFERRED.get()).push(socket) };
    write_field!(socket, pending, true);
}

/// The native deleter may reenter Rust; no vector borrow crosses it. `clear` retains capacity.
fn delete_deferred(delete: extern "C" fn(*mut c_void)) {
    /* Calls deleter on all items */
    let count = unsafe { (*DEFERRED.get()).len() };
    for index in 0..count {
        let socket = unsafe { *(*DEFERRED.get()).as_ptr().add(index) };
        delete(shell(socket));
    }
    unsafe { (*DEFERRED.get()).clear() };
}

/// `NetworkGameSocketHandler::ProcessDeferredDeletions`.
pub(crate) fn process_deferred(s: &Services) {
    delete_deferred(s.socket_delete);
}

/// The destruction of `_deferred_deletions` at exit: delete the remaining sockets, then release
/// the vector's capacity.
fn deferred_shutdown(delete: extern "C" fn(*mut c_void)) {
    delete_deferred(delete);
    unsafe { *DEFERRED.get() = Vec::new() };
}

/// The virtual `NetworkGameSocketHandler::CloseConnection(NetworkRecvStatus)`, dispatched on
/// the socket's role; the query socket's override stays native.
pub(crate) fn specialized_close(
    s: &Services,
    socket: *mut GameSocket,
    status: NetworkRecvStatus,
    work: Actions,
) -> NetworkRecvStatus {
    match read_field!(socket, role) {
        role::CLIENT => crate::network_client::close_connection(s.client, s, socket, status),
        role::SERVER => {
            crate::network_connection::server_close(s.connection, s, socket, status, work)
        }
        _ => NetworkRecvStatus((s.query_close)(shell(socket), status.0)),
    }
}

/// `NetworkGameSocketHandler::CloseConnection(bool)`: a socket can make errors. When that
/// happens this handles what to do. For clients: close connection and drop back to main-menu.
/// For servers: close connection and that is it.
pub(crate) fn close_connection(
    s: &Services,
    socket: *mut GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(s);
    /* Clients drop back to the main menu */
    if !(s.command.company.network_server)() && (s.command.company.finance.networking)() {
        (s.client.emergency_save)();
        crate::session::set_pending(crate::openttd::SwitchMode::MENU);
        (s.lifecycle.set_networking)(false);
        (s.show_lost_connection)();

        return specialized_close(
            s,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
            work,
        );
    }

    specialized_close(
        s,
        socket,
        NetworkRecvStatus::NETWORK_RECV_STATUS_CONNECTION_LOST,
        work,
    )
}

/// Direct common TCP send; virtual game close stays in this owner.
pub(crate) fn send(
    s: &Services,
    socket: *mut GameSocket,
    work: Actions,
) -> crate::network_tcp::Send {
    let result = unsafe {
        crate::network_tcp::send_packets(
            crate::network_tcp::Context {
                services: s.tcp,
                socket: tcp(socket),
            },
            false,
            crate::network_frame::packet_check(s),
        )
    };
    if result == crate::network_tcp::Send::Close {
        close_connection(s, socket, work);
        return crate::network_tcp::Send::Closed;
    }
    result
}

pub(crate) fn send_closing(s: &Services, socket: *mut GameSocket) {
    unsafe {
        crate::network_tcp::send_packets(
            crate::network_tcp::Context {
                services: s.tcp,
                socket: tcp(socket),
            },
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
    close_connection(s, socket, work).0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_process_deferred(s: &Services) {
    process_deferred(s);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_deferred_shutdown(delete: extern "C" fn(*mut c_void)) {
    deferred_shutdown(delete);
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
        if !(s.tcp.writable)(tcp(client.socket)) {
            continue;
        }
        let sent = send(s, client.socket, work);
        if work.pending() {
            work.push(crate::network_action::Resume::SendClients(cursor));
            return;
        }
        if sent != crate::network_tcp::Send::Closed
            && crate::network_server::status(client.socket)
                == crate::network_server::ClientStatus::STATUS_MAP
        {
            /* This client is in the middle of a map-send, call the function for that */
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
                    let id = crate::network_connection::next_client_id().0;
                    if !(s.server_accept)(frame, id) {
                        return Err(());
                    }
                }
            }
        }
        receive_clients(session, s, frame, 0, work)
    } else {
        let game = (s.client_game)();
        if (s.tcp.can_send_receive)(tcp(game)) {
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
        if !(s.server_poll_client)(frame, client.socket) {
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

/// `ClientNetworkGameSocketHandler::Receive` after its `ReceivePackets`, then `NetworkReceive`'s
/// deferred deletions.
pub(crate) fn client_received(
    s: &Services,
    frame: *mut crate::session::Frame,
    work: Actions,
) -> bool {
    let res = work.status();
    if res != NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY {
        /* The client made an error of which we can not recover.
         * Close the connection and drop back to the main menu. */
        crate::network_client::client_error(s.client, s, (s.client_game)(), res);
        return receive_finish(s, frame, false, false, work);
    }
    let networking = (s.command.company.finance.networking)();
    receive_finish(s, frame, false, networking, work)
}

/// `NetworkReceive`'s tail: the server's poll end, deferred deletions and the receive result.
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
    work.set_received(result);
    result
}
