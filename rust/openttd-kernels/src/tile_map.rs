/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Helpers of `tile_map.h` for the audited map view.
//! `None` denotes an unavailable raw backend or a failed native precondition.
use crate::map_func::TileTypeAccess;
use crate::slope_type::Slope;
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
use crate::tile_type::TropicZone;

#[allow(clippy::inline_always)]
#[inline(always)]
/// `IsTileType`: whether `GetTileType` of the tile is `kind`.
pub(crate) fn is_tile_type(s: &crate::services::Services, tile: TileIndex, kind: TileType) -> bool {
    crate::map_func::World::new(s).kind(tile) == kind
}

#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_type(m: &TileTypeAccess, t: TileIndex) -> Option<TileType> {
    let fields = m.require_range(t)?;
    Some(get_tile_type_from_fields(&fields))
}
/// `GetTileType`'s bit extraction after the caller's original range assertion.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_type_from_fields(fields: &crate::map_func::TileFields<'_>) -> TileType {
    TileType(crate::bitmath_func::gb_u8(fields.kind(), 4, 4))
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tropic_zone(m: &TileTypeAccess, t: TileIndex) -> Option<TropicZone> {
    Some(TropicZone(crate::bitmath_func::gb_u8(m.kind(t)?, 0, 2)))
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_valid_tile(m: &TileTypeAccess, t: TileIndex, count: u32) -> Option<bool> {
    m.available(TileIndex(0))?;
    Some(t < TileIndex(count) && get_tile_type(m, t)? != TileType::MP_VOID)
}
#[allow(clippy::inline_always)]
#[inline(always)]
fn tile_height(m: &TileTypeAccess, t: TileIndex) -> Option<i32> {
    let fields = m.require_range(t)?;
    Some(i32::from(fields.height()))
}
#[allow(clippy::inline_always)]
#[inline(always)]
#[allow(clippy::cast_possible_truncation)] // trailing_zeros is in 0..=32.
fn corners(m: &TileTypeAccess, t: TileIndex, x: u32, y: u32) -> Option<[i32; 4]> {
    m.available(t)?;
    let map = crate::map_func::Map::from_log_x(x.trailing_zeros());
    let x1 = map.tile_x(t);
    let y1 = map.tile_y(t);
    let x2 = (x1 + 1).min(map.max_x());
    let y2 = (y1 + 1).min(crate::map_func::Map::max_y(y));
    Some([
        tile_height(m, t)?,                   // N corner
        tile_height(m, map.tile_xy(x2, y1))?, // W corner
        tile_height(m, map.tile_xy(x1, y2))?, // E corner
        tile_height(m, map.tile_xy(x2, y2))?, // S corner
    ])
}
/// `GetTileSlopeGivenHeight`, also called by `GetTileSlopeZ` in the original.
#[allow(clippy::inline_always)]
#[inline(always)]
fn get_tile_slope_given_height([n, w, e, s]: [i32; 4]) -> (Slope, i32) {
    use crate::slope_func::{SLOPE_E, SLOPE_FLAT, SLOPE_N, SLOPE_S, SLOPE_STEEP, SLOPE_W};
    /* Due to the fact that tiles must connect with each other without leaving gaps, the
     * biggest difference in height between any corner and 'min' is between 0, 1, or 2.
     *
     * Also, there is at most 1 corner with height difference of 2.
     */
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
    (Slope(slope), low)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_slope_z(
    m: &TileTypeAccess,
    t: TileIndex,
    x: u32,
    y: u32,
) -> Option<(Slope, i32)> {
    Some(get_tile_slope_given_height(corners(m, t, x, y)?))
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_slope(m: &TileTypeAccess, t: TileIndex, x: u32, y: u32) -> Option<Slope> {
    Some(get_tile_slope_z(m, t, x, y)?.0)
}
/// `IsTileFlat` without the height output: each corner is read only while the
/// earlier ones match, as the original returns at the first difference.
#[inline]
pub(crate) fn is_tile_flat(m: &TileTypeAccess, t: TileIndex, x: u32, y: u32) -> Option<bool> {
    m.available(t)?;
    let map = crate::map_func::Map::from_log_x(x.trailing_zeros());
    let x1 = map.tile_x(t);
    let y1 = map.tile_y(t);
    let x2 = (x1 + 1).min(map.max_x());
    let y2 = (y1 + 1).min(crate::map_func::Map::max_y(y));
    let height = tile_height(m, t)?;
    for corner in [
        map.tile_xy(x2, y1),
        map.tile_xy(x1, y2),
        map.tile_xy(x2, y2),
    ] {
        if tile_height(m, corner)? != height {
            return Some(false);
        }
    }
    Some(true)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_z(m: &TileTypeAccess, t: TileIndex, x: u32, y: u32) -> Option<i32> {
    let [north, west, east, south] = corners(m, t, x, y)?;
    Some(
        north // N corner
            .min(west) // W corner
            .min(east) // E corner
            .min(south), // S corner
    )
}

/// `IsTileOwner`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn is_tile_owner(
    s: &crate::services::Services,
    tile: TileIndex,
    owner: crate::company_type::Owner,
) -> bool {
    crate::map_func::World::new(s).owner(tile) == owner
}

/// `TileHash`, preserving the source unsigned subtraction.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn tile_hash(x: u32, y: u32) -> u32 {
    ((x >> 4) ^ (x >> 6) ^ (y >> 4)).wrapping_sub(y >> 6)
}

/// `TileHash2Bit` calls the canonical hash before extracting its low bits.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn tile_hash_2bit(x: u32, y: u32) -> u32 {
    crate::bitmath_func::gb(tile_hash(x, y), 0, 2)
}

/// `GetTilePixelSlope`, preserving unsigned `TILE_HEIGHT` multiplication.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_pixel_slope(
    map: &TileTypeAccess,
    tile: TileIndex,
    x: u32,
    y: u32,
) -> Option<(Slope, i32)> {
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
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_max_z(map: &TileTypeAccess, tile: TileIndex, x: u32, y: u32) -> Option<i32> {
    let [north, west, east, south] = corners(map, tile, x, y)?;
    Some(
        north // N corner
            .max(west) // W corner
            .max(east) // E corner
            .max(south), // S corner
    )
}
/// `GetTileMaxPixelZ`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_max_pixel_z(
    map: &TileTypeAccess,
    tile: TileIndex,
    x: u32,
    y: u32,
) -> Option<i32> {
    Some(
        get_tile_max_z(map, tile, x, y)?
            .cast_unsigned()
            .wrapping_mul(crate::tile_type::TILE_HEIGHT)
            .cast_signed(),
    )
}
/// `TileHeightOutsideMap`, including signed clamping after the caller's conversion.
#[allow(clippy::inline_always)]
#[inline(always)]
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
        TileIndex(
            map.tile_xy(
                crate::math::clamp(x, 0, map.max_x().cast_signed(), |ok| {
                    let _ = tiles.check(ok);
                })
                .cast_unsigned(),
                crate::math::clamp(
                    y,
                    0,
                    crate::map_func::Map::max_y(height).cast_signed(),
                    |ok| {
                        let _ = tiles.check(ok);
                    },
                )
                .cast_unsigned(),
            )
            .0,
        ),
    )
}
/// `GetTilePixelSlopeOutsideMap`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_tile_pixel_slope_outside_map(
    map: &TileTypeAccess,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Option<(Slope, i32)> {
    let north = tile_height_outside_map(map, x, y, width, height)?; // N corner.
    let west = tile_height_outside_map(map, x.wrapping_add(1), y, width, height)?; // W corner.
    let east = tile_height_outside_map(map, x, y.wrapping_add(1), width, height)?; // E corner.
    let south = tile_height_outside_map(map, x.wrapping_add(1), y.wrapping_add(1), width, height)?; // S corner.
    let (slope, height_z) = get_tile_slope_given_height([north, west, east, south]);
    Some((
        slope,
        height_z
            .cast_unsigned()
            .wrapping_mul(crate::tile_type::TILE_HEIGHT)
            .cast_signed(),
    ))
}

