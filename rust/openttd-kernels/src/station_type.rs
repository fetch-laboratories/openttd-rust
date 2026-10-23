/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `station_type.h` constants, asserted by native compilation.
pub const NEW_STATION: u16 = 0xFFFD;
pub const INVALID_STATION: u16 = 0xFFFF;
pub const RAIL_WAYPOINT: u8 = 7;
pub const ROAD_WAYPOINT: u8 = 8;
pub const BUOY: u8 = 6;
pub const FACILITY_TRAIN: u8 = 1;
pub const FACILITY_TRUCK: u8 = 2;
pub const FACILITY_BUS: u8 = 4;
pub const FACILITY_DOCK: u8 = 16;
pub const MAX_LENGTH_STATION_NAME_CHARS: usize = 32;
