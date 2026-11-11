/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Connection admission and map pacing. Socket and client-info pools and save filters stay
//! native. Fields are accessed at their original observation points; no borrow survives a leaf.
#![allow(unsafe_code, missing_docs)]
use crate::company_type::CompanyID;
use crate::debug::{Bytes, debug};
use crate::network_action::{Actions, Resume};
use crate::network_command::next_client;
use crate::network_frame::Services as FrameServices;
use crate::network_frame::{
    packet_check, packet_new, packet_recv_string, packet_send, packet_send_string,
};
use crate::network_server::{self as server, ClientStatus};
use crate::network_socket as socket;
use crate::network_type::{ClientID, NetworkErrorCode, NetworkRecvStatus};
use crate::openttd::{PauseMode, PauseModes};
use crate::packet::{self, State as PacketState};
use crate::raw_field::write_field;
use crate::tcp_game::PacketGameType;
use std::ffi::c_void;

pub struct PacketWriter {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Services {
    pub admin_error: extern "C" fn(*mut c_void, u8),
    pub admin_joined: extern "C" fn(*mut c_void),
    pub admin_quit: extern "C" fn(*mut c_void),
    pub allocate_info: extern "C" fn(*mut c_void, *mut c_void, u8),
    pub auth:
        extern "C" fn(*mut c_void) -> *const crate::network_crypto::NetworkAuthenticationHandler,
    pub auth_create: extern "C" fn(*mut c_void),
    pub client_company: extern "C" fn(*mut c_void) -> u8,
    pub client_error_announcement: extern "C" fn(*mut c_void, u8),
    pub clients_connected: *mut u8,
    pub clients_on: *mut u8,
    pub company_allows_client: extern "C" fn(u8, *mut c_void) -> bool,
    pub enable_encryption:
        extern "C" fn(*mut c_void, *const u8, *mut crate::auth::Stream, *mut crate::auth::Stream),
    pub error_announcement: extern "C" fn(*mut c_void, u32, crate::services::ByteSpan, bool, bool),
    pub error_message: extern "C" fn(u8) -> u32,
    pub grf: &'static crate::newgrf_loading::Services,
    pub info: extern "C" fn(*mut c_void) -> *mut c_void,
    pub join_date: extern "C" fn(*mut c_void) -> i32,
    pub joined: extern "C" fn(*mut c_void),
    pub lost_connection_announcement: extern "C" fn(*mut c_void),
    pub map_begin: extern "C" fn(*mut c_void) -> *mut PacketWriter,
    pub map_create: extern "C" fn(*mut c_void),
    pub map_destroy: extern "C" fn(*mut c_void),
    pub map_end: extern "C" fn(*mut PacketWriter),
    pub map_enqueue: extern "C" fn(*mut PacketWriter),
    pub map_front: extern "C" fn(*mut PacketWriter) -> *mut crate::network_tcp::Packet,
    pub max_clients: extern "C" fn() -> u8,
    pub max_companies: extern "C" fn() -> u8,
    pub min_active_clients: extern "C" fn() -> u8,
    pub newgrf_version: extern "C" fn() -> u32,
    pub pause_on_join: extern "C" fn() -> bool,
    pub quit_announcement: extern "C" fn(*mut c_void),
    pub revision_compatible: extern "C" fn(*mut c_void) -> bool,
    pub send_client_info: extern "C" fn(*mut c_void, *mut c_void) -> u8,
    pub send_config: extern "C" fn(*mut c_void) -> u8,
    pub send_game_info: extern "C" fn(*mut c_void) -> u8,
    pub unique_name: extern "C" fn(*mut c_void) -> bool,
    pub valid_name: extern "C" fn(*mut c_void) -> bool,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkConnectionServices" ["probe"] => Services {
        admin_error,
        admin_joined,
        admin_quit,
        allocate_info,
        auth,
        auth_create,
        client_company,
        client_error_announcement,
        clients_connected,
        clients_on,
        company_allows_client,
        enable_encryption,
        error_announcement,
        error_message,
        grf,
        info,
        join_date,
        joined,
        lost_connection_announcement,
        map_begin,
        map_create,
        map_destroy,
        map_end,
        map_enqueue,
        map_front,
        max_clients,
        max_companies,
        min_active_clients,
        newgrf_version,
        pause_on_join,
        quit_announcement,
        revision_compatible,
        send_client_info,
        send_config,
        send_game_info,
        unique_name,
        valid_name,
    };
}

