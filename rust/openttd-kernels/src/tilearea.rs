/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared orthogonal tile-area helpers from `tilearea_type.h` and `tilearea.cpp`.
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
    /// Original `Intersects`, including empty-area handling.
    #[must_use]
    pub fn intersects(self, other: Self, map: Map) -> bool {
        if other.w == 0 || self.w == 0 {
            return false;
        }
        assert!(other.h != 0 && self.h != 0);
        let left1 = map.tile_x(self.tile);
        let top1 = map.tile_y(self.tile);
        let right1 = left1 + u32::from(self.w) - 1;
        let bottom1 = top1 + u32::from(self.h) - 1;
        let left2 = map.tile_x(other.tile);
        let top2 = map.tile_y(other.tile);
        let right2 = left2 + u32::from(other.w) - 1;
        let bottom2 = top2 + u32::from(other.h) - 1;
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
        self.tile = (sy as u32) * map.size_x() + sx as u32;
        self.w = (ex - sx) as u16;
        self.h = (ey - sy) as u16;
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
