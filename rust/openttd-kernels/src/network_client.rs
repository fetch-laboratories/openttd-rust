/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Client admission, map transfer and disconnect decisions. Save filters, presentation and
//! socket transport stay native. Raw field accesses end before callbacks, including map loads.
#![allow(unsafe_code, missing_docs)]
use crate::debug::debug;
use crate::network_frame::Services as FrameServices;
use crate::network_frame::{
    packet_check, packet_new, packet_recv_string, packet_send, packet_send_string,
};
use crate::network_internal::NetworkJoinStatus;
use crate::network_socket as socket;
use crate::network_type::{
    NETWORK_ERROR_DESYNC, NETWORK_ERROR_GENERAL, NETWORK_ERROR_KICKED,
    NETWORK_ERROR_NEWGRF_MISMATCH, NETWORK_ERROR_SAVEGAME_FAILED, NETWORK_RECV_STATUS_CLOSE_QUERY,
    NETWORK_RECV_STATUS_DESYNC, NETWORK_RECV_STATUS_MALFORMED_PACKET,
    NETWORK_RECV_STATUS_NEWGRF_MISMATCH, NETWORK_RECV_STATUS_OKAY, NETWORK_RECV_STATUS_SAVEGAME,
    NETWORK_RECV_STATUS_SERVER_BANNED, NETWORK_RECV_STATUS_SERVER_ERROR,
    NETWORK_RECV_STATUS_SERVER_FULL,
};
use crate::openttd::{GameMode, SwitchMode};
use crate::packet::{self, State as PacketState};
use crate::tcp_game::PacketGameType;
use std::ffi::c_void;
#[repr(C)]
pub struct Services {
    pub auth:
        extern "C" fn(*mut c_void) -> *const crate::network_crypto::NetworkAuthenticationHandler,
    pub auth_create: extern "C" fn(*mut c_void),
    pub check_grf: extern "C" fn(u32, *const u8) -> bool,
    pub client_name: extern "C" fn() -> crate::services::ByteSpan,
    pub delete_info: extern "C" fn(*mut c_void),
    pub dirty_join: extern "C" fn(),
    pub emergency_save: extern "C" fn(),
    pub enable_encryption:
        extern "C" fn(*mut c_void, *mut crate::auth::Stream, *mut crate::auth::Stream),
    pub enter_multiplayer: extern "C" fn(),
    pub join: extern "C" fn(*mut c_void),
    pub join_bytes: *mut u32,
    pub join_bytes_total: *mut u32,
    pub join_company: *const u8,
    pub join_status: *mut u8,
    pub join_waiting: *mut u8,
    pub leave: extern "C" fn(*mut c_void),
    pub leave_error: extern "C" fn(*mut c_void, u8),
    pub map_append_data: extern "C" fn(*mut c_void, crate::services::ByteSpan) -> u32,
    pub map_create: extern "C" fn(*mut c_void),
    pub map_exists: extern "C" fn(*mut c_void) -> bool,
    pub map_load_prepare: extern "C" fn(*mut c_void, *mut crate::session::Load),
    pub map_release: extern "C" fn(*mut c_void),
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
    pub wait_for_quit_packets: extern "C" fn(),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkClientServices" ["probe"] => Services {
        auth,
        auth_create,
        check_grf,
        client_name,
        delete_info,
        dirty_join,
        emergency_save,
        enable_encryption,
        enter_multiplayer,
        join,
        join_bytes,
        join_bytes_total,
        join_company,
        join_status,
        join_waiting,
        leave,
        leave_error,
        map_append_data,
        map_create,
        map_exists,
        map_load_prepare,
        map_release,
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
        wait_for_quit_packets,
    };
}

/// `ClientNetworkGameSocketHandler::ServerStatus`.
/// `ClientNetworkGameSocketHandler::ServerStatus`: status of the connection with the server,
/// retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServerStatus(pub u8);

