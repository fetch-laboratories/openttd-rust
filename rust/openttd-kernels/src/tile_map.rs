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
pub(crate) fn is_tile_type(s: &crate::services::Services, tile: u32, kind: u8) -> bool {
    (s.tile_type)(tile) == kind
}

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
#[inline]
#[allow(clippy::cast_possible_truncation)] // trailing_zeros is in 0..=32.
fn corners(m: &TileTypeAccess, t: u32, x: u32, y: u32) -> Option<[i32; 4]> {
    m.available(t)?;
    let map = crate::map_func::Map::from_log_x(x.trailing_zeros());
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
#[inline]
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

/// `IsTileOwner`.
pub fn is_tile_owner(s: &crate::services::Services, tile: u32, owner: u8) -> bool {
    (s.tile_owner)(tile) == owner
}

/// `TileHash`, preserving the source unsigned subtraction.
pub(crate) fn tile_hash(x: u32, y: u32) -> u32 {
    ((x >> 4) ^ (x >> 6) ^ (y >> 4)).wrapping_sub(y >> 6)
}

/// `TileHash2Bit` calls the canonical hash before extracting its low bits.
pub(crate) fn tile_hash_2bit(x: u32, y: u32) -> u32 {
    tile_hash(x, y) & 3
}

/// `GetTilePixelSlope`, preserving unsigned `TILE_HEIGHT` multiplication.
#[inline]
pub(crate) fn get_tile_pixel_slope(
    map: &TileTypeAccess,
    tile: u32,
    x: u32,
    y: u32,
) -> Option<(u8, i32)> {
    let (slope, height_z) = get_tile_slope_z(map, tile, x, y)?;
    Some((
        slope,
        height_z
            .cast_unsigned()
            .wrapping_mul(crate::tile_type::TILE_HEIGHT)
            .cast_signed(),
    ))
}
/// `GetTileMaxZ`.
#[inline]
pub(crate) fn get_tile_max_z(map: &TileTypeAccess, tile: u32, x: u32, y: u32) -> Option<i32> {
    let [north, west, east, south] = corners(map, tile, x, y)?;
    Some(north.max(west).max(east.max(south)))
}
/// `GetTileMaxPixelZ`.
#[inline]
pub(crate) fn get_tile_max_pixel_z(map: &TileTypeAccess, tile: u32, x: u32, y: u32) -> Option<i32> {
    Some(
        get_tile_max_z(map, tile, x, y)?
            .cast_unsigned()
            .wrapping_mul(crate::tile_type::TILE_HEIGHT)
            .cast_signed(),
    )
}
/// `TileHeightOutsideMap`, including signed clamping after the caller's conversion.
#[inline]
pub(crate) fn tile_height_outside_map(
    tiles: &TileTypeAccess,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Option<i32> {
    let map = crate::map_func::Map::from_log_x(width.trailing_zeros());
    tile_height(
        tiles,
        map.tile_xy(
            x.clamp(0, map.max_x().cast_signed()).cast_unsigned(),
            y.clamp(0, crate::map_func::Map::max_y(height).cast_signed())
                .cast_unsigned(),
        ),
    )
}
/// `GetTilePixelSlopeOutsideMap`.
#[inline]
pub(crate) fn get_tile_pixel_slope_outside_map(
    map: &TileTypeAccess,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Option<(u8, i32)> {
    let north = tile_height_outside_map(map, x, y, width, height)?;
    let west = tile_height_outside_map(map, x.wrapping_add(1), y, width, height)?;
    let east = tile_height_outside_map(map, x, y.wrapping_add(1), width, height)?;
    let south = tile_height_outside_map(map, x.wrapping_add(1), y.wrapping_add(1), width, height)?;
    let (slope, height_z) = get_tile_slope_given_height([north, west, east, south]);
    Some((
        slope,
        height_z
            .cast_unsigned()
            .wrapping_mul(crate::tile_type::TILE_HEIGHT)
            .cast_signed(),
    ))
}

#[allow(clippy::cast_possible_truncation)] // GB returns the original field width.
impl crate::map_func::World<'_> {
    pub(crate) fn owner(&self, t: u32) -> u8 {
        self.check(t < self.types.count() && self.kind(t) != crate::tile_type::MP_VOID);
        self.check(
            self.kind(t) != crate::tile_type::MP_HOUSE
                && self.kind(t) != crate::tile_type::MP_INDUSTRY,
        );
        crate::bitmath_func::gb(u32::from(Self::byte(self.types.m1(t))), 0, 5) as u8
    }
    /// `Map::SizeX` and `Map::SizeY` of the acquired allocation.
    fn size(&self) -> (u32, u32) {
        (
            self.dimensions.size_x(),
            self.types.count() >> self.dimensions.log_x,
        )
    }
    fn asserted<T>(&self, value: Option<T>) -> T {
        value.unwrap_or_else(|| {
            self.check(false);
            unreachable!("native map precondition")
        })
    }
    /// `GetTilePixelSlope`.
    pub(crate) fn tile_pixel_slope(&self, t: u32) -> (u8, i32) {
        let (x, y) = self.size();
        self.asserted(get_tile_pixel_slope(&self.types, t, x, y))
    }
    /// `GetTilePixelSlopeOutsideMap`.
    pub(crate) fn tile_pixel_slope_outside_map(&self, x: i32, y: i32) -> (u8, i32) {
        let (width, height) = self.size();
        self.asserted(get_tile_pixel_slope_outside_map(
            &self.types,
            x,
            y,
            width,
            height,
        ))
    }
    /// `GetTileMaxPixelZ`.
    pub(crate) fn tile_max_pixel_z(&self, t: u32) -> i32 {
        let (x, y) = self.size();
        self.asserted(get_tile_max_pixel_z(&self.types, t, x, y))
    }
}