impl TileTypeAccess {
    /// Run a header helper whose `None` is a failed precondition. The failure
    /// retries with each source assertion dispatched to the native handler.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn asserted<T>(
        &self,
        shared: &crate::services::Services,
        read: impl Fn(&TileTypeAccess) -> Option<T>,
    ) -> T {
        read(self).unwrap_or_else(|| {
            self.with_native_assert(crate::movement::table(shared).native_assert, read)
                .expect("live map scalar")
        })
    }
}

#[allow(clippy::cast_possible_truncation)] // GB returns the original field width.
impl crate::map_func::World<'_> {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn owner(&self, t: TileIndex) -> crate::company_type::Owner {
        self.assert_with(|| self.is_valid_tile(t));
        self.assert_with(|| self.kind(t) != TileType::MP_HOUSE);
        self.assert_with(|| self.kind(t) != TileType::MP_INDUSTRY);
        crate::company_type::CompanyID(crate::bitmath_func::gb(
            u32::from(Self::byte(self.types.m1(t))),
            0,
            5,
        ) as u8)
    }
    /// `Map::SizeX` and `Map::SizeY` of the acquired allocation.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn size(&self) -> (u32, u32) {
        (self.dimensions.size_x(), self.size_y())
    }
    /// Run a header helper with the entry's native handler at each source assertion.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn asserted<T>(&self, read: impl Fn(&TileTypeAccess) -> Option<T>) -> T {
        self.types.asserted(self.shared, read)
    }
    /// `GetTilePixelSlope`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn tile_pixel_slope(&self, t: TileIndex) -> (Slope, i32) {
        let (x, y) = self.size();
        self.asserted(|m| get_tile_pixel_slope(m, t, x, y))
    }
    /// `GetTilePixelSlopeOutsideMap`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn tile_pixel_slope_outside_map(&self, x: i32, y: i32) -> (Slope, i32) {
        let (width, height) = self.size();
        self.asserted(|m| get_tile_pixel_slope_outside_map(m, x, y, width, height))
    }
    /// `GetTileSlopeZ`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn tile_slope_z(&self, t: TileIndex) -> (Slope, i32) {
        let (_, size_y) = self.size();
        if t.0 < self.types.count() {
            // Clamped corners of an in-range tile are in range: no `TileHeight` assertion fails.
            let map = self.dimensions;
            let (x1, y1) = (map.tile_x(t), map.tile_y(t));
            let x2 = (x1 + 1).min(map.max_x());
            let y2 = (y1 + 1).min(crate::map_func::Map::max_y(size_y));
            let height = |c: TileIndex| {
                self.types
                    .require_range(c)
                    .map_or(0, |f| i32::from(f.height()))
            };
            return get_tile_slope_given_height([
                height(t),                   // Height of the North corner.
                height(map.tile_xy(x2, y1)), // Height of the West corner.
                height(map.tile_xy(x1, y2)), // Height of the East corner.
                height(map.tile_xy(x2, y2)), // Height of the South corner.
            ]);
        }
        self.tile_slope_z_outside(t)
    }
    // An out-of-range tile keeps the native assertion policy of each `TileHeight`.
    #[cold]
    #[inline(never)]
    fn tile_slope_z_outside(&self, t: TileIndex) -> (Slope, i32) {
        let (x, y) = self.size();
        self.asserted(|m| get_tile_slope_z(m, t, x, y))
    }
    /// `GetTileSlope`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn tile_slope(&self, t: TileIndex) -> Slope {
        self.tile_slope_z(t).0
    }
    /// `IsValidTile`: checks if a tile is valid, i.e. inside the map and not a void tile.
    pub(crate) fn is_valid_tile(&self, t: TileIndex) -> bool {
        self.asserted(|m| is_valid_tile(m, t, m.count()))
    }
    /// `IsTileFlat` without the height output.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn tile_flat(&self, t: TileIndex) -> bool {
        let (x, y) = self.size();
        self.asserted(|m| is_tile_flat(m, t, x, y))
    }
    /// `GetTileMaxPixelZ`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn tile_max_pixel_z(&self, t: TileIndex) -> i32 {
        let (x, y) = self.size();
        self.asserted(|m| get_tile_max_pixel_z(m, t, x, y))
    }
}

