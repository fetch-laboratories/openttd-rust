/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `pathfinder/pathfinder_func.h` helpers.
use crate::direction_type::DiagDirection;
use crate::map_func::World;
use crate::math::clamp_u;
use crate::movement::MovementServices;
use crate::road_type::{RoadBits, RoadTramType};
use crate::services::Services;
use crate::station_cmd::CommandServicesTable;
use crate::station_type::{StationID, StationType};
use crate::tile_type::TileIndex;
use crate::track_func::track_status_to_trackdir_bits;
use crate::track_type::TrackdirBits;
use crate::transport_type::TransportType;

/// `CalcClosestStationTile`: calculates the tile of given station that is
/// closest to a given tile. For this we assume the station is a rectangle, as
/// defined by its tile area (x, y, w, h). `commands` is the shared station
/// command table, which serves the virtual `GetTileArea`.
pub(crate) fn calc_closest_station_tile(
    s: &Services,
    commands: &CommandServicesTable,
    map: &World<'_>,
    station: StationID,
    tile: TileIndex,
    station_type: StationType,
) -> TileIndex {
    let st = (s.base_station)(station.0);
    let ta = (commands.tile_area)(st, station_type.0);

    /* If the rail station is (temporarily) not present, use the station sign to drive near the station */
    if ta.tile == TileIndex::INVALID {
        return TileIndex((s.base_station_xy)(station.0));
    }

    let minx = map.dimensions.tile_x(ta.tile); // topmost corner of station
    let miny = map.dimensions.tile_y(ta.tile);
    let maxx = minx.wrapping_add(u32::from(ta.w)).wrapping_sub(1); // lowermost corner of station
    let maxy = miny.wrapping_add(u32::from(ta.h)).wrapping_sub(1);

    /* we are going the aim for the x coordinate of the closest corner
     * but if we are between those coordinates, we will aim for our own x coordinate */
    let x = clamp_u(map.dimensions.tile_x(tile), minx, maxx, |ok| map.check(ok));

    /* same for y coordinate, see above comment */
    let y = clamp_u(map.dimensions.tile_y(tile), miny, maxy, |ok| map.check(ok));

    /* return the tile of our target coordinates */
    map.dimensions.tile_xy(x, y)
}

/// `GetTrackdirBitsForRoad`: wrapper around `GetTileTrackStatus()` and
/// `TrackStatusToTrackdirBits()`, as for single tram bits `GetTileTrackStatus()`
/// returns 0. The reason for this is that there are no half-tile `TrackBits` in
/// `OpenTTD`. This tile, however, is a valid tile for trams, one on which they can
/// reverse safely. To "fix" this, pretend that if we are on a half-tile, we are
/// in fact on a straight tram track tile. `CFollowTrackT` will make sure the
/// pathfinders cannot exit on the wrong side and allows reversing on such tiles.
#[inline]
pub(crate) fn get_trackdir_bits_for_road(
    movement: &MovementServices,
    tile: TileIndex,
    rtt: RoadTramType,
) -> TrackdirBits {
    let map = World::new(movement.shared());
    get_trackdir_bits_for_road_in_world(movement, &map, tile, rtt)
}
/// [`get_trackdir_bits_for_road`] within an enclosing synchronous entry's
/// descriptor, loading fields at each query.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_trackdir_bits_for_road_in_world(
    movement: &MovementServices,
    map: &World<'_>,
    tile: TileIndex,
    rtt: RoadTramType,
) -> TrackdirBits {
    let mut bits = TrackdirBits(track_status_to_trackdir_bits(
        movement.get_tile_track_status_in_world(
            map,
            tile,
            TransportType::TRANSPORT_ROAD,
            u32::from(rtt.0),
            DiagDirection::INVALID_DIAGDIR,
        ),
    ));

    if rtt == RoadTramType::RTT_TRAM
        && bits == TrackdirBits::TRACKDIR_BIT_NONE
        && map.is_normal_road_tile(tile.0)
    {
        let rb = RoadBits(map.road_bits(tile.0, RoadTramType::RTT_TRAM.0));
        match rb {
            RoadBits::ROAD_NE | RoadBits::ROAD_SW => {
                bits = TrackdirBits::TRACKDIR_BIT_X_NE | TrackdirBits::TRACKDIR_BIT_X_SW;
            }
            RoadBits::ROAD_NW | RoadBits::ROAD_SE => {
                bits = TrackdirBits::TRACKDIR_BIT_Y_NW | TrackdirBits::TRACKDIR_BIT_Y_SE;
            }
            _ => {}
        }
    }

    bits
}
/// # Safety
/// The immutable shared table and map remain live and serialized for this call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_trackdirs(
    movement: *const MovementServices,
    tile: u32,
    rtt: RoadTramType,
) -> u16 {
    get_trackdir_bits_for_road(unsafe { &*movement }, TileIndex(tile), rtt).0
}
