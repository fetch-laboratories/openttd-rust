/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ServerNetworkGameSocketHandler::ClientStatus` from `network_server.h`, read and written in
//! place on the server-side game socket shell.
#![allow(unsafe_code)]

use crate::network_socket::{self as socket, GameSocket};

/// Status of a client, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientStatus(pub u8);

impl ClientStatus {
    /// The client is not connected nor active.
    pub const STATUS_INACTIVE: Self = Self(0);
    /// The client is authorizing with game (server) password.
    pub const STATUS_AUTH_GAME: Self = Self(1);
    /// The client is identifying itself.
    pub const STATUS_IDENTIFY: Self = Self(2);
    /// The client is checking `NewGRFs`.
    pub const STATUS_NEWGRFS_CHECK: Self = Self(3);
    /// The client is authorized.
    pub const STATUS_AUTHORIZED: Self = Self(4);
    /// The client is waiting as someone else is downloading the map.
    pub const STATUS_MAP_WAIT: Self = Self(5);
    /// The client is downloading the map.
    pub const STATUS_MAP: Self = Self(6);
    /// The client has downloaded the map.
    pub const STATUS_DONE_MAP: Self = Self(7);
    /// The client is catching up the delayed frames.
    pub const STATUS_PRE_ACTIVE: Self = Self(8);
    /// The client is active within in the game.
    pub const STATUS_ACTIVE: Self = Self(9);
    /// Must ALWAYS be on the end of this list!! (period).
    pub const STATUS_END: Self = Self(10);
}

/// `ServerNetworkGameSocketHandler::status` of a live server-side socket.
pub(crate) fn status(cs: *mut GameSocket) -> ClientStatus {
    ClientStatus(socket::status(cs))
}

/// Stores `ServerNetworkGameSocketHandler::status` of a live server-side socket.
pub(crate) fn set_status(cs: *mut GameSocket, status: ClientStatus) {
    crate::raw_field::write_field!(cs, status, status.0);
}