/// `TileHeight`; a failed `tile < Map::Size()` calls the native assertion.
#[allow(clippy::cast_sign_loss)] // `tile_height` reads a byte, so its i32 is nonnegative.
pub(crate) fn tile_height_in(w: &crate::map_func::World<'_>, t: TileIndex) -> u32 {
    tile_height(&w.types, t).map_or_else(
        || {
            w.check(false);
            0
        },
        |height| height as u32,
    )
}
/// `IsInnerTile`; `freeform_edges` reads `construction.freeform_edges`, only in the
/// final operand and only for a tile on the upper edges, as the original does.
pub(crate) fn is_inner_tile(
    w: &crate::map_func::World<'_>,
    t: TileIndex,
    freeform_edges: impl FnOnce() -> bool,
) -> bool {
    w.check(t.0 < w.types.count());
    let map = w.dimensions;
    let x = map.tile_x(t);
    let y = map.tile_y(t);
    x < map.max_x()
        && y < crate::map_func::Map::max_y(w.size_y())
        && ((x > 0 && y > 0) || !freeform_edges())
}
/// `SetTileHeight`.
#[allow(clippy::cast_possible_truncation)] // `tile.height() = height` stores the low byte.
pub(crate) fn set_tile_height(w: &crate::map_func::World<'_>, t: TileIndex, height: u32) {
    w.check(t.0 < w.types.count());
    w.check(height <= crate::tile_type::MAX_TILE_HEIGHT);
    let _ = w.types.set_height(t, height as u8);
}
/// `SetTileType`; the inner-tile assertion reads `freeform_edges` only in assertion builds.
#[allow(clippy::cast_possible_truncation)] // `SB` assigns the promoted result to a byte.
pub(crate) fn set_tile_type(
    w: &crate::map_func::World<'_>,
    t: TileIndex,
    kind: TileType,
    freeform_edges: impl FnOnce() -> bool,
) {
    w.check(t.0 < w.types.count());
    /* VOID tiles (and no others) are exactly allowed at the lower left and right
     * edges of the map. If _settings_game.construction.freeform_edges is true,
     * the upper edges of the map are also VOID tiles. */
    if w.types.assertions() {
        w.check(is_inner_tile(w, t, freeform_edges) == (kind != TileType::MP_VOID));
    }
    let mut value = u32::from(crate::map_func::World::byte(w.types.kind(t)));
    crate::bitmath_func::sb(&mut value, 4, 4, u32::from(kind.0));
    let _ = w.types.set_kind(t, value as u8);
}

