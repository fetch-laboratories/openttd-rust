/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `station_type.h` constants, asserted by native compilation.
pub const NEW_STATION: u16 = StationID::NEW.0;
pub const INVALID_STATION: u16 = StationID::INVALID.0;
pub const RAIL_WAYPOINT: u8 = StationType::RailWaypoint.0;
pub const ROAD_WAYPOINT: u8 = StationType::RoadWaypoint.0;
pub const BUOY: u8 = StationType::Buoy.0;
pub const FACILITY_TRAIN: u8 = 1;
pub const FACILITY_TRUCK: u8 = 2;
pub const FACILITY_BUS: u8 = 4;
pub const FACILITY_AIRPORT: u8 = 8;
pub const FACILITY_DOCK: u8 = 16;
pub const MAX_LENGTH_STATION_NAME_CHARS: usize = 32;

/// `StationType::Truck`.
pub const STATION_TRUCK: u8 = StationType::Truck.0;
/// `StationType::Airport`.
pub const STATION_AIRPORT: u8 = StationType::Airport.0;
/// `StationType::Bus`.
pub const STATION_BUS: u8 = StationType::Bus.0;

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
pub const HVOT_WAYPOINT: u8 = StationHadVehicleOfType::HVOT_WAYPOINT.0;

/// Opaque native `BaseStation`; valid only while its native pool shell lives.
#[repr(C)]
pub struct BaseStation {
    _private: [u8; 0],
}

/// Native `StationID` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationID(pub u16);
impl StationID {
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(u16::MAX);
}

impl StationID {
    /// Native `NEW_STATION`.
    pub const NEW: Self = Self(0xFFFD);
}

/// Opaque native `Station`, distinct from its station/waypoint pool base.
#[repr(C)]
pub struct Station {
    _private: [u8; 0],
}

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
pub const STATION_RAIL: u8 = StationType::Rail.0;
/// `StationType::Oilrig`.
pub const STATION_OILRIG: u8 = StationType::Oilrig.0;
/// `StationType::Dock`.
pub const STATION_DOCK: u8 = StationType::Dock.0;

/// `StationAnimationTrigger::TileLoop`.
pub const SAT_TILE_LOOP: u8 = 7;
/// `AirportAnimationTrigger::TileLoop`.
pub const AAT_TILE_LOOP: u8 = 1;

/// Native station tile kind (uint8).
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct StationType(pub u8);

/// Native `RoadStopType : uint8_t` (bus 0, truck 1).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoadStopType(pub u8);

#[allow(non_upper_case_globals)]
impl StationType {
    /// Native `Rail`.
    pub const Rail: Self = Self(0);
    /// Native `Airport`.
    pub const Airport: Self = Self(1);
    /// Native `Truck`.
    pub const Truck: Self = Self(2);
    /// Native `Bus`.
    pub const Bus: Self = Self(3);
    /// Native `Oilrig`.
    pub const Oilrig: Self = Self(4);
    /// Native `Dock`.
    pub const Dock: Self = Self(5);
    /// Native `Buoy`.
    pub const Buoy: Self = Self(6);
    /// Native `RailWaypoint`.
    pub const RailWaypoint: Self = Self(7);
    /// Native `RoadWaypoint`.
    pub const RoadWaypoint: Self = Self(8);
    /// Native `End`.
    pub const End: Self = Self(9);
}

/// Native `StationFacility`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationFacility(pub u8);

#[allow(non_upper_case_globals)]
impl StationFacility {
    /// Station with train station
    pub const Train: Self = Self(0);
    /// Station with truck stops
    pub const TruckStop: Self = Self(1);
    /// Station with bus stops
    pub const BusStop: Self = Self(2);
    /// Station with an airport
    pub const Airport: Self = Self(3);
    /// Station with a dock
    pub const Dock: Self = Self(4);
    /// Station is a waypoint
    pub const Waypoint: Self = Self(7);
}

/// Native `StationHadVehicleOfType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationHadVehicleOfType(pub u8);

