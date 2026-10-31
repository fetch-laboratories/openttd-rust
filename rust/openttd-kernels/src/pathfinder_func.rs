/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `pathfinder/pathfinder_func.h` road track helper.
use crate::direction_type::INVALID_DIAGDIR;
use crate::movement::MovementServices;
use crate::track_func::track_status_to_trackdir_bits;
use crate::track_type::{
    TRACKDIR_BIT_X_NE, TRACKDIR_BIT_X_SW, TRACKDIR_BIT_Y_NW, TRACKDIR_BIT_Y_SE,
};
use crate::transport_type::TRANSPORT_ROAD;

#[allow(unsafe_code)]
pub(crate) fn road_trackdirs(movement: &MovementServices, tile: u32, tram: bool) -> u16 {
    let mut bits = track_status_to_trackdir_bits(movement.track_status(
        tile,
        TRANSPORT_ROAD,
        u32::from(tram),
        INVALID_DIAGDIR,
    ));
    if tram && bits == 0 {
        // SAFETY: Game-loop/command entries exclude map reset/load/mutex yield.
        let map = movement.tile_types.map(|allocate| unsafe {
            crate::map_func::TileTypeAccess::raw(allocate(), movement.shared().tile_type)
        });
        let normal = map.as_ref().and_then(|m| {
            if m.read(tile) == crate::tile_type::MP_ROAD {
                crate::road_map::is_normal_road(m, tile)
            } else {
                Some(false)
            }
        });
        if normal.unwrap_or_else(|| (movement.shared().is_normal_road_tile)(tile)) {
            let road = map
                .as_ref()
                .and_then(|m| crate::road_map::get_road_bits(m, tile, crate::road_type::RTT_TRAM))
                .unwrap_or_else(|| (movement.shared().normal_road_bits)(tile, 1));
            bits = match road.0 {
                crate::road_type::ROAD_NE | crate::road_type::ROAD_SW => {
                    TRACKDIR_BIT_X_NE | TRACKDIR_BIT_X_SW
                }
                crate::road_type::ROAD_NW | crate::road_type::ROAD_SE => {
                    TRACKDIR_BIT_Y_NW | TRACKDIR_BIT_Y_SE
                }
                _ => bits,
            };
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
    tram: crate::services::RoadTramType,
) -> u16 {
    road_trackdirs(unsafe { &*movement }, tile, tram.0)
}
