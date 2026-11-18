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
/// direction (`X_NE`, `Y_SE`, `X_SW` or `Y_NW`) only bits 0 and 3
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
    /// Only set when a vehicle has entered the stop
    pub const RVS_ENTERED_STOP: Self = Self(2);
    /// Only used when retrieving move data
    pub const RVS_DRIVE_SIDE: Self = Self(4);
    /// The vehicle is in a road stop
    pub const RVS_IN_ROAD_STOP: Self = Self(5);
    /// The vehicle is in a drive-through road stop
    pub const RVS_IN_DT_ROAD_STOP: Self = Self(6);

    /* Bit sets of the above specified bits */
    /// The vehicle is in a road stop
    pub const RVSB_IN_ROAD_STOP: Self = Self(1 << Self::RVS_IN_ROAD_STOP.0);
    /// `RVSB_IN_ROAD_STOP + TRACKDIR_END`.
    pub const RVSB_IN_ROAD_STOP_END: Self =
        Self(Self::RVSB_IN_ROAD_STOP.0 + crate::track_type::Trackdir::TRACKDIR_END.0);
    /// The vehicle is in a drive-through road stop
    pub const RVSB_IN_DT_ROAD_STOP: Self = Self(1 << Self::RVS_IN_DT_ROAD_STOP.0);
    /// `RVSB_IN_DT_ROAD_STOP + TRACKDIR_END`.
    pub const RVSB_IN_DT_ROAD_STOP_END: Self =
        Self(Self::RVSB_IN_DT_ROAD_STOP.0 + crate::track_type::Trackdir::TRACKDIR_END.0);

    /// The vehicle is at the opposite side of the road
    pub const RVSB_DRIVE_SIDE: Self = Self(1 << Self::RVS_DRIVE_SIDE.0);

    /// The mask used to extract track dirs
    pub const RVSB_TRACKDIR_MASK: Self = Self(0x0F);
    /// Only bits 0 and 3 are used to encode the trackdir for road stops
    pub const RVSB_ROAD_STOP_TRACKDIR_MASK: Self = Self(0x09);
}

/// `RVS_USING_SECOND_BAY` bit number.
pub const RVS_USING_SECOND_BAY: u8 = RoadVehicleStates::RVS_USING_SECOND_BAY.0;
/// `RVS_IN_ROAD_STOP` bit number.
pub const RVS_IN_ROAD_STOP: u8 = RoadVehicleStates::RVS_IN_ROAD_STOP.0;
/// `RVS_IN_DT_ROAD_STOP` bit number.
pub const RVS_IN_DT_ROAD_STOP: u8 = RoadVehicleStates::RVS_IN_DT_ROAD_STOP.0;
/// `RVS_ENTERED_STOP` bit number.
pub const RVS_ENTERED_STOP: u8 = RoadVehicleStates::RVS_ENTERED_STOP.0;
/// `RVS_DRIVE_SIDE` bit number.
pub const RVS_DRIVE_SIDE: u8 = RoadVehicleStates::RVS_DRIVE_SIDE.0;
/// `RVSB_IN_ROAD_STOP` state mask.
pub const RVSB_IN_ROAD_STOP: u8 = RoadVehicleStates::RVSB_IN_ROAD_STOP.0;
/// `RVSB_IN_ROAD_STOP_END`.
pub const RVSB_IN_ROAD_STOP_END: u8 = RoadVehicleStates::RVSB_IN_ROAD_STOP_END.0;
/// `RVSB_IN_DT_ROAD_STOP` state mask.
pub const RVSB_IN_DT_ROAD_STOP: u8 = RoadVehicleStates::RVSB_IN_DT_ROAD_STOP.0;
/// `RVSB_IN_DT_ROAD_STOP_END`.
pub const RVSB_IN_DT_ROAD_STOP_END: u8 = RoadVehicleStates::RVSB_IN_DT_ROAD_STOP_END.0;
/// `RVSB_DRIVE_SIDE`.
pub const RVSB_DRIVE_SIDE: u8 = RoadVehicleStates::RVSB_DRIVE_SIDE.0;
/// `RVSB_TRACKDIR_MASK`.
pub const RVSB_TRACKDIR_MASK: u8 = RoadVehicleStates::RVSB_TRACKDIR_MASK.0;
/// `RVSB_ROAD_STOP_TRACKDIR_MASK`.
pub const RVSB_ROAD_STOP_TRACKDIR_MASK: u8 = RoadVehicleStates::RVSB_ROAD_STOP_TRACKDIR_MASK.0;
/// `RVSB_IN_DEPOT`.
pub const RVSB_IN_DEPOT: u8 = RoadVehicleStates::RVSB_IN_DEPOT.0;
/// `RVSB_WORMHOLE`.
pub const RVSB_WORMHOLE: u8 = RoadVehicleStates::RVSB_WORMHOLE.0;

/* State information about the Road Vehicle controller */
/// We should enter the next tile
pub const RDE_NEXT_TILE: u8 = 0x80;
/// We just finished turning
pub const RDE_TURNED: u8 = 0x40;

/* Start frames for when a vehicle enters a tile/changes its state.
 * The start frame is different for vehicles that turned around or
 * are leaving the depot as the do not start at the edge of the tile.
 * For trams there are a few different start frames as there are two
 * places where trams can turn. */
/// `RVC_DEFAULT_START_FRAME`.
pub const RVC_DEFAULT_START_FRAME: u8 = 0;
/// `RVC_TURN_AROUND_START_FRAME`.
pub const RVC_TURN_AROUND_START_FRAME: u8 = 1;
/// `RVC_DEPOT_START_FRAME`.
pub const RVC_DEPOT_START_FRAME: u8 = 6;
/// `RVC_START_FRAME_AFTER_LONG_TRAM`.
pub const RVC_START_FRAME_AFTER_LONG_TRAM: u8 = 21;
/// `RVC_TURN_AROUND_START_FRAME_SHORT_TRAM`.
pub const RVC_TURN_AROUND_START_FRAME_SHORT_TRAM: u8 = 16;
/* Stop frame for a vehicle in a drive-through stop */
/// `RVC_DRIVE_THROUGH_STOP_FRAME`.
pub const RVC_DRIVE_THROUGH_STOP_FRAME: u8 = 11;
/// `RVC_DEPOT_STOP_FRAME`.
pub const RVC_DEPOT_STOP_FRAME: u8 = 11;

/// The number of ticks a vehicle has for overtaking.
pub const RV_OVERTAKE_TIMEOUT: u8 = 35;
