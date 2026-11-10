/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Connection admission and map pacing. Socket and client-info pools and save filters stay
//! native. Fields are accessed at their original observation points; no borrow survives a leaf.
#![allow(unsafe_code, missing_docs)]
use crate::debug::{Bytes, debug};
use crate::network_action::{Actions, Resume};
use crate::network_command::next_client;
use crate::network_frame::{
    Services as FrameServices,
    server_status::{
        ACTIVE, AUTHORIZED, DONE_MAP, IDENTIFY, INACTIVE, MAP, MAP_WAIT, NEWGRFS_CHECK, PRE_ACTIVE,
    },
};
use crate::network_frame::{
    packet_check, packet_new, packet_recv_string, packet_send, packet_send_string,
};
use crate::network_socket as socket;
use crate::network_type::{
    NETWORK_ERROR_COMPANY_MISMATCH, NETWORK_ERROR_CONNECTION_LOST, NETWORK_ERROR_FULL,
    NETWORK_ERROR_ILLEGAL_PACKET, NETWORK_ERROR_INVALID_CLIENT_NAME, NETWORK_ERROR_KICKED,
    NETWORK_ERROR_NAME_IN_USE, NETWORK_ERROR_NOT_AUTHORIZED, NETWORK_ERROR_NOT_EXPECTED,
    NETWORK_ERROR_WRONG_REVISION, NETWORK_RECV_STATUS_CLIENT_QUIT,
    NETWORK_RECV_STATUS_MALFORMED_PACKET, NETWORK_RECV_STATUS_OKAY,
    NETWORK_RECV_STATUS_SERVER_ERROR,
};
use crate::openttd::PauseMode;
use crate::packet::{self, State as PacketState};
use crate::raw_field::write_field;
use crate::tcp_game::{
    PACKET_SERVER_ERROR, PACKET_SERVER_ERROR_QUIT, PACKET_SERVER_JOIN, PACKET_SERVER_MAP_BEGIN,
    PACKET_SERVER_NEWGAME, PACKET_SERVER_QUIT, PACKET_SERVER_SHUTDOWN, PACKET_SERVER_WAIT,
    PACKET_SERVER_WELCOME,
};
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
fn status(_s: &Services, socket: *mut socket::GameSocket) -> u8 {
    socket::status(socket)
}
fn set_status(_s: &Services, socket: *mut socket::GameSocket, value: u8) {
    crate::raw_field::write_field!(socket, status, value);
}
pub(crate) fn reset_lag(_s: &Services, socket: *mut socket::GameSocket) {
    let sync = socket::sync(socket);
    write_field!(sync, last_frame, crate::network_frame::frame_counter());
    write_field!(
        sync,
        last_frame_server,
        crate::network_frame::frame_counter()
    );
}

pub(crate) fn check_pause_on_join(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause = crate::session::pause();
    if pause & PauseMode::ERROR.bit() != 0
        || (!(s.pause_on_join)() && pause & PauseMode::JOIN.bit() == 0)
    {
        return;
    }
    let mut cursor = 0;
    let mut joining = false;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if socket::status(client.socket) >= AUTHORIZED && socket::status(client.socket) < ACTIVE {
            joining = true;
            break;
        }
    }
    if joining != (pause & PauseMode::JOIN.bit() != 0) {
        crate::command_post::pause(unsafe { &*session.shared }, PauseMode::JOIN, joining);
    }
}

pub(crate) fn check_min_active_clients(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause = crate::session::pause();
    if pause & PauseMode::ERROR.bit() != 0
        || !(unsafe { &*(session.world)() }.dedicated)()
        || ((s.min_active_clients)() == 0 && pause & PauseMode::ACTIVE_CLIENTS.bit() == 0)
    {
        return;
    }
    let mut count = 0_u32;
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if socket::status(client.socket) != ACTIVE {
            continue;
        }
        if !(f.command.company.shared.company_valid)((s.client_company)(socket::server(
            client.socket,
        ))) {
            continue;
        }
        count += 1;
    }
    let required = count < u32::from((s.min_active_clients)());
    if required != (pause & PauseMode::ACTIVE_CLIENTS.bit() != 0) {
        crate::command_post::pause(
            unsafe { &*session.shared },
            PauseMode::ACTIVE_CLIENTS,
            required,
        );
    }
}

