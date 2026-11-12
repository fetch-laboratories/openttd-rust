/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Game authentication transitions call the integrated authentication and packet owners.
//! Native shells keep provider/password lifetimes and adopt streams after key exchange.
//! Handler metadata is resolved once per entry; no borrow crosses a provider/UI callback.
#![allow(unsafe_code, missing_docs)]
use crate::auth;
use crate::debug::debug;
use crate::network_action::Actions;
use crate::network_client::ServerStatus;
use crate::network_crypto::{NetworkAuthenticationMethod, RequestResult, ResponseResult};
use crate::network_crypto_internal::X25519KeyExchangeSide;
use crate::network_frame::{Services, packet_check, packet_new, packet_send};
use crate::network_server::{self as server, ClientStatus};
use crate::network_socket::{self as socket, GameSocket};
use crate::network_type::{NetworkErrorCode, NetworkRecvStatus};
use crate::packet::State;
use crate::tcp_game::PacketGameType;

/// `ServerNetworkGameSocketHandler::SendAuthRequest`: request the game password.
pub(crate) fn send_request(
    f: &Services,
    socket: *mut GameSocket,
    work: Actions,
) -> NetworkRecvStatus {
    let s = f.connection;
    let native = socket::server(socket);
    debug!(
        net,
        9,
        "client[{}] SendAuthRequest()",
        (f.dispatch.client_id)(socket)
    );

    /* Invalid packet when status is anything but STATUS_INACTIVE or STATUS_AUTH_GAME. */
    if !matches!(
        server::status(socket),
        ClientStatus::STATUS_INACTIVE | ClientStatus::STATUS_AUTH_GAME
    ) {
        return socket::specialized_close(
            f,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET,
            work,
        );
    }

    debug!(
        net,
        9,
        "client[{}] status = AUTH_GAME",
        (f.dispatch.client_id)(socket)
    );
    server::set_status(socket, ClientStatus::STATUS_AUTH_GAME);

    /* Reset 'lag' counters */
    crate::network_connection::reset_lag(socket);

    if (s.auth)(native).is_null() {
        (s.auth_create)(native);
    }

    let owner = (f.auth_owner)((s.auth)(native));
    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_AUTH_REQUEST);
    unsafe {
        auth::send_request(owner, &raw mut p, packet_check(f));
    }

    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendAuthResponse`.
pub(crate) fn send_response(f: &Services) -> NetworkRecvStatus {
    let s = f.client;
    debug!(net, 9, "Client::SendAuthResponse()");

    let my_client = (f.client_game)();
    let owner = (f.auth_owner)((s.auth)(socket::shell(my_client)));
    let mut p = packet_new(f, my_client, PacketGameType::PACKET_CLIENT_AUTH_RESPONSE);
    unsafe {
        auth::send_response(owner, &raw mut p);
    }
    packet_send(f, my_client, &mut p);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_AUTH_REQUEST`.
