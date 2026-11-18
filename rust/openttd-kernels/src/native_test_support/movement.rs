/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native oracle for the private production movement context.
use crate::map_func::{TileTypeAccess, TileTypeAllocation, World};
use crate::road_map::{get_disallowed_road_directions, get_road_bits, has_tile_road_type};
use crate::road_type::{RTT_ROAD, RTT_TRAM};
use crate::services::Services;
use crate::tile_type::{TileIndex, TileType};

/// The native header accessors' results that the map-read fixture records.
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
    pub tree_ground: u32,
    pub tree_density: u32,
    pub tree_type: u32,
    pub tree_count: u32,
    pub tree_growth: u32,
    pub clear_snow: u32,
    pub clear_density: u32,
    pub clear_counter: u32,
    pub fence_ne: u32,
    pub fence_se: u32,
    pub fence_sw: u32,
    pub fence_nw: u32,
    pub field_type: u32,
    pub field_industry: u32,
}

/// Compare the production movement context's map reads with the native header
/// accessors that `observe` runs, before and after one `mutate`.
/// `mutate` preserves the map allocation and dimensions.
pub(crate) fn map_read_probe(
    allocation: TileTypeAllocation,
    shared: &Services,
    tile: TileIndex,
    mutate: extern "C" fn(u32),
    observe: extern "C" fn(u32, *mut MapLeafOracle),
) -> bool {
    // SAFETY: The native probe supplies the same no-reset interval as its entry.
    let world = World {
        types: unsafe { TileTypeAccess::raw(allocation, shared.tile_type) },
        shared,
        dimensions: shared.map(),
    };
    for pass in 0..2 {
        let mut expected = MapLeafOracle::default();
        observe(tile.0, &raw mut expected);
        let (slope, z) = world.tile_slope_z(tile);
        if u32::from(slope.0) != expected.slope || z != expected.z {
            return false;
        }
        let kind = TileType((shared.tile_type)(tile.0));
        if kind == TileType::MP_WATER
            && (u32::from(world.water_class(tile.0)) != expected.water_class
                || u32::from(world.asserted(|m| crate::water_map::is_coast(m, tile.0)))
                    != expected.coast
                || u32::from(
                    world.asserted(|m| crate::water_map::is_non_flooding_water_tile(m, tile.0)),
                ) != expected.non_flooding)
        {
            return false;
        }
        if u32::from(world.is_normal_road_tile(tile.0)) != expected.normal_road {
            return false;
        }
        if expected.normal_road != 0
            && (u32::from(world.disallowed_road_directions(tile.0)) != expected.directions
                || u32::from(world.has_road_works(tile.0)) != expected.works
                || u32::from(world.road_bits(tile.0, false)) != expected.road_bits
                || u32::from(world.road_bits(tile.0, true)) != expected.tram_bits)
        {
            return false;
        }
        if (shared.may_have_road)(tile.0)
            && (u32::from(world.asserted(|m| has_tile_road_type(m, tile.0, RTT_ROAD)))
                != expected.has_road
                || u32::from(world.asserted(|m| has_tile_road_type(m, tile.0, RTT_TRAM)))
                    != expected.has_tram)
        {
            return false;
        }
        if world.kind(tile) != kind {
            return false;
        }
        if crate::road_map::may_have_road(&world.types, tile)
            != Some((shared.may_have_road)(tile.0))
        {
            return false;
        }
        let station = crate::station_map::get_station_type(&world.types, tile);
        if kind == TileType::MP_STATION {
            if station != Some((shared.station_type)(tile.0)) {
                return false;
            }
        } else if world.types.assertions() == station.is_some() {
            return false;
        }
        if !(kind == TileType::MP_ROAD && world.is_normal_road_tile(tile.0))
            && world.types.assertions()
            && (get_road_bits(&world.types, tile.0, RTT_ROAD).is_some()
                || get_disallowed_road_directions(&world.types, tile.0).is_some())
        {
            return false;
        }
        if !(shared.may_have_road)(tile.0)
            && world.types.assertions()
            && has_tile_road_type(&world.types, tile.0, RTT_ROAD).is_some()
        {
            return false;
        }
        if !world.types.assertions()
            && (get_road_bits(&world.types, tile.0, RTT_TRAM).is_none()
                || has_tile_road_type(&world.types, tile.0, RTT_TRAM).is_none())
        {
            return false;
        }
        if pass == 0 {
            mutate(tile.0);
        }
    }
    true
}
