/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Copied, synchronous shared-world leaves. C++ wrappers are noexcept: environmental
//! failures terminate there. No world reference or borrow crosses a leaf call.
#![allow(unsafe_code, clippy::cast_possible_truncation)]
use std::ffi::c_void;

/// Native `ClearGround` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClearGroundType(pub u8);
/// Native `RailGroundType` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RailGroundType(pub u8);
/// Native `Slope` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Slope(pub u8);
/// Native `Track` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Track(pub u8);
/// Native `Foundation` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Foundation(pub u8);
/// Native `Roadside` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Roadside(pub u8);
/// Native `RoadBits` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoadBits(pub u8);
/// Native `RoadType` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoadType(pub u8);
/// Native `TreeGround` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TreeGround(pub u8);
/// Native `WaterClass` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WaterClass(pub u8);
/// Native `FloodingBehaviour` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FloodingBehaviour(pub u8);
/// Native `LandscapeType` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LandscapeType(pub u8);

/// Original `TILE_HEIGHT`, shared by raw-to-pixel map consumers.
pub const TILE_HEIGHT: u32 = 8;

/// Native stack-owned command result, including encoded error parameters.
/// Rust only borrows its address and invokes the native operations below.
#[repr(C)]
pub struct CommandCost {
    _private: [u8; 0],
}
/// Copied immutable command-result operations. Every input/output denotes a live
/// native object for this call; C++ owns construction, storage and destruction.
/// Moves and additions use the original native implementation without snapshots.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandCostServices {
    /// Assign native successful zero cost.
    pub zero: extern "C" fn(*mut CommandCost),
    /// Assign the native `StringID` error constructor, including `INVALID_STRING_ID`.
    pub error: extern "C" fn(*mut CommandCost, u32),
    /// Assign native `ExpensesType` and saturating `Money` value.
    pub expense_money: extern "C" fn(*mut CommandCost, u8, i64),
    /// Use native `CommandCostWithParam(StringID, uint64_t)` encoding.
    pub error_param: extern "C" fn(*mut CommandCost, u32, u64),
    /// Add the moved native result, retaining first-error semantics.
    pub add: extern "C" fn(*mut CommandCost, *mut CommandCost),
    /// Move-assign every native field, including encoded parameters.
    pub move_cost: extern "C" fn(*mut CommandCost, *mut CommandCost),
    /// Add native Money with its original saturation.
    pub amount: extern "C" fn(*mut CommandCost, i64),
    /// Query native `Succeeded()`.
    pub success: extern "C" fn(*const CommandCost) -> bool,
    /// Query native `GetErrorMessage()`.
    pub error_message: extern "C" fn(*const CommandCost) -> u32,
    /// Query native `GetCost()`.
    pub money: extern "C" fn(*const CommandCost) -> i64,
}

/// Ordinary direct commands. Native result storage retains complete errors and
/// parameters; owner accesses must end before a command can reenter Rust.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandServices {
    /// Original `CMD_LANDSCAPE_CLEAR`, with actual `DoCommandFlags` width.
    pub landscape_clear: extern "C" fn(*mut CommandCost, u32, u16),
    /// Original ground-vehicle command check; output is live native result storage.
    pub ensure_no_vehicle: unsafe extern "C" fn(*mut CommandCost, u32),
}

/// Opaque native town shell. Its address identifies the shell, not the Rust owner.
#[repr(C)]
pub struct TownHandle {
    _private: [u8; 0],
}

/// Source `TileType::MP_HOUSE`.
pub const MP_HOUSE: u8 = 3;
/// Synchronous borrowed bytes, including embedded NULs. The producing entry
/// specifies lifetime; no native callback may retain this view after mutation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ByteSpan {
    /// Readable bytes for the producing entry's specified lifetime.
    pub data: *const u8,
    /// Full byte count; embedded NULs do not terminate the span.
    pub size: usize,
}

/// Allocation descriptor acquired once by an audited native entry. The base
/// pointer comes from `Tile::base_tiles.get()`, with the full allocation's provenance.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TileTypeAllocation {
    /// Allocation-derived bytes; null only together with a zero count.
    pub base: *const u8,
    /// Actual native `Map::Size()` at acquisition, not a copied tile snapshot;
    /// zero selects the original callback backend for every read.
    pub count: u32,
}

