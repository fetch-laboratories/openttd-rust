/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Copied, synchronous shared-world leaves. C++ wrappers are noexcept: environmental
//! failures terminate there. No world reference or borrow crosses a leaf call.
#![allow(unsafe_code, clippy::cast_possible_truncation)]
pub use crate::map_func::Map;
use std::ffi::c_void;

/// Native `RoadTramType : bool`; true denotes the tram mask.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RoadTramType(pub bool);
/// Native `ClearGround` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClearGroundType(pub u8);
/// Native `Slope` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Slope(pub u8);
/// Native `Foundation` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Foundation(pub u8);
/// Native `Roadside` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Roadside(pub u8);
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
/// Native `LandscapeType : uint8_t`; every underlying byte remains valid.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LandscapeType(pub u8);
impl LandscapeType {
    /// Native `LandscapeType::Toyland`.
    pub const TOYLAND: Self = Self(3);
}

/// Shared scalar result of `TimerGameCalendar::ConvertDateToYMD`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CalendarYmd {
    /// Calendar year.
    pub year: i32,
    /// Zero-based month.
    pub month: u8,
    /// One-based day.
    pub day: u8,
}

/// Explicit C ABI carrier for a native calendar Date, converted in C++.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CalendarDate(pub i32);
/// Distinct C ABI carrier for a native economy Date, converted in C++.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EconomyDate(pub i32);

/// Native stack-owned command result, including encoded error parameters.
/// Rust only borrows its address and invokes the native operations below.
#[repr(C)]
pub struct CommandCost {
    _private: [u8; 0],
}
/// Opaque native stack capacity; no native field is read, copied or represented.
/// Native compilation pins sufficient size/alignment on every supported platform.
#[repr(C, align(16))]
pub struct CommandCostStorage {
    /// Uninterpreted, uninitialized capacity lent to native lifetime operations.
    pub storage: [std::mem::MaybeUninit<u8>; 128],
}
/// Native `ExpensesType : uint8_t` carrier.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ExpenseType(pub u8);
/// Copied immutable command-result operations. Every input/output denotes a live
/// native object for this call; C++ owns construction, storage and destruction.
/// Moves and additions use the original native implementation without snapshots.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandCostServices {
    /// Start a native `CommandCost` lifetime in pinned opaque stack storage.
    pub construct: unsafe extern "C" fn(*mut CommandCostStorage) -> *mut CommandCost,
    /// End exactly the native lifetime begun by construct.
    pub destroy: unsafe extern "C" fn(*mut CommandCost),
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
    /// Original expense category; read only for a nonzero successful execution.
    pub expense: extern "C" fn(*const CommandCost) -> ExpenseType,
    /// Native `MakeError`, keeping owner and encoded message.
    pub make_error: unsafe extern "C" fn(*mut CommandCost, u32),
}

/// Ordinary direct commands. Native result storage retains complete errors and
/// parameters; owner accesses must end before a command can reenter Rust.
/// Pointer callbacks require live caller-owned costs and, when nonnull, a live
/// native train. They neither retain nor free those pointers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandServices {
    /// Enter native command nesting and report whether this entry is top-level.
    pub enter: extern "C" fn() -> bool,
    /// Leave exactly one command nesting level.
    pub leave: extern "C" fn(),
    /// Canonical native finance owner resolution and presentation leaves.
    pub finance: &'static crate::company::FinanceServices,
    /// Current infinite-money difficulty setting.
    pub infinite_money: extern "C" fn() -> bool,
    /// Native error storage and local encoded currency formatting only.
    pub money_error: unsafe extern "C" fn(*mut CommandCost, i64),
    /// Canonical infrastructure window invalidation.
    pub infrastructure_dirty: extern "C" fn(u8),
    /// Original `CMD_LANDSCAPE_CLEAR`, with actual `DoCommandFlags` width.
    pub landscape_commands: extern "C" fn() -> *const crate::landscape_cmd::LandscapeServices,
    /// Original `CheckTileOwnership`, preserving native encoded errors.
    pub tile_ownership: unsafe extern "C" fn(*mut CommandCost, u32),
    /// Original `CheckOwnership`.
    pub ownership: unsafe extern "C" fn(*mut CommandCost, u8, u32),
    /// Direct Rust occupancy owner, shared by every command.
    pub occupancy: extern "C" fn() -> *const crate::vehicle::occupancy::Commands,
}

/// Opaque native town shell. Its address identifies the shell, not the Rust owner.
pub use crate::town_type::TownHandle;

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

pub use crate::map_func::TileTypeAllocation;
pub(crate) use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAccess};

/// Opaque native `Train` shell; never dereferenced in Rust.
#[repr(C)]
pub struct Train {
    _private: [u8; 0],
}

/// Canonical immutable world settings shared by owner tables.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EnvironmentServices {
    /// Scenario editor mode.
    pub editor: extern "C" fn() -> bool,
    /// Construction freeform edges.
    pub freeform_edges: extern "C" fn() -> bool,
}

