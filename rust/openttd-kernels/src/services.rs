/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Copied, synchronous shared-world leaves. C++ wrappers are noexcept: environmental
//! failures terminate there. No world reference or borrow crosses a leaf call.
#![allow(unsafe_code, clippy::cast_possible_truncation)]
pub use crate::clear_map::ClearGround as ClearGroundType;
pub use crate::direction_type::DiagDirection;
pub use crate::economy_type::ExpensesType as ExpenseType;
pub use crate::landscape_type::LandscapeType;
pub use crate::map_func::Map;
pub use crate::rail_map::RailGroundType;
pub use crate::rail_map::RailTileType;
pub use crate::rail_type::RailType;
pub use crate::road_map::RoadTileType as RoadTileKind;
pub use crate::road_map::Roadside;
pub use crate::road_type::RoadBits;
pub use crate::road_type::RoadTramType;
pub use crate::road_type::RoadType;
pub use crate::signal_type::SignalState;
pub use crate::signal_type::SignalType;
pub use crate::signal_type::SignalVariant;
pub use crate::slope_type::Foundation;
pub use crate::slope_type::Slope;
pub use crate::station_type::RoadStopType;
pub use crate::station_type::StationType;
pub use crate::timer_game_calendar::CalendarDate;
pub use crate::timer_game_economy::EconomyDate;
pub use crate::track_type::Track;
pub use crate::track_type::TrackBits;
pub use crate::track_type::Trackdir;
pub use crate::transport_type::TransportType;
pub use crate::tree_map::TreeGround;
pub use crate::water::FloodingBehaviour;
pub use crate::water_map::WaterClass;
use std::ffi::c_void;

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
    /// Native owned-by parameter formatting after the local-company decision.
    pub encode_owned_by:
        extern "C" fn(*mut CommandCost, crate::company_type::Owner, crate::tile_type::TileIndex),
    /// Native error-owner metadata.
    pub error_owner: extern "C" fn(*mut CommandCost, crate::company_type::Owner),
}

/// Ordinary direct commands. Native result storage retains complete errors and
/// parameters; owner accesses must end before a command can reenter Rust.
/// Pointer callbacks require live caller-owned costs and, when nonnull, a live
/// native train. They neither retain nor free those pointers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandServices {
    /// Shared command metadata from the native traits table.
    pub command_flags: extern "C" fn(u8) -> u16,
    /// Original command type for pause admission.
    pub command_type: extern "C" fn(u8) -> u8,
    /// Original command-range validation.
    pub command_valid: extern "C" fn(u8) -> bool,
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
    /// Company owner for direct ownership checks.
    pub company: extern "C" fn() -> *const crate::company::Services,
    /// Direct Rust occupancy owner, shared by every command.
    pub occupancy: extern "C" fn() -> *const crate::vehicle::occupancy::Commands,
    /// Shared map, company and generation observations of the invocation lifecycle.
    pub shared: &'static Services,
    /// Native result storage of every invocation.
    pub cost: &'static CommandCostServices,
    /// Rail map table of the signal buffer owner for `UpdateSignalsInBuffer`.
    pub rail_map: &'static RailMapServices,
    /// Native signal leaves of the signal buffer owner.
    pub signals: &'static crate::signal::SignalServices,
    /// `_shift_pressed`.
    pub shift_pressed: extern "C" fn() -> bool,
    /// `_settings_game.construction.command_pause_level`.
    pub pause_level: extern "C" fn() -> u8,
    /// `Company::GetIfValid(company)->last_build_coordinate = tile`.
    pub last_build_coordinate: extern "C" fn(u8, u32),
    /// `STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY`.
    pub not_enough_cash: u32,
    /// `ShowErrorMessage` with `STR_ERROR_NOT_ALLOWED_WHILE_PAUSED` at pixel coordinates.
    pub paused_error: extern "C" fn(u32, i32, i32),
    /// `ShowErrorMessage` for a failed result at pixel coordinates.
    pub error_message: extern "C" fn(u32, i32, i32, *const CommandCost),
    /// `ShowEstimatedCostOrIncome`.
    pub estimate: extern "C" fn(i64, i32, i32),
}

