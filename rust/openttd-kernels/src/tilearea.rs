/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Orthogonal and diagonal tile areas from `tilearea_type.h` and `tilearea.cpp`.
//!
//! Native `OrthogonalTileArea` facades pass their area in place; each export
//! borrows it only for its call and keeps base integers. The orthogonal
//! iterator's advance stays inline in C++ loops. Diagonal areas and
//! [`TileIterator`] serve Rust loops only.
#![allow(unsafe_code)]
use crate::map_func::Map;
use crate::math::is_inside_bs;
use crate::tile_type::TileIndex;

/// `OrthogonalTileArea`: a rectangle of tiles, laid out as the native struct.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrthogonalTileArea {
    /// The base tile of the area.
    pub tile: TileIndex,
    /// The width of the area.
    pub w: u16,
    /// The height of the area.
    pub h: u16,
}
/// Shorthand for the much more common orthogonal tile area.
pub type TileArea = OrthogonalTileArea;
impl OrthogonalTileArea {
    /// `OrthogonalTileArea(tile, w, h)`: construct this tile area with some set values.
    #[must_use]
    pub const fn new(tile: TileIndex, w: u16, h: u16) -> Self {
        Self { tile, w, h }
    }
    /// `OrthogonalTileArea(start, end)`: the area spanning both corners in either order.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_corners(
        map: Map,
        size: u32,
        start: TileIndex,
        end: TileIndex,
        check: impl Fn(bool),
    ) -> Self {
        check(start.0 < size);
        check(end.0 < size);
        let mut sx = map.tile_x(start);
        let mut sy = map.tile_y(start);
        let mut ex = map.tile_x(end);
        let mut ey = map.tile_y(end);
        if sx > ex {
            std::mem::swap(&mut sx, &mut ex);
        }
        if sy > ey {
            std::mem::swap(&mut sy, &mut ey);
        }
        Self {
            tile: map.tile_xy(sx, sy),
            w: (ex - sx + 1) as u16,
            h: (ey - sy + 1) as u16,
        }
    }
    /// `Add`: add a single tile to a tile area; enlarge if needed.
    #[allow(clippy::cast_possible_truncation)]
    pub fn add(&mut self, to_add: TileIndex, map: Map) {
        if self.tile == TileIndex::INVALID {
            self.tile = to_add;
            self.w = 1;
            self.h = 1;
            return;
        }
        let mut sx = map.tile_x(self.tile);
        let mut sy = map.tile_y(self.tile);
        let mut ex = sx.wrapping_add(u32::from(self.w)).wrapping_sub(1);
        let mut ey = sy.wrapping_add(u32::from(self.h)).wrapping_sub(1);
        let ax = map.tile_x(to_add);
        let ay = map.tile_y(to_add);
        sx = ax.min(sx);
        sy = ay.min(sy);
        ex = ax.max(ex);
        ey = ay.max(ey);
        self.tile = map.tile_xy(sx, sy);
        self.w = ex.wrapping_sub(sx).wrapping_add(1) as u16;
        self.h = ey.wrapping_sub(sy).wrapping_add(1) as u16;
    }
    /// `Intersects`: does this tile area intersect with another?
    #[must_use]
    pub fn intersects(self, ta: Self, map: Map, check: impl FnOnce(bool)) -> bool {
        if ta.w == 0 || self.w == 0 {
            return false;
        }
        check(ta.w != 0 && ta.h != 0 && self.w != 0 && self.h != 0);
        let left1 = map.tile_x(self.tile);
        let top1 = map.tile_y(self.tile);
        let right1 = left1.wrapping_add(u32::from(self.w)).wrapping_sub(1);
        let bottom1 = top1.wrapping_add(u32::from(self.h)).wrapping_sub(1);
        let left2 = map.tile_x(ta.tile);
        let top2 = map.tile_y(ta.tile);
        let right2 = left2.wrapping_add(u32::from(ta.w)).wrapping_sub(1);
        let bottom2 = top2.wrapping_add(u32::from(ta.h)).wrapping_sub(1);
        !(left2 > right1 || right2 < left1 || top2 > bottom1 || bottom2 < top1)
    }
    /// `Contains`: does this tile area contain a tile?
    #[must_use]
    pub fn contains(self, tile: TileIndex, map: Map, check: impl FnOnce(bool)) -> bool {
        if self.w == 0 {
            return false;
        }
        check(self.w != 0 && self.h != 0);
        let left = map.tile_x(self.tile);
        let top = map.tile_y(self.tile);
        let tile_x = map.tile_x(tile);
        let tile_y = map.tile_y(tile);
        is_inside_bs(tile_x as usize, left as usize, usize::from(self.w))
            && is_inside_bs(tile_y as usize, top as usize, usize::from(self.h))
    }
    /// `Expand`: expand a tile area by `rad` tiles in each direction, keeping within map bounds.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn expand(&mut self, rad: i32, map: Map, size_y: u32) {
        let x = map.tile_x(self.tile) as i32;
        let y = map.tile_y(self.tile) as i32;
        let sx = x.wrapping_sub(rad).max(0);
        let sy = y.wrapping_sub(rad).max(0);
        let ex = x
            .wrapping_add(i32::from(self.w))
            .wrapping_add(rad)
            .min(map.size_x() as i32);
        let ey = y
            .wrapping_add(i32::from(self.h))
            .wrapping_add(rad)
            .min(size_y as i32);
        self.tile = map.tile_xy(sx as u32, sy as u32);
        self.w = ex.wrapping_sub(sx) as u16;
        self.h = ey.wrapping_sub(sy) as u16;
    }
    /// `ClampToMap`: clamp the tile area to map borders. The unsigned remaining
    /// extent converts to `int` before the minimum.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn clamp_to_map(&mut self, map: Map, size: u32, check: impl FnOnce(bool)) {
        check(self.tile.0 < size);
        let size_y = size >> map.log_x;
        self.w =
            i32::from(self.w).min(map.size_x().wrapping_sub(map.tile_x(self.tile)) as i32) as u16;
        self.h = i32::from(self.h).min(size_y.wrapping_sub(map.tile_y(self.tile)) as i32) as u16;
    }
    /// The range-`for` over `begin()` and `end()`: each tile of the
    /// [`OrthogonalTileIterator`] walk until it reaches `INVALID_TILE`.
    pub fn tiles(self, map: Map) -> impl Iterator<Item = TileIndex> + use<> {
        let mut iter = OrthogonalTileIterator::new(self);
        std::iter::from_fn(move || {
            if iter.tile == TileIndex::INVALID {
                return None;
            }
            let tile = iter.tile;
            iter.step(map);
            Some(tile)
        })
    }
    /// Raw adapter of [`Self::tiles`] for callers in unfinished files.
    pub fn tiles_raw(self, map: Map) -> impl Iterator<Item = u32> + use<> {
        self.tiles(map).map(|tile| tile.0)
    }
}

