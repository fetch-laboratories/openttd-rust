/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `tile_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::TileTypeAccess;

#[inline]
pub(crate) fn get_tile_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(m.kind(t)? >> 4)
}
#[inline]
pub(crate) fn get_tropic_zone(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(m.kind(t)? & 3)
}
#[inline]
pub(crate) fn is_valid_tile(m: &TileTypeAccess, t: u32, count: u32) -> Option<bool> {
    m.available(0)?;
    Some(t < count && get_tile_type(m, t)? != crate::tile_type::MP_VOID)
}
#[inline]
fn tile_height(m: &TileTypeAccess, t: u32) -> Option<i32> {
    Some(i32::from(m.height(t)?))
}
#[allow(clippy::cast_possible_truncation)] // trailing_zeros is in 0..=32.
fn corners(m: &TileTypeAccess, t: u32, x: u32, y: u32) -> Option<[i32; 4]> {
    m.available(t)?;
    let map = crate::map_func::Map {
        log_x: x.trailing_zeros() as u8,
    };
    let x1 = map.tile_x(t);
    let y1 = map.tile_y(t);
    let x2 = (x1 + 1).min(map.max_x());
    let y2 = (y1 + 1).min(crate::map_func::Map::max_y(y));
    Some([
        tile_height(m, t)?,
        tile_height(m, map.tile_xy(x2, y1))?,
        tile_height(m, map.tile_xy(x1, y2))?,
        tile_height(m, map.tile_xy(x2, y2))?,
    ])
}
/// `GetTileSlopeGivenHeight`, also called by `GetTileSlopeZ` in the original.
fn get_tile_slope_given_height([n, w, e, s]: [i32; 4]) -> (u8, i32) {
    use crate::slope_func::{SLOPE_E, SLOPE_FLAT, SLOPE_N, SLOPE_S, SLOPE_STEEP, SLOPE_W};
    let low = n.min(w).min(e.min(s));
    let high = n.max(w).max(e.max(s));
    let mut slope = SLOPE_FLAT;
    for (h, bit) in [(n, SLOPE_N), (w, SLOPE_W), (e, SLOPE_E), (s, SLOPE_S)] {
        if h != low {
            slope |= bit;
        }
    }
    if high - low == 2 {
        slope |= SLOPE_STEEP;
    }
    (slope, low)
}
#[inline]
pub(crate) fn get_tile_slope_z(m: &TileTypeAccess, t: u32, x: u32, y: u32) -> Option<(u8, i32)> {
    Some(get_tile_slope_given_height(corners(m, t, x, y)?))
}
#[inline]
pub(crate) fn get_tile_slope(m: &TileTypeAccess, t: u32, x: u32, y: u32) -> Option<u8> {
    Some(get_tile_slope_z(m, t, x, y)?.0)
}
#[inline]
pub(crate) fn get_tile_z(m: &TileTypeAccess, t: u32, x: u32, y: u32) -> Option<i32> {
    let [north, west, east, south] = corners(m, t, x, y)?;
    Some(north.min(west).min(east.min(south)))
}

/// Native default probe: compare audited reads with the unchanged typed services.
/// # Safety
/// Same allocation contract as the scheduler; all tables are live and immutable.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub unsafe extern "C" fn openttd_rust_map_read_probe(
    allocation: crate::services::TileTypeAllocation,
    shared: *const crate::services::Services,
    clear: *const crate::ground::ClearServices,
    trees: *const crate::trees::TreeServices,
    tile: u32,
) -> bool {
    let (s, c, trees) = unsafe { (&*shared, &*clear, &*trees) };
    let map = unsafe { TileTypeAccess::raw(allocation, s.tile_type) };
    let x = (s.map_max_x)() + 1;
    let y = (s.map_max_y)() + 1;
    if get_tile_slope(&map, tile, x, y) != Some((s.tile_slope)(tile))
        || get_tile_z(&map, tile, x, y) != Some((s.tile_z)(tile))
        || get_tropic_zone(&map, tile) != Some((s.tropic_zone)(tile))
    {
        return false;
    }
    match map.read(tile) {
        0 => {
            use crate::clear_map::{
                get_clear_counter, get_clear_density, get_clear_ground, get_fence, get_field_type,
                get_industry_index_of_field, is_snow_tile,
            };
            if is_snow_tile(&map, tile) != Some((c.snow)(tile))
                || get_clear_ground(&map, tile) != Some((s.clear_ground)(tile).0)
                || get_clear_density(&map, tile).map(u32::from) != Some((c.density)(tile))
                || get_clear_counter(&map, tile).map(u32::from) != Some((c.counter)(tile))
            {
                return false;
            }
            if get_clear_ground(&map, tile) == Some(crate::clear_map::CLEAR_FIELDS) {
                let fences = (c.fences)(tile);
                let field = (c.field)(tile);
                if [fences.ne, fences.se, fences.sw, fences.nw]
                    .into_iter()
                    .zip(0_u8..)
                    .any(|(expected, dir)| get_fence(&map, tile, dir) != Some(expected))
                    || get_field_type(&map, tile) != Some(field.field_type)
                    || get_industry_index_of_field(&map, tile) != Some(field.industry)
                {
                    return false;
                }
            } else if allocation.assertions && get_field_type(&map, tile).is_some() {
                return false;
            }
        }
        4 => {
            use crate::tree_map::{
                get_tree_count, get_tree_density, get_tree_ground, get_tree_growth, get_tree_type,
            };
            let tree = (trees.tree_observation)(tile);
            if get_tree_ground(&map, tile) != Some(tree.ground)
                || get_tree_density(&map, tile) != Some(tree.density)
                || get_tree_type(&map, tile) != Some(tree.species)
                || get_tree_count(&map, tile) != Some(tree.count)
                || get_tree_growth(&map, tile) != Some(tree.growth)
            {
                return false;
            }
        }
        6 => {
            let water = unsafe { &*(s.water_services)() };
            if crate::water_map::is_coast(&map, tile) != Some((water.coast)(tile))
                || crate::water_map::is_non_flooding_water_tile(&map, tile)
                    != Some((water.non_flooding)(tile))
                || crate::water_map::get_water_class(&map, tile) != Some((c.water_class)(tile).0)
            {
                return false;
            }
        }
        _ => {}
    }
    let kind = map.read(tile);
    (!allocation.assertions || kind == 0 || crate::clear_map::is_snow_tile(&map, tile).is_none())
        && (!allocation.assertions
            || kind == 4
            || crate::tree_map::get_tree_type(&map, tile).is_none())
        && (!allocation.assertions || kind == 6 || crate::water_map::is_coast(&map, tile).is_none())
        && (allocation.assertions || crate::clear_map::is_snow_tile(&map, tile).is_some())
}

/// `IsTileOwner`.
pub fn is_tile_owner(s: &crate::services::Services, tile: u32, owner: u8) -> bool {
    (s.tile_owner)(tile) == owner
}

/// `TileHash2Bit`, including the unsigned subtraction in `TileHash`.
pub(crate) fn tile_hash_2bit(x: u32, y: u32) -> u32 {
    ((x >> 4) ^ (x >> 6) ^ (y >> 4)).wrapping_sub(y >> 6) & 3
}
