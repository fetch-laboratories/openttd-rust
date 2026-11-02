/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Connection admission and map pacing. Socket and client-info pools and save filters stay
//! native. Fields are accessed at their original observation points; no borrow survives a leaf.
#![allow(unsafe_code, missing_docs)]
use crate::network_command::next_client;
use crate::network_frame::{
    Services as FrameServices, SocketSync,
    server_status::{
        ACTIVE, AUTHORIZED, DONE_MAP, IDENTIFY, INACTIVE, MAP, MAP_WAIT, NEWGRFS_CHECK, PRE_ACTIVE,
    },
};
use crate::network_type::{
    NETWORK_ERROR_COMPANY_MISMATCH, NETWORK_ERROR_CONNECTION_LOST, NETWORK_ERROR_FULL,
    NETWORK_ERROR_ILLEGAL_PACKET, NETWORK_ERROR_INVALID_CLIENT_NAME, NETWORK_ERROR_KICKED,
    NETWORK_ERROR_NAME_IN_USE, NETWORK_ERROR_NOT_AUTHORIZED, NETWORK_ERROR_NOT_EXPECTED,
    NETWORK_ERROR_WRONG_REVISION, NETWORK_RECV_STATUS_CLIENT_QUIT,
    NETWORK_RECV_STATUS_MALFORMED_PACKET, NETWORK_RECV_STATUS_OKAY,
    NETWORK_RECV_STATUS_SERVER_ERROR,
};
use crate::openttd::PauseMode;
use crate::raw_field::write_field;
use crate::tcp_game::{
    PACKET_SERVER_ERROR, PACKET_SERVER_ERROR_QUIT, PACKET_SERVER_JOIN, PACKET_SERVER_MAP_BEGIN,
    PACKET_SERVER_NEWGAME, PACKET_SERVER_QUIT, PACKET_SERVER_SHUTDOWN, PACKET_SERVER_WAIT,
    PACKET_SERVER_WELCOME,
};
use std::ffi::c_void;

