/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `map_func.h` over map dimensions read once per entry.
#![allow(unsafe_code)]
use crate::direction_type::{Axis, DiagDirection, Direction};
use crate::map_type::{TileIndexDiff, TileIndexDiffC};
use crate::tile_type::TileIndex;

/// Map dimensions read once per entry.
#[derive(Clone, Copy)]
pub struct Map {
    /// `Map::LogX()` captured for this synchronous entry.
    pub log_x: u8,
}
impl Map {
    /// `Map::Iterate`, over `Map::Size()` captured for a no-reset entry.
    #[must_use]
    #[inline]
    pub fn iterate(count: u32) -> std::ops::Range<u32> {
        0..count
    }

    /// Capture a `Map::LogX()` service result; the original dimension is a `uint8`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    pub fn from_log_x(log_x: u32) -> Self {
        Self { log_x: log_x as u8 }
    }
    /// `Map::SizeX()`, from the dimension captured at entry.
    #[must_use]
    #[inline]
    pub fn size_x(self) -> u32 {
        1_u32 << self.log_x
    }
    /// `TileXY`, over the captured map width.
    #[must_use]
    #[inline]
    pub fn tile_xy(self, x: u32, y: u32) -> TileIndex {
        TileIndex(y.wrapping_shl(u32::from(self.log_x)).wrapping_add(x))
    }
    /// `TileVirtXY`, including the unsigned coordinate shifts and addition.
    #[must_use]
    #[inline]
    pub fn tile_virt_xy(self, x: u32, y: u32) -> TileIndex {
        self.tile_xy(x >> 4, y >> 4)
    }
    /// `Map::ScaleBySize`, preserving its pre-division unsigned shift.
    #[must_use]
    #[inline]
    pub fn scale_by_size(self, size_y: u32, n: u32) -> u32 {
        // Subtract 12 to avoid overflow for large n; the minimum map is 64 by 64.
        let scaled = n.wrapping_shl(u32::from(self.log_x) + size_y.ilog2() - 12);
        crate::math::ceil_div(scaled, 1 << 4)
    }
    /// `AddTileIndexDiffCWrap`: add a coordinate offset, returning invalid if it
    /// would leave the map. A negative coordinate becomes large when cast unsigned.
    #[must_use]
    #[inline]
    pub fn add_tile_index_diff_c_wrap(
        self,
        tile: TileIndex,
        diff: TileIndexDiffC,
        size_y: u32,
    ) -> TileIndex {
        let x = self.tile_x(tile).wrapping_add_signed(i32::from(diff.x));
        let y = self.tile_y(tile).wrapping_add_signed(i32::from(diff.y));
        if x >= self.size_x() || y >= size_y {
            TileIndex::INVALID
        } else {
            self.tile_xy(x, y)
        }
    }
    /// `Map::WrapToMap`, preserving the canonical power-of-two tile mask.
    #[must_use]
    #[inline]
    pub fn wrap_to_map(self, tile: TileIndex, size_y: u32) -> TileIndex {
        TileIndex(tile.0 & (self.size_x() * size_y - 1))
    }
    /// `RandomTileSeed`, using the original `Map::WrapToMap` helper.
    #[must_use]
    #[inline]
    pub fn random_tile_seed(self, size_y: u32, random: u32) -> TileIndex {
        self.wrap_to_map(TileIndex(random), size_y)
    }
    /// `TileAddWrap`, including freeform-edge exclusion.
    #[must_use]
    pub fn tile_add_wrap(
        self,
        tile: TileIndex,
        dx: i32,
        dy: i32,
        size_y: u32,
        freeform: bool,
    ) -> TileIndex {
        let x = self.tile_x(tile).wrapping_add_signed(dx);
        let y = self.tile_y(tile).wrapping_add_signed(dy);
        if ((x == 0 || y == 0) && freeform) || x >= self.max_x() || y >= Self::max_y(size_y) {
            TileIndex::INVALID
        } else {
            self.tile_xy(x, y)
        }
    }
    /// `Map::MaxX`.
    #[must_use]
    #[inline]
    pub fn max_x(self) -> u32 {
        self.size_x() - 1
    }
    /// `Map::MaxY`, over the height captured at entry.
    #[must_use]
    #[inline]
    pub fn max_y(size_y: u32) -> u32 {
        size_y - 1
    }
    /// `TileX`.
    #[must_use]
    #[inline]
    pub fn tile_x(self, tile: TileIndex) -> u32 {
        tile.0 & (self.size_x() - 1)
    }
    /// `TileY`.
    #[must_use]
    #[inline]
    pub fn tile_y(self, tile: TileIndex) -> u32 {
        tile.0 >> self.log_x
    }
    /// Original `DistanceManhattan`.
    #[must_use]
    #[inline]
    pub fn distance_manhattan(self, a: TileIndex, b: TileIndex) -> u32 {
        crate::math::delta(self.tile_x(a), self.tile_x(b))
            .wrapping_add(crate::math::delta(self.tile_y(a), self.tile_y(b)))
    }
    /// Original `DistanceMax`, the L-infinity norm.
    #[must_use]
    #[inline]
    pub fn distance_max(self, a: TileIndex, b: TileIndex) -> u32 {
        crate::math::delta(self.tile_x(a), self.tile_x(b))
            .max(crate::math::delta(self.tile_y(a), self.tile_y(b)))
    }
    /// `TileIndexToTileIndexDiffC`: the unsigned coordinate differences
    /// `TileX(a) - TileX(b)` and `TileY(a) - TileY(b)`, narrowed to `int16_t`.
    #[must_use]
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "original TileIndexDiffC int16_t member assignment"
    )]
    pub fn tile_index_to_tile_index_diff_c(
        self,
        tile_a: crate::tile_type::TileIndex,
        tile_b: crate::tile_type::TileIndex,
    ) -> crate::map_type::TileIndexDiffC {
        crate::map_type::TileIndexDiffC {
            x: self.tile_x(tile_a).wrapping_sub(self.tile_x(tile_b)) as i16,
            y: self.tile_y(tile_a).wrapping_sub(self.tile_y(tile_b)) as i16,
        }
    }
    /// `TileDiffXY`: coordinate offsets can be negative. Multiplication optimizes
    /// better on MSVC than shifting: zero shifted by a variable is not optimized
    /// properly. Constant x/y often eliminate the multiplication entirely.
    #[must_use]
    #[inline]
    pub fn diff(self, x: i32, y: i32) -> TileIndexDiff {
        y.cast_unsigned()
            .wrapping_mul(self.size_x())
            .wrapping_add(x.cast_unsigned())
            .cast_signed()
    }
    /// `TileOffsByAxis`: convert an `Axis` to the tile offset in southern
    /// direction (SW or SE).
    #[must_use]
    #[inline]
    pub fn offs_by_axis(self, axis: Axis, check: impl Fn(bool)) -> TileIndexDiff {
        if !crate::direction_func::is_valid_axis(axis) {
            check(false);
        }
        self.to_tile_index_diff(TILEOFFS_BY_AXIS[usize::from(axis.0)])
    }
    /// `DiagdirBetweenTiles`: direction from `from` towards `to`, or invalid
    /// when the tiles do not lie on an axis.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn diagdir_between_tiles(self, from: TileIndex, to: TileIndex) -> DiagDirection {
        let dx = (self.tile_x(to) as i32).wrapping_sub(self.tile_x(from) as i32);
        let dy = (self.tile_y(to) as i32).wrapping_sub(self.tile_y(from) as i32);
        if dx == 0 {
            if dy == 0 {
                return DiagDirection::INVALID_DIAGDIR;
            }
            return if dy < 0 {
                DiagDirection::DIAGDIR_NW
            } else {
                DiagDirection::DIAGDIR_SE
            };
        }
        if dy != 0 {
            return DiagDirection::INVALID_DIAGDIR;
        }
        if dx < 0 {
            DiagDirection::DIAGDIR_NE
        } else {
            DiagDirection::DIAGDIR_SW
        }
    }
    /// `DistanceFromEdge`: gets the distance to the edge of the map. Returns
    /// the distance to the nearest edge of the map, in any direction.
    #[must_use]
    pub fn distance_from_edge(self, tile: TileIndex, size_y: u32) -> u32 {
        let xl = self.tile_x(tile);
        let yl = self.tile_y(tile);
        let xh = self.max_x().wrapping_sub(xl);
        let yh = Self::max_y(size_y).wrapping_sub(yl);
        xl.min(yl).min(xh.min(yh))
    }
    /// `DistanceSquare`: gets the 'Square' distance between the two given tiles.
    /// The 'Square' distance is the sum of the squares of the x and y
    /// distances; `sqrt` of it would be the Euclidean distance.
    #[must_use]
    #[inline]
    pub fn distance_square(self, t0: TileIndex, t1: TileIndex) -> u32 {
        let dx = self.tile_x(t0).wrapping_sub(self.tile_x(t1)).cast_signed();
        let dy = self.tile_y(t0).wrapping_sub(self.tile_y(t1)).cast_signed();
        (dx * dx + dy * dy).cast_unsigned()
    }
    /// `TileAddXY`: adds a given offset of x and y to a tile.
    #[must_use]
    #[inline]
    pub fn tile_add_xy(self, tile: TileIndex, x: i32, y: i32) -> TileIndex {
        TileIndex(tile.0.wrapping_add(self.diff(x, y).cast_unsigned()))
    }
    /// `ToTileIndexDiff`: return the offset between two tiles from a
    /// `TileIndexDiffC` struct.
    #[must_use]
    #[inline]
    pub fn to_tile_index_diff(self, tidc: TileIndexDiffC) -> TileIndexDiff {
        self.diff(i32::from(tidc.x), i32::from(tidc.y))
    }
    /// `TileOffsByDiagDir`: convert a `DiagDirection` to a `TileIndexDiff`.
    #[must_use]
    #[inline]
    pub fn tile_offs_by_diag_dir(self, dir: DiagDirection, check: impl Fn(bool)) -> TileIndexDiff {
        self.to_tile_index_diff(tile_index_diff_c_by_diag_dir(dir, check))
    }
    /// `TileOffsByDir`: convert a `Direction` to a `TileIndexDiff`.
    #[must_use]
    #[inline]
    pub fn tile_offs_by_dir(self, dir: Direction, check: impl Fn(bool)) -> TileIndexDiff {
        self.to_tile_index_diff(tile_index_diff_c_by_dir(dir, check))
    }
    /// `TileAddByDir`: adds a `Direction` to a tile.
    #[must_use]
    #[inline]
    pub fn tile_add_by_dir(
        self,
        tile: TileIndex,
        dir: Direction,
        check: impl Fn(bool),
    ) -> TileIndex {
        TileIndex(
            tile.0
                .wrapping_add(self.tile_offs_by_dir(dir, check).cast_unsigned()),
        )
    }
    /// `TileAddByDiagDir`: adds a `DiagDirection` to a tile.
    #[must_use]
    #[inline]
    pub fn tile_add_by_diag_dir(
        self,
        tile: TileIndex,
        dir: DiagDirection,
        check: impl Fn(bool),
    ) -> TileIndex {
        TileIndex(
            tile.0
                .wrapping_add(self.tile_offs_by_diag_dir(dir, check).cast_unsigned()),
        )
    }
    /// Raw adapter: `DistanceFromEdge` over a base tile, for files no pass has
    /// finished yet.
    #[must_use]
    pub fn distance_from_edge_raw(self, tile: u32, size_y: u32) -> u32 {
        self.distance_from_edge(TileIndex(tile), size_y)
    }
    /// Raw adapter: `DistanceSquare` over base tiles.
    #[must_use]
    #[inline]
    pub fn distance_square_raw(self, a: u32, b: u32) -> u32 {
        self.distance_square(TileIndex(a), TileIndex(b))
    }
    /// Raw adapter: `TileOffsByDiagDir` as the unsigned pattern of its
    /// `TileIndexDiff`; the precondition fails through the canonical native assertion.
    #[must_use]
    #[inline]
    pub fn tile_offs_by_diag_dir_raw(self, dir: u8) -> u32 {
        self.tile_offs_by_diag_dir(DiagDirection(dir), native_check)
            .cast_unsigned()
    }
    /// Raw adapter: `TileAddByDiagDir` over base values; the precondition fails
    /// through the canonical native assertion.
    #[must_use]
    #[inline]
    pub fn tile_add_by_diag_dir_raw(self, tile: u32, dir: u8) -> u32 {
        self.tile_add_by_diag_dir(TileIndex(tile), DiagDirection(dir), native_check)
            .0
    }
    /// Raw adapter: `TileAddByDir` over base values; the precondition fails
    /// through the canonical native assertion.
    #[must_use]
    #[inline]
    pub fn tile_add_by_dir_raw(self, tile: u32, dir: u8) -> u32 {
        self.tile_add_by_dir(TileIndex(tile), Direction(dir), native_check)
            .0
    }
    /// `AddTileIndexDiffCWrap(tile, TileIndexDiffCByDiagDir(dir))`.
    /// Unlike `TileAddWrap`, this includes `MP_VOID` edge tiles.
    #[must_use]
    #[inline]
    pub fn add_diff_c_wrap(self, tile: TileIndex, dir: DiagDirection, size_y: u32) -> TileIndex {
        let diff = tile_index_diff_c_by_diag_dir(dir, native_check);
        self.add_tile_index_diff_c_wrap(tile, diff, size_y)
    }
}

