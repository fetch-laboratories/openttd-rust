/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared road vehicle state values from `roadveh.h`.
//! `road_services.h` asserts every value against the original header.

/// Native `RoadVehicleStates`: Road vehicle states. The values Rust names.
///
/// Lower 4 bits are used for vehicle track direction. (Trackdirs)
/// When in a road stop (bit 5 or bit 6 set) these bits give the
/// track direction of the entry to the road stop.
/// As the entry direction will always be a diagonal
/// direction (X_NE, Y_SE, X_SW or Y_NW) only bits 0 and 3
/// are needed to hold this direction. Bit 1 is then used to show
/// that the vehicle is using the second road stop bay.
/// Bit 2 is then used for drive-through stops to show the vehicle
/// is stopping at this road stop.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadVehicleStates(pub u8);
impl RoadVehicleStates {
    /* Numeric values */
    /// The vehicle is in a depot
    pub const RVSB_IN_DEPOT: Self = Self(0xFE);
    /// The vehicle is in a tunnel and/or bridge
    pub const RVSB_WORMHOLE: Self = Self(0xFF);

    /* Bit numbers */
    /// Only used while in a road stop
    pub const RVS_USING_SECOND_BAY: Self = Self(1);
    /// The vehicle is in a road stop
    pub const RVS_IN_ROAD_STOP: Self = Self(5);
    /// The vehicle is in a drive-through road stop
    pub const RVS_IN_DT_ROAD_STOP: Self = Self(6);

    /* Bit sets of the above specified bits */
    /// The vehicle is in a road stop
    pub const RVSB_IN_ROAD_STOP: Self = Self(1 << Self::RVS_IN_ROAD_STOP.0);
    /// The vehicle is in a drive-through road stop
    pub const RVSB_IN_DT_ROAD_STOP: Self = Self(1 << Self::RVS_IN_DT_ROAD_STOP.0);
}

/// `RVS_USING_SECOND_BAY` bit number.
pub const RVS_USING_SECOND_BAY: u8 = RoadVehicleStates::RVS_USING_SECOND_BAY.0;
/// `RVS_IN_ROAD_STOP` bit number.
pub const RVS_IN_ROAD_STOP: u8 = RoadVehicleStates::RVS_IN_ROAD_STOP.0;
/// `RVS_IN_DT_ROAD_STOP` bit number.
pub const RVS_IN_DT_ROAD_STOP: u8 = RoadVehicleStates::RVS_IN_DT_ROAD_STOP.0;
/// `RVSB_IN_ROAD_STOP` state mask.
pub const RVSB_IN_ROAD_STOP: u8 = RoadVehicleStates::RVSB_IN_ROAD_STOP.0;
/// `RVSB_IN_DT_ROAD_STOP` state mask.
pub const RVSB_IN_DT_ROAD_STOP: u8 = RoadVehicleStates::RVSB_IN_DT_ROAD_STOP.0;
/// `RVSB_IN_DEPOT`.
pub const RVSB_IN_DEPOT: u8 = RoadVehicleStates::RVSB_IN_DEPOT.0;
/// `RVSB_WORMHOLE`.
pub const RVSB_WORMHOLE: u8 = RoadVehicleStates::RVSB_WORMHOLE.0;

/* Stop frame for a vehicle in a drive-through stop */
/// `RVC_DEPOT_STOP_FRAME`.
pub const RVC_DEPOT_STOP_FRAME: u8 = 11;
