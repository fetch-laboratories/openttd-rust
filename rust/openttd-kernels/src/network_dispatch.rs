/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Game packet dispatch and receive iteration. Packet ownership lives in the call-local
//! action frame, including the previous packet during the next receive, as in `ReceivePackets`.
//! Native retained handlers use that frame for exceptions and temporary strings/load arguments.
#![allow(unsafe_code, missing_docs)]
use crate::debug::debug;
use crate::network_action::{Actions, Resume};
use crate::network_frame::Services as Network;
use crate::network_socket::{self as socket, GameSocket, role};
use crate::network_tcp::{self as tcp, Packet};
use crate::network_type::NetworkRecvStatus;
use crate::packet;
use crate::raw_field::{read_field, write_field};
use crate::session::{Frame, Services as Session};
use crate::tcp_game::PacketGameType;
#[repr(C)]
pub struct Services {
    pub client_chat: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub client_id: extern "C" fn(*mut GameSocket) -> u32,
    pub client_move: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub client_rcon: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub client_set_name: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub map_load_args: extern "C" fn(*mut Frame) -> *mut crate::session::Load,
    pub packet_end: extern "C" fn(*mut Frame),
    pub server_banned: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_chat: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_client_info: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_config_update: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_error: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_external_chat: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_full: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_game_info: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_move: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub server_rcon: extern "C" fn(*mut GameSocket, *mut Packet, *mut Frame) -> u8,
    pub text_begin: extern "C" fn(*mut Frame) -> *mut std::ffi::c_void,
}

/// `NetworkGameSocketHandler::ReceiveInvalidPacket`: the base class's handler of every packet
/// type the socket's role does not receive.
fn receive_invalid_packet(
    s: &Network,
    game: *mut GameSocket,
    packet_type: PacketGameType,
) -> NetworkRecvStatus {
    debug!(
        net,
        0,
        "[tcp/game] Received illegal packet type {} from client {}",
        packet_type.0,
        (s.dispatch.client_id)(game)
    );
    NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET
}

