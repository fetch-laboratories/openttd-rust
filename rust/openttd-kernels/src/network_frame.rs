/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NetworkGameLoop`, `NetworkServer_Tick`, `NetworkCalculateLag` and the frame, sync and ack
//! packets. The frame counters and sync seeds are process-wide state that C++ aliases in place;
//! each game socket shell holds its `SocketSync` record in place as a base subobject.
//! Packets, sockets and console presentation stay native leaves. No borrow of either record
//! survives a leaf: fields are read and written by raw place access.
//! `RANDOM_DEBUG` builds (`ENABLE_NETWORK_SYNC_EVERY_FRAME`, `NETWORK_SEND_DOUBLE_SEED`) are
//! developer-only and outside this owner.
#![allow(unsafe_code, missing_docs)]

use crate::debug::debug;
use crate::game_state::GameState;
use crate::gfx_type::MILLISECONDS_PER_TICK;
use crate::network_action::{Actions, Resume};
use crate::network_command::next_client;
use crate::network_socket as socket;
use crate::network_type::{
    NETWORK_ERROR_NOT_AUTHORIZED, NETWORK_ERROR_TIMEOUT_COMPUTER, NETWORK_ERROR_TIMEOUT_JOIN,
    NETWORK_ERROR_TIMEOUT_MAP, NETWORK_ERROR_TIMEOUT_PASSWORD,
    NETWORK_RECV_STATUS_MALFORMED_PACKET, NETWORK_RECV_STATUS_OKAY,
};
use crate::packet::{self, State as PacketState};
use crate::raw_field::{read_field, write_field};
use crate::session::Frame;
use crate::timer_game_tick::DAY_TICKS;
use server_status::{
    ACTIVE, AUTH_GAME, AUTHORIZED, DONE_MAP, IDENTIFY, INACTIVE, MAP, MAP_WAIT, NEWGRFS_CHECK,
    PRE_ACTIVE,
};
use std::ffi::c_void;

const NANOSECONDS_PER_MILLISECOND: i64 = 1_000_000;

use crate::tcp_game::{PACKET_CLIENT_ACK, PACKET_SERVER_FRAME, PACKET_SERVER_SYNC};
/// `ServerNetworkGameSocketHandler::ClientStatus`.
pub mod server_status {
    pub const INACTIVE: u8 = 0;
    pub const AUTH_GAME: u8 = 1;
    pub const IDENTIFY: u8 = 2;
    pub const NEWGRFS_CHECK: u8 = 3;
    pub const AUTHORIZED: u8 = 4;
    pub const MAP_WAIT: u8 = 5;
    pub const MAP: u8 = 6;
    pub const DONE_MAP: u8 = 7;
    pub const PRE_ACTIVE: u8 = 8;
    pub const ACTIVE: u8 = 9;
}
use crate::network_client::status::ACTIVE as CLIENT_STATUS_ACTIVE;

crate::abi::shared_records! {
    fn abi_record;
    /// The frame globals of `network.cpp` and `network_client.cpp`.
    "OpenTTDNetworkFrameState" ["probe"] => pub struct NetworkFrameState {
        pub frame_counter_server: u32,
        pub frame_counter_max: u32,
        pub frame_counter: u32,
        pub last_sync_frame: u32,
        pub sync_seed_1: u32,
        pub sync_seed_2: u32,
        pub sync_frame: u32,
        /// `network_client.cpp`'s `last_ack_frame`.
        pub last_ack_frame: u32,
        pub network_first_time: bool,
    }
    /// Frame and lag fields of a game socket; the server-side fields are unused by the client.
    "OpenTTDNetworkSocketSync" ["probe"] => pub struct SocketSync {
        /// Server: bytes that may be received now.
        pub receive_limit: usize,
        pub last_frame: u32,
        pub last_frame_server: u32,
        /// Server: the last frame the right token came back.
        pub last_token_frame: u32,
        /// Server: whether the slow-client warning was printed.
        pub lag_test: u8,
        /// Server: the token sent and awaited; 0 requests a new one.
        pub last_token: u8,
        /// Client: the token to send back.
        pub token: u8,
    }
}

/// Native leaves of the frame protocol; `game` is a `NetworkGameSocketHandler *`, `socket` a
/// `NetworkClientSocket *`, and packets are `Packet *` owned until `packet_send`.
#[repr(C)]
pub struct Services {
    pub admins_receive: extern "C" fn(*mut Frame) -> bool,
    pub admins_send: extern "C" fn(*mut Frame) -> bool,
    pub auth_owner: extern "C" fn(
        *const crate::network_crypto::NetworkAuthenticationHandler,
    ) -> *mut crate::auth::Handler,
    pub bytes_per_frame: extern "C" fn() -> u16,
    pub bytes_per_frame_burst: extern "C" fn() -> u16,
    pub client: &'static crate::network_client::Services,
    pub client_desync: extern "C" fn(),
    /// `my_client` as a `NetworkGameSocketHandler *`.
    pub client_game: extern "C" fn() -> *mut socket::GameSocket,
    pub command: &'static crate::network_command::Services,
    pub connection: &'static crate::network_connection::Services,
    pub console_bad_acks: extern "C" fn(*mut c_void),
    pub console_download_timeout: extern "C" fn(*mut c_void),
    pub console_init_timeout: extern "C" fn(*mut c_void),
    pub console_join_timeout: extern "C" fn(*mut c_void),
    pub console_lagging: extern "C" fn(*mut c_void, u32, bool),
    pub console_password_timeout: extern "C" fn(*mut c_void),
    pub console_slow: extern "C" fn(*mut c_void, u32),
    pub dispatch: &'static crate::network_dispatch::Services,
    pub frame_freq: extern "C" fn() -> u8,
    pub interactive_random_range: extern "C" fn(u32) -> u32,
    pub lifecycle: &'static crate::network_lifecycle::Services,
    pub max_download_time: extern "C" fn() -> u16,
    pub max_init_time: extern "C" fn() -> u16,
    pub max_join_time: extern "C" fn() -> u16,
    pub max_lag_time: extern "C" fn() -> u16,
    pub max_password_time: extern "C" fn() -> u16,
    pub on_server_connect: extern "C" fn(*mut Frame) -> bool,
    pub packet: &'static packet::Services,
    pub query_close: extern "C" fn(*mut c_void, u8) -> u8,
    /// `_random.state`, read in place.
    pub random: *const u32,
    /// The developer-only `DEBUG_DUMP_COMMANDS` replay.
    pub replay_commands: extern "C" fn(),
    pub server_accept: extern "C" fn(*mut Frame, u32) -> bool,
    pub server_accept_next: extern "C" fn(*mut Frame) -> socket::Poll,
    pub server_poll: extern "C" fn(*mut Frame) -> socket::Poll,
    pub server_poll_client: extern "C" fn(*mut Frame, *mut socket::GameSocket) -> bool,
    pub server_poll_end: extern "C" fn(*mut Frame),
    pub show_lost_connection: extern "C" fn(),
    pub socket_delete: extern "C" fn(*mut c_void),
    pub sync_freq: extern "C" fn() -> u16,
    /// The server's daily `sync:` desync log line.
    pub sync_log: extern "C" fn(),
    pub tcp: &'static crate::network_tcp::Services,
    pub text_append: extern "C" fn(*mut c_void, *const u8, usize),
    pub text_reset: extern "C" fn(*mut c_void),
}

pub(crate) fn socket_has_quit(s: &Services, game: *mut socket::GameSocket) -> bool {
    unsafe { (s.packet.has_quit)(socket::base(game)) }
}
pub(crate) fn packet_check(s: &Services) -> extern "C" fn(bool) {
    crate::movement::table(s.command.company.shared).native_assert
}
pub(crate) fn packet_new(s: &Services, game: *mut socket::GameSocket, kind: u8) -> PacketState {
    unsafe {
        packet::new_outgoing(
            socket::base(game),
            s.packet,
            packet_check(s),
            kind,
            crate::network_config::COMPAT_MTU,
        )
    }
}
pub(crate) fn packet_send(s: &Services, game: *mut socket::GameSocket, p: &mut PacketState) {
    unsafe {
        packet::prepare_to_send(p, s.packet, packet_check(s));
    }
    (s.tcp.packet_enqueue)(socket::tcp(game), p);
}
pub(crate) fn packet_send_string(
    s: &Services,
    p: &mut PacketState,
    text: crate::services::ByteSpan,
) {
    let bytes = unsafe { crate::utf8::borrow(text.data, text.size) };
    p.send_string(bytes, packet_check(s));
}
pub(crate) fn packet_recv_string(
    s: &Services,
    p: *mut PacketState,
    text: *mut c_void,
    length: usize,
) {
    (s.text_reset)(text);
    unsafe {
        packet::recv_string_into(
            packet::ReceiveContext {
                state: p,
                services: s.packet,
            },
            packet_check(s),
            length,
            crate::string_type::StringValidationSettings::from_element(
                crate::string_type::StringValidationSetting::ReplaceWithQuestionMark,
            ),
            |bytes| {
                (s.text_append)(text, bytes.as_ptr(), bytes.len());
            },
        );
    }
}

static STATE: GameState<NetworkFrameState> = GameState::new(NetworkFrameState {
    frame_counter_server: 0,
    frame_counter_max: 0,
    frame_counter: 0,
    last_sync_frame: 0,
    sync_seed_1: 0,
    sync_seed_2: 0,
    sync_frame: 0,
    last_ack_frame: 0,
    network_first_time: false,
});

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_frame_storage() -> *mut NetworkFrameState {
    STATE.get()
}

macro_rules! state {
    ($field:ident) => {
        read_field!(STATE.get(), $field)
    };
    ($field:ident = $value:expr) => {
        write_field!(STATE.get(), $field, $value)
    };
}

pub(crate) fn frame_counter() -> u32 {
    state!(frame_counter)
}
pub(crate) fn frame_counter_max() -> u32 {
    state!(frame_counter_max)
}

fn random_state(s: &Services, index: usize) -> u32 {
    // SAFETY: `random` addresses the two-word native `_random.state`.
    unsafe { s.random.add(index).read() }
}

/// `NetworkCalculateLag`.
///
/// # Safety
/// `sync` is the live record of a server-side client socket.
pub unsafe fn calculate_lag(s: &Services, sync: *const SocketSync) -> u32 {
    let last_frame_server = read_field!(sync, last_frame_server);
    // The original stores the unsigned difference in an int, then returns it as uint.
    let mut lag = last_frame_server.wrapping_sub(read_field!(sync, last_frame));
    // A client that missed its daily ACK lags one more frame for every frame that passes.
    let limit = last_frame_server
        .wrapping_add(DAY_TICKS as u32)
        .wrapping_add(u32::from((s.frame_freq)()));
    if limit < frame_counter() {
        lag = lag.wrapping_add(frame_counter().wrapping_sub(limit));
    }
    lag
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_calculate_lag(
    s: &Services,
    sync: *const SocketSync,
) -> u32 {
    unsafe { calculate_lag(s, sync) }
}

/// `ServerNetworkGameSocketHandler::SendFrame`.
///
/// # Safety
/// `game` is a live server-side socket and `sync` its record.
pub(crate) unsafe fn send_frame(
    s: &Services,
    game: *mut socket::GameSocket,
    sync: *mut SocketSync,
) -> u8 {
    let mut p = packet_new(s, game, PACKET_SERVER_FRAME);
    p.send_uint32(frame_counter(), packet_check(s));
    p.send_uint32(frame_counter_max(), packet_check(s));
    // A zero token asks for a new one.
    if read_field!(sync, last_token) == 0 {
        // The uint8_t store truncates, as the original's.
        #[allow(clippy::cast_possible_truncation)]
        let token = ((s.interactive_random_range)(u32::from(u8::MAX - 1)) + 1) as u8;
        write_field!(sync, last_token, token);
        p.send_uint8(read_field!(sync, last_token), packet_check(s));
    }
    packet_send(s, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::SendSync`.
