/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `track_type.h` constants shared by simulation owners.

pub const TRACK_BIT_X: u8 = 1;
pub const TRACK_BIT_Y: u8 = 2;
/// `TRACK_BIT_HORZ`.
pub const TRACK_BIT_HORZ: u8 = 12;
/// `TRACK_BIT_VERT`.
pub const TRACK_BIT_VERT: u8 = 48;
pub const TRACK_BIT_WORMHOLE: u8 = 0x40;
pub const TRACK_BIT_MASK: u16 = 0x3f;
pub const TRACKDIR_BIT_MASK: u32 = 0x3f3f;
/// `TRACK_BIT_DEPOT`.
pub const TRACK_BIT_DEPOT: u8 = 0x80;