impl CommandServices {
    /// Ownership checks require a live borrowed native command result.
    pub(crate) unsafe fn ownership(&self, out: *mut CommandCost, owner: u8, tile: u32) {
        let company = unsafe { &*(self.company)() };
        crate::company::check_ownership(
            company,
            Cost(out, company.cost),
            crate::company_type::CompanyID(owner),
            crate::tile_type::TileIndex(tile),
        );
    }
    /// Tile ownership checks also require a live canonical map interval.
    pub(crate) unsafe fn tile_ownership(&self, out: *mut CommandCost, tile: u32) {
        let company = unsafe { &*(self.company)() };
        crate::company::check_tile_ownership(
            company,
            Cost(out, company.cost),
            crate::tile_type::TileIndex(tile),
        );
    }
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
impl ByteSpan {
    /// The viewed bytes; an empty span may carry a null pointer.
    ///
    /// # Safety
    /// The span denotes live readable bytes for the returned lifetime.
    pub(crate) unsafe fn bytes<'a>(self) -> &'a [u8] {
        if self.size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
    /// Byte equality of two live spans, as `std::string` or `std::string_view` `==`.
    pub(crate) fn same_bytes(self, other: Self) -> bool {
        self.size == other.size
            && (self.size == 0
                // SAFETY: Both spans denote live readable bytes for the producing entry.
                || unsafe {
                    std::slice::from_raw_parts(self.data, self.size)
                        == std::slice::from_raw_parts(other.data, other.size)
                })
    }
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
    /// Native `GetAllCompatibleRailTypes`, including empty and all-bit masks.
    pub all_compatible_railtypes: extern "C" fn(u64) -> u64,
    /// Typed Rust post lifecycles and native argument codecs.
    pub command_post: extern "C" fn() -> &'static crate::command_post::Services,
    /// Complete synchronous movement owner; immutable process-lifetime table.
    pub movement: *const crate::movement::MovementServices,
    /// Canonical native tile owner write.
    pub set_tile_owner: extern "C" fn(u32, u8),
    /// `AutoslopeEnabled()`.
    pub autoslope: extern "C" fn() -> bool,
    /// Original native `bay_stop_direction` map observation.
    pub bay_stop_direction: extern "C" fn(u32) -> u8,
    /// Original native `bridge_axis` map observation.
    pub bridge_axis: extern "C" fn(u32) -> u8,
    /// Bridge specification speed at this bridge tile.
    pub bridge_speed: extern "C" fn(u32) -> u16,
    /// `construction.build_on_slopes`.
    pub build_on_slopes: extern "C" fn() -> bool,
    /// Native leaves for direct shared Rust clear-square control.
    pub clear_square_services: ClearSquareServices,
    /// Whether this company pool ID is a live human company.
    pub company_human: extern "C" fn(u8) -> bool,
    /// Canonical live infrastructure of a source-valid native company.
    pub company_infrastructure:
        extern "C" fn(u8) -> *mut crate::company_base::CompanyInfrastructure,
    /// Whether this company pool ID is live.
    pub company_valid: extern "C" fn(u8) -> bool,
    /// Opaque context for test fixtures; production leaves ignore it.
    pub context: *mut c_void,
    /// Canonical `ShowCostOrIncomeAnimation` presentation leaf.
    pub cost_animation: extern "C" fn(i32, i32, i32, i64),
    /// Original `GetCrossingRoadBits`.
    pub crossing_road_bits: extern "C" fn(u32) -> RoadBits,
    /// Current native command actor, read at the original short-circuit point.
    pub current_company: extern "C" fn() -> u8,
    /// `GetDepotIndex`.
    pub depot_index: extern "C" fn(u32) -> u16,
    /// Shared live disaster sound setting.
    pub disaster_sound: extern "C" fn() -> bool,
    /// Original native `drive_through_axis` map observation.
    pub drive_through_axis: extern "C" fn(u32) -> u8,
    /// Original pathfinding setting, observed at the calling branch.
    pub forbid_90_deg: extern "C" fn() -> bool,
    /// `FreeTrainTrackReservation(v)`.
    pub free_train_reservation: unsafe extern "C" fn(*const Train),
    /// Original native `has_road_type` map observation.
    pub has_road_type: extern "C" fn(u32) -> bool,
    /// Original native `has_water_ground` map observation.
    pub has_water_ground: extern "C" fn(u32) -> bool,
    /// `TileVirtXY` query: 0 not industry, 1 industry, 2 bubble catcher; writes tile.
    pub industry: extern "C" fn(i32, i32, *mut u32) -> u32,
    /// Original native `is_bay_stop` map observation.
    pub is_bay_stop: extern "C" fn(u32) -> bool,
    /// Original native `is_bridge` map observation.
    pub is_bridge: extern "C" fn(u32) -> bool,
    /// Original native `is_drive_through_stop` map observation.
    pub is_drive_through_stop: extern "C" fn(u32) -> bool,
    /// Original native `is_plain_rail` map observation.
    pub is_plain_rail: extern "C" fn(u32) -> bool,
    /// Original native `is_road_depot` map observation.
    pub is_road_depot: extern "C" fn(u32) -> bool,
    /// `IsWaterTile`: water category and clear subtype (sea, canal or river).
    pub is_water_tile: extern "C" fn(u32) -> bool,
    /// Canonical `Tile` and `Map` statics, loaded at each observation.
    pub map: *const crate::map_func::MapStatics,
    /// Shared addresses of canonical settings; field values are loaded at their source branches.
    pub settings: *const crate::settings_type::SettingAddresses,
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
    /// Original native `other_tunnel_end` map observation.
    pub other_tunnel_end: extern "C" fn(u32) -> u32,
    /// Shared live plane crashes observation.
    pub plane_crashes: extern "C" fn() -> u8,
    /// Shared live plane speed observation.
    pub plane_speed: extern "C" fn() -> u8,
    /// Native `RoadTypeInfo` powered mask, retaining `GetRoadTypeInfo`'s assertion.
    pub powered_roadtypes: extern "C" fn(RoadType) -> u64,
    /// One shared game Random draw, including debug logging.
    pub random: extern "C" fn(*mut c_void) -> u32,
    /// Original `GetAnyRoadBits`, parameterized by `RoadTramType`.
    pub road_bits: extern "C" fn(u32, u8, bool) -> u8,
    /// Only the road type's construction price multiplier.
    pub road_cost_multiplier: extern "C" fn(u8) -> u16,
    /// Original native `road_depot_direction` map observation.
    pub road_depot_direction: extern "C" fn(u32) -> u8,
    /// Original native `road_type` map observation.
    pub road_type: extern "C" fn(u32) -> u8,
    /// Canonical native `HasCrossingReservation`.
    pub crossing_reserved: extern "C" fn(u32) -> bool,
    /// Canonical native `SetCrossingReservation`.
    pub set_crossing_reserved: extern "C" fn(u32, bool),
    /// Original `MayHaveRoad`, including road-stop and transport guards.
    pub may_have_road: extern "C" fn(u32) -> bool,
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
    /// Native record deletion and canonical configured height address.
    pub snow_line_support: crate::snow_line::Support,
    /// `GetStationIndex`.
    pub station_index: extern "C" fn(u32) -> u16,
    /// `Station::Get(station)->had_vehicle_of_type`, as its base byte.
    pub station_had_vehicle_of_type: extern "C" fn(u16) -> u8,
    /// `Station::Get(station)->had_vehicle_of_type |= type`.
    pub station_add_vehicle_of_type: extern "C" fn(u16, u8),
    /// `Station::Get(station)->xy`.
    pub station_xy: extern "C" fn(u16) -> u32,
    /// `BaseStation::Get(station)->xy`.
    pub base_station_xy: extern "C" fn(u16) -> u32,
    /// `Station::Get(station)->facilities`, as its base byte.
    pub station_facilities: extern "C" fn(u16) -> u8,
    /// Canonical shared `docking_tile` service.
    pub road_commands: extern "C" fn() -> *const crate::road_cmd::RoadCommandServices,
    /// Stateless native STL median selection and strict partition over split keys.
    pub kdtree_partition: extern "C" fn(*mut crate::kdtree::SplitKey<u16>, usize) -> usize,
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
    /// Persistent storage backup release on leaving command or test mode.
    pub storage_discard: extern "C" fn(*mut std::ffi::c_void),
    /// Canonical shared `tile_animation_add` service.

    /// Canonical shared `tile_animation_delete` service.

    /// Canonical shared `tile_animation_set_frame` service.
    pub tile_animation_set_frame: extern "C" fn(u32, u8),
    /// Canonical shared `town_next_id` service.
    pub town_next_id: extern "C" fn(u32) -> u32,
    /// Canonical native town shell resolver.
    pub town_by_id: extern "C" fn(u32) -> *mut TownHandle,
    /// Canonical shared `town_owner_by_id` service.
    pub town_owner_by_id: extern "C" fn(u32) -> *mut crate::town::State,
    /// Canonical shared `utf8_length` service.
    pub utf8_length: extern "C" fn(*const u8, usize) -> usize,
    /// Canonical shared `vehicle_next` service.
    pub vehicle_next: extern "C" fn(u32) -> Option<crate::vehicle::VehicleRef>,
    /// Whether a water tile is the middle of a lock.
    pub water_middle_lock: extern "C" fn(u32) -> bool,
    /// Original native `tile_max_z` map observation.
    pub tile_max_z: extern "C" fn(u32) -> i32,
    /// `GetTileOwner`.
    pub tile_owner: extern "C" fn(u32) -> u8,
    /// `GetTileSlope`.
    pub tile_slope: extern "C" fn(u32) -> u8,
    /// `GetTileType`.
    pub tile_type: extern "C" fn(u32) -> u8,
    /// Original native `tile_valid` map observation.
    pub tile_valid: extern "C" fn(u32) -> bool,
    /// `GetTileZ`.
    pub tile_z: extern "C" fn(u32) -> i32,
    /// Original native `town_index` map observation.
    pub town_index: extern "C" fn(u32) -> u16,
    /// Resolve the single Rust owner of a live native town shell.
    pub town_owner: extern "C" fn(*const TownHandle) -> *mut crate::town::State,
    /// Original train track direction, computed lazily at the consuming branch.
    pub train_trackdir: unsafe extern "C" fn(*const Train) -> Trackdir,
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
    /// Native road stop shells and their vehicle reads.
    pub road_stops: extern "C" fn() -> *const crate::road_stop::RoadStopServices,
    /// `GetNearbyTile` with signed offsets.
    pub nearby_tile: extern "C" fn(u8, u32) -> u32,
    /// `GetNearbyTileInformation`.
    pub nearby_tile_information: extern "C" fn(u32, bool) -> u32,
    /// Native client-info pool lookup; null when the ID is unknown.
    pub network_client_info: extern "C" fn(u32) -> *mut c_void,
    /// `MarkWholeScreenDirty`.
    pub screen_dirty: extern "C" fn(),
    /// `std::string` at `ptr`: `data()`, with `size()` in `length`.
    pub string_data: extern "C" fn(ptr: *const c_void, length: *mut usize) -> *const u8,
    /// `GetTerrainType`.
    pub terrain_type: extern "C" fn(u32) -> u32,
    /// `AppendCopyableBadgeList` over readable spans; output holds their combined capacity.
    pub copyable_badges: extern "C" fn(
        crate::newgrf_specs::BadgeSpan,
        crate::newgrf_specs::BadgeSpan,
        u8,
        *mut u32,
    ) -> usize,
    /// `GRFFile::grf_version` of a non-null file.
    pub grf_file_version: extern "C" fn(*const c_void) -> u8,
    /// `RustGRFPriceShift`: the live file multiplier, including its null-file default.
    pub grf_price_shift: extern "C" fn(*const c_void, u32) -> i32,
    /// `GRFFile::cargo_map` entry of a non-null file.
    pub grf_cargo_map: extern "C" fn(*const c_void, u8) -> u8,
    /// `ConvertBooleanCallback` for a non-null file.
    pub grf_boolean_callback: extern "C" fn(*const c_void, u16, u16) -> bool,
    /// Loaded GRF file at an index, or null past the end.
    pub grf_file_at_or_null: extern "C" fn(usize) -> *mut c_void,
    /// `GRFFile::grfid` of a non-null file.
    pub grf_file_id: extern "C" fn(*const c_void) -> u32,
    /// `GRFFile::filename` of a non-null file, live while the file is.
    pub grf_file_name: extern "C" fn(*const c_void) -> ByteSpan,
    /// Canonical native `GRFFile::GetParam` shared by loaders and resolvers.
    pub grf_parameter: extern "C" fn(*const c_void, u32) -> u32,
    /// `_rust_newgrf_specs`; read through `specs`.
    pub newgrf_specs: *const crate::newgrf_specs::Specs,
    /// `ResolverObject::grffile`.
    pub resolver_grf_file: extern "C" fn(*const c_void) -> *const c_void,
    /// `GetBadgeVariableResult` for the resolver's file and a badge list.
    pub resolver_badges: extern "C" fn(*const c_void, *const u32, usize, u32) -> u32,
    /// `GRFFile::grfid` of the resolver's non-null file.
    pub resolver_grf_file_id: extern "C" fn(*const c_void) -> u32,
    /// `GRFFile::grf_version` of the resolver's non-null file.
    pub resolver_grf_version: extern "C" fn(*const c_void) -> u8,
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
    /// Canonical `Map::LogY`, including before the first allocation.
    pub(crate) fn map_log_y(&self) -> u32 {
        unsafe { (*self.map).log_y.read() }
    }
    /// Canonical `Map::Size()`, loaded at the observation point.
    pub(crate) fn map_size(&self) -> u32 {
        unsafe { (*self.map).size.read() }
    }
    /// Immutable address catalog; its targets are read with field-sized raw loads.
    pub(crate) fn settings(&self) -> &crate::settings_type::SettingAddresses {
        unsafe { &*self.settings }
    }

    /// Typed `NewGRF` specification leaves.
    #[must_use]
    pub fn specs(&self) -> &crate::newgrf_specs::Specs {
        // SAFETY: `GetRustSharedServices` points at the process-lifetime `_rust_newgrf_specs`.
        #[allow(unsafe_code)]
        unsafe {
            &*self.newgrf_specs
        }
    }
    /// Capture `Map::LogX()` and native `TileAdd` policy for a synchronous entry.
    #[must_use]
    pub fn map(&self) -> Map {
        // SAFETY: The immutable table and canonical statics outlive the entry;
        // map reset/load/mutex yield is excluded while its dimensions are used.
        unsafe { (*self.map).dimensions() }
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
    "OpenTTDMapStatics" ["landscape"] => crate::map_func::MapStatics {
        base,
        extended,
        size,
        log_x,
        log_y,
        assertions,
        debug,
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
    "OpenTTDRailStationAdmission" ["probe"] => RailStationAdmission {
        rail,
        owner,
        axis,
        blocked,
    };
    "OpenTTDRailWormholeAdmission" ["probe"] => RailWormholeAdmission {
        owner,
        transport,
        direction,
    };
    "OpenTTDRailMapServices" ["probe"] => RailMapServices {
        ground,
        traversal,
        station_admission,
        wormhole_admission,
        set_signal_state,
        other_tunnel_bridge_end,
        invalid_rail_type,
        set_rail_type,
        set_track_bits,
        set_present_signals,
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
        wormhole_reserved,
    };
    "OpenTTDCommandServices" ["probe"] => crate::services::CommandServices {
        command_flags, command_type, command_valid,
        finance,
        infinite_money,
        money_error,
        infrastructure_dirty,
        landscape_commands,
        company,
        occupancy,
        shared,
        cost,
        rail_map,
        signals,
        shift_pressed,
        pause_level,
        last_build_coordinate,
        not_enough_cash,
        paused_error,
        error_message,
        estimate,
    };
    "OpenTTDCommandAdmission" ["probe"] => crate::command::Admission {
        rejected,
        estimate_only,
        only_sending,
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
        encode_owned_by,
        error_owner,
    };
    "OpenTTDClearSquareServices" ["probe"] => ClearSquareServices {


        docking,
        generating_world,
    };
    "OpenTTDSharedServices" ["probe"] => crate::services::Services {
        all_compatible_railtypes,
        command_post,
        movement,
        set_tile_owner,
        autoslope,
        bay_stop_direction,
        bridge_axis,
        bridge_speed,
        build_on_slopes,
        clear_square_services,
        company_human,
        company_infrastructure,
        company_valid,
        context,
        cost_animation,
        crossing_road_bits,
        current_company,
        depot_index,
        disaster_sound,
        drive_through_axis,
        forbid_90_deg,
        free_train_reservation,
        has_road_type,
        has_water_ground,
        industry,
        is_bay_stop,
        is_bridge,
        is_drive_through_stop,
        is_plain_rail,
        is_road_depot,
        is_water_tile,
        map,
        settings,
        map_max_x,
        map_max_y,
        mark_bridge_dirty,
        mark_dirty,
        next_company,
        no_jetcrash,
        other_tunnel_end,
        plane_crashes,
        plane_speed,
        powered_roadtypes,
        random,
        road_bits,
        road_cost_multiplier,
        road_depot_direction,
        road_type,
        crossing_reserved,
        set_crossing_reserved,
        may_have_road,
        set_road_bits,
        set_road_type,
        set_road_owner,
        set_road_directions,
        road_powered,
        type_availability,
        pbs_services,
        service_at_helipad,
        set_tropic_zone,
        snow_line_support,
        station_index,
        station_had_vehicle_of_type,
        station_add_vehicle_of_type,
        station_xy,
        base_station_xy,
        station_facilities,
        road_commands,
        kdtree_partition,
        station_commands,
        station_catchment,
        docking_tile,
        station_gfx,
        station_serve_neutral,
        station_type,
        storage_discard,

        tile_animation_set_frame,
        town_next_id,
        town_by_id,
        town_owner_by_id,
        utf8_length,
        vehicle_next,
        water_middle_lock,
        tile_max_z,
        tile_owner,
        tile_slope,
        tile_type,
        tile_valid,
        tile_z,
        town_index,
        town_owner,
        train_trackdir,
        tram_type,
        tropic_zone,
        try_path_reserve,
        tunnel_bridge_direction,
        tunnel_transport,
        update_level_crossing,
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
        road_stops,
        nearby_tile,
        nearby_tile_information,
        network_client_info,
        screen_dirty,
        string_data,
        terrain_type,
        copyable_badges,
        grf_file_version,
        grf_price_shift,
        grf_cargo_map,
        grf_boolean_callback,
        grf_file_at_or_null,
        grf_file_id,
        grf_file_name,
        grf_parameter,
        newgrf_specs,
        resolver_grf_file,
        resolver_badges,
        resolver_grf_file_id,
        resolver_grf_version,
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

#[cfg(test)]
extern "C" fn unexpected_station_visits(_: u16) -> u8 {
    unreachable!()
}
#[cfg(test)]
extern "C" fn unexpected_station_visit(_: u16, _: u8) {
    unreachable!()
}
#[cfg(test)]
extern "C" fn unexpected_station_field(_: u16) -> u32 {
    unreachable!()
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
    unsafe extern "C" fn unused_snow_line_destroy(_: *mut crate::snow_line::SnowLine) {
        unreachable!()
    }
    extern "C" fn unexpected_parameter(_: *const c_void, _: u32) -> u32 {
        unreachable!()
    }
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
    extern "C" fn set_reserved(_: u32, _: bool) {}
    extern "C" fn set_three(_: u32, _: u8, _: u8) {}
    extern "C" fn two_types(_: u8, _: u8) -> bool {
        false
    }
    extern "C" fn unexpected_error(_: u32, _: u32) {
        std::process::abort()
    }
    extern "C" fn unexpected_path() {
        std::process::abort()
    }
    extern "C" fn unexpected_string_data(_: *const c_void, _: *mut usize) -> *const u8 {
        std::process::abort()
    }
    extern "C" fn unexpected_client_info(_: u32) -> *mut c_void {
        std::process::abort()
    }
    extern "C" fn unexpected_nearby(_: u8, _: u32) -> u32 {
        std::process::abort()
    }
    extern "C" fn unexpected_information(_: u32, _: bool) -> u32 {
        std::process::abort()
    }
    extern "C" fn unexpected_terrain(_: u32) -> u32 {
        std::process::abort()
    }
    extern "C" fn unexpected_version(_: *const c_void) -> u8 {
        std::process::abort()
    }
    extern "C" fn unexpected_price_shift(_: *const c_void, _: u32) -> i32 {
        std::process::abort()
    }
    extern "C" fn unexpected_cargo_map(_: *const c_void, _: u8) -> u8 {
        std::process::abort()
    }
    extern "C" fn unexpected_boolean(_: *const c_void, _: u16, _: u16) -> bool {
        std::process::abort()
    }
    extern "C" fn unexpected_badges(_: *const c_void, _: *const u32, _: usize, _: u32) -> u32 {
        std::process::abort()
    }
    extern "C" fn unexpected_file_at(_: usize) -> *mut c_void {
        std::process::abort()
    }
    extern "C" fn unexpected_file_id(_: *const c_void) -> u32 {
        std::process::abort()
    }
    extern "C" fn unexpected_file_name(_: *const c_void) -> ByteSpan {
        std::process::abort()
    }
    extern "C" fn unexpected_file(_: *const c_void) -> *const c_void {
        std::process::abort()
    }
    extern "C" fn unexpected_compatible(_: u64) -> u64 {
        unreachable!()
    }
    extern "C" fn unexpected_powered(_: RoadType) -> u64 {
        unreachable!()
    }
    extern "C" fn unexpected_post() -> &'static crate::command_post::Services {
        std::process::abort()
    }
    extern "C" fn unexpected_copyable(
        _: crate::newgrf_specs::BadgeSpan,
        _: crate::newgrf_specs::BadgeSpan,
        _: u8,
        _: *mut u32,
    ) -> usize {
        std::process::abort()
    }
    Services {
        copyable_badges: unexpected_copyable,
        all_compatible_railtypes: unexpected_compatible,
        command_post: unexpected_post,
        grf_file_version: unexpected_version,
        grf_price_shift: unexpected_price_shift,
        grf_cargo_map: unexpected_cargo_map,
        grf_boolean_callback: unexpected_boolean,
        grf_file_at_or_null: unexpected_file_at,
        grf_file_id: unexpected_file_id,
        grf_file_name: unexpected_file_name,
        grf_parameter: unexpected_parameter,
        newgrf_specs: std::ptr::null(),
        resolver_grf_file: unexpected_file,
        resolver_badges: unexpected_badges,
        resolver_grf_file_id: unexpected_file_id,
        resolver_grf_version: unexpected_version,
        nearby_tile: unexpected_nearby,
        nearby_tile_information: unexpected_information,
        network_client_info: unexpected_client_info,
        screen_dirty: unexpected_path,
        string_data: unexpected_string_data,
        terrain_type: unexpected_terrain,
        show_error: unexpected_error,
        not_reached: unexpected_path,
        movement: std::ptr::null(),
        set_tile_owner: zone,
        autoslope: flag0,
        bay_stop_direction: byte,
        bridge_axis: byte,
        bridge_speed: short,
        build_on_slopes: flag0,
        clear_square_services: ClearSquareServices {
            docking: flag,
            generating_world: flag0,
        },
        company_human: unused_company_valid,
        company_infrastructure: unused_company_infrastructure,
        company_valid: unused_company_valid,
        context,
        cost_animation: fixture_cost_animation,
        crossing_road_bits: fixture_crossing_road_bits,
        current_company: unused_current_company,
        depot_index: short,
        disaster_sound: flag0,
        drive_through_axis: byte,
        forbid_90_deg: flag0,
        free_train_reservation: train,
        has_road_type: flag,
        has_water_ground: flag,
        industry,
        is_bay_stop: flag,
        is_bridge: flag,
        is_drive_through_stop: flag,
        is_plain_rail: flag,
        is_road_depot: flag,
        is_water_tile: flag,
        map: crate::map_func::fixture::statics(),
        settings: std::ptr::null(),
        map_max_x: word0,
        map_max_y: word0,
        mark_bridge_dirty: tile,
        mark_dirty: tile,
        next_company: unused_next_company,
        no_jetcrash: flag0,
        other_tunnel_end: word,
        plane_crashes: zero_byte,
        plane_speed: zero_byte,
        powered_roadtypes: unexpected_powered,
        random,
        road_bits: bits,
        road_cost_multiplier: unused_road_cost_multiplier,
        road_depot_direction: byte,
        road_type: byte,
        crossing_reserved: flag,
        set_crossing_reserved: set_reserved,
        may_have_road: flag,
        set_road_bits: set_three,
        set_road_type: set_three,
        set_road_owner: set_three,
        set_road_directions: zone,
        road_powered: two_types,
        type_availability: fixture_type_availability,
        pbs_services: train_safe,
        service_at_helipad: flag0,
        set_tropic_zone: zone,
        snow_line_support: crate::snow_line::Support {
            height: &0,
            destroy: unused_snow_line_destroy,
        },
        station_index: short,
        station_had_vehicle_of_type: unexpected_station_visits,
        station_add_vehicle_of_type: unexpected_station_visit,
        station_xy: unexpected_station_field,
        base_station_xy: unexpected_station_field,
        station_facilities: unexpected_station_visits,
        road_commands: fixture_road_commands,
        kdtree_partition: fixture_kdtree_partition,
        station_commands: fixture_station_commands,
        station_catchment: fixture_station_catchment,
        docking_tile: flag,
        station_gfx: byte,
        station_serve_neutral: flag0,
        station_type: fixture_station_type,
        storage_discard: fixture_storage_discard,

        tile_animation_set_frame: zone,
        town_next_id: word,
        town_by_id: fixture_town_by_id,
        town_owner_by_id: unused_town_owner_by_id,
        utf8_length: unused_utf8_length,
        vehicle_next: unused_vehicle_next,
        water_middle_lock: flag,
        tile_max_z: height,
        tile_owner: byte,
        tile_slope: byte,
        tile_type: byte,
        tile_valid: flag,
        tile_z: height,
        town_index: index,
        town_owner: unused_town_owner,
        train_trackdir: fixture_train_trackdir,
        tram_type: fixture_tram_type,
        tropic_zone: byte,
        try_path_reserve: train_path,
        tunnel_bridge_direction: byte,
        tunnel_transport: byte,
        update_level_crossing: fixture_crossing_update,
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
        road_stops: fixture_road_stops,
    }
}
#[cfg(test)]
extern "C" fn fixture_tram_type(_: u32) -> RoadType {
    RoadType(0)
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

/// Native `Axis` with its unchanged uint8 representation.
pub use crate::direction_type::Axis;
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
    pub station_admission: extern "C" fn(u32, u8, u8) -> RailStationAdmission,
    /// Consumed source-phase observation; guarded fields are not hoisted.
    pub wormhole_admission: extern "C" fn(u32, u8) -> RailWormholeAdmission,

    /// Native `set_signal_state` map operation at the original observation point.
    pub set_signal_state: extern "C" fn(u32, u8, u8) -> (),
    /// Native `other_tunnel_bridge_end` map operation at the original observation point.
    pub other_tunnel_bridge_end: extern "C" fn(u32) -> u32,
    /// Native `INVALID_RAILTYPE`.
    pub invalid_rail_type: extern "C" fn() -> RailType,
    /// Native `SetRailType`.
    pub set_rail_type: extern "C" fn(u32, u8),
    /// Native `SetTrackBits`.
    pub set_track_bits: extern "C" fn(u32, u8),
    /// Native `SetPresentSignals`.
    pub set_present_signals: extern "C" fn(u32, u8),
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
        crate::company::check_tile_ownership(
            unsafe { &*(command.company)() },
            self,
            crate::tile_type::TileIndex(tile),
        );
    }
    /// `CheckOwnership`.
    pub(crate) fn ownership(self, command: &CommandServices, owner: u8, tile: u32) {
        // SAFETY: The impl contract above.
        crate::company::check_ownership(
            unsafe { &*(command.company)() },
            self,
            crate::company_type::CompanyID(owner),
            crate::tile_type::TileIndex(tile),
        );
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

#[cfg(test)]
extern "C" fn fixture_road_stops() -> *const crate::road_stop::RoadStopServices {
    panic!("unused road stop table")
}
#[cfg(test)]
extern "C" fn fixture_storage_discard(_: *mut std::ffi::c_void) {}
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

#[cfg(test)]
extern "C" fn fixture_road_commands() -> *const crate::road_cmd::RoadCommandServices {
    panic!("unused road commands")
}

#[cfg(test)]
extern "C" fn fixture_type_availability() -> *const crate::type_availability::Services {
    std::process::abort()
}

#[cfg(test)]
extern "C" fn fixture_town_by_id(_: u32) -> *mut TownHandle {
    panic!("unused town shell")
}

#[cfg(test)]
extern "C" fn fixture_kdtree_partition(_: *mut crate::kdtree::SplitKey<u16>, _: usize) -> usize {
    unreachable!()
}

#[cfg(test)]
extern "C" fn fixture_cost_animation(_: i32, _: i32, _: i32, _: i64) {
    panic!("unused cost animation")
}

/// Fixture leaf for entries that do not observe company infrastructure.
#[cfg(test)]
pub(crate) extern "C" fn unused_company_infrastructure(
    _: u8,
) -> *mut crate::company_base::CompanyInfrastructure {
    panic!("unexpected company infrastructure access")
}