/// Typed native leaves used by the complete shared clear-square policy.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearSquareServices {
    /// Original tile procedure animation capability.

    /// Original animated-tile erasure, including immediate erasure mode.

    /// Original docking marker before map replacement.
    pub docking: extern "C" fn(u32) -> bool,
    /// Original generation setting at map replacement.
    pub generating_world: extern "C" fn() -> bool,
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
    /// Complete synchronous movement owner; immutable process-lifetime table.
    pub movement_services: extern "C" fn() -> *const crate::movement::MovementServices,
    /// Canonical native tile owner write.
    pub set_tile_owner: extern "C" fn(u32, u8),
    /// `AutoslopeEnabled()`.
    pub autoslope: extern "C" fn() -> bool,
    /// Original native `bay_stop_direction` map observation.
    pub bay_stop_direction: extern "C" fn(u32) -> u8,
    /// `IsBridgeAbove`.
    pub bridge_above: extern "C" fn(u32) -> bool,
    /// Original native `bridge_axis` map observation.
    pub bridge_axis: extern "C" fn(u32) -> u8,
    /// Bridge specification speed at this bridge tile.
    pub bridge_speed: extern "C" fn(u32) -> u16,
    /// `construction.build_on_slopes`.
    pub build_on_slopes: extern "C" fn() -> bool,
    /// Native leaves for direct shared Rust clear-square control.
    pub clear_square_services: ClearSquareServices,
    /// Original native uncached nearest-town spatial lookup.
    pub calc_closest_town: extern "C" fn(u32, u32) -> *const TownHandle,
    /// Original native nearest-town spatial lookup; the result may be null.
    pub closest_town: extern "C" fn(u32, u32) -> *const TownHandle,
    /// Whether this company pool ID is live.
    pub company_valid: extern "C" fn(u8) -> bool,
    /// Opaque context for test fixtures; production leaves ignore it.
    pub context: *mut c_void,
    /// Original `GetCrossingRoadBits`.
    pub crossing_road_bits: extern "C" fn(u32) -> RoadBits,
    /// Current native command actor, read at the original short-circuit point.
    pub current_company: extern "C" fn() -> u8,
    /// `GetDepotIndex`.
    pub depot_index: extern "C" fn(u32) -> u16,
    /// Original native `disallowed_road_directions` map observation.
    pub disallowed_road_directions: extern "C" fn(u32) -> u8,
    /// Shared live disaster sound observation.
    pub disaster_sound: extern "C" fn() -> bool,
    /// Original native `distance_from_edge` map observation.
    pub distance_from_edge: extern "C" fn(u32) -> u32,
    /// Original native `drive_through_axis` map observation.
    pub drive_through_axis: extern "C" fn(u32) -> u8,
    /// Original pathfinding setting, observed at the calling branch.
    pub forbid_90_deg: extern "C" fn() -> bool,
    /// `FreeTrainTrackReservation(v)`.
    pub free_train_reservation: unsafe extern "C" fn(*const Train),
    /// Original native `has_road_type` map observation.
    pub has_road_type: extern "C" fn(u32) -> bool,
    /// Original normal-road works predicate.
    pub has_road_works: extern "C" fn(u32) -> bool,
    /// Original native `has_water_ground` map observation.
    pub has_water_ground: extern "C" fn(u32) -> bool,
    /// `TileVirtXY` query: 0 not industry, 1 industry, 2 bubble catcher; writes tile.
    pub industry: extern "C" fn(i32, i32, *mut u32) -> u32,
    /// Original native `is_any_road_stop` map observation.
    pub is_any_road_stop: extern "C" fn(u32) -> bool,
    /// Original native `is_bay_stop` map observation.
    pub is_bay_stop: extern "C" fn(u32) -> bool,
    /// Original native `is_bridge` map observation.
    pub is_bridge: extern "C" fn(u32) -> bool,
    /// Original native `is_drive_through_stop` map observation.
    pub is_drive_through_stop: extern "C" fn(u32) -> bool,
    /// `IsLevelCrossingTile`.
    pub is_level_crossing: extern "C" fn(u32) -> bool,
    /// Original native `IsNormalRoadTile` map observation.
    pub is_normal_road_tile: extern "C" fn(u32) -> bool,
    /// Original native `is_plain_rail` map observation.
    pub is_plain_rail: extern "C" fn(u32) -> bool,
    /// Original native `is_road_depot` map observation.
    pub is_road_depot: extern "C" fn(u32) -> bool,
    /// Original native `is_sea` map observation.
    pub is_sea: extern "C" fn(u32) -> bool,
    /// `IsWaterTile`: water category and clear subtype (sea, canal or river).
    pub is_water_tile: extern "C" fn(u32) -> bool,
    /// Shared live map log x observation.
    pub map_log_x: extern "C" fn() -> u32,
    /// Shared live map max x observation.
    pub map_max_x: extern "C" fn() -> u32,
    /// Shared live map max y observation.
    pub map_max_y: extern "C" fn() -> u32,
    /// Original `MarkBridgeDirty`.
    pub mark_bridge_dirty: extern "C" fn(u32),
    /// `MarkTileDirtyByTile` with the default bridge offset.
    pub mark_dirty: extern "C" fn(u32),
    /// Next live company at or after this ID; 255 denotes end.
    pub next_company: extern "C" fn(u8) -> u8,
    /// Shared live no jetcrash observation.
    pub no_jetcrash: extern "C" fn() -> bool,
    /// Original `GetRoadBits`; only called for a normal road tile.
    pub normal_road_bits: extern "C" fn(u32, u8) -> RoadBits,
    /// Original native `other_tunnel_end` map observation.
    pub other_tunnel_end: extern "C" fn(u32) -> u32,
    /// Shared live plane crashes observation.
    pub plane_crashes: extern "C" fn() -> u8,
    /// Shared live plane speed observation.
    pub plane_speed: extern "C" fn() -> u8,
    /// One shared game Random draw, including debug logging.
    pub random: extern "C" fn(*mut c_void) -> u32,
    /// Original `GetAnyRoadBits`, parameterized by `RoadTramType`.
    pub road_bits: extern "C" fn(u32, u8, bool) -> u8,
    /// Only the road type's construction price multiplier.
    pub road_cost_multiplier: extern "C" fn(u8) -> u16,
    /// Original native `road_depot_direction` map observation.
    pub road_depot_direction: extern "C" fn(u32) -> u8,
    /// Original native `road_owner` map observation.
    pub road_owner: extern "C" fn(u32) -> u8,
    /// Original native `road_type` map observation.
    pub road_type: extern "C" fn(u32) -> u8,
    /// Canonical native `HasCrossingReservation`.
    pub crossing_reserved: extern "C" fn(u32) -> bool,
    /// Canonical native `SetCrossingReservation`.
    pub set_crossing_reserved: extern "C" fn(u32, bool),
    /// Original `MayHaveRoad`, including road-stop and transport guards.
    pub may_have_road: extern "C" fn(u32) -> bool,
    /// Canonical native `road_tile_kind` service.
    pub road_tile_kind: extern "C" fn(u32) -> RoadTileKind,
    /// Canonical native `set_road_bits` service.
    pub set_road_bits: extern "C" fn(u32, u8, u8),
    /// Canonical native `set_road_type` service.
    pub set_road_type: extern "C" fn(u32, u8, u8),
    /// Canonical native `set_road_owner` service.
    pub set_road_owner: extern "C" fn(u32, u8, u8),
    /// Canonical native `set_road_directions` service.
    pub set_road_directions: extern "C" fn(u32, u8),
    /// Canonical native `road_powered` service.
    pub road_powered: extern "C" fn(u8, u8) -> bool,
    /// Leaves for direct Rust type introduction and admission.
    pub type_availability: extern "C" fn() -> *const crate::type_availability::Services,
    /// Native leaves for direct calls to the shared Rust PBS owner.
    pub pbs_services: extern "C" fn(*const Services) -> *const crate::pbs::PbsServices,
    /// Shared live service at helipad observation.
    pub service_at_helipad: extern "C" fn() -> bool,
    /// `SetTropicZone`.
    pub set_tropic_zone: extern "C" fn(u32, u8),
    /// Shared live slope observation.
    /// Current snow line used by town, industry and trees.
    pub snow_line: extern "C" fn() -> u8,
    /// `GetStationIndex`.
    pub station_index: extern "C" fn(u32) -> u16,
    /// Canonical shared `docking_tile` service.
    pub road_commands: extern "C" fn() -> *const crate::road_cmd::RoadCommandServices,
    /// Canonical native station command table.
    pub station_commands: extern "C" fn() -> *const crate::station_cmd::CommandServicesTable,
    /// Canonical native station catchment table.
    pub station_catchment: extern "C" fn() -> *const crate::station::CatchmentServices,
    /// Original docking flag.
    pub docking_tile: extern "C" fn(u32) -> bool,
    /// Canonical shared `station_gfx` service.
    pub station_gfx: extern "C" fn(u32) -> u8,
    /// Canonical shared `station_serve_neutral` service.
    pub station_serve_neutral: extern "C" fn() -> bool,
    /// Canonical shared `station_type` service.
    pub station_type: extern "C" fn(u32) -> StationType,
    /// Canonical shared `tile_animation_add` service.

    /// Canonical shared `tile_animation_delete` service.

    /// Canonical shared `tile_animation_set_frame` service.
    pub tile_animation_set_frame: extern "C" fn(u32, u8),
    /// Canonical shared `town_next_id` service.
    pub town_next_id: extern "C" fn(u32) -> u32,
    /// Canonical shared `town_owner_by_id` service.
    pub town_owner_by_id: extern "C" fn(u32) -> *mut crate::town::State,
    /// Canonical shared `utf8_length` service.
    pub utf8_length: extern "C" fn(*const u8, usize) -> usize,
    /// Canonical shared `vehicle_next` service.
    pub vehicle_next: extern "C" fn(u32) -> Option<crate::vehicle::VehicleRef>,
    /// Whether a water tile is the middle of a lock.
    pub water_middle_lock: extern "C" fn(u32) -> bool,
    /// Original native `tile_height` map observation.
    pub tile_height: extern "C" fn(u32) -> u32,
    /// Original native `tile_max_z` map observation.
    pub tile_max_z: extern "C" fn(u32) -> i32,
    /// `GetTileOwner`.
    pub tile_owner: extern "C" fn(u32) -> u8,
    /// `GetTileSlope`.
    pub tile_slope: extern "C" fn(u32) -> u8,
    /// `GetTileType`.
    pub tile_type: extern "C" fn(u32) -> u8,
    /// Original `TileAddWrap`, including map and freeform-edge checks.
    pub tile_add_wrap: extern "C" fn(u32, i32, i32) -> u32,
    /// Original native `tile_valid` map observation.
    pub tile_valid: extern "C" fn(u32) -> bool,
    /// Original `GetTileSlopeZ`, observed together at the source call.
    pub tile_slope_z: extern "C" fn(u32) -> FoundationSlope,
    /// `GetTileZ`.
    pub tile_z: extern "C" fn(u32) -> i32,
    /// Original native `town_index` map observation.
    pub town_index: extern "C" fn(u32) -> u16,
    /// Resolve the single Rust owner of a live native town shell.
    pub town_owner: extern "C" fn(*const TownHandle) -> *mut crate::town::State,
    /// Original train track direction, computed lazily at the consuming branch.
    pub train_trackdir: unsafe extern "C" fn(*const Train) -> Trackdir,
    /// Original tram owner.
    pub tram_owner: extern "C" fn(u32) -> u8,
    /// Original tram road type (including `INVALID_ROADTYPE`).
    pub tram_type: extern "C" fn(u32) -> RoadType,
    /// `GetTropicZone`.
    pub tropic_zone: extern "C" fn(u32) -> u8,
    /// `TryPathReserve(v, mark_as_stuck, first_tile_okay)`.
    pub try_path_reserve: unsafe extern "C" fn(*mut Train, bool, bool) -> bool,
    /// `GetTunnelBridgeDirection`.
    pub tunnel_bridge_direction: extern "C" fn(u32) -> u8,
    /// Original native `tunnel_transport` map observation.
    pub tunnel_transport: extern "C" fn(u32) -> u8,
    /// Original `UpdateLevelCrossing`; callers retain their original sound argument.
    pub update_level_crossing: extern "C" fn(u32, bool),
    /// Rebuild the canonical town spatial index.
    pub rebuild_town_tree: extern "C" fn(),
    /// Whether the economy is in recession.
    pub recession: extern "C" fn() -> bool,
    /// Global game tick counter.
    pub tick_counter: extern "C" fn() -> u64,
    /// Number of towns in the pool.
    pub town_count: extern "C" fn() -> usize,
    /// `ApplyFoundationToSlope`, returning the height added and the modified slope.
    pub apply_foundation: extern "C" fn(Foundation, Slope) -> FoundationSlope,
    /// `GetClearGround`.
    pub clear_ground: extern "C" fn(u32) -> ClearGroundType,
    /// `GetFoundationSlope`, returning both the foundation slope and its z.
    /// Original `HasTileWaterClass` predicate.
    pub has_water_class: extern "C" fn(u32) -> bool,
    /// `MakeShore`.
    pub make_shore: extern "C" fn(u32),
    /// `MakeSnow(tile, density)`.
    pub make_snow: extern "C" fn(u32, u32),
    /// `SndPlayTileFx(sound, tile)`.
    pub play_sound: extern "C" fn(u32, u16),
    /// Canonical clear-field fence write, retaining native enum and integer widths.
    pub set_fence: extern "C" fn(u32, DiagDirection, u32),
    /// `GetTreeGround`.
    pub tree_ground: extern "C" fn(u32) -> TreeGround,
    /// Immutable water table, resolved lazily to avoid native static-init cycles.
    pub water_services: extern "C" fn() -> *const crate::water::WaterServices,
    /// `for (Station *st : Station::Iterate())`, visiting each station with its
    /// canonical service owner and index. Visits may reenter but never add or
    /// delete a station.
    pub for_each_station: extern "C" fn(*mut c_void, StationVisitor),
    /// `GetRoadStopType`.
    pub road_stop_type: extern "C" fn(u32) -> RoadStopType,
    /// Native road stop shells and their vehicle reads.
    pub road_stops: extern "C" fn() -> *const crate::road_stop::RoadStopServices,
    /// `ShowErrorMessage` with two string IDs at `WL_ERROR`.
    pub show_error: extern "C" fn(u32, u32),
    /// Native `NOT_REACHED`.
    pub not_reached: extern "C" fn(),
}
/// Station visitor: context, station, its service owner and its index.
pub type StationVisitor = unsafe extern "C" fn(
    *mut c_void,
    *const crate::station::Station,
    *mut crate::station_service::Service,
    u16,
);
/// `GetFoundationSlope` result: height and slope after the tile's foundation.
#[repr(C)]
pub struct FoundationSlope {
    /// Height of the foundation.
    pub z: i32,
    /// Slope after applying the foundation.
    pub slope: Slope,
}
impl Services {
    /// Capture `Map::LogX()` once for a synchronous entry.
    #[must_use]
    pub fn map(&self) -> Map {
        Map::from_log_x((self.map_log_x)())
    }
    /// Draw one shared random word.
    #[must_use]
    pub fn random(&self) -> u32 {
        (self.random)(self.context)
    }
    /// Scale one word with the original 64-bit multiply and shift.
    #[must_use]
    pub fn random_range(&self, limit: u32) -> u32 {
        crate::random_func::scale_to_limit(self.random(), limit)
    }
    /// Visit every station in pool order. No owner reference spans a visit.
    #[allow(unsafe_code)]
    pub(crate) fn for_each_station<
        F: FnMut(*const crate::station::Station, *mut crate::station_service::Service, u16),
    >(
        &self,
        mut visit: F,
    ) {
        unsafe extern "C" fn invoke<
            F: FnMut(*const crate::station::Station, *mut crate::station_service::Service, u16),
        >(
            context: *mut c_void,
            station: *const crate::station::Station,
            service: *mut crate::station_service::Service,
            id: u16,
        ) {
            // SAFETY: The service borrows this unique closure only during the call.
            unsafe { (&mut *context.cast::<F>())(station, service, id) }
        }
        (self.for_each_station)(std::ptr::from_mut(&mut visit).cast(), invoke::<F>);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNativeTileTypeAccess::ExtendedTile" ["landscape"] => crate::services::ExtendedTileLayout {
        m6,
        m7,
        m8,
    };
    "OpenTTDCommandCostStorage" ["probe"] => crate::services::CommandCostStorage {
        storage,
    };
    "OpenTTDEnvironmentServices" ["common"] => EnvironmentServices {
        editor,
        freeform_edges,
    };
    "OpenTTDFoundationSlope" ["landscape"] => FoundationSlope {
        z,
        slope,
    };
    "OpenTTDTileTypeAllocation" ["landscape"] => crate::services::TileTypeAllocation {
        base,
        extended,
        count,
        assertions,
    };
    "OpenTTDNativeTileTypeAccess::BaseTile" ["landscape"] => BaseTileLayout {
        kind => type,
        height,
        m2,
        m1,
        m3,
        m4,
        m5,
    };
    "OpenTTDRandomSeeds" ["engine"] => RandomSeeds {
        random,
        interactive,
    };
    "OpenTTDRandomServices" ["engine"] => RandomServices {
        save,
        restore,
        set_seed,
    };
    "OpenTTDRailTraversal" ["probe"] => RailTraversal {
        depot,
        tracks,
    };
    "OpenTTDRailSignalPresence" ["probe"] => RailSignalPresence {
        signals,
        track,
        kind,
        forward,
        reverse,
    };
    "OpenTTDRailSignalUpdate" ["probe"] => RailSignalUpdate {
        kind,
        reservations,
    };
    "OpenTTDRailStationAdmission" ["probe"] => RailStationAdmission {
        rail,
        owner,
        axis,
        blocked,
    };
    "OpenTTDRailCrossingAdmission" ["probe"] => RailCrossingAdmission {
        crossing,
        owner,
        axis,
    };
    "OpenTTDRailWormholeAdmission" ["probe"] => RailWormholeAdmission {
        owner,
        transport,
        direction,
    };
    "OpenTTDRailMapServices" ["probe"] => RailMapServices {
        ground,
        traversal,
        signal_presence,
        signal_update,
        station_admission,
        crossing_admission,
        wormhole_admission,
        is_rail_depot,
        rail_depot_direction,
        has_signal_trackdir,
        signal_state,
        set_signal_state,
        other_tunnel_bridge_end,
        has_signals,
        signal_type,
        invalid_rail_type,
        rail_type,
        set_rail_type,
        rail_tile_type,
        set_track_bits,
        present_signals,
        set_present_signals,
        signal_states,
        set_signal_states,
        set_has_signals,
        set_signal_type,
        signal_variant,
        set_signal_variant,
        cycle_signal_side,
        rail_reservation,
        set_track_reservation,
        depot_reserved,
        set_depot_exit,
        make_rail_normal,
        make_rail_depot,
        crossing_rail_bits,
        crossing_road_axis,
        has_station_rail,
        is_rail_station,
        station_blocked,
        station_track,
        wormhole_reserved,
    };
    "OpenTTDCommandServices" ["probe"] => crate::services::CommandServices {
        enter,
        leave,
        finance,
        infinite_money,
        money_error,
        infrastructure_dirty,
        landscape_commands,
        tile_ownership,
        ownership,
        occupancy,
    };
    "OpenTTDCommandCostServices" ["probe"] => crate::services::CommandCostServices {
        construct,
        destroy,
        zero,
        error,
        expense_money,
        error_param,
        add,
        move_cost => move,
        amount,
        success,
        error_message,
        money,
        expense,
        make_error,
    };
    "OpenTTDClearSquareServices" ["probe"] => ClearSquareServices {


        docking,
        generating_world,
    };
    "OpenTTDSharedServices" ["probe"] => crate::services::Services {
        movement_services,
        set_tile_owner,
        autoslope,
        bay_stop_direction,
        bridge_above,
        bridge_axis,
        bridge_speed,
        build_on_slopes,
        clear_square_services,
        calc_closest_town,
        closest_town,
        company_valid,
        context,
        crossing_road_bits,
        current_company,
        depot_index,
        disallowed_road_directions,
        disaster_sound,
        distance_from_edge,
        drive_through_axis,
        forbid_90_deg,
        free_train_reservation,
        has_road_type,
        has_road_works,
        has_water_ground,
        industry,
        is_any_road_stop,
        is_bay_stop,
        is_bridge,
        is_drive_through_stop,
        is_level_crossing,
        is_normal_road_tile,
        is_plain_rail,
        is_road_depot,
        is_sea,
        is_water_tile,
        map_log_x,
        map_max_x,
        map_max_y,
        mark_bridge_dirty,
        mark_dirty,
        next_company,
        no_jetcrash,
        normal_road_bits,
        other_tunnel_end,
        plane_crashes,
        plane_speed,
        random,
        road_bits,
        road_cost_multiplier,
        road_depot_direction,
        road_owner,
        road_type,
        crossing_reserved,
        set_crossing_reserved,
        may_have_road,
        road_tile_kind,
        set_road_bits,
        set_road_type,
        set_road_owner,
        set_road_directions,
        road_powered,
        type_availability,
        pbs_services,
        service_at_helipad,
        set_tropic_zone,
        snow_line,
        station_index,
        road_commands,
        station_commands,
        station_catchment,
        docking_tile,
        station_gfx,
        station_serve_neutral,
        station_type,

        tile_animation_set_frame,
        town_next_id,
        town_owner_by_id,
        utf8_length,
        vehicle_next,
        water_middle_lock,
        tile_height,
        tile_max_z,
        tile_owner,
        tile_slope,
        tile_type,
        tile_add_wrap,
        tile_valid,
        tile_slope_z,
        tile_z,
        town_index,
        town_owner,
        train_trackdir,
        tram_owner,
        tram_type,
        tropic_zone,
        try_path_reserve,
        tunnel_bridge_direction,
        tunnel_transport,
        update_level_crossing,
        rebuild_town_tree,
        recession,
        tick_counter,
        town_count,
        apply_foundation,
        clear_ground,
        has_water_class,
        make_shore,
        make_snow,
        play_sound,
        set_fence,
        tree_ground,
        water_services,
        for_each_station,
        road_stop_type,
        road_stops,
        show_error,
        not_reached,
    };
    "OpenTTDByteSpan" ["industry"] => ByteSpan {
        data,
        size,
    };
    "OpenTTDCalendarDate" ["maintenance"] => CalendarDate {
        0 => value,
    };
    "OpenTTDEconomyDate" ["maintenance"] => EconomyDate {
        0 => value,
    };
    "LandscapeType" ["maintenance"] => LandscapeType {
    };
}