/// The canonical native assertion, for the raw adapters' header preconditions.
fn native_check(ok: bool) {
    crate::services_assert::check(ok);
}
/// `_tileoffs_by_axis`: southern (SW/SE) coordinate offset for each axis.
const TILEOFFS_BY_AXIS: [TileIndexDiffC; Axis::AXIS_END.0 as usize] =
    [TileIndexDiffC { x: 1, y: 0 }, TileIndexDiffC { x: 0, y: 1 }];
/// `_tileoffs_by_diagdir`: `TileIndexDiffC` for each `DiagDirection`.
const TILEOFFS_BY_DIAGDIR: [TileIndexDiffC; DiagDirection::DIAGDIR_END.0 as usize] = [
    TileIndexDiffC { x: -1, y: 0 }, // DIAGDIR_NE
    TileIndexDiffC { x: 0, y: 1 },  // DIAGDIR_SE
    TileIndexDiffC { x: 1, y: 0 },  // DIAGDIR_SW
    TileIndexDiffC { x: 0, y: -1 }, // DIAGDIR_NW
];
/// `_tileoffs_by_dir`: `TileIndexDiffC` for each `Direction`.
const TILEOFFS_BY_DIR: [TileIndexDiffC; Direction::DIR_END.0 as usize] = [
    TileIndexDiffC { x: -1, y: -1 }, // DIR_N
    TileIndexDiffC { x: -1, y: 0 },  // DIR_NE
    TileIndexDiffC { x: -1, y: 1 },  // DIR_E
    TileIndexDiffC { x: 0, y: 1 },   // DIR_SE
    TileIndexDiffC { x: 1, y: 1 },   // DIR_S
    TileIndexDiffC { x: 1, y: 0 },   // DIR_SW
    TileIndexDiffC { x: 1, y: -1 },  // DIR_W
    TileIndexDiffC { x: 0, y: -1 },  // DIR_NW
];
/// `TileIndexDiffCByDiagDir`: returns the `TileIndexDiffC` offset from a
/// `DiagDirection`.
#[must_use]
#[inline]
pub fn tile_index_diff_c_by_diag_dir(dir: DiagDirection, check: impl Fn(bool)) -> TileIndexDiffC {
    if !crate::direction_func::is_valid_diag_direction(dir) {
        check(false);
    }
    TILEOFFS_BY_DIAGDIR[usize::from(dir.0)]
}
/// `TileIndexDiffCByDir`: returns the `TileIndexDiffC` offset from a `Direction`.
#[must_use]
#[inline]
pub fn tile_index_diff_c_by_dir(dir: Direction, check: impl Fn(bool)) -> TileIndexDiffC {
    if !crate::direction_func::is_valid_direction(dir) {
        check(false);
    }
    TILEOFFS_BY_DIR[usize::from(dir.0)]
}

