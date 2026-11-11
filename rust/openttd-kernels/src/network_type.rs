/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants of `network_type.h`, pinned beside the company native services.
/// Team chat destination.
pub const DESTTYPE_TEAM: u32 = DestType::DESTTYPE_TEAM.0 as u32;
/// Encoded network public-key capacity, including the terminating NUL.
pub const NETWORK_PUBLIC_KEY_LENGTH: usize = 65;
/// `NetworkErrorCode` values sent by the frame protocol.
pub const NETWORK_ERROR_NOT_AUTHORIZED: u8 = NetworkErrorCode::NETWORK_ERROR_NOT_AUTHORIZED.0;
/// Native `NETWORK_ERROR_TIMEOUT_PASSWORD`.
pub const NETWORK_ERROR_TIMEOUT_PASSWORD: u8 = NetworkErrorCode::NETWORK_ERROR_TIMEOUT_PASSWORD.0;
/// Native `NETWORK_ERROR_TIMEOUT_COMPUTER`.
pub const NETWORK_ERROR_TIMEOUT_COMPUTER: u8 = NetworkErrorCode::NETWORK_ERROR_TIMEOUT_COMPUTER.0;
/// Native `NETWORK_ERROR_TIMEOUT_MAP`.
pub const NETWORK_ERROR_TIMEOUT_MAP: u8 = NetworkErrorCode::NETWORK_ERROR_TIMEOUT_MAP.0;
/// Native `NETWORK_ERROR_TIMEOUT_JOIN`.
pub const NETWORK_ERROR_TIMEOUT_JOIN: u8 = NetworkErrorCode::NETWORK_ERROR_TIMEOUT_JOIN.0;
/// Native `NETWORK_RECV_STATUS_OKAY`.
pub const NETWORK_RECV_STATUS_OKAY: u8 = NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY.0;
/// Native `NETWORK_RECV_STATUS_MALFORMED_PACKET`.
pub const NETWORK_RECV_STATUS_MALFORMED_PACKET: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET.0;

/// Native `NETWORK_ERROR_NOT_EXPECTED`.
pub const NETWORK_ERROR_NOT_EXPECTED: u8 = NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED.0;

/// Native `NETWORK_ERROR_WRONG_REVISION`.
pub const NETWORK_ERROR_WRONG_REVISION: u8 = NetworkErrorCode::NETWORK_ERROR_WRONG_REVISION.0;
/// Native `NETWORK_ERROR_FULL`.
pub const NETWORK_ERROR_FULL: u8 = NetworkErrorCode::NETWORK_ERROR_FULL.0;
/// Native `NETWORK_ERROR_COMPANY_MISMATCH`.
pub const NETWORK_ERROR_COMPANY_MISMATCH: u8 = NetworkErrorCode::NETWORK_ERROR_COMPANY_MISMATCH.0;
/// Native `NETWORK_ERROR_INVALID_CLIENT_NAME`.
pub const NETWORK_ERROR_INVALID_CLIENT_NAME: u8 =
    NetworkErrorCode::NETWORK_ERROR_INVALID_CLIENT_NAME.0;
/// Native `NETWORK_ERROR_NAME_IN_USE`.
pub const NETWORK_ERROR_NAME_IN_USE: u8 = NetworkErrorCode::NETWORK_ERROR_NAME_IN_USE.0;
/// Native `NETWORK_ERROR_KICKED`.
pub const NETWORK_ERROR_KICKED: u8 = NetworkErrorCode::NETWORK_ERROR_KICKED.0;
/// Native `NETWORK_ERROR_ILLEGAL_PACKET`.
pub const NETWORK_ERROR_ILLEGAL_PACKET: u8 = NetworkErrorCode::NETWORK_ERROR_ILLEGAL_PACKET.0;
/// Native `NETWORK_RECV_STATUS_CLIENT_QUIT`.
pub const NETWORK_RECV_STATUS_CLIENT_QUIT: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT.0;
/// Native `NETWORK_RECV_STATUS_SERVER_ERROR`.
pub const NETWORK_RECV_STATUS_SERVER_ERROR: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_ERROR.0;

