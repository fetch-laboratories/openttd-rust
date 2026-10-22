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
    map_fields! { extended: ExtendedTileLayout => m6: u8 }
}
