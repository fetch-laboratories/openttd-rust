/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants of `network_type.h`, pinned beside the company native services.
/// Team chat destination.
pub const DESTTYPE_TEAM: u32 = 1;
/// Encoded network public-key capacity, including the terminating NUL.
pub const NETWORK_PUBLIC_KEY_LENGTH: usize = 65;
/// `NetworkErrorCode` values sent by the frame protocol.
pub const NETWORK_ERROR_NOT_AUTHORIZED: u8 = 6;
pub const NETWORK_ERROR_TIMEOUT_PASSWORD: u8 = 16;
pub const NETWORK_ERROR_TIMEOUT_COMPUTER: u8 = 17;
pub const NETWORK_ERROR_TIMEOUT_MAP: u8 = 18;
pub const NETWORK_ERROR_TIMEOUT_JOIN: u8 = 19;
/// `NetworkRecvStatus` values returned by the frame protocol.
pub const NETWORK_RECV_STATUS_OKAY: u8 = 0;
pub const NETWORK_RECV_STATUS_MALFORMED_PACKET: u8 = 5;

pub const NETWORK_ERROR_NOT_EXPECTED: u8 = 7;

pub const NETWORK_ERROR_WRONG_REVISION: u8 = 8;
pub const NETWORK_ERROR_FULL: u8 = 14;
pub const NETWORK_ERROR_COMPANY_MISMATCH: u8 = 11;
pub const NETWORK_ERROR_INVALID_CLIENT_NAME: u8 = 20;
pub const NETWORK_ERROR_NAME_IN_USE: u8 = 9;
pub const NETWORK_ERROR_KICKED: u8 = 12;
pub const NETWORK_ERROR_ILLEGAL_PACKET: u8 = 4;
pub const NETWORK_RECV_STATUS_CLIENT_QUIT: u8 = 4;
pub const NETWORK_RECV_STATUS_SERVER_ERROR: u8 = 6;

/// First non-server client identity.
pub const CLIENT_ID_FIRST: u32 = 2;
pub const NETWORK_ERROR_GENERAL: u8 = 0;
pub const NETWORK_ERROR_DESYNC: u8 = 1;
pub const NETWORK_ERROR_SAVEGAME_FAILED: u8 = 2;
pub const NETWORK_ERROR_CONNECTION_LOST: u8 = 3;
pub const NETWORK_ERROR_NEWGRF_MISMATCH: u8 = 5;
pub const NETWORK_RECV_STATUS_DESYNC: u8 = 1;
pub const NETWORK_RECV_STATUS_NEWGRF_MISMATCH: u8 = 2;
pub const NETWORK_RECV_STATUS_SAVEGAME: u8 = 3;
pub const NETWORK_RECV_STATUS_SERVER_FULL: u8 = 7;
pub const NETWORK_RECV_STATUS_SERVER_BANNED: u8 = 8;
pub const NETWORK_RECV_STATUS_CLOSE_QUERY: u8 = 9;

/// Server identity.
pub const CLIENT_ID_SERVER: u32 = 1;
