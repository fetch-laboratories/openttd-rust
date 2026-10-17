/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Rail track, depot and signal commands and the rail clear/owner/terraform/
//! build-above tile procs (#265). Rust owns control flow; map, pools, company
//! counters and every `CommandCost` stay canonical native storage. Nested
//! commands are ordinary native dispatches that may reenter this module: no
//! Rust borrow of native state exists, so every callback is a direct call.
//! No entry draws Random. Native leaves are noexcept; panics abort.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::fn_params_excessive_bools
)]
pub use crate::services::Train;
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Map, RailMapServices, Services,
};
use crate::witness as w;

/// Opaque caller-owned native frame holding the conversion area iterator and
/// the ordered affected-train list of `CmdConvertRail`.
#[repr(C)]
pub struct ConvertFrame {
    _private: [u8; 0],
}

/// Native `Foundation` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Foundation(pub u8);
/// Native `FloodingBehaviour` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct FloodingBehaviour(pub u8);

/// `ApplyFoundationToSlope` result: height increase and the modified slope.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SlopeZ {
    /// Height added by the foundation.
    pub z: i32,
    /// Slope on top of the foundation.
    pub slope: u8,
}
/// Train position read only when the safe-waiting check runs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainPosition {
    /// `v->tile`.
    pub tile: u32,
    /// `v->GetVehicleTrackdir()`.
    pub trackdir: u8,
}
/// Train fields read together at the signal re-reservation branch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrainParking {
    /// `Train::GetTrack()`.
    pub track: u8,
    /// `vehstatus` bits.
    pub status: u8,
    /// `cur_speed`.
    pub speed: u16,
    /// `current_order` type.
    pub order: u8,
}
/// Call-local normal road observations, read at their original guards.
#[derive(Clone, Copy)]
pub struct NormalRoad {
    /// `GetRoadTypeRoad`.
    pub road_type: u8,
    /// `GetRoadTypeTram`.
    pub tram_type: u8,
    /// `GetRoadBits(RTT_ROAD)`.
    pub road_bits: u8,
    /// `GetRoadBits(RTT_TRAM)`.
    pub tram_bits: u8,
}
/// Level-crossing road fields read together when the crossing is removed.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrossingRoad {
    /// `GetCrossingRoadBits`.
    pub bits: u8,
    /// `GetRoadTypeRoad`.
    pub road_type: u8,
    /// `GetRoadTypeTram`.
    pub tram_type: u8,
    /// `GetTownIndex`.
    pub town: u16,
    /// `GetRoadOwner(RTT_ROAD)`.
    pub road_owner: u8,
    /// `GetRoadOwner(RTT_TRAM)`.
    pub tram_owner: u8,
}
/// Generated native `StringID` values used as command errors.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Strings {
    pub impossible_combination: u32,
    pub already_built: u32,
    pub build_on_water: u32,
    pub sloped_wrong: u32,
    pub remove_signals_first: u32,
    pub road_works: u32,
    pub oneway_road: u32,
    pub crossing_rail: u32,
    pub crossing_road: u32,
    pub no_track: u32,
    pub owned_by: u32,
    pub flat_land: u32,
    pub demolish_bridge: u32,
    pub no_suitable_track: u32,
    pub no_signals: u32,
    pub owned_by_another: u32,
    pub remove_track: u32,
    pub demolish_building: u32,
    pub ship_in_way: u32,
}

/// Immutable native service table. Every function is a direct `noexcept`
/// call; nested commands, reservations and crossing updates may reenter Rust
/// owners, which is safe because no Rust borrow of native state exists.
/// Pointer-taking leaves require live native cost slots, trains and conversion
/// frames under the entry contract; their function-pointer types are unsafe.
/// Private policy calls uphold that contract and retain only copied identities,
/// never native references, through destructive or reentrant services.
#[repr(C)]
pub struct RailCommandServices {
    pub shared: *const Services,
    pub rail: *const RailMapServices,
    pub cost: *const CommandCostServices,
    pub command: *const CommandServices,
    /// Signal-buffer occupancy and assertion services (`_signal_services`).
    pub signal: *const crate::signal::SignalServices,
    pub strings: Strings,
    /// `ValParamRailType`.
    pub rail_type_valid: extern "C" fn(u8) -> bool,
    /// `IsCompatibleRail`.
    pub compatible_rail: extern "C" fn(u8, u8) -> bool,
    /// `HasPowerOnRail(RailType, RailType)`.
    pub powered_rail: extern "C" fn(u8, u8) -> bool,
    /// `HasPowerOnRail(v->GetRailTypes(), RailType)`.
    pub train_powered: unsafe extern "C" fn(*mut Train, u8) -> bool,
    /// `RailNoLevelCrossings`.
    pub rail_no_crossings: extern "C" fn(u8) -> bool,
    /// `RoadNoLevelCrossing`.
    pub road_no_crossing: extern "C" fn(u8) -> bool,
    /// `RailBuildCost`.
    pub rail_build_cost: extern "C" fn(u8) -> i64,
    /// `RailClearCost`.
    pub rail_clear_cost: extern "C" fn(u8) -> i64,
    /// `RailConvertCost`.
    pub rail_convert_cost: extern "C" fn(u8, u8) -> i64,
    /// `RoadBuildCost`.
    pub road_build_cost: extern "C" fn(u8) -> i64,
    /// `construction.crossing_with_competitor`.
    pub crossing_with_competitor: extern "C" fn() -> bool,
    /// `vehicle.disable_elrails`.
    pub disable_elrails: extern "C" fn() -> bool,
    /// `GetRailFoundation`.
    pub rail_foundation: extern "C" fn(u8, u8) -> Foundation,
    /// `ApplyFoundationToSlope`.
    pub apply_foundation: extern "C" fn(u8, u8) -> SlopeZ,
    /// `GetSlopeZInCorner`.
    pub slope_z_in_corner: extern "C" fn(u8, u8) -> i32,
    /// `GetFloodingBehaviour`.
    pub flooding: extern "C" fn(u32) -> FloodingBehaviour,
    /// `AutoslopeCheckForEntranceEdge`.
    pub entrance_edge: extern "C" fn(u32, i32, u8, u8) -> bool,
    /// `HasVehicleOnTile(tile, ship)`.
    pub ship_on_tile: extern "C" fn(u32) -> bool,
    /// `CMD_BUILD_SINGLE_RAIL`.
    pub build_rail: unsafe extern "C" fn(*mut CommandCost, u16, u32, u8, u8, bool),
    /// `CMD_REMOVE_SINGLE_RAIL`.
    pub remove_rail: unsafe extern "C" fn(*mut CommandCost, u16, u32, u8),
    /// `CMD_BUILD_SINGLE_SIGNAL`.
    pub build_signal: unsafe extern "C" fn(
        *mut CommandCost,
        u16,
        u32,
        u8,
        u8,
        u8,
        bool,
        bool,
        bool,
        u8,
        u8,
        u8,
        u8,
    ),
    /// `CMD_REMOVE_SINGLE_SIGNAL`.
    pub remove_signal: unsafe extern "C" fn(*mut CommandCost, u16, u32, u8),
    /// `CMD_CONVERT_RAIL`.
    pub convert_rail: unsafe extern "C" fn(*mut CommandCost, u16, u32, u32, u8, bool),
    /// `infrastructure.rail[rt] += delta` with uint32 wrap.
    pub rail_infrastructure: extern "C" fn(u8, u8, u32),
    /// `infrastructure.road[rt] += delta` with uint32 wrap.
    pub road_infrastructure: extern "C" fn(u8, u8, u32),
    /// `infrastructure.signal += delta` with uint32 wrap.
    pub signal_infrastructure: extern "C" fn(u8, u32),
    /// `DirtyCompanyInfrastructureWindows`.
    pub infrastructure_dirty: extern "C" fn(u8),
    /// `GetTrainForReservation`.
    pub reserving_train: extern "C" fn(u32, u8) -> *mut Train,
    /// Train fields read at the signal re-reservation branch.
    pub parking: unsafe extern "C" fn(*mut Train) -> TrainParking,
    /// `v->tile` and `v->GetVehicleTrackdir()`, read only for the safe-waiting check.
    pub train_position: unsafe extern "C" fn(*mut Train) -> TrainPosition,
    /// `YapfNotifyTrackLayoutChange`.
    pub layout_changed: extern "C" fn(u32, u8),
    /// `MarkDirtyAdjacentLevelCrossingTiles`.
    pub mark_crossings: extern "C" fn(u32, u8),
    /// `UpdateAdjacentLevelCrossingTilesOnLevelCrossingRemoval`.
    pub crossing_removed: extern "C" fn(u32, u8),
    /// `MakeRoadCrossing(tile, road, tram, rail, road_axis, railtype, road_rt, tram_rt, town)`.
    pub make_crossing: extern "C" fn(u32, u8, u8, u8, u8, u8, u8, u8, u16),
    /// `MakeRoadNormal(tile, bits, road_rt, tram_rt, town, road_owner, tram_owner)`.
    pub make_road: extern "C" fn(u32, CrossingRoad),
    /// `Depot::CanAllocateItem`.
    pub depot_available: extern "C" fn() -> bool,
    /// `new Depot(tile)`, returning its ID.
    pub new_depot: extern "C" fn(u32) -> u16,
    /// `MakeDefaultName(Depot::Get(id))`.
    pub depot_name: extern "C" fn(u16),
    /// `delete Depot::GetByTile(tile)`.
    pub delete_depot: extern "C" fn(u32),
    /// `InvalidateWindowData` of the depot and build-vehicle windows.
    pub depot_windows: extern "C" fn(u32),
    /// `DeleteNewGRFInspectWindow(GSF_RAILTYPES, tile)`.
    pub close_inspect: extern "C" fn(u32),
    /// `MakeShore`, keeping the docking flag as the original does.
    pub make_shore: extern "C" fn(u32),
    /// `IsPossibleDockingTile` then `CheckForDockingTile`.
    pub docking_check: extern "C" fn(u32),
    /// Begin `TileIterator::Create(start, end, diagonal)` in the frame.
    pub area_begin: unsafe extern "C" fn(*mut ConvertFrame, u32, u32, bool),
    /// Current iterator tile (`INVALID_TILE` at end).
    pub area_tile: unsafe extern "C" fn(*mut ConvertFrame) -> u32,
    /// Advance the iterator.
    pub area_next: unsafe extern "C" fn(*mut ConvertFrame),
    /// `DiagonalTileArea`/`OrthogonalTileArea(start, end).Contains(tile)`.
    pub area_contains: extern "C" fn(u32, u32, bool, u32) -> bool,
    /// `include(affected, first)` for every train on the tile, in iteration order.
    pub collect_trains: unsafe extern "C" fn(*mut ConvertFrame, u32),
    /// `ConsistChanged(CCF_TRACK)` for each collected train, in insertion order.
    pub consist_changed: unsafe extern "C" fn(*mut ConvertFrame),
}

