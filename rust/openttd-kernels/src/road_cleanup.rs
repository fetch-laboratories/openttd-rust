/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Planned road connectivity over the canonical map; `road.cpp`.
use crate::direction_type::DiagDirection;
use crate::map_func::World;
use crate::services::{RoadBits, Services};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;

/// `IsPossibleCrossing`, including the foundation-adjusted surface.
pub(crate) fn is_possible_crossing(world: &World<'_>, tile: u32, axis: u8) -> bool {
    world.kind(TileIndex(tile)) == TileType::MP_RAILWAY
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
        let neighbour = world
            .dimensions
            .tile_add_by_diag_dir(
                TileIndex(tile),
                DiagDirection(dir),
                crate::map_func::native_check,
            )
            .0;
        let target = crate::road_func::diag_dir_to_road_bits_raw(dir, check);
        if bits.0 & target != 0 {
            let mirrored = crate::road_func::mirror_road_bits(target, check);
            let connective = valid(neighbour)
                && match world.kind(TileIndex(neighbour)) {
                    TileType::MP_CLEAR | TileType::MP_TREES => true,
                    TileType::MP_TUNNELBRIDGE | TileType::MP_STATION | TileType::MP_ROAD => {
                        if world.is_normal_road_tile(neighbour) {
                            true
                        } else {
                            let road = (shared.road_bits)(neighbour, 0, false);
                            let tram = (shared.road_bits)(neighbour, 1, false);
                            (road | tram) & mirrored != crate::road_type::ROAD_NONE
                        }
                    }
                    TileType::MP_RAILWAY => is_possible_crossing(
                        &world,
                        neighbour,
                        crate::direction_func::diag_dir_to_axis_raw(dir, check),
                    ),
                    TileType::MP_WATER => {
                        !world.asserted(|m| crate::water_map::is_water(m, neighbour))
                    }
                    _ => false,
                };
            if !connective {
                bits.0 ^= target;
            }
        }
    }
    bits
}
