/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `economy_type.h` `Price` indices. `services.cpp` asserts every value.

/// `PR_BUILD_SIGNALS`.
pub const PR_BUILD_SIGNALS: usize = 3;
/// `PR_BUILD_DEPOT_TRAIN`.
pub const PR_BUILD_DEPOT_TRAIN: usize = 5;
/// `PR_CLEAR_ROUGH`.
pub const PR_CLEAR_ROUGH: usize = 23;
/// `PR_CLEAR_SIGNALS`.
pub const PR_CLEAR_SIGNALS: usize = 28;
/// `PR_CLEAR_DEPOT_TRAIN`.
pub const PR_CLEAR_DEPOT_TRAIN: usize = 30;
/// `PR_CLEAR_WATER`.
pub const PR_CLEAR_WATER: usize = 34;
/// `PR_BUILD_FOUNDATION`.
pub const PR_BUILD_FOUNDATION: usize = 57;

/// `PR_TERRAFORM`.
pub const PR_TERRAFORM: usize = 21;
/// `PR_CLEAR_CANAL`.
pub const PR_CLEAR_CANAL: usize = 61;

/// `PR_CLEAR_TREES`.
pub const PR_CLEAR_TREES: usize = 26;

/// `PR_BUILD_BRIDGE`.
pub const PR_BUILD_BRIDGE: usize = 4;
/// `PR_BUILD_TUNNEL`.
pub const PR_BUILD_TUNNEL: usize = 8;
/// `PR_CLEAR_BRIDGE`.
pub const PR_CLEAR_BRIDGE: usize = 29;
/// `PR_CLEAR_TUNNEL`.
pub const PR_CLEAR_TUNNEL: usize = 33;
/// `PR_BUILD_AQUEDUCT`.
pub const PR_BUILD_AQUEDUCT: usize = 62;
/// `PR_CLEAR_AQUEDUCT`.
pub const PR_CLEAR_AQUEDUCT: usize = 63;

/// `PR_TOWN_ACTION`.
pub const PR_TOWN_ACTION: usize = 56;

/// `EXPENSES_CONSTRUCTION`.
pub const EXPENSES_CONSTRUCTION: u8 = 0;
/// `EXPENSES_OTHER`.
pub const EXPENSES_OTHER: u8 = 12;

/// `PR_BUILD_STATION_TRUCK`.
pub const PR_BUILD_STATION_TRUCK: usize = 13;
/// `PR_BUILD_WAYPOINT_RAIL`.
pub const PR_BUILD_WAYPOINT_RAIL: usize = 52;
/// `PR_BUILD_WAYPOINT_BUOY`.
pub const PR_BUILD_WAYPOINT_BUOY: usize = 54;
/// `PR_CLEAR_WAYPOINT_BUOY`.
pub const PR_CLEAR_WAYPOINT_BUOY: usize = 55;
/// `ROAD_STOP_TRACKBIT_FACTOR`.
pub const ROAD_STOP_TRACKBIT_FACTOR: u32 = 2;

/// `PR_END`.
pub const PR_END: u8 = 71;
/// `INVALID_PRICE`.
pub const INVALID_PRICE: u8 = 255;

/// `PR_BUILD_INDUSTRY`.
pub const PR_BUILD_INDUSTRY: usize = 48;
/// `PR_CLEAR_INDUSTRY`.
pub const PR_CLEAR_INDUSTRY: usize = 49;
/// `PR_BUILD_INDUSTRY_RAW`.
pub const PR_BUILD_INDUSTRY_RAW: usize = 58;
/// `PR_CLEAR_HOUSE`.
pub const PR_CLEAR_HOUSE: usize = 40;

/// `PR_BUILD_VEHICLE_TRAIN`.
pub const PR_BUILD_VEHICLE_TRAIN: u32 = 15;
/// `PR_BUILD_VEHICLE_WAGON`.
pub const PR_BUILD_VEHICLE_WAGON: u32 = 16;
/// `PR_BUILD_VEHICLE_AIRCRAFT`.
pub const PR_BUILD_VEHICLE_AIRCRAFT: u32 = 17;
/// `PR_BUILD_VEHICLE_ROAD`.
pub const PR_BUILD_VEHICLE_ROAD: u32 = 18;
/// `PR_BUILD_VEHICLE_SHIP`.
pub const PR_BUILD_VEHICLE_SHIP: u32 = 19;
/// `PR_RUNNING_AIRCRAFT`.
pub const PR_RUNNING_AIRCRAFT: u32 = 45;
/// `PR_RUNNING_SHIP`.
pub const PR_RUNNING_SHIP: u32 = 47;