/// `OrthogonalTileIterator`: iterator to iterate over a tile area (rectangle) of the map.
#[derive(Clone, Copy)]
pub struct OrthogonalTileIterator {
    /// The current tile we are at, `INVALID_TILE` when we're done.
    pub tile: TileIndex,
    /// The width of the iterated area.
    w: i32,
    /// The current 'x' position in the rectangle.
    x: i32,
    /// The current 'y' position in the rectangle.
    y: i32,
}
impl OrthogonalTileIterator {
    /// `OrthogonalTileIterator(const OrthogonalTileArea &)`.
    #[must_use]
    pub fn new(ta: TileArea) -> Self {
        Self {
            tile: if ta.w == 0 || ta.h == 0 {
                TileIndex::INVALID
            } else {
                ta.tile
            },
            w: i32::from(ta.w),
            x: i32::from(ta.w),
            y: i32::from(ta.h),
        }
    }
    /// `operator++`: move ourselves to the next tile in the rectangle on the map.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn advance(&mut self, map: Map, check: impl FnOnce(bool)) {
        check(self.tile != TileIndex::INVALID);
        self.step(map);
    }
    /// The body of `operator++` after its assertion, which cannot fail inside a range-`for`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn step(&mut self, map: Map) {
        self.x -= 1;
        if self.x > 0 {
            self.tile = TileIndex(self.tile.0.wrapping_add(1));
        } else {
            self.y -= 1;
            if self.y > 0 {
                self.x = self.w;
                self.tile += map.diff(1 - self.w, 1);
            } else {
                self.tile = TileIndex::INVALID;
            }
        }
    }
}

