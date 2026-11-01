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