pub(crate) fn receive_request(
    f: &Services,
    game: *mut GameSocket,
    p: *mut State,
) -> NetworkRecvStatus {
    let s = f.client;
    if !matches!(
        crate::network_client::status(game),
        ServerStatus::STATUS_JOIN | ServerStatus::STATUS_AUTH_GAME
    ) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::status = AUTH_GAME");
    crate::raw_field::write_field!(game, status, ServerStatus::STATUS_AUTH_GAME.0);

    debug!(net, 9, "Client::Receive_SERVER_AUTH_REQUEST()");

    let native = socket::shell(game);
    if (s.auth)(native).is_null() {
        (s.auth_create)(native);
    }
    let owner = (f.auth_owner)((s.auth)(native));
    match unsafe { auth::receive_request(owner, p) } {
        RequestResult::ReadyForResponse => send_response(f),

        RequestResult::AwaitUserInput => NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY,

        _ => NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET,
    }
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_ENABLE_ENCRYPTION`.
pub(crate) fn receive_enable(
    f: &Services,
    game: *mut GameSocket,
    p: *mut State,
) -> NetworkRecvStatus {
    let s = f.client;
    if crate::network_client::status(game) != ServerStatus::STATUS_AUTH_GAME {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let native = socket::shell(game);
    let handler = (s.auth)(native);
    if handler.is_null() {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let owner = (f.auth_owner)(handler);

    debug!(net, 9, "Client::Receive_SERVER_ENABLE_ENCRYPTION()");

    if !unsafe { auth::receive_enable(owner, p) } {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    let receive = unsafe { auth::stream_new(owner, X25519KeyExchangeSide::SERVER) };
    let send = unsafe { auth::stream_new(owner, X25519KeyExchangeSide::CLIENT) };
    (s.enable_encryption)(native, receive, send);

    debug!(net, 9, "Client::status = ENCRYPTED");
    crate::network_client::set_status(game, ServerStatus::STATUS_ENCRYPTED);

    crate::network_client::send_identify(s, f)
}

/// `GetErrorForAuthenticationMethod` (`network_server.cpp`).
fn get_error_for_authentication_method(method: NetworkAuthenticationMethod) -> NetworkErrorCode {
    match method {
        NetworkAuthenticationMethod::X25519_PAKE => NetworkErrorCode::NETWORK_ERROR_WRONG_PASSWORD,
        NetworkAuthenticationMethod::X25519_AuthorizedKey => {
            NetworkErrorCode::NETWORK_ERROR_NOT_ON_ALLOW_LIST
        }
        _ => crate::services_assert::not_reached(),
    }
}

/// `ServerNetworkGameSocketHandler::SendEnableEncryption`: notify the client that the
/// authentication has completed and tell that for the remainder of this socket encryption is
/// enabled.
fn send_enable_encryption(
    f: &Services,
    socket: *mut GameSocket,
    owner: *mut auth::Handler,
    work: Actions,
) -> NetworkRecvStatus {
    debug!(
        net,
        9,
        "client[{}] SendEnableEncryption()",
        (f.dispatch.client_id)(socket)
    );

    /* Invalid packet when status is anything but STATUS_AUTH_GAME. */
    if server::status(socket) != ClientStatus::STATUS_AUTH_GAME {
        return socket::specialized_close(
            f,
            socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET,
            work,
        );
    }

    let mut p = packet_new(f, socket, PacketGameType::PACKET_SERVER_ENABLE_ENCRYPTION);
    unsafe {
        auth::send_enable(owner, &raw mut p);
    }
    packet_send(f, socket, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_AUTH_RESPONSE`.
pub(crate) fn receive_response(
    f: &Services,
    socket: *mut GameSocket,
    p: *mut State,
    work: Actions,
) -> NetworkRecvStatus {
    let s = f.connection;
    if server::status(socket) != ClientStatus::STATUS_AUTH_GAME {
        return crate::network_connection::send_error(
            s,
            f,
            socket,
            NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED,
            crate::network_connection::EMPTY_TEXT,
            work,
        );
    }

    let native = socket::server(socket);
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_AUTH_RESPONSE()",
        (f.dispatch.client_id)(socket)
    );

    let owner = (f.auth_owner)((s.auth)(native));
    let authentication_method = unsafe { (*owner).method() };
    match unsafe { auth::receive_response(owner, p) } {
        ResponseResult::Authenticated => (),

        ResponseResult::RetryNextMethod => return send_request(f, socket, work),

        _ => {
            return crate::network_connection::send_error(
                s,
                f,
                socket,
                get_error_for_authentication_method(authentication_method),
                crate::network_connection::EMPTY_TEXT,
                work,
            );
        }
    }

    let status = send_enable_encryption(f, socket, owner, work);
    if status != NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY {
        return status;
    }

    let peer = unsafe { (*owner).peer() };
    let receive = unsafe { auth::stream_new(owner, X25519KeyExchangeSide::CLIENT) };
    let send = unsafe { auth::stream_new(owner, X25519KeyExchangeSide::SERVER) };
    (s.enable_encryption)(native, peer, receive, send);

    debug!(
        net,
        9,
        "client[{}] status = IDENTIFY",
        (f.dispatch.client_id)(socket)
    );
    server::set_status(socket, ClientStatus::STATUS_IDENTIFY);

    /* Reset 'lag' counters */
    crate::network_connection::reset_lag(socket);

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_auth_send_response(f: &Services) -> u8 {
    send_response(f).0
}
