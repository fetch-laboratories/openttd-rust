/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `station_map.h` helpers for live map fields and remaining typed services.
use crate::bitmath_func::{gb, has_bit, sb, toggle_bit};
use crate::direction_type::{Axis, DiagDirection};
use crate::map_func::TileTypeAccess;
use crate::services::Services;
use crate::station_type::{RoadStopType, StationID, StationType};
use crate::tile_type::TileType;
use crate::tile_type::{MP_STATION, TileIndex};
use crate::track_type::Track;

/// Native `StationGfx`: index of station graphics (`_station_display_datas`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StationGfx(pub u8);

/// Offset for the dock water parts (`GFX_DOCK_BASE_WATER_PART`).
pub const GFX_DOCK_BASE_WATER_PART: u8 = 4;

/// `GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET`: offset for drive through parts.
pub const GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET: u8 = 4;

/// `GetStationType`: get the station type of this tile, including its native
/// tile-type assertion policy.
#[allow(clippy::inline_always)]
#[inline(always)]
#[allow(clippy::cast_possible_truncation)] // GB returns the original four-bit value.
pub(crate) fn get_station_type(m: &TileTypeAccess, tile: TileIndex) -> Option<StationType> {
    let fields = m.require_type(tile, TileType::MP_STATION)?;
    Some(StationType(gb(u32::from(fields.m6()), 3, 4) as u8))
}

/// `IsDock`: is tile `t` a dock tile?
pub(crate) fn is_dock(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)? == StationType::Dock)
}

/// `IsBuoy`: is tile `t` a buoy tile?
pub(crate) fn is_buoy(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)? == StationType::Buoy)
}

/// `IsTruckStop`: is the station at `t` a truck stop?
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_truck_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)? == StationType::Truck)
}

/// `IsBusStop`: is the station at `t` a bus stop?
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_bus_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)? == StationType::Bus)
}

/// `IsRoadWaypoint`: is the station at `t` a road waypoint?
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_road_waypoint(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)? == StationType::RoadWaypoint)
}

/// `IsAnyRoadStop`: is the station at `t` a road station? Preserves the
/// source predicate order.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_any_road_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    m.require_type(tile, TileType::MP_STATION)?;
    Some(is_truck_stop(m, tile)? || is_bus_stop(m, tile)? || is_road_waypoint(m, tile)?)
}

/// `IsStationRoadStop`: is the station at `t` a road station?
#[inline]
pub(crate) fn is_station_road_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    m.require_type(tile, TileType::MP_STATION)?;
    Some(is_truck_stop(m, tile)? || is_bus_stop(m, tile)?)
}

/// `IsAirport`: is this station tile an airport? The station type leaf keeps
/// its native assertion.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_airport(tile: TileIndex, s: &Services) -> bool {
    crate::map_func::World::new(s).is_airport(tile)
}
/// Temporary raw adapter for `animated_tile.rs` and `newgrf_airport.rs`.
#[inline]
pub(crate) fn is_airport_raw(tile: u32, s: &Services) -> bool {
    is_airport(TileIndex(tile), s)
}

/// `IsRailWaypointTile`: is this tile a station tile and a rail waypoint?
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_rail_waypoint_tile(tile: TileIndex, s: &Services) -> bool {
    crate::map_func::World::new(s).is_rail_waypoint_tile(tile)
}
/// `IsRoadWaypointTile`: is this tile a station tile and a road waypoint?
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_road_waypoint_tile(tile: TileIndex, s: &Services) -> bool {
    (s.tile_type)(tile.0) == MP_STATION && (s.station_type)(tile.0) == StationType::RoadWaypoint
}

/// `GetAirportGfx`: get the station graphics of this airport tile.
/// `GetStationGfx` retains its native station assertion; translation calls
/// the Rust owner directly.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_airport_gfx(
    tile: TileIndex,
    s: &Services,
    airport: &crate::newgrf_airport::Services,
) -> StationGfx {
    let world = crate::map_func::World::new(s);
    world.assert_with(|| world.is_airport(tile));
    StationGfx(crate::newgrf_airport::translate_tile(
        (s.station_gfx)(tile.0),
        airport,
    ))
}
/// Temporary raw adapter for `newgrf_airport.rs`.
#[inline]
pub(crate) fn get_airport_gfx_raw(
    tile: u32,
    s: &Services,
    airport: &crate::newgrf_airport::Services,
) -> u8 {
    get_airport_gfx(TileIndex(tile), s, airport).0
}