/// Layout-only shell for the actual native base tile. Never create a reference,
/// read or copy a value of this type; it supplies stride/ABI metadata only.
#[repr(C)]
pub(crate) struct BaseTileLayout {
    pub kind: u8,
    pub height: u8,
    pub m2: u16,
    pub m1: u8,
    pub m3: u8,
    pub m4: u8,
    pub m5: u8,
}

/// One shared reader with callback and audited entry-resolved raw backends.
/// No clone/copy implementation: allocation validity covers this entry-local
/// reader's entire lifetime. No map reference or slice is ever formed.
pub(crate) struct TileTypeAccess {
    allocation: TileTypeAllocation,
    callback: extern "C" fn(u32) -> u8,
}
impl TileTypeAccess {
    /// Resolve the audited allocation once, retaining only descriptor metadata.
    /// # Safety
    /// A nonzero count needs a nonnull base that retains provenance for `count * sizeof(TileBase)` live,
    /// initialized bytes, within `isize::MAX`, until this reader is dropped. Map
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
                count: 0,
            },
            callback,
        }
    }

    /// Read the original high nibble at this observation point. Out-of-range
    /// reads go through the original native accessor/assertion policy; no new
    /// rejection, panic or unchecked in-range-only timing shortcut is added.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn read(&self, tile: u32) -> u8 {
        if tile < self.allocation.count {
            // SAFETY: Constructor contract gives live allocation provenance and
            // serialization. Original map bounds keep the byte offset in isize
            // even on 32-bit targets. Exactly one initialized u8 is read; no
            // Rust/C++ field reference is made or retained across a callback.
            let offset = (tile as usize) * std::mem::size_of::<BaseTileLayout>()
                + std::mem::offset_of!(BaseTileLayout, kind);
            unsafe { self.allocation.base.add(offset).read() >> 4 }
        } else {
            (self.callback)(tile)
        }
    }
}

/// Default native ABI probe of the same shared reader used by every owner.
/// # Safety
/// For nonzero counts, uphold `TileTypeAccess::raw`'s validity/serialization
/// contract for this call. The callback is the original noexcept accessor or an
/// equivalent live fixture; it keeps the native out-of-range assertion policy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_shared_tile_type(
    allocation: TileTypeAllocation,
    callback: extern "C" fn(u32) -> u8,
    tile: u32,
) -> u8 {
    unsafe { TileTypeAccess::raw(allocation, callback) }.read(tile)
}