pub(crate) fn send_welcome(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendWelcome()",
        (f.dispatch.client_id)(socket)
    );
    if status(s, socket) != NEWGRFS_CHECK {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_MALFORMED_PACKET, work);
    }
    debug!(
        net,
        9,
        "client[{}] status = AUTHORIZED",
        (f.dispatch.client_id)(socket)
    );
    set_status(s, socket, AUTHORIZED);
    reset_lag(s, socket);
    unsafe { s.clients_on.write(s.clients_on.read().wrapping_add(1)) };
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_WELCOME); // PACKET_SERVER_WELCOME
    p.send_uint32((f.dispatch.client_id)(socket), packet_check(f));
    packet_send(f, game, &mut p);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket != socket && socket::status(client.socket) >= AUTHORIZED {
            (s.send_client_info)(
                socket::server(socket),
                (s.info)(socket::server(client.socket)),
            );
        }
    }
    (s.send_client_info)(
        socket::server(socket),
        (f.command.company.shared.network_client_info)(crate::network_type::CLIENT_ID_SERVER),
    )
}

pub(crate) fn send_wait(s: &Services, f: &FrameServices, socket: *mut socket::GameSocket) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendWait()",
        (f.dispatch.client_id)(socket)
    );
    let mut waiting = 1_u32;
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if socket::status(client.socket) != MAP_WAIT {
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
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_WAIT); // PACKET_SERVER_WAIT
    #[allow(clippy::cast_possible_truncation)]
    p.send_uint8(waiting as u8, packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn next_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    ignore: *mut socket::GameSocket,
    work: Actions,
) {
    debug!(
        net,
        9,
        "client[{}] CheckNextClientToSendMap()",
        (f.dispatch.client_id)(socket)
    );
    let mut best: *mut socket::GameSocket = std::ptr::null_mut();
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket == ignore {
            continue;
        }
        if socket::status(client.socket) == MAP_WAIT
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
    if !best.is_null() {
        set_status(s, best, AUTHORIZED);
        send_map(s, f, best, work);
        if work.pending() {
            work.push(Resume::Waits(1));
            return;
        }
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if socket::status(client.socket) == MAP_WAIT {
                send_wait(s, f, client.socket);
            }
        }
    }
}

pub(crate) fn update_waiters(f: &FrameServices, count: usize) {
    for _ in 0..count {
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if socket::status(client.socket) == MAP_WAIT {
                send_wait(f.connection, f, client.socket);
            }
        }
    }
}

pub(crate) fn send_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) < AUTHORIZED {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_AUTHORIZED, EMPTY_TEXT, work);
    }
    if status(s, socket) == AUTHORIZED {
        debug!(
            net,
            9,
            "client[{}] SendMap(): first_packet",
            (f.dispatch.client_id)(socket)
        );
        (s.map_create)(socket::server(socket));
        let game = socket;
        let mut p = packet_new(f, game, PACKET_SERVER_MAP_BEGIN); // PACKET_SERVER_MAP_BEGIN
        p.send_uint32(crate::network_frame::frame_counter(), packet_check(f));
        packet_send(f, game, &mut p);
        unsafe {
            crate::network_command::sync_command_queue(socket::outgoing(socket));
        };
        debug!(
            net,
            9,
            "client[{}] status = MAP",
            (f.dispatch.client_id)(socket)
        );
        set_status(s, socket, MAP);
        reset_lag(s, socket);
        work.save(socket);
        return NETWORK_RECV_STATUS_OKAY;
    }
    map_transfer(f, socket, work);
    NETWORK_RECV_STATUS_OKAY
}

/// `PacketWriter::TransferToNetworkQueue`; its native mutex remains locked until `map_end`.
fn transfer_to_network_queue(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> bool {
    let writer = (s.map_begin)(socket::server(socket));
    if writer.is_null() {
        return false;
    }
    let mut last = false;
    loop {
        let packet = (s.map_front)(writer);
        if packet.is_null() {
            break;
        }
        let record = (f.tcp.packet_record)(packet);
        last = unsafe {
            packet::get_type(
                record,
                packet::Context {
                    services: f.packet,
                    check: packet_check(f),
                },
            )
        } == crate::tcp_game::PACKET_SERVER_MAP_DONE;
        unsafe {
            packet::prepare_to_send(
                record,
                packet::Context {
                    services: f.packet,
                    check: packet_check(f),
                },
            );
        }
        (s.map_enqueue)(writer);
        if last {
            break;
        }
    }
    (s.map_end)(writer);
    last
}

pub(crate) fn map_transfer(f: &FrameServices, socket: *mut socket::GameSocket, work: Actions) {
    let s = f.connection;
    if status(s, socket) == MAP && transfer_to_network_queue(s, f, socket) {
        debug!(
            net,
            9,
            "client[{}] SendMap(): last_packet",
            (f.dispatch.client_id)(socket)
        );
        (s.map_destroy)(socket::server(socket));
        debug!(
            net,
            9,
            "client[{}] status = DONE_MAP",
            (f.dispatch.client_id)(socket)
        );
        set_status(s, socket, DONE_MAP);
        next_map(s, f, socket, std::ptr::null_mut(), work);
    }
}

pub(crate) fn receive_getmap(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) < AUTHORIZED || crate::network_frame::socket_has_quit(f, socket) {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_AUTHORIZED, EMPTY_TEXT, work);
    }
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_GETMAP()",
        (f.dispatch.client_id)(socket)
    );
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if socket::status(client.socket) == MAP {
            debug!(
                net,
                9,
                "client[{}] status = MAP_WAIT",
                (f.dispatch.client_id)(socket)
            );
            set_status(s, socket, MAP_WAIT);
            return send_wait(s, f, socket);
        }
    }
    send_map(s, f, socket, work)
}

