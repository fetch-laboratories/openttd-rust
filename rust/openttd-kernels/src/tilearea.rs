/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Orthogonal and diagonal tile areas from `tilearea_type.h` and `tilearea.cpp`.
//!
//! Native `OrthogonalTileArea` facades pass their area in place; each export
//! borrows it only for its call. The orthogonal iterator's advance stays inline
//! in C++ loops. Diagonal areas and [`TileIterator`] serve Rust loops only.
#![allow(unsafe_code)]
use crate::map_func::Map;
/// Native `OrthogonalTileArea` scalar fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TileArea {
    /// North-west corner.
    pub tile: u32,
    /// Width in tiles.
    pub w: u16,
    /// Height in tiles.
    pub h: u16,
}
impl TileArea {
    /// `OrthogonalTileArea(start, end)`, spanning both corners in either order.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn between(map: Map, a: u32, b: u32) -> Self {
        let x = map.tile_x(a).min(map.tile_x(b));
        let y = map.tile_y(a).min(map.tile_y(b));
        Self {
            tile: (y << map.log_x) + x,
            w: (map.tile_x(a).max(map.tile_x(b)) - x + 1) as u16,
            h: (map.tile_y(a).max(map.tile_y(b)) - y + 1) as u16,
        }
    }
    /// Original `OrthogonalTileArea(start, end)`, asserting both corners are on the map.
    #[must_use]
    pub fn from_corners(map: Map, size: u32, start: u32, end: u32, check: impl Fn(bool)) -> Self {
        check(start < size);
        check(end < size);
        Self::between(map, start, end)
    }
    /// Original `Add`: an invalid base tile becomes the single added tile.
    #[allow(clippy::cast_possible_truncation)]
    pub fn add(&mut self, to_add: u32, map: Map) {
        if self.tile == crate::tile_type::INVALID_TILE {
            *self = Self {
                tile: to_add,
                w: 1,
                h: 1,
            };
            return;
        }
        let sx = map.tile_x(self.tile);
        let sy = map.tile_y(self.tile);
        let ex = sx.wrapping_add(u32::from(self.w)).wrapping_sub(1);
        let ey = sy.wrapping_add(u32::from(self.h)).wrapping_sub(1);
        let ax = map.tile_x(to_add);
        let ay = map.tile_y(to_add);
        let (sx, sy, ex, ey) = (ax.min(sx), ay.min(sy), ax.max(ex), ay.max(ey));
        self.tile = map.tile_xy(sx, sy);
        self.w = ex.wrapping_sub(sx).wrapping_add(1) as u16;
        self.h = ey.wrapping_sub(sy).wrapping_add(1) as u16;
    }
    /// Original `Intersects`, including its empty-width return before the assertion.
    #[must_use]
    pub fn intersects(self, other: Self, map: Map, check: impl FnOnce(bool)) -> bool {
        if other.w == 0 || self.w == 0 {
            return false;
        }
        check(other.h != 0 && self.h != 0);
        let left1 = map.tile_x(self.tile);
        let top1 = map.tile_y(self.tile);
        let right1 = left1.wrapping_add(u32::from(self.w)).wrapping_sub(1);
        let bottom1 = top1.wrapping_add(u32::from(self.h)).wrapping_sub(1);
        let left2 = map.tile_x(other.tile);
        let top2 = map.tile_y(other.tile);
        let right2 = left2.wrapping_add(u32::from(other.w)).wrapping_sub(1);
        let bottom2 = top2.wrapping_add(u32::from(other.h)).wrapping_sub(1);
        !(left2 > right1 || right2 < left1 || top2 > bottom1 || bottom2 < top1)
    }
    /// Original `Contains`, including its empty-width return before the assertion.
    #[must_use]
    pub fn contains(self, tile: u32, map: Map, check: impl FnOnce(bool)) -> bool {
        if self.w == 0 {
            return false;
        }
        check(self.w != 0 && self.h != 0);
        // `IsInsideBS` in unsigned arithmetic.
        map.tile_x(tile).wrapping_sub(map.tile_x(self.tile)) < u32::from(self.w)
            && map.tile_y(tile).wrapping_sub(map.tile_y(self.tile)) < u32::from(self.h)
    }
    /// Original `Expand`, including its clipping to the full map dimensions.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn expand(&mut self, radius: i32, map: Map, size_y: u32) {
        let x = map.tile_x(self.tile) as i32;
        let y = map.tile_y(self.tile) as i32;
        let sx = (x - radius).max(0);
        let sy = (y - radius).max(0);
        let ex = (x + i32::from(self.w) + radius).min(map.size_x() as i32);
        let ey = (y + i32::from(self.h) + radius).min(size_y as i32);
        self.tile = map.tile_xy(sx as u32, sy as u32);
        self.w = (ex - sx) as u16;
        self.h = (ey - sy) as u16;
    }
    /// Original `ClampToMap`; the unsigned remaining extent converts to `int` before the minimum.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn clamp_to_map(&mut self, map: Map, size: u32, check: impl FnOnce(bool)) {
        check(self.tile < size);
        let size_y = size >> map.log_x;
        let w = i32::from(self.w).min(map.size_x().wrapping_sub(map.tile_x(self.tile)) as i32);
        let h = i32::from(self.h).min(size_y.wrapping_sub(map.tile_y(self.tile)) as i32);
        self.w = w as u16;
        self.h = h as u16;
    }
    /// Original row-major orthogonal tile iterator.
    pub fn tiles(self, map: Map) -> impl Iterator<Item = u32> + use<> {
        let tile = self.tile;
        let width = u32::from(self.w);
        (0..u32::from(self.h))
            .flat_map(move |y| {
                (0..width).map(move |x| {
                    tile.wrapping_add(y.wrapping_mul(map.size_x()))
                        .wrapping_add(x)
                })
            })
            .take_while(|&tile| tile != crate::tile_type::INVALID_TILE)
    }
}