#[repr(C)]
pub struct Services {
    pub admin_error: extern "C" fn(*mut c_void, u8),
    pub admin_joined: extern "C" fn(*mut c_void),
    pub admin_quit: extern "C" fn(*mut c_void),
    pub allocate_info: extern "C" fn(*mut c_void, *mut c_void, u8),
    pub auth_request: extern "C" fn(*mut c_void) -> u8,
    pub client_company: extern "C" fn(*mut c_void) -> u8,
    pub client_error_announcement: extern "C" fn(*mut c_void, u8),
    pub client_id: extern "C" fn(*mut c_void) -> u32,
    pub clients_connected: *mut u8,
    pub clients_on: *mut u8,
    pub company_allows_client: extern "C" fn(u8, *mut c_void) -> bool,
    pub debug_authorized: extern "C" fn(*mut c_void),
    pub debug_closed: extern "C" fn(*mut c_void),
    pub debug_done_map: extern "C" fn(*mut c_void),
    pub debug_game_info: extern "C" fn(*mut c_void),
    pub debug_getmap: extern "C" fn(*mut c_void),
    pub debug_identify: extern "C" fn(*mut c_void),
    pub debug_join: extern "C" fn(*mut c_void, *mut c_void, u32),
    pub debug_map: extern "C" fn(*mut c_void),
    pub debug_map_first: extern "C" fn(*mut c_void),
    pub debug_map_last: extern "C" fn(*mut c_void),
    pub debug_map_ok: extern "C" fn(*mut c_void),
    pub debug_map_wait: extern "C" fn(*mut c_void),
    pub debug_newgrf_check: extern "C" fn(*mut c_void),
    pub debug_newgrfs: extern "C" fn(*mut c_void),
    pub debug_newgrfs_checked: extern "C" fn(*mut c_void),
    pub debug_next_map: extern "C" fn(*mut c_void),
    pub debug_pre_active: extern "C" fn(*mut c_void),
    pub debug_receive_error: extern "C" fn(*mut c_void, u8),
    pub debug_receive_quit: extern "C" fn(*mut c_void),
    pub debug_send_error: extern "C" fn(*mut c_void, u8),
    pub debug_send_error_quit: extern "C" fn(*mut c_void, u32, u8),
    pub debug_send_join: extern "C" fn(*mut c_void, u32),
    pub debug_send_newgame: extern "C" fn(*mut c_void),
    pub debug_send_quit: extern "C" fn(*mut c_void, u32),
    pub debug_send_shutdown: extern "C" fn(*mut c_void),
    pub debug_send_wait: extern "C" fn(*mut c_void),
    pub debug_send_welcome: extern "C" fn(*mut c_void),
    pub error_announcement: extern "C" fn(*mut c_void, u32, crate::services::ByteSpan, bool),
    pub error_message: extern "C" fn(u8) -> u32,
    pub error_unjoined: extern "C" fn(*mut c_void, u32),
    pub game: extern "C" fn(*mut c_void) -> *mut c_void,
    pub grfs_empty: extern "C" fn() -> bool,
    pub grfs_packet: extern "C" fn(*mut c_void) -> *mut c_void,
    pub info: extern "C" fn(*mut c_void) -> *mut c_void,
    pub join_date: extern "C" fn(*mut c_void) -> i32,
    pub joined: extern "C" fn(*mut c_void),
    pub lost_connection_announcement: extern "C" fn(*mut c_void),
    pub map_create: extern "C" fn(*mut c_void),
    pub map_destroy: extern "C" fn(*mut c_void),
    pub map_save: extern "C" fn(*mut c_void),
    pub map_transfer: extern "C" fn(*mut c_void) -> bool,
    pub max_clients: extern "C" fn() -> u8,
    pub max_companies: extern "C" fn() -> u8,
    pub min_active_clients: extern "C" fn() -> u8,
    pub network_server: *mut bool,
    pub networking: *mut bool,
    pub newgrf_version: extern "C" fn() -> u32,
    pub outgoing_queue: extern "C" fn(*mut c_void) -> *mut crate::network_command::CommandQueue,
    pub pause_on_join: extern "C" fn() -> bool,
    pub quit_announcement: extern "C" fn(*mut c_void),
    pub revision_compatible: extern "C" fn(*mut c_void) -> bool,
    pub send_client_info: extern "C" fn(*mut c_void, *mut c_void) -> u8,
    pub send_config: extern "C" fn(*mut c_void) -> u8,
    pub send_game_info: extern "C" fn(*mut c_void) -> u8,
    pub status: extern "C" fn(*mut c_void) -> *mut u8,
    pub sync: extern "C" fn(*mut c_void) -> *mut SocketSync,
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
        auth_request,
        client_company,
        client_error_announcement,
        client_id,
        clients_connected,
        clients_on,
        company_allows_client,
        debug_authorized,
        debug_closed,
        debug_done_map,
        debug_game_info,
        debug_getmap,
        debug_identify,
        debug_join,
        debug_map,
        debug_map_first,
        debug_map_last,
        debug_map_ok,
        debug_map_wait,
        debug_newgrf_check,
        debug_newgrfs,
        debug_newgrfs_checked,
        debug_next_map,
        debug_pre_active,
        debug_receive_error,
        debug_receive_quit,
        debug_send_error,
        debug_send_error_quit,
        debug_send_join,
        debug_send_newgame,
        debug_send_quit,
        debug_send_shutdown,
        debug_send_wait,
        debug_send_welcome,
        error_announcement,
        error_message,
        error_unjoined,
        game,
        grfs_empty,
        grfs_packet,
        info,
        join_date,
        joined,
        lost_connection_announcement,
        map_create,
        map_destroy,
        map_save,
        map_transfer,
        max_clients,
        max_companies,
        min_active_clients,
        network_server,
        networking,
        newgrf_version,
        outgoing_queue,
        pause_on_join,
        quit_announcement,
        revision_compatible,
        send_client_info,
        send_config,
        send_game_info,
        status,
        sync,
        unique_name,
        valid_name,
    };
}

// Native sockets and their fields remain live until their original deferred-deletion point.
fn status(s: &Services, socket: *mut c_void) -> u8 {
    unsafe { (s.status)(socket).read() }
}
fn set_status(s: &Services, socket: *mut c_void, value: u8) {
    unsafe { (s.status)(socket).write(value) };
}
fn reset_lag(s: &Services, socket: *mut c_void) {
    let sync = (s.sync)(socket);
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
        if client.status >= AUTHORIZED && client.status < ACTIVE {
            joining = true;
            break;
        }
    }
    if joining != (pause & PauseMode::JOIN.bit() != 0) {
        (session.post_pause)(PauseMode::JOIN, joining);
    }
}

pub(crate) fn check_min_active_clients(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause = crate::session::pause();
    if pause & PauseMode::ERROR.bit() != 0
        || !(session.dedicated)()
        || ((s.min_active_clients)() == 0 && pause & PauseMode::ACTIVE_CLIENTS.bit() == 0)
    {
        return;
    }
    let mut count = 0_u32;
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status != ACTIVE {
            continue;
        }
        if !(f.command.company.shared.company_valid)((s.client_company)(client.socket)) {
            continue;
        }
        count += 1;
    }
    let required = count < u32::from((s.min_active_clients)());
    if required != (pause & PauseMode::ACTIVE_CLIENTS.bit() != 0) {
        (session.post_pause)(PauseMode::ACTIVE_CLIENTS, required);
    }
}

