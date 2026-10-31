/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `station_map.h` helpers for live map fields and remaining typed services.
use crate::bitmath_func::{gb, has_bit, sb, toggle_bit};
use crate::map_func::TileTypeAccess;
use crate::services::Services;
use crate::station_type::{
    RAIL_WAYPOINT, ROAD_WAYPOINT, STATION_AIRPORT, STATION_BUS, STATION_TRUCK,
};
use crate::tile_type::{MP_STATION, TileIndex};

/// `GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET`.
pub const GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET: u8 = 4;

/// `GetStationType`, including its native tile-type assertion policy.
#[allow(clippy::inline_always)]
#[inline(always)]
#[allow(clippy::cast_possible_truncation)] // GB returns the original four-bit value.
pub(crate) fn get_station_type(
    m: &TileTypeAccess,
    tile: TileIndex,
) -> Option<crate::services::StationType> {
    let fields = m.require_type(tile.0, MP_STATION)?;
    Some(crate::services::StationType(
        gb(u32::from(fields.m6()), 3, 4) as u8,
    ))
}

/// `IsTruckStop`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_truck_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)?.0 == STATION_TRUCK)
}

/// `IsBusStop`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_bus_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)?.0 == STATION_BUS)
}

/// `IsRoadWaypoint`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_road_waypoint(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(get_station_type(m, tile)?.0 == ROAD_WAYPOINT)
}

/// `IsAnyRoadStop`, preserving the source predicate order.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_any_road_stop(m: &TileTypeAccess, tile: TileIndex) -> Option<bool> {
    m.require_type(tile.0, MP_STATION)?;
    Some(is_truck_stop(m, tile)? || is_bus_stop(m, tile)? || is_road_waypoint(m, tile)?)
}

/// `IsAirport`; the station type leaf keeps its native assertion.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_airport(tile: u32, s: &Services) -> bool {
    (s.station_type)(tile).0 == STATION_AIRPORT
}

/// `IsRailWaypoint`; the station type leaf keeps its native assertion.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_rail_waypoint(tile: u32, s: &Services) -> bool {
    (s.station_type)(tile).0 == RAIL_WAYPOINT
}
/// `IsRailWaypointTile`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_rail_waypoint_tile(tile: u32, s: &Services) -> bool {
    (s.tile_type)(tile) == MP_STATION && is_rail_waypoint(tile, s)
}
/// `IsRoadWaypointTile`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_road_waypoint_tile(tile: u32, s: &Services) -> bool {
    (s.tile_type)(tile) == MP_STATION && (s.station_type)(tile).0 == ROAD_WAYPOINT
}

/// `GetAirportGfx`: airport membership is established by each caller before
/// this read, proving its native `IsAirport` assertion. `GetStationGfx` retains its
/// native station assertion; translation calls the Rust owner directly.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_airport_gfx(
    tile: u32,
    s: &Services,
    airport: &crate::newgrf_airport::Services,
) -> u8 {
    crate::newgrf_airport::translate_tile((s.station_gfx)(tile), airport)
}

/// `Station::GetByTile`, retaining its valid-station-or-null lookup.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn station_by_tile(
    tile: u32,
    s: &crate::newgrf_airport::Services,
) -> *mut crate::station::BaseStation {
    (s.by_tile)(tile).cast()
}
/// Original `IsHangar`, retaining its assertion and airport gate before lookup.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_hangar(tile: u32, s: &crate::newgrf_airport::Services) -> bool {
    (s.check)((s.shared().tile_type)(tile) == MP_STATION);
    if !is_airport(tile, s.shared()) {
        return false;
    }
    let st = station_by_tile(tile, s);
    let spec = crate::newgrf_airport::station_spec(st, s);
    for i in 0..(s.depot_count)(spec) {
        let offset = (s.depot_offset)(spec, i);
        if (s.rotated_offset)(st, offset, spec) == tile {
            return true;
        }
    }
    false
}
/// Original `IsHangarTile`, including its non-station early return.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_hangar_tile(tile: u32, s: &crate::newgrf_airport::Services) -> bool {
    (s.shared().tile_type)(tile) == MP_STATION && is_hangar(tile, s)
}

