/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `direction_type.h` values used by the shared helpers and ports.
//! `services.cpp` asserts every value.

/// `DIR_END`.
pub const DIR_END: u8 = 8;
/// `DIR_N`.
pub const DIR_N: u8 = 0;
/// `DIR_NE`.
pub const DIR_NE: u8 = 1;
/// `DIR_E`.
pub const DIR_E: u8 = 2;
/// `DIR_SE`.
pub const DIR_SE: u8 = 3;
/// `DIR_S`.
pub const DIR_S: u8 = 4;
/// `DIR_SW`.
pub const DIR_SW: u8 = 5;
/// `DIR_W`.
pub const DIR_W: u8 = 6;
/// `DIR_NW`.
pub const DIR_NW: u8 = 7;
/// `DIAGDIR_NE`.
pub const DIAGDIR_NE: u8 = 0;
/// `DIAGDIR_SE`.
pub const DIAGDIR_SE: u8 = 1;
/// `DIAGDIR_SW`.
pub const DIAGDIR_SW: u8 = 2;
/// `DIAGDIR_NW`.
pub const DIAGDIR_NW: u8 = 3;
/// `INVALID_DIAGDIR`.
pub const INVALID_DIAGDIR: u8 = 0xFF;

/// `DIAGDIRDIFF_SAME`.
pub const DIAGDIRDIFF_SAME: u8 = 0;
/// `DIAGDIRDIFF_90RIGHT`.
pub const DIAGDIRDIFF_90RIGHT: u8 = 1;
/// `DIAGDIRDIFF_REVERSE`.
pub const DIAGDIRDIFF_REVERSE: u8 = 2;
/// `DIAGDIRDIFF_90LEFT`.
pub const DIAGDIRDIFF_90LEFT: u8 = 3;
/// `DIAGDIRDIFF_END`.
pub const DIAGDIRDIFF_END: u8 = 4;
/// `AXIS_X`.
pub const AXIS_X: u8 = 0;
/// `AXIS_Y`.
pub const AXIS_Y: u8 = 1;
/// `INVALID_AXIS`.
pub const INVALID_AXIS: u8 = 0xFF;

/// `AXIS_END`.
pub const AXIS_END: u8 = 2;

pub const DIR_W: u8 = 6;
pub const DIRDIFF_REVERSE: u8 = 4;
pub const DIRDIFF_SAME: u8 = 0;