/// Immutable call table supplied by the game (or isolated test world). No global
/// registration or game symbol imports are needed by standalone Cargo binaries.
/// The context and function addresses outlive the component invocation; callbacks
/// do not throw; world operations may reenter after owner accesses end. Output
/// pointers are borrowed for that call only.
/// Map predicates take a `TileIndex` value and return the accessor's underlying type.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Services {
    /// Opaque context for test fixtures; production leaves ignore it.
    pub context: *mut c_void,
    /// One shared game Random draw, including debug logging.
    pub random: extern "C" fn(*mut c_void) -> u32,
    /// `TileVirtXY` query: 0 not industry, 1 industry, 2 bubble catcher; writes tile.
    pub industry: extern "C" fn(i32, i32, *mut u32) -> u32,
    /// `GetTileType`.
    pub tile_type: extern "C" fn(u32) -> u8,
    /// `IsBridgeAbove`.
    pub bridge_above: extern "C" fn(u32) -> bool,
    /// `GetTropicZone`.
    pub tropic_zone: extern "C" fn(u32) -> u8,
    /// `GetTileZ`.
    pub tile_z: extern "C" fn(u32) -> i32,
    /// `GetTileSlope`.
    pub tile_slope: extern "C" fn(u32) -> u8,
    /// `MarkTileDirtyByTile` with the default bridge offset.
    pub mark_dirty: extern "C" fn(u32),
    /// `SetTropicZone`.
    pub set_tropic_zone: extern "C" fn(u32, u8),
    /// `GetTileOwner`.
    pub tile_owner: extern "C" fn(u32) -> u8,
    /// `GetStationIndex`.
    pub station_index: extern "C" fn(u32) -> u16,
    /// `GetDepotIndex`.
    pub depot_index: extern "C" fn(u32) -> u16,
    /// `IsLevelCrossingTile`.
    pub is_level_crossing: extern "C" fn(u32) -> bool,
    /// `GetTunnelBridgeDirection`.
    pub tunnel_bridge_direction: extern "C" fn(u32) -> u8,
    /// Bridge specification speed at this bridge tile.
    pub bridge_speed: extern "C" fn(u32) -> u16,
    /// `IsWaterTile`: water category and clear subtype (sea, canal or river).
    pub is_water_tile: extern "C" fn(u32) -> bool,
    /// Whether this company pool ID is live.
    pub company_valid: extern "C" fn(u8) -> bool,
    /// Next live company at or after this ID; 255 denotes end.
    pub next_company: extern "C" fn(u8) -> u8,
    /// Current native command actor, read at the original short-circuit point.
    pub current_company: extern "C" fn() -> u8,
    /// Original native nearest-town spatial lookup; the result may be null.
    pub closest_town: extern "C" fn(u32, u32) -> *const TownHandle,
    /// Resolve the single Rust owner of a live native town shell.
    pub town_owner: extern "C" fn(*const TownHandle) -> *mut crate::town::State,
    /// Original native `tile_valid` map observation.
    pub tile_valid: extern "C" fn(u32) -> bool,
    /// Original native `distance_from_edge` map observation.
    pub distance_from_edge: extern "C" fn(u32) -> u32,
    /// Original native `tile_max_z` map observation.
    pub tile_max_z: extern "C" fn(u32) -> i32,
    /// Original native `tile_height` map observation.
    pub tile_height: extern "C" fn(u32) -> u32,
    /// Original native `bridge_axis` map observation.
    pub bridge_axis: extern "C" fn(u32) -> u8,
    /// Original native `has_water_ground` map observation.
    pub has_water_ground: extern "C" fn(u32) -> bool,
    /// Original native `is_sea` map observation.
    pub is_sea: extern "C" fn(u32) -> bool,
    /// Original native `is_plain_rail` map observation.
    pub is_plain_rail: extern "C" fn(u32) -> bool,
    /// Original native `is_bridge` map observation.
    pub is_bridge: extern "C" fn(u32) -> bool,
    /// Original native `tunnel_transport` map observation.
    pub tunnel_transport: extern "C" fn(u32) -> u8,
    /// Original native `other_tunnel_end` map observation.
    pub other_tunnel_end: extern "C" fn(u32) -> u32,
    /// Original native `is_road_depot` map observation.
    pub is_road_depot: extern "C" fn(u32) -> bool,
    /// Original native `road_depot_direction` map observation.
    pub road_depot_direction: extern "C" fn(u32) -> u8,
    /// Original native `is_normal_road` map observation.
    pub is_normal_road: extern "C" fn(u32) -> bool,
    /// Original native `disallowed_road_directions` map observation.
    pub disallowed_road_directions: extern "C" fn(u32) -> u8,
    /// Original native `is_bay_stop` map observation.
    pub is_bay_stop: extern "C" fn(u32) -> bool,
    /// Original native `bay_stop_direction` map observation.
    pub bay_stop_direction: extern "C" fn(u32) -> u8,
    /// Original native `is_drive_through_stop` map observation.
    pub is_drive_through_stop: extern "C" fn(u32) -> bool,
    /// Original native `drive_through_axis` map observation.
    pub drive_through_axis: extern "C" fn(u32) -> u8,
    /// Original native `is_any_road_stop` map observation.
    pub is_any_road_stop: extern "C" fn(u32) -> bool,
    /// Original native `road_type` map observation.
    pub road_type: extern "C" fn(u32) -> u8,
    /// Original native `has_road_type` map observation.
    pub has_road_type: extern "C" fn(u32) -> bool,
    /// Original native `road_owner` map observation.
    pub road_owner: extern "C" fn(u32) -> u8,
    /// Original native `town_index` map observation.
    pub town_index: extern "C" fn(u32) -> u16,
    /// Original `GetAnyRoadBits`, parameterized by `RoadTramType`.
    pub road_bits: extern "C" fn(u32, u8, bool) -> u8,
    /// Current snow line used by town, industry and trees.
    pub snow_line: extern "C" fn() -> u8,
    /// Current calendar year used by town and industry.
    pub calendar_year: extern "C" fn() -> i32,
    /// Shared live map log x observation.
    pub map_log_x: extern "C" fn() -> u32,
    /// Shared live map max x observation.
    pub map_max_x: extern "C" fn() -> u32,
    /// Shared live map max y observation.
    pub map_max_y: extern "C" fn() -> u32,
    /// Shared live plane speed observation.
    pub plane_speed: extern "C" fn() -> u8,
    /// Shared live no jetcrash observation.
    pub no_jetcrash: extern "C" fn() -> bool,
    /// Shared live plane crashes observation.
    pub plane_crashes: extern "C" fn() -> u8,
    /// Shared live service at helipad observation.
    pub service_at_helipad: extern "C" fn() -> bool,
    /// Shared live disaster sound observation.
    pub disaster_sound: extern "C" fn() -> bool,
    /// Shared live economy date observation.
    pub economy_date: extern "C" fn() -> i32,
    /// Shared live calendar date observation.
    pub calendar_date: extern "C" fn() -> i32,
    /// Shared live slope observation.
    pub slope: extern "C" fn(i32, i32) -> i32,
    /// `GetClearGround`.
    pub clear_ground: extern "C" fn(u32) -> ClearGroundType,
    /// `GetTreeGround`.
    pub tree_ground: extern "C" fn(u32) -> TreeGround,
    /// `MakeSnow(tile, density)`.
    pub make_snow: extern "C" fn(u32, u32),
    /// `MakeShore`.
    pub make_shore: extern "C" fn(u32),
    /// Native `TileLoop_Water` facade; may reenter Rust owners.
    pub tile_loop_water: extern "C" fn(u32),
    /// `SndPlayTileFx(sound, tile)`.
    pub play_sound: extern "C" fn(u32, u16),
    /// `GetFoundationSlope`, returning both the foundation slope and its z.
    pub foundation: extern "C" fn(u32) -> FoundationSlope,
    /// Canonical clear-field fence write, retaining native enum and integer widths.
    pub set_fence: extern "C" fn(u32, DiagDirection, u32),
    /// Original `HasTileWaterClass` predicate.
    pub has_water_class: extern "C" fn(u32) -> bool,
}
/// `GetFoundationSlope` result: height and slope after the tile's foundation.
#[repr(C)]
pub struct FoundationSlope {
    /// Height of the foundation.
    pub z: i32,
    /// Slope after applying the foundation.
    pub slope: Slope,
}
impl Services {
    /// Draw one shared random word.
    #[must_use]
    pub fn random(&self) -> u32 {
        (self.random)(self.context)
    }
    /// Scale one word with the original 64-bit multiply and shift.
    #[must_use]
    pub fn random_range(&self, limit: u32) -> u32 {
        ((u64::from(self.random()) * u64::from(limit)) >> 32) as u32
    }
}
/// Standalone test table: map predicates return zero and writes do nothing.
#[cfg(test)]
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn fixture(
    context: *mut c_void,
    random: extern "C" fn(*mut c_void) -> u32,
    industry: extern "C" fn(i32, i32, *mut u32) -> u32,
) -> Services {
    extern "C" fn zero_byte() -> u8 {
        0
    }
    extern "C" fn zero_year() -> i32 {
        0
    }
    extern "C" fn byte(_: u32) -> u8 {
        0
    }
    extern "C" fn flag(_: u32) -> bool {
        false
    }
    extern "C" fn short(_: u32) -> u16 {
        0
    }
    extern "C" fn height(_: u32) -> i32 {
        0
    }
    extern "C" fn tile(_: u32) {}
    extern "C" fn zone(_: u32, _: u8) {}
    extern "C" fn word(_: u32) -> u32 {
        0
    }
    extern "C" fn index(_: u32) -> u16 {
        0
    }
    extern "C" fn bits(_: u32, _: u8, _: bool) -> u8 {
        0
    }
    extern "C" fn flag0() -> bool {
        false
    }
    extern "C" fn word0() -> u32 {
        0
    }
    extern "C" fn slope(_: i32, _: i32) -> i32 {
        0
    }
    extern "C" fn clear_ground(_: u32) -> ClearGroundType {
        ClearGroundType(0)
    }
    extern "C" fn tree_ground(_: u32) -> TreeGround {
        TreeGround(0)
    }
    extern "C" fn make_snow(_: u32, _: u32) {}
    extern "C" fn play_sound(_: u32, _: u16) {}
    extern "C" fn set_fence(_: u32, _: DiagDirection, _: u32) {}
    extern "C" fn foundation(_: u32) -> FoundationSlope {
        FoundationSlope {
            z: 0,
            slope: Slope(0),
        }
    }
    Services {
        context,
        random,
        industry,
        tile_type: byte,
        bridge_above: flag,
        tropic_zone: byte,
        tile_z: height,
        tile_slope: byte,
        mark_dirty: tile,
        set_tropic_zone: zone,
        tile_owner: byte,
        station_index: short,
        depot_index: short,
        is_level_crossing: flag,
        tunnel_bridge_direction: byte,
        bridge_speed: short,
        is_water_tile: flag,
        company_valid: unused_company_valid,
        next_company: unused_next_company,
        current_company: unused_current_company,
        closest_town: unused_closest_town,
        town_owner: unused_town_owner,
        tile_valid: flag,
        distance_from_edge: word,
        tile_max_z: height,
        tile_height: word,
        bridge_axis: byte,
        has_water_ground: flag,
        is_sea: flag,
        is_plain_rail: flag,
        is_bridge: flag,
        tunnel_transport: byte,
        other_tunnel_end: word,
        is_road_depot: flag,
        road_depot_direction: byte,
        is_normal_road: flag,
        disallowed_road_directions: byte,
        is_bay_stop: flag,
        bay_stop_direction: byte,
        is_drive_through_stop: flag,
        drive_through_axis: byte,
        is_any_road_stop: flag,
        road_type: byte,
        has_road_type: flag,
        road_owner: byte,
        town_index: index,
        road_bits: bits,
        snow_line: zero_byte,
        calendar_year: zero_year,
        map_log_x: word0,
        map_max_x: word0,
        map_max_y: word0,
        plane_speed: zero_byte,
        no_jetcrash: flag0,
        plane_crashes: zero_byte,
        service_at_helipad: flag0,
        disaster_sound: flag0,
        economy_date: zero_year,
        calendar_date: zero_year,
        slope,
        clear_ground,
        tree_ground,
        make_snow,
        make_shore: tile,
        tile_loop_water: tile,
        play_sound,
        foundation,
        set_fence,
        has_water_class: flag,
    }
}
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
    /// `tile + TileOffsByDiagDir(dir)`.
    #[must_use]
    pub fn add_diag(self, tile: u32, dir: u8) -> u32 {
        const OFFSETS: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
        let (x, y) = OFFSETS[usize::from(dir)];
        tile.wrapping_add(self.diff(x, y))
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

/// Source `Chance16I` uses uint32 multiplication/addition after uint16 truncation.
#[must_use]
pub fn chance16_i(a: u32, b: u32, r: u32) -> bool {
    (((r & 65535).wrapping_mul(b).wrapping_add(b / 2)) >> 16) < a
}

#[cfg(test)]
pub(crate) extern "C" fn unused_company_valid(_: u8) -> bool {
    false
}
#[cfg(test)]
pub(crate) extern "C" fn unused_next_company(_: u8) -> u8 {
    u8::MAX
}

#[cfg(test)]
pub(crate) extern "C" fn unused_closest_town(_: u32, _: u32) -> *const TownHandle {
    std::ptr::null()
}
#[cfg(test)]
pub(crate) extern "C" fn unused_town_owner(_: *const TownHandle) -> *mut crate::town::State {
    std::ptr::null_mut()
}

#[cfg(test)]
pub(crate) extern "C" fn unused_current_company() -> u8 {
    u8::MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chance_retains_low_word_rounding_and_u32_wrap() {
        assert!(chance16_i(1, 200, 0));
        assert!(chance16_i(1, 200, 327));
        assert!(!chance16_i(1, 200, 328));
        assert!(!chance16_i(1, 200, 65535));
        assert!(chance16_i(1, 200, 0xffff_0000));
        assert_eq!(
            chance16_i(1, u32::MAX, 65535),
            ((65535_u32.wrapping_mul(u32::MAX).wrapping_add(u32::MAX / 2)) >> 16) < 1
        );
    }
}

/// Shared complete RNG state; both streams survive scoped engine reseeding.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RandomSeeds {
    /// Simulation stream state.
    pub random: [u32; 2],
    /// Presentation stream state.
    pub interactive: [u32; 2],
}
#[repr(C)]
/// Borrowed synchronous shared RNG operations.
pub struct RandomServices {
    /// Save both streams without a draw.
    pub save: unsafe extern "C" fn() -> RandomSeeds,
    /// Restore both streams without a draw.
    pub restore: unsafe extern "C" fn(RandomSeeds),
    /// Original `SetRandomSeed` operation.
    pub set_seed: unsafe extern "C" fn(u32),
}