/// First non-server client identity.
pub const CLIENT_ID_FIRST: u32 = ClientID::CLIENT_ID_FIRST.0;
/// Native `NETWORK_ERROR_GENERAL`.
pub const NETWORK_ERROR_GENERAL: u8 = NetworkErrorCode::NETWORK_ERROR_GENERAL.0;
/// Native `NETWORK_ERROR_DESYNC`.
pub const NETWORK_ERROR_DESYNC: u8 = NetworkErrorCode::NETWORK_ERROR_DESYNC.0;
/// Native `NETWORK_ERROR_SAVEGAME_FAILED`.
pub const NETWORK_ERROR_SAVEGAME_FAILED: u8 = NetworkErrorCode::NETWORK_ERROR_SAVEGAME_FAILED.0;
/// Native `NETWORK_ERROR_CONNECTION_LOST`.
pub const NETWORK_ERROR_CONNECTION_LOST: u8 = NetworkErrorCode::NETWORK_ERROR_CONNECTION_LOST.0;
/// Native `NETWORK_ERROR_NEWGRF_MISMATCH`.
pub const NETWORK_ERROR_NEWGRF_MISMATCH: u8 = NetworkErrorCode::NETWORK_ERROR_NEWGRF_MISMATCH.0;
/// Native `NETWORK_RECV_STATUS_DESYNC`.
pub const NETWORK_RECV_STATUS_DESYNC: u8 = NetworkRecvStatus::NETWORK_RECV_STATUS_DESYNC.0;
/// Native `NETWORK_RECV_STATUS_NEWGRF_MISMATCH`.
pub const NETWORK_RECV_STATUS_NEWGRF_MISMATCH: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_NEWGRF_MISMATCH.0;
/// Native `NETWORK_RECV_STATUS_SAVEGAME`.
pub const NETWORK_RECV_STATUS_SAVEGAME: u8 = NetworkRecvStatus::NETWORK_RECV_STATUS_SAVEGAME.0;
/// Native `NETWORK_RECV_STATUS_SERVER_FULL`.
pub const NETWORK_RECV_STATUS_SERVER_FULL: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_FULL.0;
/// Native `NETWORK_RECV_STATUS_SERVER_BANNED`.
pub const NETWORK_RECV_STATUS_SERVER_BANNED: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_SERVER_BANNED.0;
/// Native `NETWORK_RECV_STATUS_CLOSE_QUERY`.
pub const NETWORK_RECV_STATUS_CLOSE_QUERY: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_CLOSE_QUERY.0;
/// Native `NETWORK_RECV_STATUS_CONNECTION_LOST`.
pub const NETWORK_RECV_STATUS_CONNECTION_LOST: u8 =
    NetworkRecvStatus::NETWORK_RECV_STATUS_CONNECTION_LOST.0;

/// Server identity.
pub const CLIENT_ID_SERVER: u32 = ClientID::CLIENT_ID_SERVER.0;

/// Coordinator-local server visibility.
pub const SERVER_GAME_TYPE_LOCAL: u8 = ServerGameType::SERVER_GAME_TYPE_LOCAL.0;

/// Native `NETWORK_ERROR_WRONG_PASSWORD`.
pub const NETWORK_ERROR_WRONG_PASSWORD: u8 = NetworkErrorCode::NETWORK_ERROR_WRONG_PASSWORD.0;
/// Native `NETWORK_ERROR_NOT_ON_ALLOW_LIST`.
pub const NETWORK_ERROR_NOT_ON_ALLOW_LIST: u8 = NetworkErrorCode::NETWORK_ERROR_NOT_ON_ALLOW_LIST.0;

/// `INVALID_CLIENT_ID`.
pub const INVALID_CLIENT_ID: u32 = ClientID::INVALID_CLIENT_ID.0;

/// Server command rejection codes from `network_type.h`.
pub const NETWORK_ERROR_CHEATER: u8 = NetworkErrorCode::NETWORK_ERROR_CHEATER.0;
/// `NETWORK_ERROR_TOO_MANY_COMMANDS`.
pub const NETWORK_ERROR_TOO_MANY_COMMANDS: u8 = NetworkErrorCode::NETWORK_ERROR_TOO_MANY_COMMANDS.0;

/// `NetworkRecvStatus` from `network/core/core.h`: status of a network client; reasons why a
/// client has quit. Every underlying byte is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkRecvStatus(pub u8);

impl NetworkRecvStatus {
    /// Everything is okay.
    pub const NETWORK_RECV_STATUS_OKAY: Self = Self(0);
    /// A desync did occur.
    pub const NETWORK_RECV_STATUS_DESYNC: Self = Self(1);
    /// We did not have the required `NewGRFs`.
    pub const NETWORK_RECV_STATUS_NEWGRF_MISMATCH: Self = Self(2);
    /// Something went wrong (down)loading the savegame.
    pub const NETWORK_RECV_STATUS_SAVEGAME: Self = Self(3);
    /// The connection is lost gracefully. Other clients are already informed of this leaving client.
    pub const NETWORK_RECV_STATUS_CLIENT_QUIT: Self = Self(4);
    /// We apparently send a malformed packet.
    pub const NETWORK_RECV_STATUS_MALFORMED_PACKET: Self = Self(5);
    /// The server told us we made an error.
    pub const NETWORK_RECV_STATUS_SERVER_ERROR: Self = Self(6);
    /// The server is full.
    pub const NETWORK_RECV_STATUS_SERVER_FULL: Self = Self(7);
    /// The server has banned us.
    pub const NETWORK_RECV_STATUS_SERVER_BANNED: Self = Self(8);
    /// Done querying the server.
    pub const NETWORK_RECV_STATUS_CLOSE_QUERY: Self = Self(9);
    /// The connection is lost unexpectedly.
    pub const NETWORK_RECV_STATUS_CONNECTION_LOST: Self = Self(10);
}

