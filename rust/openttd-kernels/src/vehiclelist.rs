/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `vehiclelist.h` values. `services.cpp` asserts every value.

/// `VL_STANDARD`.
pub const VL_STANDARD: u8 = 0;
/// `VL_SHARED_ORDERS`.
pub const VL_SHARED_ORDERS: u8 = 1;
/// `VL_STATION_LIST`.
pub const VL_STATION_LIST: u8 = 2;
/// `VL_DEPOT_LIST`.
pub const VL_DEPOT_LIST: u8 = 3;
/// `VL_GROUP_LIST`.
pub const VL_GROUP_LIST: u8 = 4;
