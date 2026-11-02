/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Client admission, map transfer and disconnect decisions. Save filters, presentation and
//! socket transport stay native. Raw field accesses end before callbacks, including map loads.
#![allow(unsafe_code, missing_docs)]
use crate::network_frame::Services as FrameServices;
use crate::network_internal::{
    NETWORK_JOIN_STATUS_AUTHORIZING, NETWORK_JOIN_STATUS_DOWNLOADING,
    NETWORK_JOIN_STATUS_PROCESSING, NETWORK_JOIN_STATUS_REGISTERING, NETWORK_JOIN_STATUS_WAITING,
};
use crate::network_type::{
    NETWORK_ERROR_DESYNC, NETWORK_ERROR_GENERAL, NETWORK_ERROR_KICKED,
    NETWORK_ERROR_NEWGRF_MISMATCH, NETWORK_ERROR_SAVEGAME_FAILED, NETWORK_RECV_STATUS_CLOSE_QUERY,
    NETWORK_RECV_STATUS_DESYNC, NETWORK_RECV_STATUS_MALFORMED_PACKET,
    NETWORK_RECV_STATUS_NEWGRF_MISMATCH, NETWORK_RECV_STATUS_OKAY, NETWORK_RECV_STATUS_SAVEGAME,
    NETWORK_RECV_STATUS_SERVER_BANNED, NETWORK_RECV_STATUS_SERVER_ERROR,
    NETWORK_RECV_STATUS_SERVER_FULL,
};
use crate::openttd::{GameMode, SwitchMode};
use crate::tcp_game::{
    PACKET_CLIENT_ERROR, PACKET_CLIENT_GETMAP, PACKET_CLIENT_IDENTIFY, PACKET_CLIENT_JOIN,
    PACKET_CLIENT_MAP_OK, PACKET_CLIENT_NEWGRFS_CHECKED, PACKET_CLIENT_QUIT,
};
use std::ffi::c_void;
#[repr(C)]
pub struct Services {
    pub check_grf: extern "C" fn(*mut c_void, *mut c_void) -> bool,
    pub client_list_changed: extern "C" fn(),
    pub client_name: extern "C" fn() -> crate::services::ByteSpan,
    pub debug_banned: extern "C" fn(),
    pub debug_close: extern "C" fn(*mut c_void),
    pub debug_connected: extern "C" fn(),
    pub debug_error: extern "C" fn(u8),
    pub debug_error_quit: extern "C" fn(u32),
    pub debug_full: extern "C" fn(),
    pub debug_grfs: extern "C" fn(u32),
    pub debug_join: extern "C" fn(u32),
    pub debug_join_authorizing: extern "C" fn(),
    pub debug_join_downloading: extern "C" fn(),
    pub debug_join_processing: extern "C" fn(),
    pub debug_join_registering: extern "C" fn(),
    pub debug_join_waiting: extern "C" fn(),
    pub debug_map_begin: extern "C" fn(u32),
    pub debug_map_done: extern "C" fn(),
    pub debug_map_size: extern "C" fn(u32),
    pub debug_newgame: extern "C" fn(),
    pub debug_quit: extern "C" fn(u32),
    pub debug_send_error: extern "C" fn(u8),
    pub debug_send_getmap: extern "C" fn(),
    pub debug_send_grfs_ok: extern "C" fn(),
    pub debug_send_identify: extern "C" fn(),
    pub debug_send_join: extern "C" fn(),
    pub debug_send_map_ok: extern "C" fn(),
    pub debug_send_quit: extern "C" fn(),
    pub debug_shutdown: extern "C" fn(),
    pub debug_status_active: extern "C" fn(),
    pub debug_status_authorized: extern "C" fn(),
    pub debug_status_join: extern "C" fn(),
    pub debug_status_map: extern "C" fn(),
    pub debug_status_map_wait: extern "C" fn(),
    pub debug_wait: extern "C" fn(),
    pub debug_welcome: extern "C" fn(u32),
    pub delete_info: extern "C" fn(*mut c_void),
    pub dirty_join: extern "C" fn(),
    pub emergency_save: extern "C" fn(),
    pub enter_multiplayer: extern "C" fn(),
    pub join: extern "C" fn(*mut c_void),
    pub join_bytes: *mut u32,
    pub join_bytes_total: *mut u32,
    pub join_company: *const u8,
    pub join_status: *mut u8,
    pub join_waiting: *mut u8,
    pub leave: extern "C" fn(*mut c_void),
    pub leave_error: extern "C" fn(*mut c_void, u8),
    pub map_add_packet: extern "C" fn(*mut c_void, *mut c_void) -> u32,
    pub map_create: extern "C" fn(*mut c_void),
    pub map_exists: extern "C" fn(*mut c_void) -> bool,
    pub map_load: extern "C" fn(*mut c_void) -> bool,
    pub new_company: extern "C" fn(u32),
    pub own_client_id: *mut u32,
    pub reconnect: *mut u8,
    pub revision: extern "C" fn() -> crate::services::ByteSpan,
    pub show_banned: extern "C" fn(),
    pub show_client_list: extern "C" fn(),
    pub show_full: extern "C" fn(),
    pub show_join_window: extern "C" fn(),
    pub show_lag: extern "C" fn(i64),
    pub show_newgrf_mismatch: extern "C" fn(),
    pub show_reboot: extern "C" fn(),
    pub show_savegame_error: extern "C" fn(),
    pub show_server_error: extern "C" fn(u8, *mut c_void),
    pub show_shutdown: extern "C" fn(),
    pub status: extern "C" fn(*mut c_void) -> *mut u8,
    pub unknown_quit: extern "C" fn(u32),
    pub wait_for_quit_packets: extern "C" fn(),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkClientServices" ["probe"] => Services {
        check_grf,
        client_list_changed,
        client_name,
        debug_banned,
        debug_close,
        debug_connected,
        debug_error,
        debug_error_quit,
        debug_full,
        debug_grfs,
        debug_join,
        debug_join_authorizing,
        debug_join_downloading,
        debug_join_processing,
        debug_join_registering,
        debug_join_waiting,
        debug_map_begin,
        debug_map_done,
        debug_map_size,
        debug_newgame,
        debug_quit,
        debug_send_error,
        debug_send_getmap,
        debug_send_grfs_ok,
        debug_send_identify,
        debug_send_join,
        debug_send_map_ok,
        debug_send_quit,
        debug_shutdown,
        debug_status_active,
        debug_status_authorized,
        debug_status_join,
        debug_status_map,
        debug_status_map_wait,
        debug_wait,
        debug_welcome,
        delete_info,
        dirty_join,
        emergency_save,
        enter_multiplayer,
        join,
        join_bytes,
        join_bytes_total,
        join_company,
        join_status,
        join_waiting,
        leave,
        leave_error,
        map_add_packet,
        map_create,
        map_exists,
        map_load,
        new_company,
        own_client_id,
        reconnect,
        revision,
        show_banned,
        show_client_list,
        show_full,
        show_join_window,
        show_lag,
        show_newgrf_mismatch,
        show_reboot,
        show_savegame_error,
        show_server_error,
        show_shutdown,
        status,
        unknown_quit,
        wait_for_quit_packets,
    };
}