/// Native byte enums use transparent carriers: every bit pattern remains valid
/// across FFI, including source sentinels and values outside named variants.
/// Native `DiagDirection` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct DiagDirection(pub u8);
/// Native `TrackBits` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct TrackBits(pub u8);
/// Native `SignalType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SignalType(pub u8);
/// Native `SignalState` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SignalState(pub u8);
/// Native `Axis` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Axis(pub u8);
/// Native `TransportType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct TransportType(pub u8);
/// Fields consumed together at one original rail/signal branch phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailTraversal {
    /// Native `depot` observation; later-branch fields are read only after their guards.
    pub depot: bool,
    /// Native `tracks` observation; later-branch fields are read only after their guards.
    pub tracks: TrackBits,
}
/// Fields consumed together at one original rail/signal branch phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailSignalPresence {
    /// Native `signals` observation; later-branch fields are read only after their guards.
    pub signals: bool,
    /// Native `track` observation; later-branch fields are read only after their guards.
    pub track: bool,
    /// Native `kind` observation; later-branch fields are read only after their guards.
    pub kind: SignalType,
    /// Native `forward` observation; later-branch fields are read only after their guards.
    pub forward: bool,
    /// Native `reverse` observation; later-branch fields are read only after their guards.
    pub reverse: bool,
}
/// Fields consumed together at one original rail/signal branch phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailSignalUpdate {
    /// Native `kind` observation; later-branch fields are read only after their guards.
    pub kind: SignalType,
    /// Native `reservations` observation; later-branch fields are read only after their guards.
    pub reservations: TrackBits,
}
/// Fields consumed together at one original rail/signal branch phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailStationAdmission {
    /// Native `rail` observation; later-branch fields are read only after their guards.
    pub rail: bool,
    /// Native `owner` observation; later-branch fields are read only after their guards.
    pub owner: u8,
    /// Native `axis` observation; later-branch fields are read only after their guards.
    pub axis: Axis,
    /// Native `blocked` observation; later-branch fields are read only after their guards.
    pub blocked: bool,
}
/// Fields consumed together at one original rail/signal branch phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailCrossingAdmission {
    /// Native `crossing` observation; later-branch fields are read only after their guards.
    pub crossing: bool,
    /// Native `owner` observation; later-branch fields are read only after their guards.
    pub owner: u8,
    /// Native `axis` observation; later-branch fields are read only after their guards.
    pub axis: Axis,
}
/// Fields consumed together at one original rail/signal branch phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailWormholeAdmission {
    /// Native `owner` observation; later-branch fields are read only after their guards.
    pub owner: u8,
    /// Native `transport` observation; later-branch fields are read only after their guards.
    pub transport: TransportType,
    /// Native `direction` observation; later-branch fields are read only after their guards.
    pub direction: DiagDirection,
}