#[cfg(test)]
mod tests {
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation, World};
    use std::cell::Cell;

    thread_local! {
        static ASSERTS: Cell<u32> = const { Cell::new(0) };
        static MOVEMENT: crate::movement::MovementServices = {
            let mut movement = crate::train_reservation::tests::movement_table();
            movement.native_assert = native_assert;
            movement
        };
    }
    /// A native assertion that returns, counting each failed condition.
    extern "C" fn native_assert(ok: bool) {
        if !ok {
            ASSERTS.set(ASSERTS.get() + 1);
        }
    }
    extern "C" fn no_random(_: *mut std::ffi::c_void) -> u32 {
        std::process::abort()
    }
    extern "C" fn no_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        std::process::abort()
    }
    extern "C" fn no_callback(_: u32) -> u8 {
        std::process::abort()
    }

    /// `Map::Size()` is 4096 of 64 by 64; tile 4096 lies past it but inside
    /// these backing arrays, where the assertion-disabled original reads it.
    fn tiles() -> (Vec<BaseTileLayout>, Vec<ExtendedTileLayout>) {
        let mut base: Vec<BaseTileLayout> = (0..8192)
            .map(|_| BaseTileLayout {
                kind: crate::tile_type::MP_CLEAR << 4,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect();
        let mut extended: Vec<ExtendedTileLayout> = (0..8192)
            .map(|_| ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        // A clear tile whose road fields hold normal road, one-way bits 2, road
        // bits 11, tram bits 14, road type 45, tram type 37 and roadworks.
        base[5].m5 = 0b0010_1011;
        base[5].m3 = 0x0e;
        base[5].m4 = 0b1010_1101;
        extended[5].m8 = (37 << 6) | 5;
        extended[5].m6 = 6 << 3;
        for tile in [6, 4096] {
            base[tile].m5 = 0b0110_1011;
            base[tile].m3 = 0x0e;
            base[tile].m4 = 0b1010_1101;
            extended[tile].m8 = (37 << 6) | 5;
            extended[tile].m6 = 6 << 3;
        }
        for (tile, height) in [(4096, 3), (4097, 4), (4032, 4), (4033, 5)] {
            base[tile].height = height;
        }
        (base, extended)
    }

    type Read = for<'a> fn(&World<'a>, u32) -> [i32; 2];
    type Case = (&'static str, Read, [[i32; 2]; 2], [u32; 3]);

    // Clear tiles 5, 6 and 4096 carry normal/crossing/crossing road bits.
    // Tile 4096 also fails GetTileType's range assertion, where it is called.
    const ROAD_CASES: &[Case] = &[
        (
            "GetRoadTileType",
            |w, t| [w.road_tile_type(t).0.into(), 0],
            [[0, 0], [1, 0]],
            [1, 1, 2],
        ),
        (
            "IsNormalRoad",
            |w, t| [w.is_normal_road(t).into(), 0],
            [[1, 0], [0, 0]],
            [1, 1, 2],
        ),
        (
            "IsNormalRoadTile",
            |w, t| [w.is_normal_road_tile(t).into(), 0],
            [[0, 0]; 2],
            [0, 0, 1],
        ),
        (
            "GetDisallowedRoadDirections",
            |w, t| [w.disallowed_road_directions(t).into(), 0],
            [[2, 0]; 2],
            [1, 2, 3],
        ),
        (
            "GetRoadBits road",
            |w, t| [w.road_bits(t, false).into(), 0],
            [[11, 0]; 2],
            [1, 2, 3],
        ),
        (
            "GetRoadBits tram",
            |w, t| [w.road_bits(t, true).into(), 0],
            [[14, 0]; 2],
            [1, 2, 3],
        ),
        (
            "GetRoadTypeRoad",
            |w, t| [w.road_type(t, false).into(), 0],
            [[45, 0]; 2],
            [1, 1, 2],
        ),
        (
            "GetRoadTypeTram",
            |w, t| [w.road_type(t, true).into(), 0],
            [[37, 0]; 2],
            [1, 1, 2],
        ),
        (
            "HasRoadWorks",
            |w, t| [w.has_road_works(t).into(), 0],
            [[1, 0]; 2],
            [0; 3],
        ),
    ];

    /// Returning native assertions preserve nested assertion sites and raw results.
    #[test]
    #[allow(unsafe_code)] // The raw reader over this test's arrays only.
    fn failed_world_preconditions_assert_then_read_raw_bits() {
        use crate::slope_func::{SLOPE_E, SLOPE_S, SLOPE_STEEP, SLOPE_W};
        let (base, extended) = tiles();
        let height_cases: [(&str, Read, [i32; 2], u32); 4] = [
            (
                "GetTileSlopeZ",
                |w, t| {
                    let (slope, z) = w.tile_slope_z(crate::tile_type::TileIndex(t));
                    [slope.0.into(), z]
                },
                [(SLOPE_W | SLOPE_S | SLOPE_E | SLOPE_STEEP).into(), 3],
                2,
            ),
            (
                "GetTilePixelSlope",
                |w, t| {
                    let (slope, z) = w.tile_pixel_slope(crate::tile_type::TileIndex(t));
                    [slope.0.into(), z]
                },
                [(SLOPE_W | SLOPE_S | SLOPE_E | SLOPE_STEEP).into(), 24],
                2,
            ),
            (
                "GetTileMaxPixelZ",
                |w, t| [w.tile_max_pixel_z(crate::tile_type::TileIndex(t)), 0],
                [40, 0],
                2,
            ),
            (
                "GetTilePixelSlopeOutsideMap",
                |w, _| {
                    let (slope, z) = w.tile_pixel_slope_outside_map(0, 64);
                    [slope.0.into(), z]
                },
                [(SLOPE_W | SLOPE_S).into(), 32],
                0,
            ),
        ];
        let shared = crate::services::Services {
            movement: MOVEMENT.with(std::ptr::from_ref),
            ..crate::services::fixture(std::ptr::null_mut(), no_random, no_industry)
        };
        for assertions in [true, false] {
            let road_cases = ROAD_CASES
                .iter()
                .flat_map(|&(name, read, expected, asserts)| {
                    [5, 6, 4096].into_iter().enumerate().map(move |(i, tile)| {
                        (name, read, tile, expected[usize::from(i != 0)], asserts[i])
                    })
                });
            let height_cases = height_cases
                .iter()
                .map(|&(name, read, expected, asserts)| (name, read, 4096, expected, asserts));
            for (name, read, tile, expected, asserts) in road_cases.chain(height_cases) {
                let allocation = TileTypeAllocation {
                    base: base.as_ptr(),
                    extended: extended.as_ptr(),
                    count: 4096,
                    assertions,
                };
                // SAFETY: Both arrays hold 8192 initialized tiles that outlive the reader.
                let types =
                    unsafe { crate::map_func::TileTypeAccess::raw(allocation, no_callback) };
                let world = World {
                    types,
                    shared: &shared,
                    dimensions: crate::map_func::Map::from_log_x(6),
                };
                crate::map_func::fixture::set(allocation, 6);
                for world in [world, World::new(&shared)] {
                    ASSERTS.set(0);
                    assert_eq!(
                        read(&world, tile),
                        expected,
                        "{name}, tile {tile}, assertions {assertions}"
                    );
                    let asserts = if assertions { asserts } else { 0 };
                    assert_eq!(
                        ASSERTS.get(),
                        asserts,
                        "{name}, tile {tile}, assertions {assertions}"
                    );
                }
            }
        }
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 0,
                assertions: true,
            },
            6,
        );
    }

    /// `IsInnerTile`, `SetTileHeight` and `SetTileType` assert `tile < Map::Size()`
    /// themselves: tile 4096 of the 64 by 64 map fails that range assertion once each,
    /// although these backing arrays extend past it, and a returning handler resumes.
    #[test]
    #[allow(unsafe_code)] // The raw reader over this test's arrays only.
    fn setters_assert_the_map_size_range() {
        let (base, extended) = tiles();
        let shared = crate::services::Services {
            movement: MOVEMENT.with(std::ptr::from_ref),
            ..crate::services::fixture(std::ptr::null_mut(), no_random, no_industry)
        };
        let allocation = TileTypeAllocation {
            base: base.as_ptr(),
            extended: extended.as_ptr(),
            count: 4096,
            assertions: true,
        };
        crate::map_func::fixture::set(allocation, 6);
        let world = World::new(&shared);
        let outside = crate::tile_type::TileIndex(4096);
        ASSERTS.set(0);
        let _ = super::is_inner_tile(&world, outside, || false);
        assert_eq!(ASSERTS.get(), 1, "IsInnerTile");
        ASSERTS.set(0);
        super::set_tile_height(&world, outside, 2);
        assert_eq!(ASSERTS.get(), 1, "SetTileHeight");
        ASSERTS.set(0);
        // The range assertion, then the inner-tile assertion's own range assertion; a void
        // type is not inner, so the type assertion holds.
        super::set_tile_type(&world, outside, crate::tile_type::TileType::MP_VOID, || {
            false
        });
        assert_eq!(ASSERTS.get(), 2, "SetTileType");
        // The setting is read only for an upper-edge tile inside the lower borders.
        let reads = std::cell::Cell::new(0);
        let read = || {
            reads.set(reads.get() + 1);
            true
        };
        for (tile, inner, expected) in [
            (65, true, 0),
            (63, false, 0),
            (4032, false, 0),
            (1, false, 1),
        ] {
            reads.set(0);
            assert_eq!(
                super::is_inner_tile(&world, crate::tile_type::TileIndex(tile), read),
                inner
            );
            assert_eq!(
                reads.get(),
                expected,
                "IsInnerTile setting reads, tile {tile}"
            );
        }
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 0,
                assertions: true,
            },
            6,
        );
    }
}
