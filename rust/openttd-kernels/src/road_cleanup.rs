/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Planned road connectivity over the canonical map; `road.cpp`.
use crate::map_func::World;
use crate::services::{RoadBits, Services};
use crate::tile_type::{
    MP_CLEAR, MP_RAILWAY, MP_ROAD, MP_STATION, MP_TREES, MP_TUNNELBRIDGE, MP_WATER,
};

/// `IsPossibleCrossing`, including the foundation-adjusted surface.
pub(crate) fn is_possible_crossing(world: &World<'_>, tile: u32, axis: u8) -> bool {
    world.kind(tile) == MP_RAILWAY
        && world.rail_tile_type(tile) == crate::rail_map::RAIL_TILE_NORMAL
        && world.tracks(tile)
            == if axis == crate::direction_type::AXIS_X {
                crate::track_type::TRACK_BIT_Y
            } else {
                crate::track_type::TRACK_BIT_X
            }
        && crate::movement::foundation_slope(world.shared, tile)
            .slope
            .0
            == crate::slope_func::SLOPE_FLAT
}

/// `CleanUpRoadBits`. Only a selected direction bit may be removed; high bits survive.
pub(crate) fn clean_up_road_bits(shared: &Services, tile: u32, mut bits: RoadBits) -> RoadBits {
    let world = World::new(shared);
    let valid =
        |tile| world.asserted(|m| crate::tile_map::is_valid_tile(m, tile, world.types.count()));
    if !valid(tile) {
        return RoadBits(crate::road_type::ROAD_NONE);
    }
    let check = |ok| world.check(ok);
    for dir in crate::direction_type::DIAGDIR_BEGIN..crate::direction_type::DIAGDIR_END {
        // `TileAdd` also preserves its native `_DEBUG` coordinate assertions.
        let neighbour = (crate::movement::table(shared).add_by_diag_dir)(tile, dir);
        let target = crate::road_func::diag_dir_to_road_bits(dir, check);
        if bits.0 & target != 0 {
            let mirrored = crate::road_func::mirror_road_bits(target, check);
            let connective = valid(neighbour)
                && match world.kind(neighbour) {
                    MP_CLEAR | MP_TREES => true,
                    MP_TUNNELBRIDGE | MP_STATION | MP_ROAD => {
                        if world.is_normal_road_tile(neighbour) {
                            true
                        } else {
                            let road = (shared.road_bits)(neighbour, 0, false);
                            let tram = (shared.road_bits)(neighbour, 1, false);
                            (road | tram) & mirrored != crate::road_type::ROAD_NONE
                        }
                    }
                    MP_RAILWAY => is_possible_crossing(
                        &world,
                        neighbour,
                        crate::direction_func::diag_dir_to_axis(dir, check),
                    ),
                    MP_WATER => !world.asserted(|m| crate::water_map::is_water(m, neighbour)),
                    _ => false,
                };
            if !connective {
                bits.0 ^= target;
            }
        }
    }
    bits
}