// Native sockets and their fields remain live until their original deferred-deletion point.

/// Reset 'lag' counters: `last_frame = last_frame_server = _frame_counter`.
pub(crate) fn reset_lag(socket: *mut socket::GameSocket) {
    let sync = socket::sync(socket);
    write_field!(sync, last_frame, crate::network_frame::frame_counter());
    write_field!(
        sync,
        last_frame_server,
        crate::network_frame::frame_counter()
    );
}

/// `CheckPauseHelper`: helper function for the pause checkers. If pause is true and the
/// current pause mode isn't set the game will be paused, if it it false
/// and the pause mode is set the game will be unpaused. In the other
/// cases nothing happens to the pause state.
/// `pause` is whether we'd like to pause and `pm` the mode which we would like to pause with.
fn check_pause_helper(session: &crate::session::Services, pause: bool, pm: PauseMode) {
    if pause == PauseModes(crate::session::pause()).test(pm) {
        return;
    }

    crate::command_post::pause(unsafe { &*session.shared }, pm, pause);
}

/// `NetworkCountActiveClients`: counts the number of active clients connected.
/// It has to be in `STATUS_ACTIVE` and not a spectator.
fn network_count_active_clients(f: &FrameServices) -> u32 {
    let s = f.connection;
    let mut count = 0_u32;

    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if server::status(client.socket) != ClientStatus::STATUS_ACTIVE {
            continue;
        }
        if !(f.command.company.shared.company_valid)((s.client_company)(socket::server(
            client.socket,
        ))) {
            continue;
        }
        count += 1;
    }

    count
}

/// `CheckMinActiveClients`: check if the minimum number of active clients has been reached and
/// pause or unpause the game as appropriate.
pub(crate) fn check_min_active_clients(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause_mode = PauseModes(crate::session::pause());
    if pause_mode.test(PauseMode::Error)
        || !(unsafe { &*(session.world)() }.dedicated)()
        || ((s.min_active_clients)() == 0 && !pause_mode.test(PauseMode::ActiveClients))
    {
        return;
    }
    check_pause_helper(
        session,
        network_count_active_clients(f) < u32::from((s.min_active_clients)()),
        PauseMode::ActiveClients,
    );
}

/// `NetworkHasJoiningClient`: whether one client is joining (but not authorizing).
fn network_has_joining_client(f: &FrameServices) -> bool {
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED
            && server::status(client.socket) < ClientStatus::STATUS_ACTIVE
        {
            return true;
        }
    }

    false
}

/// `CheckPauseOnJoin`: check whether we should pause on join.
pub(crate) fn check_pause_on_join(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause_mode = PauseModes(crate::session::pause());
    if pause_mode.test(PauseMode::Error)
        || (!(s.pause_on_join)() && !pause_mode.test(PauseMode::Join))
    {
        return;
    }
    check_pause_helper(session, network_has_joining_client(f), PauseMode::Join);
}

/// `ServerNetworkGameSocketHandler::SendWelcome`: send the client a welcome message with some
/// basic information.
pub(crate) fn send_welcome(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendWelcome()",
        (f.dispatch.client_id)(socket)
    );

    /* Invalid packet when status is anything but STATUS_NEWGRFS_CHECK. */
    if server::status(socket) != ClientStatus::STATUS_NEWGRFS_CHECK {
        return close_connection(
            s,
            f,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] status = AUTHORIZED",
        (f.dispatch.client_id)(socket)
    );
    server::set_status(socket, ClientStatus::STATUS_AUTHORIZED);

    /* Reset 'lag' counters */
    reset_lag(socket);

    unsafe { s.clients_on.write(s.clients_on.read().wrapping_add(1)) };

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_WELCOME);
    p.send_uint32((f.dispatch.client_id)(socket), packet_check(f));
    packet_send(f, socket, &mut p);

    /* Transmit info about all the active clients */
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket != socket
            && server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED
        {
            (s.send_client_info)(
                socket::server(socket),
                (s.info)(socket::server(client.socket)),
            );
        }
    }
    /* Also send the info of the server */
    NetworkRecvStatus((s.send_client_info)(
        socket::server(socket),
        (f.command.company.shared.network_client_info)(ClientID::CLIENT_ID_SERVER.0),
    ))
}