/// Standalone test table: map predicates return zero and writes do nothing.
#[cfg(test)]
#[must_use]
#[allow(clippy::too_many_lines)] // One field per shared service.
pub fn fixture(
    context: *mut c_void,
    random: extern "C" fn(*mut c_void) -> u32,
    industry: extern "C" fn(i32, i32, *mut u32) -> u32,
) -> Services {
    extern "C" fn zero_byte() -> u8 {
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
    extern "C" fn noop() {}
    extern "C" fn tile(_: u32) {}
    extern "C" fn train(_: *const Train) {}
    extern "C" fn train_path(_: *mut Train, _: bool, _: bool) -> bool {
        false
    }
    extern "C" fn train_safe(_: *const Services) -> *const crate::pbs::PbsServices {
        std::process::abort()
    }
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
    extern "C" fn count0() -> usize {
        0
    }
    extern "C" fn tick0() -> u64 {
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
    extern "C" fn apply_foundation(_: Foundation, slope: Slope) -> FoundationSlope {
        FoundationSlope { z: 0, slope }
    }
    extern "C" fn foundation(_: u32) -> FoundationSlope {
        FoundationSlope {
            z: 0,
            slope: Slope(0),
        }
    }
    extern "C" fn set_reserved(_: u32, _: bool) {}
    extern "C" fn set_three(_: u32, _: u8, _: u8) {}
    extern "C" fn road_kind(_: u32) -> RoadTileKind {
        RoadTileKind(0)
    }
    extern "C" fn two_types(_: u8, _: u8) -> bool {
        false
    }
    extern "C" fn unexpected_error(_: u32, _: u32) {
        std::process::abort()
    }
    extern "C" fn unexpected_path() {
        std::process::abort()
    }
    Services {
        show_error: unexpected_error,
        not_reached: unexpected_path,
        movement_services: fixture_movement,
        set_tile_owner: zone,
        autoslope: flag0,
        bay_stop_direction: byte,
        bridge_above: flag,
        bridge_axis: byte,
        bridge_speed: short,
        build_on_slopes: flag0,
        clear_square_services: ClearSquareServices {
            docking: flag,
            generating_world: flag0,
        },
        calc_closest_town: unused_closest_town,
        closest_town: unused_closest_town,
        company_valid: unused_company_valid,
        context,
        crossing_road_bits: fixture_crossing_road_bits,
        current_company: unused_current_company,
        depot_index: short,
        disallowed_road_directions: byte,
        disaster_sound: flag0,
        distance_from_edge: word,
        drive_through_axis: byte,
        forbid_90_deg: flag0,
        free_train_reservation: train,
        has_road_type: flag,
        has_road_works: flag,
        has_water_ground: flag,
        industry,
        is_any_road_stop: flag,
        is_bay_stop: flag,
        is_bridge: flag,
        is_drive_through_stop: flag,
        is_level_crossing: flag,
        is_normal_road_tile: flag,
        is_plain_rail: flag,
        is_road_depot: flag,
        is_sea: flag,
        is_water_tile: flag,
        map_log_x: word0,
        map_max_x: word0,
        map_max_y: word0,
        mark_bridge_dirty: tile,
        mark_dirty: tile,
        next_company: unused_next_company,
        no_jetcrash: flag0,
        normal_road_bits: fixture_normal_road_bits,
        other_tunnel_end: word,
        plane_crashes: zero_byte,
        plane_speed: zero_byte,
        random,
        road_bits: bits,
        road_cost_multiplier: unused_road_cost_multiplier,
        road_depot_direction: byte,
        road_owner: byte,
        road_type: byte,
        crossing_reserved: flag,
        set_crossing_reserved: set_reserved,
        may_have_road: flag,
        road_tile_kind: road_kind,
        set_road_bits: set_three,
        set_road_type: set_three,
        set_road_owner: set_three,
        set_road_directions: zone,
        road_powered: two_types,
        type_availability: fixture_type_availability,
        pbs_services: train_safe,
        service_at_helipad: flag0,
        set_tropic_zone: zone,
        snow_line: zero_byte,
        station_index: short,
        road_commands: fixture_road_commands,
        station_commands: fixture_station_commands,
        station_catchment: fixture_station_catchment,
        docking_tile: flag,
        station_gfx: byte,
        station_serve_neutral: flag0,
        station_type: fixture_station_type,

        tile_animation_set_frame: zone,
        town_next_id: word,
        town_owner_by_id: unused_town_owner_by_id,
        utf8_length: unused_utf8_length,
        vehicle_next: unused_vehicle_next,
        water_middle_lock: flag,
        tile_height: word,
        tile_max_z: height,
        tile_owner: byte,
        tile_slope: byte,
        tile_type: byte,
        tile_add_wrap: unused_tile_add_wrap,
        tile_valid: flag,
        tile_slope_z: foundation,
        tile_z: height,
        town_index: index,
        town_owner: unused_town_owner,
        train_trackdir: fixture_train_trackdir,
        tram_owner: byte,
        tram_type: fixture_tram_type,
        tropic_zone: byte,
        try_path_reserve: train_path,
        tunnel_bridge_direction: byte,
        tunnel_transport: byte,
        update_level_crossing: fixture_crossing_update,
        rebuild_town_tree: noop,
        recession: flag0,
        tick_counter: tick0,
        town_count: count0,
        apply_foundation,
        clear_ground,
        has_water_class: flag,
        make_shore: tile,
        make_snow,
        play_sound,
        set_fence,
        tree_ground,
        water_services: fixture_water_services,
        for_each_station: fixture_stations,
        road_stop_type: fixture_road_stop_type,
        road_stops: fixture_road_stops,
    }
}
#[cfg(test)]
extern "C" fn fixture_tram_type(_: u32) -> RoadType {
    RoadType(0)
}
#[cfg(test)]
extern "C" fn fixture_normal_road_bits(_: u32, _: u8) -> RoadBits {
    RoadBits(0)
}
#[cfg(test)]
extern "C" fn fixture_crossing_road_bits(_: u32) -> RoadBits {
    RoadBits(0)
}
#[cfg(test)]
extern "C" fn fixture_crossing_update(_: u32, _: bool) {}
#[cfg(test)]
extern "C" fn fixture_train_trackdir(_: *const Train) -> Trackdir {
    Trackdir(0)
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
    use crate::random_func::chance16_i;
    #[test]
    fn chance_retains_low_word_rounding_and_u32_wrap() {
        assert!(chance16_i(1, 200, 0));
        assert!(chance16_i(1, 200, 327));
        assert!(!chance16_i(1, 200, 328));
        assert!(!chance16_i(1, 200, 65535));
        assert!(chance16_i(1, 200, 0xffff_0000));
        // 0xffff_ffff + 0x7fff_ffff wraps to 0x7fff_fffe, so the quotient is 32767.
        assert!(chance16_i(32768, u32::MAX, 1));
        assert!(!chance16_i(32767, u32::MAX, 1));
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
/// Native `RailType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RailType(pub u8);
/// Native `RailTileType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RailTileType(pub u8);
/// Native `RailGroundType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RailGroundType(pub u8);
/// Native `SignalVariant` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SignalVariant(pub u8);
/// Native `RoadType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoadType(pub u8);
/// Native `RoadBits` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoadBits(pub u8);
/// Native `Trackdir` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Trackdir(pub u8);
/// Native `Track` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Track(pub u8);
/// Native `TrackBits` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
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
pub use crate::direction_type::Axis;
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
    /// Canonical track bits and terrain leaves, shared with water and tile loops.
    pub ground: *const crate::ground::RailServices,
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
    /// Native `HasSignals`.
    pub has_signals: extern "C" fn(u32) -> bool,
    /// Native `GetSignalType`.
    pub signal_type: extern "C" fn(u32, u8) -> SignalType,
    /// Native `INVALID_RAILTYPE`.
    pub invalid_rail_type: extern "C" fn() -> RailType,
    /// Native `GetRailType`.
    pub rail_type: extern "C" fn(u32) -> RailType,
    /// Native `SetRailType`.
    pub set_rail_type: extern "C" fn(u32, u8),
    /// Native `GetRailTileType`.
    pub rail_tile_type: extern "C" fn(u32) -> RailTileType,
    /// Native `SetTrackBits`.
    pub set_track_bits: extern "C" fn(u32, u8),
    /// Native `GetPresentSignals`.
    pub present_signals: extern "C" fn(u32) -> std::ffi::c_uint,
    /// Native `SetPresentSignals`.
    pub set_present_signals: extern "C" fn(u32, u8),
    /// Native `GetSignalStates`.
    pub signal_states: extern "C" fn(u32) -> std::ffi::c_uint,
    /// Native `SetSignalStates`.
    pub set_signal_states: extern "C" fn(u32, u8),
    /// Native `SetHasSignals`.
    pub set_has_signals: extern "C" fn(u32, bool),
    /// Native `SetSignalType`.
    pub set_signal_type: extern "C" fn(u32, u8, u8),
    /// Native `GetSignalVariant`.
    pub signal_variant: extern "C" fn(u32, u8) -> SignalVariant,
    /// Native `SetSignalVariant`.
    pub set_signal_variant: extern "C" fn(u32, u8, u8),
    /// Native `CycleSignalSide`.
    pub cycle_signal_side: extern "C" fn(u32, u8),
    /// Native `GetRailReservationTrackBits`.
    pub rail_reservation: extern "C" fn(u32) -> TrackBits,
    /// Native `SetTrackReservation`.
    pub set_track_reservation: extern "C" fn(u32, u8),
    /// Native `HasDepotReservation`.
    pub depot_reserved: extern "C" fn(u32) -> bool,
    /// Native `SetRailDepotExitDirection`.
    pub set_depot_exit: extern "C" fn(u32, u8),
    /// Native `MakeRailNormal(tile, owner, bits, railtype)`.
    pub make_rail_normal: extern "C" fn(u32, u8, u8, u8),
    /// Native `MakeRailDepot(tile, owner, depot, dir, railtype)`.
    pub make_rail_depot: extern "C" fn(u32, u8, u16, u8, u8),
    /// Native `GetCrossingRailBits`.
    pub crossing_rail_bits: extern "C" fn(u32) -> TrackBits,
    /// Native `GetCrossingRoadAxis`.
    pub crossing_road_axis: extern "C" fn(u32) -> Axis,
    /// Native `HasStationRail`.
    pub has_station_rail: extern "C" fn(u32) -> bool,
    /// Native `IsRailStationTile`.
    pub is_rail_station: extern "C" fn(u32) -> bool,
    /// Native `IsStationTileBlocked`.
    pub station_blocked: extern "C" fn(u32) -> bool,
    /// Native `GetRailStationTrack`.
    pub station_track: extern "C" fn(u32) -> Track,
    /// Native `HasTunnelBridgeReservation`.
    pub wormhole_reserved: extern "C" fn(u32) -> bool,
}

/// A live native `CommandCost` borrowed for the synchronous entry.
#[derive(Clone, Copy)]
pub(crate) struct Cost<'a>(
    pub(crate) *mut CommandCost,
    pub(crate) &'a CommandCostServices,
);
impl Cost<'_> {
    pub(crate) fn zero(self) {
        (self.1.zero)(self.0);
    }
    pub(crate) fn error(self, message: u32) {
        (self.1.error)(self.0, message);
    }
    pub(crate) fn set(self, money: i64) {
        (self.1.expense_money)(self.0, crate::economy_type::EXPENSES_CONSTRUCTION, money);
    }
    pub(crate) fn add_money(self, money: i64) {
        (self.1.amount)(self.0, money);
    }
    /// `AddCost(CommandCost &&)`.
    pub(crate) fn add(self, other: Self) {
        (self.1.add)(self.0, other.0);
    }
    pub(crate) fn take(self, other: Self) {
        (self.1.move_cost)(self.0, other.0);
    }
    pub(crate) fn make_error(self, message: u32) {
        unsafe { (self.1.make_error)(self.0, message) };
    }
    pub(crate) fn ok(self) -> bool {
        (self.1.success)(self.0)
    }
    pub(crate) fn message(self) -> u32 {
        (self.1.error_message)(self.0)
    }
    pub(crate) fn money(self) -> i64 {
        (self.1.money)(self.0)
    }
}
/// Command checks that write their result into a `Cost`, whose native slot is live for
/// the synchronous entry; the checks take plain tile, owner and track values.
impl Cost<'_> {
    /// `CheckTileOwnership`.
    pub(crate) fn tile_ownership(self, command: &CommandServices, tile: u32) {
        // SAFETY: The impl contract above.
        unsafe { (command.tile_ownership)(self.0, tile) };
    }
    /// `CheckOwnership`.
    pub(crate) fn ownership(self, command: &CommandServices, owner: u8, tile: u32) {
        // SAFETY: The impl contract above.
        unsafe { (command.ownership)(self.0, owner, tile) };
    }
    /// `EnsureNoVehicleOnGround`.
    pub(crate) fn no_vehicle(self, command: &CommandServices, tile: u32) {
        // SAFETY: The impl contract above.
        unsafe { command.no_vehicle(self.0, tile) };
    }
    /// `EnsureNoTrainOnTrackBits`.
    pub(crate) fn no_train(self, command: &CommandServices, tile: u32, bits: u8) {
        // SAFETY: The impl contract above.
        unsafe { command.no_train(self.0, tile, bits) };
    }
    /// `TunnelBridgeIsFree` with no ignored vehicle.
    pub(crate) fn wormhole_free(self, command: &CommandServices, tile: u32, end: u32) {
        // SAFETY: The impl contract above; a null `ignore` names no vehicle.
        unsafe { command.wormhole_free(self.0, tile, end, std::ptr::null()) };
    }
}

