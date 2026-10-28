/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `map_func.h` over map dimensions read once per entry.
#![allow(unsafe_code)]

/// Map dimensions read once per entry.
#[derive(Clone, Copy)]
pub struct Map {
    /// `Map::LogX()` captured for this synchronous entry.
    pub log_x: u8,
}
impl Map {
    /// `Map::Iterate`, over `Map::Size()` captured for a no-reset entry.
    #[must_use]
    pub fn iterate(count: u32) -> std::ops::Range<u32> {
        0..count
    }

    /// Capture a `Map::LogX()` service result; the original dimension is a `uint8`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_log_x(log_x: u32) -> Self {
        Self { log_x: log_x as u8 }
    }
    /// `Map::SizeX()`, from the dimension captured at entry.
    #[must_use]
    pub fn size_x(self) -> u32 {
        1_u32 << self.log_x
    }
    /// `TileXY`, over the captured map width.
    #[must_use]
    pub fn tile_xy(self, x: u32, y: u32) -> u32 {
        y.wrapping_shl(u32::from(self.log_x)).wrapping_add(x)
    }
    /// `TileVirtXY`, including the unsigned coordinate shifts and addition.
    #[must_use]
    pub fn tile_virt_xy(self, x: u32, y: u32) -> u32 {
        self.tile_xy(x >> 4, y >> 4)
    }
    /// `Map::ScaleBySize`, preserving its pre-division unsigned shift.
    #[must_use]
    pub fn scale_by_size(self, size_y: u32, n: u32) -> u32 {
        let scaled = n.wrapping_shl(u32::from(self.log_x) + size_y.ilog2() - 12);
        scaled.wrapping_add(15) / 16
    }
    /// `AddTileIndexDiffCWrap` for an arbitrary original coordinate offset.
    #[must_use]
    pub fn add_offset_c_wrap(self, tile: u32, dx: i32, dy: i32, size_y: u32) -> u32 {
        let x = self.tile_x(tile).wrapping_add_signed(dx);
        let y = self.tile_y(tile).wrapping_add_signed(dy);
        if x >= self.size_x() || y >= size_y {
            crate::tile_type::INVALID_TILE
        } else {
            self.tile_xy(x, y)
        }
    }
    /// `Map::WrapToMap`, preserving the canonical power-of-two tile mask.
    #[must_use]
    pub fn wrap_to_map(self, tile: u32, size_y: u32) -> u32 {
        tile & (self.size_x() * size_y - 1)
    }
    /// `RandomTileSeed`, using the original `Map::WrapToMap` helper.
    #[must_use]
    pub fn random_tile_seed(self, size_y: u32, random: u32) -> u32 {
        self.wrap_to_map(random, size_y)
    }
    /// `Map::MaxX`.
    #[must_use]
    pub fn max_x(self) -> u32 {
        self.size_x() - 1
    }
    /// `Map::MaxY`, over the height captured at entry.
    #[must_use]
    pub fn max_y(size_y: u32) -> u32 {
        size_y - 1
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
    /// Original `DistanceManhattan`.
    #[must_use]
    pub fn distance_manhattan(self, a: u32, b: u32) -> u32 {
        self.tile_x(a).abs_diff(self.tile_x(b)) + self.tile_y(a).abs_diff(self.tile_y(b))
    }
    /// Original `DistanceMax`, the L-infinity norm.
    #[must_use]
    pub fn distance_max(self, a: u32, b: u32) -> u32 {
        self.tile_x(a)
            .abs_diff(self.tile_x(b))
            .max(self.tile_y(a).abs_diff(self.tile_y(b)))
    }
    /// `TileAddXY`: `tile + TileDiffXY(x, y)` in unsigned arithmetic.
    #[must_use]
    pub fn tile_add_xy(self, tile: u32, x: i32, y: i32) -> u32 {
        tile.wrapping_add(self.diff(x, y))
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
        let (x, y) = tile_index_diff_c_by_diag_dir(dir);
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
        let (dx, dy) = tile_index_diff_c_by_diag_dir(dir);
        self.add_offset_c_wrap(tile, dx, dy, size_y)
    }
}

/// `TileIndexDiffCByDiagDir`, in the original NE/SE/SW/NW order.
#[must_use]
pub fn tile_index_diff_c_by_diag_dir(dir: u8) -> (i32, i32) {
    const OFFSETS: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
    OFFSETS[usize::from(dir)]
}

