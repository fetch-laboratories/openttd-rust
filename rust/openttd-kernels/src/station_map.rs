/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `station_map.h` helpers.
use crate::{
    services::Services,
    station_type::{STATION_BUS, STATION_TRUCK},
    tile_type::MP_STATION,
};

/// `IsStationRoadStopTile`: a bus or truck stop, not a road waypoint.
pub(crate) fn is_station_road_stop_tile(s: &Services, tile: u32) -> bool {
    (s.tile_type)(tile) == MP_STATION
        && matches!((s.station_type)(tile).0, STATION_TRUCK | STATION_BUS)
}