/// Addresses of the canonical `Tile` and `Map` statics; entries load them instead
/// of calling. Each array static is a `std::unique_ptr` whose representation is
/// its array pointer (asserted by `OpenTTDNativeTileTypeAccess`).
#[repr(C)]
pub struct MapStatics {
    /// `Tile::base_tiles`.
    pub base: *const *const BaseTileLayout,
    /// `Tile::extended_tiles`.
    pub extended: *const *const ExtendedTileLayout,
    /// `Map::size`.
    pub size: *const u32,
    /// `Map::log_x`.
    pub log_x: *const u32,
    /// `Map::log_y`, including its pre-allocation zero value.
    pub log_y: *const u32,
    /// Native assert policy of the C++ build.
    pub assertions: bool,
}
impl MapStatics {
    /// Load the allocation and dimensions current at this observation point.
    /// # Safety
    /// Every address is live; the loads are serialized with `Map::Allocate`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) unsafe fn load(&self) -> (TileTypeAllocation, Map) {
        unsafe {
            (
                TileTypeAllocation {
                    base: self.base.read(),
                    extended: self.extended.read(),
                    count: self.size.read(),
                    assertions: self.assertions,
                },
                Map::from_log_x(self.log_x.read()),
            )
        }
    }
}

/// Allocation descriptor loaded once by an audited native entry, with the
/// provenance of the full canonical allocation.
#[derive(Clone, Copy)]
pub struct TileTypeAllocation {
    /// Canonical base array; null only together with a zero count.
    pub base: *const BaseTileLayout,
    /// Canonical extended array from the same native allocation.
    pub extended: *const ExtendedTileLayout,
    /// Actual native `Map::Size()` at acquisition, not a copied tile snapshot;
    /// zero selects the original callback backend for every read.
    pub count: u32,
    /// Native assert policy of the C++ build.
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
    native_assert: Option<extern "C" fn(bool)>,
}
// The constructor's provenance/serialization contract covers each initialized
// scalar; every load ends before any callback and native aliases remain legal.
macro_rules! map_fields {
    ($array:ident: $layout:ty => $($field:ident: $scalar:ty),*) => {$ (
        #[allow(clippy::inline_always)]
    #[inline(always)]
        pub(crate) fn $field(&self, tile: u32) -> Option<$scalar> {
            Some(self.fields(tile)?.$field())
        }
    )*};
}
/// Field addresses for one tile after its header getter checks the backend and
/// source preconditions. The borrow keeps allocation metadata live; scalars are
/// loaded afresh, never copied or referenced as a tile record.
pub(crate) struct TileFields<'a> {
    base: *const BaseTileLayout,
    extended: *const ExtendedTileLayout,
    _metadata: std::marker::PhantomData<&'a TileTypeAccess>,
}
macro_rules! tile_fields {
    ($array:ident => $($field:ident: $scalar:ty),*) => {$ (
        #[allow(clippy::inline_always)]
        #[inline(always)]
        pub(crate) fn $field(&self) -> $scalar {
            // SAFETY: The getter's boundary proves the field's readable storage;
            // native aliases may mutate it between these field-sized loads.
            unsafe { std::ptr::addr_of!((*self.$array).$field).read() }
        }
    )*};
}
impl TileFields<'_> {
    tile_fields! { base => kind: u8, height: u8, m1: u8, m2: u16, m3: u8, m4: u8, m5: u8 }
    tile_fields! { extended => m6: u8, m7: u8, m8: u16 }
}