/// Native `ClientPoolID`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientPoolID(pub u16);

impl ClientPoolID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(256);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

/// Native `AdminID`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdminID(pub u8);

impl AdminID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(16);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(255);
}

/// Native `NetworkVehicleType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkVehicleType(pub u8);

impl NetworkVehicleType {
    /// Native `NETWORK_VEH_TRAIN`.
    pub const NETWORK_VEH_TRAIN: Self = Self(0);
    /// Native `NETWORK_VEH_LORRY`.
    pub const NETWORK_VEH_LORRY: Self = Self(1);
    /// Native `NETWORK_VEH_BUS`.
    pub const NETWORK_VEH_BUS: Self = Self(2);
    /// Native `NETWORK_VEH_PLANE`.
    pub const NETWORK_VEH_PLANE: Self = Self(3);
    /// Native `NETWORK_VEH_SHIP`.
    pub const NETWORK_VEH_SHIP: Self = Self(4);
    /// Native `NETWORK_VEH_END`.
    pub const NETWORK_VEH_END: Self = Self(5);
}

/// Native `ServerGameType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServerGameType(pub u8);

impl ServerGameType {
    /// Native `SERVER_GAME_TYPE_LOCAL`.
    pub const SERVER_GAME_TYPE_LOCAL: Self = Self(0);
    /// Native `SERVER_GAME_TYPE_PUBLIC`.
    pub const SERVER_GAME_TYPE_PUBLIC: Self = Self(1);
    /// Native `SERVER_GAME_TYPE_INVITE_ONLY`.
    pub const SERVER_GAME_TYPE_INVITE_ONLY: Self = Self(2);
}

/// Native `ClientID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientID(pub u32);

impl ClientID {
    /// Client is not part of anything
    pub const INVALID_CLIENT_ID: Self = Self(0);
    /// Servers always have this ID
    pub const CLIENT_ID_SERVER: Self = Self(1);
    /// The first client ID
    pub const CLIENT_ID_FIRST: Self = Self(2);
}

/// Native `DestType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DestType(pub u8);

impl DestType {
    /// Send message/notice to all clients (All)
    pub const DESTTYPE_BROADCAST: Self = Self(0);
    /// Send message/notice to everyone playing the same company (Team)
    pub const DESTTYPE_TEAM: Self = Self(1);
    /// Send message/notice to only a certain client (Private)
    pub const DESTTYPE_CLIENT: Self = Self(2);
}

/// Native `NetworkAction`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkAction(pub u8);

impl NetworkAction {
    /// Native `NETWORK_ACTION_JOIN`.
    pub const NETWORK_ACTION_JOIN: Self = Self(0);
    /// Native `NETWORK_ACTION_LEAVE`.
    pub const NETWORK_ACTION_LEAVE: Self = Self(1);
    /// Native `NETWORK_ACTION_SERVER_MESSAGE`.
    pub const NETWORK_ACTION_SERVER_MESSAGE: Self = Self(2);
    /// Native `NETWORK_ACTION_CHAT`.
    pub const NETWORK_ACTION_CHAT: Self = Self(3);
    /// Native `NETWORK_ACTION_CHAT_COMPANY`.
    pub const NETWORK_ACTION_CHAT_COMPANY: Self = Self(4);
    /// Native `NETWORK_ACTION_CHAT_CLIENT`.
    pub const NETWORK_ACTION_CHAT_CLIENT: Self = Self(5);
    /// Native `NETWORK_ACTION_GIVE_MONEY`.
    pub const NETWORK_ACTION_GIVE_MONEY: Self = Self(6);
    /// Native `NETWORK_ACTION_NAME_CHANGE`.
    pub const NETWORK_ACTION_NAME_CHANGE: Self = Self(7);
    /// Native `NETWORK_ACTION_COMPANY_SPECTATOR`.
    pub const NETWORK_ACTION_COMPANY_SPECTATOR: Self = Self(8);
    /// Native `NETWORK_ACTION_COMPANY_JOIN`.
    pub const NETWORK_ACTION_COMPANY_JOIN: Self = Self(9);
    /// Native `NETWORK_ACTION_COMPANY_NEW`.
    pub const NETWORK_ACTION_COMPANY_NEW: Self = Self(10);
    /// Native `NETWORK_ACTION_KICKED`.
    pub const NETWORK_ACTION_KICKED: Self = Self(11);
    /// Native `NETWORK_ACTION_EXTERNAL_CHAT`.
    pub const NETWORK_ACTION_EXTERNAL_CHAT: Self = Self(12);
}

