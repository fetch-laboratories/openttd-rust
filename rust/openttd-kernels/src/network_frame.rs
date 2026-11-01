/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NetworkGameLoop`, `NetworkServer_Tick`, `NetworkCalculateLag` and the frame, sync and ack
//! packets. The frame counters and sync seeds are process-wide state that C++ aliases in place;
//! each game socket shell holds its `SocketSync` record in place as a base subobject.
//! Packets, sockets and console presentation stay native leaves. No borrow of either record
//! survives a leaf: fields are read and written by raw place access.
//! `RANDOM_DEBUG` builds (`ENABLE_NETWORK_SYNC_EVERY_FRAME`, `NETWORK_SEND_DOUBLE_SEED`) are
//! developer-only and outside this owner.
#![allow(unsafe_code, missing_docs)]

use crate::game_state::GameState;
use crate::network_command::{Client, next_client};
use crate::network_type::{
    NETWORK_ERROR_NOT_AUTHORIZED, NETWORK_ERROR_TIMEOUT_COMPUTER, NETWORK_ERROR_TIMEOUT_JOIN,
    NETWORK_ERROR_TIMEOUT_MAP, NETWORK_ERROR_TIMEOUT_PASSWORD,
    NETWORK_RECV_STATUS_MALFORMED_PACKET, NETWORK_RECV_STATUS_OKAY,
};
use crate::raw_field::{read_field, write_field};
use crate::session::Frame;
use crate::timer_game_tick::DAY_TICKS;
use server_status::{
    ACTIVE, AUTH_GAME, AUTHORIZED, DONE_MAP, IDENTIFY, INACTIVE, MAP, MAP_WAIT, NEWGRFS_CHECK,
    PRE_ACTIVE,
};
use std::ffi::c_void;

/// `MILLISECONDS_PER_TICK` from `gfx_type.h`.
const MILLISECONDS_PER_TICK: u32 = 27;
const NANOSECONDS_PER_MILLISECOND: i64 = 1_000_000;

/// `PacketGameType` values sent or received here.
pub const PACKET_SERVER_FRAME: u8 = 26;
pub const PACKET_CLIENT_ACK: u8 = 27;
pub const PACKET_SERVER_SYNC: u8 = 28;

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
/// `ClientNetworkGameSocketHandler::STATUS_ACTIVE`.
pub const CLIENT_STATUS_ACTIVE: u8 = 8;

/// The frame globals of `network.cpp` and `network_client.cpp`.
#[repr(C)]
pub struct State {
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
#[repr(C)]
pub struct SocketSync {
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

/// Native leaves of the frame protocol; `game` is a `NetworkGameSocketHandler *`, `socket` a
/// `NetworkClientSocket *`, and packets are `Packet *` owned until `packet_send`.
#[repr(C)]
pub struct Services {
    pub command: &'static crate::network_command::Services,
    pub connection: &'static crate::network_connection::Services,
    /// `_random.state`, read in place.
    pub random: *const u32,
    pub bytes_per_frame: extern "C" fn() -> u16,
    pub bytes_per_frame_burst: extern "C" fn() -> u16,
    pub client_desync: extern "C" fn(),
    /// `my_client` as a `NetworkGameSocketHandler *`.
    pub client_game: extern "C" fn() -> *mut c_void,
    pub client_sync: extern "C" fn() -> *mut SocketSync,
    pub console_bad_acks: extern "C" fn(*mut c_void),
    pub console_download_timeout: extern "C" fn(*mut c_void),
    pub console_init_timeout: extern "C" fn(*mut c_void),
    pub console_join_timeout: extern "C" fn(*mut c_void),
    pub console_lagging: extern "C" fn(*mut c_void, u32, bool),
    pub console_password_timeout: extern "C" fn(*mut c_void),
    pub console_slow: extern "C" fn(*mut c_void, u32),
    pub debug_ack_sent: extern "C" fn(u32),
    pub debug_active: extern "C" fn(*mut c_void),
    pub debug_missed_sync: extern "C" fn(u32, u32),
    pub debug_receive_ack: extern "C" fn(*mut c_void, u32),
    pub debug_receive_sync: extern "C" fn(u32, u32),
    pub debug_send_ack: extern "C" fn(),
    pub debug_send_sync: extern "C" fn(*mut c_void, u32, u32),
    pub frame_freq: extern "C" fn() -> u8,
    pub interactive_random_range: extern "C" fn(u32) -> u32,
    pub max_download_time: extern "C" fn() -> u16,
    pub max_init_time: extern "C" fn() -> u16,
    pub max_join_time: extern "C" fn() -> u16,
    pub max_lag_time: extern "C" fn() -> u16,
    pub max_password_time: extern "C" fn() -> u16,
    /// `std::chrono::steady_clock::now()` as nanoseconds since its epoch.
    pub now: extern "C" fn() -> i64,
    pub on_server_connect: extern "C" fn(),
    pub packet_can_read: extern "C" fn(*mut c_void, usize) -> bool,
    pub packet_new: extern "C" fn(*mut c_void, u8) -> *mut c_void,
    pub packet_recv_uint8: extern "C" fn(*mut c_void) -> u8,
    pub packet_recv_uint32: extern "C" fn(*mut c_void) -> u32,
    pub packet_send: extern "C" fn(*mut c_void, *mut c_void),
    pub packet_send_uint8: extern "C" fn(*mut c_void, u8),
    pub packet_send_uint32: extern "C" fn(*mut c_void, u32),
    /// `NetworkReceive`; records an exception in the frame and returns false.
    pub receive: extern "C" fn(*mut Frame, *mut bool) -> bool,
    /// The `DEBUG_DUMP_COMMANDS` replay of `commands.log`; empty without that define.
    pub replay_commands: extern "C" fn(),
    /// `NetworkSend`; records an exception in the frame and returns false.
    pub send: extern "C" fn(*mut Frame) -> bool,
    pub send_error: extern "C" fn(*mut c_void, u8) -> u8,
    pub sync_freq: extern "C" fn() -> u16,
    /// The server's daily `sync:` desync log line.
    pub sync_log: extern "C" fn(),
    /// `last_packet` becomes now after `SendWait`.
    pub wait_sent: extern "C" fn(*mut c_void),
}

static STATE: GameState<State> = GameState::new(State {
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
pub extern "C" fn openttd_rust_network_frame_storage() -> *mut State {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_send_frame(
    s: &Services,
    game: *mut c_void,
    sync: *mut SocketSync,
) -> u8 {
    let p = (s.packet_new)(game, PACKET_SERVER_FRAME);
    (s.packet_send_uint32)(p, frame_counter());
    (s.packet_send_uint32)(p, frame_counter_max());
    // A zero token asks for a new one.
    if read_field!(sync, last_token) == 0 {
        // The uint8_t store truncates, as the original's.
        #[allow(clippy::cast_possible_truncation)]
        let token = ((s.interactive_random_range)(u32::from(u8::MAX - 1)) + 1) as u8;
        write_field!(sync, last_token, token);
        (s.packet_send_uint8)(p, read_field!(sync, last_token));
    }
    (s.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::SendSync`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_sync(s: &Services, game: *mut c_void) -> u8 {
    (s.debug_send_sync)(game, frame_counter(), state!(sync_seed_1));
    let p = (s.packet_new)(game, PACKET_SERVER_SYNC);
    (s.packet_send_uint32)(p, frame_counter());
    (s.packet_send_uint32)(p, state!(sync_seed_1));
    (s.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_ACK`.
///
/// # Safety
/// `socket` is a live server-side socket, `game` the same socket, `status` its status field
/// and `sync` its record; `p` is the packet being handled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_receive_ack(
    s: &Services,
    socket: *mut c_void,
    game: *mut c_void,
    status: *mut u8,
    sync: *mut SocketSync,
    p: *mut c_void,
) -> u8 {
    // SAFETY: `status` is the socket's one-byte status field (see above).
    if unsafe { *status } < AUTHORIZED {
        return (s.send_error)(socket, NETWORK_ERROR_NOT_AUTHORIZED);
    }
    let frame = (s.packet_recv_uint32)(p);
    (s.debug_receive_ack)(game, frame);
    // The client is catching up with the server.
    if unsafe { *status } == PRE_ACTIVE {
        if frame.wrapping_add(DAY_TICKS as u32) < frame_counter() {
            return NETWORK_RECV_STATUS_OKAY;
        }
        (s.debug_active)(game);
        unsafe { *status = ACTIVE };
        write_field!(sync, last_token_frame, frame_counter());
        (s.on_server_connect)();
    }
    let token = (s.packet_recv_uint8)(p);
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
    (s.debug_send_ack)();
    let game = (s.client_game)();
    let p = (s.packet_new)(game, PACKET_CLIENT_ACK);
    (s.packet_send_uint32)(p, frame_counter());
    let sync = (s.client_sync)();
    (s.packet_send_uint8)(p, read_field!(sync, token));
    (s.packet_send)(game, p);
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_FRAME`.
///
/// # Safety
/// `sync` is the live client socket's record and `p` the packet being handled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_receive_frame(
    s: &Services,
    status: u8,
    sync: *mut SocketSync,
    p: *mut c_void,
) -> u8 {
    if status != CLIENT_STATUS_ACTIVE {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    state!(frame_counter_server = (s.packet_recv_uint32)(p));
    state!(frame_counter_max = (s.packet_recv_uint32)(p));
    if (s.packet_can_read)(p, 1) {
        write_field!(sync, token, (s.packet_recv_uint8)(p));
    }
    // Acknowledge once per day to save bandwidth.
    if !state!(network_first_time) && state!(last_ack_frame) < frame_counter() {
        state!(last_ack_frame = frame_counter().wrapping_add(DAY_TICKS as u32));
        (s.debug_ack_sent)(frame_counter());
        send_ack(s);
    }
    NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_SYNC`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_sync(
    s: &Services,
    status: u8,
    p: *mut c_void,
) -> u8 {
    if status != CLIENT_STATUS_ACTIVE {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    state!(sync_frame = (s.packet_recv_uint32)(p));
    state!(sync_seed_1 = (s.packet_recv_uint32)(p));
    (s.debug_receive_sync)(state!(sync_frame), state!(sync_seed_1));
    NETWORK_RECV_STATUS_OKAY
}

/// `NetworkClient_Connected`'s reset of the client frame state.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_client_connected() {
    state!(frame_counter = 0);
    state!(frame_counter_server = 0);
    state!(last_ack_frame = 0);
}

fn state_loop(session: &crate::session::Services, frame: *mut Frame) -> Result<(), ()> {
    // SAFETY: The session frame is live through this game loop entry.
    if unsafe { crate::session::openttd_rust_session_state_loop(session, frame) } {
        Ok(())
    } else {
        Err(())
    }
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
                return Ok(false);
            }
            // The first sync tells the server this client is ready at its frame.
            if state!(network_first_time) {
                state!(network_first_time = false);
                send_ack(s);
            }
            state!(sync_frame = 0);
        } else if sync_frame < frame_counter() {
            (s.debug_missed_sync)(sync_frame, frame_counter());
            state!(sync_frame = 0);
        }
    }
    Ok(true)
}

/// `NetworkServer_Tick`.
fn server_tick(s: &Services, send_frame: bool) {
    let mut send_sync = false;
    if frame_counter() >= state!(last_sync_frame).wrapping_add(u32::from((s.sync_freq)())) {
        state!(last_sync_frame = frame_counter());
        send_sync = true;
    }
    let mut cursor = 0;
    while let Some(client) = next_client(s.command, &mut cursor) {
        let Client {
            socket,
            game,
            sync,
            last_packet,
            status,
            ..
        } = client;
        // Bytes per frame accrue up to the burst amount.
        let limit = read_field!(sync, receive_limit)
            .wrapping_add(usize::from((s.bytes_per_frame)()))
            .min(usize::from((s.bytes_per_frame_burst)()));
        write_field!(sync, receive_limit, limit);

        let lag = unsafe { calculate_lag(s, sync) };
        let within = |milliseconds: u32| {
            last_packet + i64::from(milliseconds) * NANOSECONDS_PER_MILLISECOND > (s.now)()
        };
        match status {
            ACTIVE => {
                if lag > u32::from((s.max_lag_time)()) {
                    // A packet within the lag means a lagging client, else a lost connection.
                    let recent = within(lag.wrapping_mul(MILLISECONDS_PER_TICK));
                    (s.console_lagging)(socket, lag, recent);
                    (s.send_error)(socket, NETWORK_ERROR_TIMEOUT_COMPUTER);
                    continue;
                }
                // Warn once, and only while packets still arrive.
                if lag > DAY_TICKS as u32 && read_field!(sync, lag_test) == 0 && within(2000) {
                    (s.console_slow)(socket, frame_counter());
                    write_field!(sync, lag_test, 1);
                }
                let token_age = read_field!(sync, last_frame_server)
                    .wrapping_sub(read_field!(sync, last_token_frame));
                if token_age >= u32::from((s.max_lag_time)()) {
                    (s.console_bad_acks)(socket);
                    (s.send_error)(socket, NETWORK_ERROR_TIMEOUT_COMPUTER);
                    continue;
                }
            }
            INACTIVE | IDENTIFY | NEWGRFS_CHECK | AUTHORIZED => {
                if lag > u32::from((s.max_init_time)()) {
                    (s.console_init_timeout)(socket);
                    (s.send_error)(socket, NETWORK_ERROR_TIMEOUT_COMPUTER);
                    continue;
                }
            }
            MAP_WAIT => {
                // Keep a waiting client informed every two seconds.
                if (s.now)() > last_packet + 2000 * NANOSECONDS_PER_MILLISECOND {
                    crate::network_connection::openttd_rust_network_send_wait(
                        s.connection,
                        s,
                        socket,
                    );
                    (s.wait_sent)(socket);
                }
            }
            MAP => {
                if lag > u32::from((s.max_download_time)()) {
                    (s.console_download_timeout)(socket);
                    (s.send_error)(socket, NETWORK_ERROR_TIMEOUT_MAP);
                    continue;
                }
            }
            DONE_MAP | PRE_ACTIVE => {
                if lag > u32::from((s.max_join_time)()) {
                    (s.console_join_timeout)(socket);
                    (s.send_error)(socket, NETWORK_ERROR_TIMEOUT_JOIN);
                    continue;
                }
            }
            AUTH_GAME => {
                if lag > u32::from((s.max_password_time)()) {
                    (s.console_password_timeout)(socket);
                    (s.send_error)(socket, NETWORK_ERROR_TIMEOUT_PASSWORD);
                    continue;
                }
            }
            // `STATUS_END`: the original's NOT_REACHED.
            _ => unreachable!(),
        }
        if status >= PRE_ACTIVE {
            unsafe {
                crate::network_command::handle_command_queue(s.command, socket, client.outgoing);
            }
            if send_frame {
                unsafe { openttd_rust_network_send_frame(s, game, sync) };
            }
            if send_sync {
                openttd_rust_network_send_sync(s, game);
            }
        }
    }
}

/// `NetworkGameLoop`, entered from the session loop while networking.
pub(crate) fn game_loop(
    session: &crate::session::Services,
    s: &Services,
    frame: *mut Frame,
) -> Result<(), ()> {
    if !(session.networking)() {
        return Ok(());
    }
    let mut received = false;
    if !(s.receive)(frame, &raw mut received) {
        return Err(());
    }
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
        server_tick(s, send_frame);
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
    if (s.send)(frame) { Ok(()) } else { Err(()) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkFrameState" ["probe"] => State {
        frame_counter,
        frame_counter_max,
        frame_counter_server,
        last_ack_frame,
        last_sync_frame,
        network_first_time,
        sync_frame,
        sync_seed_1,
        sync_seed_2,
    };
    "OpenTTDNetworkSocketSync" ["probe"] => SocketSync {
        lag_test,
        last_frame,
        last_frame_server,
        last_token,
        last_token_frame,
        receive_limit,
        token,
    };
    "OpenTTDNetworkFrameServices" ["probe"] => Services {
        bytes_per_frame,
        bytes_per_frame_burst,
        client_desync,
        client_game,
        client_sync,
        command,
        connection,
        console_bad_acks,
        console_download_timeout,
        console_init_timeout,
        console_join_timeout,
        console_lagging,
        console_password_timeout,
        console_slow,
        debug_ack_sent,
        debug_active,
        debug_missed_sync,
        debug_receive_ack,
        debug_receive_sync,
        debug_send_ack,
        debug_send_sync,
        frame_freq,
        interactive_random_range,
        max_download_time,
        max_init_time,
        max_join_time,
        max_lag_time,
        max_password_time,
        now,
        on_server_connect,
        packet_can_read,
        packet_new,
        packet_recv_uint8,
        packet_recv_uint32,
        packet_send,
        packet_send_uint8,
        packet_send_uint32,
        random,
        receive,
        replay_commands,
        send,
        send_error,
        sync_freq,
        sync_log,
        wait_sent,
    };
}
