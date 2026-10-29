/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared station predicates from `station_map.h`, through typed map services.
use crate::services::Services;
use crate::station_type::{
    RAIL_WAYPOINT, ROAD_WAYPOINT, STATION_AIRPORT, STATION_BUS, STATION_TRUCK,
};
use crate::tile_type::MP_STATION;

/// `IsAirport`; the station type leaf keeps its native assertion.
pub(crate) fn is_airport(tile: u32, s: &Services) -> bool {
    (s.station_type)(tile).0 == STATION_AIRPORT
}

/// `IsRailWaypoint`; the station type leaf keeps its native assertion.
pub(crate) fn is_rail_waypoint(tile: u32, s: &Services) -> bool {
    (s.station_type)(tile).0 == RAIL_WAYPOINT
}
/// `IsRailWaypointTile`.
pub(crate) fn is_rail_waypoint_tile(tile: u32, s: &Services) -> bool {
    (s.tile_type)(tile) == MP_STATION && is_rail_waypoint(tile, s)
}
/// `IsRoadWaypointTile`.
pub(crate) fn is_road_waypoint_tile(tile: u32, s: &Services) -> bool {
    (s.tile_type)(tile) == MP_STATION && (s.station_type)(tile).0 == ROAD_WAYPOINT
}

/// `HasStationTileRail`, retaining its guard before `HasStationRail`'s assertions.
pub(crate) fn has_station_tile_rail(
    tile: u32,
    s: &Services,
    rail: &crate::services::RailMapServices,
) -> bool {
    (s.tile_type)(tile) == MP_STATION && (rail.has_station_rail)(tile)
}

/// `IsStationRoadStopTile`: a bus or truck stop, not a road waypoint.
pub(crate) fn is_station_road_stop_tile(s: &Services, tile: u32) -> bool {
    (s.tile_type)(tile) == MP_STATION
        && matches!((s.station_type)(tile).0, STATION_TRUCK | STATION_BUS)
}

/// `GetAirportGfx`: airport membership is established by each caller before
/// this read, proving its native `IsAirport` assertion. `GetStationGfx` retains its
/// native station assertion; translation calls the Rust owner directly.
pub(crate) fn get_airport_gfx(
    tile: u32,
    s: &Services,
    airport: &crate::newgrf_airport::Services,
) -> u8 {
    crate::newgrf_airport::translate_tile((s.station_gfx)(tile), airport)
}

/// `Station::GetByTile`, retaining its valid-station-or-null lookup.
pub(crate) fn station_by_tile(
    tile: u32,
    s: &crate::newgrf_airport::Services,
) -> *mut crate::station::BaseStation {
    (s.by_tile)(tile).cast()
}
/// Original `IsHangar`, retaining its assertion and airport gate before lookup.
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
pub(crate) fn is_hangar_tile(tile: u32, s: &crate::newgrf_airport::Services) -> bool {
    (s.shared().tile_type)(tile) == MP_STATION && is_hangar(tile, s)
}

impl crate::map_func::World<'_> {
    pub(crate) fn station_kind(&self, t: u32) -> u8 {
        self.check(self.kind(t) == MP_STATION);
        (self.byte(self.types.m6(t)) >> 3) & 15
    }
    pub(crate) fn rail_station(&self, t: u32) -> bool {
        self.kind(t) == MP_STATION && self.station_kind(t) == crate::station_type::STATION_RAIL
    }
    pub(crate) fn has_station_rail(&self, t: u32) -> bool {
        self.kind(t) == MP_STATION
            && matches!(
                self.station_kind(t),
                crate::station_type::STATION_RAIL | RAIL_WAYPOINT
            )
    }
    pub(crate) fn station_road_stop(&self, t: u32) -> bool {
        self.kind(t) == MP_STATION && matches!(self.station_kind(t), STATION_TRUCK | STATION_BUS)
    }
    pub(crate) fn bay_stop(&self, t: u32) -> bool {
        self.station_road_stop(t) && self.byte(self.types.m5(t)) < 4
    }
    pub(crate) fn bay_dir(&self, t: u32) -> u8 {
        self.check(self.bay_stop(t));
        self.byte(self.types.m5(t))
    }
    pub(crate) fn station_axis(&self, t: u32) -> u8 {
        self.check(self.has_station_rail(t));
        self.byte(self.types.m5(t)) & 1
    }
    pub(crate) fn station_reserved(&self, t: u32) -> bool {
        self.check(self.has_station_rail(t));
        self.byte(self.types.m6(t)) & 4 != 0
    }
    pub(crate) fn compatible_station(&self, t: u32, start: u32) -> bool {
        self.check(self.rail_station(start));
        self.rail_station(t)
            && self.byte(self.types.m3(t)) & 1 == 0
            && (crate::pbs::Pbs::acquire(self.shared).0.compatible)(self.rail_type(t))
                & (1_u64 << self.rail_type(start))
                != 0
            && self.station_axis(t) == self.station_axis(start)
            && self.word(self.types.m2(t)) == self.word(self.types.m2(start))
    }
}
