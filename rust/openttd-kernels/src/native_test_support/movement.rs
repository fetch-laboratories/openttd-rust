/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native oracle for the private production movement context.
use super::{Context, MapObservations, table};
use crate::services::Services;
use crate::tile_type::{MP_ROAD, TILE_SIZE};

/// Native map probe uses the production context against unchanged native helpers.
/// The mutation callback preserves allocation/dimensions and serialization.
pub(crate) fn map_read_probe(
    allocation: crate::map_func::TileTypeAllocation,
    shared: &Services,
    tile: u32,
    mutate: extern "C" fn(u32),
) -> bool {
    // SAFETY: The native probe supplies the same no-reset interval as its entry.
    let observations = MapObservations {
        types: unsafe { crate::map_func::TileTypeAccess::raw(allocation, shared.tile_type) },
        x: (shared.map_max_x)() + 1,
        y: (shared.map_max_y)() + 1,
    };
    let s = table(shared);
    let context = Context::<true> {
        s,
        observations: Some(&observations),
    };
    for pass in 0..2 {
        let map = crate::map_func::Map::from_log_x(observations.x.trailing_zeros());
        let x = (map.tile_x(tile) * TILE_SIZE + TILE_SIZE / 2).cast_signed();
        let y = (map.tile_y(tile) * TILE_SIZE + TILE_SIZE / 2).cast_signed();
        let expected = (s.pixel_location)(x, y);
        let actual = context.pixel_location(x, y);
        let slope = context.pixel_slope(tile);
        let expected_slope = (s.pixel_slope)(tile);
        let kind = (shared.tile_type)(tile);
        if actual.tile != expected.tile
            || actual.kind != expected.kind
            || context.tile_type(tile) != kind
            || slope.slope != expected_slope.slope
            || slope.z != expected_slope.z
        {
            return false;
        }
        if kind == MP_ROAD && (s.normal_road)(tile) {
            let tail = context.normal_road_status_tail(tile);
            let expected = (s.normal_road_status_tail)(tile);
            if tail.disallowed != expected.disallowed || tail.road_works != expected.road_works {
                return false;
            }
            for kind in [crate::road_type::RTT_ROAD, crate::road_type::RTT_TRAM] {
                if context.normal_road_bits(tile, kind)
                    != (shared.normal_road_bits)(tile, u8::from(kind.0))
                {
                    return false;
                }
            }
        } else if observations.types.assertions()
            && (crate::road_map::get_road_bits(
                &observations.types,
                tile,
                crate::road_type::RTT_ROAD,
            )
            .is_some()
                || crate::road_map::get_disallowed_road_directions(&observations.types, tile)
                    .is_some())
        {
            return false;
        }
        if (shared.may_have_road)(tile) {
            for kind in [crate::road_type::RTT_ROAD, crate::road_type::RTT_TRAM] {
                if context.has_tile_road_type(tile, kind) != (s.has_tile_road_type)(tile, kind) {
                    return false;
                }
            }
        } else if observations.types.assertions()
            && crate::road_map::has_tile_road_type(
                &observations.types,
                shared,
                tile,
                crate::road_type::RTT_ROAD,
            )
            .is_some()
        {
            return false;
        }
        if !observations.types.assertions()
            && (crate::road_map::get_road_bits(
                &observations.types,
                tile,
                crate::road_type::RTT_TRAM,
            )
            .is_none()
                || crate::road_map::has_tile_road_type(
                    &observations.types,
                    shared,
                    tile,
                    crate::road_type::RTT_TRAM,
                )
                .is_none())
        {
            return false;
        }
        if pass == 0 {
            mutate(tile);
        }
    }
    true
}