/// Allocation descriptor acquired once by an audited native entry. The base
/// pointer comes from `Tile::base_tiles.get()`, with the full allocation's provenance.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TileTypeAllocation {
    /// Canonical base array; null only together with a zero count.
    pub base: *const BaseTileLayout,
    /// Canonical extended array from the same native allocation.
    pub extended: *const ExtendedTileLayout,
    /// Actual native `Map::Size()` at acquisition, not a copied tile snapshot;
    /// zero selects the original callback backend for every read.
    pub count: u32,
    /// Native assert policy captured once at the audited entry.
    pub assertions: bool,
}

/// Layout-only shell for the actual native base tile. Never create a reference,
/// read or copy a value of this type; it supplies stride/ABI metadata only.
#[repr(C)]
#[allow(missing_docs)]
pub struct BaseTileLayout {
    pub kind: u8,
    pub height: u8,
    pub m2: u16,
    pub m1: u8,
    pub m3: u8,
    pub m4: u8,
    pub m5: u8,
}

/// Layout-only native extended tile. Never read a whole value or form a reference.
#[repr(C)]
#[allow(missing_docs)]
pub struct ExtendedTileLayout {
    pub m6: u8,
    pub m7: u8,
    pub m8: u16,
}

/// One shared reader with callback and audited entry-resolved raw backends.
/// No clone/copy implementation: allocation validity covers this entry-local
/// reader's entire lifetime. No map reference or slice is ever formed.
pub(crate) struct TileTypeAccess {
    allocation: TileTypeAllocation,
    callback: extern "C" fn(u32) -> u8,
}
// The constructor's provenance/serialization contract covers each initialized
// scalar; every load ends before any callback and native aliases remain legal.
macro_rules! map_fields {
    ($array:ident: $layout:ty => $($field:ident: $scalar:ty),*) => {$ (
        #[inline]
        pub(crate) fn $field(&self, tile: u32) -> Option<$scalar> {
            self.available(tile)?;
            // SAFETY: Live allocation under the entry contract, one scalar only.
            unsafe { Some(std::ptr::addr_of!((*self.allocation.$array.add(tile as usize)).$field).read()) }
        }
    )*};
}
impl TileTypeAccess {
    /// Resolve the audited allocation once, retaining only descriptor metadata.
    /// # Safety
    /// A nonzero count needs both canonical pointers with provenance for `count` live,
    /// initialized base/extended tiles, within `isize::MAX`, until this reader is dropped. Map
    /// reset and game-state-mutex yield are excluded throughout the entry. Each
    /// read must be serialized with native writes; native aliases are permitted.
    pub(crate) unsafe fn raw(
        allocation: TileTypeAllocation,
        callback: extern "C" fn(u32) -> u8,
    ) -> Self {
        Self {
            allocation,
            callback,
        }
    }

    /// Reader for entries without an audited allocation: every read uses the
    /// original noexcept accessor.
    pub(crate) fn callback(callback: extern "C" fn(u32) -> u8) -> Self {
        Self {
            allocation: TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 0,
                assertions: false,
            },
            callback,
        }
    }

    /// Read the original high nibble at this observation point. Checked failures
    /// go through the native accessor; asserts-off reads add no range rejection.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn read(&self, tile: u32) -> u8 {
        crate::tile_map::get_tile_type(self, tile).unwrap_or_else(|| (self.callback)(tile))
    }
    /// Raw backend availability; assertion-disabled builds add no range rejection.
    #[inline]
    pub(crate) fn available(&self, tile: u32) -> Option<()> {
        (self.allocation.count != 0 && (!self.assertions() || tile < self.allocation.count))
            .then_some(())
    }
    #[inline]
    pub(crate) fn assertions(&self) -> bool {
        self.allocation.assertions
    }
    pub(crate) fn count(&self) -> u32 {
        self.allocation.count
    }
    #[inline]
    pub(crate) fn require_type(&self, tile: u32, kind: u8) -> Option<()> {
        self.available(tile)?;
        if self.assertions() && crate::tile_map::get_tile_type(self, tile)? != kind {
            return None;
        }
        Some(())
    }
    // Named, field-sized loads. No Rust reference or record value is formed.
    map_fields! { base: BaseTileLayout => kind: u8, height: u8, m1: u8, m2: u16, m3: u8, m4: u8, m5: u8 }
    map_fields! { extended: ExtendedTileLayout => m6: u8, m8: u16 }
}