pub(crate) fn receive_map_ok(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) == DONE_MAP && !crate::network_frame::socket_has_quit(f, socket) {
        debug!(
            net,
            9,
            "client[{}] Receive_CLIENT_MAP_OK()",
            (f.dispatch.client_id)(socket)
        );
        (s.joined)(socket::server(socket));
        debug!(
            net,
            9,
            "client[{}] status = PRE_ACTIVE",
            (f.dispatch.client_id)(socket)
        );
        set_status(s, socket, PRE_ACTIVE);
        unsafe {
            crate::network_command::handle_command_queue(
                f.command,
                socket,
                socket::outgoing(socket),
            );
        };
        unsafe { crate::network_frame::send_frame(f, socket, socket::sync(socket)) };
        crate::network_frame::send_sync(f, socket);
        reset_lag(s, socket);
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if socket::status(client.socket) >= AUTHORIZED {
                (s.send_client_info)(
                    socket::server(client.socket),
                    (s.info)(socket::server(socket)),
                );
                send_join(s, f, client.socket, (f.dispatch.client_id)(socket));
            }
        }
        (s.admin_joined)(socket::server(socket));
        return (s.send_config)(socket::server(socket));
    }
    send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT, work)
}

pub(crate) fn send_newgrf_check(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendNewGRFCheck()",
        (f.dispatch.client_id)(socket)
    );
    if status(s, socket) != IDENTIFY {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_MALFORMED_PACKET, work);
    }
    debug!(
        net,
        9,
        "client[{}] status = NEWGRFS_CHECK",
        (f.dispatch.client_id)(socket)
    );
    set_status(s, socket, NEWGRFS_CHECK);
    if unsafe { (s.grf.config_at)(0) }.is_null() {
        return send_welcome(s, f, socket, work);
    }
    let mut p = unsafe {
        packet::new_outgoing(
            socket::base(socket),
            packet::Context {
                services: f.packet,
                check: packet_check(f),
            },
            crate::tcp_game::PACKET_SERVER_CHECK_NEWGRFS,
            crate::network_config::TCP_MTU,
        )
    };
    let mut count = 0_u32;
    let mut cursor = 0;
    loop {
        let config = unsafe { (s.grf.config_at)(cursor) };
        if config.is_null() {
            break;
        }
        if unsafe { (s.grf.config_flags)(config) } & (1 << crate::newgrf_config::STATIC) == 0 {
            count += 1;
        }
        cursor += 1;
    }
    #[allow(clippy::cast_possible_truncation)]
    p.send_uint8(count as u8, packet_check(f));
    cursor = 0;
    loop {
        let config = unsafe { (s.grf.config_at)(cursor) };
        if config.is_null() {
            break;
        }
        if unsafe { (s.grf.config_flags)(config) } & (1 << crate::newgrf_config::STATIC) == 0 {
            p.send_uint32(unsafe { (s.grf.config_id)(config) }, packet_check(f));
            let hash = unsafe { (s.grf.config_md5)(config) };
            p.send_bytes(unsafe { crate::utf8::borrow(hash.data, hash.size) });
        }
        cursor += 1;
    }
    packet_send(f, socket, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_newgrfs_checked(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) != NEWGRFS_CHECK {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT, work);
    }
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_NEWGRFS_CHECKED()",
        (f.dispatch.client_id)(socket)
    );
    send_welcome(s, f, socket, work)
}

pub(crate) fn receive_game_info(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_GAME_INFO()",
        (f.dispatch.client_id)(socket)
    );
    (s.send_game_info)(socket::server(socket))
}

