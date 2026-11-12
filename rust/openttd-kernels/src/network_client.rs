/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Client admission, map transfer and disconnect decisions. Save filters, presentation and
//! socket transport stay native. Raw field accesses end before callbacks, including map loads.
#![allow(unsafe_code, missing_docs)]
use crate::company_type::CompanyID;
use crate::debug::debug;
use crate::network_frame::Services as FrameServices;
use crate::network_frame::{
    packet_check, packet_new, packet_recv_string, packet_send, packet_send_string,
};
use crate::network_internal::NetworkJoinStatus;
use crate::network_socket as socket;
use crate::network_type::{ClientID, NetworkErrorCode, NetworkRecvStatus};
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

/// `ClientNetworkGameSocketHandler::status` of the live client-side socket.
pub(crate) fn status(game: *mut socket::GameSocket) -> ServerStatus {
    ServerStatus(socket::status(game))
}

/// Stores `ClientNetworkGameSocketHandler::status` of the live client-side socket.
pub(crate) fn set_status(game: *mut socket::GameSocket, status: ServerStatus) {
    socket::set_status(game, status.0);
}

/// `_network_join.company`.
fn join_company(s: &Services) -> CompanyID {
    CompanyID(unsafe { s.join_company.read() })
}
/// `_network_join_status`.
fn set_join_status(s: &Services, value: NetworkJoinStatus) {
    unsafe { s.join_status.write(value.0) };
}
/// `_network_own_client_id`.
fn own_client_id(s: &Services) -> ClientID {
    ClientID(unsafe { s.own_client_id.read() })
}
/// Stores `_network_own_client_id`.
fn set_own_client_id(s: &Services, client_id: ClientID) {
    unsafe { s.own_client_id.write(client_id.0) };
}
/// Native `SetWindowDirty(WC_NETWORK_STATUS_WINDOW, WN_NETWORK_STATUS_WINDOW_JOIN)`.
fn set_join_window_dirty(s: &Services) {
    (s.dirty_join)();
}
/// A packet of `kind` with no payload, sent to the server.
fn send_empty(f: &FrameServices, kind: PacketGameType) -> NetworkRecvStatus {
    let my_client = (f.client_game)();
    let mut p = packet_new(f, my_client, kind);
    packet_send(f, my_client, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendJoin`: tell the server we would like to join.
pub(crate) fn send_join(s: &Services, f: &FrameServices) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendJoin()");

    debug!(net, 9, "Client::status = JOIN");
    let my_client = (f.client_game)();
    set_status(my_client, ServerStatus::STATUS_JOIN);
    debug!(net, 9, "Client::join_status = AUTHORIZING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_AUTHORIZING);
    set_join_window_dirty(s);

    let mut p = packet_new(f, my_client, PacketGameType::PACKET_CLIENT_JOIN);
    packet_send_string(f, &mut p, (s.revision)());
    p.send_uint32((f.connection.newgrf_version)(), packet_check(f));
    packet_send(f, my_client, &mut p);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendIdentify`.
pub(crate) fn send_identify(s: &Services, f: &FrameServices) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendIdentify()");

    let my_client = (f.client_game)();
    let mut p = packet_new(f, my_client, PacketGameType::PACKET_CLIENT_IDENTIFY);
    packet_send_string(f, &mut p, (s.client_name)()); // Client name
    p.send_uint8(join_company(s).0, packet_check(f)); // PlayAs
    packet_send(f, my_client, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendNewGRFsOk`: tell the server we got all the `NewGRFs`.
pub(crate) fn send_newgrfs_ok(f: &FrameServices) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendNewGRFsOk()");

    send_empty(f, PacketGameType::PACKET_CLIENT_NEWGRFS_CHECKED)
}

/// `ClientNetworkGameSocketHandler::SendGetMap`: request the map from the server.
pub(crate) fn send_get_map(f: &FrameServices) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendGetMap()");

    debug!(net, 9, "Client::status = MAP_WAIT");
    set_status((f.client_game)(), ServerStatus::STATUS_MAP_WAIT);

    send_empty(f, PacketGameType::PACKET_CLIENT_GETMAP)
}

/// `ClientNetworkGameSocketHandler::SendMapOk`: tell the server we received the complete map.
pub(crate) fn send_map_ok(f: &FrameServices) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendMapOk()");

    debug!(net, 9, "Client::status = ACTIVE");
    set_status((f.client_game)(), ServerStatus::STATUS_ACTIVE);

    send_empty(f, PacketGameType::PACKET_CLIENT_MAP_OK)
}

/// `ClientNetworkGameSocketHandler::SendError`: send an error-packet over the network.
pub(crate) fn send_error(f: &FrameServices, errorno: NetworkErrorCode) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendError(): errorno={}", errorno.0);

    let my_client = (f.client_game)();
    let mut p = packet_new(f, my_client, PacketGameType::PACKET_CLIENT_ERROR);

    p.send_uint8(errorno.0, packet_check(f));
    packet_send(f, my_client, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendQuit`: tell the server we would like to quit.
pub(crate) fn send_quit(f: &FrameServices) -> NetworkRecvStatus {
    debug!(net, 9, "Client::SendQuit()");

    send_empty(f, PacketGameType::PACKET_CLIENT_QUIT)
}

/// `ClientNetworkGameSocketHandler::IsConnected`: whether the client is actually connected
/// (and in the game).
pub(crate) fn is_connected(f: &FrameServices) -> bool {
    let my_client = (f.client_game)();
    !my_client.is_null() && status(my_client) == ServerStatus::STATUS_ACTIVE
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_FULL`.
pub(crate) fn receive_server_full(s: &Services) -> NetworkRecvStatus {
    debug!(net, 9, "Client::Receive_SERVER_FULL()");

    /* We try to join a server which is full */
    (s.show_full)();

    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_FULL
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_BANNED`.
pub(crate) fn receive_server_banned(s: &Services) -> NetworkRecvStatus {
    debug!(net, 9, "Client::Receive_SERVER_BANNED()");

    /* We try to join a server where we are banned */
    (s.show_banned)();

    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_BANNED
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_ERROR`. The native presentation maps the
/// error to its string; `text` receives the kick message.
pub(crate) fn receive_server_error(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
    text: *mut c_void,
) -> NetworkRecvStatus {
    let error = NetworkErrorCode(unsafe { packet::recv_uint8(p, f.packet) });

    debug!(net, 9, "Client::Receive_SERVER_ERROR(): error={}", error.0);

    /* In case of kicking a client, we assume there is a kick message in the packet if we can read one byte */
    if error == NetworkErrorCode::NETWORK_ERROR_KICKED
        && unsafe { packet::can_read(p, f.packet, 1, false) }
    {
        packet_recv_string(f, p, text, crate::network_config::NETWORK_CHAT_LENGTH);
        (s.show_server_error)(error.0, text);
    } else {
        (s.show_server_error)(error.0, std::ptr::null_mut());
    }

    /* Perform an emergency save if we had already entered the game */
    if status(game) == ServerStatus::STATUS_ACTIVE {
        (s.emergency_save)();
    }

    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_CHECK_NEWGRFS`.
pub(crate) fn receive_server_check_newgrfs(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) != ServerStatus::STATUS_ENCRYPTED {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    let grf_count = u32::from(unsafe { packet::recv_uint8(p, f.packet) });

    debug!(
        net,
        9, "Client::Receive_SERVER_CHECK_NEWGRFS(): grf_count={}", grf_count
    );

    let ret = check_newgrfs(f.packet, p, grf_count, s.check_grf);

    if ret == NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY {
        /* Start receiving the map */
        return send_newgrfs_ok(f);
    }

    /* NewGRF mismatch, bail out */
    (s.show_newgrf_mismatch)();
    ret
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_WELCOME`.
pub(crate) fn receive_server_welcome(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) < ServerStatus::STATUS_ENCRYPTED
        || status(game) >= ServerStatus::STATUS_AUTHORIZED
    {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::status = AUTHORIZED");
    set_status(game, ServerStatus::STATUS_AUTHORIZED);

    set_own_client_id(s, ClientID(unsafe { packet::recv_uint32(p, f.packet) }));

    debug!(
        net,
        9,
        "Client::Receive_SERVER_WELCOME(): client_id={}",
        own_client_id(s).0
    );

    /* Start receiving the map */
    send_get_map(f)
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_WAIT`.
pub(crate) fn receive_server_wait(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    /* We set the internal wait state when requesting the map. */
    if status(game) != ServerStatus::STATUS_MAP_WAIT {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    debug!(net, 9, "Client::Receive_SERVER_WAIT()");

    /* But... only now we set the join status to waiting, instead of requesting. */
    debug!(net, 9, "Client::join_status = WAITING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_WAITING);
    unsafe { s.join_waiting.write(packet::recv_uint8(p, f.packet)) };
    set_join_window_dirty(s);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_MAP_BEGIN`. The native shell owns the
/// `savegame` reader.
pub(crate) fn receive_server_map_begin(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) < ServerStatus::STATUS_AUTHORIZED || status(game) >= ServerStatus::STATUS_MAP {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::status = MAP");
    set_status(game, ServerStatus::STATUS_MAP);

    if (s.map_exists)(socket::shell(game)) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
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
    set_join_window_dirty(s);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_MAP_SIZE`.
pub(crate) fn receive_server_map_size(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) != ServerStatus::STATUS_MAP {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    if !(s.map_exists)(socket::shell(game)) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    unsafe { s.join_bytes_total.write(packet::recv_uint32(p, f.packet)) };
    set_join_window_dirty(s);

    debug!(
        net,
        9,
        "Client::Receive_SERVER_MAP_SIZE(): bytes_total={}",
        unsafe { s.join_bytes_total.read() }
    );

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_MAP_DATA`.
pub(crate) fn receive_server_map_data(
    s: &Services,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) != ServerStatus::STATUS_MAP {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    if !(s.map_exists)(socket::shell(game)) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    /* We are still receiving data, put it to the file */
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
    set_join_window_dirty(s);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_MAP_DONE` up to its `SafeLoad`; a native map
/// save requested during the load resumes in `map_loaded`.
pub(crate) fn receive_server_map_done(
    s: &Services,
    f: &FrameServices,
    session: &crate::session::Services,
    game: *mut socket::GameSocket,
    frame: *mut crate::session::Frame,
    load: *mut crate::session::Load,
) -> NetworkRecvStatus {
    if status(game) != ServerStatus::STATUS_MAP {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    if !(s.map_exists)(socket::shell(game)) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    debug!(net, 9, "Client::Receive_SERVER_MAP_DONE()");

    debug!(net, 9, "Client::join_status = PROCESSING");
    set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_PROCESSING);
    set_join_window_dirty(s);

    /* The map is done downloading, load it */
    /* Set the abstract filetype. This is read during savegame load. */
    (s.map_load_prepare)(socket::shell(game), load);

    let Ok(load_success) =
        crate::session::safe_load(session, frame, load, crate::openttd::GameMode::NORMAL)
    else {
        let work = unsafe {
            crate::network_action::Actions::from_storage((session.network_actions)(frame))
        };
        work.push(crate::network_action::Resume::MapLoaded(game));
        return if work.pending() {
            NetworkRecvStatus::NETWORK_RECV_STATUS_SAVEGAME
        } else {
            NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
        };
    };
    map_loaded(s, f, session, game, load_success)
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_MAP_DONE` after its `SafeLoad`.
pub(crate) fn map_loaded(
    s: &Services,
    f: &FrameServices,
    session: &crate::session::Services,
    game: *mut socket::GameSocket,
    load_success: bool,
) -> NetworkRecvStatus {
    (s.map_release)(socket::shell(game));

    /* Long savegame loads shouldn't affect the lag calculation! */
    socket::set_last_packet(game, (f.tcp.now)());

    if !load_success {
        (s.show_savegame_error)();
        return NetworkRecvStatus::NETWORK_RECV_STATUS_SAVEGAME;
    }
    /* If the savegame has successfully loaded, ALL windows have been removed,
     * only toolbar/statusbar and gamefield are visible */

    /* Say we received the map and loaded it correctly! */
    send_map_ok(f);

    /* As we skipped switch-mode, update the time we "switched". */
    (session.reset_session_stats)();

    (s.show_client_list)();

    /* New company/spectator (invalid company) or company we want to join is not active
     * Switch local company to spectator and await the server's judgement */
    if join_company(s) == CompanyID::NEW_COMPANY
        || !(f.command.company.shared.company_valid)(join_company(s).0)
    {
        crate::company::set_local_company(f.command.company, CompanyID::SPECTATOR);

        if join_company(s) != CompanyID::SPECTATOR {
            /* We have arrived and ready to start playing; send a command to make a new company;
             * the server will give us a client-id and let us in */
            debug!(net, 9, "Client::join_status = REGISTERING");
            set_join_status(s, NetworkJoinStatus::NETWORK_JOIN_STATUS_REGISTERING);
            (s.show_join_window)();
            crate::command_post::new_company(f.command.company.shared, own_client_id(s).0);
        }
    } else {
        /* take control over an existing company */
        crate::company::set_local_company(f.command.company, join_company(s));
    }

    (s.enter_multiplayer)();

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_ERROR_QUIT`.
pub(crate) fn receive_server_error_quit(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) < ServerStatus::STATUS_AUTHORIZED {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    let client_id = ClientID(unsafe { packet::recv_uint32(p, f.packet) });

    debug!(
        net,
        9, "Client::Receive_SERVER_ERROR_QUIT(): client_id={}", client_id.0
    );

    let ci = (f.command.company.shared.network_client_info)(client_id.0);
    if !ci.is_null() {
        (s.leave_error)(ci, unsafe { packet::recv_uint8(p, f.packet) });
        (s.delete_info)(ci);
    }

    ((f.command.company.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_QUIT`.
pub(crate) fn receive_server_quit(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) < ServerStatus::STATUS_AUTHORIZED {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    let client_id = ClientID(unsafe { packet::recv_uint32(p, f.packet) });

    debug!(
        net,
        9, "Client::Receive_SERVER_QUIT(): client_id={}", client_id.0
    );

    let ci = (f.command.company.shared.network_client_info)(client_id.0);
    if ci.is_null() {
        debug!(
            net,
            1, "Unknown client ({}) is leaving the game", client_id.0
        );
    } else {
        (s.leave)(ci);
        (s.delete_info)(ci);
    }

    ((f.command.company.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);

    /* If we come here it means we could not locate the client.. strange :s */
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_JOIN`.
pub(crate) fn receive_server_join(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    p: *mut PacketState,
) -> NetworkRecvStatus {
    if status(game) < ServerStatus::STATUS_AUTHORIZED {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    let client_id = ClientID(unsafe { packet::recv_uint32(p, f.packet) });

    debug!(
        net,
        9, "Client::Receive_SERVER_JOIN(): client_id={}", client_id.0
    );

    let ci = (f.command.company.shared.network_client_info)(client_id.0);
    if !ci.is_null() {
        (s.join)(ci);
    }

    ((f.command.company.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_SHUTDOWN`.
pub(crate) fn receive_server_shutdown(
    s: &Services,
    game: *mut socket::GameSocket,
) -> NetworkRecvStatus {
    debug!(net, 9, "Client::Receive_SERVER_SHUTDOWN()");

    /* Only when we're trying to join we really
     * care about the server shutting down. */
    if status(game) >= ServerStatus::STATUS_JOIN {
        (s.show_shutdown)();
    }

    if status(game) == ServerStatus::STATUS_ACTIVE {
        (s.emergency_save)();
    }

    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_NEWGAME`.
pub(crate) fn receive_server_newgame(
    s: &Services,
    game: *mut socket::GameSocket,
) -> NetworkRecvStatus {
    debug!(net, 9, "Client::Receive_SERVER_NEWGAME()");

    /* Only when we're trying to join we really
     * care about the server shutting down. */
    if status(game) >= ServerStatus::STATUS_JOIN {
        /* To throttle the reconnects a bit, every clients waits its
         * Client ID modulo 16 + 1 (value 0 means no reconnect).
         * This way reconnects should be spread out a bit. */
        #[allow(clippy::cast_possible_truncation)]
        unsafe {
            s.reconnect.write((own_client_id(s).0 % 16 + 1) as u8);
        };
        (s.show_reboot)();
    }

    if status(game) == ServerStatus::STATUS_ACTIVE {
        (s.emergency_save)();
    }

    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR
}

/// `ClientNetworkGameSocketHandler::CloseConnection(NetworkRecvStatus)`.
pub(crate) fn close_connection(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    status: NetworkRecvStatus,
) -> NetworkRecvStatus {
    crate::movement::native_assert(
        f.command.company.shared,
        status != NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY,
    );
    if socket::pending(game) {
        return status;
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

        /* Wait a number of ticks so our leave message can reach the server.
         * This is especially needed for Windows servers as they seem to get
         * the "socket is closed" message before receiving our leave message,
         * which would trigger the server to close the connection as well. */
        (s.wait_for_quit_packets)();
    }

    socket::defer(game);

    status
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_close(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    status: u8,
) -> u8 {
    close_connection(s, f, game, NetworkRecvStatus(status)).0
}

/// `ClientNetworkGameSocketHandler::ClientError`: handle an error coming from the client side.
pub(crate) fn client_error(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    res: NetworkRecvStatus,
) {
    if socket::pending(game) {
        return;
    }

    /* First, send a CLIENT_ERROR to the server, so it knows we are
     *  disconnected (and why!) */

    /* We just want to close the connection.. */
    if res == NetworkRecvStatus::NETWORK_RECV_STATUS_CLOSE_QUERY {
        unsafe { (f.packet.mark_closed)(socket::base(game)) };
        close_connection(s, f, game, res);
        (f.lifecycle.set_networking)(false);

        (f.command.company.close_network_status)();
        return;
    }

    let errorno = match res {
        NetworkRecvStatus::NETWORK_RECV_STATUS_DESYNC => NetworkErrorCode::NETWORK_ERROR_DESYNC,
        NetworkRecvStatus::NETWORK_RECV_STATUS_SAVEGAME => {
            NetworkErrorCode::NETWORK_ERROR_SAVEGAME_FAILED
        }
        NetworkRecvStatus::NETWORK_RECV_STATUS_NEWGRF_MISMATCH => {
            NetworkErrorCode::NETWORK_ERROR_NEWGRF_MISMATCH
        }
        _ => NetworkErrorCode::NETWORK_ERROR_GENERAL,
    };

    if res == NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR
        || res == NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_FULL
        || res == NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_BANNED
    {
        /* This means the server closed the connection. Emergency save is
         * already created if this was appropriate during handling of the
         * disconnect. */
        close_connection(s, f, game, res);
    } else {
        /* This means we as client made a boo-boo. */
        send_error(f, errorno);

        /* Close connection before we make an emergency save, as the save can
         * take a bit of time; better that the server doesn't stall while we
         * are doing the save, and already disconnects us. */
        close_connection(s, f, game, res);
        (s.emergency_save)();
    }

    (f.command.company.close_network_status)();

    if crate::session::mode() != GameMode::MENU {
        crate::session::set_pending(SwitchMode::MENU);
    }
    (f.lifecycle.set_networking)(false);
}

/// `ClientNetworkGameSocketHandler::CheckConnection`: check the connection's state, i.e. is
/// the connection still up? Durations are `steady_clock` nanoseconds.
pub(crate) fn check_connection(
    s: &Services,
    f: &FrameServices,
    game: *mut socket::GameSocket,
    work: crate::network_action::Actions,
) {
    /// `std::chrono::seconds(1)` in `steady_clock` nanoseconds.
    const SECONDS: i64 = 1_000_000_000;

    /* Only once we're authorized we can expect a steady stream of packets. */
    if status(game) < ServerStatus::STATUS_AUTHORIZED {
        return;
    }

    /* 5 seconds are roughly twice the server's "you're slow" threshold (1 game day). */
    let lag = (f.tcp.now)().wrapping_sub(socket::last_packet(game));
    if lag < 5 * SECONDS {
        return;
    }

    /* 20 seconds are (way) more than 4 game days after which
     * the server will forcefully disconnect you. */
    if lag > 20 * SECONDS {
        socket::close_connection(f, game, work);
        return;
    }

    /* Prevent showing the lag message every tick; just update it when needed. */
    if unsafe { LAST_LAG.get().read() } / SECONDS == lag / SECONDS {
        return;
    }

    unsafe { LAST_LAG.get().write(lag) };
    (s.show_lag)(lag / SECONDS);
}

/// `CheckConnection`'s function-local `static last_lag`.
static LAST_LAG: crate::game_state::GameState<i64> = crate::game_state::GameState::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_is_connected_abi(
    _s: &Services,
    f: &FrameServices,
) -> u8 {
    u8::from(is_connected(f))
}

/// `NetworkClient_Connected`: is called after a client is connected to the server.
pub(crate) fn network_client_connected(s: &Services, f: &FrameServices) {
    /* Set the frame-counter to 0 so nothing happens till we are ready */
    crate::network_frame::client_connected();

    debug!(net, 9, "Client::NetworkClient_Connected()");

    /* Request the game-info */
    send_join(s, f);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_connected(s: &Services, f: &FrameServices) {
    network_client_connected(s, f);
}

/// The `Receive_SERVER_CHECK_NEWGRFS` loop over `grf_count` identifiers; `find` is the native
/// `FindGRFConfig(FGCM_EXACT)` lookup with its not-found debug line.
fn check_newgrfs(
    services: &packet::Services,
    p: *mut PacketState,
    mut grf_count: u32,
    find: extern "C" fn(u32, *const u8) -> bool,
) -> NetworkRecvStatus {
    let mut ret = NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY;

    /* Check all GRFs */
    while grf_count > 0 {
        let c = unsafe { crate::network_game_info::deserialize_grf_identifier(p, services) };

        /* Check whether we know this GRF */
        if !find(c.grfid, c.md5sum.as_ptr()) {
            /* We do not know this GRF, bail out of initialization */
            ret = NetworkRecvStatus::NETWORK_RECV_STATUS_NEWGRF_MISMATCH;
        }
        grf_count -= 1;
    }

    ret
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
            check_newgrfs(packet::test_services(), &raw mut p, 2, check),
            NetworkRecvStatus::NETWORK_RECV_STATUS_NEWGRF_MISMATCH
        );
        assert_eq!(p.position, 24);
    }
}