const INVALID_STRING: u32 = 0xFFFF;
const CONSTRUCTION: u8 = 0;
const EXECUTE: u16 = 1 << 0;
const AUTO: u16 = 1 << 1;
const BANKRUPT: u16 = 1 << 5;
const INVALID_TILE: u32 = u32::MAX;

const MP_RAILWAY: u8 = 1;
const MP_ROAD: u8 = 2;
const MP_STATION: u8 = 5;
const MP_WATER: u8 = 6;
const MP_TUNNELBRIDGE: u8 = 9;

const OWNER_WATER: u8 = 0x11;
const OWNER_DEITY: u8 = 0x12;
const INVALID_OWNER: u8 = 0xFF;

const TRACK_X: u8 = 0;
const TRACK_Y: u8 = 1;
const INVALID_TRACK: u8 = 0xFF;
const BIT_X: u8 = 1;
const BIT_Y: u8 = 2;
const BIT_UPPER: u8 = 4;
const BIT_LOWER: u8 = 8;
const BIT_LEFT: u8 = 16;
const BIT_RIGHT: u8 = 32;
const BIT_HORZ: u8 = 12;
const BIT_VERT: u8 = 48;
const BIT_DEPOT: u8 = 0x80;
const INVALID_TRACKDIR: u8 = 0xFF;
const INVALID_DIAGDIR: u8 = 0xFF;

const SLOPE_FLAT: u8 = 0;
const SLOPE_W: u8 = 1;
const SLOPE_S: u8 = 2;
const SLOPE_E: u8 = 4;
const SLOPE_N: u8 = 8;
const SLOPE_STEEP: u8 = 0x10;
const SLOPE_EW: u8 = 5;
const SLOPE_NS: u8 = 10;
const SLOPE_HALFTILE_MASK: u8 = 0xE0;
/// `VALID_LEVEL_CROSSING_SLOPES`.
const CROSSING_SLOPES: u32 =
    (1 << 14) | (1 << 13) | (1 << 11) | (1 << 10) | (1 << 7) | (1 << 5) | 1;

const FOUNDATION_NONE: u8 = 0;
const FOUNDATION_RAIL_W: u8 = 10;
const FOUNDATION_RAIL_N: u8 = 13;
const FOUNDATION_INVALID: u8 = 0xFF;

const GROUND_BARREN: u8 = 0;
const GROUND_HALF_WATER: u8 = 13;
const RAIL_TILE_NORMAL: u8 = 0;
const RAIL_TILE_SIGNALS: u8 = 1;
const RAIL_TILE_DEPOT: u8 = 3;

const SIGTYPE_BLOCK: u8 = 0;
const SIGTYPE_ENTRY: u8 = 1;
const SIGTYPE_EXIT: u8 = 2;
const SIGTYPE_LAST: u8 = 5;
const SIG_ELECTRIC: u8 = 0;
const SIG_SEMAPHORE: u8 = 1;

const RAILTYPE_RAIL: u8 = 0;
const RAILTYPE_ELECTRIC: u8 = 1;
const INVALID_ROADTYPE: u8 = 63;
const ROAD_X: u8 = 10;
const ROAD_Y: u8 = 5;
const STOPPED: u8 = 1 << 1;
const OT_LOADING: u8 = 3;

const TILE_AXIAL_DISTANCE: i32 = 192;
const TILE_CORNER_DISTANCE: i32 = 128;
const LEVELCROSSING_TRACKBIT_FACTOR: u32 = 2;
const TUNNELBRIDGE_TRACKBIT_FACTOR: u32 = 4;

const PR_BUILD_SIGNALS: usize = 3;
const PR_BUILD_DEPOT_TRAIN: usize = 5;
const PR_CLEAR_ROUGH: usize = 23;
const PR_CLEAR_SIGNALS: usize = 28;
const PR_CLEAR_DEPOT_TRAIN: usize = 30;
const PR_CLEAR_WATER: usize = 34;
const PR_BUILD_FOUNDATION: usize = 57;

/// `_trackdelta`, indexed by trackdir.
const TRACK_DELTA: [(i32, i32); 16] = [
    (-1, 0),
    (0, 1),
    (-1, 0),
    (0, 1),
    (1, 0),
    (0, 1),
    (0, 0),
    (0, 0),
    (1, 0),
    (0, -1),
    (0, -1),
    (1, 0),
    (0, -1),
    (-1, 0),
    (0, 0),
    (0, 0),
];
/// `_valid_tracks_on_leveled_foundation`.
const LEVELED_TRACKS: [u8; 15] = [
    0,
    BIT_LEFT,
    BIT_LOWER,
    BIT_Y | BIT_LOWER | BIT_LEFT,
    BIT_RIGHT,
    63,
    BIT_X | BIT_LOWER | BIT_RIGHT,
    63,
    BIT_UPPER,
    BIT_X | BIT_UPPER | BIT_LEFT,
    63,
    63,
    BIT_Y | BIT_UPPER | BIT_RIGHT,
    63,
    63,
];
const SIGNAL_ALONG: [u8; 14] = [8, 8, 8, 2, 4, 1, 0, 0, 4, 4, 4, 1, 8, 2];
const SIGNAL_AGAINST: [u8; 14] = [4, 4, 4, 1, 8, 2, 0, 0, 8, 8, 8, 2, 4, 1];
const SIGNAL_ON_TRACK: [u8; 6] = [0xC, 0xC, 0xC, 3, 0xC, 3];
const TRACKDIR_EXIT: [u8; 14] = [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0];
/// `_exitdir_reaches_trackdirs`.
const EXIT_REACHES: [u16; 4] = [0x1009, 0x0016, 0x0520, 0x2a00];

fn tracks_overlap(bits: u8) -> bool {
    if bits == 0 || bits & bits.wrapping_sub(1) == 0 {
        return false;
    }
    bits != BIT_HORZ && bits != BIT_VERT
}
/// Infrastructure pieces of a plain rail tile: squared when tracks overlap.
fn pieces(bits: u8) -> u32 {
    let n = bits.count_ones();
    if tracks_overlap(bits) { n * n } else { n }
}
fn diagonal(trackdir: u8) -> bool {
    trackdir & 7 < 2
}
fn reaches(trackdir: u8) -> u16 {
    EXIT_REACHES[usize::from(TRACKDIR_EXIT[usize::from(trackdir)])]
}
fn trackdir_tracks(bits: u16) -> u8 {
    ((bits | (bits >> 8)) & 0x3F) as u8
}
fn is_pbs(sigtype: u8) -> bool {
    sigtype == 4 || sigtype == 5
}
fn kill_first_bit(value: u8) -> u8 {
    value & value.wrapping_sub(1)
}
fn steep(slope: u8) -> bool {
    slope & SLOPE_STEEP != 0
}
fn one_corner_raised(slope: u8) -> bool {
    matches!(slope, SLOPE_W | SLOPE_S | SLOPE_E | SLOPE_N)
}
/// Saturating unary minus of `OverflowSafeInt64`.
fn negate(money: i64) -> i64 {
    if money == i64::MIN { i64::MAX } else { -money }
}
fn price(index: usize) -> i64 {
    crate::company::price(index)
}

/// A live native `CommandCost` borrowed for the synchronous entry.
#[derive(Clone, Copy)]
struct Cost<'a>(*mut CommandCost, &'a CommandCostServices);
impl Cost<'_> {
    fn zero(self) {
        (self.1.zero)(self.0);
    }
    fn error(self, message: u32) {
        (self.1.error)(self.0, message);
    }
    fn set(self, money: i64) {
        (self.1.expense_money)(self.0, CONSTRUCTION, money);
    }
    fn add_money(self, money: i64) {
        (self.1.amount)(self.0, money);
    }
    /// `AddCost(CommandCost &&)`.
    fn add(self, other: Self) {
        (self.1.add)(self.0, other.0);
    }
    fn take(self, other: Self) {
        (self.1.move_cost)(self.0, other.0);
    }
    fn make_error(self, message: u32) {
        unsafe { (self.1.make_error)(self.0, message) };
    }
    fn ok(self) -> bool {
        (self.1.success)(self.0)
    }
    fn message(self) -> u32 {
        (self.1.error_message)(self.0)
    }
    fn money(self) -> i64 {
        (self.1.money)(self.0)
    }
}