/// `OrthogonalTileIterator`, which copies the area geometry at construction.
#[derive(Clone, Copy)]
pub struct OrthogonalTileIterator {
    /// Current tile, `INVALID_TILE` at the end.
    pub tile: u32,
    w: i32,
    x: i32,
    y: i32,
}
impl OrthogonalTileIterator {
    /// Construct over `ta`; an empty extent starts at `INVALID_TILE`.
    #[must_use]
    pub fn new(ta: TileArea) -> Self {
        Self {
            tile: if ta.w == 0 || ta.h == 0 {
                crate::tile_type::INVALID_TILE
            } else {
                ta.tile
            },
            w: i32::from(ta.w),
            x: i32::from(ta.w),
            y: i32::from(ta.h),
        }
    }
    /// `operator++`, asserting that the end has not been reached.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn advance(&mut self, map: Map, check: impl FnOnce(bool)) {
        check(self.tile != crate::tile_type::INVALID_TILE);
        self.x -= 1;
        if self.x > 0 {
            self.tile = self.tile.wrapping_add(1);
        } else {
            self.y -= 1;
            if self.y > 0 {
                self.x = self.w;
                self.tile = self.tile.wrapping_add(map.diff(1 - self.w, 1));
            } else {
                self.tile = crate::tile_type::INVALID_TILE;
            }
        }
    }
}

/// `DiagonalTileArea`: extents are one past the end, signed by direction.
#[derive(Clone, Copy)]
pub struct DiagonalTileArea {
    /// Base tile of the area.
    pub tile: u32,
    /// Extent along `TileY + TileX`.
    pub a: i16,
    /// Extent along `TileY - TileX`.
    pub b: i16,
}
impl DiagonalTileArea {
    /// Original `DiagonalTileArea(start, end)`. Unsigned coordinate sums narrow to
    /// `int16_t` before the one-past-end step, which also narrows.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_corners(map: Map, size: u32, start: u32, end: u32, check: impl Fn(bool)) -> Self {
        check(start < size);
        check(end < size);
        let (sx, sy, ex, ey) = (
            map.tile_x(start),
            map.tile_y(start),
            map.tile_x(end),
            map.tile_y(end),
        );
        let a = ey.wrapping_add(ex).wrapping_sub(sy).wrapping_sub(sx) as i16;
        let b = ey.wrapping_sub(ex).wrapping_sub(sy).wrapping_add(sx) as i16;
        let past = |e: i16| {
            if e > 0 {
                e.wrapping_add(1)
            } else {
                e.wrapping_sub(1)
            }
        };
        Self {
            tile: start,
            a: past(a),
            b: past(b),
        }
    }
    /// Original `Contains`; unsigned coordinate sums convert to `int`.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn contains(self, tile: u32, map: Map) -> bool {
        let rotated = |t: u32| {
            (
                map.tile_y(t).wrapping_add(map.tile_x(t)) as i32,
                map.tile_y(t).wrapping_sub(map.tile_x(t)) as i32,
            )
        };
        let (a, b) = rotated(tile);
        let (mut start_a, mut start_b) = rotated(self.tile);
        let mut end_a = start_a.wrapping_add(i32::from(self.a));
        let mut end_b = start_b.wrapping_add(i32::from(self.b));
        // Swap if necessary, preserving the one-past-end semantics.
        if start_a > end_a {
            (start_a, end_a) = (end_a.wrapping_add(1), start_a.wrapping_add(1));
        }
        if start_b > end_b {
            (start_b, end_b) = (end_b.wrapping_add(1), start_b.wrapping_add(1));
        }
        a >= start_a && a < end_a && b >= start_b && b < end_b
    }
}