impl ServerStatus {
    /// We are trying to join a server.
    pub const STATUS_JOIN: Self = Self(1);
    /// Last action was requesting game (server) password.
    pub const STATUS_AUTH_GAME: Self = Self(2);
    /// The game authentication has completed and from here on the connection to the server is
    /// encrypted.
    pub const STATUS_ENCRYPTED: Self = Self(3);
    /// The client is authorized at the server.
    pub const STATUS_AUTHORIZED: Self = Self(5);
    /// The client is waiting as someone else is downloading the map.
    pub const STATUS_MAP_WAIT: Self = Self(6);
    /// The client is downloading the map.
    pub const STATUS_MAP: Self = Self(7);
    /// The client is active within in the game.
    pub const STATUS_ACTIVE: Self = Self(8);
}

pub(crate) mod status {
    use super::ServerStatus;
    pub const JOIN: u8 = ServerStatus::STATUS_JOIN.0;
    pub const ENCRYPTED: u8 = ServerStatus::STATUS_ENCRYPTED.0;
    pub const AUTHORIZED: u8 = ServerStatus::STATUS_AUTHORIZED.0;
    pub const MAP_WAIT: u8 = ServerStatus::STATUS_MAP_WAIT.0;
    pub const MAP: u8 = ServerStatus::STATUS_MAP.0;
    pub const ACTIVE: u8 = ServerStatus::STATUS_ACTIVE.0;
}