pub(crate) fn send_welcome(s: &Services, f: &FrameServices, socket: *mut c_void) -> u8 {
    (s.debug_send_welcome)(socket);
    if status(s, socket) != NEWGRFS_CHECK {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_MALFORMED_PACKET);
    }
    (s.debug_authorized)(socket);
    set_status(s, socket, AUTHORIZED);
    reset_lag(s, socket);
    unsafe { s.clients_on.write(s.clients_on.read().wrapping_add(1)) };
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_WELCOME); // PACKET_SERVER_WELCOME
    (f.packet_send_uint32)(p, (s.client_id)(socket));
    (f.packet_send)(game, p);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket != socket && client.status >= AUTHORIZED {
            (s.send_client_info)(socket, (s.info)(client.socket));
        }
    }
    (s.send_client_info)(
        socket,
        (f.command.company.shared.network_client_info)(crate::network_type::CLIENT_ID_SERVER),
    )
}

pub(crate) fn send_wait(s: &Services, f: &FrameServices, socket: *mut c_void) -> u8 {
    (s.debug_send_wait)(socket);
    let mut waiting = 1_u32;
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status != MAP_WAIT {
            continue;
        }
        if (s.join_date)(client.socket) < (s.join_date)(socket)
            || ((s.join_date)(client.socket) == (s.join_date)(socket)
                && (s.client_id)(client.socket) < (s.client_id)(socket))
        {
            waiting += 1;
        }
    }
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_WAIT); // PACKET_SERVER_WAIT
    #[allow(clippy::cast_possible_truncation)]
    (f.packet_send_uint8)(p, waiting as u8);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn next_map(s: &Services, f: &FrameServices, socket: *mut c_void, ignore: *mut c_void) {
    (s.debug_next_map)(socket);
    let mut best: *mut c_void = std::ptr::null_mut();
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket == ignore {
            continue;
        }
        if client.status == MAP_WAIT
            && (best.is_null()
                || (s.join_date)(best) > (s.join_date)(client.socket)
                || ((s.join_date)(best) == (s.join_date)(client.socket)
                    && (s.client_id)(best) > (s.client_id)(client.socket)))
        {
            best = client.socket;
        }
    }
    if !best.is_null() {
        set_status(s, best, AUTHORIZED);
        openttd_rust_network_send_map(s, f, best);
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if client.status == MAP_WAIT {
                send_wait(s, f, client.socket);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) < AUTHORIZED {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_AUTHORIZED, EMPTY_TEXT);
    }
    if status(s, socket) == AUTHORIZED {
        (s.debug_map_first)(socket);
        (s.map_create)(socket);
        let game = (s.game)(socket);
        let p = (f.packet_new)(game, PACKET_SERVER_MAP_BEGIN); // PACKET_SERVER_MAP_BEGIN
        (f.packet_send_uint32)(p, crate::network_frame::frame_counter());
        (f.packet_send)(game, p);
        unsafe {
            crate::network_command::sync_command_queue((s.outgoing_queue)(socket));
        };
        (s.debug_map)(socket);
        set_status(s, socket, MAP);
        reset_lag(s, socket);
        (s.map_save)(socket);
    }
    if status(s, socket) == MAP && (s.map_transfer)(socket) {
        (s.debug_map_last)(socket);
        (s.map_destroy)(socket);
        (s.debug_done_map)(socket);
        set_status(s, socket, DONE_MAP);
        next_map(s, f, socket, std::ptr::null_mut());
    }
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_getmap(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) < AUTHORIZED || (f.socket_has_quit)((s.game)(socket)) {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_AUTHORIZED, EMPTY_TEXT);
    }
    (s.debug_getmap)(socket);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status == MAP {
            (s.debug_map_wait)(socket);
            set_status(s, socket, MAP_WAIT);
            return send_wait(s, f, socket);
        }
    }
    openttd_rust_network_send_map(s, f, socket)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_map_ok(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) == DONE_MAP && !(f.socket_has_quit)((s.game)(socket)) {
        (s.debug_map_ok)(socket);
        (s.joined)(socket);
        (s.debug_pre_active)(socket);
        set_status(s, socket, PRE_ACTIVE);
        unsafe {
            crate::network_command::handle_command_queue(
                f.command,
                socket,
                (s.outgoing_queue)(socket),
            );
        };
        unsafe { crate::network_frame::send_frame(f, (s.game)(socket), (s.sync)(socket)) };
        crate::network_frame::send_sync(f, (s.game)(socket));
        reset_lag(s, socket);
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if client.status >= AUTHORIZED {
                (s.send_client_info)(client.socket, (s.info)(socket));
                send_join(s, f, client.socket, (s.client_id)(socket));
            }
        }
        (s.admin_joined)(socket);
        return (s.send_config)(socket);
    }
    send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT)
}