/// `Station::GetByTile`: get the station belonging to a specific tile,
/// retaining its valid-station-or-null lookup.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn station_get_by_tile(
    tile: TileIndex,
    s: &crate::newgrf_airport::Services,
) -> *mut crate::station::BaseStation {
    (s.by_tile)(tile.0).cast()
}
/// Temporary raw adapter for `water.rs` and `airport.rs`.
#[inline]
pub(crate) fn station_by_tile(
    tile: u32,
    s: &crate::newgrf_airport::Services,
) -> *mut crate::station::BaseStation {
    station_get_by_tile(TileIndex(tile), s)
}
/// Original `IsHangar` (`station_cmd.cpp`): check whether the given tile is a
/// hangar, retaining its assertion and airport gate before lookup.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_hangar(tile: TileIndex, s: &crate::newgrf_airport::Services) -> bool {
    let world = crate::map_func::World::new(s.shared());
    world.assert_with(|| (s.shared().tile_type)(tile.0) == MP_STATION);
    // If the tile isn't an airport there's no chance it's a hangar.
    if !is_airport(tile, s.shared()) {
        return false;
    }
    let st = station_get_by_tile(tile, s);
    let spec = crate::newgrf_airport::station_spec(st, s);
    for i in 0..(s.depot_count)(spec) {
        let offset = (s.depot_offset)(spec, i);
        if (s.rotated_offset)(st, offset, spec) == tile.0 {
            return true;
        }
    }
    false
}
/// Temporary raw adapter for `newgrf_airport.rs`.
#[inline]
pub(crate) fn is_hangar_raw(tile: u32, s: &crate::newgrf_airport::Services) -> bool {
    is_hangar(TileIndex(tile), s)
}
/// Original `IsHangarTile`: is tile `t` a hangar tile? Includes its
/// non-station early return.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_hangar_tile(t: TileIndex, s: &crate::newgrf_airport::Services) -> bool {
    (s.shared().tile_type)(t.0) == MP_STATION && is_hangar(t, s)
}
/// Raw adapter for `orders.rs`, which #825 holds; delegates to `is_hangar_tile`.
#[inline]
pub(crate) fn is_hangar_tile_raw(tile: u32, s: &crate::newgrf_airport::Services) -> bool {
    is_hangar_tile(TileIndex(tile), s)
}

/// The `assert(IsRoadWaypointTile(t))` precondition of the road waypoint
/// accessors.
fn require_road_waypoint(m: &crate::map_func::TileTypeAccess, tile: TileIndex) -> Option<()> {
    m.available(tile)?;
    if m.assertions() {
        let ok = m.read(tile) == TileType::MP_STATION && is_road_waypoint(m, tile)?;
        m.check(ok)?;
    }
    Some(())
}
/// `IsRoadWaypointOnSnowOrDesert`: check if a road waypoint tile has
/// snow/desert.
pub(crate) fn is_road_waypoint_on_snow_or_desert(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
) -> Option<bool> {
    require_road_waypoint(m, tile)?;
    Some(has_bit(m.m8(tile)?, 15))
}
/// `ToggleRoadWaypointOnSnowOrDesert`: toggle the snow/desert state of a road
/// waypoint tile.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ToggleBit stores back into the original uint16_t m8"
)]
pub(crate) fn toggle_road_waypoint_on_snow_or_desert(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
) -> Option<()> {
    require_road_waypoint(m, tile)?;
    let mut value = u32::from(m.m8(tile)?);
    toggle_bit(&mut value, 15);
    m.set_m8(tile, value as u16)
}
/// `GetRoadWaypointRoadside`: get the decorations of a road waypoint.
#[allow(
    clippy::cast_possible_truncation,
    reason = "GB returns the original two-bit Roadside value"
)]
pub(crate) fn get_road_waypoint_roadside(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
) -> Option<crate::services::Roadside> {
    require_road_waypoint(m, tile)?;
    Some(crate::services::Roadside(
        gb(u32::from(m.m3(tile)?), 2, 2) as u8
    ))
}
/// `SetRoadWaypointRoadside`: set the decorations of a road waypoint.
#[allow(
    clippy::cast_possible_truncation,
    reason = "SB stores back into the original uint8_t m3"
)]
pub(crate) fn set_road_waypoint_roadside(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
    rs: crate::services::Roadside,
) -> Option<()> {
    require_road_waypoint(m, tile)?;
    let mut bits = u32::from(m.m3(tile)?);
    sb(&mut bits, 2, 2, u32::from(rs.0));
    m.set_m3(tile, bits as u8)
}