fn status(_s: &Services, game: *mut socket::GameSocket) -> u8 {
    socket::status(game)
}
fn set_status(_s: &Services, game: *mut socket::GameSocket, value: u8) {
    crate::raw_field::write_field!(game, status, value);
}
fn join_company(s: &Services) -> u8 {
    unsafe { s.join_company.read() }
}
fn set_join_status(s: &Services, value: NetworkJoinStatus) {
    unsafe { s.join_status.write(value.0) };
}
fn empty_packet(f: &FrameServices, game: *mut socket::GameSocket, kind: PacketGameType) -> u8 {
    let mut p = packet_new(f, game, kind);
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_join(s: &Services, f: &FrameServices) -> u8 {
    debug!(net, 9, "Client::SendJoin()");
    debug!(net, 9, "Client::status = JOIN");
    let game = (f.client_game)();
    set_status(s, game, status::JOIN);
    debug!(net, 9, "Client::join_status = AUTHORIZING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_AUTHORIZING);
    (s.dirty_join)();
    let mut p = packet_new(f, game, PacketGameType::PACKET_CLIENT_JOIN);
    packet_send_string(f, &mut p, (s.revision)());
    p.send_uint32((f.connection.newgrf_version)(), packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_identify(s: &Services, f: &FrameServices) -> u8 {
    debug!(net, 9, "Client::SendIdentify()");
    let game = (f.client_game)();
    let mut p = packet_new(f, game, PacketGameType::PACKET_CLIENT_IDENTIFY);
    packet_send_string(f, &mut p, (s.client_name)());
    p.send_uint8(join_company(s), packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_grfs_ok(f: &FrameServices) -> u8 {
    debug!(net, 9, "Client::SendNewGRFsOk()");
    empty_packet(
        f,
        (f.client_game)(),
        PacketGameType::PACKET_CLIENT_NEWGRFS_CHECKED,
    )
}

pub(crate) fn send_getmap(s: &Services, f: &FrameServices) -> u8 {
    debug!(net, 9, "Client::SendGetMap()");
    debug!(net, 9, "Client::status = MAP_WAIT");
    set_status(s, (f.client_game)(), status::MAP_WAIT);
    empty_packet(f, (f.client_game)(), PacketGameType::PACKET_CLIENT_GETMAP)
}

pub(crate) fn send_map_ok(s: &Services, f: &FrameServices) -> u8 {
    debug!(net, 9, "Client::SendMapOk()");
    debug!(net, 9, "Client::status = ACTIVE");
    set_status(s, (f.client_game)(), status::ACTIVE);
    empty_packet(f, (f.client_game)(), PacketGameType::PACKET_CLIENT_MAP_OK)
}

pub(crate) fn send_error(f: &FrameServices, error: u8) -> u8 {
    debug!(net, 9, "Client::SendError(): errorno={}", error);
    let game = (f.client_game)();
    let mut p = packet_new(f, game, PacketGameType::PACKET_CLIENT_ERROR);
    p.send_uint8(error, packet_check(f));
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_quit(f: &FrameServices) -> u8 {
    debug!(net, 9, "Client::SendQuit()");
    empty_packet(f, (f.client_game)(), PacketGameType::PACKET_CLIENT_QUIT)
}

pub(crate) fn receive_full(s: &Services, _f: &FrameServices, _game: *mut socket::GameSocket) -> u8 {
    debug!(net, 9, "Client::Receive_SERVER_FULL()");
    (s.show_full)();
    NETWORK_RECV_STATUS_SERVER_FULL
}

pub(crate) fn receive_banned(
    s: &Services,
    _f: &FrameServices,
    _game: *mut socket::GameSocket,
) -> u8 {
    debug!(net, 9, "Client::Receive_SERVER_BANNED()");
    (s.show_banned)();
    NETWORK_RECV_STATUS_SERVER_BANNED
}

pub(crate) fn receive_error(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
    text: *mut c_void,
) -> u8 {
    let error = unsafe { packet::recv_uint8(p, f.packet) };
    debug!(net, 9, "Client::Receive_SERVER_ERROR(): error={}", error);
    if error == NETWORK_ERROR_KICKED && unsafe { packet::can_read(p, f.packet, 1, false) } {
        packet_recv_string(f, p, text, crate::network_config::NETWORK_CHAT_LENGTH);
        (s.show_server_error)(error, text);
    } else {
        (s.show_server_error)(error, std::ptr::null_mut());
    }
    if status(s, game) == status::ACTIVE {
        (s.emergency_save)();
    }
    NETWORK_RECV_STATUS_SERVER_ERROR
}

pub(crate) fn receive_grfs(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) != status::ENCRYPTED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let count = u32::from(unsafe { packet::recv_uint8(p, f.packet) });
    debug!(
        net,
        9, "Client::Receive_SERVER_CHECK_NEWGRFS(): grf_count={}", count
    );
    let result = check_grfs(f.packet, p, count, s.check_grf);
    if result == NETWORK_RECV_STATUS_OKAY {
        return send_grfs_ok(f);
    }
    (s.show_newgrf_mismatch)();
    result
}

pub(crate) fn receive_welcome(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) < status::ENCRYPTED || status(s, game) >= status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::status = AUTHORIZED");
    set_status(s, game, status::AUTHORIZED);
    unsafe { s.own_client_id.write(packet::recv_uint32(p, f.packet)) };
    debug!(
        net,
        9,
        "Client::Receive_SERVER_WELCOME(): client_id={}",
        unsafe { s.own_client_id.read() }
    );
    send_getmap(s, f)
}

pub(crate) fn receive_wait(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) != status::MAP_WAIT {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::Receive_SERVER_WAIT()");
    debug!(net, 9, "Client::join_status = WAITING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_WAITING);
    unsafe { s.join_waiting.write(packet::recv_uint8(p, f.packet)) };
    (s.dirty_join)();
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_map_begin(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) < status::AUTHORIZED || status(s, game) >= status::MAP {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::status = MAP");
    set_status(s, game, status::MAP);
    if (s.map_exists)(socket::shell(game)) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    (s.map_create)(socket::shell(game));
    crate::network_frame::map_begin(unsafe { packet::recv_uint32(p, f.packet) });
    debug!(
        net,
        9,
        "Client::Receive_SERVER_MAP_BEGIN(): frame_counter={}",
        crate::network_frame::frame_counter()
    );
    unsafe {
        s.join_bytes.write(0);
        s.join_bytes_total.write(0);
    }
    debug!(net, 9, "Client::join_status = DOWNLOADING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_DOWNLOADING);
    (s.dirty_join)();
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_map_size(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) != status::MAP || !(s.map_exists)(socket::shell(game)) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    unsafe { s.join_bytes_total.write(packet::recv_uint32(p, f.packet)) };
    (s.dirty_join)();
    debug!(
        net,
        9,
        "Client::Receive_SERVER_MAP_SIZE(): bytes_total={}",
        unsafe { s.join_bytes_total.read() }
    );
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_map_data(
    s: &Services,
    _f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) != status::MAP || !(s.map_exists)(socket::shell(game)) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let size = unsafe { (*p).remaining() };
    let data = unsafe { (*p).data.add(usize::from((*p).position)) };
    let total = (s.map_append_data)(
        socket::shell(game),
        crate::services::ByteSpan { data, size },
    );
    #[allow(clippy::cast_possible_wrap)]
    unsafe {
        (*p).transfer_commit(size as isize);
        s.join_bytes.write(total);
    }
    (s.dirty_join)();
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_map_done(
    s: &Services,
    f: &FrameServices,
    session: &crate::session::Services,
    game: *mut socket::GameSocket,
    frame: *mut crate::session::Frame,
    load: *mut crate::session::Load,
) -> u8 {
    if status(s, game) != status::MAP || !(s.map_exists)(socket::shell(game)) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::Receive_SERVER_MAP_DONE()");
    debug!(net, 9, "Client::join_status = PROCESSING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_PROCESSING);
    (s.dirty_join)();
    (s.map_load_prepare)(socket::shell(game), load);
    let Ok(success) =
        crate::session::safe_load(session, frame, load, crate::openttd::GameMode::NORMAL)
    else {
        let work = unsafe {
            crate::network_action::Actions::from_storage((session.network_actions)(frame))
        };
        work.push(crate::network_action::Resume::MapLoaded(game));
        return if work.pending() {
            NETWORK_RECV_STATUS_SAVEGAME
        } else {
            NETWORK_RECV_STATUS_OKAY
        };
    };
    map_loaded(s, f, session, game, success)
}

pub(crate) fn map_loaded(
    s: &Services,
    f: &FrameServices,
    session: &crate::session::Services,
    game: *mut socket::GameSocket,
    success: bool,
) -> u8 {
    (s.map_release)(socket::shell(game));
    socket::set_last_packet(game, (f.tcp.now)());
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
            debug!(net, 9, "Client::join_status = REGISTERING");
            set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_REGISTERING);
            (s.show_join_window)();
            crate::command_post::new_company(f.command.company.shared, unsafe {
                s.own_client_id.read()
            });
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

pub(crate) fn receive_error_quit(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) < status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let id = unsafe { packet::recv_uint32(p, f.packet) };
    debug!(
        net,
        9, "Client::Receive_SERVER_ERROR_QUIT(): client_id={}", id
    );
    let info = (f.command.company.shared.network_client_info)(id);
    if !info.is_null() {
        (s.leave_error)(info, unsafe { packet::recv_uint8(p, f.packet) });
        (s.delete_info)(info);
    }
    ((f.command.company.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_quit(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) < status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let id = unsafe { packet::recv_uint32(p, f.packet) };
    debug!(net, 9, "Client::Receive_SERVER_QUIT(): client_id={}", id);
    let info = (f.command.company.shared.network_client_info)(id);
    if info.is_null() {
        debug!(net, 1, "Unknown client ({}) is leaving the game", id);
    } else {
        (s.leave)(info);
        (s.delete_info)(info);
    }
    ((f.command.company.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_join(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> u8 {
    if status(s, game) < status::AUTHORIZED {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let id = unsafe { packet::recv_uint32(p, f.packet) };
    debug!(net, 9, "Client::Receive_SERVER_JOIN(): client_id={}", id);
    let info = (f.command.company.shared.network_client_info)(id);
    if !info.is_null() {
        (s.join)(info);
    }
    ((f.command.company.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_shutdown(
    s: &Services,
    _f: &FrameServices,
    game: *mut socket::GameSocket,
) -> u8 {
    debug!(net, 9, "Client::Receive_SERVER_SHUTDOWN()");
    if status(s, game) >= status::JOIN {
        (s.show_shutdown)();
    }
    if status(s, game) == status::ACTIVE {
        (s.emergency_save)();
    }
    NETWORK_RECV_STATUS_SERVER_ERROR
}

pub(crate) fn receive_newgame(
    s: &Services,
    _f: &FrameServices,
    game: *mut socket::GameSocket,
) -> u8 {
    debug!(net, 9, "Client::Receive_SERVER_NEWGAME()");
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

fn close(s: &Services, f: &FrameServices, game: *mut socket::GameSocket, result: u8) -> u8 {
    crate::movement::native_assert(f.command.company.shared, result != NETWORK_RECV_STATUS_OKAY);
    if socket::pending(game) {
        return result;
    }
    crate::movement::native_assert(f.command.company.shared, (f.tcp.valid)(socket::tcp(game)));
    if !crate::network_frame::socket_has_quit(f, game) {
        debug!(
            net,
            3,
            "Closed client connection {}",
            (f.dispatch.client_id)(game)
        );
        socket::send_closing(f, game);
        (s.wait_for_quit_packets)();
    }
    socket::defer(game);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_close(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    result: u8,
) -> u8 {
    close(s, f, game, result)
}

pub(crate) fn client_error(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    result: u8,
) {
    if socket::pending(game) {
        return;
    }
    if result == NETWORK_RECV_STATUS_CLOSE_QUERY {
        unsafe { (f.packet.mark_closed)(socket::base(game)) };
        close(s, f, game, result);
        (f.lifecycle.set_networking)(false);
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
        send_error(f, error);
        close(s, f, game, result);
        (s.emergency_save)();
    }
    (f.command.company.close_network_status)();
    if crate::session::mode() != GameMode::MENU {
        crate::session::set_pending(SwitchMode::MENU);
    }
    (f.lifecycle.set_networking)(false);
}

const SECOND: i64 = 1_000_000_000;
static LAST_LAG: crate::game_state::GameState<i64> = crate::game_state::GameState::new(0);
pub(crate) fn check_connection(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    work: crate::network_action::Actions,
) {
    if status(s, game) < status::AUTHORIZED {
        return;
    }
    let lag = (f.tcp.now)().wrapping_sub(socket::last_packet(game));
    if lag < 5 * SECOND {
        return;
    }
    if lag > 20 * SECOND {
        socket::close(f, game, work);
        return;
    }
    if unsafe { LAST_LAG.get().read() } / SECOND == lag / SECOND {
        return;
    }
    unsafe { LAST_LAG.get().write(lag) };
    (s.show_lag)(lag / SECOND);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_is_connected_abi(
    s: &Services,
    f: &FrameServices,
) -> u8 {
    let game = (f.client_game)();
    u8::from(!game.is_null() && status(s, game) == status::ACTIVE)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_connected(s: &Services, f: &FrameServices) {
    crate::network_frame::client_connected();
    debug!(net, 9, "Client::NetworkClient_Connected()");
    send_join(s, f);
}

fn check_grfs(
    services: &packet::Services,
    p: *mut PacketState,
    count: u32,
    check: extern "C" fn(u32, *const u8) -> bool,
) -> u8 {
    let mut result = NETWORK_RECV_STATUS_OKAY;
    for _ in 0..count {
        let id = unsafe { packet::recv_uint32(p, services) };
        let mut hash = [0; crate::md5::MD5_HASH_BYTES];
        unsafe {
            (*p).recv_bytes(&mut hash);
        }
        if !check(id, hash.as_ptr()) {
            result = NETWORK_RECV_STATUS_NEWGRF_MISMATCH;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_newgrf_checksum_starts_empty() {
        extern "C" fn assert_condition(condition: bool) {
            assert!(condition);
        }
        extern "C" fn check(id: u32, hash: *const u8) -> bool {
            let hash = unsafe { std::slice::from_raw_parts(hash, crate::md5::MD5_HASH_BYTES) };
            match id {
                7 => {
                    assert_eq!(hash, [42; 16]);
                    true
                }
                8 => {
                    assert_eq!(hash, [0; 16]);
                    false
                }
                _ => panic!("unexpected NewGRF"),
            }
        }
        if crate::services_assert::isolate_test_check(
            "network_client::tests::each_newgrf_checksum_starts_empty",
            assert_condition,
        ) {
            return;
        }
        let mut p = PacketState::new(100, std::ptr::null_mut());
        p.send_bytes(&7_u32.to_le_bytes());
        p.send_bytes(&[42; 16]);
        p.send_bytes(&8_u32.to_le_bytes());
        assert_eq!(
            check_grfs(packet::test_services(), &raw mut p, 2, check),
            NETWORK_RECV_STATUS_NEWGRF_MISMATCH
        );
        assert_eq!(p.position, 24);
    }
}