pub(crate) const EMPTY_TEXT: crate::services::ByteSpan = crate::services::ByteSpan {
    data: std::ptr::null(),
    size: 0,
};
static NEXT_CLIENT_ID: crate::game_state::GameState<u32> =
    crate::game_state::GameState::new(crate::network_type::CLIENT_ID_FIRST);

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_next_client_id() -> u32 {
    // The native ClientID increment wraps without resetting when the server restarts.
    unsafe {
        let id = NEXT_CLIENT_ID.get().read();
        NEXT_CLIENT_ID.get().write(id.wrapping_add(1));
        id
    }
}

pub(crate) fn receive_join(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    text: *mut c_void,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) != INACTIVE {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT, work);
    }
    if unsafe { s.clients_on.read() } >= (s.max_clients)() {
        return send_error(s, f, socket, NETWORK_ERROR_FULL, EMPTY_TEXT, work);
    }
    packet_recv_string(f, p, text, crate::network_config::NETWORK_REVISION_LENGTH);
    let version = unsafe { packet::recv_uint32(p, f.packet) };
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_JOIN(): client_revision={}, newgrf_version={}",
        (f.dispatch.client_id)(socket),
        Bytes(unsafe {
            let mut size = 0;
            let data = (f.command.company.shared.string_data)(text, &raw mut size);
            crate::utf8::borrow(data, size)
        }),
        version
    );
    if !(s.revision_compatible)(text) || (s.newgrf_version)() != version {
        return send_error(s, f, socket, NETWORK_ERROR_WRONG_REVISION, EMPTY_TEXT, work);
    }
    crate::network_auth::send_request(f, socket, work)
}

pub(crate) fn receive_identify(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    name: *mut c_void,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) != IDENTIFY {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT, work);
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
        name,
        crate::network_config::NETWORK_CLIENT_NAME_LENGTH,
    );
    let mut company = unsafe { packet::recv_uint8(p, f.packet) };
    if crate::network_frame::socket_has_quit(f, socket) {
        return NETWORK_RECV_STATUS_CLIENT_QUIT;
    }
    match company {
        crate::company_type::COMPANY_NEW_COMPANY => {
            if (f.command.company.company_count)() >= usize::from((s.max_companies)()) {
                return send_error(s, f, socket, NETWORK_ERROR_FULL, EMPTY_TEXT, work);
            }
        }
        crate::company_type::COMPANY_SPECTATOR => {}
        _ => {
            if !(f.command.company.shared.company_human)(company) {
                return send_error(
                    s,
                    f,
                    socket,
                    NETWORK_ERROR_COMPANY_MISMATCH,
                    EMPTY_TEXT,
                    work,
                );
            }
            if !(s.company_allows_client)(company, socket::server(socket)) {
                company = crate::company_type::COMPANY_SPECTATOR;
            }
        }
    }
    if !(s.valid_name)(name) {
        return send_error(
            s,
            f,
            socket,
            NETWORK_ERROR_INVALID_CLIENT_NAME,
            EMPTY_TEXT,
            work,
        );
    }
    if !(s.unique_name)(name) {
        return send_error(s, f, socket, NETWORK_ERROR_NAME_IN_USE, EMPTY_TEXT, work);
    }
    (s.allocate_info)(socket::server(socket), name, company);
    let finances = (f.command.company.finance.company_finances)(company);
    if !finances.is_null() {
        write_field!(finances, empty, 0);
    }
    send_newgrf_check(s, f, socket, work)
}

pub(crate) fn send_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    mut error: u8,
    reason: crate::services::ByteSpan,
    work: Actions,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendError(): error={}",
        (f.dispatch.client_id)(socket),
        error
    );
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_ERROR);
    p.send_uint8(error, packet_check(f));
    if reason.size != 0 {
        packet_send_string(f, &mut p, reason);
    }
    packet_send(f, game, &mut p);
    let strid = (s.error_message)(error);
    let authorized = status(s, socket) >= AUTHORIZED;
    (s.error_announcement)(
        socket::server(socket),
        strid,
        reason,
        error == NETWORK_ERROR_KICKED && reason.size != 0,
        authorized,
    );
    if authorized {
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if socket::status(client.socket) >= AUTHORIZED && client.socket != socket {
                if matches!(
                    error,
                    NETWORK_ERROR_NOT_AUTHORIZED
                        | NETWORK_ERROR_NOT_EXPECTED
                        | NETWORK_ERROR_WRONG_REVISION
                ) {
                    error = NETWORK_ERROR_ILLEGAL_PACKET;
                }
                send_error_quit(s, f, client.socket, (f.dispatch.client_id)(socket), error);
            }
        }
        (s.admin_error)(socket::server(socket), error);
    }
    close_connection(s, f, socket, NETWORK_RECV_STATUS_SERVER_ERROR, work)
}

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
    send_error(s, f, socket, error, reason, work)
}