/// `ClientNetworkGameSocketHandler::ServerStatus`.
pub(crate) mod status {
    pub const JOIN: u8 = 1;
    pub const ENCRYPTED: u8 = 3;
    pub const AUTHORIZED: u8 = 5;
    pub const MAP_WAIT: u8 = 6;
    pub const MAP: u8 = 7;
    pub const ACTIVE: u8 = 8;
}

fn status(s: &Services, game: *mut c_void) -> u8 {
    unsafe { (s.status)(game).read() }
}
fn set_status(s: &Services, game: *mut c_void, value: u8) {
    unsafe { (s.status)(game).write(value) };
}
fn join_company(s: &Services) -> u8 {
    unsafe { s.join_company.read() }
}
fn set_join_status(s: &Services, value: u8) {
    unsafe { s.join_status.write(value) };
}
fn empty_packet(f: &FrameServices, game: *mut c_void, kind: u8) -> u8 {
    let p = (f.packet_new)(game, kind);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_join(s: &Services, f: &FrameServices) -> u8 {
    (s.debug_send_join)();
    (s.debug_status_join)();
    let game = (f.client_game)();
    set_status(s, game, status::JOIN);
    (s.debug_join_authorizing)();
    set_join_status(s, NETWORK_JOIN_STATUS_AUTHORIZING);
    (s.dirty_join)();
    let p = (f.packet_new)(game, PACKET_CLIENT_JOIN);
    (f.packet_send_string)(p, (s.revision)());
    (f.packet_send_uint32)(p, (f.connection.newgrf_version)());
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_send_identify(s: &Services, f: &FrameServices) -> u8 {
    (s.debug_send_identify)();
    let game = (f.client_game)();
    let p = (f.packet_new)(game, PACKET_CLIENT_IDENTIFY);
    (f.packet_send_string)(p, (s.client_name)());
    (f.packet_send_uint8)(p, join_company(s));
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_grfs_ok(s: &Services, f: &FrameServices) -> u8 {
    (s.debug_send_grfs_ok)();
    empty_packet(f, (f.client_game)(), PACKET_CLIENT_NEWGRFS_CHECKED)
}

pub(crate) fn send_getmap(s: &Services, f: &FrameServices) -> u8 {
    (s.debug_send_getmap)();
    (s.debug_status_map_wait)();
    set_status(s, (f.client_game)(), status::MAP_WAIT);
    empty_packet(f, (f.client_game)(), PACKET_CLIENT_GETMAP)
}

pub(crate) fn send_map_ok(s: &Services, f: &FrameServices) -> u8 {
    (s.debug_send_map_ok)();
    (s.debug_status_active)();
    set_status(s, (f.client_game)(), status::ACTIVE);
    empty_packet(f, (f.client_game)(), PACKET_CLIENT_MAP_OK)
}

pub(crate) fn send_error(s: &Services, f: &FrameServices, error: u8) -> u8 {
    (s.debug_send_error)(error);
    let game = (f.client_game)();
    let p = (f.packet_new)(game, PACKET_CLIENT_ERROR);
    (f.packet_send_uint8)(p, error);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_quit(s: &Services, f: &FrameServices) -> u8 {
    (s.debug_send_quit)();
    empty_packet(f, (f.client_game)(), PACKET_CLIENT_QUIT)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_full(
    s: &Services,
    _f: &FrameServices,
    _game: *mut c_void,
) -> u8 {
    (s.debug_full)();
    (s.show_full)();
    NETWORK_RECV_STATUS_SERVER_FULL
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_banned(
    s: &Services,
    _f: &FrameServices,
    _game: *mut c_void,
) -> u8 {
    (s.debug_banned)();
    (s.show_banned)();
    NETWORK_RECV_STATUS_SERVER_BANNED
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_error(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
    text: *mut c_void,
) -> u8 {
    let error = (f.packet_recv_uint8)(p);
    (s.debug_error)(error);
    if error == NETWORK_ERROR_KICKED && (f.packet_can_read)(p, 1) {
        (f.packet_recv_string)(p, text, crate::network_config::NETWORK_CHAT_LENGTH);
        (s.show_server_error)(error, text);
    } else {
        (s.show_server_error)(error, std::ptr::null_mut());
    }
    if status(s, game) == status::ACTIVE {
        (s.emergency_save)();
    }
    NETWORK_RECV_STATUS_SERVER_ERROR
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_grfs(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
    identifier: *mut c_void,
) -> u8 {
    if status(s, game) != status::ENCRYPTED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let count = u32::from((f.packet_recv_uint8)(p));
    let mut result = NETWORK_RECV_STATUS_OKAY;
    (s.debug_grfs)(count);
    for _ in 0..count {
        if !(s.check_grf)(p, identifier) {
            result = NETWORK_RECV_STATUS_NEWGRF_MISMATCH;
        }
    }
    if result == NETWORK_RECV_STATUS_OKAY {
        return send_grfs_ok(s, f);
    }
    (s.show_newgrf_mismatch)();
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_welcome(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) < status::ENCRYPTED || status(s, game) >= status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    (s.debug_status_authorized)();
    set_status(s, game, status::AUTHORIZED);
    unsafe { s.own_client_id.write((f.packet_recv_uint32)(p)) };
    (s.debug_welcome)(unsafe { s.own_client_id.read() });
    send_getmap(s, f)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_wait(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) != status::MAP_WAIT {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    (s.debug_wait)();
    (s.debug_join_waiting)();
    set_join_status(s, NETWORK_JOIN_STATUS_WAITING);
    unsafe { s.join_waiting.write((f.packet_recv_uint8)(p)) };
    (s.dirty_join)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_map_begin(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) < status::AUTHORIZED || status(s, game) >= status::MAP {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    (s.debug_status_map)();
    set_status(s, game, status::MAP);
    if (s.map_exists)(game) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    (s.map_create)(game);
    crate::network_frame::map_begin((f.packet_recv_uint32)(p));
    (s.debug_map_begin)(crate::network_frame::frame_counter());
    unsafe {
        s.join_bytes.write(0);
        s.join_bytes_total.write(0);
    }
    (s.debug_join_downloading)();
    set_join_status(s, NETWORK_JOIN_STATUS_DOWNLOADING);
    (s.dirty_join)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_map_size(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) != status::MAP || !(s.map_exists)(game) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    unsafe { s.join_bytes_total.write((f.packet_recv_uint32)(p)) };
    (s.dirty_join)();
    (s.debug_map_size)(unsafe { s.join_bytes_total.read() });
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_map_data(
    s: &Services,
    _f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) != status::MAP || !(s.map_exists)(game) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    unsafe { s.join_bytes.write((s.map_add_packet)(game, p)) };
    (s.dirty_join)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_map_done(
    s: &Services,
    f: &FrameServices,
    session: &crate::session::Services,
    game: *mut c_void,
) -> u8 {
    if status(s, game) != status::MAP || !(s.map_exists)(game) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    (s.debug_map_done)();
    (s.debug_join_processing)();
    set_join_status(s, NETWORK_JOIN_STATUS_PROCESSING);
    (s.dirty_join)();
    let success = (s.map_load)(game);
    (f.socket_set_last_packet_now)(game);
    if !success {
        (s.show_savegame_error)();
        return NETWORK_RECV_STATUS_SAVEGAME;
    }
    send_map_ok(s, f);
    (session.reset_session_stats)();
    (s.show_client_list)();
    if join_company(s) == crate::company_type::COMPANY_NEW_COMPANY
        || !(f.command.company.shared.company_valid)(join_company(s))
    {
        crate::company::set_local_company(
            f.command.company,
            crate::company_type::CompanyID::SPECTATOR,
        );
        if join_company(s) != crate::company_type::COMPANY_SPECTATOR {
            (s.debug_join_registering)();
            set_join_status(s, NETWORK_JOIN_STATUS_REGISTERING);
            (s.show_join_window)();
            (s.new_company)(unsafe { s.own_client_id.read() });
        }
    } else {
        crate::company::set_local_company(
            f.command.company,
            crate::company_type::CompanyID(join_company(s)),
        );
    }
    (s.enter_multiplayer)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_error_quit(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) < status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let id = (f.packet_recv_uint32)(p);
    (s.debug_error_quit)(id);
    let info = (f.connection.info_by_id)(id);
    if !info.is_null() {
        (s.leave_error)(info, (f.packet_recv_uint8)(p));
        (s.delete_info)(info);
    }
    (s.client_list_changed)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_quit(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) < status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let id = (f.packet_recv_uint32)(p);
    (s.debug_quit)(id);
    let info = (f.connection.info_by_id)(id);
    if info.is_null() {
        (s.unknown_quit)(id);
    } else {
        (s.leave)(info);
        (s.delete_info)(info);
    }
    (s.client_list_changed)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_join(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    p: *mut c_void,
) -> u8 {
    if status(s, game) < status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let id = (f.packet_recv_uint32)(p);
    (s.debug_join)(id);
    let info = (f.connection.info_by_id)(id);
    if !info.is_null() {
        (s.join)(info);
    }
    (s.client_list_changed)();
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_shutdown(
    s: &Services,
    _f: &FrameServices,
    game: *mut c_void,
) -> u8 {
    (s.debug_shutdown)();
    if status(s, game) >= status::JOIN {
        (s.show_shutdown)();
    }
    if status(s, game) == status::ACTIVE {
        (s.emergency_save)();
    }
    NETWORK_RECV_STATUS_SERVER_ERROR
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_receive_newgame(
    s: &Services,
    _f: &FrameServices,
    game: *mut c_void,
) -> u8 {
    (s.debug_newgame)();
    if status(s, game) >= status::JOIN {
        #[allow(clippy::cast_possible_truncation)]
        unsafe {
            s.reconnect.write((s.own_client_id.read() % 16 + 1) as u8);
        };
        (s.show_reboot)();
    }
    if status(s, game) == status::ACTIVE {
        (s.emergency_save)();
    }
    NETWORK_RECV_STATUS_SERVER_ERROR
}

fn close(s: &Services, f: &FrameServices, game: *mut c_void, result: u8) -> u8 {
    assert_ne!(result, NETWORK_RECV_STATUS_OKAY);
    if (f.socket_pending_deletion)(game) {
        return result;
    }
    assert!((f.socket_valid)(game));
    if !(f.socket_has_quit)(game) {
        (s.debug_close)(game);
        (f.socket_send_packets)(game, true);
        (s.wait_for_quit_packets)();
    }
    (f.socket_defer_deletion)(game);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_close(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    result: u8,
) -> u8 {
    close(s, f, game, result)
}

pub(crate) fn client_error(s: &Services, f: &FrameServices, game: *mut c_void, result: u8) {
    if (f.socket_pending_deletion)(game) {
        return;
    }
    if result == NETWORK_RECV_STATUS_CLOSE_QUERY {
        (f.socket_mark_closed)(game);
        close(s, f, game, result);
        unsafe { f.connection.networking.write(false) };
        (f.command.company.close_network_status)();
        return;
    }
    let error = match result {
        NETWORK_RECV_STATUS_DESYNC => NETWORK_ERROR_DESYNC,
        NETWORK_RECV_STATUS_SAVEGAME => NETWORK_ERROR_SAVEGAME_FAILED,
        NETWORK_RECV_STATUS_NEWGRF_MISMATCH => NETWORK_ERROR_NEWGRF_MISMATCH,
        _ => NETWORK_ERROR_GENERAL,
    };
    if matches!(
        result,
        NETWORK_RECV_STATUS_SERVER_ERROR
            | NETWORK_RECV_STATUS_SERVER_FULL
            | NETWORK_RECV_STATUS_SERVER_BANNED
    ) {
        close(s, f, game, result);
    } else {
        send_error(s, f, error);
        close(s, f, game, result);
        (s.emergency_save)();
    }
    (f.command.company.close_network_status)();
    if crate::session::mode() != GameMode::MENU {
        crate::session::set_pending(SwitchMode::MENU);
    }
    unsafe { f.connection.networking.write(false) };
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_error(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
    result: u8,
) {
    client_error(s, f, game, result);
}

const SECOND: i64 = 1_000_000_000;
static LAST_LAG: crate::game_state::GameState<i64> = crate::game_state::GameState::new(0);
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_check_connection(
    s: &Services,
    f: &FrameServices,
    game: *mut c_void,
) {
    if status(s, game) < status::AUTHORIZED {
        return;
    }
    let lag = (f.now)().wrapping_sub((f.socket_last_packet)(game));
    if lag < 5 * SECOND {
        return;
    }
    if lag > 20 * SECOND {
        (f.socket_close)(game);
        return;
    }
    if unsafe { LAST_LAG.get().read() } / SECOND == lag / SECOND {
        return;
    }
    unsafe { LAST_LAG.get().write(lag) };
    (s.show_lag)(lag / SECOND);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_is_connected(
    s: &Services,
    f: &FrameServices,
) -> bool {
    let game = (f.client_game)();
    !game.is_null() && status(s, game) == status::ACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_connected(s: &Services, f: &FrameServices) {
    crate::network_frame::client_connected();
    (s.debug_connected)();
    send_join(s, f);
}
