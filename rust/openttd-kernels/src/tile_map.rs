/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `tile_map.h` for the audited map reader.
//! `None` denotes an unavailable raw backend or a failed native precondition.
use crate::map_func::TileTypeAccess;

#[inline]
pub(crate) fn is_tile_type(s: &crate::services::Services, tile: u32, kind: u8) -> bool {
    (s.tile_type)(tile) == kind
}

#[inline]
pub(crate) fn get_tile_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_range(t)?;
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
    m.require_range(t)?;
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
/// `IsTileFlat` without the height output.
#[inline]
pub(crate) fn is_tile_flat(m: &TileTypeAccess, t: u32, x: u32, y: u32) -> Option<bool> {
    let [height, x2_y1, x1_y2, x2_y2] = corners(m, t, x, y)?;
    Some(x2_y1 == height && x1_y2 == height && x2_y2 == height)
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
    pub(crate) fn size(&self) -> (u32, u32) {
        (
            self.dimensions.size_x(),
            self.types.count() >> self.dimensions.log_x,
        )
    }
    /// Run a header helper whose `None` is a failed precondition. The failure
    /// retries with each source assertion dispatched to the native handler.
    pub(crate) fn asserted<T>(&self, read: impl Fn(&TileTypeAccess) -> Option<T>) -> T {
        read(&self.types).unwrap_or_else(|| {
            self.types
                .with_native_assert(crate::movement::table(self.shared).native_assert, read)
                .expect("live map scalar")
        })
    }
    /// `GetTilePixelSlope`.
    pub(crate) fn tile_pixel_slope(&self, t: u32) -> (u8, i32) {
        let (x, y) = self.size();
        self.asserted(|m| get_tile_pixel_slope(m, t, x, y))
    }
    /// `GetTilePixelSlopeOutsideMap`.
    pub(crate) fn tile_pixel_slope_outside_map(&self, x: i32, y: i32) -> (u8, i32) {
        let (width, height) = self.size();
        self.asserted(|m| get_tile_pixel_slope_outside_map(m, x, y, width, height))
    }
    /// `GetTileMaxPixelZ`.
    pub(crate) fn tile_max_pixel_z(&self, t: u32) -> i32 {
        let (x, y) = self.size();
        self.asserted(|m| get_tile_max_pixel_z(m, t, x, y))
    }
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
    extern "C" fn movement_services() -> *const crate::movement::MovementServices {
        MOVEMENT.with(std::ptr::from_ref)
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
        let height_cases: [(&str, Read, [i32; 2], u32); 3] = [
            (
                "GetTilePixelSlope",
                |w, t| {
                    let (slope, z) = w.tile_pixel_slope(t);
                    [slope.into(), z]
                },
                [(SLOPE_W | SLOPE_S | SLOPE_E | SLOPE_STEEP).into(), 24],
                2,
            ),
            (
                "GetTileMaxPixelZ",
                |w, t| [w.tile_max_pixel_z(t), 0],
                [40, 0],
                2,
            ),
            (
                "GetTilePixelSlopeOutsideMap",
                |w, _| {
                    let (slope, z) = w.tile_pixel_slope_outside_map(0, 64);
                    [slope.into(), z]
                },
                [(SLOPE_W | SLOPE_S).into(), 32],
                0,
            ),
        ];
        let shared = crate::services::Services {
            movement_services,
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
                // SAFETY: Both arrays hold 8192 initialized tiles that outlive the reader.
                let types = unsafe {
                    crate::map_func::TileTypeAccess::raw(
                        TileTypeAllocation {
                            base: base.as_ptr(),
                            extended: extended.as_ptr(),
                            count: 4096,
                            assertions,
                        },
                        no_callback,
                    )
                };
                let world = World {
                    types,
                    shared: &shared,
                    dimensions: crate::map_func::Map::from_log_x(6),
                };
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
}
