/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Game authentication transitions call the integrated authentication and packet owners.
//! Native shells keep provider/password lifetimes and adopt streams after key exchange.
//! Handler metadata is resolved once per entry; no borrow crosses a provider/UI callback.
#![allow(unsafe_code, missing_docs)]
use crate::auth;
use crate::debug::debug;
use crate::network_action::Actions;
use crate::network_client::status as client;
use crate::network_crypto::{NetworkAuthenticationMethod, RequestResult, ResponseResult};
use crate::network_frame::{
    Services, packet_check, packet_new, packet_send, server_status as server,
};
use crate::network_socket::{self as socket, GameSocket};
use crate::network_type::{
    NETWORK_ERROR_NOT_EXPECTED, NETWORK_RECV_STATUS_MALFORMED_PACKET, NETWORK_RECV_STATUS_OKAY,
};
use crate::packet::State;
use crate::raw_field::write_field;
use crate::tcp_game::{
    PACKET_CLIENT_AUTH_RESPONSE, PACKET_SERVER_AUTH_REQUEST, PACKET_SERVER_ENABLE_ENCRYPTION,
};

pub(crate) fn send_request(f: &Services, game: *mut GameSocket, work: Actions) -> u8 {
    let s = f.connection;
    let native = socket::server(game);
    debug!(
        net,
        9,
        "client[{}] SendAuthRequest()",
        (f.dispatch.client_id)(game)
    );
    if !matches!(socket::status(game), server::INACTIVE | server::AUTH_GAME) {
        return socket::specialized_close(f, game, NETWORK_RECV_STATUS_MALFORMED_PACKET, work);
    }
    debug!(
        net,
        9,
        "client[{}] status = AUTH_GAME",
        (f.dispatch.client_id)(game)
    );
    write_field!(game, status, server::AUTH_GAME);
    crate::network_connection::reset_lag(s, game);
    if (s.auth)(native).is_null() {
        (s.auth_create)(native);
    }
    let owner = (f.auth_owner)((s.auth)(native));
    let mut p = packet_new(f, game, PACKET_SERVER_AUTH_REQUEST);
    unsafe {
        auth::send_request(owner, &raw mut p, packet_check(f));
    }
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn send_response(f: &Services) -> u8 {
    let s = f.client;
    debug!(net, 9, "Client::SendAuthResponse()");
    let game = (f.client_game)();
    let owner = (f.auth_owner)((s.auth)(socket::shell(game)));
    let mut p = packet_new(f, game, PACKET_CLIENT_AUTH_RESPONSE);
    unsafe {
        auth::send_response(owner, &raw mut p);
    }
    packet_send(f, game, &mut p);
    NETWORK_RECV_STATUS_OKAY
}

pub(crate) fn receive_request(f: &Services, game: *mut GameSocket, p: *mut State) -> u8 {
    let s = f.client;
    if !matches!(socket::status(game), client::JOIN | client::AUTH_GAME) {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    debug!(net, 9, "Client::status = AUTH_GAME");
    write_field!(game, status, client::AUTH_GAME);
    debug!(net, 9, "Client::Receive_SERVER_AUTH_REQUEST()");
    let native = socket::shell(game);
    if (s.auth)(native).is_null() {
        (s.auth_create)(native);
    }
    let owner = (f.auth_owner)((s.auth)(native));
    match unsafe { auth::receive_request(owner, p) } {
        RequestResult::ReadyForResponse => send_response(f),
        RequestResult::AwaitUserInput => NETWORK_RECV_STATUS_OKAY,
        _ => NETWORK_RECV_STATUS_MALFORMED_PACKET,
    }
}

pub(crate) fn receive_enable(f: &Services, game: *mut GameSocket, p: *mut State) -> u8 {
    let s = f.client;
    if socket::status(game) != client::AUTH_GAME {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let native = socket::shell(game);
    let handler = (s.auth)(native);
    if handler.is_null() {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let owner = (f.auth_owner)(handler);
    debug!(net, 9, "Client::Receive_SERVER_ENABLE_ENCRYPTION()");
    if !unsafe { auth::receive_enable(owner, p) } {
        return NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }
    let receive = unsafe {
        auth::stream_new(
            owner,
            crate::network_crypto_internal::X25519KeyExchangeSide::SERVER,
        )
    };
    let send = unsafe {
        auth::stream_new(
            owner,
            crate::network_crypto_internal::X25519KeyExchangeSide::CLIENT,
        )
    };
    (s.enable_encryption)(native, receive, send);
    debug!(net, 9, "Client::status = ENCRYPTED");
    write_field!(game, status, client::ENCRYPTED);
    crate::network_client::send_identify(s, f)
}

/// `GetErrorForAuthenticationMethod` (`network_server.cpp`).
fn get_error_for_authentication_method(
    method: NetworkAuthenticationMethod,
) -> crate::network_type::NetworkErrorCode {
    use crate::network_type::NetworkErrorCode;
    match method {
        NetworkAuthenticationMethod::X25519_PAKE => NetworkErrorCode::NETWORK_ERROR_WRONG_PASSWORD,
        NetworkAuthenticationMethod::X25519_AuthorizedKey => {
            NetworkErrorCode::NETWORK_ERROR_NOT_ON_ALLOW_LIST
        }
        _ => crate::services_assert::not_reached(),
    }
}
pub(crate) fn receive_response(
    f: &Services,
    game: *mut GameSocket,
    p: *mut State,
    work: Actions,
) -> u8 {
    let s = f.connection;
    if socket::status(game) != server::AUTH_GAME {
        return crate::network_connection::send_error(
            s,
            f,
            game,
            NETWORK_ERROR_NOT_EXPECTED,
            crate::network_connection::EMPTY_TEXT,
            work,
        );
    }
    let native = socket::server(game);
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_AUTH_RESPONSE()",
        (f.dispatch.client_id)(game)
    );
    let owner = (f.auth_owner)((s.auth)(native));
    let method = unsafe { (*owner).method() };
    match unsafe { auth::receive_response(owner, p) } {
        ResponseResult::Authenticated => (),
        ResponseResult::RetryNextMethod => return send_request(f, game, work),
        _ => {
            return crate::network_connection::send_error(
                s,
                f,
                game,
                get_error_for_authentication_method(method).0,
                crate::network_connection::EMPTY_TEXT,
                work,
            );
        }
    }
    debug!(
        net,
        9,
        "client[{}] SendEnableEncryption()",
        (f.dispatch.client_id)(game)
    );
    if socket::status(game) != server::AUTH_GAME {
        return socket::specialized_close(f, game, NETWORK_RECV_STATUS_MALFORMED_PACKET, work);
    }
    let mut p = packet_new(f, game, PACKET_SERVER_ENABLE_ENCRYPTION);
    unsafe {
        auth::send_enable(owner, &raw mut p);
    }
    packet_send(f, game, &mut p);
    let peer = unsafe { (*owner).peer() };
    let receive = unsafe {
        auth::stream_new(
            owner,
            crate::network_crypto_internal::X25519KeyExchangeSide::CLIENT,
        )
    };
    let send = unsafe {
        auth::stream_new(
            owner,
            crate::network_crypto_internal::X25519KeyExchangeSide::SERVER,
        )
    };
    (s.enable_encryption)(native, peer, receive, send);
    debug!(
        net,
        9,
        "client[{}] status = IDENTIFY",
        (f.dispatch.client_id)(game)
    );
    write_field!(game, status, server::IDENTIFY);
    crate::network_connection::reset_lag(s, game);
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_auth_send_response(f: &Services) -> u8 {
    send_response(f)
}
