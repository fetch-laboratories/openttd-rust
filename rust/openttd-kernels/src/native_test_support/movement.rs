/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native oracle for the private production movement context.
use crate::services::Services;
use crate::tile_type::{MP_ROAD, MP_STATION, TileIndex};

/// Unchanged native header observations for the existing map-read fixture.
#[repr(C)]
#[derive(Default)]
pub(crate) struct MapLeafOracle {
    pub slope: u32,
    pub z: i32,
    pub water_class: u32,
    pub coast: u32,
    pub non_flooding: u32,
    pub normal_road: u32,
    pub directions: u32,
    pub works: u32,
    pub road_bits: u32,
    pub tram_bits: u32,
    pub has_road: u32,
    pub has_tram: u32,
}

/// Native map probe uses the production context against unchanged native helpers.
/// The mutation callback preserves allocation/dimensions and serialization.
pub(crate) fn map_read_probe(
    allocation: crate::map_func::TileTypeAllocation,
    shared: &Services,
    tile: u32,
    mutate: extern "C" fn(u32),
    observe: extern "C" fn(u32, *mut MapLeafOracle),
) -> bool {
    // SAFETY: The native probe supplies the same no-reset interval as its entry.
    let world = crate::map_func::World {
        types: unsafe { crate::map_func::TileTypeAccess::raw(allocation, shared.tile_type) },
        shared,
        dimensions: shared.map(),
    };
    for pass in 0..2 {
        let mut expected = MapLeafOracle::default();
        observe(tile, &raw mut expected);
        let (slope, z) = world.tile_slope_z(crate::tile_type::TileIndex(tile));
        if u32::from(slope.0) != expected.slope || z != expected.z {
            return false;
        }
        let kind = (shared.tile_type)(tile);
        if kind == crate::tile_type::MP_WATER
            && (u32::from(world.water_class(tile)) != expected.water_class
                || u32::from(world.asserted(|m| crate::water_map::is_coast(m, tile)))
                    != expected.coast
                || u32::from(
                    world.asserted(|m| crate::water_map::is_non_flooding_water_tile(m, tile)),
                ) != expected.non_flooding)
        {
            return false;
        }
        if u32::from(world.is_normal_road_tile(tile)) != expected.normal_road {
            return false;
        }
        if expected.normal_road != 0
            && (u32::from(world.disallowed_road_directions(tile)) != expected.directions
                || u32::from(world.has_road_works(tile)) != expected.works
                || u32::from(world.road_bits(tile, false)) != expected.road_bits
                || u32::from(world.road_bits(tile, true)) != expected.tram_bits)
        {
            return false;
        }
        if (shared.may_have_road)(tile)
            && (u32::from(world.asserted(|m| {
                crate::road_map::has_tile_road_type(m, tile, crate::road_type::RTT_ROAD)
            })) != expected.has_road
                || u32::from(world.asserted(|m| {
                    crate::road_map::has_tile_road_type(m, tile, crate::road_type::RTT_TRAM)
                })) != expected.has_tram)
        {
            return false;
        }
        if world.kind(crate::tile_type::TileIndex(tile)) != crate::tile_type::TileType(kind) {
            return false;
        }
        if crate::road_map::may_have_road(&world.types, TileIndex(tile))
            != Some((shared.may_have_road)(tile))
        {
            return false;
        }
        let station = crate::station_map::get_station_type(&world.types, TileIndex(tile));
        if kind == MP_STATION {
            if station.map(|kind| kind.0) != Some((shared.station_type)(tile).0) {
                return false;
            }
        } else if world.types.assertions() == station.is_some() {
            return false;
        }
        if !(kind == MP_ROAD && world.is_normal_road_tile(tile))
            && world.types.assertions()
            && (crate::road_map::get_road_bits(&world.types, tile, crate::road_type::RTT_ROAD)
                .is_some()
                || crate::road_map::get_disallowed_road_directions(&world.types, tile).is_some())
        {
            return false;
        }
        if !(shared.may_have_road)(tile)
            && world.types.assertions()
            && crate::road_map::has_tile_road_type(&world.types, tile, crate::road_type::RTT_ROAD)
                .is_some()
        {
            return false;
        }
        if !world.types.assertions()
            && (crate::road_map::get_road_bits(&world.types, tile, crate::road_type::RTT_TRAM)
                .is_none()
                || crate::road_map::has_tile_road_type(
                    &world.types,
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