/// Native station tile kind (uint8).
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct StationType(pub u8);
/// Native `RoadStopType : uint8_t` (bus 0, truck 1).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoadStopType(pub u8);
#[cfg(test)]
extern "C" fn fixture_road_stop_type(_: u32) -> RoadStopType {
    panic!("unused road stop type")
}
#[cfg(test)]
extern "C" fn fixture_road_stops() -> *const crate::road_stop::RoadStopServices {
    panic!("unused road stop table")
}
#[cfg(test)]
extern "C" fn fixture_station_type(_: u32) -> StationType {
    panic!("unused station type")
}

#[cfg(test)]
extern "C" fn unused_town_owner_by_id(_: u32) -> *mut crate::town::State {
    panic!("unused town lookup");
}
#[cfg(test)]
extern "C" fn unused_utf8_length(_: *const u8, _: usize) -> usize {
    panic!("unused text length");
}
#[cfg(test)]
extern "C" fn unused_vehicle_next(_: u32) -> Option<crate::vehicle::VehicleRef> {
    panic!("unused vehicle iteration");
}

#[cfg(test)]
extern "C" fn fixture_stations(_: *mut c_void, _: StationVisitor) {}
#[cfg(test)]
extern "C" fn fixture_water_services() -> *const crate::water::WaterServices {
    panic!("unexpected water service access");
}

#[cfg(test)]
extern "C" fn fixture_station_commands() -> *const crate::station_cmd::CommandServicesTable {
    panic!("unused station command table");
}
#[cfg(test)]
extern "C" fn fixture_station_catchment() -> *const crate::station::CatchmentServices {
    panic!("unused station catchment table");
}

#[cfg(test)]
extern "C" fn unused_road_cost_multiplier(_: u8) -> u16 {
    panic!("unused road cost multiplier")
}

/// Native road tile kind.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RoadTileKind(pub u8);

#[cfg(test)]
extern "C" fn fixture_road_commands() -> *const crate::road_cmd::RoadCommandServices {
    panic!("unused road commands")
}

#[cfg(test)]
extern "C" fn unused_tile_add_wrap(_: u32, _: i32, _: i32) -> u32 {
    panic!("unused tile addition")
}

#[cfg(test)]
#[cfg(test)]
extern "C" fn fixture_movement() -> *const crate::movement::MovementServices {
    std::ptr::null()
}

#[cfg(test)]
extern "C" fn fixture_type_availability() -> *const crate::type_availability::Services {
    std::process::abort()
}