/// `DiagonalTileArea`: extents are one past the end, signed by direction.
#[derive(Clone, Copy)]
pub struct DiagonalTileArea {
    /// Base tile of the area.
    pub tile: TileIndex,
    /// Extent in diagonal "x" direction (may be negative to signify the area stretches to the left).
    pub a: i16,
    /// Extent in diagonal "y" direction (may be negative to signify the area stretches upwards).
    pub b: i16,
}
impl DiagonalTileArea {
    /// `DiagonalTileArea(start, end)`: create a diagonal tile area from two corners.
    /// Unsigned coordinate sums narrow to `int16_t` before the one-past-end step.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_corners(
        map: Map,
        size: u32,
        start: TileIndex,
        end: TileIndex,
        check: impl Fn(bool),
    ) -> Self {
        check(start.0 < size);
        check(end.0 < size);
        // Unfortunately we can't find a new base and make all a and b positive because
        // the new base might be a "flattened" corner where there actually is no single
        // tile. If we try anyway the result is either inaccurate ("one off" half of the
        // time) or the code gets much more complex;
        //
        // We also need to increment/decrement a and b here to have one-past-end semantics
        // for a and b, just the way the orthogonal tile area does it for w and h.
        let mut a = map
            .tile_y(end)
            .wrapping_add(map.tile_x(end))
            .wrapping_sub(map.tile_y(start))
            .wrapping_sub(map.tile_x(start)) as i16;
        let mut b = map
            .tile_y(end)
            .wrapping_sub(map.tile_x(end))
            .wrapping_sub(map.tile_y(start))
            .wrapping_add(map.tile_x(start)) as i16;
        if a > 0 {
            a = a.wrapping_add(1);
        } else {
            a = a.wrapping_sub(1);
        }
        if b > 0 {
            b = b.wrapping_add(1);
        } else {
            b = b.wrapping_sub(1);
        }
        Self { tile: start, a, b }
    }
    /// `Contains`: does this tile area contain a tile? Unsigned coordinate sums convert to `int`.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn contains(self, tile: TileIndex, map: Map) -> bool {
        let a = map.tile_y(tile).wrapping_add(map.tile_x(tile)) as i32;
        let b = map.tile_y(tile).wrapping_sub(map.tile_x(tile)) as i32;
        let mut start_a = map.tile_y(self.tile).wrapping_add(map.tile_x(self.tile)) as i32;
        let mut start_b = map.tile_y(self.tile).wrapping_sub(map.tile_x(self.tile)) as i32;
        let mut end_a = start_a.wrapping_add(i32::from(self.a));
        let mut end_b = start_b.wrapping_add(i32::from(self.b));
        // Swap if necessary, preserving the "one past end" semantics.
        if start_a > end_a {
            let tmp = start_a;
            start_a = end_a.wrapping_add(1);
            end_a = tmp.wrapping_add(1);
        }
        if start_b > end_b {
            let tmp = start_b;
            start_b = end_b.wrapping_add(1);
            end_b = tmp.wrapping_add(1);
        }
        a >= start_a && a < end_a && b >= start_b && b < end_b
    }
}