/// `ServerNetworkGameSocketHandler::SendWait`: tell the client that its put in a waiting queue.
pub(crate) fn send_wait(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendWait()",
        (f.dispatch.client_id)(socket)
    );

    let mut waiting = 1_u32; // current player getting the map counts as 1

    /* Count how many clients are waiting in the queue, in front of you! */
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if server::status(client.socket) != ClientStatus::STATUS_MAP_WAIT {
            continue;
        }
        if (s.join_date)(socket::server(client.socket)) < (s.join_date)(socket::server(socket))
            || ((s.join_date)(socket::server(client.socket))
                == (s.join_date)(socket::server(socket))
                && (f.dispatch.client_id)(client.socket) < (f.dispatch.client_id)(socket))
        {
            waiting += 1;
        }
    }

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_WAIT);
    #[allow(clippy::cast_possible_truncation)]
    p.send_uint8(waiting as u8, packet_check(f));
    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::CheckNextClientToSendMap`.
pub(crate) fn check_next_client_to_send_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    ignore_cs: *mut socket::GameSocket,
    work: Actions,
) {
    debug!(
        net,
        9,
        "client[{}] CheckNextClientToSendMap()",
        (f.dispatch.client_id)(socket)
    );

    /* Find the best candidate for joining, i.e. the first joiner. */
    let mut best: *mut socket::GameSocket = std::ptr::null_mut();
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if ignore_cs == client.socket {
            continue;
        }

        if server::status(client.socket) == ClientStatus::STATUS_MAP_WAIT
            && (best.is_null()
                || (s.join_date)(socket::server(best))
                    > (s.join_date)(socket::server(client.socket))
                || ((s.join_date)(socket::server(best))
                    == (s.join_date)(socket::server(client.socket))
                    && (f.dispatch.client_id)(best) > (f.dispatch.client_id)(client.socket)))
        {
            best = client.socket;
        }
    }

    /* Is there someone else to join? */
    if !best.is_null() {
        /* Let the first start joining. */
        server::set_status(best, ClientStatus::STATUS_AUTHORIZED);
        send_map(s, f, best, work);
        if work.pending() {
            work.push(Resume::Waits(1));
            return;
        }

        /* And update the rest. */
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if server::status(client.socket) == ClientStatus::STATUS_MAP_WAIT {
                send_wait(s, f, client.socket);
            }
        }
    }
}

/// `CheckNextClientToSendMap`'s resumed "update the rest" loop, `count` times.
pub(crate) fn update_waiters(f: &FrameServices, count: usize) {
    for _ in 0..count {
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if server::status(client.socket) == ClientStatus::STATUS_MAP_WAIT {
                send_wait(f.connection, f, client.socket);
            }
        }
    }
}

/// `ServerNetworkGameSocketHandler::SendMap`: this sends the map to the client. The save runs
/// as a deferred action after the first packet, which then resumes in `map_transfer`.
pub(crate) fn send_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    if server::status(socket) < ClientStatus::STATUS_AUTHORIZED {
        /* Illegal call, return error and ignore the packet */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NOT_AUTHORIZED,
            EMPTY_TEXT,
            work,
        );
    }

    if server::status(socket) == ClientStatus::STATUS_AUTHORIZED {
        debug!(
            net,
            9,
            "client[{}] SendMap(): first_packet",
            (f.dispatch.client_id)(socket)
        );

        (s.map_create)(socket::server(socket));

        /* Now send the _frame_counter and how many packets are coming */
        let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_MAP_BEGIN);
        p.send_uint32(crate::network_frame::frame_counter(), packet_check(f));
        packet_send(f, socket, &mut p);

        unsafe {
            crate::network_command::sync_command_queue(socket::outgoing(socket));
        };
        debug!(
            net,
            9,
            "client[{}] status = MAP",
            (f.dispatch.client_id)(socket)
        );
        server::set_status(socket, ClientStatus::STATUS_MAP);
        /* Mark the start of download */
        reset_lag(socket);

        /* Make a dump of the current game */
        work.save(socket);
        return NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY;
    }

    map_transfer(f, socket, work);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `PacketWriter::TransferToNetworkQueue`: transfer all packets from here to the network's queue
/// while holding the lock on our mutex; its native mutex remains locked until `map_end`.
/// True iff the last packet of the map has been sent.
fn transfer_to_network_queue(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> bool {
    /* Unsafe check for the queue being empty or not. */
    let writer = (s.map_begin)(socket::server(socket));
    if writer.is_null() {
        return false;
    }

    let mut last_packet = false;
    loop {
        let packet = (s.map_front)(writer);
        if packet.is_null() {
            break;
        }
        let record = (f.tcp.packet_record)(packet);
        last_packet =
            PacketGameType(unsafe { packet::get_type(record, f.packet, packet_check(f)) })
                == PacketGameType::PACKET_SERVER_MAP_DONE;
        unsafe {
            packet::prepare_to_send(record, f.packet, packet_check(f));
        }
        (s.map_enqueue)(writer);

        if last_packet {
            break;
        }
    }
    (s.map_end)(writer);

    last_packet
}

/// `ServerNetworkGameSocketHandler::SendMap`'s `STATUS_MAP` transfer step.
pub(crate) fn map_transfer(f: &FrameServices, socket: *mut socket::GameSocket, work: Actions) {
    let s = f.connection;
    if server::status(socket) == ClientStatus::STATUS_MAP && transfer_to_network_queue(s, f, socket)
    {
        debug!(
            net,
            9,
            "client[{}] SendMap(): last_packet",
            (f.dispatch.client_id)(socket)
        );

        /* Done reading, make sure saving is done as well */
        (s.map_destroy)(socket::server(socket));

        /* Set the status to DONE_MAP, no we will wait for the client
         *  to send it is ready (maybe that happens like never ;)) */
        debug!(
            net,
            9,
            "client[{}] status = DONE_MAP",
            (f.dispatch.client_id)(socket)
        );
        server::set_status(socket, ClientStatus::STATUS_DONE_MAP);

        check_next_client_to_send_map(s, f, socket, std::ptr::null_mut(), work);
    }
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_GETMAP`.
pub(crate) fn receive_getmap(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    /* The client was never joined.. so this is impossible, right?
     *  Ignore the packet, give the client a warning, and close the connection */
    if server::status(socket) < ClientStatus::STATUS_AUTHORIZED
        || crate::network_frame::socket_has_quit(f, socket)
    {
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NOT_AUTHORIZED,
            EMPTY_TEXT,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_GETMAP()",
        (f.dispatch.client_id)(socket)
    );

    /* Check if someone else is receiving the map */
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if server::status(client.socket) == ClientStatus::STATUS_MAP {
            /* Tell the new client to wait */
            debug!(
                net,
                9,
                "client[{}] status = MAP_WAIT",
                (f.dispatch.client_id)(socket)
            );
            server::set_status(socket, ClientStatus::STATUS_MAP_WAIT);
            return send_wait(s, f, socket);
        }
    }

    /* We receive a request to upload the map.. give it to the client! */
    send_map(s, f, socket, work)
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_MAP_OK`.
pub(crate) fn receive_map_ok(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    /* Client has the map, now start syncing */
    if server::status(socket) == ClientStatus::STATUS_DONE_MAP
        && !crate::network_frame::socket_has_quit(f, socket)
    {
        debug!(
            net,
            9,
            "client[{}] Receive_CLIENT_MAP_OK()",
            (f.dispatch.client_id)(socket)
        );
        (s.joined)(socket::server(socket));

        /* Mark the client as pre-active, and wait for an ACK
         *  so we know it is done loading and in sync with us */
        debug!(
            net,
            9,
            "client[{}] status = PRE_ACTIVE",
            (f.dispatch.client_id)(socket)
        );
        server::set_status(socket, ClientStatus::STATUS_PRE_ACTIVE);
        unsafe {
            crate::network_command::handle_command_queue(
                f.command,
                socket,
                socket::outgoing(socket),
            );
        };
        unsafe { crate::network_frame::send_frame(f, socket, socket::sync(socket)) };
        crate::network_frame::send_sync(f, socket);

        /* This is the frame the client receives
         *  we need it later on to make sure the client is not too slow */
        reset_lag(socket);

        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED {
                (s.send_client_info)(
                    socket::server(client.socket),
                    (s.info)(socket::server(socket)),
                );
                send_join(f, client.socket, ClientID((f.dispatch.client_id)(socket)));
            }
        }

        (s.admin_joined)(socket::server(socket));

        /* also update the new client with our max values */
        return NetworkRecvStatus((s.send_config)(socket::server(socket)));
    }

    /* Wrong status for this packet, give a warning to client, and close connection */
    send_error(
        s,
        f,
        socket,
        NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED,
        EMPTY_TEXT,
        work,
    )
}

/// `ServerNetworkGameSocketHandler::SendNewGRFCheck`: send the check for the `NewGRFs`.
pub(crate) fn send_newgrf_check(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendNewGRFCheck()",
        (f.dispatch.client_id)(socket)
    );

    /* Invalid packet when status is anything but STATUS_IDENTIFY. */
    if server::status(socket) != ClientStatus::STATUS_IDENTIFY {
        return close_connection(
            s,
            f,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] status = NEWGRFS_CHECK",
        (f.dispatch.client_id)(socket)
    );
    server::set_status(socket, ClientStatus::STATUS_NEWGRFS_CHECK);

    if unsafe { (s.grf.config_at)(0) }.is_null() {
        /* There are no NewGRFs, so they're welcome. */
        return send_welcome(s, f, socket, work);
    }

    let mut p = unsafe {
        packet::new_outgoing(
            socket::base(socket),
            f.packet,
            packet_check(f),
            PacketGameType::PACKET_SERVER_CHECK_NEWGRFS.0,
            crate::network_config::TCP_MTU,
        )
    };
    let is_static = |config| {
        let flags = unsafe { (s.grf.config_flags)(config) };
        flags & (1 << crate::newgrf_config::STATIC) != 0
    };

    let mut grf_count = 0_u32;
    let mut cursor = 0;
    loop {
        let c = unsafe { (s.grf.config_at)(cursor) };
        if c.is_null() {
            break;
        }
        if !is_static(c) {
            grf_count += 1;
        }
        cursor += 1;
    }
    #[allow(clippy::cast_possible_truncation)]
    p.send_uint8(grf_count as u8, packet_check(f));

    cursor = 0;
    loop {
        let c = unsafe { (s.grf.config_at)(cursor) };
        if c.is_null() {
            break;
        }
        if !is_static(c) {
            let md5sum = unsafe { (s.grf.config_md5)(c) };
            // SAFETY: The native `GRFConfig::ident.md5sum` is a live `MD5Hash`.
            let ident = crate::newgrf_config::GRFIdentifier {
                grfid: unsafe { (s.grf.config_id)(c) },
                md5sum: unsafe { md5sum.data.cast::<crate::md5::MD5Hash>().read_unaligned() },
            };
            crate::network_game_info::serialize_grf_identifier(&mut p, &ident, packet_check(f));
        }
        cursor += 1;
    }

    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_NEWGRFS_CHECKED`.