/// `DiagonalTileIterator` fields after the inherited current tile.
#[derive(Clone, Copy)]
pub struct DiagonalTileCursor {
    /// Base tile x coordinate.
    pub base_x: u32,
    /// Base tile y coordinate.
    pub base_y: u32,
    /// Current rotated x coordinate.
    pub a_cur: i32,
    /// Current rotated y coordinate.
    pub b_cur: i32,
    /// Rotated x coordinate of the end.
    pub a_max: i32,
    /// Rotated y coordinate of the end.
    pub b_max: i32,
}
impl DiagonalTileCursor {
    /// Original `operator++` from `tile`, clipping at the map borders. Its loop
    /// continues while `tile > Map::Size()`, so only `INVALID_TILE` is skipped.
    #[allow(clippy::cast_sign_loss)]
    pub fn advance(&mut self, tile: u32, map: Map, size: u32, check: impl FnOnce(bool)) -> u32 {
        check(tile != crate::tile_type::INVALID_TILE);
        let size_y = size >> map.log_x;
        let mut tile;
        loop {
            if self.a_max == 1 || self.a_max == -1 {
                // Every second column has zero length; skip them completely.
                self.a_cur = 0;
                self.b_cur = if self.b_max > 0 {
                    self.b_cur.wrapping_add(2).min(self.b_max)
                } else {
                    self.b_cur.wrapping_sub(2).max(self.b_max)
                };
            } else {
                let new_line = if self.a_max > 0 {
                    self.a_cur = self.a_cur.wrapping_add(2);
                    self.a_cur >= self.a_max
                } else {
                    self.a_cur = self.a_cur.wrapping_sub(2);
                    self.a_cur <= self.a_max
                };
                if new_line {
                    // Every second line starts one tile further in the direction of a_max.
                    self.a_cur = if self.a_cur.wrapping_abs() % 2 != 0 {
                        0
                    } else if self.a_max > 0 {
                        1
                    } else {
                        -1
                    };
                    self.b_cur = if self.b_max > 0 {
                        self.b_cur.wrapping_add(1)
                    } else {
                        self.b_cur.wrapping_sub(1)
                    };
                }
            }
            let x = self
                .base_x
                .wrapping_add((self.a_cur.wrapping_sub(self.b_cur) / 2) as u32);
            let y = self
                .base_y
                .wrapping_add((self.b_cur.wrapping_add(self.a_cur) / 2) as u32);
            tile = if x >= map.size_x() || y >= size_y {
                crate::tile_type::INVALID_TILE
            } else {
                map.tile_xy(x, y)
            };
            if !(tile > size && self.b_max != self.b_cur) {
                break;
            }
        }
        if self.b_max == self.b_cur {
            crate::tile_type::INVALID_TILE
        } else {
            tile
        }
    }
}

impl DiagonalTileCursor {
    /// `DiagonalTileIterator(const DiagonalTileArea &)`, starting at the base tile.
    #[must_use]
    pub fn new(area: DiagonalTileArea, map: Map) -> Self {
        Self {
            base_x: map.tile_x(area.tile),
            base_y: map.tile_y(area.tile),
            a_cur: 0,
            b_cur: 0,
            a_max: i32::from(area.a),
            b_max: i32::from(area.b),
        }
    }
}