/// `DiagonalTileIterator`: iterator to iterate over a diagonal area of the map.
#[derive(Clone, Copy)]
pub struct DiagonalTileIterator {
    /// The current tile we are at, `INVALID_TILE` when we're done.
    pub tile: TileIndex,
    /// The base tile x coordinate from where the iterating happens.
    base_x: u32,
    /// The base tile y coordinate from where the iterating happens.
    base_y: u32,
    /// The current (rotated) x coordinate of the iteration.
    a_cur: i32,
    /// The current (rotated) y coordinate of the iteration.
    b_cur: i32,
    /// The (rotated) x coordinate of the end of the iteration.
    a_max: i32,
    /// The (rotated) y coordinate of the end of the iteration.
    b_max: i32,
}
impl DiagonalTileIterator {
    /// `DiagonalTileIterator(const DiagonalTileArea &)`, starting at the base tile.
    #[must_use]
    pub fn new(ta: DiagonalTileArea, map: Map) -> Self {
        Self {
            tile: ta.tile,
            base_x: map.tile_x(ta.tile),
            base_y: map.tile_y(ta.tile),
            a_cur: 0,
            b_cur: 0,
            a_max: i32::from(ta.a),
            b_max: i32::from(ta.b),
        }
    }
    /// `operator++`: move ourselves to the next tile in the rectangle on the map.
    /// Its loop continues while `tile > Map::Size()`, so only `INVALID_TILE` is skipped.
    #[allow(clippy::cast_sign_loss)]
    pub fn advance(&mut self, map: Map, size: u32, check: impl FnOnce(bool)) {
        check(self.tile != TileIndex::INVALID);
        let size_y = size >> map.log_x;
        // Determine the next tile, while clipping at map borders.
        loop {
            // Iterate using the rotated coordinates.
            if self.a_max == 1 || self.a_max == -1 {
                // Special case: Every second column has zero length, skip them completely.
                self.a_cur = 0;
                if self.b_max > 0 {
                    self.b_cur = self.b_cur.wrapping_add(2).min(self.b_max);
                } else {
                    self.b_cur = self.b_cur.wrapping_sub(2).max(self.b_max);
                }
            } else {
                // Every column has at least one tile to process.
                let new_line = if self.a_max > 0 {
                    self.a_cur = self.a_cur.wrapping_add(2);
                    self.a_cur >= self.a_max
                } else {
                    self.a_cur = self.a_cur.wrapping_sub(2);
                    self.a_cur <= self.a_max
                };
                if new_line {
                    // Offset of initial a_cur: one tile in the same direction as a_max
                    // every second line.
                    self.a_cur = if self.a_cur.wrapping_abs() % 2 != 0 {
                        0
                    } else if self.a_max > 0 {
                        1
                    } else {
                        -1
                    };
                    if self.b_max > 0 {
                        self.b_cur = self.b_cur.wrapping_add(1);
                    } else {
                        self.b_cur = self.b_cur.wrapping_sub(1);
                    }
                }
            }
            // And convert the coordinates back once we've gone to the next tile.
            let x = self
                .base_x
                .wrapping_add((self.a_cur.wrapping_sub(self.b_cur) / 2) as u32);
            let y = self
                .base_y
                .wrapping_add((self.b_cur.wrapping_add(self.a_cur) / 2) as u32);
            // Prevent wrapping around the map's borders.
            self.tile = if x >= map.size_x() || y >= size_y {
                TileIndex::INVALID
            } else {
                map.tile_xy(x, y)
            };
            if !(self.tile.0 > size && self.b_max != self.b_cur) {
                break;
            }
        }
        if self.b_max == self.b_cur {
            self.tile = TileIndex::INVALID;
        }
    }
}