pub(crate) fn receive_newgrfs_checked(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    if server::status(socket) != ClientStatus::STATUS_NEWGRFS_CHECK {
        /* Illegal call, return error and ignore the packet */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED,
            EMPTY_TEXT,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_NEWGRFS_CHECKED()",
        (f.dispatch.client_id)(socket)
    );

    send_welcome(s, f, socket, work)
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_GAME_INFO`.
pub(crate) fn receive_game_info(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_GAME_INFO()",
        (f.dispatch.client_id)(socket)
    );
    NetworkRecvStatus((s.send_game_info)(socket::server(socket)))
}

/// An empty `std::string_view` reason.
pub(crate) const EMPTY_TEXT: crate::services::ByteSpan = crate::services::ByteSpan {
    data: std::ptr::null(),
    size: 0,
};

/// `_network_client_id`: the identifier counter used to assign client identities.
static NETWORK_CLIENT_ID: crate::game_state::GameState<ClientID> =
    crate::game_state::GameState::new(ClientID::CLIENT_ID_FIRST);

/// `_network_client_id++` in the `ServerNetworkGameSocketHandler` constructor.
pub(crate) fn next_client_id() -> ClientID {
    // The native ClientID increment wraps without resetting when the server restarts.
    unsafe {
        let id = NETWORK_CLIENT_ID.get().read();
        NETWORK_CLIENT_ID
            .get()
            .write(ClientID(id.0.wrapping_add(1)));
        id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_next_client_id() -> u32 {
    next_client_id().0
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_JOIN`.
pub(crate) fn receive_join(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    client_revision: *mut c_void,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    if server::status(socket) != ClientStatus::STATUS_INACTIVE {
        /* Illegal call, return error and ignore the packet */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED,
            EMPTY_TEXT,
            work,
        );
    }

    if unsafe { s.clients_on.read() } >= (s.max_clients)() {
        /* Turns out we are full. Inform the user about this. */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_FULL,
            EMPTY_TEXT,
            work,
        );
    }

    packet_recv_string(
        f,
        p,
        client_revision,
        crate::network_config::NETWORK_REVISION_LENGTH,
    );
    let newgrf_version = unsafe { packet::recv_uint32(p, f.packet) };

    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_JOIN(): client_revision={}, newgrf_version={}",
        (f.dispatch.client_id)(socket),
        Bytes(unsafe {
            let mut size = 0;
            let data = (f.command.company.shared.string_data)(client_revision, &raw mut size);
            crate::utf8::borrow(data, size)
        }),
        newgrf_version
    );

    /* Check if the client has revision control enabled */
    if !(s.revision_compatible)(client_revision) || (s.newgrf_version)() != newgrf_version {
        /* Different revisions!! */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_WRONG_REVISION,
            EMPTY_TEXT,
            work,
        );
    }

    crate::network_auth::send_request(f, socket, work)
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_IDENTIFY`.
pub(crate) fn receive_identify(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    client_name: *mut c_void,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    if server::status(socket) != ClientStatus::STATUS_IDENTIFY {
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED,
            EMPTY_TEXT,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_IDENTIFY()",
        (f.dispatch.client_id)(socket)
    );

    packet_recv_string(
        f,
        p,
        client_name,
        crate::network_config::NETWORK_CLIENT_NAME_LENGTH,
    );
    let mut playas = CompanyID(unsafe { packet::recv_uint8(p, f.packet) });

    if crate::network_frame::socket_has_quit(f, socket) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT;
    }

    /* join another company does not affect these values */
    match playas {
        CompanyID::NEW_COMPANY => {
            // New company
            if (f.command.company.company_count)() >= usize::from((s.max_companies)()) {
                return send_error(
                    s,
                    f,
                    socket,
                    NetworkErrorCode::NETWORK_ERROR_FULL,
                    EMPTY_TEXT,
                    work,
                );
            }
        }
        CompanyID::SPECTATOR => {} // Spectator
        _ => {
            // Join another company (companies 1..MAX_COMPANIES (index 0..(MAX_COMPANIES-1)))
            if !(f.command.company.shared.company_human)(playas.0) {
                return send_error(
                    s,
                    f,
                    socket,
                    NetworkErrorCode::NETWORK_ERROR_COMPANY_MISMATCH,
                    EMPTY_TEXT,
                    work,
                );
            }

            if !(s.company_allows_client)(playas.0, socket::server(socket)) {
                /* When we're not authorized, just bump us to a spectator. */
                playas = CompanyID::SPECTATOR;
            }
        }
    }

    if !(s.valid_name)(client_name) {
        /* An invalid client name was given. However, the client ensures the name
         * is valid before it is sent over the network, so something went horribly
         * wrong. This is probably someone trying to troll us. */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_INVALID_CLIENT_NAME,
            EMPTY_TEXT,
            work,
        );
    }

    if !(s.unique_name)(client_name) {
        // Change name if duplicate
        /* We could not create a name for this client */
        return send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NAME_IN_USE,
            EMPTY_TEXT,
            work,
        );
    }

    (s.allocate_info)(socket::server(socket), client_name, playas.0);

    /* Make sure companies to which people try to join are not autocleaned */
    let c = (f.command.company.finance.company_finances)(playas.0);
    if !c.is_null() {
        write_field!(c, empty, 0);
    }

    send_newgrf_check(s, f, socket, work)
}

/// `ServerNetworkGameSocketHandler::SendError`: send an error to the client, and close its
/// connection. `error` is the error to disconnect for; in case of kicking a client, `reason`
/// specifies the reason for kicking the client.
pub(crate) fn send_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    mut error: NetworkErrorCode,
    reason: crate::services::ByteSpan,
    work: Actions,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendError(): error={}",
        (f.dispatch.client_id)(socket),
        error.0
    );

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_ERROR);

    p.send_uint8(error.0, packet_check(f));
    if reason.size != 0 {
        packet_send_string(f, &mut p, reason);
    }
    packet_send(f, socket, &mut p);

    let strid = (s.error_message)(error.0);

    /* Only send when the current client was in game */
    let authorized = server::status(socket) >= ClientStatus::STATUS_AUTHORIZED;
    (s.error_announcement)(
        socket::server(socket),
        strid,
        reason,
        error == NetworkErrorCode::NETWORK_ERROR_KICKED && reason.size != 0,
        authorized,
    );
    if authorized {
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED
                && client.socket != socket
            {
                /* Some errors we filter to a more general error. Clients don't have to know the real
                 *  reason a joining failed. */
                if matches!(
                    error,
                    NetworkErrorCode::NETWORK_ERROR_NOT_AUTHORIZED
                        | NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED
                        | NetworkErrorCode::NETWORK_ERROR_WRONG_REVISION
                ) {
                    error = NetworkErrorCode::NETWORK_ERROR_ILLEGAL_PACKET;
                }
                send_error_quit(
                    f,
                    client.socket,
                    ClientID((f.dispatch.client_id)(socket)),
                    error,
                );
            }
        }

        (s.admin_error)(socket::server(socket), error.0);
    }

    /* The client made a mistake, so drop the connection now! */
    close_connection(
        s,
        f,
        socket,
        NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR,
        work,
    )
}

/// `ServerNetworkGameSocketHandler::SendError` for the native socket handler.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    error: u8,
    reason: crate::services::ByteSpan,
    work: Actions,
) -> u8 {
    work.network(f);
    send_error(s, f, socket, NetworkErrorCode(error), reason, work).0
}

/// `ServerNetworkGameSocketHandler::SendErrorQuit`: tell the client another client quit with an
/// error. `client_id` is the client that quit and `errorno` the reason the client quit.
pub(crate) fn send_error_quit(
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    client_id: ClientID,
    errorno: NetworkErrorCode,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendErrorQuit(): client_id={}, errorno={}",
        (f.dispatch.client_id)(socket),
        client_id.0,
        errorno.0
    );

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_ERROR_QUIT);

    p.send_uint32(client_id.0, packet_check(f));
    p.send_uint8(errorno.0, packet_check(f));

    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::SendQuit`: tell the client another client quit.
