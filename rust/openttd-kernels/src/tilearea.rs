/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared orthogonal tile-area helpers from `tilearea_type.h` and `tilearea.cpp`.
use crate::map_func::Map;
/// Native `OrthogonalTileArea` scalar fields.
pub struct TileArea {
    /// North-west corner.
    pub tile: u32,
    /// Width in tiles.
    pub w: u16,
    /// Height in tiles.
    pub h: u16,
}
impl TileArea {
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
    pub fn tiles(&self, map: Map) -> impl Iterator<Item = u32> {
        let tile = self.tile;
        let width = u32::from(self.w);
        (0..u32::from(self.h))
            .flat_map(move |y| (0..width).map(move |x| tile + y * map.size_x() + x))
    }
}