/// `TileIterator::Create(corner1, corner2, diagonal)` for Rust area loops: an
/// enum of the two iterators, because `Create` allocates and an entry may not.
#[derive(Clone, Copy)]
pub enum TileIterator {
    /// `OrthogonalTileIterator(corner1, corner2)`.
    Orthogonal(OrthogonalTileIterator),
    /// `DiagonalTileIterator(corner1, corner2)`.
    Diagonal(DiagonalTileIterator),
}
impl TileIterator {
    /// `Create`: either iterator, whose corner constructor asserts its map range.
    #[must_use]
    pub fn create(
        map: Map,
        size: u32,
        corner1: TileIndex,
        corner2: TileIndex,
        diagonal: bool,
        check: impl Fn(bool),
    ) -> Self {
        if diagonal {
            Self::Diagonal(DiagonalTileIterator::new(
                DiagonalTileArea::from_corners(map, size, corner1, corner2, check),
                map,
            ))
        } else {
            Self::Orthogonal(OrthogonalTileIterator::new(TileArea::from_corners(
                map, size, corner1, corner2, check,
            )))
        }
    }
    /// `operator*`: the tile we are at, or `INVALID_TILE` when we're done.
    #[must_use]
    pub fn tile(&self) -> TileIndex {
        match self {
            Self::Orthogonal(iter) => iter.tile,
            Self::Diagonal(iter) => iter.tile,
        }
    }
    /// `operator++`, with each iterator's end-of-range assertion.
    pub fn advance(&mut self, map: Map, size: u32, check: impl FnOnce(bool)) {
        match self {
            Self::Orthogonal(iter) => iter.advance(map, check),
            Self::Diagonal(iter) => iter.advance(map, size, check),
        }
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
    let area = TileArea::from_corners(
        w.dimensions,
        w.types.count(),
        TileIndex(start),
        TileIndex(end),
        |ok| {
            w.check(ok);
        },
    );
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
    unsafe { (*area).add(TileIndex(to_add), w.dimensions) };
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

    // A 16x8 map; expectations come from the pinned `tilearea.cpp` bodies.
    const MAP: Map = Map::from_log_x(4);
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
            let (start, end) = (TileIndex(start), TileIndex(end));
            let area = DiagonalTileArea::from_corners(MAP, SIZE, start, end, on_map);
            assert_eq!((area.tile, area.a, area.b), (start, a, b));
            let mut iter = TileIterator::create(MAP, SIZE, start, end, true, on_map);
            let mut seen = Vec::new();
            while iter.tile() != TileIndex::INVALID {
                seen.push(iter.tile().0);
                iter.advance(MAP, SIZE, on_map);
            }
            assert_eq!(seen, sequence, "{start:?} -> {end:?}");
            let inside: Vec<u32> = (0..SIZE)
                .filter(|&t| area.contains(TileIndex(t), MAP))
                .collect();
            assert_eq!(inside, contained, "{start:?} -> {end:?}");
        }
    }

    #[test]
    fn orthogonal_corners_add_and_clamp_follow_the_original() {
        let a = TileArea::from_corners(MAP, SIZE, MAP.tile_xy(7, 5), MAP.tile_xy(2, 1), on_map);
        assert_eq!((a.tile.0, a.w, a.h), (18, 6, 5));
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
        while iter.tile() != TileIndex::INVALID {
            seen.push(iter.tile().0);
            iter.advance(MAP, SIZE, on_map);
        }
        assert_eq!(seen, [18, 19, 34, 35]);
        let mut b = TileArea::new(TileIndex::INVALID, 0, 0);
        for (x, y) in [(9, 3), (4, 6), (6, 1)] {
            b.add(MAP.tile_xy(x, y), MAP);
        }
        assert_eq!((b.tile.0, b.w, b.h), (20, 6, 6));
        let mut c = TileArea {
            tile: MAP.tile_xy(12, 6),
            w: 9,
            h: 5,
        };
        c.clamp_to_map(MAP, SIZE, on_map);
        assert_eq!((c.tile.0, c.w, c.h), (108, 4, 2));
    }

    #[test]
    fn negative_expansion_wraps_the_unsigned_base_tile() {
        // 64x64: sx = sy = 2^26 and TileXY wraps (2^26 << 6) + 2^26 to 2^26;
        // the signed extent -2^27 + 1 narrows to 1.
        let map = Map::from_log_x(6);
        let mut area = TileArea::new(TileIndex(0), 1, 1);
        area.expand(-67_108_864, map, 64);
        assert_eq!((area.tile.0, area.w, area.h), (67_108_864, 1, 1));
    }

    #[test]
    fn extreme_expansion_wraps_before_clipping_and_narrowing() {
        // OrthogonalTileArea::Expand on a 16x8 map at (3, 2), with extents (4, 3).
        // INT_MIN: starts wrap negative and clip to 0; ends are -2147483641/-2147483643.
        // INT_MAX: starts clip to 0; ends wrap to -2147483642/-2147483644.
        for (radius, expected) in [(i32::MIN, (0, 7, 5)), (i32::MAX, (0, 6, 4))] {
            let mut area = TileArea {
                tile: MAP.tile_xy(3, 2),
                w: 4,
                h: 3,
            };
            area.expand(radius, MAP, 8);
            assert_eq!((area.tile.0, area.w, area.h), expected);
        }
        // At the origin INT_MIN - INT_MIN wraps before the uint16_t narrowing.
        let mut area = TileArea::new(TileIndex(0), 1, 1);
        area.expand(i32::MIN, MAP, 8);
        assert_eq!((area.tile.0, area.w, area.h), (0, 1, 1));
    }

    #[test]
    fn zero_height_intersection_reports_the_assertion_and_wraps() {
        // The original asserts after its zero-width return; a zero height at row 0 wraps.
        let flat = TileArea::new(TileIndex(3), 2, 0);
        let one = TileArea::new(TileIndex(3), 1, 1);
        let failures = std::cell::Cell::new(0);
        let hit = flat.intersects(one, MAP, |ok| failures.set(failures.get() + u32::from(!ok)));
        assert_eq!((hit, failures.get()), (true, 1));
    }

    #[test]
    fn corner_constructor_asserts_both_corners_are_on_the_map() {
        // `OrthogonalTileArea(start, end)` asserts `start < Map::Size()` and `end < Map::Size()`.
        let failures = std::cell::Cell::new(0);
        let count = |ok: bool| failures.set(failures.get() + u32::from(!ok));
        let area = TileArea::from_corners(MAP, SIZE, TileIndex(SIZE), TileIndex(18), count);
        assert_eq!(failures.get(), 1);
        // Map::Size() is (0, 8) on the 16x8 map; the area still spans both corners.
        assert_eq!((area.tile.0, area.w, area.h), (16, 3, 8));
        let _ = TileArea::from_corners(MAP, SIZE, TileIndex(0), TileIndex(SIZE - 1), count);
        assert_eq!(failures.get(), 1);
    }

    #[test]
    fn tiles_follow_the_orthogonal_iterator_walk() {
        let walk = |area: TileArea| area.tiles(MAP).map(|t| t.0).collect::<Vec<_>>();
        // A multi-row area at (3, 2), three wide and two high.
        assert_eq!(
            walk(TileArea::new(MAP.tile_xy(3, 2), 3, 2)),
            [35, 36, 37, 51, 52, 53]
        );
        // A zero extent starts at INVALID_TILE.
        assert_eq!(walk(TileArea::new(TileIndex(5), 0, 3)), [] as [u32; 0]);
        assert_eq!(walk(TileArea::new(TileIndex(5), 3, 0)), [] as [u32; 0]);
        // The range-`for` ends where the walk reaches INVALID_TILE.
        assert_eq!(
            walk(TileArea::new(TileIndex(u32::MAX - 1), 3, 2)),
            [u32::MAX - 1]
        );
    }
}