pub(crate) fn send_sync(s: &Services, game: *mut socket::GameSocket) -> u8 {
    debug!(
        net,
        9,
        "client[{}] SendSync(), frame_counter={}, sync_seed_1={}",
        (s.dispatch.client_id)(game),
        frame_counter(),
        state!(sync_seed_1)
    );
    let mut p = packet_new(s, game, PACKET_SERVER_SYNC);
    p.send_uint32(frame_counter(), packet_check(s));
    p.send_uint32(state!(sync_seed_1), packet_check(s));
    packet_send(s, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_ACK`.
///
/// # Safety
/// `socket` is a live server-side record; `p` is the packet being handled.
pub(crate) unsafe fn receive_ack(
    s: &Services,
    socket: *mut socket::GameSocket,
    p: *mut PacketState,
    session_frame: *mut Frame,
    work: Actions,
) -> u8 {
    work.network(s);
    let sync = socket::sync(socket);
    if socket::status(socket) < AUTHORIZED {
        return send_error(s, socket, NETWORK_ERROR_NOT_AUTHORIZED, work);
    }
    let frame = unsafe { packet::recv_uint32(p, s.packet) };
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_ACK(): frame={}",
        (s.dispatch.client_id)(socket),
        frame
    );
    // The client is catching up with the server.
    if socket::status(socket) == PRE_ACTIVE {
        if frame.wrapping_add(DAY_TICKS as u32) < frame_counter() {
            return NETWORK_RECV_STATUS_OKAY;
        }
        debug!(
            net,
            9,
            "client[{}] status = ACTIVE",
            (s.dispatch.client_id)(socket)
        );
        write_field!(socket, status, ACTIVE);
        write_field!(sync, last_token_frame, frame_counter());
        if !(s.on_server_connect)(session_frame) {
            return NETWORK_RECV_STATUS_OKAY;
        }
    }
    let token = unsafe { packet::recv_uint8(p, s.packet) };
    if token == read_field!(sync, last_token) {
        // The lag test measures the client, not the token round trip, which follows each ACK.
        write_field!(sync, last_token_frame, frame_counter());
        write_field!(sync, last_token, 0);
    }
    write_field!(sync, last_frame, frame);
    write_field!(sync, last_frame_server, frame_counter());
    NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendAck`.
fn send_ack(s: &Services) {
    debug!(net, 9, "Client::SendAck()");
    let game = (s.client_game)();
    let mut p = packet_new(s, game, PACKET_CLIENT_ACK);
    p.send_uint32(frame_counter(), packet_check(s));
    let sync = socket::sync((s.client_game)());
    p.send_uint8(read_field!(sync, token), packet_check(s));
    packet_send(s, game, &mut p);
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_FRAME`.
///
/// # Safety
/// `sync` is the live client socket's record and `p` the packet being handled.
pub(crate) unsafe fn receive_frame(
    s: &Services,
    status: u8,
    sync: *mut SocketSync,
    p: *mut PacketState,
) -> u8 {
    if status != CLIENT_STATUS_ACTIVE {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    state!(frame_counter_server = unsafe { packet::recv_uint32(p, s.packet) });
    state!(frame_counter_max = unsafe { packet::recv_uint32(p, s.packet) });
    if unsafe { packet::can_read(p, s.packet, 1, false) } {
        write_field!(sync, token, unsafe { packet::recv_uint8(p, s.packet) });
    }
    // Acknowledge once per day to save bandwidth.
    if !state!(network_first_time) && state!(last_ack_frame) < frame_counter() {
        state!(last_ack_frame = frame_counter().wrapping_add(DAY_TICKS as u32));
        debug!(net, 7, "Sent ACK at {}", frame_counter());
        send_ack(s);
    }
    NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_SYNC`.
pub(crate) fn receive_sync(s: &Services, status: u8, p: *mut PacketState) -> u8 {
    if status != CLIENT_STATUS_ACTIVE {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    state!(sync_frame = unsafe { packet::recv_uint32(p, s.packet) });
    state!(sync_seed_1 = unsafe { packet::recv_uint32(p, s.packet) });
    debug!(
        net,
        9,
        "Client::Receive_SERVER_SYNC(): sync_frame={}, sync_seed_1={}",
        state!(sync_frame),
        state!(sync_seed_1)
    );
    NETWORK_RECV_STATUS_OKAY
}

/// `NetworkClient_Connected`'s reset of the client frame state.
pub(crate) fn client_connected() {
    state!(frame_counter = 0);
    state!(frame_counter_server = 0);
    state!(last_ack_frame = 0);
}
/// Reset all three map-transfer frame counters, in native assignment order.
pub(crate) fn map_begin(frame: u32) {
    state!(frame_counter_max = frame);
    state!(frame_counter_server = frame);
    state!(frame_counter = frame);
}

fn state_loop(session: &crate::session::Services, frame: *mut Frame) -> Result<(), ()> {
    // SAFETY: The session frame is live through this game loop entry.
    if unsafe { crate::session::openttd_rust_session_state_loop_abi(session, frame) != 0 } {
        Ok(())
    } else {
        Err(())
    }
}

/// `NetworkInitialize`'s sync reset.
pub(crate) fn initialize() {
    state!(sync_frame = 0);
    state!(network_first_time = true);
}
/// `NetworkServerStart`'s counter reset.
pub(crate) fn server_started() {
    state!(frame_counter = 0);
    state!(frame_counter_server = 0);
    state!(frame_counter_max = 0);
    state!(last_sync_frame = 0);
}

/// `ClientNetworkGameSocketHandler::GameLoop`; `Ok(false)` when the client must stop.
fn client_game_loop(
    session: &crate::session::Services,
    s: &Services,
    frame: *mut Frame,
) -> Result<bool, ()> {
    state!(frame_counter = frame_counter().wrapping_add(1));
    (session.assert_local_company)();
    crate::network_command::execute_local_command_queue(s.command);
    state_loop(session, frame)?;
    let sync_frame = state!(sync_frame);
    if sync_frame != 0 {
        if sync_frame == frame_counter() {
            if state!(sync_seed_1) != random_state(s, 0) {
                (s.client_desync)();
                crate::network_client::client_error(
                    s.client,
                    s,
                    (s.client_game)(),
                    crate::network_type::NETWORK_RECV_STATUS_DESYNC,
                );
                return Ok(false);
            }
            // The first sync tells the server this client is ready at its frame.
            if state!(network_first_time) {
                state!(network_first_time = false);
                send_ack(s);
            }
            state!(sync_frame = 0);
        } else if sync_frame < frame_counter() {
            debug!(
                net,
                1,
                "Missed frame for sync-test: {} / {}",
                sync_frame,
                frame_counter()
            );
            state!(sync_frame = 0);
        }
    }
    Ok(true)
}

fn send_error(s: &Services, socket: *mut socket::GameSocket, error: u8, work: Actions) -> u8 {
    crate::network_connection::send_error(
        s.connection,
        s,
        socket,
        error,
        crate::network_connection::EMPTY_TEXT,
        work,
    )
}

/// `NetworkServer_Tick`.
fn server_tick(s: &Services, send_frame: bool, work: Actions) {
    let mut send_sync = false;
    if frame_counter() >= state!(last_sync_frame).wrapping_add(u32::from((s.sync_freq)())) {
        state!(last_sync_frame = frame_counter());
        send_sync = true;
    }
    server_tick_clients(s, send_frame, send_sync, 0, work);
}

#[allow(clippy::too_many_lines)]
pub(crate) fn server_tick_clients(
    s: &Services,
    send_frame: bool,
    send_sync: bool,
    mut cursor: usize,
    work: Actions,
) {
    while let Some(client) = next_client(s.command, &mut cursor) {
        let socket = client.socket;
        let game = socket;
        let sync = socket::sync(socket);
        let last_packet = socket::last_packet(socket);
        let status = socket::status(socket);
        // Bytes per frame accrue up to the burst amount.
        let limit = read_field!(sync, receive_limit)
            .wrapping_add(usize::from((s.bytes_per_frame)()))
            .min(usize::from((s.bytes_per_frame_burst)()));
        write_field!(sync, receive_limit, limit);

        let lag = unsafe { calculate_lag(s, sync) };
        let within = |milliseconds: u32| {
            last_packet + i64::from(milliseconds) * NANOSECONDS_PER_MILLISECOND > (s.tcp.now)()
        };
        match status {
            ACTIVE => {
                if lag > u32::from((s.max_lag_time)()) {
                    // A packet within the lag means a lagging client, else a lost connection.
                    let recent = within(lag.wrapping_mul(MILLISECONDS_PER_TICK));
                    (s.console_lagging)(socket::server(socket), lag, recent);
                    send_error(s, socket, NETWORK_ERROR_TIMEOUT_COMPUTER, work);
                    if work.pending() {
                        work.push(Resume::ServerTick(cursor, send_frame, send_sync));
                        return;
                    }
                    continue;
                }
                // Warn once, and only while packets still arrive.
                if lag > DAY_TICKS as u32 && read_field!(sync, lag_test) == 0 && within(2000) {
                    (s.console_slow)(socket::server(socket), frame_counter());
                    write_field!(sync, lag_test, 1);
                }
                let token_age = read_field!(sync, last_frame_server)
                    .wrapping_sub(read_field!(sync, last_token_frame));
                if token_age >= u32::from((s.max_lag_time)()) {
                    (s.console_bad_acks)(socket::server(socket));
                    send_error(s, socket, NETWORK_ERROR_TIMEOUT_COMPUTER, work);
                    if work.pending() {
                        work.push(Resume::ServerTick(cursor, send_frame, send_sync));
                        return;
                    }
                    continue;
                }
            }
            INACTIVE | IDENTIFY | NEWGRFS_CHECK | AUTHORIZED => {
                if lag > u32::from((s.max_init_time)()) {
                    (s.console_init_timeout)(socket::server(socket));
                    send_error(s, socket, NETWORK_ERROR_TIMEOUT_COMPUTER, work);
                    if work.pending() {
                        work.push(Resume::ServerTick(cursor, send_frame, send_sync));
                        return;
                    }
                    continue;
                }
            }
            MAP_WAIT => {
                // Keep a waiting client informed every two seconds.
                if (s.tcp.now)() > last_packet + 2000 * NANOSECONDS_PER_MILLISECOND {
                    crate::network_connection::send_wait(s.connection, s, socket);
                    socket::set_last_packet(socket, (s.tcp.now)());
                }
            }
            MAP => {
                if lag > u32::from((s.max_download_time)()) {
                    (s.console_download_timeout)(socket::server(socket));
                    send_error(s, socket, NETWORK_ERROR_TIMEOUT_MAP, work);
                    if work.pending() {
                        work.push(Resume::ServerTick(cursor, send_frame, send_sync));
                        return;
                    }
                    continue;
                }
            }
            DONE_MAP | PRE_ACTIVE => {
                if lag > u32::from((s.max_join_time)()) {
                    (s.console_join_timeout)(socket::server(socket));
                    send_error(s, socket, NETWORK_ERROR_TIMEOUT_JOIN, work);
                    if work.pending() {
                        work.push(Resume::ServerTick(cursor, send_frame, send_sync));
                        return;
                    }
                    continue;
                }
            }
            AUTH_GAME => {
                if lag > u32::from((s.max_password_time)()) {
                    (s.console_password_timeout)(socket::server(socket));
                    send_error(s, socket, NETWORK_ERROR_TIMEOUT_PASSWORD, work);
                    if work.pending() {
                        work.push(Resume::ServerTick(cursor, send_frame, send_sync));
                        return;
                    }
                    continue;
                }
            }
            // `STATUS_END`: the original's NOT_REACHED.
            _ => unreachable!(),
        }
        if status >= PRE_ACTIVE {
            unsafe {
                crate::network_command::handle_command_queue(
                    s.command,
                    socket,
                    socket::outgoing(client.socket),
                );
            }
            if send_frame {
                unsafe { self::send_frame(s, game, sync) };
            }
            if send_sync {
                self::send_sync(s, game);
            }
        }
    }
}

/// `NetworkGameLoop`, entered from the session loop while networking.
pub(crate) fn game_loop(
    session: &crate::session::Services,
    s: &Services,
    frame: *mut Frame,
    work: Actions,
) -> Result<(), ()> {
    if !(unsafe { &*session.company }.finance.networking)() {
        return Ok(());
    }
    let result = socket::receive_all(session, s, frame, work);
    if work.pending() {
        work.push(Resume::FrameReceived);
        return Err(());
    }
    game_loop_received(session, s, frame, result?, work)
}

pub(crate) fn game_loop_received(
    session: &crate::session::Services,
    s: &Services,
    frame: *mut Frame,
    received: bool,
    work: Actions,
) -> Result<(), ()> {
    if !received {
        return Ok(());
    }
    if (s.command.company.network_server)() {
        (s.sync_log)();
        (s.replay_commands)();
        if frame_counter() >= frame_counter_max() {
            // Only just before sending commands, so frame_freq > 1 sends one (un)pause.
            crate::network_connection::check_pause_on_join(session, s);
            crate::network_connection::check_min_active_clients(session, s);
            crate::network_command::distribute_commands(s.command);
        }
        let mut send_frame = false;
        state!(frame_counter = frame_counter().wrapping_add(1));
        if frame_counter() > frame_counter_max() {
            state!(frame_counter_max = frame_counter().wrapping_add(u32::from((s.frame_freq)())));
            send_frame = true;
        }
        (session.assert_local_company)();
        crate::network_command::execute_local_command_queue(s.command);
        state_loop(session, frame)?;
        state!(sync_seed_1 = random_state(s, 0));
        server_tick(s, send_frame, work);
        if work.pending() {
            work.push(Resume::FrameSend);
            return Err(());
        }
    } else if state!(frame_counter_server) > frame_counter() {
        // Catch up with the server; stop when things go bad.
        while state!(frame_counter_server) > frame_counter() {
            if !client_game_loop(session, s, frame)? {
                return Ok(());
            }
        }
    } else if frame_counter_max() > frame_counter() && !client_game_loop(session, s, frame)? {
        return Ok(());
    }
    send(s, frame, work)
}

pub(crate) fn send(s: &Services, frame: *mut Frame, work: Actions) -> Result<(), ()> {
    socket::send_all(s, frame, work)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkFrameServices" ["probe"] => Services {
        admins_receive,
        admins_send,
        auth_owner,
        bytes_per_frame,
        bytes_per_frame_burst,
        client,
        client_desync,
        client_game,
        command,
        connection,
        console_bad_acks,
        console_download_timeout,
        console_init_timeout,
        console_join_timeout,
        console_lagging,
        console_password_timeout,
        console_slow,
        dispatch,
        frame_freq,
        interactive_random_range,
        lifecycle,
        max_download_time,
        max_init_time,
        max_join_time,
        max_lag_time,
        max_password_time,
        on_server_connect,
        packet,
        query_close,
        random,
        replay_commands,
        server_accept,
        server_accept_next,
        server_poll,
        server_poll_client,
        server_poll_end,
        show_lost_connection,
        socket_delete,
        sync_freq,
        sync_log,
        tcp,
        text_append,
        text_reset,
    };
}
