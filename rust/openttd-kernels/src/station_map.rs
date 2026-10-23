/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared waypoint predicates from `station_map.h`, through typed map services.
use crate::services::Services;
use crate::station_type::{RAIL_WAYPOINT, ROAD_WAYPOINT};
use crate::tile_type::MP_STATION;

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