/// Native `NetworkErrorCode`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkErrorCode(pub u8);

impl NetworkErrorCode {
    /// Try to use this one like never
    pub const NETWORK_ERROR_GENERAL: Self = Self(0);
    /// Native `NETWORK_ERROR_DESYNC`.
    pub const NETWORK_ERROR_DESYNC: Self = Self(1);
    /// Native `NETWORK_ERROR_SAVEGAME_FAILED`.
    pub const NETWORK_ERROR_SAVEGAME_FAILED: Self = Self(2);
    /// Native `NETWORK_ERROR_CONNECTION_LOST`.
    pub const NETWORK_ERROR_CONNECTION_LOST: Self = Self(3);
    /// Native `NETWORK_ERROR_ILLEGAL_PACKET`.
    pub const NETWORK_ERROR_ILLEGAL_PACKET: Self = Self(4);
    /// Native `NETWORK_ERROR_NEWGRF_MISMATCH`.
    pub const NETWORK_ERROR_NEWGRF_MISMATCH: Self = Self(5);
    /// Native `NETWORK_ERROR_NOT_AUTHORIZED`.
    pub const NETWORK_ERROR_NOT_AUTHORIZED: Self = Self(6);
    /// Native `NETWORK_ERROR_NOT_EXPECTED`.
    pub const NETWORK_ERROR_NOT_EXPECTED: Self = Self(7);
    /// Native `NETWORK_ERROR_WRONG_REVISION`.
    pub const NETWORK_ERROR_WRONG_REVISION: Self = Self(8);
    /// Native `NETWORK_ERROR_NAME_IN_USE`.
    pub const NETWORK_ERROR_NAME_IN_USE: Self = Self(9);
    /// Native `NETWORK_ERROR_WRONG_PASSWORD`.
    pub const NETWORK_ERROR_WRONG_PASSWORD: Self = Self(10);
    /// Happens in `CLIENT_COMMAND`
    pub const NETWORK_ERROR_COMPANY_MISMATCH: Self = Self(11);
    /// Native `NETWORK_ERROR_KICKED`.
    pub const NETWORK_ERROR_KICKED: Self = Self(12);
    /// Native `NETWORK_ERROR_CHEATER`.
    pub const NETWORK_ERROR_CHEATER: Self = Self(13);
    /// Native `NETWORK_ERROR_FULL`.
    pub const NETWORK_ERROR_FULL: Self = Self(14);
    /// Native `NETWORK_ERROR_TOO_MANY_COMMANDS`.
    pub const NETWORK_ERROR_TOO_MANY_COMMANDS: Self = Self(15);
    /// Native `NETWORK_ERROR_TIMEOUT_PASSWORD`.
    pub const NETWORK_ERROR_TIMEOUT_PASSWORD: Self = Self(16);
    /// Native `NETWORK_ERROR_TIMEOUT_COMPUTER`.
    pub const NETWORK_ERROR_TIMEOUT_COMPUTER: Self = Self(17);
    /// Native `NETWORK_ERROR_TIMEOUT_MAP`.
    pub const NETWORK_ERROR_TIMEOUT_MAP: Self = Self(18);
    /// Native `NETWORK_ERROR_TIMEOUT_JOIN`.
    pub const NETWORK_ERROR_TIMEOUT_JOIN: Self = Self(19);
    /// Native `NETWORK_ERROR_INVALID_CLIENT_NAME`.
    pub const NETWORK_ERROR_INVALID_CLIENT_NAME: Self = Self(20);
    /// Native `NETWORK_ERROR_NOT_ON_ALLOW_LIST`.
    pub const NETWORK_ERROR_NOT_ON_ALLOW_LIST: Self = Self(21);
    /// Native `NETWORK_ERROR_NO_AUTHENTICATION_METHOD_AVAILABLE`.
    pub const NETWORK_ERROR_NO_AUTHENTICATION_METHOD_AVAILABLE: Self = Self(22);
    /// Native `NETWORK_ERROR_END`.
    pub const NETWORK_ERROR_END: Self = Self(23);
}
