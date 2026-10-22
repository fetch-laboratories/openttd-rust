/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `map_func.h` over map dimensions read once per entry.

/// Map dimensions read once per entry.
#[derive(Clone, Copy)]
pub struct Map {
    /// `Map::LogX()` captured for this synchronous entry.
    pub log_x: u8,
}
impl Map {
    /// `Map::SizeX()`, from the dimension captured at entry.
    #[must_use]
    pub fn size_x(self) -> u32 {
        1_u32 << self.log_x
    }
    /// `TileX`.
    #[must_use]
    pub fn tile_x(self, tile: u32) -> u32 {
        tile & (self.size_x() - 1)
    }
    /// `TileY`.
    #[must_use]
    pub fn tile_y(self, tile: u32) -> u32 {
        tile >> self.log_x
    }
    /// Original `DistanceSquare` over captured map dimensions.
    #[must_use]
    pub fn distance_square(self, a: u32, b: u32) -> u32 {
        let x = self.tile_x(a).cast_signed() - self.tile_x(b).cast_signed();
        let y = self.tile_y(a).cast_signed() - self.tile_y(b).cast_signed();
        (x * x + y * y).cast_unsigned()
    }
    /// `TileDiffXY`: `y * Map::SizeX() + x` in unsigned arithmetic.
    #[must_use]
    pub fn diff(self, x: i32, y: i32) -> u32 {
        y.cast_unsigned()
            .wrapping_mul(self.size_x())
            .wrapping_add(x.cast_unsigned())
    }
    /// `TileOffsByAxis`, as the unsigned pattern of its `TileIndexDiff`.
    #[must_use]
    pub fn offs_by_axis(self, axis: u8) -> u32 {
        const OFFSETS: [(i32, i32); 2] = [(1, 0), (0, 1)];
        let (x, y) = OFFSETS[usize::from(axis)];
        self.diff(x, y)
    }
    /// `TileOffsByDiagDir`, as the unsigned pattern of its `TileIndexDiff`.
    #[must_use]
    pub fn offs_by_diag_dir(self, dir: u8) -> u32 {
        const OFFSETS: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
        let (x, y) = OFFSETS[usize::from(dir)];
        self.diff(x, y)
    }
    /// `tile + TileOffsByDiagDir(dir)`.
    #[must_use]
    pub fn add_diag(self, tile: u32, dir: u8) -> u32 {
        tile.wrapping_add(self.offs_by_diag_dir(dir))
    }
    /// `TileAddByDir(tile, dir)`.
    #[must_use]
    pub fn add_dir(self, tile: u32, dir: u8) -> u32 {
        const OFFSETS: [(i32, i32); 8] = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
        ];
        let (x, y) = OFFSETS[usize::from(dir)];
        tile.wrapping_add(self.diff(x, y))
    }
    /// `AddTileIndexDiffCWrap(tile, TileIndexDiffCByDiagDir(dir))`.
    /// Unlike `TileAddWrap`, this includes `MP_VOID` edge tiles.
    #[must_use]
    pub fn add_diff_c_wrap(self, tile: u32, dir: u8, size_y: u32) -> u32 {
        const OFFSETS: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
        let (dx, dy) = OFFSETS[usize::from(dir)];
        let x = self.tile_x(tile).wrapping_add_signed(dx);
        let y = self.tile_y(tile).wrapping_add_signed(dy);
        if x >= self.size_x() || y >= size_y {
            crate::tile_type::INVALID_TILE
        } else {
            y * self.size_x() + x
        }
    }
}