impl crate::map_func::World<'_> {
    /// `IsRailStationTile`: is this tile a station tile and a rail station?
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_rail_station_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION
            && get_station_type(&self.types, t).expect("station tile") == StationType::Rail
    }
    /// `IsRailStation`: is this station tile a rail station? Keeps
    /// `GetStationType`'s tile-type assertion.
    pub(crate) fn is_rail_station(&self, t: TileIndex) -> bool {
        self.asserted(|m| get_station_type(m, t)) == StationType::Rail
    }
    /// `IsRailWaypoint`: is this station tile a rail waypoint? Keeps
    /// `GetStationType`'s tile-type assertion.
    pub(crate) fn is_rail_waypoint(&self, t: TileIndex) -> bool {
        self.asserted(|m| get_station_type(m, t)) == StationType::RailWaypoint
    }
    /// `HasStationRail`: has this station tile a rail? In other words, is this
    /// station tile a rail station or rail waypoint? Each test keeps
    /// `GetStationType`'s tile-type assertion.
    pub(crate) fn has_station_rail(&self, t: TileIndex) -> bool {
        self.is_rail_station(t) || self.is_rail_waypoint(t)
    }
    /// `IsBuoy`, with `GetStationType`'s tile-type assertion.
    pub(crate) fn is_buoy(&self, t: TileIndex) -> bool {
        self.asserted(|m| is_buoy(m, t))
    }
    /// `IsBuoyTile`: is tile `t` a buoy tile?
    pub(crate) fn is_buoy_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION && self.is_buoy(t)
    }
    /// `IsAirport`: is this station tile an airport? Keeps `GetStationType`'s
    /// tile-type assertion.
    pub(crate) fn is_airport(&self, t: TileIndex) -> bool {
        self.asserted(|m| get_station_type(m, t)) == StationType::Airport
    }
    /// `IsAirportTile`: is this tile a station tile and an airport tile?
    pub(crate) fn is_airport_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION && self.is_airport(t)
    }
    /// `IsOilRig`: is tile `t` part of an oilrig? Keeps `GetStationType`'s
    /// tile-type assertion.
    pub(crate) fn is_oil_rig(&self, t: TileIndex) -> bool {
        self.asserted(|m| get_station_type(m, t)) == StationType::Oilrig
    }
    /// `IsDock`, with `GetStationType`'s tile-type assertion.
    pub(crate) fn is_dock(&self, t: TileIndex) -> bool {
        self.asserted(|m| is_dock(m, t))
    }
    /// `IsDockTile`: is tile `t` a dock tile?
    pub(crate) fn is_dock_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION && self.is_dock(t)
    }
    /// `GetDockDirection`: get the direction of a dock.
    pub(crate) fn get_dock_direction(&self, t: TileIndex) -> DiagDirection {
        let gfx = self.get_station_gfx(t);
        self.assert_with(|| self.is_dock(t) && gfx.0 < GFX_DOCK_BASE_WATER_PART);
        DiagDirection(gfx.0)
    }
    /// `IsDockWaterPart`: check whether a dock tile is the tile on water.
    pub(crate) fn is_dock_water_part(&self, t: TileIndex) -> bool {
        self.assert_with(|| self.is_dock_tile(t));
        let gfx = self.get_station_gfx(t);
        gfx.0 >= GFX_DOCK_BASE_WATER_PART
    }
    /// `IsAnyRoadStopTile`: is tile `t` a road stop station?
    pub(crate) fn is_any_road_stop_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION && self.asserted(|m| is_any_road_stop(m, t))
    }
    /// `GetRailStationTrack`: get the rail track of a rail station tile.
    pub(crate) fn get_rail_station_track(&self, t: TileIndex) -> Track {
        let axis = self.get_rail_station_axis(t);
        crate::track_func::axis_to_track(axis, |ok| self.check(ok))
    }
    /// `IsDriveThroughStopTile`: is tile `t` a drive through road stop
    /// station or waypoint?
    pub(crate) fn is_drive_through_stop_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION
            && is_any_road_stop(&self.types, t).expect("station tile")
            && self.get_station_gfx(t).0 >= GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET
    }
    /// `GetDriveThroughStopAxis`: gets the axis of the drive through stop.
    pub(crate) fn get_drive_through_stop_axis(&self, t: TileIndex) -> Axis {
        self.assert_with(|| self.is_drive_through_stop_tile(t));
        Axis(
            self.get_station_gfx(t)
                .0
                .wrapping_sub(GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET),
        )
    }
    /// `GetRoadStopType`: get the road stop type of this tile.
    pub(crate) fn get_road_stop_type(&self, t: TileIndex) -> RoadStopType {
        let station_type = || self.asserted(|m| get_station_type(m, t));
        self.assert_with(|| {
            station_type() == StationType::Truck || station_type() == StationType::Bus
        });
        if station_type() == StationType::Truck {
            RoadStopType::Truck
        } else {
            RoadStopType::Bus
        }
    }
    /// `IsRailWaypointTile`: is this tile a station tile and a rail waypoint?
    pub(crate) fn is_rail_waypoint_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION
            && get_station_type(&self.types, t).expect("station tile") == StationType::RailWaypoint
    }
    /// `HasStationTileRail`: is this a station tile with rail, a rail station
    /// or rail waypoint?
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn has_station_tile_rail(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION
            && matches!(
                get_station_type(&self.types, t).expect("station tile"),
                StationType::Rail | StationType::RailWaypoint
            )
    }
    /// `IsStationRoadStopTile`: is tile `t` a road stop station?
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_station_road_stop_tile(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_STATION
            && matches!(
                get_station_type(&self.types, t).expect("station tile"),
                StationType::Truck | StationType::Bus
            )
    }
    /// `IsBayRoadStopTile`: is tile `t` a bay (non-drive through) road stop
    /// station?
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_bay_road_stop_tile(&self, t: TileIndex) -> bool {
        self.is_station_road_stop_tile(t)
            && self.get_station_gfx(t).0 < GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET
    }
    /// `GetBayRoadStopDir`: gets the direction the bay road stop entrance
    /// points towards.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_bay_road_stop_dir(&self, t: TileIndex) -> DiagDirection {
        self.assert_with(|| self.is_bay_road_stop_tile(t));
        DiagDirection(self.get_station_gfx(t).0)
    }
    /// `GetStationGfx`: get the station graphics of this tile.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_station_gfx(&self, t: TileIndex) -> StationGfx {
        self.assert_with(|| self.kind(t) == TileType::MP_STATION);
        StationGfx(Self::byte(self.types.m5(t)))
    }
    /// `GetStationIndex`: get `StationID` from a tile.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_station_index(&self, t: TileIndex) -> StationID {
        self.assert_with(|| self.kind(t) == TileType::MP_STATION);
        StationID(Self::word(self.types.m2(t)))
    }
    /// `IsStationTileBlocked`: is tile `t` a blocked tile?
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_station_tile_blocked(&self, t: TileIndex) -> bool {
        self.assert_with(|| self.has_station_rail(t));
        has_bit(Self::byte(self.types.m3(t)), 0)
    }
    /// `GetRailStationAxis`: get the rail direction of a rail station.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn get_rail_station_axis(&self, t: TileIndex) -> Axis {
        self.assert_with(|| self.has_station_rail(t));
        if has_bit(self.get_station_gfx(t).0, 0) {
            Axis::AXIS_Y
        } else {
            Axis::AXIS_X
        }
    }
    /// `HasStationReservation`: get the reservation state of the rail station.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn has_station_reservation(&self, t: TileIndex) -> bool {
        self.assert_with(|| self.has_station_rail(t));
        has_bit(Self::byte(self.types.m6(t)), 2)
    }
    /// `IsCompatibleTrainStationTile`: check if `test_tile` is a valid
    /// continuation to the rail station tile `station_tile`: it is a rail
    /// station tile with a compatible railtype, tracks in the same direction,
    /// the same station, and not blocked.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_compatible_train_station_tile(
        &self,
        test_tile: TileIndex,
        station_tile: TileIndex,
    ) -> bool {
        self.assert_with(|| self.is_rail_station_tile(station_tile));
        self.is_rail_station_tile(test_tile)
            && !self.is_station_tile_blocked(test_tile)
            && crate::rail::is_compatible_rail(
                self.shared,
                self.rail_type(test_tile.0),
                self.rail_type(station_tile.0),
            )
            && self.get_rail_station_axis(test_tile) == self.get_rail_station_axis(station_tile)
            && self.get_station_index(test_tile) == self.get_station_index(station_tile)
    }
}