struct Ctx<'a> {
    s: &'a RailCommandServices,
    shared: &'a Services,
    rail: &'a RailMapServices,
    cost: &'a CommandCostServices,
    command: &'a CommandServices,
    map: Map,
}
impl<'a> Ctx<'a> {
    /// # Safety
    /// The table and its nested tables are valid for the synchronous call.
    unsafe fn new(s: *const RailCommandServices) -> Self {
        w::hit(w::RAIL_CMD_ENTRIES);
        // SAFETY: native immutable process-lifetime tables.
        let s = unsafe { &*s };
        let shared = unsafe { &*s.shared };
        Self {
            s,
            shared,
            rail: unsafe { &*s.rail },
            cost: unsafe { &*s.cost },
            command: unsafe { &*s.command },
            map: Map {
                log_x: (shared.map_log_x)() as u8,
            },
        }
    }
    fn c(&self, cost: *mut CommandCost) -> Cost<'a> {
        Cost(cost, self.cost)
    }
    fn map_size(&self) -> u32 {
        ((self.shared.map_max_y)().wrapping_add(1)) << self.map.log_x
    }
    fn tile_type(&self, tile: u32) -> u8 {
        (self.shared.tile_type)(tile)
    }
    fn current(&self) -> u8 {
        (self.shared.current_company)()
    }
    fn tracks(&self, tile: u32) -> u8 {
        (self.rail.rail_tracks)(tile).0
    }
    /// The native uint query contains only the four signal-presence bits.
    fn present_signals(&self, tile: u32) -> u8 {
        (self.rail.present_signals)(tile) as u8
    }
    fn has_track(&self, tile: u32, track: u8) -> bool {
        self.tracks(tile) >> track & 1 != 0
    }
    /// `HasSignalOnTrack`: signal tile type and a present signal on the track.
    fn has_signal(&self, tile: u32, track: u8) -> bool {
        (self.rail.has_signals)(tile)
            && self.present_signals(tile) & SIGNAL_ON_TRACK[usize::from(track)] != 0
    }
    fn reserved(&self, tile: u32) -> u8 {
        (self.rail.reserved_tracks)(tile).0
    }
    fn signal_type(&self, tile: u32, track: u8) -> u8 {
        (self.rail.signal_type)(tile, track).0
    }
    /// Direct signal-buffer context over this entry's immutable tables.
    fn signals(&self) -> crate::signal::Context {
        crate::signal::Context {
            shared: self.s.shared,
            rail: self.s.rail,
            native: self.s.signal,
            log_x: std::ffi::c_uint::from(self.map.log_x),
        }
    }
    fn add_track(&self, tile: u32, track: u8, owner: u8) {
        self.signals().add_track(tile, track, owner);
    }
    fn add_side(&self, tile: u32, side: u8, owner: u8) {
        self.signals().add_side(tile, side, owner);
    }
    fn rail_infra(&self, owner: u8, railtype: u8, delta: u32) {
        (self.s.rail_infrastructure)(owner, railtype, delta);
    }
    /// `TileX`/`TileY` delta add, as `tile += ToTileIndexDiff(_trackdelta[td])`.
    fn step(&self, tile: u32, trackdir: u8) -> u32 {
        let (x, y) = TRACK_DELTA[usize::from(trackdir)];
        tile.wrapping_add(self.map.diff(x, y))
    }
    fn tunnel_length(&self, begin: u32, end: u32) -> u32 {
        let (x1, y1) = (self.map.tile_x(begin) as i32, self.map.tile_y(begin) as i32);
        let (x2, y2) = (self.map.tile_x(end) as i32, self.map.tile_y(end) as i32);
        ((x2 + y2 - x1 - y1).abs() - 1) as u32
    }

    /// `CheckRailSlope`.
    fn check_rail_slope(&self, out: Cost<'_>, tileh: u8, bits: u8, existing: u8, tile: u32) {
        if (self.s.flooding)(tile).0 != 0
            && !steep(tileh)
            && !LEVELED_TRACKS[usize::from(tileh)] & (bits | existing) != 0
        {
            out.error(self.s.strings.build_on_water);
            return;
        }
        let f_new = (self.s.rail_foundation)(tileh, bits | existing).0;
        if f_new == FOUNDATION_INVALID
            || (f_new != FOUNDATION_NONE && !(self.shared.build_on_slopes)())
        {
            out.error(self.s.strings.sloped_wrong);
            return;
        }
        let f_old = (self.s.rail_foundation)(tileh, existing).0;
        out.set(if f_new == f_old {
            0
        } else {
            price(PR_BUILD_FOUNDATION)
        });
    }

    /// `CmdBuildSingleRail`.
    fn build_single_rail(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        other: Cost<'_>,
        flags: u16,
        tile: u32,
        railtype: u8,
        track: u8,
        auto_remove_signals: bool,
    ) {
        cost.set(0);
        if !(self.s.rail_type_valid)(railtype) || track >= 6 {
            cost.error(INVALID_STRING);
            return;
        }
        let tileh = (self.shared.tile_slope)(tile);
        let trackbit = 1_u8 << track;
        let strings = &self.s.strings;
        let mut fall_through = false;
        match self.tile_type(tile) {
            MP_RAILWAY => {
                // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                unsafe { (self.command.tile_ownership)(ret.0, tile) };
                if !ret.ok() {
                    return cost.take(ret);
                }
                if !(self.shared.is_plain_rail)(tile) {
                    w::hit(w::RAIL_CMD_BUILD_NOT_PLAIN);
                    return (self.command.landscape_clear)(cost.0, tile, flags);
                }
                if !(self.s.compatible_rail)((self.rail.rail_type)(tile).0, railtype) {
                    return cost.error(strings.impossible_combination);
                }
                // CheckTrackCombination, then EnsureNoTrainOnTrack.
                if self.tracks(tile) | trackbit == self.tracks(tile) {
                    ret.error(strings.already_built);
                } else {
                    ret.zero();
                }
                if ret.ok() {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.no_train)(ret.0, tile, trackbit) };
                }
                if !ret.ok() {
                    return cost.take(ret);
                }
                self.check_rail_slope(ret, tileh, trackbit, self.tracks(tile), tile);
                if !ret.ok() {
                    return cost.take(ret);
                }
                cost.add_money(ret.money());
                if (self.rail.has_signals)(tile) && tracks_overlap(self.tracks(tile) | trackbit) {
                    if !auto_remove_signals {
                        return cost.error(strings.remove_signals_first);
                    }
                    w::hit(w::RAIL_CMD_BUILD_REMOVE_SIGNALS);
                    for track_it in 0..6 {
                        if self.has_track(tile, track_it) && self.has_signal(tile, track_it) {
                            unsafe { (self.s.remove_signal)(other.0, flags, tile, track_it) };
                            if !other.ok() {
                                return cost.take(other);
                            }
                            cost.add_money(other.money());
                        }
                    }
                }
                let current = (self.rail.rail_type)(tile).0;
                if current != railtype && !(self.s.powered_rail)(railtype, current) {
                    if (self.s.powered_rail)(current, railtype) {
                        w::hit(w::RAIL_CMD_BUILD_CONVERT);
                        unsafe { (self.s.convert_rail)(ret.0, flags, tile, tile, railtype, false) };
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                        cost.add_money(ret.money());
                    } else {
                        return cost.error(INVALID_STRING);
                    }
                }
                if flags & EXECUTE != 0 {
                    w::hit(w::RAIL_CMD_BUILD_RAIL);
                    (self.rail.set_rail_ground)(tile, GROUND_BARREN);
                    let bits = self.tracks(tile);
                    (self.rail.set_track_bits)(tile, bits | trackbit);
                    let owner = (self.shared.tile_owner)(tile);
                    self.rail_infra(
                        owner,
                        (self.rail.rail_type)(tile).0,
                        pieces(bits).wrapping_neg(),
                    );
                    let owner = (self.shared.tile_owner)(tile);
                    self.rail_infra(
                        owner,
                        (self.rail.rail_type)(tile).0,
                        pieces(bits | trackbit),
                    );
                    (self.s.infrastructure_dirty)((self.shared.tile_owner)(tile));
                }
            }
            MP_ROAD => {
                if CROSSING_SLOPES >> tileh & 1 == 0 {
                    return cost.error(strings.sloped_wrong);
                }
                if !(self.s.crossing_with_competitor)() && self.current() != OWNER_DEITY {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.tile_ownership)(ret.0, tile) };
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }
                // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                unsafe { (self.command.no_vehicle)(ret.0, tile) };
                if !ret.ok() {
                    return cost.take(ret);
                }
                let mut built = false;
                if (self.shared.is_normal_road)(tile) {
                    if (self.shared.has_road_works)(tile) {
                        return cost.error(strings.road_works);
                    }
                    if (self.shared.disallowed_road_directions)(tile) != 0 {
                        return cost.error(strings.oneway_road);
                    }
                    if (self.s.rail_no_crossings)(railtype) {
                        return cost.error(strings.crossing_rail);
                    }
                    let road_type = (self.shared.road_type)(tile);
                    let tram_type = (self.shared.tram_type)(tile).0;
                    if road_type != INVALID_ROADTYPE && (self.s.road_no_crossing)(road_type) {
                        return cost.error(strings.crossing_road);
                    }
                    if tram_type != INVALID_ROADTYPE && (self.s.road_no_crossing)(tram_type) {
                        return cost.error(strings.crossing_road);
                    }
                    let road = NormalRoad {
                        road_type,
                        tram_type,
                        road_bits: (self.shared.normal_road_bits)(tile, 0).0,
                        tram_bits: (self.shared.normal_road_bits)(tile, 1).0,
                    };
                    let bits = road.road_bits | road.tram_bits;
                    if (track == TRACK_X && bits & ROAD_X == 0)
                        || (track == TRACK_Y && bits & ROAD_Y == 0)
                    {
                        if self
                            .build_crossing(cost, ret, flags, tile, railtype, track, road)
                            .is_none()
                        {
                            return;
                        }
                        built = true;
                    }
                }
                if !built {
                    if (self.shared.is_level_crossing)(tile)
                        && (self.rail.crossing_rail_bits)(tile).0 == trackbit
                    {
                        return cost.error(strings.already_built);
                    }
                    fall_through = true;
                }
            }
            _ => fall_through = true,
        }
        if fall_through {
            let water_ground = self.tile_type(tile) == MP_WATER && one_corner_raised(tileh);
            self.check_rail_slope(ret, tileh, trackbit, 0, tile);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
            (self.command.landscape_clear)(ret.0, tile, flags);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
            if water_ground {
                cost.add_money(negate(price(PR_CLEAR_WATER)));
                cost.add_money(price(PR_CLEAR_ROUGH));
            }
            if flags & EXECUTE != 0 {
                w::hit(w::RAIL_CMD_BUILD_CLEAR);
                let current = self.current();
                (self.rail.make_rail_normal)(tile, current, trackbit, railtype);
                if water_ground {
                    w::hit(w::RAIL_CMD_BUILD_HALF_WATER);
                    (self.rail.set_rail_ground)(tile, GROUND_HALF_WATER);
                    (self.s.docking_check)(tile);
                }
                self.rail_infra(self.current(), railtype, 1);
                (self.s.infrastructure_dirty)(self.current());
            }
        }
        if flags & EXECUTE != 0 {
            (self.shared.mark_dirty)(tile);
            self.add_track(tile, track, self.current());
            (self.s.layout_changed)(tile, track);
        }
        cost.add_money((self.s.rail_build_cost)(railtype));
    }

    /// Level-crossing branch of `CmdBuildSingleRail`; `Err` returns early.
    fn build_crossing(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        railtype: u8,
        track: u8,
        road: NormalRoad,
    ) -> Option<()> {
        let road_owner = (self.shared.road_owner)(tile);
        let tram_owner = (self.shared.tram_owner)(tile);
        if (self.shared.company_valid)(tram_owner) && road.tram_bits.is_power_of_two() {
            // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
            unsafe { (self.command.ownership)(ret.0, tram_owner) };
            if !ret.ok() {
                cost.take(ret);
                return None;
            }
        }
        let new_road = if road.road_bits == 0 {
            0
        } else {
            2_u32.wrapping_sub(road.road_bits.count_ones())
        };
        if new_road > 0 {
            cost.add_money(
                (self.s.road_build_cost)(road.road_type).saturating_mul(i64::from(new_road)),
            );
        }
        let new_tram = if road.tram_bits == 0 {
            0
        } else {
            2_u32.wrapping_sub(road.tram_bits.count_ones())
        };
        if new_tram > 0 {
            cost.add_money(
                (self.s.road_build_cost)(road.tram_type).saturating_mul(i64::from(new_tram)),
            );
        }
        if flags & EXECUTE != 0 {
            w::hit(w::RAIL_CMD_BUILD_CROSSING);
            let current = self.current();
            let axis = u8::from(track == TRACK_X); // AXIS_Y for TRACK_X.
            let town = (self.shared.town_index)(tile);
            (self.s.make_crossing)(
                tile,
                road_owner,
                tram_owner,
                current,
                axis,
                railtype,
                road.road_type,
                road.tram_type,
                town,
            );
            (self.shared.update_level_crossing)(tile, false);
            (self.s.mark_crossings)(tile, (self.rail.crossing_road_axis)(tile).0);
            self.rail_infra(self.current(), railtype, LEVELCROSSING_TRACKBIT_FACTOR);
            (self.s.infrastructure_dirty)(self.current());
            if new_road > 0 && (self.shared.company_valid)(road_owner) {
                (self.s.road_infrastructure)(road_owner, road.road_type, new_road);
                (self.s.infrastructure_dirty)(road_owner);
            }
            if new_tram > 0 && (self.shared.company_valid)(tram_owner) {
                (self.s.road_infrastructure)(tram_owner, road.tram_type, new_tram);
                (self.s.infrastructure_dirty)(tram_owner);
            }
        }
        Some(())
    }

    /// `CmdRemoveSingleRail`.
    fn remove_single_rail(&self, cost: Cost<'_>, ret: Cost<'_>, flags: u16, tile: u32, track: u8) {
        cost.set(0);
        let mut crossing = false;
        if track >= 6 {
            return cost.error(INVALID_STRING);
        }
        let trackbit = 1_u8 << track;
        let mut owner = INVALID_OWNER;
        let mut v: *mut Train = std::ptr::null_mut();
        let no_track = self.s.strings.no_track;
        match self.tile_type(tile) {
            MP_ROAD => {
                if !(self.shared.is_level_crossing)(tile)
                    || (self.rail.crossing_rail_bits)(tile).0 != trackbit
                {
                    return cost.error(no_track);
                }
                if self.current() != OWNER_WATER {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.tile_ownership)(ret.0, tile) };
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }
                if flags & BANKRUPT == 0 {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.no_vehicle)(ret.0, tile) };
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }
                cost.add_money((self.s.rail_clear_cost)((self.rail.rail_type)(tile).0));
                if flags & EXECUTE != 0 {
                    w::hit(w::RAIL_CMD_REMOVE_CROSSING);
                    (self.s.crossing_removed)(tile, (self.rail.crossing_road_axis)(tile).0);
                    if self.reserved(tile) & trackbit != 0 {
                        v = (self.s.reserving_train)(tile, track);
                        if !v.is_null() {
                            unsafe { (self.shared.free_train_reservation)(v) };
                        }
                    }
                    owner = (self.shared.tile_owner)(tile);
                    self.rail_infra(
                        owner,
                        (self.rail.rail_type)(tile).0,
                        LEVELCROSSING_TRACKBIT_FACTOR.wrapping_neg(),
                    );
                    (self.s.infrastructure_dirty)(owner);
                    let road = CrossingRoad {
                        bits: (self.shared.crossing_road_bits)(tile).0,
                        road_type: (self.shared.road_type)(tile),
                        tram_type: (self.shared.tram_type)(tile).0,
                        town: (self.shared.town_index)(tile),
                        road_owner: (self.shared.road_owner)(tile),
                        tram_owner: (self.shared.tram_owner)(tile),
                    };
                    (self.s.make_road)(tile, road);
                    (self.s.close_inspect)(tile);
                }
            }
            MP_RAILWAY => {
                if !(self.shared.is_plain_rail)(tile) {
                    return cost.error(no_track);
                }
                if self.current() != OWNER_WATER {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.tile_ownership)(ret.0, tile) };
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }
                // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                unsafe { (self.command.no_train)(ret.0, tile, trackbit) };
                if !ret.ok() {
                    return cost.take(ret);
                }
                let mut present = self.tracks(tile);
                if present & trackbit == 0 {
                    return cost.error(no_track);
                }
                if present == BIT_X | BIT_Y {
                    crossing = true;
                }
                cost.add_money((self.s.rail_clear_cost)((self.rail.rail_type)(tile).0));
                if self.has_signal(tile, track) {
                    w::hit(w::RAIL_CMD_REMOVE_SIGNAL_CHARGE);
                    unsafe { (self.s.remove_signal)(ret.0, flags, tile, track) };
                    cost.add(ret);
                }
                if flags & EXECUTE != 0 {
                    w::hit(w::RAIL_CMD_REMOVE_RAIL);
                    if self.reserved(tile) & trackbit != 0 {
                        v = (self.s.reserving_train)(tile, track);
                        if !v.is_null() {
                            unsafe { (self.shared.free_train_reservation)(v) };
                        }
                    }
                    owner = (self.shared.tile_owner)(tile);
                    self.rail_infra(
                        owner,
                        (self.rail.rail_type)(tile).0,
                        pieces(present).wrapping_neg(),
                    );
                    present ^= trackbit;
                    self.rail_infra(owner, (self.rail.rail_type)(tile).0, pieces(present));
                    (self.s.infrastructure_dirty)(owner);
                    if present == 0 {
                        let tileh = (self.shared.tile_slope)(tile);
                        if (self.rail.rail_ground)(tile).0 == GROUND_HALF_WATER
                            && one_corner_raised(tileh)
                        {
                            w::hit(w::RAIL_CMD_REMOVE_SHORE);
                            (self.s.make_shore)(tile);
                        } else {
                            (self.shared.clear_square)(tile);
                        }
                        (self.s.close_inspect)(tile);
                    } else {
                        (self.rail.set_track_bits)(tile, present);
                        let reservation = (self.rail.rail_reservation)(tile).0;
                        (self.rail.set_track_reservation)(tile, reservation & present);
                    }
                }
            }
            _ => return cost.error(no_track),
        }
        if flags & EXECUTE != 0 {
            (self.shared.mark_dirty)(tile);
            if crossing {
                w::hit(w::RAIL_CMD_REMOVE_DIAGONAL_CROSSING);
                self.add_track(tile, TRACK_X, owner);
                self.add_track(tile, TRACK_Y, owner);
                (self.s.layout_changed)(tile, TRACK_X);
                (self.s.layout_changed)(tile, TRACK_Y);
            } else {
                self.add_track(tile, track, owner);
                (self.s.layout_changed)(tile, track);
            }
            if !v.is_null() {
                unsafe { (self.shared.try_path_reserve)(v, true, false) };
            }
        }
    }

    /// `ValidateAutoDrag`; false is `CMD_ERROR`.
    fn validate_auto_drag(&self, trackdir: &mut u8, start: u32, end: u32) -> bool {
        let x = self.map.tile_x(start) as i32;
        let y = self.map.tile_y(start) as i32;
        let ex = self.map.tile_x(end) as i32;
        let ey = self.map.tile_y(end) as i32;
        if *trackdir & 7 >= 6 {
            return false;
        }
        let dx = ex - x;
        let dy = ey - y;
        let (mut delta_x, mut delta_y) = TRACK_DELTA[usize::from(*trackdir)];
        if !diagonal(*trackdir) {
            let (ox, oy) = TRACK_DELTA[usize::from(*trackdir ^ 1)];
            delta_x += ox;
            delta_y += oy;
        }
        while (delta_x <= 0 && dx > 0)
            || (delta_x >= 0 && dx < 0)
            || (delta_y <= 0 && dy > 0)
            || (delta_y >= 0 && dy < 0)
        {
            if *trackdir & 8 == 0 {
                *trackdir |= 8;
                delta_x = -delta_x;
                delta_y = -delta_y;
            } else {
                return false;
            }
        }
        if !diagonal(*trackdir) {
            let (tx, ty) = TRACK_DELTA[usize::from(*trackdir)];
            if dx.abs() != dy.abs() && dx.abs() + ty.abs() != dy.abs() + tx.abs() {
                return false;
            }
        }
        true
    }

    /// `CmdRailTrackHelper`.
    fn rail_track_helper(
        &self,
        total: Cost<'_>,
        last_error: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        mut tile: u32,
        end_tile: u32,
        railtype: u8,
        track: u8,
        remove: bool,
        auto_remove_signals: bool,
        fail_on_obstacle: bool,
    ) {
        total.set(0);
        if (!remove && !(self.s.rail_type_valid)(railtype)) || track >= 6 {
            return total.error(INVALID_STRING);
        }
        let size = self.map_size();
        if end_tile >= size || tile >= size {
            return total.error(INVALID_STRING);
        }
        let mut trackdir = track;
        if !self.validate_auto_drag(&mut trackdir, tile, end_tile) {
            return total.error(INVALID_STRING);
        }
        w::hit(if remove {
            w::RAIL_CMD_DRAG_REMOVE
        } else {
            w::RAIL_CMD_DRAG_BUILD
        });
        let already = self.s.strings.already_built;
        let mut had_success = false;
        last_error.error(INVALID_STRING);
        loop {
            if remove {
                unsafe { (self.s.remove_rail)(ret.0, flags, tile, trackdir & 7) };
            } else {
                unsafe {
                    (self.s.build_rail)(
                        ret.0,
                        flags,
                        tile,
                        railtype,
                        trackdir & 7,
                        auto_remove_signals,
                    );
                };
            }
            if !remove && !fail_on_obstacle && last_error.message() == already {
                had_success = true;
            }
            if ret.ok() {
                had_success = true;
                total.add_money(ret.money());
            } else {
                last_error.take(ret);
                if last_error.message() != already && !remove {
                    if fail_on_obstacle {
                        return total.take(last_error);
                    }
                    if had_success {
                        w::hit(w::RAIL_CMD_DRAG_OBSTACLE);
                        break;
                    }
                }
                if last_error.message() == self.s.strings.owned_by && remove {
                    break;
                }
            }
            if tile == end_tile {
                break;
            }
            tile = self.step(tile, trackdir);
            if !diagonal(trackdir) {
                trackdir ^= 1;
            }
        }
        if !had_success {
            total.take(last_error);
        }
    }

    /// `CmdBuildTrainDepot`.
    fn build_depot(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        railtype: u8,
        dir: u8,
    ) {
        if !(self.s.rail_type_valid)(railtype) || dir >= 4 {
            return cost.error(INVALID_STRING);
        }
        let tileh = (self.shared.tile_slope)(tile);
        cost.set(0);
        if tileh != SLOPE_FLAT {
            const INCLINED: [u8; 4] = [12, 6, 3, 9];
            let entrance = INCLINED[usize::from(dir)];
            let fits = if steep(tileh) {
                tileh & entrance == entrance
            } else {
                tileh & entrance != 0
            };
            if !(self.shared.build_on_slopes)() || !fits {
                return cost.error(self.s.strings.flat_land);
            }
            cost.add_money(price(PR_BUILD_FOUNDATION));
        }
        let mut rotate = false;
        if self.tile_type(tile) == MP_RAILWAY
            && (self.rail.is_rail_depot)(tile)
            && railtype == (self.rail.rail_type)(tile).0
        {
            // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
            unsafe { (self.command.tile_ownership)(ret.0, tile) };
            if !ret.ok() {
                return cost.take(ret);
            }
            if dir == (self.rail.rail_depot_direction)(tile).0 {
                return cost.zero();
            }
            // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
            unsafe { (self.command.no_vehicle)(ret.0, tile) };
            if !ret.ok() {
                return cost.take(ret);
            }
            rotate = true;
        }
        if !rotate {
            (self.command.landscape_clear)(ret.0, tile, flags);
            cost.add(ret);
            if !cost.ok() {
                return;
            }
            if (self.shared.bridge_above)(tile) {
                return cost.error(self.s.strings.demolish_bridge);
            }
            if !(self.s.depot_available)() {
                return cost.error(INVALID_STRING);
            }
        }
        if flags & EXECUTE != 0 {
            if rotate {
                w::hit(w::RAIL_CMD_DEPOT_ROTATE);
                (self.rail.set_depot_exit)(tile, dir);
            } else {
                w::hit(w::RAIL_CMD_DEPOT_BUILD);
                let depot = (self.s.new_depot)(tile);
                (self.rail.make_rail_depot)(tile, self.current(), depot, dir, railtype);
                (self.s.depot_name)(depot);
                self.rail_infra(self.current(), railtype, 1);
                (self.s.infrastructure_dirty)(self.current());
            }
            (self.shared.mark_dirty)(tile);
            self.add_side(tile, INVALID_DIAGDIR, self.current());
            (self.s.layout_changed)(tile, dir & 1);
        }
        cost.add_money(price(PR_BUILD_DEPOT_TRAIN));
        cost.add_money((self.s.rail_build_cost)(railtype));
    }

    /// `CmdBuildSingleSignal`.
    fn build_signal(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        tile: u32,
        track: u8,
        mut sigtype: u8,
        mut sigvar: u8,
        convert_signal: bool,
        skip_existing_signals: bool,
        ctrl_pressed: bool,
        cycle_start: u8,
        cycle_stop: u8,
        mut num_dir_cycle: u8,
        signals_copy: u8,
    ) {
        if sigtype > SIGTYPE_LAST || sigvar > SIG_SEMAPHORE {
            return cost.error(INVALID_STRING);
        }
        if cycle_start > cycle_stop || cycle_stop > SIGTYPE_LAST {
            return cost.error(INVALID_STRING);
        }
        if ctrl_pressed {
            sigvar ^= SIG_SEMAPHORE;
        }
        if track >= 6 || !(self.shared.is_plain_rail)(tile) || !self.has_track(tile, track) {
            return cost.error(self.s.strings.no_track);
        }
        let on_track = SIGNAL_ON_TRACK[usize::from(track)];
        if signals_copy != 0 && signals_copy & on_track == 0 {
            return cost.error(INVALID_STRING);
        }
        // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
        unsafe { (self.command.tile_ownership)(cost.0, tile) };
        if !cost.ok() {
            return;
        }
        if tracks_overlap(self.tracks(tile)) {
            return cost.error(self.s.strings.no_suitable_track);
        }
        if skip_existing_signals && self.has_signal(tile, track) {
            return cost.zero();
        }
        if convert_signal && !self.has_signal(tile, track) {
            return cost.error(self.s.strings.no_signals);
        }
        if !self.has_signal(tile, track) {
            cost.set(price(PR_BUILD_SIGNALS));
        } else if signals_copy != 0 && sigvar != (self.rail.signal_variant)(tile, track).0 {
            cost.set(price(PR_BUILD_SIGNALS).saturating_add(price(PR_CLEAR_SIGNALS)));
        } else if convert_signal {
            if ctrl_pressed || (self.rail.signal_variant)(tile, track).0 != sigvar {
                cost.set(price(PR_BUILD_SIGNALS).saturating_add(price(PR_CLEAR_SIGNALS)));
            } else {
                cost.zero();
            }
        } else {
            cost.zero();
        }
        if flags & EXECUTE == 0 {
            return;
        }
        let mut v: *mut Train = std::ptr::null_mut();
        if self.reserved(tile) & (1 << track) != 0 {
            v = (self.s.reserving_train)(tile, track);
            if !v.is_null() {
                unsafe { (self.shared.free_train_reservation)(v) };
            }
        }
        if !(self.rail.has_signals)(tile) {
            (self.rail.set_has_signals)(tile, true);
            (self.rail.set_signal_states)(tile, 0xF);
            (self.rail.set_present_signals)(tile, 0);
            (self.rail.set_signal_type)(tile, track, sigtype);
            (self.rail.set_signal_variant)(tile, track, sigvar);
        }
        let present = self.present_signals(tile);
        (self.s.signal_infrastructure)(
            (self.shared.tile_owner)(tile),
            present.count_ones().wrapping_neg(),
        );
        if signals_copy == 0 {
            if !self.has_signal(tile, track) {
                w::hit(w::RAIL_CMD_SIGNAL_BUILD);
                let added = if is_pbs(sigtype) {
                    kill_first_bit(on_track)
                } else {
                    on_track
                };
                (self.rail.set_present_signals)(tile, self.present_signals(tile) | added);
                (self.rail.set_signal_type)(tile, track, sigtype);
                (self.rail.set_signal_variant)(tile, track, sigvar);
                while num_dir_cycle > 0 {
                    num_dir_cycle -= 1;
                    (self.rail.cycle_signal_side)(tile, track);
                }
            } else if convert_signal {
                if ctrl_pressed {
                    w::hit(w::RAIL_CMD_SIGNAL_VARIANT);
                    let variant = if (self.rail.signal_variant)(tile, track).0 == SIG_ELECTRIC {
                        SIG_SEMAPHORE
                    } else {
                        SIG_ELECTRIC
                    };
                    (self.rail.set_signal_variant)(tile, track, variant);
                    sigtype = self.signal_type(tile, track);
                } else {
                    w::hit(w::RAIL_CMD_SIGNAL_CONVERT);
                    (self.rail.set_signal_type)(tile, track, sigtype);
                    (self.rail.set_signal_variant)(tile, track, sigvar);
                    self.pbs_one_way(tile, sigtype, on_track);
                }
            } else if ctrl_pressed {
                w::hit(w::RAIL_CMD_SIGNAL_CYCLE_TYPE);
                sigtype = self.signal_type(tile, track).wrapping_add(1);
                if sigtype < cycle_start || sigtype > cycle_stop {
                    sigtype = cycle_start;
                }
                (self.rail.set_signal_type)(tile, track, sigtype);
                self.pbs_one_way(tile, sigtype, on_track);
            } else {
                w::hit(w::RAIL_CMD_SIGNAL_CYCLE_SIDE);
                (self.rail.cycle_signal_side)(tile, track);
                sigtype = self.signal_type(tile, track);
            }
        } else {
            w::hit(w::RAIL_CMD_SIGNAL_COPY);
            let present = self.present_signals(tile);
            (self.rail.set_present_signals)(
                tile,
                (present & !on_track) | (signals_copy & on_track),
            );
            (self.rail.set_signal_variant)(tile, track, sigvar);
            (self.rail.set_signal_type)(tile, track, sigtype);
        }
        let present = self.present_signals(tile);
        (self.s.signal_infrastructure)((self.shared.tile_owner)(tile), present.count_ones());
        (self.s.infrastructure_dirty)((self.shared.tile_owner)(tile));
        if is_pbs(sigtype) {
            w::hit(w::RAIL_CMD_SIGNAL_PBS_STATE);
            let mask = u32::from(self.present_signals(tile) & on_track);
            let reserved = (self.rail.rail_reservation)(tile).0 >> track & 1 != 0 && {
                // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                unsafe { (self.command.no_vehicle)(ret.0, tile) };
                ret.ok()
            };
            let states = (self.rail.signal_states)(tile);
            let value = (states & !mask) | (if reserved { u32::MAX } else { 0 } & mask);
            (self.rail.set_signal_states)(tile, value as u8);
        }
        (self.shared.mark_dirty)(tile);
        self.add_track(tile, track, self.current());
        (self.s.layout_changed)(tile, track);
        if !v.is_null() {
            let p = unsafe { (self.s.parking)(v) };
            if p.track != BIT_DEPOT {
                let parked = (p.status & STOPPED != 0 && p.speed == 0) || p.order == OT_LOADING;
                if !parked || {
                    let at = unsafe { (self.s.train_position)(v) };
                    // SAFETY: The reservation train remains live through this synchronous check.
                    !unsafe {
                        (self.shared.safe_waiting_position)(
                            v,
                            at.tile,
                            at.trackdir,
                            (self.shared.forbid_90_deg)(),
                        )
                    }
                } {
                    w::hit(w::RAIL_CMD_SIGNAL_RESERVE);
                    unsafe { (self.shared.try_path_reserve)(v, true, false) };
                }
            }
        }
    }
    /// Shared one-way PBS presence rewrite of the convert and cycle branches.
    fn pbs_one_way(&self, tile: u32, sigtype: u8, on_track: u8) {
        let present = self.present_signals(tile);
        if is_pbs(sigtype) && present & on_track == on_track {
            let present = self.present_signals(tile);
            (self.rail.set_present_signals)(tile, (present & !on_track) | kill_first_bit(on_track));
        }
    }

    /// `AdvanceSignalAutoFill`.
    fn advance_autofill(&self, tile: &mut u32, trackdir: &mut u8, remove: bool) -> bool {
        if self.tile_type(*tile) == MP_TUNNELBRIDGE {
            *tile = (self.rail.other_tunnel_bridge_end)(*tile);
        }
        let (dx, dy) = TRACK_DELTA[usize::from(*trackdir)];
        let x = self.map.tile_x(*tile) as i32 + dx;
        let y = self.map.tile_y(*tile) as i32 + dy;
        if x as u32 >= self.map.size_x() || y as u32 > (self.shared.map_max_y)() {
            *tile = INVALID_TILE;
            return false;
        }
        *tile = ((y as u32) << self.map.log_x) | x as u32;
        let mut bits = ((self.rail.rail_track_status)(*tile, INVALID_DIAGDIR) & 0x3F3F) as u16;
        if tracks_overlap(trackdir_tracks(bits)) {
            return false;
        }
        bits &= reaches(*trackdir);
        if bits == 0 {
            return false;
        }
        *trackdir = bits.trailing_zeros() as u8;
        bits &= bits - 1;
        if bits != 0 {
            return false;
        }
        match self.tile_type(*tile) {
            MP_RAILWAY => {
                if (self.rail.is_rail_depot)(*tile) {
                    return false;
                }
                if !remove && self.has_signal(*tile, *trackdir & 7) {
                    return false;
                }
            }
            MP_ROAD => {
                if !(self.shared.is_level_crossing)(*tile) {
                    return false;
                }
            }
            MP_TUNNELBRIDGE => {
                if (self.shared.tunnel_transport)(*tile) != 0 {
                    return false;
                }
                if (self.shared.tunnel_bridge_direction)(*tile)
                    != TRACKDIR_EXIT[usize::from(*trackdir)]
                {
                    return false;
                }
            }
            _ => return false,
        }
        true
    }

    /// `CmdSignalTrackHelper`.
    fn signal_track_helper(
        &self,
        total: Cost<'_>,
        last_error: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        mut tile: u32,
        end_tile: u32,
        mut track: u8,
        mut sigtype: u8,
        mut sigvar: u8,
        mode: bool,
        remove: bool,
        autofill: bool,
        minimise_gaps: bool,
        mut signal_density: i32,
    ) {
        total.set(0);
        if end_tile >= self.map_size() || track >= 6 {
            return total.error(INVALID_STRING);
        }
        if signal_density == 0 || signal_density > 20 {
            return total.error(INVALID_STRING);
        }
        if !remove && (sigtype > SIGTYPE_LAST || sigvar > SIG_SEMAPHORE) {
            return total.error(INVALID_STRING);
        }
        if !(self.shared.is_plain_rail)(tile) {
            return total.error(self.s.strings.no_track);
        }
        let start_tile = tile;
        signal_density = signal_density.wrapping_mul(TILE_AXIAL_DISTANCE);
        let mut trackdir = track;
        if !self.validate_auto_drag(&mut trackdir, tile, end_tile) {
            return total.error(INVALID_STRING);
        }
        track = trackdir & 7;
        let start_trackdir = trackdir;
        if !self.has_track(tile, track) {
            return total.error(INVALID_STRING);
        }
        let signals = if self.has_signal(tile, track) {
            let signals = self.present_signals(tile) & SIGNAL_ON_TRACK[usize::from(track)];
            sigvar = (self.rail.signal_variant)(tile, track).0;
            sigtype = self.signal_type(tile, track);
            if sigtype == SIGTYPE_ENTRY || sigtype == SIGTYPE_EXIT {
                sigtype = SIGTYPE_BLOCK;
            }
            signals
        } else if is_pbs(sigtype) {
            SIGNAL_ALONG[usize::from(trackdir)]
        } else {
            SIGNAL_ON_TRACK[usize::from(track)]
        };
        let mut signal_dir = 0_u8;
        if signals & SIGNAL_ALONG[usize::from(trackdir)] != 0 {
            signal_dir |= 1;
        }
        if signals & SIGNAL_AGAINST[usize::from(trackdir)] != 0 {
            signal_dir |= 2;
        }
        w::hit(if remove {
            w::RAIL_CMD_SIGNAL_DRAG_REMOVE
        } else {
            w::RAIL_CMD_SIGNAL_DRAG
        });
        let mut signal_ctr = 0_i32;
        let mut last_used_ctr = signal_density.wrapping_neg();
        let mut last_suitable_ctr = 0_i32;
        let mut last_suitable_tile = INVALID_TILE;
        let mut last_suitable_trackdir = INVALID_TRACKDIR;
        last_error.error(INVALID_STRING);
        let mut had_success = false;
        let no_track = self.s.strings.no_track;
        let mut build_signal = |tile: u32, trackdir: u8, test_only: bool, signal_ctr: i32| {
            let mut signals = 0_u8;
            if signal_dir & 1 != 0 {
                signals |= SIGNAL_ALONG[usize::from(trackdir)];
            }
            if signal_dir & 2 != 0 {
                signals |= SIGNAL_AGAINST[usize::from(trackdir)];
            }
            let do_flags = if test_only { flags & !EXECUTE } else { flags };
            if remove {
                unsafe { (self.s.remove_signal)(ret.0, do_flags, tile, trackdir & 7) };
            } else {
                unsafe {
                    (self.s.build_signal)(
                        ret.0,
                        do_flags,
                        tile,
                        trackdir & 7,
                        sigtype,
                        sigvar,
                        false,
                        signal_ctr == 0,
                        mode,
                        SIGTYPE_BLOCK,
                        SIGTYPE_BLOCK,
                        0,
                        signals,
                    );
                };
            }
            let succeeded = ret.ok();
            if test_only {
                return succeeded;
            }
            if succeeded {
                had_success = true;
                total.add_money(ret.money());
            } else if ret.message() != no_track || last_error.message() == INVALID_STRING {
                last_error.take(ret);
            }
            succeeded
        };
        loop {
            if remove {
                build_signal(tile, trackdir, false, signal_ctr);
            } else if minimise_gaps {
                w::hit(w::RAIL_CMD_SIGNAL_MINIMISE);
                if signal_ctr > last_used_ctr.wrapping_add(signal_density)
                    && last_suitable_tile != INVALID_TILE
                    && build_signal(
                        last_suitable_tile,
                        last_suitable_trackdir,
                        false,
                        signal_ctr,
                    )
                {
                    last_suitable_tile = INVALID_TILE;
                    last_used_ctr = last_suitable_ctr;
                }
                if signal_ctr == last_used_ctr.wrapping_add(signal_density) {
                    if build_signal(tile, trackdir, false, signal_ctr) {
                        last_used_ctr = signal_ctr;
                        last_suitable_tile = INVALID_TILE;
                    }
                } else if build_signal(tile, trackdir, true, signal_ctr) {
                    last_suitable_tile = tile;
                    last_suitable_ctr = signal_ctr;
                    last_suitable_trackdir = trackdir;
                }
            } else if signal_ctr >= last_used_ctr.wrapping_add(signal_density) {
                build_signal(tile, trackdir, false, signal_ctr);
                last_used_ctr = signal_ctr;
            }
            if autofill {
                w::hit(w::RAIL_CMD_SIGNAL_AUTOFILL);
                match self.tile_type(tile) {
                    MP_RAILWAY => {
                        signal_ctr = signal_ctr.wrapping_add(if diagonal(trackdir) {
                            TILE_AXIAL_DISTANCE
                        } else {
                            TILE_CORNER_DISTANCE
                        });
                    }
                    MP_ROAD => signal_ctr = signal_ctr.wrapping_add(TILE_AXIAL_DISTANCE),
                    MP_TUNNELBRIDGE => {
                        w::hit(w::RAIL_CMD_SIGNAL_WORMHOLE);
                        let other = (self.rail.other_tunnel_bridge_end)(tile);
                        let len = self
                            .tunnel_length(tile, other)
                            .wrapping_add(2)
                            .wrapping_mul(TILE_AXIAL_DISTANCE as u32);
                        if remove || minimise_gaps {
                            signal_ctr = signal_ctr.wrapping_add(len as i32);
                        } else {
                            signal_ctr = signal_ctr.wrapping_add(TILE_AXIAL_DISTANCE);
                            let mut i = TILE_AXIAL_DISTANCE as u32;
                            while i < len {
                                if signal_ctr >= last_used_ctr.wrapping_add(signal_density) {
                                    last_used_ctr = signal_ctr;
                                }
                                signal_ctr = signal_ctr.wrapping_add(TILE_AXIAL_DISTANCE);
                                i = i.wrapping_add(TILE_AXIAL_DISTANCE as u32);
                            }
                        }
                    }
                    _ => {}
                }
                if !self.advance_autofill(&mut tile, &mut trackdir, remove) {
                    break;
                }
                if tile == start_tile && trackdir == start_trackdir {
                    break;
                }
            } else {
                if tile == end_tile {
                    break;
                }
                signal_ctr = signal_ctr.wrapping_add(if diagonal(trackdir) {
                    TILE_AXIAL_DISTANCE
                } else {
                    TILE_CORNER_DISTANCE
                });
                tile = self.step(tile, trackdir);
                if !diagonal(trackdir) {
                    trackdir ^= 1;
                }
            }
        }
        if !remove
            && minimise_gaps
            && signal_ctr > last_used_ctr.wrapping_add(signal_density)
            && last_suitable_tile != INVALID_TILE
        {
            build_signal(
                last_suitable_tile,
                last_suitable_trackdir,
                false,
                signal_ctr,
            );
        }
        if !had_success {
            total.take(last_error);
        }
    }

    /// `CmdRemoveSingleSignal`.
    fn remove_signal(&self, cost: Cost<'_>, flags: u16, tile: u32, track: u8) {
        if track >= 6 || !(self.shared.is_plain_rail)(tile) || !self.has_track(tile, track) {
            return cost.error(self.s.strings.no_track);
        }
        if !self.has_signal(tile, track) {
            return cost.error(self.s.strings.no_signals);
        }
        if self.current() != OWNER_WATER {
            // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
            unsafe { (self.command.tile_ownership)(cost.0, tile) };
            if !cost.ok() {
                return;
            }
        }
        if flags & EXECUTE != 0 {
            w::hit(w::RAIL_CMD_SIGNAL_REMOVE);
            let mut v: *mut Train = std::ptr::null_mut();
            if self.reserved(tile) & (1 << track) != 0 {
                v = (self.s.reserving_train)(tile, track);
            } else if is_pbs(self.signal_type(tile, track)) {
                let mut td = track;
                let mut i = 0;
                while v.is_null() && i < 2 {
                    if (self.rail.has_signal_trackdir)(tile, td ^ 8) {
                        let next = self.map.add_diag(tile, TRACKDIR_EXIT[usize::from(td)]);
                        let tracks = trackdir_tracks(reaches(td));
                        let reserved = self.reserved(next);
                        if reserved & tracks != 0 {
                            w::hit(w::RAIL_CMD_SIGNAL_REMOVE_PATH_END);
                            let found =
                                (self.reserved(next) & tracks & 0x3F).trailing_zeros() as u8;
                            v = (self.s.reserving_train)(next, found);
                        }
                    }
                    i += 1;
                    td ^= 8;
                }
            }
            let present = self.present_signals(tile);
            (self.s.signal_infrastructure)(
                (self.shared.tile_owner)(tile),
                present.count_ones().wrapping_neg(),
            );
            let on_track = SIGNAL_ON_TRACK[usize::from(track)];
            (self.rail.set_present_signals)(tile, self.present_signals(tile) & !on_track);
            let present = self.present_signals(tile);
            (self.s.signal_infrastructure)((self.shared.tile_owner)(tile), present.count_ones());
            (self.s.infrastructure_dirty)((self.shared.tile_owner)(tile));
            if self.present_signals(tile) == 0 {
                (self.rail.set_signal_states)(tile, 0);
                (self.rail.set_has_signals)(tile, false);
                (self.rail.set_signal_variant)(tile, INVALID_TRACK, SIG_ELECTRIC);
            }
            self.add_track(tile, track, (self.shared.tile_owner)(tile));
            (self.s.layout_changed)(tile, track);
            if !v.is_null() {
                unsafe { (self.shared.try_path_reserve)(v, false, false) };
            }
            (self.shared.mark_dirty)(tile);
        }
        cost.set(price(PR_CLEAR_SIGNALS));
    }

    /// `CmdConvertRail`.
    fn convert_rail(
        &self,
        frame: *mut ConvertFrame,
        cost: Cost<'_>,
        error: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        area_end: u32,
        area_start: u32,
        totype: u8,
        diagonal_area: bool,
    ) {
        if !(self.s.rail_type_valid)(totype) {
            return cost.error(INVALID_STRING);
        }
        if area_start >= self.map_size() {
            return cost.error(INVALID_STRING);
        }
        let execute = flags & EXECUTE != 0;
        cost.set(0);
        error.error(self.s.strings.no_suitable_track);
        let mut found = false;
        unsafe { (self.s.area_begin)(frame, area_start, area_end, diagonal_area) };
        loop {
            let tile = unsafe { (self.s.area_tile)(frame) };
            if tile == INVALID_TILE {
                break;
            }
            if self.convert_tile(
                frame,
                cost,
                error,
                ret,
                execute,
                tile,
                area_start,
                area_end,
                totype,
                diagonal_area,
            ) {
                found = true;
            }
            unsafe { (self.s.area_next)(frame) };
        }
        if execute {
            unsafe { (self.s.consist_changed)(frame) };
        }
        if !found {
            cost.take(error);
        }
    }
    /// One iteration of the `CmdConvertRail` area loop; true when the tile
    /// set `found_convertible_track`.
    fn convert_tile(
        &self,
        frame: *mut ConvertFrame,
        cost: Cost<'_>,
        error: Cost<'_>,
        ret: Cost<'_>,
        execute: bool,
        tile: u32,
        area_start: u32,
        area_end: u32,
        totype: u8,
        diagonal_area: bool,
    ) -> bool {
        let tt = self.tile_type(tile);
        match tt {
            MP_RAILWAY => {}
            MP_STATION => {
                if !(self.rail.has_station_rail)(tile) {
                    return false;
                }
            }
            MP_ROAD => {
                if !(self.shared.is_level_crossing)(tile) {
                    return false;
                }
                if (self.s.rail_no_crossings)(totype) {
                    error.make_error(self.s.strings.crossing_rail);
                    return false;
                }
            }
            MP_TUNNELBRIDGE => {
                if (self.shared.tunnel_transport)(tile) != 0 {
                    return false;
                }
            }
            _ => return false,
        }
        let kind = (self.rail.rail_type)(tile).0;
        if kind == totype
            || ((self.s.disable_elrails)() && totype == RAILTYPE_RAIL && kind == RAILTYPE_ELECTRIC)
        {
            return false;
        }
        // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
        unsafe { (self.command.tile_ownership)(ret.0, tile) };
        if !ret.ok() {
            error.take(ret);
            return false;
        }
        // At most one entry per reserved track (six), or one for a wormhole.
        // Keep the source insertion order without allocating on each tile.
        let mut affected = [std::ptr::null_mut(); 6];
        let mut affected_len = 0;
        if tt != MP_TUNNELBRIDGE {
            if !(self.s.compatible_rail)(kind, totype) {
                if (self.shared.is_plain_rail)(tile) {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.no_train)(ret.0, tile, self.tracks(tile)) };
                } else {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.no_vehicle)(ret.0, tile) };
                }
                if !ret.ok() {
                    error.take(ret);
                    return false;
                }
            }
            if execute {
                let mut reserved = self.reserved(tile);
                while reserved != 0 {
                    let track = reserved.trailing_zeros() as u8;
                    reserved &= reserved - 1;
                    if let Some(v) = self.reroute(tile, track, totype) {
                        affected[affected_len] = v;
                        affected_len += 1;
                    }
                }
                if !(self.rail.is_rail_station)(tile) || !(self.rail.station_blocked)(tile) {
                    let owner = (self.shared.tile_owner)(tile);
                    let mut num = if (self.shared.is_level_crossing)(tile) {
                        LEVELCROSSING_TRACKBIT_FACTOR
                    } else {
                        1
                    };
                    if (self.shared.is_plain_rail)(tile) {
                        num = pieces(self.tracks(tile));
                    }
                    self.rail_infra(owner, kind, num.wrapping_neg());
                    self.rail_infra(owner, totype, num);
                    (self.s.infrastructure_dirty)(owner);
                }
                (self.rail.set_rail_type)(tile, totype);
                (self.shared.mark_dirty)(tile);
                unsafe { (self.s.collect_trains)(frame, tile) };
            }
        }
        match tt {
            MP_RAILWAY => {
                if (self.rail.rail_tile_type)(tile).0 == RAIL_TILE_DEPOT {
                    if execute {
                        w::hit(w::RAIL_CMD_CONVERT_DEPOT);
                        let dir = (self.rail.rail_depot_direction)(tile).0;
                        (self.s.layout_changed)(tile, dir & 1);
                        (self.s.depot_windows)(tile);
                    }
                    cost.add_money((self.s.rail_convert_cost)(kind, totype));
                } else {
                    if execute {
                        w::hit(w::RAIL_CMD_CONVERT_RAIL);
                        let mut tracks = self.tracks(tile);
                        while tracks != 0 {
                            let track = tracks.trailing_zeros() as u8;
                            tracks &= tracks - 1;
                            (self.s.layout_changed)(tile, track);
                        }
                    }
                    cost.add_money(
                        (self.s.rail_convert_cost)(kind, totype)
                            .saturating_mul(i64::from(self.tracks(tile).count_ones())),
                    );
                }
            }
            MP_TUNNELBRIDGE => {
                let endtile = (self.rail.other_tunnel_bridge_end)(tile);
                if endtile < tile
                    && (self.s.area_contains)(area_start, area_end, diagonal_area, endtile)
                {
                    return false;
                }
                if !(self.s.compatible_rail)((self.rail.rail_type)(tile).0, totype) {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.wormhole_free)(ret.0, tile, endtile, std::ptr::null()) };
                    if !ret.ok() {
                        error.take(ret);
                        return false;
                    }
                }
                if execute {
                    w::hit(w::RAIL_CMD_CONVERT_WORMHOLE);
                    let track = (self.shared.tunnel_bridge_direction)(tile) & 1;
                    if (self.rail.wormhole_reserved)(tile)
                        && let Some(v) = self.reroute(tile, track, totype)
                    {
                        affected[affected_len] = v;
                        affected_len += 1;
                    }
                    let num = self
                        .tunnel_length(tile, endtile)
                        .wrapping_add(2)
                        .wrapping_mul(TUNNELBRIDGE_TRACKBIT_FACTOR);
                    let owner = (self.shared.tile_owner)(tile);
                    self.rail_infra(owner, (self.rail.rail_type)(tile).0, num.wrapping_neg());
                    self.rail_infra(owner, totype, num);
                    (self.s.infrastructure_dirty)(owner);
                    (self.rail.set_rail_type)(tile, totype);
                    (self.rail.set_rail_type)(endtile, totype);
                    unsafe { (self.s.collect_trains)(frame, tile) };
                    unsafe { (self.s.collect_trains)(frame, endtile) };
                    (self.s.layout_changed)(tile, track);
                    (self.s.layout_changed)(endtile, track);
                    if (self.shared.is_bridge)(tile) {
                        (self.shared.mark_bridge_dirty)(tile);
                    } else {
                        (self.shared.mark_dirty)(tile);
                        (self.shared.mark_dirty)(endtile);
                    }
                }
                let length = self.tunnel_length(tile, endtile).wrapping_add(2);
                cost.add_money(
                    (self.s.rail_convert_cost)(kind, totype).saturating_mul(i64::from(length)),
                );
            }
            _ => {
                if execute {
                    w::hit(w::RAIL_CMD_CONVERT_STATION_CROSSING);
                    let track = if tt == MP_STATION {
                        (self.rail.station_track)(tile).0
                    } else {
                        (self.rail.crossing_road_axis)(tile).0 ^ 1
                    };
                    (self.s.layout_changed)(tile, track);
                }
                cost.add_money((self.s.rail_convert_cost)(kind, totype));
            }
        }
        for &v in &affected[..affected_len] {
            unsafe { (self.shared.try_path_reserve)(v, true, false) };
        }
        true
    }
    /// Free an unpowered reserving train and remember it for re-reservation.
    fn reroute(&self, tile: u32, track: u8, totype: u8) -> Option<*mut Train> {
        let v = (self.s.reserving_train)(tile, track);
        if !v.is_null() && !unsafe { (self.s.train_powered)(v, totype) } {
            w::hit(w::RAIL_CMD_CONVERT_REROUTE);
            unsafe { (self.shared.free_train_reservation)(v) };
            Some(v)
        } else {
            None
        }
    }

    /// `RemoveTrainDepot`.
    fn remove_depot(&self, cost: Cost<'_>, flags: u16, tile: u32) {
        if self.current() != OWNER_WATER {
            // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
            unsafe { (self.command.tile_ownership)(cost.0, tile) };
            if !cost.ok() {
                return;
            }
        }
        // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
        unsafe { (self.command.no_vehicle)(cost.0, tile) };
        if !cost.ok() {
            return;
        }
        if flags & EXECUTE != 0 {
            w::hit(w::RAIL_CMD_CLEAR_DEPOT);
            let dir = (self.rail.rail_depot_direction)(tile).0;
            let owner = (self.shared.tile_owner)(tile);
            let mut v: *mut Train = std::ptr::null_mut();
            if (self.rail.depot_reserved)(tile) {
                v = (self.s.reserving_train)(tile, dir & 1);
                if !v.is_null() {
                    unsafe { (self.shared.free_train_reservation)(v) };
                }
            }
            self.rail_infra(owner, (self.rail.rail_type)(tile).0, 1_u32.wrapping_neg());
            (self.s.infrastructure_dirty)(owner);
            (self.s.delete_depot)(tile);
            (self.shared.clear_square)(tile);
            self.add_side(tile, dir, owner);
            (self.s.layout_changed)(tile, dir & 1);
            if !v.is_null() {
                unsafe { (self.shared.try_path_reserve)(v, true, false) };
            }
        }
        cost.set(price(PR_CLEAR_DEPOT_TRAIN));
    }

    /// `ClearTile_Track`.
    fn clear_tile(&self, cost: Cost<'_>, ret: Cost<'_>, flags: u16, tile: u32) {
        cost.set(0);
        if flags & AUTO != 0 {
            if (self.shared.tile_owner)(tile) != self.current() {
                return cost.error(self.s.strings.owned_by_another);
            }
            return cost.error(if (self.shared.is_plain_rail)(tile) {
                self.s.strings.remove_track
            } else {
                self.s.strings.demolish_building
            });
        }
        match (self.rail.rail_tile_type)(tile).0 {
            RAIL_TILE_SIGNALS | RAIL_TILE_NORMAL => {
                w::hit(w::RAIL_CMD_CLEAR_RAIL);
                let tileh = (self.shared.tile_slope)(tile);
                let water_ground = (self.rail.rail_ground)(tile).0 == GROUND_HALF_WATER
                    && one_corner_raised(tileh);
                let mut tracks = self.tracks(tile);
                while tracks != 0 {
                    let track = tracks.trailing_zeros() as u8;
                    tracks &= tracks - 1;
                    unsafe { (self.s.remove_rail)(ret.0, flags, tile, track) };
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                    cost.add_money(ret.money());
                }
                if water_ground
                    && flags & BANKRUPT == 0
                    && (self.shared.company_valid)(self.current())
                {
                    // SAFETY: The entry supplies live native cost slots and source-valid map/train inputs.
                    unsafe { (self.command.no_vehicle)(ret.0, tile) };
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                    if flags & EXECUTE != 0 {
                        w::hit(w::RAIL_CMD_CLEAR_WATER);
                        (self.shared.clear_square)(tile);
                    }
                    cost.add_money(price(PR_CLEAR_WATER));
                }
            }
            RAIL_TILE_DEPOT => self.remove_depot(cost, flags, tile),
            _ => cost.error(INVALID_STRING),
        }
    }

    /// `ChangeTileOwner_Track`.
    fn change_owner(&self, ret: Cost<'_>, tile: u32, old_owner: u8, new_owner: u8) {
        if (self.shared.tile_owner)(tile) != old_owner {
            return;
        }
        if new_owner == INVALID_OWNER {
            w::hit(w::RAIL_CMD_OWNER_CLEAR);
            return (self.command.landscape_clear)(ret.0, tile, EXECUTE | BANKRUPT);
        }
        w::hit(w::RAIL_CMD_OWNER_TRANSFER);
        let num = if (self.shared.is_plain_rail)(tile) {
            pieces(self.tracks(tile))
        } else {
            1
        };
        let railtype = (self.rail.rail_type)(tile).0;
        self.rail_infra(old_owner, railtype, num.wrapping_neg());
        self.rail_infra(new_owner, railtype, num);
        if (self.rail.has_signals)(tile) {
            let signals = self.present_signals(tile).count_ones();
            (self.s.signal_infrastructure)(old_owner, signals.wrapping_neg());
            (self.s.signal_infrastructure)(new_owner, signals);
        }
        (self.rail.set_tile_owner)(tile, new_owner);
    }

    /// `TestAutoslopeOnRailTile`.
    fn test_autoslope(
        &self,
        cost: Cost<'_>,
        flags: u16,
        tile: u32,
        mut z_old: i32,
        mut tileh_old: u8,
        mut z_new: i32,
        mut tileh_new: u8,
        bits: u8,
    ) {
        let remove_track = self.s.strings.remove_track;
        if !(self.shared.build_on_slopes)() || !(self.shared.autoslope)() {
            return cost.error(remove_track);
        }
        self.check_rail_slope(cost, tileh_new, bits, 0, tile);
        if !cost.ok() {
            return cost.error(remove_track);
        }
        // ApplyFoundationToSlope also rewrites the slope in place.
        let top = (self.s.apply_foundation)((self.s.rail_foundation)(tileh_old, bits).0, tileh_old);
        z_old += top.z;
        tileh_old = top.slope;
        let top = (self.s.apply_foundation)((self.s.rail_foundation)(tileh_new, bits).0, tileh_new);
        z_new += top.z;
        tileh_new = top.slope;
        let corner = match bits {
            BIT_LEFT => 0,
            BIT_LOWER => 1,
            BIT_RIGHT => 2,
            BIT_UPPER => 3,
            _ => {
                if z_old != z_new || tileh_old != tileh_new {
                    return cost.error(remove_track);
                }
                return cost.set(price(PR_BUILD_FOUNDATION));
            }
        };
        z_old += (self.s.slope_z_in_corner)(tileh_old & !SLOPE_HALFTILE_MASK, corner);
        z_new += (self.s.slope_z_in_corner)(tileh_new & !SLOPE_HALFTILE_MASK, corner);
        if z_old != z_new {
            return cost.error(remove_track);
        }
        cost.set(price(PR_BUILD_FOUNDATION));
        if tileh_old != tileh_new {
            if (self.rail.rail_ground)(tile).0 == GROUND_HALF_WATER && one_corner_raised(tileh_old)
            {
                cost.add_money(price(PR_CLEAR_WATER));
            }
            if flags & EXECUTE != 0 {
                (self.rail.set_rail_ground)(tile, GROUND_BARREN);
            }
        }
    }

    /// `TerraformTile_Track`.
    fn terraform(&self, cost: Cost<'_>, flags: u16, tile: u32, z_new: i32, tileh_new: u8) {
        let tileh_old = (self.shared.tile_slope)(tile);
        let z_old = (self.shared.tile_z)(tile);
        if (self.shared.is_plain_rail)(tile) {
            let bits = self.tracks(tile);
            let was_water = (self.rail.rail_ground)(tile).0 == GROUND_HALF_WATER
                && one_corner_raised(tileh_old);
            if was_water && (self.s.ship_on_tile)(tile) {
                return cost.error(self.s.strings.ship_in_way);
            }
            self.test_autoslope(cost, flags, tile, z_old, tileh_old, z_new, tileh_new, bits);
            let allowed = match bits {
                BIT_RIGHT => 0,
                BIT_UPPER => 1,
                BIT_LEFT => 2,
                BIT_LOWER => 3,
                _ => return,
            };
            let f_old = (self.s.rail_foundation)(tileh_old, bits).0;
            if tileh_old != SLOPE_NS
                && tileh_old != SLOPE_EW
                && (FOUNDATION_RAIL_W..=FOUNDATION_RAIL_N).contains(&f_old)
            {
                return;
            }
            for corner in 0..4 {
                if corner == allowed {
                    continue;
                }
                if z_old + (self.s.slope_z_in_corner)(tileh_old, corner)
                    != z_new + (self.s.slope_z_in_corner)(tileh_new, corner)
                {
                    return;
                }
            }
            w::hit(w::RAIL_CMD_TERRAFORM_CORNER);
            if flags & EXECUTE != 0 {
                (self.rail.set_rail_ground)(tile, GROUND_BARREN);
            }
            return cost.set(if was_water { price(PR_CLEAR_WATER) } else { 0 });
        }
        if (self.shared.build_on_slopes)()
            && (self.shared.autoslope)()
            && (self.s.entrance_edge)(
                tile,
                z_new,
                tileh_new,
                (self.rail.rail_depot_direction)(tile).0,
            )
        {
            w::hit(w::RAIL_CMD_TERRAFORM_DEPOT);
            return cost.set(price(PR_BUILD_FOUNDATION));
        }
        (self.command.landscape_clear)(cost.0, tile, flags);
    }
}