pub(crate) fn send_newgrf_check(s: &Services, f: &FrameServices, socket: *mut c_void) -> u8 {
    (s.debug_newgrf_check)(socket);
    if status(s, socket) != IDENTIFY {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_MALFORMED_PACKET);
    }
    (s.debug_newgrfs)(socket);
    set_status(s, socket, NEWGRFS_CHECK);
    if (s.grfs_empty)() {
        return send_welcome(s, f, socket);
    }
    let p = (s.grfs_packet)(socket);
    (f.packet_send)((s.game)(socket), p);
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_newgrfs_checked(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) != NEWGRFS_CHECK {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT);
    }
    (s.debug_newgrfs_checked)(socket);
    send_welcome(s, f, socket)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_game_info(
    s: &Services,
    _f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    (s.debug_game_info)(socket);
    (s.send_game_info)(socket)
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

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_join(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    p: *mut c_void,
    text: *mut c_void,
) -> u8 {
    if status(s, socket) != INACTIVE {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT);
    }
    if unsafe { s.clients_on.read() } >= (s.max_clients)() {
        return send_error(s, f, socket, NETWORK_ERROR_FULL, EMPTY_TEXT);
    }
    (f.packet_recv_string)(p, text, crate::network_config::NETWORK_REVISION_LENGTH);
    let version = (f.packet_recv_uint32)(p);
    (s.debug_join)(socket, text, version);
    if !(s.revision_compatible)(text) || (s.newgrf_version)() != version {
        return send_error(s, f, socket, NETWORK_ERROR_WRONG_REVISION, EMPTY_TEXT);
    }
    (s.auth_request)(socket)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_identify(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    p: *mut c_void,
    name: *mut c_void,
) -> u8 {
    if status(s, socket) != IDENTIFY {
        return send_error(s, f, socket, NETWORK_ERROR_NOT_EXPECTED, EMPTY_TEXT);
    }
    (s.debug_identify)(socket);
    (f.packet_recv_string)(p, name, crate::network_config::NETWORK_CLIENT_NAME_LENGTH);
    let mut company = (f.packet_recv_uint8)(p);
    if (f.socket_has_quit)((s.game)(socket)) {
        return NETWORK_RECV_STATUS_CLIENT_QUIT;
    }
    match company {
        crate::company_type::COMPANY_NEW_COMPANY => {
            if (f.command.company.company_count)() >= usize::from((s.max_companies)()) {
                return send_error(s, f, socket, NETWORK_ERROR_FULL, EMPTY_TEXT);
            }
        }
        crate::company_type::COMPANY_SPECTATOR => {}
        _ => {
            if !(f.command.company.shared.company_human)(company) {
                return send_error(s, f, socket, NETWORK_ERROR_COMPANY_MISMATCH, EMPTY_TEXT);
            }
            if !(s.company_allows_client)(company, socket) {
                company = crate::company_type::COMPANY_SPECTATOR;
            }
        }
    }
    if !(s.valid_name)(name) {
        return send_error(s, f, socket, NETWORK_ERROR_INVALID_CLIENT_NAME, EMPTY_TEXT);
    }
    if !(s.unique_name)(name) {
        return send_error(s, f, socket, NETWORK_ERROR_NAME_IN_USE, EMPTY_TEXT);
    }
    (s.allocate_info)(socket, name, company);
    let finances = (f.command.company.finance.company_finances)(company);
    if !finances.is_null() {
        write_field!(finances, empty, 0);
    }
    send_newgrf_check(s, f, socket)
}

pub(crate) fn send_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    mut error: u8,
    reason: crate::services::ByteSpan,
) -> u8 {
    (s.debug_send_error)(socket, error);
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_ERROR);
    (f.packet_send_uint8)(p, error);
    if reason.size != 0 {
        (f.packet_send_string)(p, reason);
    }
    (f.packet_send)(game, p);
    let strid = (s.error_message)(error);
    if status(s, socket) >= AUTHORIZED {
        (s.error_announcement)(
            socket,
            strid,
            reason,
            error == NETWORK_ERROR_KICKED && reason.size != 0,
        );
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if client.status >= AUTHORIZED && client.socket != socket {
                if matches!(
                    error,
                    NETWORK_ERROR_NOT_AUTHORIZED
                        | NETWORK_ERROR_NOT_EXPECTED
                        | NETWORK_ERROR_WRONG_REVISION
                ) {
                    error = NETWORK_ERROR_ILLEGAL_PACKET;
                }
                send_error_quit(s, f, client.socket, (s.client_id)(socket), error);
            }
        }
        (s.admin_error)(socket, error);
    } else {
        (s.error_unjoined)(socket, strid);
    }
    close_connection(s, f, socket, NETWORK_RECV_STATUS_SERVER_ERROR)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    error: u8,
    reason: crate::services::ByteSpan,
) -> u8 {
    send_error(s, f, socket, error, reason)
}

