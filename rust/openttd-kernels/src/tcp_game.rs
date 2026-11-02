/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `PacketGameType` from `network/core/tcp_game.h`.
#![allow(dead_code)]
pub const PACKET_SERVER_WELCOME: u8 = 16;
pub const PACKET_SERVER_WAIT: u8 = 19;
pub const PACKET_SERVER_MAP_BEGIN: u8 = 20;
pub const PACKET_SERVER_JOIN: u8 = 25;
pub const PACKET_SERVER_ERROR: u8 = 3;
pub const PACKET_SERVER_ERROR_QUIT: u8 = 43;
pub const PACKET_SERVER_QUIT: u8 = 41;
pub const PACKET_SERVER_SHUTDOWN: u8 = 9;
pub const PACKET_SERVER_NEWGAME: u8 = 8;
pub const PACKET_CLIENT_JOIN: u8 = 2;
pub const PACKET_CLIENT_IDENTIFY: u8 = 13;
pub const PACKET_CLIENT_NEWGRFS_CHECKED: u8 = 15;
pub const PACKET_CLIENT_GETMAP: u8 = 18;
pub const PACKET_CLIENT_MAP_OK: u8 = 24;
pub const PACKET_CLIENT_ERROR: u8 = 42;
pub const PACKET_CLIENT_QUIT: u8 = 40;
pub const PACKET_SERVER_FRAME: u8 = 26;
pub const PACKET_CLIENT_ACK: u8 = 27;
pub const PACKET_SERVER_SYNC: u8 = 28;
