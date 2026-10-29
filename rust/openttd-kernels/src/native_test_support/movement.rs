/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native oracle for the private production movement context.
use super::{Context, MapObservations, table};
use crate::services::Services;
use crate::tile_type::{MP_ROAD, MP_STATION, TileIndex};

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
        dimensions: shared.map(),
        types: unsafe { crate::map_func::TileTypeAccess::raw(allocation, shared.tile_type) },
    };
    let s = table(shared);
    let context = Context::<true> {
        s,
        observations: Some(&observations),
    };
    for pass in 0..2 {
        let kind = (shared.tile_type)(tile);
        if context.tile_type(tile) != kind {
            return false;
        }
        if crate::road_map::may_have_road(&observations.types, TileIndex(tile))
            != Some((shared.may_have_road)(tile))
        {
            return false;
        }
        let station = crate::station_map::get_station_type(&observations.types, TileIndex(tile));
        if kind == MP_STATION {
            if station.map(|kind| kind.0) != Some((shared.station_type)(tile).0) {
                return false;
            }
        } else if observations.types.assertions() == station.is_some() {
            return false;
        }
        if kind == MP_ROAD && (shared.is_normal_road)(tile) {
            if context.disallowed_road_directions(tile) != (shared.disallowed_road_directions)(tile)
                || context.has_road_works(tile) != (shared.has_road_works)(tile)
            {
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
