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
pub const FACILITY_AIRPORT: u8 = 8;
pub const FACILITY_DOCK: u8 = 16;
pub const MAX_LENGTH_STATION_NAME_CHARS: usize = 32;

/// `StationType::Truck`.
pub const STATION_TRUCK: u8 = 2;
/// `StationType::Airport`.
pub const STATION_AIRPORT: u8 = 1;
/// `StationType::Bus`.
pub const STATION_BUS: u8 = 3;

/// `StationAnimationTrigger` values shared by its existing callers.
pub const SAT_BUILT: u8 = 0;
pub const SAT_NEW_CARGO: u8 = 1;
pub const SAT_CARGO_TAKEN: u8 = 2;
pub const SAT_VEHICLE_ARRIVES: u8 = 3;
pub const SAT_VEHICLE_DEPARTS: u8 = 4;
pub const SAT_VEHICLE_LOADS: u8 = 5;
pub const SAT_ACCEPTANCE_TICK: u8 = 6;
pub const SAT_PATH_RESERVATION: u8 = 8;
/// `AirportAnimationTrigger` values.
pub const AAT_BUILT: u8 = 0;
pub const AAT_NEW_CARGO: u8 = 2;
pub const AAT_CARGO_TAKEN: u8 = 3;
pub const AAT_ACCEPTANCE_TICK: u8 = 4;
pub const AAT_AIRPLANE_TOUCHDOWN: u8 = 5;

/// `StationRandomTrigger` values: native resolvers retain reseed policy.
pub const SRT_NEW_CARGO: u8 = 0;
pub const SRT_CARGO_TAKEN: u8 = 1;
pub const SRT_VEHICLE_ARRIVES: u8 = 2;
pub const SRT_VEHICLE_DEPARTS: u8 = 3;
pub const SRT_VEHICLE_LOADS: u8 = 4;

/// `StationRandomTrigger::PathReservation`.
pub const SRT_PATH_RESERVATION: u8 = 5;

/// `StationHadVehicleOfType::HVOT_WAYPOINT`.
pub const HVOT_WAYPOINT: u8 = 1 << 6;

/// Maximum modified airport catchment.
pub const MAX_CATCHMENT: u8 = 10;
/// `CA_NONE`: catchment without facilities.
pub const CA_NONE: u32 = 0;
/// `CA_BUS`, with modified catchment.
pub const CA_BUS: u32 = 3;
/// `CA_TRUCK`, with modified catchment.
pub const CA_TRUCK: u32 = 3;
/// `CA_TRAIN`, with modified catchment.
pub const CA_TRAIN: u32 = 4;
/// `CA_DOCK`, with modified catchment.
pub const CA_DOCK: u32 = 5;
/// `CA_UNMODIFIED`: every facility without modified catchment.
pub const CA_UNMODIFIED: u32 = 4;
/// `StationType::Rail`.
pub const STATION_RAIL: u8 = 0;
/// `StationType::Oilrig`.
pub const STATION_OILRIG: u8 = 4;
/// `StationType::Dock`.
pub const STATION_DOCK: u8 = 5;