/// `TileIterator::Create(corner1, corner2, diagonal)` for Rust area loops.
#[derive(Clone, Copy)]
pub enum TileIterator {
    /// `OrthogonalTileIterator(corner1, corner2)`.
    Orthogonal(OrthogonalTileIterator),
    /// `DiagonalTileIterator(corner1, corner2)` and its current tile.
    Diagonal(u32, DiagonalTileCursor),
}
impl TileIterator {
    /// Build the iterator; both corner constructors assert their map range.
    #[must_use]
    pub fn create(
        map: Map,
        size: u32,
        corner1: u32,
        corner2: u32,
        diagonal: bool,
        check: impl Fn(bool),
    ) -> Self {
        if diagonal {
            let area = DiagonalTileArea::from_corners(map, size, corner1, corner2, check);
            Self::Diagonal(area.tile, DiagonalTileCursor::new(area, map))
        } else {
            Self::Orthogonal(OrthogonalTileIterator::new(TileArea::from_corners(
                map, size, corner1, corner2, check,
            )))
        }
    }
    /// The current tile, `INVALID_TILE` at the end.
    #[must_use]
    pub fn tile(&self) -> u32 {
        match self {
            Self::Orthogonal(iter) => iter.tile,
            Self::Diagonal(tile, _) => *tile,
        }
    }
    /// `operator++`, with each iterator's end-of-range assertion.
    pub fn advance(&mut self, map: Map, size: u32, check: impl FnOnce(bool)) {
        match self {
            Self::Orthogonal(iter) => iter.advance(map, check),
            Self::Diagonal(tile, cursor) => *tile = cursor.advance(*tile, map, size, check),
        }
    }
}

/// `DiagonalTileArea(start, end).Contains(tile)` or its `OrthogonalTileArea` form.
#[must_use]
pub fn corners_contain(
    map: Map,
    size: u32,
    start: u32,
    end: u32,
    diagonal: bool,
    tile: u32,
    check: impl Fn(bool),
) -> bool {
    if diagonal {
        DiagonalTileArea::from_corners(map, size, start, end, check).contains(tile, map)
    } else {
        TileArea::from_corners(map, size, start, end, &check).contains(tile, map, &check)
    }
}

/// Map dimensions and the native assertion for a facade entry.
/// # Safety
/// `shared` is the live shared table; the map is not being reset.
unsafe fn entry<'a>(shared: *const crate::services::Services) -> crate::map_func::World<'a> {
    crate::map_func::World::new(unsafe { &*shared })
}

