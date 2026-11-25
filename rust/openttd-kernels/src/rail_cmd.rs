/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The rail track, depot and signal commands of `rail_cmd.cpp`, with the rail clear, owner,
//! terraform and build-above tile procs. Map writers, pools, company counters and every
//! `CommandCost` stay native. Nested commands run here under `command::nested` and may
//! reenter this module. No entry draws `Random`.
//!
//! Every entry builds one [`Ctx`] over the immutable process-lifetime tables and holds the
//! game-state mutex; Rust holds no reference to native state across a call, and every
//! `CommandCost` pointer is a distinct live native object for the synchronous call.
#![allow(
    unsafe_code,
    clippy::too_many_lines,
    reason = "the native command tables, cost slots and train records; the original's long command bodies"
)]
use crate::bitmath_func::{has_bit, has_exactly_one_bit, kill_first_bit, set_bit8, toggle_bit8};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_base::CompanyInfrastructure;
use crate::company_type::{CompanyID, Owner};
use crate::container_func::include;
use crate::depot_func::can_build_depot_by_tileh;
use crate::direction_func::is_valid_diag_direction;
use crate::direction_type::{Axis, DiagDirection};
use crate::economy_type::{LEVELCROSSING_TRACKBIT_FACTOR, Price};
use crate::map_func::{Map, World};
use crate::map_type::TileIndexDiffC;
use crate::movement::get_rail_foundation;
use crate::newgrf::GrfSpecFeature;
use crate::newgrf_debug::delete_newgrf_inspect_window;
use crate::order_type::OrderType;
use crate::pbs::Pbs;
use crate::rail::{
    has_power_on_rail, has_power_on_rail_types, is_compatible_rail, rail_build_cost,
    rail_clear_cost, rail_convert_cost, rail_no_level_crossings, val_param_rail_type,
};
use crate::rail_map::{RailGroundType, RailTileType, is_pbs_signal};
use crate::rail_type::RailType;
use crate::rail_yapf::yapf_notify_track_layout_change;
use crate::road_type::{DisallowedRoadDirections, RoadBits, RoadType};
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Cost, RailMapServices, Services,
};
use crate::signal_func::{signal_against_trackdir, signal_along_trackdir, signal_on_track};
use crate::signal_type::{SignalType, SignalVariant};
use crate::slope_func::{
    is_slope_with_one_corner_raised, is_special_rail_foundation, is_steep_slope,
    remove_halftile_slope,
};
use crate::slope_type::{Corner, Foundation, Slope, VALID_LEVEL_CROSSING_SLOPES};
use crate::strings_type::INVALID_STRING_ID;
use crate::table::strings::{
    STR_ERROR_ALREADY_BUILT, STR_ERROR_AREA_IS_OWNED_BY_ANOTHER,
    STR_ERROR_BUILDING_MUST_BE_DEMOLISHED, STR_ERROR_CAN_T_BUILD_ON_WATER,
    STR_ERROR_CROSSING_DISALLOWED_RAIL, STR_ERROR_CROSSING_DISALLOWED_ROAD,
    STR_ERROR_CROSSING_ON_ONEWAY_ROAD, STR_ERROR_FLAT_LAND_REQUIRED,
    STR_ERROR_IMPOSSIBLE_TRACK_COMBINATION, STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION,
    STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST, STR_ERROR_MUST_REMOVE_RAILROAD_TRACK,
    STR_ERROR_MUST_REMOVE_SIGNALS_FIRST, STR_ERROR_NO_SUITABLE_RAILROAD_TRACK, STR_ERROR_OWNED_BY,
    STR_ERROR_ROAD_WORKS_IN_PROGRESS, STR_ERROR_SHIP_IN_THE_WAY, STR_ERROR_THERE_ARE_NO_SIGNALS,
    STR_ERROR_THERE_IS_NO_RAILROAD_TRACK,
};
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{
    diag_dir_to_diag_track, is_diagonal_trackdir, is_valid_track, remove_first_track,
    remove_first_trackdir, reverse_trackdir, track_bits_to_track, track_status_to_trackdir_bits,
    track_to_track_bits, track_to_trackdir, trackdir_bits_to_track_bits,
    trackdir_reaches_trackdirs, trackdir_to_exitdir, trackdir_to_track, tracks_overlap,
};
use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
use crate::train::{CCF_TRACK, Handle};
use crate::transport_type::TransportType;
use crate::vehicle_base::{TILE_AXIAL_DISTANCE, TILE_CORNER_DISTANCE, VehState};
use crate::vehicle_func::{has_vehicle_on_tile, vehicles_on_tile};
use crate::vehicle_type::VehicleType;
use crate::window_func::invalidate_window_data;
use crate::window_type::{WindowClass, WindowNumber};
use crate::witness as w;

/// `MakeRoadNormal`'s arguments when a level crossing is removed. `MakeRoadNormal` stays a
/// native map writer; class types cross as their base integers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrossingRoad {
    /// `GetCrossingRoadBits`.
    pub bits: RoadBits,
    /// `GetRoadTypeRoad`.
    pub road_type: RoadType,
    /// `GetRoadTypeTram`.
    pub tram_type: RoadType,
    /// `GetTownIndex`.
    pub town: u16,
    /// `GetRoadOwner(RTT_ROAD)`.
    pub road_owner: u8,
    /// `GetRoadOwner(RTT_TRAM)`.
    pub tram_owner: u8,
}

/// Immutable native service table: the native callees of the rail commands that have no
/// Rust port, and those of the road, tunnel, water, depot and station commands that share
/// the table.
#[repr(C)]
pub struct RailCommandServices {
    pub shared: *const Services,
    pub rail: *const RailMapServices,
    pub cost: *const CommandCostServices,
    pub command: *const CommandServices,
    /// Signal-buffer occupancy and assertion services (`_signal_services`).
    pub signal: *const crate::signal::SignalServices,
    /// The train table, for the reservation and level crossing owners.
    pub train: *const crate::train::Leaves,
    /// `RoadNoLevelCrossing`.
    pub road_no_crossing: extern "C" fn(u8) -> bool,
    /// `construction.crossing_with_competitor`.
    pub crossing_with_competitor: extern "C" fn() -> bool,
    /// `AutoslopeCheckForEntranceEdge`.
    pub entrance_edge: extern "C" fn(u32, i32, u8, u8) -> bool,
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
}

/// `_valid_tracks_on_leveled_foundation`: valid `TrackBits` on a specific (non-steep)-slope
/// with leveled foundation.
pub(crate) const VALID_TRACKS_ON_LEVELED_FOUNDATION: [TrackBits; 15] = [
    TrackBits::TRACK_BIT_NONE,
    TrackBits::TRACK_BIT_LEFT,
    TrackBits::TRACK_BIT_LOWER,
    TrackBits(
        TrackBits::TRACK_BIT_Y.0 | TrackBits::TRACK_BIT_LOWER.0 | TrackBits::TRACK_BIT_LEFT.0,
    ),
    TrackBits::TRACK_BIT_RIGHT,
    TrackBits::TRACK_BIT_ALL,
    TrackBits(
        TrackBits::TRACK_BIT_X.0 | TrackBits::TRACK_BIT_LOWER.0 | TrackBits::TRACK_BIT_RIGHT.0,
    ),
    TrackBits::TRACK_BIT_ALL,
    TrackBits::TRACK_BIT_UPPER,
    TrackBits(
        TrackBits::TRACK_BIT_X.0 | TrackBits::TRACK_BIT_UPPER.0 | TrackBits::TRACK_BIT_LEFT.0,
    ),
    TrackBits::TRACK_BIT_ALL,
    TrackBits::TRACK_BIT_ALL,
    TrackBits(
        TrackBits::TRACK_BIT_Y.0 | TrackBits::TRACK_BIT_UPPER.0 | TrackBits::TRACK_BIT_RIGHT.0,
    ),
    TrackBits::TRACK_BIT_ALL,
    TrackBits::TRACK_BIT_ALL,
];

/// `_trackdelta`, indexed by trackdir.
const TRACKDELTA: [TileIndexDiffC; 16] = [
    TileIndexDiffC { x: -1, y: 0 },
    TileIndexDiffC { x: 0, y: 1 },
    TileIndexDiffC { x: -1, y: 0 },
    TileIndexDiffC { x: 0, y: 1 },
    TileIndexDiffC { x: 1, y: 0 },
    TileIndexDiffC { x: 0, y: 1 },
    TileIndexDiffC { x: 0, y: 0 },
    TileIndexDiffC { x: 0, y: 0 },
    TileIndexDiffC { x: 1, y: 0 },
    TileIndexDiffC { x: 0, y: -1 },
    TileIndexDiffC { x: 0, y: -1 },
    TileIndexDiffC { x: 1, y: 0 },
    TileIndexDiffC { x: 0, y: -1 },
    TileIndexDiffC { x: -1, y: 0 },
    TileIndexDiffC { x: 0, y: 0 },
    TileIndexDiffC { x: 0, y: 0 },
];

/// The infrastructure pieces of plain rail `bits`: the original's `CountBits`, squared
/// when the tracks overlap.
fn pieces(bits: TrackBits) -> u32 {
    let pieces = bits.0.count_ones();
    if tracks_overlap(bits) {
        pieces * pieces
    } else {
        pieces
    }
}

/// `CmdBuildSingleRail`'s parameters.
#[derive(Clone, Copy)]
pub(crate) struct CmdBuildSingleRail {
    /// Tile to build on.
    pub tile: TileIndex,
    /// Railtype of the piece being built (normal, mono, maglev).
    pub railtype: RailType,
    /// Track orientation.
    pub track: Track,
    /// False is an error on a signal in the way, true removes signals in the way.
    pub auto_remove_signals: bool,
}

/// `CmdRemoveSingleRail`'s parameters.
#[derive(Clone, Copy)]
pub(crate) struct CmdRemoveSingleRail {
    /// Tile to remove the track from.
    pub tile: TileIndex,
    /// Rail orientation.
    pub track: Track,
}

/// `CmdRailTrackHelper`'s parameters.
#[derive(Clone, Copy)]
struct RailTrackHelper {
    /// Start tile of the drag.
    tile: TileIndex,
    /// End tile of the drag.
    end_tile: TileIndex,
    /// Railroad type normal/maglev, used only for building.
    railtype: RailType,
    /// Track orientation.
    track: Track,
    /// Remove tracks?
    remove: bool,
    /// False is an error on a signal in the way, true removes signals in the way; used only
    /// for building.
    auto_remove_signals: bool,
    /// False builds from and up to an obstacle, true fails on an obstacle (used for AIs).
    fail_on_obstacle: bool,
}

/// `CmdBuildTrainDepot`'s parameters.
#[derive(Clone, Copy)]
struct CmdBuildTrainDepot {
    /// Position of the train depot.
    tile: TileIndex,
    /// Rail type.
    railtype: RailType,
    /// Entrance direction.
    dir: DiagDirection,
}

/// `CmdBuildSingleSignal`'s parameters.
#[derive(Clone, Copy)]
struct CmdBuildSingleSignal {
    /// Tile where to build the signals.
    tile: TileIndex,
    /// Track orientation.
    track: Track,
    /// Type of the signal.
    sigtype: SignalType,
    /// Variant of signal type (normal/semaphore).
    sigvar: SignalVariant,
    /// Convert the present signal type and variant.
    convert_signal: bool,
    /// True doesn't modify an existing signal but doesn't fail either, false always sets the
    /// new signal type.
    skip_existing_signals: bool,
    /// True overrides signal/semaphore, or pre/exit/combo signal or toggles the variant
    /// (CTRL-toggle).
    ctrl_pressed: bool,
    /// Start the cycle from this signal type.
    cycle_start: SignalType,
    /// Wrap around after this signal type.
    cycle_stop: SignalType,
    /// Cycle the signal direction this many times.
    num_dir_cycle: u8,
    /// Used for `CmdBuildManySignals()` to copy the direction of the first signal.
    signals_copy: u8,
}

/// `CmdSignalTrackHelper`'s parameters.
#[derive(Clone, Copy)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "CmdSignalTrackHelper's four bool parameters"
)]
struct SignalTrackHelper {
    /// Start tile of the drag.
    tile: TileIndex,
    /// End tile of the drag.
    end_tile: TileIndex,
    /// Track orientation.
    track: Track,
    /// Default signal type.
    sigtype: SignalType,
    /// Signal variant to build.
    sigvar: SignalVariant,
    /// True overrides signal/semaphore, or pre/exit/combo signal (CTRL-toggle).
    mode: bool,
    /// Remove signals?
    remove: bool,
    /// Fill beyond the selected stretch?
    autofill: bool,
    /// False keeps a fixed distance, true minimises the gaps between signals.
    minimise_gaps: bool,
    /// User defined signal density.
    signal_density: i32,
}

/// `CmdRemoveSingleSignal`'s parameters.
#[derive(Clone, Copy)]
struct CmdRemoveSingleSignal {
    /// Coordinates where the signal is being deleted from.
    tile: TileIndex,
    /// Track orientation.
    track: Track,
}

/// `CmdConvertRail`'s parameters.
#[derive(Clone, Copy)]
struct CmdConvertRail {
    /// End tile of the rail conversion drag.
    tile: TileIndex,
    /// Start tile of the drag.
    area_start: TileIndex,
    /// New railtype to convert to.
    totype: RailType,
    /// Build diagonally or not.
    diagonal: bool,
}

/// `TestAutoslopeOnRailTile`'s parameters.
#[derive(Clone, Copy)]
struct TestAutoslopeOnRailTile {
    /// The tile.
    tile: TileIndex,
    /// The command flags.
    flags: DoCommandFlags,
    /// Old `TileZ`.
    z_old: i32,
    /// Old `TileSlope`.
    tileh_old: Slope,
    /// New `TileZ`.
    z_new: i32,
    /// New `TileSlope`.
    tileh_new: Slope,
    /// Trackbits.
    rail_bits: TrackBits,
}

