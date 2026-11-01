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