#[allow(non_upper_case_globals)]
impl StationHadVehicleOfType {
    /// Station has seen no vehicles
    pub const HVOT_NONE: Self = Self(0);
    /// Station has seen a train
    pub const HVOT_TRAIN: Self = Self(2);
    /// Station has seen a bus
    pub const HVOT_BUS: Self = Self(4);
    /// Station has seen a truck
    pub const HVOT_TRUCK: Self = Self(8);
    /// Station has seen an aircraft
    pub const HVOT_AIRCRAFT: Self = Self(16);
    /// Station has seen a ship
    pub const HVOT_SHIP: Self = Self(32);
    /// Station is a waypoint (NewGRF only!)
    pub const HVOT_WAYPOINT: Self = Self(64);
}

/// Native `StationRandomTrigger`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationRandomTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl StationRandomTrigger {
    /// Trigger station on new cargo arrival.
    pub const NewCargo: Self = Self(0);
    /// Trigger station when cargo is completely taken.
    pub const CargoTaken: Self = Self(1);
    /// Trigger platform when train arrives.
    pub const VehicleArrives: Self = Self(2);
    /// Trigger platform when train leaves.
    pub const VehicleDeparts: Self = Self(3);
    /// Trigger platform when train loads/unloads.
    pub const VehicleLoads: Self = Self(4);
    /// Trigger platform when train reserves path.
    pub const PathReservation: Self = Self(5);
}

/// Native `StationAnimationTrigger`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationAnimationTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl StationAnimationTrigger {
    /// Trigger tile when built.
    pub const Built: Self = Self(0);
    /// Trigger station on new cargo arrival.
    pub const NewCargo: Self = Self(1);
    /// Trigger station when cargo is completely taken.
    pub const CargoTaken: Self = Self(2);
    /// Trigger platform when train arrives.
    pub const VehicleArrives: Self = Self(3);
    /// Trigger platform when train leaves.
    pub const VehicleDeparts: Self = Self(4);
    /// Trigger platform when train loads/unloads.
    pub const VehicleLoads: Self = Self(5);
    /// Trigger station every 250 ticks.
    pub const AcceptanceTick: Self = Self(6);
    /// Trigger in the periodic tile loop.
    pub const TileLoop: Self = Self(7);
    /// Trigger platform when train reserves path.
    pub const PathReservation: Self = Self(8);
    /// Native `End`.
    pub const End: Self = Self(9);
}

/// Native `AirportAnimationTrigger`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportAnimationTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl AirportAnimationTrigger {
    /// Triggered when the airport is built (for all tiles at the same time).
    pub const Built: Self = Self(0);
    /// Triggered in the periodic tile loop.
    pub const TileLoop: Self = Self(1);
    /// Triggered when new cargo arrives at the station (for all tiles at the same time).
    pub const NewCargo: Self = Self(2);
    /// Triggered when a cargo type is completely removed from the station (for all tiles at the same time).
    pub const CargoTaken: Self = Self(3);
    /// Triggered every 250 ticks (for all tiles at the same time).
    pub const AcceptanceTick: Self = Self(4);
    /// Triggered when an airplane (not a helicopter) touches down at the airport (for single tile).
    pub const AirplaneTouchdown: Self = Self(5);
}

/// Native `StationFacilities`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationFacilities(pub u8);

impl StationFacilities {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: StationFacility) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `StationRandomTriggers`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationRandomTriggers(pub u8);

impl StationRandomTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: StationRandomTrigger) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `StationAnimationTriggers`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationAnimationTriggers(pub u16);

impl StationAnimationTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: StationAnimationTrigger) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

/// Native `AirportAnimationTriggers`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportAnimationTriggers(pub u8);

impl AirportAnimationTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: AirportAnimationTrigger) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `RoadStopID`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadStopID(pub u16);

impl RoadStopID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

impl StationID {
    /// Join an adjacent station.
    pub const ADJACENT: Self = Self(0xFFFE);
}