/// Temporary raw adapters for callers outside workset 03; each delegates to
/// the typed accessor named after the original.
impl crate::map_func::World<'_> {
    #[inline]
    pub(crate) fn rail_station(&self, t: u32) -> bool {
        self.is_rail_station_tile(TileIndex(t))
    }
    pub(crate) fn station_has_rail(&self, t: u32) -> bool {
        self.has_station_rail(TileIndex(t))
    }
    pub(crate) fn any_road_stop_tile(&self, t: u32) -> bool {
        self.is_any_road_stop_tile(TileIndex(t))
    }
    pub(crate) fn station_track(&self, t: u32) -> u8 {
        self.get_rail_station_track(TileIndex(t)).0
    }
    pub(crate) fn drive_through_stop_tile(&self, t: u32) -> bool {
        self.is_drive_through_stop_tile(TileIndex(t))
    }
    pub(crate) fn drive_through_axis(&self, t: u32) -> u8 {
        self.get_drive_through_stop_axis(TileIndex(t)).0
    }
    pub(crate) fn road_stop_type(&self, t: u32) -> u8 {
        self.get_road_stop_type(TileIndex(t)).0
    }
    pub(crate) fn rail_waypoint_tile(&self, t: u32) -> bool {
        self.is_rail_waypoint_tile(TileIndex(t))
    }
    #[inline]
    pub(crate) fn station_rail_raw(&self, t: u32) -> bool {
        self.has_station_tile_rail(TileIndex(t))
    }
    #[inline]
    pub(crate) fn station_road_stop(&self, t: u32) -> bool {
        self.is_station_road_stop_tile(TileIndex(t))
    }
    #[inline]
    pub(crate) fn bay_stop(&self, t: u32) -> bool {
        self.is_bay_road_stop_tile(TileIndex(t))
    }
    #[inline]
    pub(crate) fn bay_dir(&self, t: u32) -> u8 {
        self.get_bay_road_stop_dir(TileIndex(t)).0
    }
    #[inline]
    pub(crate) fn station_index(&self, t: u32) -> StationID {
        self.get_station_index(TileIndex(t))
    }
    #[inline]
    pub(crate) fn station_blocked(&self, t: u32) -> bool {
        self.is_station_tile_blocked(TileIndex(t))
    }
    #[inline]
    pub(crate) fn station_axis(&self, t: u32) -> u8 {
        self.get_rail_station_axis(TileIndex(t)).0
    }
    #[inline]
    pub(crate) fn station_reserved(&self, t: u32) -> bool {
        self.has_station_reservation(TileIndex(t))
    }
    #[inline]
    pub(crate) fn compatible_station(&self, t: u32, start: u32) -> bool {
        self.is_compatible_train_station_tile(TileIndex(t), TileIndex(start))
    }
}
