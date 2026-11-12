/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `pathfinder/pathfinder_func.h` road track helper.
use crate::direction_type::DiagDirection;
use crate::map_func::World;
use crate::movement::MovementServices;
use crate::road_type::{RoadBits, RoadTramType};
use crate::tile_type::TileIndex;
use crate::track_func::track_status_to_trackdir_bits;
use crate::track_type::TrackdirBits;
use crate::transport_type::TransportType;

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
/// Temporary raw adapter of [`get_trackdir_bits_for_road`] for `road_yapf.rs`
/// until its pass.
#[inline]
pub(crate) fn road_trackdirs(movement: &MovementServices, tile: u32, tram: bool) -> u16 {
    get_trackdir_bits_for_road(movement, TileIndex(tile), RoadTramType(tram)).0
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
