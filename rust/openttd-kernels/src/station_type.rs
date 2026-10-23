/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `station_type.h` values. `services.cpp` asserts every value.

/// `StationID::Invalid()`.
pub const INVALID_STATION: u16 = 0xFFFF;
/// `StationType::Truck`.
pub const STATION_TRUCK: u8 = 2;
/// `StationType::Bus`.
pub const STATION_BUS: u8 = 3;
