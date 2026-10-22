/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared road vehicle state values from `roadveh.h`.
//! `road_services.h` asserts every value against the original header.

/// `RVS_USING_SECOND_BAY` bit number.
pub const RVS_USING_SECOND_BAY: u8 = 1;
/// `RVS_IN_ROAD_STOP` bit number.
pub const RVS_IN_ROAD_STOP: u8 = 5;
/// `RVS_IN_DT_ROAD_STOP` bit number.
pub const RVS_IN_DT_ROAD_STOP: u8 = 6;
/// `RVSB_IN_ROAD_STOP` state mask.
pub const RVSB_IN_ROAD_STOP: u8 = 1 << RVS_IN_ROAD_STOP;
/// `RVSB_IN_DT_ROAD_STOP` state mask.
pub const RVSB_IN_DT_ROAD_STOP: u8 = 1 << RVS_IN_DT_ROAD_STOP;
