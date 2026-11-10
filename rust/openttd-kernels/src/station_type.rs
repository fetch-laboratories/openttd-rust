/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `station_type.h` constants, asserted by native compilation.
/// Native `NEW_STATION`.
pub const NEW_STATION: u16 = StationID::NEW.0;
/// Native `INVALID_STATION`.
pub const INVALID_STATION: u16 = StationID::INVALID.0;
/// Native `RAIL_WAYPOINT`.
pub const RAIL_WAYPOINT: u8 = StationType::RailWaypoint.0;
/// Native `ROAD_WAYPOINT`.
pub const ROAD_WAYPOINT: u8 = StationType::RoadWaypoint.0;
/// Native `BUOY`.
pub const BUOY: u8 = StationType::Buoy.0;
/// Native `FACILITY_TRAIN`.
pub const FACILITY_TRAIN: u8 = StationFacilities::from_element(StationFacility::Train).0;
/// Native `FACILITY_TRUCK`.
pub const FACILITY_TRUCK: u8 = StationFacilities::from_element(StationFacility::TruckStop).0;
/// Native `FACILITY_BUS`.
pub const FACILITY_BUS: u8 = StationFacilities::from_element(StationFacility::BusStop).0;
/// Native `FACILITY_AIRPORT`.
pub const FACILITY_AIRPORT: u8 = StationFacilities::from_element(StationFacility::Airport).0;
/// Native `FACILITY_DOCK`.
pub const FACILITY_DOCK: u8 = StationFacilities::from_element(StationFacility::Dock).0;
/// Native `MAX_LENGTH_STATION_NAME_CHARS`.
pub const MAX_LENGTH_STATION_NAME_CHARS: usize = 32;

/// `StationType::Truck`.
pub const STATION_TRUCK: u8 = StationType::Truck.0;
/// `StationType::Airport`.
pub const STATION_AIRPORT: u8 = StationType::Airport.0;
/// `StationType::Bus`.
pub const STATION_BUS: u8 = StationType::Bus.0;

/// `StationAnimationTrigger` values shared by its existing callers.
pub const SAT_BUILT: u8 = StationAnimationTrigger::Built.0;
/// Native `SAT_NEW_CARGO`.
pub const SAT_NEW_CARGO: u8 = StationAnimationTrigger::NewCargo.0;
/// Native `SAT_CARGO_TAKEN`.
pub const SAT_CARGO_TAKEN: u8 = StationAnimationTrigger::CargoTaken.0;
/// Native `SAT_VEHICLE_ARRIVES`.
pub const SAT_VEHICLE_ARRIVES: u8 = StationAnimationTrigger::VehicleArrives.0;
/// Native `SAT_VEHICLE_DEPARTS`.
pub const SAT_VEHICLE_DEPARTS: u8 = StationAnimationTrigger::VehicleDeparts.0;
/// Native `SAT_VEHICLE_LOADS`.
pub const SAT_VEHICLE_LOADS: u8 = StationAnimationTrigger::VehicleLoads.0;
/// Native `SAT_ACCEPTANCE_TICK`.
pub const SAT_ACCEPTANCE_TICK: u8 = StationAnimationTrigger::AcceptanceTick.0;
/// Native `SAT_PATH_RESERVATION`.
pub const SAT_PATH_RESERVATION: u8 = StationAnimationTrigger::PathReservation.0;
/// `AirportAnimationTrigger` values.
pub const AAT_BUILT: u8 = AirportAnimationTrigger::Built.0;
/// Native `AAT_NEW_CARGO`.
pub const AAT_NEW_CARGO: u8 = AirportAnimationTrigger::NewCargo.0;
/// Native `AAT_CARGO_TAKEN`.
pub const AAT_CARGO_TAKEN: u8 = AirportAnimationTrigger::CargoTaken.0;
/// Native `AAT_ACCEPTANCE_TICK`.
pub const AAT_ACCEPTANCE_TICK: u8 = AirportAnimationTrigger::AcceptanceTick.0;
/// Native `AAT_AIRPLANE_TOUCHDOWN`.
pub const AAT_AIRPLANE_TOUCHDOWN: u8 = AirportAnimationTrigger::AirplaneTouchdown.0;

/// `StationRandomTrigger` values: native resolvers retain reseed policy.
pub const SRT_NEW_CARGO: u8 = StationRandomTrigger::NewCargo.0;
/// Native `SRT_CARGO_TAKEN`.
pub const SRT_CARGO_TAKEN: u8 = StationRandomTrigger::CargoTaken.0;
/// Native `SRT_VEHICLE_ARRIVES`.
pub const SRT_VEHICLE_ARRIVES: u8 = StationRandomTrigger::VehicleArrives.0;
/// Native `SRT_VEHICLE_DEPARTS`.
pub const SRT_VEHICLE_DEPARTS: u8 = StationRandomTrigger::VehicleDeparts.0;
/// Native `SRT_VEHICLE_LOADS`.
pub const SRT_VEHICLE_LOADS: u8 = StationRandomTrigger::VehicleLoads.0;

/// `StationRandomTrigger::PathReservation`.
pub const SRT_PATH_RESERVATION: u8 = StationRandomTrigger::PathReservation.0;

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
pub const SAT_TILE_LOOP: u8 = StationAnimationTrigger::TileLoop.0;
/// `AirportAnimationTrigger::TileLoop`.
pub const AAT_TILE_LOOP: u8 = AirportAnimationTrigger::TileLoop.0;

/// Native station tile kind (uint8).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationType(pub u8);

/// Native `RoadStopType : uint8_t` (bus 0, truck 1).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Station is a waypoint (`NewGRF` only!)
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

impl StationFacilities {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: StationFacility) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native `BaseBitSet::Any(other)`: whether any of the given elements are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl StationRandomTriggers {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: StationRandomTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl StationAnimationTriggers {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: StationAnimationTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl AirportAnimationTriggers {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: AirportAnimationTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl StationFacility {
    /// Fake facility for display of recently removed station signs.
    pub const STATION_FACILITY_GHOST: Self = Self(6);
}

crate::base_bitset_type::impl_bitset_ops!(StationFacilities);
crate::base_bitset_type::impl_bitset_ops!(StationRandomTriggers);
crate::base_bitset_type::impl_bitset_ops!(StationAnimationTriggers);
crate::base_bitset_type::impl_bitset_ops!(AirportAnimationTriggers);
crate::base_bitset_type::impl_bitset_ops!(StationHadVehicleOfType);

#[allow(non_upper_case_globals)]
impl RoadStopType {
    /// A standard stop for buses.
    pub const Bus: Self = Self(0);
    /// A standard stop for trucks.
    pub const Truck: Self = Self(1);
    /// End of valid types.
    pub const End: Self = Self(2);
}

impl StationID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}
