/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NetworkJoinStatus` from `network_internal.h`.
/// Connecting to a server.
pub const NETWORK_JOIN_STATUS_CONNECTING: u8 = 0;
pub const NETWORK_JOIN_STATUS_AUTHORIZING: u8 = 1;
pub const NETWORK_JOIN_STATUS_WAITING: u8 = 2;
pub const NETWORK_JOIN_STATUS_DOWNLOADING: u8 = 3;
pub const NETWORK_JOIN_STATUS_PROCESSING: u8 = 4;
pub const NETWORK_JOIN_STATUS_REGISTERING: u8 = 5;