pub(crate) fn send_error_quit(
    _s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    client_id: u32,
    error: u8,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendErrorQuit(): client_id={}, errorno={}",
        (f.dispatch.client_id)(socket),
        client_id,
        error
    );
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_ERROR_QUIT);
    p.send_uint32(client_id, packet_check(f));
    p.send_uint8(error, packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_quit(
    _s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    client_id: u32,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendQuit(): client_id={}",
        (f.dispatch.client_id)(socket),
        client_id
    );
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_QUIT);
    p.send_uint32(client_id, packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_join(
    _s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    client_id: u32,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendJoin(): client_id={}",
        (f.dispatch.client_id)(socket),
        client_id
    );
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_JOIN);
    p.send_uint32(client_id, packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_shutdown(
    _s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendShutdown()",
        (f.dispatch.client_id)(socket)
    );
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_SHUTDOWN);
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_newgame(
    _s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendNewGame()",
        (f.dispatch.client_id)(socket)
    );
    let game = socket;
    let mut p = packet_new(f, game, PACKET_SERVER_NEWGAME);
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    work: Actions,
) -> u8 {
    work.network(f);
    let error = unsafe { packet::recv_uint8(p, f.packet) };
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_ERROR(): errorno={}",
        (f.dispatch.client_id)(socket),
        error
    );
    if status(s, socket) < DONE_MAP || crate::network_frame::socket_has_quit(f, socket) {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT, work);
    }
    (s.client_error_announcement)(socket::server(socket), error);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if socket::status(client.socket) >= AUTHORIZED {
            send_error_quit(s, f, client.socket, (f.dispatch.client_id)(socket), error);
        }
    }
    (s.admin_error)(socket::server(socket), error);
    close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT, work)
}

pub(crate) fn receive_quit(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    work: Actions,
) -> u8 {
    work.network(f);
    if status(s, socket) < DONE_MAP || crate::network_frame::socket_has_quit(f, socket) {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT, work);
    }
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_QUIT()",
        (f.dispatch.client_id)(socket)
    );
    (s.quit_announcement)(socket::server(socket));
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if socket::status(client.socket) >= AUTHORIZED && client.socket != socket {
            send_quit(s, f, client.socket, (f.dispatch.client_id)(socket));
        }
    }
    (s.admin_quit)(socket::server(socket));
    close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT, work)
}

fn close_connection(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    result: u8,
    work: Actions,
) -> u8 {
    crate::movement::native_assert(f.command.company.shared, result != NETWORK_RECV_STATUS_OKAY);
    if socket::pending(socket) || !(f.tcp.valid)(socket::tcp(socket)) {
        return result;
    }
    if result != NETWORK_RECV_STATUS_CLIENT_QUIT
        && result != NETWORK_RECV_STATUS_SERVER_ERROR
        && !crate::network_frame::socket_has_quit(f, socket)
        && status(s, socket) >= AUTHORIZED
    {
        (s.lost_connection_announcement)(socket::server(socket));
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if socket::status(client.socket) >= AUTHORIZED && client.socket != socket {
                send_error_quit(
                    s,
                    f,
                    client.socket,
                    (f.dispatch.client_id)(socket),
                    NETWORK_ERROR_CONNECTION_LOST,
                );
            }
        }
    }
    if status(s, socket) == MAP {
        (s.map_destroy)(socket::server(socket));
        next_map(s, f, socket, socket, work);
        if work.pending() {
            work.push(Resume::CloseTail(socket, result));
            return result;
        }
    }
    close_tail(s, f, socket, result)
}

pub(crate) fn close_tail(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    result: u8,
) -> u8 {
    (s.admin_error)(socket::server(socket), NETWORK_ERROR_CONNECTION_LOST);
    debug!(
        net,
        3,
        "[{}] Client #{} closed connection",
        "server",
        (f.dispatch.client_id)(socket)
    );
    if status(s, socket) >= AUTHORIZED {
        unsafe { s.clients_on.write(s.clients_on.read().wrapping_sub(1)) };
    }
    unsafe {
        s.clients_connected
            .write(s.clients_connected.read().wrapping_sub(1));
    };
    socket::send_closing(f, socket);
    socket::defer(socket);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_server_close(
    s: &Services,
    f: &FrameServices,
    socket: *mut socket::GameSocket,
    result: u8,
    work: Actions,
) -> u8 {
    work.network(f);
    close_connection(s, f, socket, result, work)
}