/// `NetworkGameSocketHandler::HandlePacket`: handle the given packet, i.e. pass it to the
/// right parser receive command, dispatched on the socket's role.
#[allow(clippy::too_many_lines)] // The fixed game protocol dispatch table.
fn handle_packet(
    session: &Session,
    s: &Network,
    game: *mut GameSocket,
    shell: *mut Packet,
    frame: *mut Frame,
    work: Actions,
) -> NetworkRecvStatus {
    let p = (s.tcp.packet_record)(shell);
    let packet_type = PacketGameType(unsafe { packet::recv_uint8(p, s.packet) });
    if crate::network_frame::socket_has_quit(s, game) {
        debug!(
            net,
            0,
            "[tcp/game] Received invalid packet from client {}",
            (s.dispatch.client_id)(game)
        );
        socket::close_connection(s, game, work);
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    socket::set_last_packet(game, (s.tcp.now)());
    match packet_type {
        PacketGameType::PACKET_CLIENT_ACK => match read_field!(game, role) {
            role::SERVER => unsafe { crate::network_frame::receive_ack(s, game, p, frame, work) },
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_AUTH_RESPONSE => match read_field!(game, role) {
            role::SERVER => crate::network_auth::receive_response(s, game, p, work),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_CHAT => match read_field!(game, role) {
            role::SERVER => NetworkRecvStatus((s.dispatch.client_chat)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_COMMAND => match read_field!(game, role) {
            role::SERVER => crate::network_command::receive_client(s, game, p, frame, work),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_ERROR => match read_field!(game, role) {
            role::SERVER => {
                crate::network_connection::receive_error(s.connection, s, game, p, work)
            }
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_GAME_INFO => match read_field!(game, role) {
            role::SERVER => crate::network_connection::receive_game_info(s.connection, s, game),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_GETMAP => match read_field!(game, role) {
            role::SERVER => crate::network_connection::receive_getmap(s.connection, s, game, work),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_IDENTIFY => match read_field!(game, role) {
            role::SERVER => crate::network_connection::receive_identify(
                s.connection,
                s,
                game,
                p,
                (s.dispatch.text_begin)(frame),
                work,
            ),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_JOIN => match read_field!(game, role) {
            role::SERVER => crate::network_connection::receive_join(
                s.connection,
                s,
                game,
                p,
                (s.dispatch.text_begin)(frame),
                work,
            ),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_MAP_OK => match read_field!(game, role) {
            role::SERVER => crate::network_connection::receive_map_ok(s.connection, s, game, work),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_MOVE => match read_field!(game, role) {
            role::SERVER => NetworkRecvStatus((s.dispatch.client_move)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_NEWGRFS_CHECKED => match read_field!(game, role) {
            role::SERVER => {
                crate::network_connection::receive_newgrfs_checked(s.connection, s, game, work)
            }
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_QUIT => match read_field!(game, role) {
            role::SERVER => crate::network_connection::receive_quit(s.connection, s, game, work),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_RCON => match read_field!(game, role) {
            role::SERVER => NetworkRecvStatus((s.dispatch.client_rcon)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_CLIENT_SET_NAME => match read_field!(game, role) {
            role::SERVER => NetworkRecvStatus((s.dispatch.client_set_name)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_AUTH_REQUEST => match read_field!(game, role) {
            role::CLIENT => crate::network_auth::receive_request(s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_BANNED => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_banned(s.client),
            role::QUERY => NetworkRecvStatus((s.dispatch.server_banned)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_CHAT => match read_field!(game, role) {
            role::CLIENT => NetworkRecvStatus((s.dispatch.server_chat)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_CHECK_NEWGRFS => match read_field!(game, role) {
            role::CLIENT => {
                crate::network_client::receive_server_check_newgrfs(s.client, s, game, p)
            }
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_CLIENT_INFO => match read_field!(game, role) {
            role::CLIENT => NetworkRecvStatus((s.dispatch.server_client_info)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_COMMAND => match read_field!(game, role) {
            role::CLIENT => crate::network_command::receive_server(s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_CONFIG_UPDATE => match read_field!(game, role) {
            role::CLIENT => {
                NetworkRecvStatus((s.dispatch.server_config_update)(game, shell, frame))
            }
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_ENABLE_ENCRYPTION => match read_field!(game, role) {
            role::CLIENT => crate::network_auth::receive_enable(s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_ERROR => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_error(
                s.client,
                s,
                game,
                p,
                (s.dispatch.text_begin)(frame),
            ),
            role::QUERY => NetworkRecvStatus((s.dispatch.server_error)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_ERROR_QUIT => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_error_quit(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_EXTERNAL_CHAT => match read_field!(game, role) {
            role::CLIENT => {
                NetworkRecvStatus((s.dispatch.server_external_chat)(game, shell, frame))
            }
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_FRAME => match read_field!(game, role) {
            role::CLIENT => unsafe {
                crate::network_frame::receive_frame(
                    s,
                    crate::network_client::status(game),
                    socket::sync(game),
                    p,
                )
            },
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_FULL => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_full(s.client),
            role::QUERY => NetworkRecvStatus((s.dispatch.server_full)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_GAME_INFO => match read_field!(game, role) {
            role::QUERY => NetworkRecvStatus((s.dispatch.server_game_info)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_JOIN => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_join(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_MAP_BEGIN => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_map_begin(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_MAP_DATA => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_map_data(s.client, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_MAP_DONE => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_map_done(
                s.client,
                s,
                session,
                game,
                frame,
                (s.dispatch.map_load_args)(frame),
            ),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_MAP_SIZE => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_map_size(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_MOVE => match read_field!(game, role) {
            role::CLIENT => NetworkRecvStatus((s.dispatch.server_move)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_NEWGAME => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_newgame(s.client, game),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_QUIT => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_quit(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_RCON => match read_field!(game, role) {
            role::CLIENT => NetworkRecvStatus((s.dispatch.server_rcon)(game, shell, frame)),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_SHUTDOWN => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_shutdown(s.client, game),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_SYNC => match read_field!(game, role) {
            role::CLIENT => {
                crate::network_frame::receive_sync(s, crate::network_client::status(game), p)
            }
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_WAIT => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_wait(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        PacketGameType::PACKET_SERVER_WELCOME => match read_field!(game, role) {
            role::CLIENT => crate::network_client::receive_server_welcome(s.client, s, game, p),
            _ => receive_invalid_packet(s, game, packet_type),
        },
        _ => {
            debug!(
                net,
                0,
                "[tcp/game] Received invalid packet type {} from client {}",
                packet_type.0,
                (s.dispatch.client_id)(game)
            );
            socket::close_connection(s, game, work);
            NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET
        }
    }
}

/// `ServerNetworkGameSocketHandler::ReceivePacket` over `NetworkTCPSocketHandler::ReceivePacket`,
/// whose read loop is `network_tcp::receive_packet` and whose `PrepareToRead` check is here.
fn receive_packet(s: &Network, game: *mut GameSocket, work: Actions) -> *mut Packet {
    let server = read_field!(game, role) == role::SERVER;
    /* Only allow receiving when we have some buffer free; this value
     * can go negative, but eventually it will become positive again. */
    if server && read_field!(socket::sync(game), receive_limit) == 0 {
        return std::ptr::null_mut();
    }
    /* We can receive a packet, so try that and if needed account for
     * the amount of received data. */
    match unsafe {
        tcp::receive_packet(
            tcp::Context {
                services: s.tcp,
                socket: socket::tcp(game),
            },
            crate::network_frame::packet_check(s),
        )
    } {
        tcp::Receive::None => return std::ptr::null_mut(),
        tcp::Receive::Close => {
            socket::close_connection(s, game, work);
            return std::ptr::null_mut();
        }
        tcp::Receive::Ready => (),
    }
    let p = (s.tcp.receive_packet)(socket::tcp(game));
    if !unsafe { packet::prepare_to_read(p, s.packet) } {
        debug!(
            net,
            0, "Invalid packet received (too small / decryption error)"
        );
        socket::close_connection(s, game, work);
        return std::ptr::null_mut();
    }
    let shell = (s.tcp.receive_take)(socket::tcp(game));
    if server {
        let sync = socket::sync(game);
        write_field!(
            sync,
            receive_limit,
            read_field!(sync, receive_limit).wrapping_sub(read_field!(p, size))
        );
    }
    shell
}

/// `NetworkGameSocketHandler::ReceivePackets`: do the actual receiving of packets. As long as
/// `HandlePacket` returns OKAY packets are handled. Upon failure, or no more packets to process
/// the last result of `HandlePacket` is returned. `Err` is a recorded exception or a pending
/// resumable step.
pub(crate) fn receive_packets(
    session: &Session,
    s: &Network,
    game: *mut GameSocket,
    frame: *mut Frame,
    work: Actions,
) -> Result<NetworkRecvStatus, ()> {
    loop {
        let packet = receive_packet(s, game, work);
        if work.pending() {
            work.set_status(NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY);
            work.push(Resume::ReceiveEnd);
            return Err(());
        }
        work.drop_packet();
        if packet.is_null() {
            work.set_status(NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY);
            return Ok(NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY);
        }
        work.hold_packet(packet);
        let res = handle_packet(session, s, game, packet, frame, work);
        work.set_status(res);
        if work.pending() {
            work.push(Resume::ReceivedPacket(game));
            return Err(());
        }
        (s.dispatch.packet_end)(frame);
        if (session.failed)(frame) {
            work.drop_packet();
            return Err(());
        }
        if res != NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY {
            work.drop_packet();
            return Ok(res);
        }
    }
}

/// `ReceivePackets` resumed after a handler's pending step.
pub(crate) fn received_packet(
    session: &Session,
    s: &Network,
    game: *mut GameSocket,
    frame: *mut Frame,
    work: Actions,
) -> Result<NetworkRecvStatus, ()> {
    (s.dispatch.packet_end)(frame);
    let res = work.status();
    if res != NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY {
        work.drop_packet();
        return Ok(res);
    }
    receive_packets(session, s, game, frame, work)
}

/// # Safety
/// Serialized native entry; all sockets, packets, services and the action frame remain live
/// until its driver completes. A recorded exception returns before native unwinding.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_receive_packets(
    session: &Session,
    s: &Network,
    game: *mut GameSocket,
    frame: *mut Frame,
    work: Actions,
) {
    work.bind(s, session, frame);
    let _ = receive_packets(session, s, game, frame, work);
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkDispatchServices" ["probe"] => Services {
        client_chat,
        client_id,
        client_move,
        client_rcon,
        client_set_name,
        map_load_args,
        packet_end,
        server_banned,
        server_chat,
        server_client_info,
        server_config_update,
        server_error,
        server_external_chat,
        server_full,
        server_game_info,
        server_move,
        server_rcon,
        text_begin,
    };
}