// SAFETY for every entry: the service table and its nested tables are valid
// immutable process-lifetime tables; every `CommandCost` pointer denotes a
// distinct live native stack object for the synchronous call. The caller holds
// the game-state mutex. Rust holds no reference to native state across calls.

/// `CmdBuildSingleRail`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_build_single(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    b: *mut CommandCost,
    flags: u16,
    tile: u32,
    railtype: u8,
    track: u8,
    auto_remove_signals: bool,
) {
    let x = unsafe { Ctx::new(s) };
    x.build_single_rail(
        x.c(out),
        x.c(a),
        x.c(b),
        flags,
        tile,
        railtype,
        track,
        auto_remove_signals,
    );
}
/// `CmdRemoveSingleRail`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_remove_single(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    flags: u16,
    tile: u32,
    track: u8,
) {
    let x = unsafe { Ctx::new(s) };
    x.remove_single_rail(x.c(out), x.c(a), flags, tile, track);
}
/// `CmdRailTrackHelper` for `CmdBuildRailroadTrack`/`CmdRemoveRailroadTrack`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_track_drag(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    b: *mut CommandCost,
    flags: u16,
    tile: u32,
    end_tile: u32,
    railtype: u8,
    track: u8,
    remove: bool,
    auto_remove_signals: bool,
    fail_on_obstacle: bool,
) {
    let x = unsafe { Ctx::new(s) };
    x.rail_track_helper(
        x.c(out),
        x.c(a),
        x.c(b),
        flags,
        tile,
        end_tile,
        railtype,
        track,
        remove,
        auto_remove_signals,
        fail_on_obstacle,
    );
}
/// `CmdBuildTrainDepot`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_build_depot(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    flags: u16,
    tile: u32,
    railtype: u8,
    dir: u8,
) {
    let x = unsafe { Ctx::new(s) };
    x.build_depot(x.c(out), x.c(a), flags, tile, railtype, dir);
}
/// `CmdBuildSingleSignal`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_build_signal(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    flags: u16,
    tile: u32,
    track: u8,
    sigtype: u8,
    sigvar: u8,
    convert_signal: bool,
    skip_existing_signals: bool,
    ctrl_pressed: bool,
    cycle_start: u8,
    cycle_stop: u8,
    num_dir_cycle: u8,
    signals_copy: u8,
) {
    let x = unsafe { Ctx::new(s) };
    x.build_signal(
        x.c(out),
        x.c(a),
        flags,
        tile,
        track,
        sigtype,
        sigvar,
        convert_signal,
        skip_existing_signals,
        ctrl_pressed,
        cycle_start,
        cycle_stop,
        num_dir_cycle,
        signals_copy,
    );
}
/// `CmdSignalTrackHelper` for `CmdBuildSignalTrack`/`CmdRemoveSignalTrack`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_signal_drag(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    b: *mut CommandCost,
    flags: u16,
    tile: u32,
    end_tile: u32,
    track: u8,
    sigtype: u8,
    sigvar: u8,
    mode: bool,
    remove: bool,
    autofill: bool,
    minimise_gaps: bool,
    signal_density: i32,
) {
    let x = unsafe { Ctx::new(s) };
    x.signal_track_helper(
        x.c(out),
        x.c(a),
        x.c(b),
        flags,
        tile,
        end_tile,
        track,
        sigtype,
        sigvar,
        mode,
        remove,
        autofill,
        minimise_gaps,
        signal_density,
    );
}
/// `CmdRemoveSingleSignal`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_remove_signal(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    track: u8,
) {
    let x = unsafe { Ctx::new(s) };
    x.remove_signal(x.c(out), flags, tile, track);
}
/// `CmdConvertRail`; `frame` is the caller-owned iterator/train-list frame.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_convert(
    s: *const RailCommandServices,
    frame: *mut ConvertFrame,
    out: *mut CommandCost,
    a: *mut CommandCost,
    b: *mut CommandCost,
    flags: u16,
    tile: u32,
    area_start: u32,
    totype: u8,
    diagonal_area: bool,
) {
    let x = unsafe { Ctx::new(s) };
    x.convert_rail(
        frame,
        x.c(out),
        x.c(a),
        x.c(b),
        flags,
        tile,
        area_start,
        totype,
        diagonal_area,
    );
}
/// `ClearTile_Track`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_clear_tile(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    flags: u16,
    tile: u32,
) {
    let x = unsafe { Ctx::new(s) };
    x.clear_tile(x.c(out), x.c(a), flags, tile);
}
/// `ChangeTileOwner_Track`; `a` receives the discarded clear result.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_change_owner(
    s: *const RailCommandServices,
    a: *mut CommandCost,
    tile: u32,
    old_owner: u8,
    new_owner: u8,
) {
    let x = unsafe { Ctx::new(s) };
    x.change_owner(x.c(a), tile, old_owner, new_owner);
}
/// `TerraformTile_Track`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_terraform(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    z_new: i32,
    tileh_new: u8,
) {
    let x = unsafe { Ctx::new(s) };
    x.terraform(x.c(out), flags, tile, z_new, tileh_new);
}
/// `CheckBuildAbove_Track`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_build_above(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
) {
    let x = unsafe { Ctx::new(s) };
    if (x.shared.is_plain_rail)(tile) {
        return x.c(out).zero();
    }
    w::hit(w::RAIL_CMD_BUILD_ABOVE_CLEAR);
    (x.command.landscape_clear)(out, tile, flags);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn track_helpers_follow_source_tables() {
        assert!(!tracks_overlap(BIT_HORZ));
        assert!(!tracks_overlap(BIT_VERT));
        assert!(tracks_overlap(BIT_X | BIT_Y));
        assert_eq!(pieces(BIT_X | BIT_Y), 4);
        assert_eq!(pieces(BIT_HORZ), 2);
        assert_eq!(trackdir_tracks(reaches(0)), BIT_X | BIT_LOWER | BIT_LEFT);
        assert_eq!(negate(i64::MIN), i64::MAX);
        assert_eq!(kill_first_bit(0xC), 0x8);
    }
}