/// Native `OrthogonalTileArea(start, end)`.
/// # Safety
/// `out` is a writable native area; `shared` is the live shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tile_area_corners(
    out: *mut TileArea,
    start: u32,
    end: u32,
    shared: *const crate::services::Services,
) {
    let w = unsafe { entry(shared) };
    let area = TileArea::from_corners(w.dimensions, w.types.count(), start, end, |ok| {
        w.check(ok);
    });
    unsafe { out.write(area) };
}
/// Native `OrthogonalTileArea::Add`.
/// # Safety
/// `area` is a live native area with no other borrow; `shared` is the live shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tile_area_add(
    area: *mut TileArea,
    to_add: u32,
    shared: *const crate::services::Services,
) {
    let w = unsafe { entry(shared) };
    unsafe { (*area).add(to_add, w.dimensions) };
}
/// Native `OrthogonalTileArea::Expand`.
/// # Safety
/// `area` is a live native area with no other borrow; `shared` is the live shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tile_area_expand(
    area: *mut TileArea,
    radius: i32,
    shared: *const crate::services::Services,
) {
    let w = unsafe { entry(shared) };
    unsafe { (*area).expand(radius, w.dimensions, w.size_y()) };
}
/// Native `OrthogonalTileArea::ClampToMap`.
/// # Safety
/// `area` is a live native area with no other borrow; `shared` is the live shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tile_area_clamp_to_map(
    area: *mut TileArea,
    shared: *const crate::services::Services,
) {
    let w = unsafe { entry(shared) };
    unsafe { (*area).clamp_to_map(w.dimensions, w.types.count(), |ok| w.check(ok)) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile_type::INVALID_TILE;

    // A 16x8 map; expectations come from the pinned `tilearea.cpp` bodies.
    const MAP: Map = Map { log_x: 4 };
    const SIZE: u32 = 128;

    fn on_map(ok: bool) {
        assert!(ok);
    }

    #[test]
    fn diagonal_areas_follow_the_original_extents_order_and_clipping() {
        type Case = (u32, u32, i16, i16, &'static [u32], &'static [u32]);
        let cases: &[Case] = &[
            (
                51,
                38,
                3,
                -5,
                &[51, 68, 52, 36, 53, 37, 21, 38],
                &[21, 36, 37, 38, 51, 52, 53, 68],
            ),
            (
                38,
                51,
                -3,
                5,
                &[38, 21, 37, 53, 36, 52, 68, 51],
                &[21, 36, 37, 38, 51, 52, 53, 68],
            ),
            (85, 85, -1, -1, &[85], &[85]),
            (17, 68, 7, -1, &[17, 34, 51, 68], &[17, 34, 51, 68]),
            (
                30,
                127,
                8,
                6,
                &[
                    30, 47, 46, 63, 45, 62, 79, 61, 78, 95, 60, 77, 94, 111, 76, 93, 110, 127,
                ],
                &[
                    30, 45, 46, 47, 60, 61, 62, 63, 76, 77, 78, 79, 93, 94, 95, 110, 111, 127,
                ],
            ),
            (
                112,
                2,
                -6,
                -10,
                &[
                    112, 96, 97, 80, 81, 64, 82, 65, 48, 66, 49, 32, 67, 50, 33, 51, 34, 17, 52,
                    35, 18, 36, 19, 2,
                ],
                &[
                    2, 17, 18, 19, 32, 33, 34, 35, 36, 48, 49, 50, 51, 52, 64, 65, 66, 67, 80, 81,
                    82, 96, 97, 112,
                ],
            ),
            (68, 69, 2, -2, &[68, 69], &[68, 69]),
        ];
        for &(start, end, a, b, sequence, contained) in cases {
            let area = DiagonalTileArea::from_corners(MAP, SIZE, start, end, on_map);
            assert_eq!((area.tile, area.a, area.b), (start, a, b));
            let mut iter = TileIterator::create(MAP, SIZE, start, end, true, on_map);
            let mut seen = Vec::new();
            while iter.tile() != INVALID_TILE {
                seen.push(iter.tile());
                iter.advance(MAP, SIZE, on_map);
            }
            assert_eq!(seen, sequence, "{start} -> {end}");
            let inside: Vec<u32> = (0..SIZE).filter(|&t| area.contains(t, MAP)).collect();
            assert_eq!(inside, contained, "{start} -> {end}");
        }
    }

    #[test]
    fn orthogonal_corners_add_and_clamp_follow_the_original() {
        let a = TileArea::from_corners(MAP, SIZE, MAP.tile_xy(7, 5), MAP.tile_xy(2, 1), on_map);
        assert_eq!((a.tile, a.w, a.h), (18, 6, 5));
        // Reversed corners iterate x-fast from the north corner.
        let mut iter = TileIterator::create(
            MAP,
            SIZE,
            MAP.tile_xy(3, 2),
            MAP.tile_xy(2, 1),
            false,
            on_map,
        );
        let mut seen = Vec::new();
        while iter.tile() != INVALID_TILE {
            seen.push(iter.tile());
            iter.advance(MAP, SIZE, on_map);
        }
        assert_eq!(seen, [18, 19, 34, 35]);
        let mut b = TileArea {
            tile: INVALID_TILE,
            w: 0,
            h: 0,
        };
        for (x, y) in [(9, 3), (4, 6), (6, 1)] {
            b.add(MAP.tile_xy(x, y), MAP);
        }
        assert_eq!((b.tile, b.w, b.h), (20, 6, 6));
        let mut c = TileArea {
            tile: MAP.tile_xy(12, 6),
            w: 9,
            h: 5,
        };
        c.clamp_to_map(MAP, SIZE, on_map);
        assert_eq!((c.tile, c.w, c.h), (108, 4, 2));
    }

    #[test]
    fn negative_expansion_wraps_the_unsigned_base_tile() {
        // 64x64: sx = sy = 2^26 and TileXY wraps (2^26 << 6) + 2^26 to 2^26;
        // the signed extent -2^27 + 1 narrows to 1.
        let map = Map::from_log_x(6);
        let mut area = TileArea {
            tile: 0,
            w: 1,
            h: 1,
        };
        area.expand(-67_108_864, map, 64);
        assert_eq!((area.tile, area.w, area.h), (67_108_864, 1, 1));
    }

    #[test]
    fn zero_height_intersection_reports_the_assertion_and_wraps() {
        // The original asserts after its zero-width return; a zero height at row 0 wraps.
        let flat = TileArea {
            tile: 3,
            w: 2,
            h: 0,
        };
        let one = TileArea {
            tile: 3,
            w: 1,
            h: 1,
        };
        let failures = std::cell::Cell::new(0);
        let hit = flat.intersects(one, MAP, |ok| failures.set(failures.get() + u32::from(!ok)));
        assert_eq!((hit, failures.get()), (true, 1));
    }
}
