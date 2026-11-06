/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native `track_type.h` values used by movement policies.
#![allow(missing_docs)]
pub const TRACK_BIT_NONE: u8 = 0;
pub const INVALID_TRACKDIR: u8 = 0xFF;
pub const TRACK_BIT_X: u8 = 1;
pub const TRACK_BIT_Y: u8 = 2;
pub const TRACK_BIT_UPPER: u8 = 4;
pub const TRACK_BIT_LOWER: u8 = 8;
pub const TRACK_BIT_LEFT: u8 = 16;
pub const TRACK_BIT_RIGHT: u8 = 32;
pub const TRACK_BIT_ALL: u8 = 63;
pub const TRACK_BIT_WORMHOLE: u8 = 64;
pub const TRACK_BIT_DEPOT: u8 = 128;
pub const TRACK_UPPER: u8 = 2;
pub const TRACK_LOWER: u8 = 3;
pub const TRACKDIR_BIT_X_NE: u16 = 1;
pub const TRACKDIR_BIT_Y_SE: u16 = 2;
pub const TRACKDIR_BIT_UPPER_E: u16 = 4;
pub const TRACKDIR_BIT_LOWER_E: u16 = 8;
pub const TRACKDIR_BIT_LEFT_S: u16 = 16;
pub const TRACKDIR_BIT_RIGHT_S: u16 = 32;
pub const TRACKDIR_BIT_X_SW: u16 = 256;
pub const TRACKDIR_BIT_Y_NW: u16 = 512;
pub const TRACKDIR_BIT_UPPER_W: u16 = 1024;
pub const TRACKDIR_BIT_LOWER_W: u16 = 2048;
pub const TRACKDIR_BIT_LEFT_N: u16 = 4096;
pub const TRACKDIR_BIT_RIGHT_N: u16 = 8192;

/// `TRACK_BIT_HORZ`.
pub const TRACK_BIT_HORZ: u8 = TRACK_BIT_UPPER | TRACK_BIT_LOWER;
/// `TRACK_BIT_VERT`.
pub const TRACK_BIT_VERT: u8 = TRACK_BIT_LEFT | TRACK_BIT_RIGHT;
/// `TRACKDIR_X_NE`.
pub const TRACKDIR_X_NE: u8 = 0;
/// `TRACKDIR_Y_SE`.
pub const TRACKDIR_Y_SE: u8 = 1;
/// `TRACKDIR_X_SW`.
pub const TRACKDIR_X_SW: u8 = 8;
/// `TRACKDIR_Y_NW`.
pub const TRACKDIR_Y_NW: u8 = 9;

/// `TRACKDIR_BIT_MASK`.
pub const TRACKDIR_BIT_MASK: u32 = 0x3f3f;
/// `TRACK_BIT_MASK`.
pub const TRACK_BIT_MASK: u8 = 0x3f;

/// `TRACK_END`.
pub const TRACK_END: u8 = 6;

/// `TRACK_X`.
pub const TRACK_X: u8 = 0;
/// `TRACK_Y`.
pub const TRACK_Y: u8 = 1;
/// `TRACK_LEFT`.
pub const TRACK_LEFT: u8 = 4;
/// `TRACK_RIGHT`.
pub const TRACK_RIGHT: u8 = 5;
/// `INVALID_TRACK`.
pub const INVALID_TRACK: u8 = 0xFF;
/// `INVALID_TRACK_BIT`.
pub const INVALID_TRACK_BIT: u8 = 0xFF;
/// `TRACKDIR_UPPER_E`.
pub const TRACKDIR_UPPER_E: u8 = 2;
/// `TRACKDIR_LOWER_E`.
pub const TRACKDIR_LOWER_E: u8 = 3;
/// `TRACKDIR_LEFT_S`.
pub const TRACKDIR_LEFT_S: u8 = 4;
/// `TRACKDIR_RIGHT_S`.
pub const TRACKDIR_RIGHT_S: u8 = 5;
/// `TRACKDIR_UPPER_W`.
pub const TRACKDIR_UPPER_W: u8 = 10;
/// `TRACKDIR_LOWER_W`.
pub const TRACKDIR_LOWER_W: u8 = 11;
/// `TRACKDIR_LEFT_N`.
pub const TRACKDIR_LEFT_N: u8 = 12;
/// `TRACKDIR_RIGHT_N`.
pub const TRACKDIR_RIGHT_N: u8 = 13;