// Stores write one scalar of the canonical allocation, whose native owner is mutable.
macro_rules! map_stores {
    ($array:ident: $layout:ty => $($setter:ident . $field:ident: $scalar:ty),*) => {$ (
        #[inline]
        pub(crate) fn $setter(&self, tile: u32, value: $scalar) -> Option<()> {
            self.available(tile)?;
            // SAFETY: Live allocation under the entry contract, one scalar only.
            unsafe { std::ptr::addr_of_mut!((*self.allocation.$array.cast_mut().add(tile as usize)).$field).write(value) };
            Some(())
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
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) unsafe fn raw(
        allocation: TileTypeAllocation,
        callback: extern "C" fn(u32) -> u8,
    ) -> Self {
        Self {
            allocation,
            callback,
            native_assert: None,
        }
    }

    /// Read the original high nibble at this observation point. Checked failures
    /// go through the native accessor; asserts-off reads add no range rejection.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn read(&self, tile: u32) -> u8 {
        crate::tile_map::get_tile_type(self, tile).unwrap_or_else(|| (self.callback)(tile))
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn field_addresses(&self, tile: u32) -> TileFields<'_> {
        TileFields {
            base: self.allocation.base.wrapping_add(tile as usize),
            extended: self.allocation.extended.wrapping_add(tile as usize),
            _metadata: std::marker::PhantomData,
        }
    }
    /// Raw Tile fields have no source assertions; callback-only readers use None.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn fields(&self, tile: u32) -> Option<TileFields<'_>> {
        self.available(tile)?;
        Some(self.field_addresses(tile))
    }

    /// Raw backend availability; assertion-disabled builds add no range rejection.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn available(&self, tile: u32) -> Option<()> {
        if tile < self.allocation.count {
            return Some(());
        }
        if self.allocation.count == 0 {
            return None;
        }
        (!self.assertions() || self.native_assert.is_some()).then_some(())
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn assertions(&self) -> bool {
        self.allocation.assertions
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn count(&self) -> u32 {
        self.allocation.count
    }
    /// Probe source preconditions without callbacks; the caller handles a failed
    /// predicate through native assertions or an existing typed fallback.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn without_native_assert(&self) -> Self {
        Self {
            allocation: self.allocation,
            callback: self.callback,
            native_assert: None,
        }
    }

    /// Retry a header helper with native assertions at each source assertion site.
    /// Raw Tile field accessors themselves have no range assertions.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn with_native_assert<T>(
        &self,
        native_assert: extern "C" fn(bool),
        read: impl FnOnce(&Self) -> T,
    ) -> T {
        read(&Self {
            allocation: self.allocation,
            callback: self.callback,
            native_assert: Some(native_assert),
        })
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn check(&self, condition: bool) -> Option<()> {
        if self.assertions() && !condition {
            self.native_assert?;
            return self.failed_check();
        }
        Some(())
    }
    // Returning fixture handlers resume after the exact failed source assertion.
    #[cold]
    #[inline(never)]
    fn failed_check(&self) -> Option<()> {
        (self.native_assert?)(false);
        Some(())
    }
    /// The explicit bounds assertion in `GetTileType` and `TileHeight`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn require_range(&self, tile: u32) -> Option<TileFields<'_>> {
        // A successful source bounds check also proves a nonempty backend.
        if tile < self.count() {
            return Some(self.field_addresses(tile));
        }
        self.outside_range(tile)
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn require_type(&self, tile: u32, kind: u8) -> Option<TileFields<'_>> {
        let fields = self.require_range(tile)?;
        if self.assertions() && crate::tile_map::get_tile_type_from_fields(&fields) != kind {
            self.failed_check()?;
        }
        Some(fields)
    }
    // An empty backend retains fixture fallback. A real out-of-range tile keeps
    // the original assertion policy, including continued reads when assertions
    // are disabled or the native handler returns.
    #[cold]
    #[inline(never)]
    fn outside_range(&self, tile: u32) -> Option<TileFields<'_>> {
        if self.count() == 0 {
            return None;
        }
        if self.assertions() {
            self.failed_check()?;
        }
        Some(self.field_addresses(tile))
    }
    /// Field-sized stores under the same serialized allocation contract as reads.
    pub(crate) fn set_m3(&self, tile: u32, value: u8) -> Option<()> {
        self.available(tile)?;
        unsafe {
            std::ptr::addr_of!((*self.allocation.base.add(tile as usize)).m3)
                .cast_mut()
                .write(value);
        }
        Some(())
    }
    pub(crate) fn set_m8(&self, tile: u32, value: u16) -> Option<()> {
        self.available(tile)?;
        unsafe {
            std::ptr::addr_of!((*self.allocation.extended.add(tile as usize)).m8)
                .cast_mut()
                .write(value);
        }
        Some(())
    }
    // Named, field-sized loads. No Rust reference or record value is formed.
    map_fields! { base: BaseTileLayout => kind: u8, m1: u8, m2: u16, m3: u8, m4: u8, m5: u8 }
    map_fields! { extended: ExtendedTileLayout => m6: u8, m7: u8, m8: u16 }
    map_stores! { base: BaseTileLayout => set_kind.kind: u8, set_height.height: u8, set_m1.m1: u8, set_m2.m2: u16, set_m4.m4: u8, set_m5.m5: u8 }
    map_stores! { extended: ExtendedTileLayout => set_m6.m6: u8, set_m7.m7: u8 }
}