/// One synchronous rail command entry: its tables, map descriptor and PBS context.
struct Ctx<'a> {
    s: &'a RailCommandServices,
    shared: &'a Services,
    rail: &'a RailMapServices,
    ground: &'a crate::ground::RailServices,
    cost: &'a CommandCostServices,
    command: &'a CommandServices,
    train: &'a crate::train::Leaves,
    world: World<'a>,
    pbs: Pbs<'a>,
}
impl<'a> Ctx<'a> {
    /// # Safety
    /// The table and its nested tables are valid for the synchronous call.
    unsafe fn new(s: *const RailCommandServices) -> Self {
        // SAFETY: native immutable process-lifetime tables.
        let s = unsafe { &*s };
        let shared = unsafe { &*s.shared };
        Self {
            s,
            shared,
            rail: unsafe { &*s.rail },
            ground: unsafe { &*(*s.rail).ground },
            cost: unsafe { &*s.cost },
            command: unsafe { &*s.command },
            train: unsafe { &*s.train },
            world: World::new(shared),
            pbs: Pbs::acquire(shared),
        }
    }
    /// The native assertion of the shared header helpers.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        move |ok| self.world.check(ok)
    }
    fn map(&self) -> Map {
        self.world.dimensions
    }
    fn c(&self, cost: *mut CommandCost) -> Cost<'a> {
        Cost(cost, self.cost)
    }
    /// `_current_company`.
    fn current_company(&self) -> CompanyID {
        CompanyID((self.shared.current_company)())
    }
    /// The signal buffer over this entry's tables.
    fn signals(&self) -> crate::signal::Context {
        crate::signal::Context {
            shared: self.s.shared,
            rail: self.s.rail,
            native: self.s.signal,
        }
    }
    /// The path reservation context of the train table.
    fn reservation(&self) -> crate::train_reservation::Game<'_> {
        crate::train_reservation::Game::new(self.train, self.shared)
    }
    /// `FreeTrainTrackReservation`.
    fn free_train_track_reservation(&self, v: Handle) {
        crate::train_reservation::free_train_track_reservation(&self.reservation(), v);
    }
    /// `TryPathReserve`.
    fn try_path_reserve(&self, v: Handle, mark_as_stuck: bool) {
        crate::train_reservation::try_path_reserve(&self.reservation(), v, mark_as_stuck, false);
    }
    /// `DirtyCompanyInfrastructureWindows`.
    fn dirty_company_infrastructure_windows(&self, company: CompanyID) {
        (self.command.infrastructure_dirty)(company.0);
    }
    /// `MarkTileDirtyByTile`.
    fn mark_tile_dirty_by_tile(&self, tile: TileIndex) {
        (self.shared.mark_dirty)(tile.0);
    }
    /// `Command<CMD_LANDSCAPE_CLEAR>::Do(flags, tile)` into `out`.
    fn landscape_clear(&self, out: Cost<'_>, flags: DoCommandFlags, tile: TileIndex) {
        crate::landscape_cmd::clear_command(self.command, out.0, tile.0, flags.0);
    }
    /// A nested `Command<...>::Do(flags, tile, ...)` running `body` with the nested
    /// command's flags.
    fn nested(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
        mut body: impl FnMut(DoCommandFlags),
    ) {
        crate::command::nested(self.command, self.shared, out, tile.0, flags.0, |flags| {
            // Each run of a nested body is a command entry, as the native dispatch's was.
            w::hit(w::RAIL_CMD_ENTRIES);
            body(DoCommandFlags(flags));
        });
    }
    /// `Command<CMD_BUILD_SINGLE_RAIL>::Do(flags, ...)` into `out`.
    fn build_single_rail_command(
        &self,
        out: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildSingleRail,
    ) {
        // SAFETY: the scratch results live only for the synchronous nested command.
        unsafe {
            crate::command::with_cost(self.cost, |ret| {
                crate::command::with_cost(self.cost, |other| {
                    self.nested(out, args.tile, flags, |flags| {
                        self.cmd_build_single_rail(out, ret, other, flags, args);
                    });
                });
            });
        }
    }
    /// `Command<CMD_REMOVE_SINGLE_RAIL>::Do(flags, ...)` into `out`.
    fn remove_single_rail_command(
        &self,
        out: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveSingleRail,
    ) {
        // SAFETY: the scratch result lives only for the synchronous nested command.
        unsafe {
            crate::command::with_cost(self.cost, |ret| {
                self.nested(out, args.tile, flags, |flags| {
                    self.cmd_remove_single_rail(out, ret, flags, args);
                });
            });
        }
    }
    /// `Command<CMD_BUILD_SINGLE_SIGNAL>::Do(flags, ...)` into `out`.
    fn build_single_signal_command(
        &self,
        out: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildSingleSignal,
    ) {
        // SAFETY: the scratch result lives only for the synchronous nested command.
        unsafe {
            crate::command::with_cost(self.cost, |ret| {
                self.nested(out, args.tile, flags, |flags| {
                    self.cmd_build_single_signal(out, ret, flags, args);
                });
            });
        }
    }
    /// `Command<CMD_REMOVE_SINGLE_SIGNAL>::Do(flags, ...)` into `out`.
    fn remove_single_signal_command(
        &self,
        out: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveSingleSignal,
    ) {
        self.nested(out, args.tile, flags, |flags| {
            self.cmd_remove_single_signal(out, flags, args);
        });
    }
    /// `Command<CMD_CONVERT_RAIL>::Do(flags, ...)` into `out`.
    fn convert_rail_command(&self, out: Cost<'_>, flags: DoCommandFlags, args: CmdConvertRail) {
        // SAFETY: the scratch results live only for the synchronous nested command.
        unsafe {
            crate::command::with_cost(self.cost, |error| {
                crate::command::with_cost(self.cost, |ret| {
                    self.nested(out, args.tile, flags, |flags| {
                        self.cmd_convert_rail(out, error, ret, flags, args);
                    });
                });
            });
        }
    }

    /// `EnsureNoTrainOnTrack`: tests if a vehicle interacts with the specified track.
    /// All track bits interact except parallel `TRACK_BIT_HORZ` or `TRACK_BIT_VERT`.
    /// Writes a succeeded command (no train found), or a failed command (a train was found).
    fn ensure_no_train_on_track(&self, out: Cost<'_>, tile: TileIndex, track: Track) {
        let rail_bits = track_to_track_bits(track, self.check());
        self.command.no_train(out.0, tile, rail_bits);
    }

    /// `CheckTrackCombination`: check that the new track bits may be built.
    fn check_track_combination(&self, out: Cost<'_>, tile: TileIndex, to_build: TrackBits) {
        if !self.world.is_plain_rail(tile) {
            return out.error(STR_ERROR_IMPOSSIBLE_TRACK_COMBINATION.0);
        }

        /* So, we have a tile with tracks on it (and possibly signals). Let's see
         * what tracks first */
        let current = self.world.get_track_bits(tile); // The current track layout.
        let future = current | to_build; // The track layout we want to build.

        /* Are we really building something new? */
        if current == future {
            /* Nothing new is being built */
            return out.error(STR_ERROR_ALREADY_BUILT.0);
        }

        /* Normally, we may overlap and any combination is valid */
        out.zero();
    }

    /// `CheckRailSlope`: tests if a track can be built on a tile; writes the error message or
    /// the cost of building the foundation. `tile` is used for the water test.
    fn check_rail_slope(
        &self,
        out: Cost<'_>,
        tileh: Slope,
        rail_bits: TrackBits,
        existing: TrackBits,
        tile: TileIndex,
    ) {
        /* don't allow building on the lower side of a coast */
        // SAFETY: Immutable shared tables outlive this synchronous command entry.
        if unsafe {
            crate::water::get_flooding_behaviour(tile, (self.shared.water_services)(), self.shared)
        } != crate::water::FloodingBehaviour::FLOOD_NONE
            && !is_steep_slope(tileh)
            && (!VALID_TRACKS_ON_LEVELED_FOUNDATION[usize::from(tileh.0)] & (rail_bits | existing))
                != TrackBits::TRACK_BIT_NONE
        {
            return out.error(STR_ERROR_CAN_T_BUILD_ON_WATER.0);
        }

        let f_new = get_rail_foundation(tileh, rail_bits | existing, self.check());

        /* check track/slope combination */
        if f_new == Foundation::FOUNDATION_INVALID
            || (f_new != Foundation::FOUNDATION_NONE && !(self.shared.build_on_slopes)())
        {
            return out.error(STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION.0);
        }

        let f_old = get_rail_foundation(tileh, existing, self.check());
        out.set(if f_new == f_old {
            0
        } else {
            crate::company::price(Price::PR_BUILD_FOUNDATION)
        });
    }

    /// `CmdBuildSingleRail`: build a single piece of rail.
    fn cmd_build_single_rail(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        ret_remove_signals: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildSingleRail,
    ) {
        let CmdBuildSingleRail {
            tile,
            railtype,
            track,
            auto_remove_signals,
        } = args;
        cost.set(0);

        if !val_param_rail_type(self.shared, railtype) || !is_valid_track(track) {
            return cost.error(INVALID_STRING_ID);
        }

        let tileh = self.world.tile_slope(tile);
        let trackbit = track_to_track_bits(track, self.check());

        'switch: {
            match self.world.kind(tile) {
                TileType::MP_RAILWAY => {
                    ret.tile_ownership(self.command, tile.0);
                    if !ret.ok() {
                        return cost.take(ret);
                    }

                    if !self.world.is_plain_rail(tile) {
                        w::hit(w::RAIL_CMD_BUILD_NOT_PLAIN);
                        return self.landscape_clear(cost, flags, tile); // just get appropriate error message
                    }

                    if !is_compatible_rail(self.shared, self.world.get_rail_type(tile), railtype) {
                        return cost.error(STR_ERROR_IMPOSSIBLE_TRACK_COMBINATION.0);
                    }

                    self.check_track_combination(ret, tile, trackbit);
                    if ret.ok() {
                        self.ensure_no_train_on_track(ret, tile, track);
                    }
                    if !ret.ok() {
                        return cost.take(ret);
                    }

                    self.check_rail_slope(
                        ret,
                        tileh,
                        trackbit,
                        self.world.get_track_bits(tile),
                        tile,
                    );
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                    cost.add_money(ret.money());

                    if self.world.has_signals(tile)
                        && tracks_overlap(
                            self.world.get_track_bits(tile)
                                | track_to_track_bits(track, self.check()),
                        )
                    {
                        /* If adding the new track causes any overlap, all signals must be removed first */
                        if !auto_remove_signals {
                            return cost.error(STR_ERROR_MUST_REMOVE_SIGNALS_FIRST.0);
                        }

                        w::hit(w::RAIL_CMD_BUILD_REMOVE_SIGNALS);
                        for track_iter in Track::TRACK_BEGIN.0..Track::TRACK_END.0 {
                            // The original's `track_it`.
                            let track_iter = Track(track_iter);
                            if self.world.has_track(tile, track_iter)
                                && self.world.has_signal_on_track(tile, track_iter)
                            {
                                self.remove_single_signal_command(
                                    ret_remove_signals,
                                    flags,
                                    CmdRemoveSingleSignal {
                                        tile,
                                        track: track_iter,
                                    },
                                );
                                if !ret_remove_signals.ok() {
                                    return cost.take(ret_remove_signals);
                                }
                                cost.add_money(ret_remove_signals.money());
                            }
                        }
                    }

                    /* If the rail types don't match, try to convert only if engines of
                     * the new rail type are not powered on the present rail type and engines of
                     * the present rail type are powered on the new rail type. */
                    if self.world.get_rail_type(tile) != railtype
                        && !has_power_on_rail(self.shared, railtype, self.world.get_rail_type(tile))
                    {
                        if has_power_on_rail(self.shared, self.world.get_rail_type(tile), railtype)
                        {
                            w::hit(w::RAIL_CMD_BUILD_CONVERT);
                            self.convert_rail_command(
                                ret,
                                flags,
                                CmdConvertRail {
                                    tile,
                                    area_start: tile,
                                    totype: railtype,
                                    diagonal: false,
                                },
                            );
                            if !ret.ok() {
                                return cost.take(ret);
                            }
                            cost.add_money(ret.money());
                        } else {
                            return cost.error(INVALID_STRING_ID);
                        }
                    }

                    if flags.test(DoCommandFlag::Execute) {
                        w::hit(w::RAIL_CMD_BUILD_RAIL);
                        (self.ground.set_ground)(tile.0, RailGroundType::Barren);
                        let bits = self.world.get_track_bits(tile);
                        (self.rail.set_track_bits)(tile.0, (bits | trackbit).0);
                        let owner = self.world.owner(tile);
                        let infrastructure = CompanyInfrastructure::get(self.shared, owner);
                        // SAFETY: the company is live and serialized; the rail type indexes
                        // its infrastructure array.
                        unsafe {
                            /* Subtract old infrastructure count. */
                            CompanyInfrastructure::add_rail(
                                infrastructure,
                                self.world.get_rail_type(tile),
                                pieces(bits).wrapping_neg(),
                            );
                            /* Add new infrastructure count. */
                            CompanyInfrastructure::add_rail(
                                infrastructure,
                                self.world.get_rail_type(tile),
                                pieces(bits | trackbit),
                            );
                        }
                        self.dirty_company_infrastructure_windows(self.world.owner(tile));
                    }
                    break 'switch;
                }

                TileType::MP_ROAD => {
                    /* Level crossings may only be built on these slopes */
                    if !has_bit(VALID_LEVEL_CROSSING_SLOPES, tileh.0) {
                        return cost.error(STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION.0);
                    }

                    if !(self.s.crossing_with_competitor)()
                        && self.current_company() != Owner::DEITY
                    {
                        ret.tile_ownership(self.command, tile.0);
                        if !ret.ok() {
                            return cost.take(ret);
                        }
                    }

                    self.command.no_vehicle(ret.0, tile);
                    if !ret.ok() {
                        return cost.take(ret);
                    }

                    if self.world.is_normal_road(tile.0) {
                        if self.world.has_road_works(tile.0) {
                            return cost.error(STR_ERROR_ROAD_WORKS_IN_PROGRESS.0);
                        }

                        if DisallowedRoadDirections(self.world.disallowed_road_directions(tile.0))
                            != DisallowedRoadDirections::DRD_NONE
                        {
                            return cost.error(STR_ERROR_CROSSING_ON_ONEWAY_ROAD.0);
                        }

                        if rail_no_level_crossings(self.shared, railtype) {
                            return cost.error(STR_ERROR_CROSSING_DISALLOWED_RAIL.0);
                        }

                        let roadtype_road = RoadType(self.world.road_type(tile.0, false));
                        let roadtype_tram = RoadType(self.world.road_type(tile.0, true));

                        if roadtype_road != RoadType::INVALID_ROADTYPE
                            && (self.s.road_no_crossing)(roadtype_road.0)
                        {
                            return cost.error(STR_ERROR_CROSSING_DISALLOWED_ROAD.0);
                        }
                        if roadtype_tram != RoadType::INVALID_ROADTYPE
                            && (self.s.road_no_crossing)(roadtype_tram.0)
                        {
                            return cost.error(STR_ERROR_CROSSING_DISALLOWED_ROAD.0);
                        }

                        let road = RoadBits(self.world.road_bits(tile.0, false));
                        let tram = RoadBits(self.world.road_bits(tile.0, true));
                        if (track == Track::TRACK_X
                            && (road | tram) & RoadBits::ROAD_X == RoadBits::ROAD_NONE)
                            || (track == Track::TRACK_Y
                                && (road | tram) & RoadBits::ROAD_Y == RoadBits::ROAD_NONE)
                        {
                            let road_owner = CompanyID(self.world.road_owner(tile.0, false));
                            let tram_owner = CompanyID(self.world.road_owner(tile.0, true));
                            /* Disallow breaking end-of-line of someone else
                             * so trams can still reverse on this tile. */
                            if (self.shared.company_valid)(tram_owner.0)
                                && has_exactly_one_bit(tram.0)
                            {
                                ret.ownership(self.command, tram_owner.0, TileIndex::INVALID.0);
                                if !ret.ok() {
                                    return cost.take(ret);
                                }
                            }

                            let num_new_road_pieces = if road == RoadBits::ROAD_NONE {
                                0
                            } else {
                                2_u32.wrapping_sub(road.0.count_ones())
                            };
                            if num_new_road_pieces > 0 {
                                cost.add_money(
                                    crate::road_cost::build_cost(self.shared, roadtype_road.0)
                                        .saturating_mul(i64::from(num_new_road_pieces)),
                                );
                            }

                            let num_new_tram_pieces = if tram == RoadBits::ROAD_NONE {
                                0
                            } else {
                                2_u32.wrapping_sub(tram.0.count_ones())
                            };
                            if num_new_tram_pieces > 0 {
                                cost.add_money(
                                    crate::road_cost::build_cost(self.shared, roadtype_tram.0)
                                        .saturating_mul(i64::from(num_new_tram_pieces)),
                                );
                            }

                            if flags.test(DoCommandFlag::Execute) {
                                w::hit(w::RAIL_CMD_BUILD_CROSSING);
                                (self.s.make_crossing)(
                                    tile.0,
                                    road_owner.0,
                                    tram_owner.0,
                                    self.current_company().0,
                                    if track == Track::TRACK_X {
                                        Axis::AXIS_Y.0
                                    } else {
                                        Axis::AXIS_X.0
                                    },
                                    railtype.0,
                                    roadtype_road.0,
                                    roadtype_tram.0,
                                    (self.shared.town_index)(tile.0),
                                );
                                crate::train::update_level_crossing(
                                    self.train,
                                    self.shared,
                                    tile,
                                    false,
                                    false,
                                );
                                crate::train::mark_dirty_adjacent_level_crossing_tiles(
                                    self.train,
                                    self.shared,
                                    tile,
                                    Axis(self.world.crossing_road_axis(tile.0)),
                                );
                                // SAFETY: the company is live and serialized; the types
                                // index its infrastructure arrays.
                                unsafe {
                                    CompanyInfrastructure::add_rail(
                                        CompanyInfrastructure::get(
                                            self.shared,
                                            self.current_company(),
                                        ),
                                        railtype,
                                        LEVELCROSSING_TRACKBIT_FACTOR,
                                    );
                                }
                                self.dirty_company_infrastructure_windows(self.current_company());
                                if num_new_road_pieces > 0
                                    && (self.shared.company_valid)(road_owner.0)
                                {
                                    // SAFETY: as above.
                                    unsafe {
                                        CompanyInfrastructure::add_road(
                                            CompanyInfrastructure::get(self.shared, road_owner),
                                            roadtype_road,
                                            num_new_road_pieces,
                                        );
                                    }
                                    self.dirty_company_infrastructure_windows(road_owner);
                                }
                                if num_new_tram_pieces > 0
                                    && (self.shared.company_valid)(tram_owner.0)
                                {
                                    // SAFETY: as above.
                                    unsafe {
                                        CompanyInfrastructure::add_road(
                                            CompanyInfrastructure::get(self.shared, tram_owner),
                                            roadtype_tram,
                                            num_new_tram_pieces,
                                        );
                                    }
                                    self.dirty_company_infrastructure_windows(tram_owner);
                                }
                            }
                            break 'switch;
                        }
                    }

                    if self.world.level_crossing(tile.0)
                        && TrackBits(self.world.crossing_rail_bits(tile.0)) == trackbit
                    {
                        return cost.error(STR_ERROR_ALREADY_BUILT.0);
                    }
                    // Falls through to the default case.
                }

                _ => {}
            }

            /* Will there be flat water on the lower halftile? */
            let water_ground = self.world.is_tile_type(tile, TileType::MP_WATER)
                && is_slope_with_one_corner_raised(tileh);

            self.check_rail_slope(ret, tileh, trackbit, TrackBits::TRACK_BIT_NONE, tile);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());

            self.landscape_clear(ret, flags, tile);
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());

            if water_ground {
                cost.add_money(crate::company::price(Price::PR_CLEAR_WATER).saturating_neg());
                cost.add_money(crate::company::price(Price::PR_CLEAR_ROUGH));
            }

            if flags.test(DoCommandFlag::Execute) {
                w::hit(w::RAIL_CMD_BUILD_CLEAR);
                (self.rail.make_rail_normal)(
                    tile.0,
                    self.current_company().0,
                    trackbit.0,
                    railtype.0,
                );
                if water_ground {
                    w::hit(w::RAIL_CMD_BUILD_HALF_WATER);
                    (self.ground.set_ground)(tile.0, RailGroundType::HalfTileWater);
                    // SAFETY: borrowed process-lifetime services; no owner borrow spans docking.
                    unsafe {
                        crate::station_cmd::check_possible_docking(
                            (self.shared.station_commands)(),
                            tile,
                        );
                    }
                }
                // SAFETY: the company is live and serialized; the rail type indexes its
                // infrastructure array.
                unsafe {
                    CompanyInfrastructure::add_rail(
                        CompanyInfrastructure::get(self.shared, self.current_company()),
                        railtype,
                        1,
                    );
                }
                self.dirty_company_infrastructure_windows(self.current_company());
            }
        }

        if flags.test(DoCommandFlag::Execute) {
            self.mark_tile_dirty_by_tile(tile);
            self.signals()
                .add_track_to_signal_buffer(tile, track, self.current_company());
            yapf_notify_track_layout_change(tile, track);
        }

        cost.add_money(rail_build_cost(self.shared, railtype));
    }

    /// `CmdRemoveSingleRail`: remove a single piece of track.
    fn cmd_remove_single_rail(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveSingleRail,
    ) {
        let CmdRemoveSingleRail { tile, track } = args;
        cost.set(0);
        let mut crossing = false;

        if !is_valid_track(track) {
            return cost.error(INVALID_STRING_ID);
        }
        let trackbit = track_to_track_bits(track, self.check());

        /* Need to read tile owner now because it may change when the rail is removed
         * Also, in case of floods, _current_company != owner
         * There may be invalid tiletype even in exec run (when removing long track),
         * so do not call GetTileOwner(tile) in any case here */
        let mut owner = Owner::INVALID;

        let mut v = Handle::NONE;

        match self.world.kind(tile) {
            TileType::MP_ROAD => {
                if !self.world.level_crossing(tile.0)
                    || TrackBits(self.world.crossing_rail_bits(tile.0)) != trackbit
                {
                    return cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0);
                }

                if self.current_company() != Owner::WATER {
                    ret.tile_ownership(self.command, tile.0);
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }

                if !flags.test(DoCommandFlag::Bankrupt) {
                    self.command.no_vehicle(ret.0, tile);
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }

                cost.add_money(rail_clear_cost(self.shared, self.world.get_rail_type(tile)));

                if flags.test(DoCommandFlag::Execute) {
                    w::hit(w::RAIL_CMD_REMOVE_CROSSING);
                    crate::train::update_adjacent_level_crossing_tiles_on_level_crossing_removal(
                        self.train,
                        self.shared,
                        tile,
                        Axis(self.world.crossing_road_axis(tile.0)),
                    );

                    if self.pbs.has_reserved_tracks(tile, trackbit) {
                        v = self.pbs.get_train_for_reservation(tile, track);
                        if v != Handle::NONE {
                            self.free_train_track_reservation(v);
                        }
                    }

                    owner = self.world.owner(tile);
                    // SAFETY: the company is live and serialized; the rail type indexes its
                    // infrastructure array.
                    unsafe {
                        CompanyInfrastructure::add_rail(
                            CompanyInfrastructure::get(self.shared, owner),
                            self.world.get_rail_type(tile),
                            LEVELCROSSING_TRACKBIT_FACTOR.wrapping_neg(),
                        );
                    }
                    self.dirty_company_infrastructure_windows(owner);
                    (self.s.make_road)(
                        tile.0,
                        CrossingRoad {
                            bits: (self.shared.crossing_road_bits)(tile.0),
                            road_type: RoadType(self.world.road_type(tile.0, false)),
                            tram_type: RoadType(self.world.road_type(tile.0, true)),
                            town: (self.shared.town_index)(tile.0),
                            road_owner: self.world.road_owner(tile.0, false),
                            tram_owner: self.world.road_owner(tile.0, true),
                        },
                    );
                    delete_newgrf_inspect_window(GrfSpecFeature::GSF_RAILTYPES, tile.0);
                }
            }

            TileType::MP_RAILWAY => {
                /* There are no rails present at depots. */
                if !self.world.is_plain_rail(tile) {
                    return cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0);
                }

                if self.current_company() != Owner::WATER {
                    ret.tile_ownership(self.command, tile.0);
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                }

                self.ensure_no_train_on_track(ret, tile, track);
                if !ret.ok() {
                    return cost.take(ret);
                }

                let mut present = self.world.get_track_bits(tile);
                if present & trackbit == TrackBits::TRACK_BIT_NONE {
                    return cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0);
                }
                if present == TrackBits::TRACK_BIT_X | TrackBits::TRACK_BIT_Y {
                    crossing = true;
                }

                cost.add_money(rail_clear_cost(self.shared, self.world.get_rail_type(tile)));

                /* Charge extra to remove signals on the track, if they are there */
                if self.world.has_signal_on_track(tile, track) {
                    w::hit(w::RAIL_CMD_REMOVE_SIGNAL_CHARGE);
                    self.remove_single_signal_command(
                        ret,
                        flags,
                        CmdRemoveSingleSignal { tile, track },
                    );
                    cost.add(ret);
                }

                if flags.test(DoCommandFlag::Execute) {
                    w::hit(w::RAIL_CMD_REMOVE_RAIL);
                    if self.pbs.has_reserved_tracks(tile, trackbit) {
                        v = self.pbs.get_train_for_reservation(tile, track);
                        if v != Handle::NONE {
                            self.free_train_track_reservation(v);
                        }
                    }

                    owner = self.world.owner(tile);

                    let infrastructure = CompanyInfrastructure::get(self.shared, owner);
                    // SAFETY: the company is live and serialized; the rail type indexes its
                    // infrastructure array.
                    unsafe {
                        /* Subtract old infrastructure count. */
                        CompanyInfrastructure::add_rail(
                            infrastructure,
                            self.world.get_rail_type(tile),
                            pieces(present).wrapping_neg(),
                        );
                        /* Add new infrastructure count. */
                        present ^= trackbit;
                        CompanyInfrastructure::add_rail(
                            infrastructure,
                            self.world.get_rail_type(tile),
                            pieces(present),
                        );
                    }
                    self.dirty_company_infrastructure_windows(owner);

                    if present == TrackBits::TRACK_BIT_NONE {
                        let tileh = self.world.tile_slope(tile);
                        /* If there is flat water on the lower halftile, convert the tile to shore so the water remains */
                        if self.world.get_rail_ground_type(tile) == RailGroundType::HalfTileWater
                            && is_slope_with_one_corner_raised(tileh)
                        {
                            w::hit(w::RAIL_CMD_REMOVE_SHORE);
                            let docking = self.world.is_docking_tile(tile);
                            (self.shared.make_shore)(tile.0);
                            // SAFETY: the station table has process lifetime.
                            (unsafe { &*(self.shared.station_commands)() }.set_docking)(
                                tile.0, docking,
                            );
                        } else {
                            crate::ground::do_clear_square(tile, self.shared);
                        }
                        delete_newgrf_inspect_window(GrfSpecFeature::GSF_RAILTYPES, tile.0);
                    } else {
                        (self.rail.set_track_bits)(tile.0, present.0);
                        (self.rail.set_track_reservation)(
                            tile.0,
                            ((self.rail.rail_reservation)(tile.0) & present).0,
                        );
                    }
                }
            }

            _ => return cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0),
        }

        if flags.test(DoCommandFlag::Execute) {
            /* if we got that far, 'owner' variable is set correctly */
            crate::services_assert::assert_with(|| (self.shared.company_valid)(owner.0));

            self.mark_tile_dirty_by_tile(tile);
            if crossing {
                w::hit(w::RAIL_CMD_REMOVE_DIAGONAL_CROSSING);
                /* crossing is set when only TRACK_BIT_X and TRACK_BIT_Y are set. As we
                 * are removing one of these pieces, we'll need to update signals for
                 * both directions explicitly, as after the track is removed it won't
                 * 'connect' with the other piece. */
                self.signals()
                    .add_track_to_signal_buffer(tile, Track::TRACK_X, owner);
                self.signals()
                    .add_track_to_signal_buffer(tile, Track::TRACK_Y, owner);
                yapf_notify_track_layout_change(tile, Track::TRACK_X);
                yapf_notify_track_layout_change(tile, Track::TRACK_Y);
            } else {
                self.signals()
                    .add_track_to_signal_buffer(tile, track, owner);
                yapf_notify_track_layout_change(tile, track);
            }

            if v != Handle::NONE {
                self.try_path_reserve(v, true);
            }
        }
    }

    /// `ValidateAutoDrag`; false is `CMD_ERROR`.
    fn validate_auto_drag(
        &self,
        trackdir: &mut Trackdir,
        start: TileIndex,
        end: TileIndex,
    ) -> bool {
        let x = self.map().tile_x(start).cast_signed();
        let y = self.map().tile_y(start).cast_signed();
        let ex = self.map().tile_x(end).cast_signed();
        let ey = self.map().tile_y(end).cast_signed();

        if !is_valid_track(trackdir_to_track(*trackdir, self.check())) {
            return false;
        }

        /* calculate delta x,y from start to end tile */
        let dx = ex.wrapping_sub(x);
        let dy = ey.wrapping_sub(y);

        /* calculate delta x,y for the first direction */
        // The original's `trdx` and `trdy`.
        let mut delta_x = i32::from(TRACKDELTA[usize::from(trackdir.0)].x);
        let mut delta_y = i32::from(TRACKDELTA[usize::from(trackdir.0)].y);

        if !is_diagonal_trackdir(*trackdir, self.check()) {
            delta_x += i32::from(TRACKDELTA[usize::from(trackdir.0 ^ 1)].x);
            delta_y += i32::from(TRACKDELTA[usize::from(trackdir.0 ^ 1)].y);
        }

        /* validate the direction */
        while (delta_x <= 0 && dx > 0)
            || (delta_x >= 0 && dx < 0)
            || (delta_y <= 0 && dy > 0)
            || (delta_y >= 0 && dy < 0)
        {
            if has_bit(trackdir.0, 3) {
                // other direction is invalid too, invalid drag
                return false;
            }
            // first direction is invalid, try the other
            set_bit8(&mut trackdir.0, 3); // reverse the direction
            delta_x = -delta_x;
            delta_y = -delta_y;
        }

        /* (for diagonal tracks, this is already made sure of by above test), but:
         * for non-diagonal tracks, check if the start and end tile are on 1 line */
        if !is_diagonal_trackdir(*trackdir, self.check()) {
            delta_x = i32::from(TRACKDELTA[usize::from(trackdir.0)].x);
            delta_y = i32::from(TRACKDELTA[usize::from(trackdir.0)].y);
            if dx.abs() != dy.abs() && dx.abs() + delta_y.abs() != dy.abs() + delta_x.abs() {
                return false;
            }
        }

        true
    }

    /// `CmdRailTrackHelper`: build or remove a stretch of railroad tracks.
    fn cmd_rail_track_helper(
        &self,
        total_cost: Cost<'_>,
        last_error: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: RailTrackHelper,
    ) {
        let RailTrackHelper {
            mut tile,
            end_tile,
            railtype,
            track,
            remove,
            auto_remove_signals,
            fail_on_obstacle,
        } = args;
        total_cost.set(0);

        if (!remove && !val_param_rail_type(self.shared, railtype)) || !is_valid_track(track) {
            return total_cost.error(INVALID_STRING_ID);
        }
        if end_tile.0 >= self.shared.map_size() || tile.0 >= self.shared.map_size() {
            return total_cost.error(INVALID_STRING_ID);
        }

        let mut trackdir = track_to_trackdir(track, self.check());

        if !self.validate_auto_drag(&mut trackdir, tile, end_tile) {
            return total_cost.error(INVALID_STRING_ID);
        }
        w::hit(if remove {
            w::RAIL_CMD_DRAG_REMOVE
        } else {
            w::RAIL_CMD_DRAG_BUILD
        });

        let mut had_success = false;
        last_error.error(INVALID_STRING_ID);
        loop {
            if remove {
                self.remove_single_rail_command(
                    ret,
                    flags,
                    CmdRemoveSingleRail {
                        tile,
                        track: trackdir_to_track(trackdir, self.check()),
                    },
                );
            } else {
                self.build_single_rail_command(
                    ret,
                    flags,
                    CmdBuildSingleRail {
                        tile,
                        railtype,
                        track: trackdir_to_track(trackdir, self.check()),
                        auto_remove_signals,
                    },
                );
            }
            if !remove && !fail_on_obstacle && last_error.message() == STR_ERROR_ALREADY_BUILT.0 {
                had_success = true;
            }

            if ret.ok() {
                had_success = true;
                total_cost.add_money(ret.money());
            } else {
                last_error.take(ret);
                if last_error.message() != STR_ERROR_ALREADY_BUILT.0 && !remove {
                    if fail_on_obstacle {
                        return total_cost.take(last_error);
                    }
                    if had_success {
                        w::hit(w::RAIL_CMD_DRAG_OBSTACLE);
                        break; // Keep going if we haven't constructed any rail yet, skipping the start of the drag
                    }
                }

                /* Ownership errors are more important. */
                if last_error.message() == STR_ERROR_OWNED_BY.0 && remove {
                    break;
                }
            }

            if tile == end_tile {
                break;
            }

            tile += self
                .map()
                .to_tile_index_diff(TRACKDELTA[usize::from(trackdir.0)]);

            /* toggle railbit for the non-diagonal tracks */
            if !is_diagonal_trackdir(trackdir, self.check()) {
                toggle_bit8(&mut trackdir.0, 0);
            }
        }

        if !had_success {
            total_cost.take(last_error);
        }
    }

    /// `CmdBuildTrainDepot`: build a train depot.
    ///
    /// When checking for the tile slope, the original's to-do is to distinguish between
    /// "Flat land required" and "land sloped in wrong direction".
    fn cmd_build_train_depot(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildTrainDepot,
    ) {
        let CmdBuildTrainDepot {
            tile,
            railtype,
            dir,
        } = args;
        /* check railtype and valid direction for depot (0 through 3), 4 in total */
        if !val_param_rail_type(self.shared, railtype) || !is_valid_diag_direction(dir) {
            return cost.error(INVALID_STRING_ID);
        }

        let tileh = self.world.tile_slope(tile);

        cost.set(0);

        /* Prohibit construction if
         * The tile is non-flat AND
         * 1) build-on-slopes is disabled
         * 2) the tile is steep i.e. spans two height levels
         * 3) the exit points in the wrong direction
         */

        if tileh != Slope::SLOPE_FLAT {
            if !(self.shared.build_on_slopes)() || !can_build_depot_by_tileh(dir, tileh) {
                return cost.error(STR_ERROR_FLAT_LAND_REQUIRED.0);
            }
            cost.add_money(crate::company::price(Price::PR_BUILD_FOUNDATION));
        }

        /* Allow the user to rotate the depot instead of having to destroy it and build it again */
        let mut rotate_existing_depot = false;
        if self.world.is_rail_depot_tile(tile) && railtype == self.world.get_rail_type(tile) {
            ret.tile_ownership(self.command, tile.0);
            if !ret.ok() {
                return cost.take(ret);
            }

            if dir == self.world.get_rail_depot_direction(tile) {
                return cost.zero();
            }

            self.command.no_vehicle(ret.0, tile);
            if !ret.ok() {
                return cost.take(ret);
            }

            rotate_existing_depot = true;
        }

        if !rotate_existing_depot {
            self.landscape_clear(ret, flags, tile);
            cost.add(ret);
            if !cost.ok() {
                return;
            }

            if self.world.bridge_above(tile) {
                return cost.error(STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST.0);
            }

            if !(self.s.depot_available)() {
                return cost.error(INVALID_STRING_ID);
            }
        }

        if flags.test(DoCommandFlag::Execute) {
            if rotate_existing_depot {
                w::hit(w::RAIL_CMD_DEPOT_ROTATE);
                (self.rail.set_depot_exit)(tile.0, dir.0);
            } else {
                w::hit(w::RAIL_CMD_DEPOT_BUILD);
                let d = (self.s.new_depot)(tile.0);

                (self.rail.make_rail_depot)(tile.0, self.current_company().0, d, dir.0, railtype.0);
                (self.s.depot_name)(d);

                // SAFETY: the company is live and serialized; the rail type indexes its
                // infrastructure array.
                unsafe {
                    CompanyInfrastructure::add_rail(
                        CompanyInfrastructure::get(self.shared, self.current_company()),
                        railtype,
                        1,
                    );
                }
                self.dirty_company_infrastructure_windows(self.current_company());
            }

            self.mark_tile_dirty_by_tile(tile);
            self.signals().add_side_to_signal_buffer(
                tile,
                DiagDirection::INVALID_DIAGDIR,
                self.current_company(),
            );
            yapf_notify_track_layout_change(tile, diag_dir_to_diag_track(dir, self.check()));
        }

        cost.add_money(crate::company::price(Price::PR_BUILD_DEPOT_TRAIN));
        cost.add_money(rail_build_cost(self.shared, railtype));
    }

    /// `CmdBuildSingleSignal`: build signals, alternate between double/single,
    /// signal/semaphore, pre/exit/combo-signals, and what-else not. If the rail piece does
    /// not have any signals, signal cycling is ignored.
    ///
    /// The original's to-do: p2 should be replaced by two bits for "along" and "against"
    /// the track.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "SetPresentSignals and SetSignalStates take the uint the original computes"
    )]
    fn cmd_build_single_signal(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdBuildSingleSignal,
    ) {
        let CmdBuildSingleSignal {
            tile,
            track,
            mut sigtype,
            mut sigvar,
            convert_signal,
            skip_existing_signals,
            ctrl_pressed,
            cycle_start,
            cycle_stop,
            mut num_dir_cycle,
            signals_copy,
        } = args;
        if sigtype > SignalType::SIGTYPE_LAST || sigvar > SignalVariant::SIG_SEMAPHORE {
            return cost.error(INVALID_STRING_ID);
        }
        if cycle_start > cycle_stop || cycle_stop > SignalType::SIGTYPE_LAST {
            return cost.error(INVALID_STRING_ID);
        }

        if ctrl_pressed {
            sigvar = SignalVariant(sigvar.0 ^ SignalVariant::SIG_SEMAPHORE.0);
        }

        /* You can only build signals on plain rail tiles, and the selected track must exist */
        if !is_valid_track(track)
            || !self.world.is_plain_rail_tile(tile)
            || !self.world.has_track(tile, track)
        {
            return cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0);
        }
        /* Protect against invalid signal copying */
        if signals_copy != 0 && (signals_copy & signal_on_track(track)) == 0 {
            return cost.error(INVALID_STRING_ID);
        }

        cost.tile_ownership(self.command, tile.0);
        if !cost.ok() {
            return;
        }

        /* See if this is a valid track combination for signals (no overlap) */
        if tracks_overlap(self.world.get_track_bits(tile)) {
            return cost.error(STR_ERROR_NO_SUITABLE_RAILROAD_TRACK.0);
        }

        /* In case we don't want to change an existing signal, return without error. */
        if skip_existing_signals && self.world.has_signal_on_track(tile, track) {
            return cost.zero();
        }

        /* you can not convert a signal if no signal is on track */
        if convert_signal && !self.world.has_signal_on_track(tile, track) {
            return cost.error(STR_ERROR_THERE_ARE_NO_SIGNALS.0);
        }

        let build_signals = crate::company::price(Price::PR_BUILD_SIGNALS);
        if !self.world.has_signal_on_track(tile, track) {
            /* build new signals */
            cost.set(build_signals);
        } else if signals_copy != 0 && sigvar != (self.rail.signal_variant)(tile.0, track.0) {
            /* convert signals <-> semaphores */
            cost.set(build_signals.saturating_add(crate::company::price(Price::PR_CLEAR_SIGNALS)));
        } else if convert_signal {
            /* convert button pressed */
            if ctrl_pressed || (self.rail.signal_variant)(tile.0, track.0) != sigvar {
                /* it costs money to change signal variant (light or semaphore) */
                cost.set(
                    build_signals.saturating_add(crate::company::price(Price::PR_CLEAR_SIGNALS)),
                );
            } else {
                /* it is free to change signal type (block, exit, entry, combo, path, etc) */
                cost.zero();
            }
        } else {
            /* it is free to change orientation or number of signals on the tile (for block/presignals which allow signals in both directions) */
            cost.zero();
        }

        if !flags.test(DoCommandFlag::Execute) {
            return;
        }

        let mut v = Handle::NONE;
        /* The new/changed signal could block our path. As this can lead to
         * stale reservations, we clear the path reservation here and try
         * to redo it later on. */
        if self
            .pbs
            .has_reserved_tracks(tile, track_to_track_bits(track, self.check()))
        {
            v = self.pbs.get_train_for_reservation(tile, track);
            if v != Handle::NONE {
                self.free_train_track_reservation(v);
            }
        }

        if !self.world.has_signals(tile) {
            /* there are no signals at all on this tile yet */
            (self.rail.set_has_signals)(tile.0, true);
            (self.rail.set_signal_states)(tile.0, 0xF); // all signals are on
            (self.rail.set_present_signals)(tile.0, 0); // no signals built by default
            (self.rail.set_signal_type)(tile.0, track.0, sigtype.0);
            (self.rail.set_signal_variant)(tile.0, track.0, sigvar.0);
        }

        let infrastructure = CompanyInfrastructure::get(self.shared, self.world.owner(tile));
        /* Subtract old signal infrastructure count. */
        // SAFETY: the company is live and serialized.
        unsafe {
            CompanyInfrastructure::add_signal(
                infrastructure,
                self.world
                    .get_present_signals(tile)
                    .count_ones()
                    .wrapping_neg(),
            );
        }

        let on_track = u32::from(signal_on_track(track));
        if signals_copy == 0 {
            if !self.world.has_signal_on_track(tile, track) {
                w::hit(w::RAIL_CMD_SIGNAL_BUILD);
                /* build new signals */
                (self.rail.set_present_signals)(
                    tile.0,
                    (self.world.get_present_signals(tile)
                        | if is_pbs_signal(sigtype) {
                            u32::from(kill_first_bit(signal_on_track(track)))
                        } else {
                            on_track
                        }) as u8,
                );
                (self.rail.set_signal_type)(tile.0, track.0, sigtype.0);
                (self.rail.set_signal_variant)(tile.0, track.0, sigvar.0);
                while num_dir_cycle > 0 {
                    num_dir_cycle -= 1;
                    (self.rail.cycle_signal_side)(tile.0, track.0);
                }
            } else if convert_signal {
                /* convert signal button pressed */
                if ctrl_pressed {
                    w::hit(w::RAIL_CMD_SIGNAL_VARIANT);
                    /* toggle the present signal variant: SIG_ELECTRIC <-> SIG_SEMAPHORE */
                    (self.rail.set_signal_variant)(
                        tile.0,
                        track.0,
                        if (self.rail.signal_variant)(tile.0, track.0)
                            == SignalVariant::SIG_ELECTRIC
                        {
                            SignalVariant::SIG_SEMAPHORE.0
                        } else {
                            SignalVariant::SIG_ELECTRIC.0
                        },
                    );
                    /* Query current signal type so the check for PBS signals below works. */
                    sigtype = self.world.get_signal_type(tile, track);
                } else {
                    w::hit(w::RAIL_CMD_SIGNAL_CONVERT);
                    /* convert the present signal to the chosen type and variant */
                    (self.rail.set_signal_type)(tile.0, track.0, sigtype.0);
                    (self.rail.set_signal_variant)(tile.0, track.0, sigvar.0);
                    if is_pbs_signal(sigtype)
                        && (self.world.get_present_signals(tile) & on_track) == on_track
                    {
                        (self.rail.set_present_signals)(
                            tile.0,
                            ((self.world.get_present_signals(tile) & !on_track)
                                | u32::from(kill_first_bit(signal_on_track(track))))
                                as u8,
                        );
                    }
                }
            } else if ctrl_pressed {
                w::hit(w::RAIL_CMD_SIGNAL_CYCLE_TYPE);
                /* cycle between cycle_start and cycle_end */
                sigtype = SignalType(self.world.get_signal_type(tile, track).0.wrapping_add(1));

                if sigtype < cycle_start || sigtype > cycle_stop {
                    sigtype = cycle_start;
                }

                (self.rail.set_signal_type)(tile.0, track.0, sigtype.0);
                if is_pbs_signal(sigtype)
                    && (self.world.get_present_signals(tile) & on_track) == on_track
                {
                    (self.rail.set_present_signals)(
                        tile.0,
                        ((self.world.get_present_signals(tile) & !on_track)
                            | u32::from(kill_first_bit(signal_on_track(track))))
                            as u8,
                    );
                }
            } else {
                w::hit(w::RAIL_CMD_SIGNAL_CYCLE_SIDE);
                /* cycle the signal side: both -> left -> right -> both -> ... */
                (self.rail.cycle_signal_side)(tile.0, track.0);
                /* Query current signal type so the check for PBS signals below works. */
                sigtype = self.world.get_signal_type(tile, track);
            }
        } else {
            w::hit(w::RAIL_CMD_SIGNAL_COPY);
            /* If CmdBuildManySignals is called with copying signals, just copy the
             * direction of the first signal given as parameter by CmdBuildManySignals */
            (self.rail.set_present_signals)(
                tile.0,
                ((self.world.get_present_signals(tile) & !on_track)
                    | (u32::from(signals_copy) & on_track)) as u8,
            );
            (self.rail.set_signal_variant)(tile.0, track.0, sigvar.0);
            (self.rail.set_signal_type)(tile.0, track.0, sigtype.0);
        }

        /* Add new signal infrastructure count. */
        // SAFETY: the company is live and serialized.
        unsafe {
            CompanyInfrastructure::add_signal(
                infrastructure,
                self.world.get_present_signals(tile).count_ones(),
            );
        }
        self.dirty_company_infrastructure_windows(self.world.owner(tile));

        if is_pbs_signal(sigtype) {
            w::hit(w::RAIL_CMD_SIGNAL_PBS_STATE);
            /* PBS signals should show red unless they are on reserved tiles without a train. */
            let mask = self.world.get_present_signals(tile) & on_track;
            let reserved = has_bit((self.rail.rail_reservation)(tile.0).0, track.0) && {
                self.command.no_vehicle(ret.0, tile);
                ret.ok()
            };
            (self.rail.set_signal_states)(
                tile.0,
                ((self.world.get_signal_states(tile) & !mask)
                    | (if reserved { u32::MAX } else { 0 } & mask)) as u8,
            );
        }
        self.mark_tile_dirty_by_tile(tile);
        self.signals()
            .add_track_to_signal_buffer(tile, track, self.current_company());
        yapf_notify_track_layout_change(tile, track);
        if v != Handle::NONE && v.track() != TrackBits::TRACK_BIT_DEPOT {
            /* Extend the train's path if it's not stopped or loading, or not at a safe position. */
            if !((v.vehicle().vehstatus().test(VehState::Stopped) && v.vehicle().cur_speed() == 0)
                || v.vehicle().current_order().is_type(OrderType::OT_LOADING))
                || !self.pbs.is_safe_waiting_position(
                    v,
                    v.vehicle().tile(),
                    crate::train::get_vehicle_trackdir(self.train, self.shared, v),
                    true,
                    (self.shared.forbid_90_deg)(),
                )
            {
                w::hit(w::RAIL_CMD_SIGNAL_RESERVE);
                self.try_path_reserve(v, true);
            }
        }
    }

    /// `AdvanceSignalAutoFill`.
    fn advance_signal_auto_fill(
        &self,
        tile: &mut TileIndex,
        trackdir: &mut Trackdir,
        remove: bool,
    ) -> bool {
        /* We only process starting tiles of tunnels or bridges so jump to the other end before moving further. */
        if self.world.is_tile_type(*tile, TileType::MP_TUNNELBRIDGE) {
            *tile = TileIndex((self.shared.other_tunnel_end)(tile.0));
        }

        *tile = self.map().add_tile_index_diff_c_wrap(
            *tile,
            TRACKDELTA[usize::from(trackdir.0)],
            self.world.size_y(),
        );
        if *tile == TileIndex::INVALID {
            return false;
        }

        /* Check for track bits on the new tile */
        let mut trackdirbits = track_status_to_trackdir_bits(
            crate::movement::table(self.shared).get_tile_track_status_in_world(
                &self.world,
                *tile,
                TransportType::TRANSPORT_RAIL,
                0,
                DiagDirection::INVALID_DIAGDIR,
            ),
        );

        if tracks_overlap(trackdir_bits_to_track_bits(trackdirbits)) {
            return false;
        }
        trackdirbits &= trackdir_reaches_trackdirs(*trackdir, self.check());

        /* No track bits, must stop */
        if trackdirbits == TrackdirBits::TRACKDIR_BIT_NONE {
            return false;
        }

        /* Get the first track dir */
        *trackdir = remove_first_trackdir(&mut trackdirbits, self.check());

        /* Any left? It's a junction so we stop */
        if trackdirbits != TrackdirBits::TRACKDIR_BIT_NONE {
            return false;
        }

        match self.world.kind(*tile) {
            TileType::MP_RAILWAY => {
                if self.world.is_rail_depot(*tile) {
                    return false;
                }
                if !remove
                    && self
                        .world
                        .has_signal_on_track(*tile, trackdir_to_track(*trackdir, self.check()))
                {
                    return false;
                }
            }

            TileType::MP_ROAD => {
                if !self.world.level_crossing(tile.0) {
                    return false;
                }
            }

            TileType::MP_TUNNELBRIDGE => {
                if TransportType(self.world.tunnel_bridge_transport(tile.0))
                    != TransportType::TRANSPORT_RAIL
                {
                    return false;
                }
                if DiagDirection(self.world.tunnel_bridge_dir(tile.0))
                    != trackdir_to_exitdir(*trackdir, self.check())
                {
                    return false;
                }
            }

            _ => return false,
        }
        true
    }

    /// `CmdSignalTrackHelper`: build many signals by dragging; `AutoSignals`.
    fn cmd_signal_track_helper(
        &self,
        total_cost: Cost<'_>,
        last_error: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: SignalTrackHelper,
    ) {
        let SignalTrackHelper {
            mut tile,
            end_tile,
            mut track,
            mut sigtype,
            mut sigvar,
            mode,
            remove,
            autofill,
            minimise_gaps,
            mut signal_density,
        } = args;
        total_cost.set(0);

        if end_tile.0 >= self.shared.map_size() || !is_valid_track(track) {
            return total_cost.error(INVALID_STRING_ID);
        }
        if signal_density == 0 || signal_density > 20 {
            return total_cost.error(INVALID_STRING_ID);
        }
        if !remove && (sigtype > SignalType::SIGTYPE_LAST || sigvar > SignalVariant::SIG_SEMAPHORE)
        {
            return total_cost.error(INVALID_STRING_ID);
        }

        if !self.world.is_plain_rail_tile(tile) {
            return total_cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0);
        }
        let start_tile = tile;

        /* Interpret signal_density as the logical length of said amount of tiles in X/Y direction. */
        signal_density = signal_density.wrapping_mul(TILE_AXIAL_DISTANCE.cast_signed());

        let mut trackdir = track_to_trackdir(track, self.check());
        if !self.validate_auto_drag(&mut trackdir, tile, end_tile) {
            return total_cost.error(INVALID_STRING_ID);
        }

        track = trackdir_to_track(trackdir, self.check()); // trackdir might have changed, keep track in sync
        let start_trackdir = trackdir;

        /* Must start on a valid track to be able to avoid loops */
        if !self.world.has_track(tile, track) {
            return total_cost.error(INVALID_STRING_ID);
        }

        let signals;
        /* copy the signal-style of the first rail-piece if existing */
        if self.world.has_signal_on_track(tile, track) {
            signals = self.world.get_present_signals(tile) & u32::from(signal_on_track(track));
            crate::services_assert::assert_with(|| signals != 0);

            /* copy signal/semaphores style (independent of CTRL) */
            sigvar = (self.rail.signal_variant)(tile.0, track.0);

            sigtype = self.world.get_signal_type(tile, track);
            /* Don't but copy entry or exit-signal type */
            if sigtype == SignalType::SIGTYPE_ENTRY || sigtype == SignalType::SIGTYPE_EXIT {
                sigtype = SignalType::SIGTYPE_BLOCK;
            }
        } else {
            // no signals exist, drag a two-way signal stretch
            signals = u32::from(if is_pbs_signal(sigtype) {
                signal_along_trackdir(trackdir)
            } else {
                signal_on_track(track)
            });
        }

        let mut signal_dir = 0_u8;
        if signals & u32::from(signal_along_trackdir(trackdir)) != 0 {
            set_bit8(&mut signal_dir, 0);
        }
        if signals & u32::from(signal_against_trackdir(trackdir)) != 0 {
            set_bit8(&mut signal_dir, 1);
        }
        w::hit(if remove {
            w::RAIL_CMD_SIGNAL_DRAG_REMOVE
        } else {
            w::RAIL_CMD_SIGNAL_DRAG
        });

        /* signal_ctr         - amount of tiles already processed
         * last_used_ctr      - amount of tiles before previously placed signal
         * signals_density    - setting to put signal on every Nth tile (double space on |, -- tracks)
         * last_suitable_ctr  - amount of tiles before last possible signal place
         * last_suitable_tile - last tile where it is possible to place a signal
         * last_suitable_trackdir - trackdir of the last tile
         **********
         * trackdir   - trackdir to build with autorail
         * semaphores - semaphores or signals
         * signals    - is there a signal/semaphore on the first tile, copy its style (two-way/single-way)
         *              and convert all others to semaphore/signal
         * remove     - 1 remove signals, 0 build signals */
        let mut signal_ctr = 0_i32;
        let mut last_used_ctr = signal_density.wrapping_neg(); // to force signal at first tile
        let mut last_suitable_ctr = 0_i32;
        let mut last_suitable_tile = TileIndex::INVALID;
        let mut last_suitable_trackdir = Trackdir::INVALID_TRACKDIR;
        last_error.error(INVALID_STRING_ID);
        let mut had_success = false;
        // The original's lambda reads `signal_ctr` by reference; it is passed at each call.
        let mut build_signal =
            |tile: TileIndex, trackdir: Trackdir, test_only: bool, signal_ctr: i32| {
                /* Pick the correct orientation for the track direction */
                let mut signals = 0_u8;
                if has_bit(signal_dir, 0) {
                    signals |= signal_along_trackdir(trackdir);
                }
                if has_bit(signal_dir, 1) {
                    signals |= signal_against_trackdir(trackdir);
                }

                let do_flags = if test_only {
                    flags.reset(DoCommandFlag::Execute)
                } else {
                    flags
                };
                if remove {
                    self.remove_single_signal_command(
                        ret,
                        do_flags,
                        CmdRemoveSingleSignal {
                            tile,
                            track: trackdir_to_track(trackdir, self.check()),
                        },
                    );
                } else {
                    self.build_single_signal_command(
                        ret,
                        do_flags,
                        CmdBuildSingleSignal {
                            tile,
                            track: trackdir_to_track(trackdir, self.check()),
                            sigtype,
                            sigvar,
                            convert_signal: false,
                            skip_existing_signals: signal_ctr == 0,
                            ctrl_pressed: mode,
                            cycle_start: SignalType::SIGTYPE_BLOCK,
                            cycle_stop: SignalType::SIGTYPE_BLOCK,
                            num_dir_cycle: 0,
                            signals_copy: signals,
                        },
                    );
                }

                if test_only {
                    return ret.ok();
                }

                if ret.ok() {
                    had_success = true;
                    total_cost.add_money(ret.money());
                } else {
                    /* The "No railway" error is the least important one. */
                    if ret.message() != STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0
                        || last_error.message() == INVALID_STRING_ID
                    {
                        last_error.take(ret);
                    }
                }
                ret.ok()
            };

        loop {
            if remove {
                /* In remove mode last_* stuff doesn't matter, we simply try to clear every tile. */
                build_signal(tile, trackdir, false, signal_ctr);
            } else if minimise_gaps {
                w::hit(w::RAIL_CMD_SIGNAL_MINIMISE);
                /* We're trying to minimize gaps wherever possible, so keep track of last suitable
                 * position and use it if current gap exceeds required signal density. */

                if signal_ctr > last_used_ctr.wrapping_add(signal_density)
                    && last_suitable_tile != TileIndex::INVALID
                {
                    /* We overshot so build a signal in last good location. */
                    if build_signal(
                        last_suitable_tile,
                        last_suitable_trackdir,
                        false,
                        signal_ctr,
                    ) {
                        last_suitable_tile = TileIndex::INVALID;
                        last_used_ctr = last_suitable_ctr;
                    }
                }

                if signal_ctr == last_used_ctr.wrapping_add(signal_density) {
                    /* Current gap matches the required density, build a signal.  */
                    if build_signal(tile, trackdir, false, signal_ctr) {
                        last_used_ctr = signal_ctr;
                        last_suitable_tile = TileIndex::INVALID;
                    }
                } else {
                    /* Test tile for a potential signal spot. */
                    if build_signal(tile, trackdir, true, signal_ctr) {
                        last_suitable_tile = tile;
                        last_suitable_ctr = signal_ctr;
                        last_suitable_trackdir = trackdir;
                    }
                }
            } else if signal_ctr >= last_used_ctr.wrapping_add(signal_density) {
                /* We're always keeping regular interval between signals so doesn't matter whether we succeed or not. */
                build_signal(tile, trackdir, false, signal_ctr);
                last_used_ctr = signal_ctr;
            }

            if autofill {
                w::hit(w::RAIL_CMD_SIGNAL_AUTOFILL);
                match self.world.kind(tile) {
                    TileType::MP_RAILWAY => {
                        signal_ctr = signal_ctr.wrapping_add(
                            if is_diagonal_trackdir(trackdir, self.check()) {
                                TILE_AXIAL_DISTANCE
                            } else {
                                TILE_CORNER_DISTANCE
                            }
                            .cast_signed(),
                        );
                    }

                    TileType::MP_ROAD => {
                        signal_ctr = signal_ctr.wrapping_add(TILE_AXIAL_DISTANCE.cast_signed());
                    }

                    TileType::MP_TUNNELBRIDGE => {
                        w::hit(w::RAIL_CMD_SIGNAL_WORMHOLE);
                        let len = crate::tunnelbridge::get_tunnel_bridge_length(
                            self.map(),
                            tile.0,
                            (self.shared.other_tunnel_end)(tile.0),
                        )
                        .wrapping_add(2)
                        .wrapping_mul(TILE_AXIAL_DISTANCE);
                        if remove || minimise_gaps {
                            signal_ctr = signal_ctr.wrapping_add_unsigned(len);
                        } else {
                            /* To keep regular interval we need to emulate placing signals on a bridge.
                             * We start with TILE_AXIAL_DISTANCE as one bridge tile gets processed in the main loop. */
                            signal_ctr = signal_ctr.wrapping_add(TILE_AXIAL_DISTANCE.cast_signed());
                            let mut i = TILE_AXIAL_DISTANCE;
                            while i < len {
                                if signal_ctr >= last_used_ctr.wrapping_add(signal_density) {
                                    last_used_ctr = signal_ctr;
                                }
                                signal_ctr =
                                    signal_ctr.wrapping_add(TILE_AXIAL_DISTANCE.cast_signed());
                                i = i.wrapping_add(TILE_AXIAL_DISTANCE);
                            }
                        }
                    }

                    _ => {}
                }

                if !self.advance_signal_auto_fill(&mut tile, &mut trackdir, remove) {
                    break;
                }

                /* Prevent possible loops */
                if tile == start_tile && trackdir == start_trackdir {
                    break;
                }
            } else {
                if tile == end_tile {
                    break;
                }

                signal_ctr = signal_ctr.wrapping_add(
                    if is_diagonal_trackdir(trackdir, self.check()) {
                        TILE_AXIAL_DISTANCE
                    } else {
                        TILE_CORNER_DISTANCE
                    }
                    .cast_signed(),
                );
                /* toggle railbit for the non-diagonal tracks (|, -- tracks) */

                tile += self
                    .map()
                    .to_tile_index_diff(TRACKDELTA[usize::from(trackdir.0)]);
                if !is_diagonal_trackdir(trackdir, self.check()) {
                    toggle_bit8(&mut trackdir.0, 0);
                }
            }
        }

        /* We may end up with the current gap exceeding the signal density so fix that if needed. */
        if !remove
            && minimise_gaps
            && signal_ctr > last_used_ctr.wrapping_add(signal_density)
            && last_suitable_tile != TileIndex::INVALID
        {
            build_signal(
                last_suitable_tile,
                last_suitable_trackdir,
                false,
                signal_ctr,
            );
        }

        if !had_success {
            total_cost.take(last_error);
        }
    }

    /// `CmdRemoveSingleSignal`: remove signals.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "SetPresentSignals takes the uint the original computes"
    )]
    fn cmd_remove_single_signal(
        &self,
        cost: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdRemoveSingleSignal,
    ) {
        let CmdRemoveSingleSignal { tile, track } = args;
        if !is_valid_track(track)
            || !self.world.is_plain_rail_tile(tile)
            || !self.world.has_track(tile, track)
        {
            return cost.error(STR_ERROR_THERE_IS_NO_RAILROAD_TRACK.0);
        }
        if !self.world.has_signal_on_track(tile, track) {
            return cost.error(STR_ERROR_THERE_ARE_NO_SIGNALS.0);
        }

        /* Only water can remove signals from anyone */
        if self.current_company() != Owner::WATER {
            cost.tile_ownership(self.command, tile.0);
            if !cost.ok() {
                return;
            }
        }

        /* Do it? */
        if flags.test(DoCommandFlag::Execute) {
            w::hit(w::RAIL_CMD_SIGNAL_REMOVE);
            let mut v = Handle::NONE;
            if self
                .pbs
                .has_reserved_tracks(tile, track_to_track_bits(track, self.check()))
            {
                v = self.pbs.get_train_for_reservation(tile, track);
            } else if is_pbs_signal(self.world.get_signal_type(tile, track)) {
                /* PBS signal, might be the end of a path reservation. */
                let mut td = track_to_trackdir(track, self.check());
                let mut i = 0;
                while v == Handle::NONE && i < 2 {
                    /* Only test the active signal side. */
                    if self
                        .world
                        .has_signal_on_trackdir(tile, reverse_trackdir(td, self.check()))
                    {
                        let next = self.map().tile_add_by_diag_dir(
                            tile,
                            trackdir_to_exitdir(td, self.check()),
                            self.check(),
                        );
                        let tracks = trackdir_bits_to_track_bits(trackdir_reaches_trackdirs(
                            td,
                            self.check(),
                        ));
                        if self.pbs.has_reserved_tracks(next, tracks) {
                            w::hit(w::RAIL_CMD_SIGNAL_REMOVE_PATH_END);
                            v = self.pbs.get_train_for_reservation(
                                next,
                                track_bits_to_track(
                                    self.pbs.get_reserved_trackbits(next) & tracks,
                                    self.check(),
                                ),
                            );
                        }
                    }
                    i += 1;
                    td = reverse_trackdir(td, self.check());
                }
            }
            // SAFETY: the company is live and serialized.
            unsafe {
                CompanyInfrastructure::add_signal(
                    CompanyInfrastructure::get(self.shared, self.world.owner(tile)),
                    self.world
                        .get_present_signals(tile)
                        .count_ones()
                        .wrapping_neg(),
                );
            }
            (self.rail.set_present_signals)(
                tile.0,
                (self.world.get_present_signals(tile) & !u32::from(signal_on_track(track))) as u8,
            );
            // SAFETY: as above.
            unsafe {
                CompanyInfrastructure::add_signal(
                    CompanyInfrastructure::get(self.shared, self.world.owner(tile)),
                    self.world.get_present_signals(tile).count_ones(),
                );
            }
            self.dirty_company_infrastructure_windows(self.world.owner(tile));

            /* removed last signal from tile? */
            if self.world.get_present_signals(tile) == 0 {
                (self.rail.set_signal_states)(tile.0, 0);
                (self.rail.set_has_signals)(tile.0, false);
                (self.rail.set_signal_variant)(
                    tile.0,
                    Track::INVALID_TRACK.0,
                    SignalVariant::SIG_ELECTRIC.0,
                ); // remove any possible semaphores
            }

            self.signals()
                .add_track_to_signal_buffer(tile, track, self.world.owner(tile));
            yapf_notify_track_layout_change(tile, track);
            if v != Handle::NONE {
                self.try_path_reserve(v, false);
            }

            self.mark_tile_dirty_by_tile(tile);
        }

        cost.set(crate::company::price(Price::PR_CLEAR_SIGNALS));
    }

    /// `CmdConvertRail`: convert one rail type to the other. You can convert normal rail to
    /// monorail/maglev easily or vice-versa.
    fn cmd_convert_rail(
        &self,
        cost: Cost<'_>,
        error: Cost<'_>,
        ret: Cost<'_>,
        flags: DoCommandFlags,
        args: CmdConvertRail,
    ) {
        let CmdConvertRail {
            tile: area_end,
            area_start,
            totype,
            diagonal,
        } = args;

        if !val_param_rail_type(self.shared, totype) {
            return cost.error(INVALID_STRING_ID);
        }
        if area_start.0 >= self.shared.map_size() {
            return cost.error(INVALID_STRING_ID);
        }

        let mut affected_trains: Vec<Handle> = Vec::new();

        cost.set(0);
        error.error(STR_ERROR_NO_SUITABLE_RAILROAD_TRACK.0); // by default, there is no track to convert.
        let mut found_convertible_track = false; // whether we actually did convert some track (see bug #7633)

        let mut iter = crate::tilearea::TileIterator::create(
            self.map(),
            self.shared.map_size(),
            area_start,
            area_end,
            diagonal,
            self.check(),
        );
        loop {
            let tile = iter.tile();
            if tile == TileIndex::INVALID {
                break;
            }
            // Each `break 'tile` is the original's `continue`: the iterator still advances.
            'tile: {
                let tt = self.world.kind(tile);

                /* Check if there is any track on tile */
                match tt {
                    TileType::MP_RAILWAY => {}
                    TileType::MP_STATION => {
                        if !self.world.has_station_rail(tile) {
                            break 'tile;
                        }
                    }
                    TileType::MP_ROAD => {
                        if !self.world.level_crossing(tile.0) {
                            break 'tile;
                        }
                        if rail_no_level_crossings(self.shared, totype) {
                            error.make_error(STR_ERROR_CROSSING_DISALLOWED_RAIL.0);
                            break 'tile;
                        }
                    }
                    TileType::MP_TUNNELBRIDGE => {
                        if TransportType(self.world.tunnel_bridge_transport(tile.0))
                            != TransportType::TRANSPORT_RAIL
                        {
                            break 'tile;
                        }
                    }
                    _ => break 'tile,
                }

                /* Original railtype we are converting from */
                let r#type = self.world.get_rail_type(tile);

                /* Converting to the same type or converting 'hidden' elrail -> rail */
                if r#type == totype
                    || (self.shared.settings().disable_elrails()
                        && totype == RailType::RAILTYPE_RAIL
                        && r#type == RailType::RAILTYPE_ELECTRIC)
                {
                    break 'tile;
                }

                /* Trying to convert other's rail */
                ret.tile_ownership(self.command, tile.0);
                if !ret.ok() {
                    error.take(ret);
                    break 'tile;
                }

                let mut vehicles_affected: Vec<Handle> = Vec::new();

                /* Vehicle on the tile when not converting Rail <-> ElRail
                 * Tunnels and bridges have special check later */
                if tt != TileType::MP_TUNNELBRIDGE {
                    if !is_compatible_rail(self.shared, r#type, totype) {
                        if self.world.is_plain_rail_tile(tile) {
                            self.command
                                .no_train(ret.0, tile, self.world.get_track_bits(tile));
                        } else {
                            self.command.no_vehicle(ret.0, tile);
                        }
                        if !ret.ok() {
                            error.take(ret);
                            break 'tile;
                        }
                    }
                    if flags.test(DoCommandFlag::Execute) {
                        // we can safely convert, too
                        let mut reserved = self.pbs.get_reserved_trackbits(tile);
                        loop {
                            let track = remove_first_track(&mut reserved, self.check());
                            if track == Track::INVALID_TRACK {
                                break;
                            }
                            let v = self.pbs.get_train_for_reservation(tile, track);
                            if v != Handle::NONE
                                && !has_power_on_rail_types(self.shared, v.railtypes(), totype)
                            {
                                w::hit(w::RAIL_CMD_CONVERT_REROUTE);
                                /* No power on new rail type, reroute. */
                                self.free_train_track_reservation(v);
                                vehicles_affected.push(v);
                            }
                        }

                        /* Update the company infrastructure counters. */
                        if !self.world.is_rail_station_tile(tile)
                            || !self.world.is_station_tile_blocked(tile)
                        {
                            let owner = self.world.owner(tile);
                            let c = CompanyInfrastructure::get(self.shared, owner);
                            let mut num_pieces = if self.world.level_crossing_tile(tile.0) {
                                LEVELCROSSING_TRACKBIT_FACTOR
                            } else {
                                1
                            };
                            if self.world.is_plain_rail_tile(tile) {
                                num_pieces = pieces(self.world.get_track_bits(tile));
                            }
                            // SAFETY: the company is live and serialized; the rail types
                            // index its infrastructure array.
                            unsafe {
                                CompanyInfrastructure::add_rail(
                                    c,
                                    r#type,
                                    num_pieces.wrapping_neg(),
                                );
                                CompanyInfrastructure::add_rail(c, totype, num_pieces);
                            }
                            self.dirty_company_infrastructure_windows(owner);
                        }

                        (self.rail.set_rail_type)(tile.0, totype.0);
                        self.mark_tile_dirty_by_tile(tile);
                        /* update power of train on this tile */
                        vehicles_on_tile(self.map(), tile, |v| {
                            if v.vehicle_type() == VehicleType::VEH_TRAIN {
                                include(&mut affected_trains, &Handle::from_vehicle(v).first());
                            }
                        });
                    }
                }

                match tt {
                    TileType::MP_RAILWAY => {
                        if self.world.get_rail_tile_type(tile) == RailTileType::Depot {
                            if flags.test(DoCommandFlag::Execute) {
                                w::hit(w::RAIL_CMD_CONVERT_DEPOT);
                                /* notify YAPF about the track layout change */
                                yapf_notify_track_layout_change(
                                    tile,
                                    self.world.get_rail_depot_track(tile),
                                );

                                /* Update build vehicle window related to this depot */
                                invalidate_window_data(
                                    WindowClass::WC_VEHICLE_DEPOT,
                                    WindowNumber::from(tile),
                                    0,
                                    false,
                                );
                                invalidate_window_data(
                                    WindowClass::WC_BUILD_VEHICLE,
                                    WindowNumber::from(tile),
                                    0,
                                    false,
                                );
                            }
                            found_convertible_track = true;
                            cost.add_money(rail_convert_cost(self.shared, r#type, totype));
                        } else {
                            // RailTileType::Normal, RailTileType::Signals
                            if flags.test(DoCommandFlag::Execute) {
                                w::hit(w::RAIL_CMD_CONVERT_RAIL);
                                /* notify YAPF about the track layout change */
                                let mut tracks = self.world.get_track_bits(tile);
                                while tracks != TrackBits::TRACK_BIT_NONE {
                                    yapf_notify_track_layout_change(
                                        tile,
                                        remove_first_track(&mut tracks, self.check()),
                                    );
                                }
                            }
                            found_convertible_track = true;
                            cost.add_money(
                                rail_convert_cost(self.shared, r#type, totype).saturating_mul(
                                    i64::from(self.world.get_track_bits(tile).0.count_ones()),
                                ),
                            );
                        }
                    }

                    TileType::MP_TUNNELBRIDGE => {
                        let endtile = TileIndex((self.shared.other_tunnel_end)(tile.0));

                        /* If both ends of tunnel/bridge are in the range, do not try to convert twice -
                         * it would cause assert because of different test and exec runs */
                        if endtile < tile {
                            if diagonal {
                                if crate::tilearea::DiagonalTileArea::from_corners(
                                    self.map(),
                                    self.shared.map_size(),
                                    area_start,
                                    area_end,
                                    self.check(),
                                )
                                .contains(endtile, self.map())
                                {
                                    break 'tile;
                                }
                            } else if crate::tilearea::OrthogonalTileArea::from_corners(
                                self.map(),
                                self.shared.map_size(),
                                area_start,
                                area_end,
                                self.check(),
                            )
                            .contains(
                                endtile,
                                self.map(),
                                self.check(),
                            ) {
                                break 'tile;
                            }
                        }

                        /* When not converting rail <-> el. rail, any vehicle cannot be in tunnel/bridge */
                        if !is_compatible_rail(self.shared, self.world.get_rail_type(tile), totype)
                        {
                            self.command.wormhole_free(ret.0, tile, endtile, None);
                            if !ret.ok() {
                                error.take(ret);
                                break 'tile;
                            }
                        }

                        if flags.test(DoCommandFlag::Execute) {
                            w::hit(w::RAIL_CMD_CONVERT_WORMHOLE);
                            let track = diag_dir_to_diag_track(
                                DiagDirection(self.world.tunnel_bridge_dir(tile.0)),
                                self.check(),
                            );
                            if (self.rail.wormhole_reserved)(tile.0) {
                                let v = self.pbs.get_train_for_reservation(tile, track);
                                if v != Handle::NONE
                                    && !has_power_on_rail_types(self.shared, v.railtypes(), totype)
                                {
                                    w::hit(w::RAIL_CMD_CONVERT_REROUTE);
                                    /* No power on new rail type, reroute. */
                                    self.free_train_track_reservation(v);
                                    vehicles_affected.push(v);
                                }
                            }

                            /* Update the company infrastructure counters. */
                            let num_pieces = crate::tunnelbridge::get_tunnel_bridge_length(
                                self.map(),
                                tile.0,
                                endtile.0,
                            )
                            .wrapping_add(2)
                            .wrapping_mul(crate::tunnelbridge::TUNNELBRIDGE_TRACKBIT_FACTOR);
                            let owner = self.world.owner(tile);
                            let c = CompanyInfrastructure::get(self.shared, owner);
                            // SAFETY: the company is live and serialized; the rail types
                            // index its infrastructure array.
                            unsafe {
                                CompanyInfrastructure::add_rail(
                                    c,
                                    self.world.get_rail_type(tile),
                                    num_pieces.wrapping_neg(),
                                );
                                CompanyInfrastructure::add_rail(c, totype, num_pieces);
                            }
                            self.dirty_company_infrastructure_windows(owner);

                            (self.rail.set_rail_type)(tile.0, totype.0);
                            (self.rail.set_rail_type)(endtile.0, totype.0);

                            vehicles_on_tile(self.map(), tile, |v| {
                                if v.vehicle_type() == VehicleType::VEH_TRAIN {
                                    include(&mut affected_trains, &Handle::from_vehicle(v).first());
                                }
                            });
                            vehicles_on_tile(self.map(), endtile, |v| {
                                if v.vehicle_type() == VehicleType::VEH_TRAIN {
                                    include(&mut affected_trains, &Handle::from_vehicle(v).first());
                                }
                            });

                            yapf_notify_track_layout_change(tile, track);
                            yapf_notify_track_layout_change(endtile, track);

                            if self.world.bridge(tile) {
                                (self.shared.mark_bridge_dirty)(tile.0);
                            } else {
                                self.mark_tile_dirty_by_tile(tile);
                                self.mark_tile_dirty_by_tile(endtile);
                            }
                        }

                        found_convertible_track = true;
                        cost.add_money(
                            rail_convert_cost(self.shared, r#type, totype).saturating_mul(
                                i64::from(
                                    crate::tunnelbridge::get_tunnel_bridge_length(
                                        self.map(),
                                        tile.0,
                                        endtile.0,
                                    )
                                    .wrapping_add(2),
                                ),
                            ),
                        );
                    }

                    _ => {
                        // MP_STATION, MP_ROAD
                        if flags.test(DoCommandFlag::Execute) {
                            w::hit(w::RAIL_CMD_CONVERT_STATION_CROSSING);
                            let track = if tt == TileType::MP_STATION {
                                self.world.get_rail_station_track(tile)
                            } else {
                                crate::road_map::get_crossing_rail_track(&self.world, tile)
                            };
                            yapf_notify_track_layout_change(tile, track);
                        }

                        found_convertible_track = true;
                        cost.add_money(rail_convert_cost(self.shared, r#type, totype));
                    }
                }

                for v in vehicles_affected {
                    self.try_path_reserve(v, true);
                }
            }
            iter.advance(self.map(), self.shared.map_size(), self.check());
        }

        if flags.test(DoCommandFlag::Execute) {
            /* Railtype changed, update trains as when entering different track */
            for v in affected_trains {
                crate::train::consist_changed(self.train, self.shared, v, CCF_TRACK);
            }
        }

        if !found_convertible_track {
            cost.take(error);
        }
    }

    /// `RemoveTrainDepot`.
    fn remove_train_depot(&self, cost: Cost<'_>, tile: TileIndex, flags: DoCommandFlags) {
        if self.current_company() != Owner::WATER {
            cost.tile_ownership(self.command, tile.0);
            if !cost.ok() {
                return;
            }
        }

        self.command.no_vehicle(cost.0, tile);
        if !cost.ok() {
            return;
        }

        if flags.test(DoCommandFlag::Execute) {
            w::hit(w::RAIL_CMD_CLEAR_DEPOT);
            /* read variables before the depot is removed */
            let dir = self.world.get_rail_depot_direction(tile);
            let owner = self.world.owner(tile);
            let mut v = Handle::NONE;

            if (self.rail.depot_reserved)(tile.0) {
                v = self
                    .pbs
                    .get_train_for_reservation(tile, diag_dir_to_diag_track(dir, self.check()));
                if v != Handle::NONE {
                    self.free_train_track_reservation(v);
                }
            }

            // SAFETY: the company is live and serialized; the rail type indexes its
            // infrastructure array.
            unsafe {
                CompanyInfrastructure::add_rail(
                    CompanyInfrastructure::get(self.shared, owner),
                    self.world.get_rail_type(tile),
                    1_u32.wrapping_neg(),
                );
            }
            self.dirty_company_infrastructure_windows(owner);

            (self.s.delete_depot)(tile.0);
            crate::ground::do_clear_square(tile, self.shared);
            self.signals().add_side_to_signal_buffer(tile, dir, owner);
            yapf_notify_track_layout_change(tile, diag_dir_to_diag_track(dir, self.check()));
            if v != Handle::NONE {
                self.try_path_reserve(v, true);
            }
        }

        cost.set(crate::company::price(Price::PR_CLEAR_DEPOT_TRAIN));
    }

    /// `ClearTile_Track`.
    fn clear_tile_track(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
    ) {
        cost.set(0);

        if flags.test(DoCommandFlag::Auto) {
            if !self.world.is_tile_owner(tile, self.current_company()) {
                return cost.error(STR_ERROR_AREA_IS_OWNED_BY_ANOTHER.0);
            }

            if self.world.is_plain_rail(tile) {
                return cost.error(STR_ERROR_MUST_REMOVE_RAILROAD_TRACK.0);
            }
            return cost.error(STR_ERROR_BUILDING_MUST_BE_DEMOLISHED.0);
        }

        match self.world.get_rail_tile_type(tile) {
            RailTileType::Signals | RailTileType::Normal => {
                w::hit(w::RAIL_CMD_CLEAR_RAIL);
                let tileh = self.world.tile_slope(tile);
                /* Is there flat water on the lower halftile that gets cleared expensively? */
                let water_ground = self.world.get_rail_ground_type(tile)
                    == RailGroundType::HalfTileWater
                    && is_slope_with_one_corner_raised(tileh);

                let mut tracks = self.world.get_track_bits(tile);
                while tracks != TrackBits::TRACK_BIT_NONE {
                    let track = remove_first_track(&mut tracks, self.check());
                    self.remove_single_rail_command(
                        ret,
                        flags,
                        CmdRemoveSingleRail { tile, track },
                    );
                    if !ret.ok() {
                        return cost.take(ret);
                    }
                    cost.add_money(ret.money());
                }

                /* When bankrupting, don't make water dirty, there could be a ship on lower halftile.
                 * Same holds for non-companies clearing the tile, e.g. disasters. */
                if water_ground
                    && !flags.test(DoCommandFlag::Bankrupt)
                    && (self.shared.company_valid)(self.current_company().0)
                {
                    self.command.no_vehicle(ret.0, tile);
                    if !ret.ok() {
                        return cost.take(ret);
                    }

                    /* The track was removed, and left a coast tile. Now also clear the water. */
                    if flags.test(DoCommandFlag::Execute) {
                        w::hit(w::RAIL_CMD_CLEAR_WATER);
                        crate::ground::do_clear_square(tile, self.shared);
                    }
                    cost.add_money(crate::company::price(Price::PR_CLEAR_WATER));
                }
            }

            RailTileType::Depot => self.remove_train_depot(cost, tile, flags),

            _ => cost.error(INVALID_STRING_ID),
        }
    }

    /// `ChangeTileOwner_Track`; `ret` receives the discarded clear result.
    fn change_tile_owner_track(
        &self,
        ret: Cost<'_>,
        tile: TileIndex,
        old_owner: Owner,
        new_owner: Owner,
    ) {
        if !self.world.is_tile_owner(tile, old_owner) {
            return;
        }

        if new_owner == Owner::INVALID {
            w::hit(w::RAIL_CMD_OWNER_CLEAR);
            self.landscape_clear(
                ret,
                DoCommandFlags::from_element(DoCommandFlag::Execute)
                    .with(DoCommandFlags::from_element(DoCommandFlag::Bankrupt)),
                tile,
            );
        } else {
            w::hit(w::RAIL_CMD_OWNER_TRANSFER);
            /* Update company infrastructure counts. No need to dirty windows here, we'll redraw the whole screen anyway. */
            let mut num_pieces = 1;
            if self.world.is_plain_rail(tile) {
                num_pieces = pieces(self.world.get_track_bits(tile));
            }
            let rt = self.world.get_rail_type(tile);
            let old_infrastructure = CompanyInfrastructure::get(self.shared, old_owner);
            let new_infrastructure = CompanyInfrastructure::get(self.shared, new_owner);
            // SAFETY: both companies are live and serialized; the rail type indexes their
            // infrastructure arrays.
            unsafe {
                CompanyInfrastructure::add_rail(old_infrastructure, rt, num_pieces.wrapping_neg());
                CompanyInfrastructure::add_rail(new_infrastructure, rt, num_pieces);
            }

            if self.world.has_signals(tile) {
                let num_sigs = self.world.get_present_signals(tile).count_ones();
                // SAFETY: as above.
                unsafe {
                    CompanyInfrastructure::add_signal(old_infrastructure, num_sigs.wrapping_neg());
                    CompanyInfrastructure::add_signal(new_infrastructure, num_sigs);
                }
            }

            (self.shared.set_tile_owner)(tile.0, new_owner.0);
        }
    }

    /// `TestAutoslopeOnRailTile`: tests if autoslope is allowed.
    fn test_autoslope_on_rail_tile(&self, cost: Cost<'_>, args: TestAutoslopeOnRailTile) {
        let TestAutoslopeOnRailTile {
            tile,
            flags,
            mut z_old,
            mut tileh_old,
            mut z_new,
            mut tileh_new,
            rail_bits,
        } = args;
        if !(self.shared.build_on_slopes)() || !(self.shared.autoslope)() {
            return cost.error(STR_ERROR_MUST_REMOVE_RAILROAD_TRACK.0);
        }

        /* Is the slope-rail_bits combination valid in general? I.e. is it safe to call GetRailFoundation() ? */
        self.check_rail_slope(cost, tileh_new, rail_bits, TrackBits::TRACK_BIT_NONE, tile);
        if !cost.ok() {
            return cost.error(STR_ERROR_MUST_REMOVE_RAILROAD_TRACK.0);
        }

        /* Get the slopes on top of the foundations */
        let top = (self.shared.apply_foundation)(
            get_rail_foundation(tileh_old, rail_bits, self.check()),
            tileh_old,
        );
        z_old += top.z;
        tileh_old = top.slope;
        let top = (self.shared.apply_foundation)(
            get_rail_foundation(tileh_new, rail_bits, self.check()),
            tileh_new,
        );
        z_new += top.z;
        tileh_new = top.slope;

        let track_corner = match rail_bits {
            TrackBits::TRACK_BIT_LEFT => Corner::CORNER_W,
            TrackBits::TRACK_BIT_LOWER => Corner::CORNER_S,
            TrackBits::TRACK_BIT_RIGHT => Corner::CORNER_E,
            TrackBits::TRACK_BIT_UPPER => Corner::CORNER_N,

            /* Surface slope must not be changed */
            _ => {
                if z_old != z_new || tileh_old != tileh_new {
                    return cost.error(STR_ERROR_MUST_REMOVE_RAILROAD_TRACK.0);
                }
                return cost.set(crate::company::price(Price::PR_BUILD_FOUNDATION));
            }
        };

        /* The height of the track_corner must not be changed. The rest ensures GetRailFoundation() already. */
        z_old += crate::landscape::get_slope_z_in_corner(
            remove_halftile_slope(tileh_old).0,
            track_corner.0,
            self.check(),
        );
        z_new += crate::landscape::get_slope_z_in_corner(
            remove_halftile_slope(tileh_new).0,
            track_corner.0,
            self.check(),
        );
        if z_old != z_new {
            return cost.error(STR_ERROR_MUST_REMOVE_RAILROAD_TRACK.0);
        }

        cost.set(crate::company::price(Price::PR_BUILD_FOUNDATION));
        /* Make the ground dirty, if surface slope has changed */
        if tileh_old != tileh_new {
            /* If there is flat water on the lower halftile add the cost for clearing it */
            if self.world.get_rail_ground_type(tile) == RailGroundType::HalfTileWater
                && is_slope_with_one_corner_raised(tileh_old)
            {
                cost.add_money(crate::company::price(Price::PR_CLEAR_WATER));
            }
            if flags.test(DoCommandFlag::Execute) {
                (self.ground.set_ground)(tile.0, RailGroundType::Barren);
            }
        }
    }

    /// `TerraformTile_Track`.
    fn terraform_tile_track(
        &self,
        cost: Cost<'_>,
        tile: TileIndex,
        flags: DoCommandFlags,
        z_new: i32,
        tileh_new: Slope,
    ) {
        let (tileh_old, z_old) = self.world.tile_slope_z(tile);
        if self.world.is_plain_rail(tile) {
            let rail_bits = self.world.get_track_bits(tile);
            /* Is there flat water on the lower halftile that must be cleared expensively? */
            let was_water = self.world.get_rail_ground_type(tile) == RailGroundType::HalfTileWater
                && is_slope_with_one_corner_raised(tileh_old);

            /* Allow clearing the water only if there is no ship */
            if was_water
                && has_vehicle_on_tile(self.map(), tile, |v| {
                    v.vehicle_type() == VehicleType::VEH_SHIP
                })
            {
                return cost.error(STR_ERROR_SHIP_IN_THE_WAY.0);
            }

            /* First test autoslope. However if it succeeds we still have to test the rest, because non-autoslope terraforming is cheaper. */
            self.test_autoslope_on_rail_tile(
                cost,
                TestAutoslopeOnRailTile {
                    tile,
                    flags,
                    z_old,
                    tileh_old,
                    z_new,
                    tileh_new,
                    rail_bits,
                },
            );

            /* When there is only a single horizontal/vertical track, one corner can be terraformed. */
            let allowed_corner = match rail_bits {
                TrackBits::TRACK_BIT_RIGHT => Corner::CORNER_W,
                TrackBits::TRACK_BIT_UPPER => Corner::CORNER_S,
                TrackBits::TRACK_BIT_LEFT => Corner::CORNER_E,
                TrackBits::TRACK_BIT_LOWER => Corner::CORNER_N,
                _ => return, // the autoslope result
            };

            let f_old = get_rail_foundation(tileh_old, rail_bits, self.check());

            /* Do not allow terraforming if allowed_corner is part of anti-zig-zag foundations */
            if tileh_old != Slope::SLOPE_NS
                && tileh_old != Slope::SLOPE_EW
                && is_special_rail_foundation(f_old)
            {
                return; // the autoslope result
            }

            /* Everything is valid, which only changes allowed_corner */
            for corner in 0..Corner::CORNER_END.0 {
                let corner = Corner(corner);
                if allowed_corner == corner {
                    continue;
                }
                if z_old
                    + crate::landscape::get_slope_z_in_corner(tileh_old.0, corner.0, self.check())
                    != z_new
                        + crate::landscape::get_slope_z_in_corner(
                            tileh_new.0,
                            corner.0,
                            self.check(),
                        )
                {
                    return; // the autoslope result
                }
            }

            w::hit(w::RAIL_CMD_TERRAFORM_CORNER);
            /* Make the ground dirty */
            if flags.test(DoCommandFlag::Execute) {
                (self.ground.set_ground)(tile.0, RailGroundType::Barren);
            }

            /* allow terraforming */
            return cost.set(if was_water {
                crate::company::price(Price::PR_CLEAR_WATER)
            } else {
                0
            });
        } else if (self.shared.build_on_slopes)()
            && (self.shared.autoslope)()
            && (self.s.entrance_edge)(
                tile.0,
                z_new,
                tileh_new.0,
                self.world.get_rail_depot_direction(tile).0,
            )
        {
            w::hit(w::RAIL_CMD_TERRAFORM_DEPOT);
            return cost.set(crate::company::price(Price::PR_BUILD_FOUNDATION));
        }
        self.landscape_clear(cost, flags, tile);
    }

    /// `CheckBuildAbove_Track`.
    fn check_build_above_track(&self, cost: Cost<'_>, tile: TileIndex, flags: DoCommandFlags) {
        if self.world.is_plain_rail(tile) {
            return cost.zero();
        }
        w::hit(w::RAIL_CMD_BUILD_ABOVE_CLEAR);
        self.landscape_clear(cost, flags, tile);
    }
}

/// `Command<CMD_REMOVE_SINGLE_RAIL>::Do(flags, ...)` into `out`, for the station commands.
/// # Safety
/// The table and its nested tables are valid for the synchronous call.
pub(crate) unsafe fn remove_single_rail_command(
    s: *const RailCommandServices,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdRemoveSingleRail,
) {
    // Each nested body run counts the entry.
    let x = unsafe { Ctx::new(s) };
    x.remove_single_rail_command(out, flags, args);
}
/// `ClearTile_Track`.
/// # Safety
/// The tables are valid and `cost` and `ret` are distinct live native results for the
/// synchronous call.
pub(crate) unsafe fn clear_tile_track(
    s: *const RailCommandServices,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    tile: TileIndex,
    flags: DoCommandFlags,
) {
    let x = unsafe { Ctx::new(s) };
    w::hit(w::RAIL_CMD_ENTRIES);
    x.clear_tile_track(x.c(cost), x.c(ret), tile, flags);
}
/// [`clear_tile_track`] for `landscape_cmd.rs` until its pass.
/// # Safety
/// As [`clear_tile_track`].
pub(crate) unsafe fn clear_tile_track_raw(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    flags: u16,
    tile: u32,
) {
    unsafe { clear_tile_track(s, out, a, TileIndex(tile), DoCommandFlags(flags)) };
}
/// `ChangeTileOwner_Track`; `ret` receives the discarded clear result.
/// # Safety
/// The tables are valid and `ret` is a live native result for the synchronous call.
pub(crate) unsafe fn change_tile_owner_track(
    s: *const RailCommandServices,
    ret: *mut CommandCost,
    tile: TileIndex,
    old_owner: Owner,
    new_owner: Owner,
) {
    let x = unsafe { Ctx::new(s) };
    w::hit(w::RAIL_CMD_ENTRIES);
    x.change_tile_owner_track(x.c(ret), tile, old_owner, new_owner);
}
/// `TerraformTile_Track`.
/// # Safety
/// The tables are valid and `cost` is a live native result for the synchronous call.
pub(crate) unsafe fn terraform_tile_track(
    s: *const RailCommandServices,
    cost: *mut CommandCost,
    tile: TileIndex,
    flags: DoCommandFlags,
    z_new: i32,
    tileh_new: Slope,
) {
    let x = unsafe { Ctx::new(s) };
    w::hit(w::RAIL_CMD_ENTRIES);
    x.terraform_tile_track(x.c(cost), tile, flags, z_new, tileh_new);
}
/// [`terraform_tile_track`] for `landscape_cmd.rs` until its pass.
/// # Safety
/// As [`terraform_tile_track`].
pub(crate) unsafe fn terraform_tile_track_raw(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    z_new: i32,
    tileh_new: u8,
) {
    unsafe {
        terraform_tile_track(
            s,
            out,
            TileIndex(tile),
            DoCommandFlags(flags),
            z_new,
            Slope(tileh_new),
        );
    }
}
/// `CheckBuildAbove_Track`.
/// # Safety
/// The tables are valid and `cost` is a live native result for the synchronous call.
pub(crate) unsafe fn check_build_above_track(
    s: *const RailCommandServices,
    cost: *mut CommandCost,
    tile: TileIndex,
    flags: DoCommandFlags,
) {
    let x = unsafe { Ctx::new(s) };
    w::hit(w::RAIL_CMD_ENTRIES);
    x.check_build_above_track(x.c(cost), tile, flags);
}

// The exports below are the native command facades' shims: each takes the command's
// parameters as base integers, with the facade's scratch results, and builds the command's
// argument struct. Every entry shares the module's contract.

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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_build_single_rail(
        x.c(out),
        x.c(a),
        x.c(b),
        DoCommandFlags(flags),
        CmdBuildSingleRail {
            tile: TileIndex(tile),
            railtype: RailType(railtype),
            track: Track(track),
            auto_remove_signals,
        },
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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_remove_single_rail(
        x.c(out),
        x.c(a),
        DoCommandFlags(flags),
        CmdRemoveSingleRail {
            tile: TileIndex(tile),
            track: Track(track),
        },
    );
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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_rail_track_helper(
        x.c(out),
        x.c(a),
        x.c(b),
        DoCommandFlags(flags),
        RailTrackHelper {
            tile: TileIndex(tile),
            end_tile: TileIndex(end_tile),
            railtype: RailType(railtype),
            track: Track(track),
            remove,
            auto_remove_signals,
            fail_on_obstacle,
        },
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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_build_train_depot(
        x.c(out),
        x.c(a),
        DoCommandFlags(flags),
        CmdBuildTrainDepot {
            tile: TileIndex(tile),
            railtype: RailType(railtype),
            dir: DiagDirection(dir),
        },
    );
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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_build_single_signal(
        x.c(out),
        x.c(a),
        DoCommandFlags(flags),
        CmdBuildSingleSignal {
            tile: TileIndex(tile),
            track: Track(track),
            sigtype: SignalType(sigtype),
            sigvar: SignalVariant(sigvar),
            convert_signal,
            skip_existing_signals,
            ctrl_pressed,
            cycle_start: SignalType(cycle_start),
            cycle_stop: SignalType(cycle_stop),
            num_dir_cycle,
            signals_copy,
        },
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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_signal_track_helper(
        x.c(out),
        x.c(a),
        x.c(b),
        DoCommandFlags(flags),
        SignalTrackHelper {
            tile: TileIndex(tile),
            end_tile: TileIndex(end_tile),
            track: Track(track),
            sigtype: SignalType(sigtype),
            sigvar: SignalVariant(sigvar),
            mode,
            remove,
            autofill,
            minimise_gaps,
            signal_density,
        },
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
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_remove_single_signal(
        x.c(out),
        DoCommandFlags(flags),
        CmdRemoveSingleSignal {
            tile: TileIndex(tile),
            track: Track(track),
        },
    );
}
/// `CmdConvertRail`.
/// # Safety
/// See the module entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_convert(
    s: *const RailCommandServices,
    out: *mut CommandCost,
    a: *mut CommandCost,
    b: *mut CommandCost,
    flags: u16,
    tile: u32,
    area_start: u32,
    totype: u8,
    diagonal: bool,
) {
    let x = unsafe { Ctx::new(s) };
    w::hit(w::RAIL_CMD_ENTRIES);
    x.cmd_convert_rail(
        x.c(out),
        x.c(a),
        x.c(b),
        DoCommandFlags(flags),
        CmdConvertRail {
            tile: TileIndex(tile),
            area_start: TileIndex(area_start),
            totype: RailType(totype),
            diagonal,
        },
    );
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
    unsafe { check_build_above_track(s, out, TileIndex(tile), DoCommandFlags(flags)) };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRailCommandServices" ["probe"] => RailCommandServices {
        shared,
        rail,
        cost,
        command,
        signal,
        train,
        road_no_crossing,
        crossing_with_competitor,
        entrance_edge,
        layout_changed,
        mark_crossings,
        crossing_removed,
        make_crossing,
        make_road,
        depot_available,
        new_depot,
        depot_name,
        delete_depot,
        depot_windows,
    };
    "OpenTTDCrossingRoad" ["probe"] => CrossingRoad {
        bits,
        road_type,
        tram_type,
        town,
        road_owner,
        tram_owner,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    /// `CountBits`, squared for overlapping tracks.
    #[test]
    fn pieces_square_overlapping_tracks() {
        assert_eq!(pieces(TrackBits::TRACK_BIT_X | TrackBits::TRACK_BIT_Y), 4);
        assert_eq!(pieces(TrackBits::TRACK_BIT_HORZ), 2);
    }
}