/// Canonical shared rail/signal map operations; pointers remain immutable for the call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailMapServices {
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub traversal: extern "C" fn(u32) -> RailTraversal,
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub signal_presence: extern "C" fn(u32, u8, u8) -> RailSignalPresence,
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub signal_update: extern "C" fn(u32, u8) -> RailSignalUpdate,
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub station_admission: extern "C" fn(u32, u8, u8) -> RailStationAdmission,
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub crossing_admission: extern "C" fn(u32, u8) -> RailCrossingAdmission,
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub wormhole_admission: extern "C" fn(u32, u8) -> RailWormholeAdmission,

    /// Native `is_rail_depot` map operation at the original observation point.
    pub is_rail_depot: extern "C" fn(u32) -> bool,
    /// Native `rail_depot_direction` map operation at the original observation point.
    pub rail_depot_direction: extern "C" fn(u32) -> DiagDirection,
    /// Native `has_signal_trackdir` map operation at the original observation point.
    pub has_signal_trackdir: extern "C" fn(u32, u8) -> bool,
    /// Native `signal_state` map operation at the original observation point.
    pub signal_state: extern "C" fn(u32, u8) -> SignalState,
    /// Native `set_signal_state` map operation at the original observation point.
    pub set_signal_state: extern "C" fn(u32, u8, u8) -> (),
    /// Native `other_tunnel_bridge_end` map operation at the original observation point.
    pub other_tunnel_bridge_end: extern "C" fn(u32) -> u32,
    /// Native `rail_track_status` map operation at the original observation point.
    pub rail_track_status: extern "C" fn(u32, u8) -> u32,
}