fn require_road_waypoint(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<()> {
    m.available(tile)?;
    if m.assertions() && (m.read(tile) != MP_STATION || !is_road_waypoint(m, TileIndex(tile))?) {
        return None;
    }
    Some(())
}
/// `IsRoadWaypointOnSnowOrDesert`.
pub(crate) fn road_waypoint_snow(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<bool> {
    require_road_waypoint(m, tile)?;
    Some(has_bit(m.m8(tile)?, 15))
}
/// `ToggleRoadWaypointOnSnowOrDesert`.
#[allow(clippy::cast_possible_truncation)] // The source stores back into uint16_t.
pub(crate) fn toggle_road_waypoint_snow(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<()> {
    require_road_waypoint(m, tile)?;
    let mut value = u32::from(m.m8(tile)?);
    toggle_bit(&mut value, 15);
    m.set_m8(tile, value as u16)
}
/// `GetRoadWaypointRoadside`.
#[allow(clippy::cast_possible_truncation)] // GB returns the original two-bit value.
pub(crate) fn road_waypoint_roadside(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::services::Roadside> {
    require_road_waypoint(m, tile)?;
    Some(crate::services::Roadside(
        gb(u32::from(m.m3(tile)?), 2, 2) as u8
    ))
}
/// `SetRoadWaypointRoadside`.
#[allow(clippy::cast_possible_truncation)] // SB stores back into the source byte.
pub(crate) fn set_road_waypoint_roadside(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    value: crate::services::Roadside,
) -> Option<()> {
    require_road_waypoint(m, tile)?;
    let mut bits = u32::from(m.m3(tile)?);
    sb(&mut bits, 2, 2, u32::from(value.0));
    m.set_m3(tile, bits as u8)
}

impl crate::map_func::World<'_> {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn rail_station(&self, t: u32) -> bool {
        self.kind(t) == MP_STATION
            && get_station_type(&self.types, TileIndex(t))
                .expect("station tile")
                .0
                == crate::station_type::STATION_RAIL
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn has_station_rail(&self, t: u32) -> bool {
        self.kind(t) == MP_STATION
            && matches!(
                get_station_type(&self.types, TileIndex(t))
                    .expect("station tile")
                    .0,
                crate::station_type::STATION_RAIL | RAIL_WAYPOINT
            )
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn station_road_stop(&self, t: u32) -> bool {
        self.kind(t) == MP_STATION
            && matches!(
                get_station_type(&self.types, TileIndex(t))
                    .expect("station tile")
                    .0,
                STATION_TRUCK | STATION_BUS
            )
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn bay_stop(&self, t: u32) -> bool {
        self.station_road_stop(t) && self.station_gfx(t) < GFX_TRUCK_BUS_DRIVETHROUGH_OFFSET
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn bay_dir(&self, t: u32) -> u8 {
        self.check(self.bay_stop(t));
        self.station_gfx(t)
    }
    /// `GetStationGfx`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn station_gfx(&self, t: u32) -> u8 {
        self.check(self.kind(t) == MP_STATION);
        Self::byte(self.types.m5(t))
    }
    /// `GetStationIndex`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn station_index(&self, t: u32) -> crate::station_type::StationID {
        self.check(self.kind(t) == MP_STATION);
        crate::station_type::StationID(Self::word(self.types.m2(t)))
    }
    /// `IsStationTileBlocked`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn station_blocked(&self, t: u32) -> bool {
        self.check(self.has_station_rail(t));
        has_bit(Self::byte(self.types.m3(t)), 0)
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn station_axis(&self, t: u32) -> u8 {
        self.check(self.has_station_rail(t));
        u8::from(has_bit(self.station_gfx(t), 0))
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn station_reserved(&self, t: u32) -> bool {
        self.check(self.has_station_rail(t));
        has_bit(Self::byte(self.types.m6(t)), 2)
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn compatible_station(&self, t: u32, start: u32) -> bool {
        self.check(self.rail_station(start));
        self.rail_station(t)
            && !self.station_blocked(t)
            && crate::rail::is_compatible_rail(
                self.shared,
                self.rail_type(t),
                self.rail_type(start),
            )
            && self.station_axis(t) == self.station_axis(start)
            && self.station_index(t) == self.station_index(start)
    }
}