pub(crate) fn send_error_quit(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    client_id: u32,
    error: u8,
) -> u8 {
    (s.debug_send_error_quit)(socket, client_id, error);
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_ERROR_QUIT);
    (f.packet_send_uint32)(p, client_id);
    (f.packet_send_uint8)(p, error);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_quit(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    client_id: u32,
) -> u8 {
    (s.debug_send_quit)(socket, client_id);
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_QUIT);
    (f.packet_send_uint32)(p, client_id);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_join(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    client_id: u32,
) -> u8 {
    (s.debug_send_join)(socket, client_id);
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_JOIN);
    (f.packet_send_uint32)(p, client_id);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_shutdown(s: &Services, f: &FrameServices, socket: *mut c_void) -> u8 {
    (s.debug_send_shutdown)(socket);
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_SHUTDOWN);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_newgame(s: &Services, f: &FrameServices, socket: *mut c_void) -> u8 {
    (s.debug_send_newgame)(socket);
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_NEWGAME);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_error(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    p: *mut c_void,
) -> u8 {
    let error = (f.packet_recv_uint8)(p);
    (s.debug_receive_error)(socket, error);
    if status(s, socket) < DONE_MAP || (f.socket_has_quit)((s.game)(socket)) {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT);
    }
    (s.client_error_announcement)(socket, error);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status >= AUTHORIZED {
            send_error_quit(s, f, client.socket, (s.client_id)(socket), error);
        }
    }
    (s.admin_error)(socket, error);
    close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_quit(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) < DONE_MAP || (f.socket_has_quit)((s.game)(socket)) {
        return close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT);
    }
    (s.debug_receive_quit)(socket);
    (s.quit_announcement)(socket);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status >= AUTHORIZED && client.socket != socket {
            send_quit(s, f, client.socket, (s.client_id)(socket));
        }
    }
    (s.admin_quit)(socket);
    close_connection(s, f, socket, NETWORK_RECV_STATUS_CLIENT_QUIT)
}

fn close_connection(s: &Services, f: &FrameServices, socket: *mut c_void, result: u8) -> u8 {
    assert_ne!(result, NETWORK_RECV_STATUS_OKAY);
    if (f.socket_pending_deletion)((s.game)(socket)) || !(f.socket_valid)((s.game)(socket)) {
        return result;
    }
    if result != NETWORK_RECV_STATUS_CLIENT_QUIT
        && result != NETWORK_RECV_STATUS_SERVER_ERROR
        && !(f.socket_has_quit)((s.game)(socket))
        && status(s, socket) >= AUTHORIZED
    {
        (s.lost_connection_announcement)(socket);
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if client.status >= AUTHORIZED && client.socket != socket {
                send_error_quit(
                    s,
                    f,
                    client.socket,
                    (s.client_id)(socket),
                    NETWORK_ERROR_CONNECTION_LOST,
                );
            }
        }
    }
    if status(s, socket) == MAP {
        (s.map_destroy)(socket);
        next_map(s, f, socket, socket);
    }
    (s.admin_error)(socket, NETWORK_ERROR_CONNECTION_LOST);
    (s.debug_closed)(socket);
    if status(s, socket) >= AUTHORIZED {
        unsafe { s.clients_on.write(s.clients_on.read().wrapping_sub(1)) };
    }
    unsafe {
        s.clients_connected
            .write(s.clients_connected.read().wrapping_sub(1));
    };
    (f.socket_send_packets)((s.game)(socket), true);
    (f.socket_defer_deletion)((s.game)(socket));
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_server_close(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    result: u8,
) -> u8 {
    close_connection(s, f, socket, result)
}