/// Entry-local canonical map descriptor. Reads and writes remain field-sized and end before calls.
pub(crate) struct World<'a> {
    pub types: TileTypeAccess,
    pub shared: &'a crate::services::Services,
    pub dimensions: Map,
}
impl<'a> World<'a> {
    /// Load the canonical descriptor and `Map::LogX()` at entry, without a call.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn new(shared: &'a crate::services::Services) -> Self {
        // SAFETY: Game-loop and command entries exclude map reset/load/mutex yield;
        // the shared table's statics address the canonical native map.
        let (allocation, dimensions) = unsafe { (*shared.map).load() };
        // SAFETY: Same entry contract; the allocation was loaded at this entry.
        let mut types = unsafe { TileTypeAccess::raw(allocation, shared.tile_type) };
        // Each failed precondition calls the native assertion once, as the inline
        // accessor does, so helpers check a tile once rather than retry through it.
        // SAFETY: A non-null movement table is the immutable process-lifetime table.
        types.native_assert = unsafe { shared.movement.as_ref() }.map(|m| m.native_assert);
        Self {
            types,
            shared,
            dimensions,
        }
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn check(&self, condition: bool) {
        crate::movement::native_assert(self.shared, condition);
    }
    /// `assert(condition)`: evaluates the condition only where the native build asserts.
    #[inline]
    pub(crate) fn assert_with(&self, condition: impl FnOnce() -> bool) {
        if self.types.assertions() {
            self.check(condition());
        }
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn kind(&self, t: u32) -> u8 {
        self.types.read(t)
    }
    /// `Map::SizeY()`, derived from the descriptor's tile count.
    pub(crate) fn size_y(&self) -> u32 {
        self.types.count() / self.dimensions.size_x()
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn byte(value: Option<u8>) -> u8 {
        value.expect("live map scalar")
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn word(value: Option<u16>) -> u16 {
        value.expect("live map scalar")
    }
}

/// Thread-local stand-ins for the canonical statics; a test sets them before an entry.
#[cfg(test)]
pub(crate) mod fixture {
    use super::{BaseTileLayout, ExtendedTileLayout, MapStatics, TileTypeAllocation};
    use std::cell::Cell;
    thread_local! {
        static BASE: Cell<*const BaseTileLayout> = const { Cell::new(std::ptr::null()) };
        static EXTENDED: Cell<*const ExtendedTileLayout> = const { Cell::new(std::ptr::null()) };
        static SIZE: Cell<u32> = const { Cell::new(0) };
        static LOG_X: Cell<u32> = const { Cell::new(0) };
        static LOG_Y: Cell<u32> = const { Cell::new(0) };
        static STATICS: Cell<MapStatics> = Cell::new(MapStatics {
            base: BASE.with(Cell::as_ptr),
            extended: EXTENDED.with(Cell::as_ptr),
            size: SIZE.with(Cell::as_ptr),
            log_x: LOG_X.with(Cell::as_ptr),
            log_y: LOG_Y.with(Cell::as_ptr),
            assertions: true,
        });
    }
    /// This thread's statics, valid for the thread's lifetime.
    pub(crate) fn statics() -> *const MapStatics {
        STATICS.with(Cell::as_ptr)
    }
    /// Install an allocation and `Map::LogX()` for the next entries on this thread.
    pub(crate) fn set(allocation: TileTypeAllocation, log_x: u32) {
        BASE.set(allocation.base);
        EXTENDED.set(allocation.extended);
        SIZE.set(allocation.count);
        LOG_X.set(log_x);
        LOG_Y.set(if allocation.count == 0 {
            0
        } else {
            allocation.count.trailing_zeros() - log_x
        });
        STATICS.with(|s| {
            // SAFETY: Thread-local cell; no reference to it is live.
            unsafe { (*s.as_ptr()).assertions = allocation.assertions };
        });
    }
}

#[cfg(test)]
mod assertion_tests {
    use super::{Map, tile_index_diff_c_by_diag_dir, tile_index_diff_c_by_dir};
    use crate::direction_type::{Axis, DiagDirection, Direction};
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    type InvalidDirection = fn(&dyn Fn(bool));

    /// The native assertion stops before an invalid table index is used. The
    /// recording mock stops in test code; production helpers contain no test trap.
    #[test]
    fn direction_preconditions_dispatch_the_original_assertion() {
        let operations: [InvalidDirection; 3] = [
            |check| {
                let _ = Map::from_log_x(6).offs_by_axis(Axis::AXIS_END, check);
            },
            |check| {
                let _ = tile_index_diff_c_by_diag_dir(DiagDirection::DIAGDIR_END, check);
            },
            |check| {
                let _ = tile_index_diff_c_by_dir(Direction::DIR_END, check);
            },
        ];
        for operation in operations {
            let calls = Cell::new(0);
            let observed = Cell::new(None);
            let result = catch_unwind(AssertUnwindSafe(|| {
                operation(&|condition| {
                    calls.set(calls.get() + 1);
                    observed.set(Some(condition));
                    panic!("recorded native assertion");
                });
            }));
            assert!(result.is_err());
            assert_eq!(calls.get(), 1);
            assert_eq!(observed.get(), Some(false));
        }
    }
}
