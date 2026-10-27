/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `bitmap_type.h`: `BitmapTileArea` storage and the live `BitmapTileIterator` cursor.
//!
//! Native facades alias the leading geometry and call the exports below. A
//! facade or cursor never outlives the storage it names; every Rust borrow of a
//! bitmap ends before the caller's next callback.
#![allow(unsafe_code)]

use crate::map_func::Map;
use crate::tile_type::INVALID_TILE;
use crate::tilearea::{OrthogonalTileIterator, TileArea};

/// `BitmapTileArea`: the inherited `TileArea` followed by one bit per tile.
#[repr(C)]
pub struct BitmapTileArea {
    /// Inherited live geometry, aliased by native `TileArea` readers.
    pub area: TileArea,
    data: Vec<bool>,
}
impl Default for BitmapTileArea {
    fn default() -> Self {
        Self::new()
    }
}
impl BitmapTileArea {
    /// Default constructor: an invalid tile and empty extent.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            area: TileArea {
                tile: INVALID_TILE,
                w: 0,
                h: 0,
            },
            data: Vec::new(),
        }
    }
    /// `Index(x, y)` in the original unsigned arithmetic.
    fn index_xy(&self, x: u32, y: u32) -> u32 {
        y.wrapping_mul(u32::from(self.area.w)).wrapping_add(x)
    }
    /// `Index(tile)` relative to the live base tile.
    fn index(&self, tile: u32, map: Map) -> usize {
        self.index_xy(
            map.tile_x(tile).wrapping_sub(map.tile_x(self.area.tile)),
            map.tile_y(tile).wrapping_sub(map.tile_y(self.area.tile)),
        ) as usize
    }
    /// `Reset`.
    pub fn reset(&mut self) {
        self.area = TileArea {
            tile: INVALID_TILE,
            w: 0,
            h: 0,
        };
        self.data.clear();
    }
    /// `Initialize(const Rect &)`: inclusive widths narrow into the uint16 geometry.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn initialize_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32, map: Map) {
        self.initialize(TileArea {
            tile: map.tile_xy(left as u32, top as u32),
            w: right.wrapping_sub(left).wrapping_add(1) as u16,
            h: bottom.wrapping_sub(top).wrapping_add(1) as u16,
        });
    }
    /// `Initialize(const TileArea &)`; the logical allocation is `Index(w, h)`.
    pub fn initialize(&mut self, ta: TileArea) {
        self.area = ta;
        self.data.clear();
        let size = self.index_xy(u32::from(ta.w), u32::from(ta.h));
        self.data.resize(size as usize, false);
    }
    /// `SetTile`, asserting containment.
    pub fn set_tile(&mut self, tile: u32, map: Map, check: impl Fn(bool)) {
        check(self.area.contains(tile, map, &check));
        let index = self.index(tile, map);
        self.data[index] = true;
    }
    /// `HasTile`.
    pub fn has_tile(&self, tile: u32, map: Map, check: impl Fn(bool)) -> bool {
        self.area.contains(tile, map, check) && self.data[self.index(tile, map)]
    }
}

/// `BitmapTileIterator`: the orthogonal cursor copies the geometry at
/// construction and filters each step through the live bitmap.
#[derive(Clone, Copy)]
pub struct BitmapTileIterator {
    iter: OrthogonalTileIterator,
}
impl BitmapTileIterator {
    /// Construct over a live bitmap.
    /// # Safety
    /// `bitmap` is live for this call; no mutable borrow of it is active.
    pub unsafe fn new(bitmap: *const BitmapTileArea, map: Map, check: impl Fn(bool)) -> Self {
        // SAFETY: Caller contract; the geometry copy ends this borrow.
        let mut it = Self {
            iter: OrthogonalTileIterator::new(unsafe { (*bitmap).area }),
        };
        // SAFETY: Caller contract.
        if !unsafe { (*bitmap).has_tile(it.iter.tile, map, &check) } {
            unsafe { it.advance(bitmap, map, check) };
        }
        it
    }
    /// The current tile, `INVALID_TILE` at the end.
    #[must_use]
    pub fn tile(&self) -> u32 {
        self.iter.tile
    }
    /// `operator++`, including the orthogonal end-of-range assertion.
    /// # Safety
    /// `bitmap` is the live storage this cursor iterates; no mutable borrow of it is active.
    pub unsafe fn advance(
        &mut self,
        bitmap: *const BitmapTileArea,
        map: Map,
        check: impl Fn(bool),
    ) {
        self.iter.advance(map, &check);
        // SAFETY: Caller contract; each membership read is an immediate borrow.
        while self.iter.tile != INVALID_TILE
            && !unsafe { (*bitmap).has_tile(self.iter.tile, map, &check) }
        {
            self.iter.advance(map, &check);
        }
    }
}