/// `client_id` is the client that quit.
pub(crate) fn send_quit(
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    client_id: ClientID,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendQuit(): client_id={}",
        (f.dispatch.client_id)(socket),
        client_id.0
    );

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_QUIT);

    p.send_uint32(client_id.0, packet_check(f));

    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::SendJoin`: tell that a client joined.
/// `client_id` is the client that joined.
pub(crate) fn send_join(
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    client_id: ClientID,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendJoin(): client_id={}",
        (f.dispatch.client_id)(socket),
        client_id.0
    );

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_JOIN);

    p.send_uint32(client_id.0, packet_check(f));

    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::SendShutdown`: tell the client we're shutting down.
pub(crate) fn send_shutdown(
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendShutdown()",
        (f.dispatch.client_id)(socket)
    );

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_SHUTDOWN);
    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::SendNewGame`: tell the client we're starting a new game.
pub(crate) fn send_new_game(
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendNewGame()",
        (f.dispatch.client_id)(socket)
    );

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_NEWGAME);
    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_ERROR`.
pub(crate) fn receive_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    /* This packets means a client noticed an error and is reporting this
     *  to us. Display the error and report it to the other clients */
    let errorno = NetworkErrorCode(unsafe { packet::recv_uint8(p, f.packet) });

    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_ERROR(): errorno={}",
        (f.dispatch.client_id)(socket),
        errorno.0
    );

    /* The client was never joined.. thank the client for the packet, but ignore it */
    if server::status(socket) < ClientStatus::STATUS_DONE_MAP
        || crate::network_frame::socket_has_quit(f, socket)
    {
        return close_connection(
            s,
            f,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
            work,
        );
    }

    (s.client_error_announcement)(socket::server(socket), errorno.0);

    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED {
            send_error_quit(
                f,
                client.socket,
                ClientID((f.dispatch.client_id)(socket)),
                errorno,
            );
        }
    }

    (s.admin_error)(socket::server(socket), errorno.0);

    close_connection(
        s,
        f,
        socket,
        NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
        work,
    )
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_QUIT`.
pub(crate) fn receive_quit(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    /* The client was never joined.. thank the client for the packet, but ignore it */
    if server::status(socket) < ClientStatus::STATUS_DONE_MAP
        || crate::network_frame::socket_has_quit(f, socket)
    {
        return close_connection(
            s,
            f,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_QUIT()",
        (f.dispatch.client_id)(socket)
    );

    /* The client wants to leave. Display this and report it to the other clients. */
    (s.quit_announcement)(socket::server(socket));

    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED
            && client.socket != socket
        {
            send_quit(f, client.socket, ClientID((f.dispatch.client_id)(socket)));
        }
    }

    (s.admin_quit)(socket::server(socket));

    close_connection(
        s,
        f,
        socket,
        NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
        work,
    )
}

/// `ServerNetworkGameSocketHandler::CloseConnection`.
pub(crate) fn close_connection(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    status: NetworkRecvStatus,
    work: Actions,
) -> NetworkRecvStatus {
    crate::movement::native_assert(
        f.command.company.shared,
        status != NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY,
    );
    /*
     * Sending a message just before leaving the game calls cs->SendPackets.
     * This might invoke this function, which means that when we close the
     * connection after cs->SendPackets we will close an already closed
     * connection. This handles that case gracefully without having to make
     * that code any more complex or more aware of the validity of the socket.
     */
    if socket::pending(socket) || !(f.tcp.valid)(socket::tcp(socket)) {
        return status;
    }

    if status != NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT
        && status != NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR
        && !crate::network_frame::socket_has_quit(f, socket)
        && server::status(socket) >= ClientStatus::STATUS_AUTHORIZED
    {
        /* We did not receive a leave message from this client... */
        (s.lost_connection_announcement)(socket::server(socket));

        /* Inform other clients of this... strange leaving ;) */
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if server::status(client.socket) >= ClientStatus::STATUS_AUTHORIZED
                && socket != client.socket
            {
                send_error_quit(
                    f,
                    client.socket,
                    ClientID((f.dispatch.client_id)(socket)),
                    NetworkErrorCode::NETWORK_ERROR_CONNECTION_LOST,
                );
            }
        }
    }

    /* If we were transferring a map to this client, stop the savegame creation
     * process and queue the next client to receive the map. */
    if server::status(socket) == ClientStatus::STATUS_MAP {
        /* Ensure the saving of the game is stopped too. */
        (s.map_destroy)(socket::server(socket));

        check_next_client_to_send_map(s, f, socket, socket, work);
        if work.pending() {
            work.push(Resume::CloseTail(socket, status.0));
            return status;
        }
    }

    close_tail(s, f, socket, status)
}

/// `ServerNetworkGameSocketHandler::CloseConnection` after the next map client is queued,
/// resumed after a deferred save.
pub(crate) fn close_tail(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    status: NetworkRecvStatus,
) -> NetworkRecvStatus {
    (s.admin_error)(
        socket::server(socket),
        NetworkErrorCode::NETWORK_ERROR_CONNECTION_LOST.0,
    );
    debug!(
        net,
        3,
        "[{}] Client #{} closed connection",
        "server",
        (f.dispatch.client_id)(socket)
    );

    /* We just lost one client :( */
    if server::status(socket) >= ClientStatus::STATUS_AUTHORIZED {
        unsafe { s.clients_on.write(s.clients_on.read().wrapping_sub(1)) };
    }
    unsafe {
        s.clients_connected
            .write(s.clients_connected.read().wrapping_sub(1));
    };

    socket::send_closing(f, socket);

    socket::defer(socket);

    status
}

/// `ServerNetworkGameSocketHandler::CloseConnection` entered from a socket handler.
pub(crate) fn server_close(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    status: NetworkRecvStatus,
    work: Actions,
) -> NetworkRecvStatus {
    work.network(f);
    close_connection(s, f, socket, status, work)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_server_close(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    result: u8,
    work: Actions,
) -> u8 {
    server_close(s, f, socket, NetworkRecvStatus(result), work).0
}
