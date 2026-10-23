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

/// `AXIS_X`.
pub const AXIS_X: u8 = 0;
/// `AXIS_Y`.
pub const AXIS_Y: u8 = 1;
/// `INVALID_AXIS`.
pub const INVALID_AXIS: u8 = 0xFF;

/// `AXIS_END`.
pub const AXIS_END: u8 = 2;