/// Allocate a native-owned bitmap (`BitmapTileArea()`).
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_bitmap_new() -> *mut BitmapTileArea {
    Box::into_raw(Box::new(BitmapTileArea::new()))
}
/// Destroy a bitmap from `openttd_rust_bitmap_new`.
/// # Safety
/// `bitmap` came from `openttd_rust_bitmap_new` and no facade or cursor still names it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bitmap_destroy(bitmap: *mut BitmapTileArea) {
    drop(unsafe { Box::from_raw(bitmap) });
}
/// Native `Initialize(const TileArea &)`.
/// # Safety
/// Live bitmap with no other borrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bitmap_initialize(
    bitmap: *mut BitmapTileArea,
    area: TileArea,
) {
    unsafe { (*bitmap).initialize(area) };
}
/// Native `SetTile`.
/// # Safety
/// Live bitmap with no other borrow; `shared` is the live shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bitmap_set(
    bitmap: *mut BitmapTileArea,
    tile: u32,
    shared: *const crate::services::Services,
) {
    let s = unsafe { &*shared };
    let map = Map::from_log_x((s.map_log_x)());
    unsafe { (*bitmap).set_tile(tile, map, |ok| crate::movement::native_assert(s, ok)) };
}
/// Native `HasTile`.
/// # Safety
/// Live bitmap with no mutable borrow; `shared` is the live shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bitmap_has(
    bitmap: *const BitmapTileArea,
    tile: u32,
    shared: *const crate::services::Services,
) -> bool {
    let s = unsafe { &*shared };
    let map = Map::from_log_x((s.map_log_x)());
    unsafe { (*bitmap).has_tile(tile, map, |ok| crate::movement::native_assert(s, ok)) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OrthogonalTileArea" ["station_service"] => TileArea { tile, w, h };
    "OpenTTDBitmapArea" ["station_service"] => TileArea { tile, w, h };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> Map {
        Map::from_log_x(6)
    }

    #[test]
    fn logical_allocation_and_membership_follow_the_original_index() {
        let m = map();
        let mut b = BitmapTileArea::new();
        b.initialize_rect(2, 3, 4, 5, m);
        assert_eq!((b.area.tile, b.area.w, b.area.h), (m.tile_xy(2, 3), 3, 3));
        // `Index(w, h) == w * (h + 1)`.
        assert_eq!(b.data.len(), 12);
        b.set_tile(m.tile_xy(4, 5), m, |ok| assert!(ok));
        assert!(b.has_tile(m.tile_xy(4, 5), m, |ok| assert!(ok)));
        assert!(!b.has_tile(m.tile_xy(5, 5), m, |ok| assert!(ok)));
        b.reset();
        assert_eq!((b.area.tile, b.area.w, b.area.h), (INVALID_TILE, 0, 0));
        assert!(!b.has_tile(m.tile_xy(4, 5), m, |ok| assert!(ok)));
    }

    #[test]
    fn cursor_visits_set_tiles_row_major_and_sees_live_membership() {
        let m = map();
        let mut b = BitmapTileArea::new();
        b.initialize(TileArea {
            tile: m.tile_xy(1, 1),
            w: 3,
            h: 2,
        });
        for (x, y) in [(3, 1), (2, 1)] {
            b.set_tile(m.tile_xy(x, y), m, |ok| assert!(ok));
        }
        let ptr = &raw mut b;
        let mut it = unsafe { BitmapTileIterator::new(ptr, m, |ok| assert!(ok)) };
        let mut seen = Vec::new();
        while it.tile() != INVALID_TILE {
            seen.push(it.tile());
            // Setting a later tile during iteration shows it, as the native pointer does.
            unsafe { (*ptr).set_tile(m.tile_xy(1, 2), m, |ok| assert!(ok)) };
            unsafe { it.advance(ptr, m, |ok| assert!(ok)) };
        }
        assert_eq!(
            seen,
            vec![m.tile_xy(2, 1), m.tile_xy(3, 1), m.tile_xy(1, 2)]
        );
    }

    #[test]
    fn default_empty_cursor_keeps_the_assertion_quirk() {
        // #71: constructing over a default-empty bitmap steps from INVALID_TILE.
        let m = map();
        let b = BitmapTileArea::new();
        let failures = std::cell::Cell::new(0);
        let it = unsafe {
            BitmapTileIterator::new(&raw const b, m, |ok| {
                if !ok {
                    failures.set(failures.get() + 1);
                }
            })
        };
        assert_eq!(it.tile(), INVALID_TILE);
        assert_eq!(failures.get(), 1);
    }
}
