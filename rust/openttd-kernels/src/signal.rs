/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions related to rail signals updating (`signal.cpp`): signal-block traversal and its
//! process-wide fixed work sets. Occupancy and dirtying callbacks can reenter, so every work-set
//! borrow ends before a callback.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::collapsible_if)]
use crate::bitmath_func::{find_first_bit, has_at_most_one_bit};
use crate::company_type::{CompanyID, Owner};
use crate::debug::debug;
use crate::direction_func::{diag_dir_to_axis, is_valid_diag_direction, reverse_diag_dir};
use crate::direction_type::DiagDirection;
use crate::game_state::GameState;
use crate::map_type::TileIndexDiff;
use crate::movement::native_assert;
use crate::rail_map::is_pbs_signal;
use crate::services::{RailMapServices, Services};
use crate::signal_func::SigSegState;
use crate::signal_type::{SignalState, SignalType};
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{
    reverse_trackdir, track_bits_to_track, track_status_to_track_bits, track_to_track_bits,
    trackdir_to_exitdir, trackdir_to_track,
};
use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
use crate::transport_type::TransportType;
use crate::vehicle::{VehicleRef, VehicleServices};
use crate::vehicle_type::VehicleType;
use std::{cell::OnceCell, ffi::c_uint};

/* These are the maximums used for updating signal blocks. */
const SIG_TBU_SIZE: usize = 64; // number of signals entering to block
const SIG_TBD_SIZE: usize = 256; // number of intersections - open nodes in current block
const SIG_GLOB_SIZE: usize = 128; // number of open blocks (block can be opened more times until detected)
const SIG_GLOB_UPDATE: usize = 64; // how many items need to be in _globset to force update

/// Incidating trackbits with given enterdir.
const ENTERDIR_TO_TRACKBITS: [TrackBits; DiagDirection::DIAGDIR_END.0 as usize] = [
    TrackBits::TRACK_BIT_3WAY_NE,
    TrackBits::TRACK_BIT_3WAY_SE,
    TrackBits::TRACK_BIT_3WAY_SW,
    TrackBits::TRACK_BIT_3WAY_NW,
];

/// Incidating trackdirbits with given enterdir.
const ENTERDIR_TO_TRACKDIRBITS: [TrackdirBits; DiagDirection::DIAGDIR_END.0 as usize] = [
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_X_SW.0
            | TrackdirBits::TRACKDIR_BIT_UPPER_W.0
            | TrackdirBits::TRACKDIR_BIT_RIGHT_S.0,
    ),
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_Y_NW.0
            | TrackdirBits::TRACKDIR_BIT_LOWER_W.0
            | TrackdirBits::TRACKDIR_BIT_RIGHT_N.0,
    ),
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_X_NE.0
            | TrackdirBits::TRACKDIR_BIT_LOWER_E.0
            | TrackdirBits::TRACKDIR_BIT_LEFT_N.0,
    ),
    TrackdirBits(
        TrackdirBits::TRACKDIR_BIT_Y_SE.0
            | TrackdirBits::TRACKDIR_BIT_UPPER_E.0
            | TrackdirBits::TRACKDIR_BIT_LEFT_S.0,
    ),
];

/// The `Tdir` of a `SmallSet`: `Trackdir` or `DiagDirection`. Its base integer crosses only
/// the diagnostic boundary.
trait SetDir: Copy + PartialEq {
    /// The base integer of the direction.
    fn base(self) -> u8;
    /// The direction with this base integer.
    fn from_base(value: u8) -> Self;
}
impl SetDir for Trackdir {
    fn base(self) -> u8 {
        self.0
    }
    fn from_base(value: u8) -> Self {
        Self(value)
    }
}
impl SetDir for DiagDirection {
    fn base(self) -> u8 {
        self.0
    }
    fn from_base(value: u8) -> Self {
        Self(value)
    }
}

/// Element of set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SSdata<Tdir> {
    tile: TileIndex,
    dir: Tdir,
}

/// Set containing 'items' items of 'tile and Tdir'.
/// No tree structure is used because it would cause slowdowns in most usual cases.
struct SmallSet<Tdir, const ITEMS: usize> {
    n: usize,           // actual number of units
    overflowed: bool,   // did we try to overflow the set?
    name: &'static str, // name, used for debugging purposes...
    /// Branch witness hit when an add to the process-wide set fails.
    full_witness: usize,
    data: [SSdata<Tdir>; ITEMS],
}

impl<Tdir: SetDir, const ITEMS: usize> SmallSet<Tdir, ITEMS> {
    /// Constructor - just set default values and 'name'. `dir` fills the unused elements.
    const fn new(name: &'static str, full_witness: usize, dir: Tdir) -> Self {
        Self {
            n: 0,
            overflowed: false,
            name,
            full_witness,
            data: [SSdata {
                tile: TileIndex(0),
                dir,
            }; ITEMS],
        }
    }

    /// Reset variables to default values.
    fn reset(&mut self) {
        self.n = 0;
        self.overflowed = false;
    }

    /// Returns value of 'overflowed': did we try to overflow the set?
    fn overflowed(&self) -> bool {
        self.overflowed
    }

    /// Checks for empty set: is the set empty?
    fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Checks for full set: is the set full?
    fn is_full(&self) -> bool {
        self.n == ITEMS
    }

    /// Reads the number of items: current number of items.
    fn items(&self) -> usize {
        self.n
    }

    /// Tries to remove first instance of given tile and dir.
    /// Returns whether the element was found and removed.
    fn remove(&mut self, tile: TileIndex, dir: Tdir) -> bool {
        for i in 0..self.n {
            if self.data[i].tile == tile && self.data[i].dir == dir {
                self.n -= 1;
                self.data[i] = self.data[self.n];
                return true;
            }
        }

        false
    }

    /// Tries to find given tile and dir in the set.
    /// Returns true iff the tile & dir element was found.
    fn is_in(&self, tile: TileIndex, dir: Tdir) -> bool {
        for i in 0..self.n {
            if self.data[i].tile == tile && self.data[i].dir == dir {
                return true;
            }
        }

        false
    }

    /// Adds tile & dir into the set, checks for full set.
    /// Sets the 'overflowed' flag if the set was full.
    /// Returns true iff the item could be added (set wasn't full). On a full set the caller
    /// issues [`Self::debug_full`] once its work-set borrow has ended.
    fn add(&mut self, tile: TileIndex, dir: Tdir) -> bool {
        if self.is_full() {
            self.overflowed = true;
            return false; // set is full
        }

        self.data[self.n].tile = tile;
        self.data[self.n].dir = dir;
        self.n += 1;

        true
    }

    /// The `Debug` message of `Add` on a full set named `name`.
    fn debug_full(name: &str) {
        debug!(
            misc,
            0, "SignalSegment too complex. Set {} is full (maximum {})", name, ITEMS
        );
    }

    /// Reads the last added element into the set.
    /// Returns `None` iff the set was empty.
    fn get(&mut self) -> Option<SSdata<Tdir>> {
        if self.n == 0 {
            return None;
        }

        self.n -= 1;
        Some(self.data[self.n])
    }
}

/// The work sets and owner of `signal.cpp`'s file statics.
struct State {
    /// `_tbuset`: set of signals that will be updated.
    tbuset: SmallSet<Trackdir, SIG_TBU_SIZE>,
    /// `_tbdset`: set of open nodes in current signal block.
    tbdset: SmallSet<DiagDirection, SIG_TBD_SIZE>,
    /// `_globset`: set of places to be updated in following runs.
    globset: SmallSet<DiagDirection, SIG_GLOB_SIZE>,
    /// `_last_owner`: last owner whose track was put into _globset.
    last_owner: Owner,
}
static STATE: GameState<State> = GameState::new(State {
    tbuset: SmallSet::new(
        "_tbuset",
        crate::witness::SIGNAL_TBU_OVERFLOW,
        Trackdir::INVALID_TRACKDIR,
    ),
    tbdset: SmallSet::new(
        "_tbdset",
        crate::witness::SIGNAL_TBD_OVERFLOW,
        DiagDirection::INVALID_DIAGDIR,
    ),
    globset: SmallSet::new(
        "_globset",
        crate::witness::SIGNAL_GLOBAL_OVERFLOW,
        DiagDirection::INVALID_DIAGDIR,
    ),
    last_owner: Owner::INVALID,
});
fn access<T>(f: impl FnOnce(&mut State) -> T) -> T {
    // SAFETY: caller holds the native game-state mutex; closure never reenters.
    unsafe { f(&mut *STATE.get()) }
}
/// `SmallSet::Add` on a process-wide set, with its full-set `Debug` after the borrow ends.
fn add_to_set<Tdir: SetDir, const ITEMS: usize>(
    set: impl FnOnce(&mut State) -> &mut SmallSet<Tdir, ITEMS>,
    tile: TileIndex,
    dir: Tdir,
) -> bool {
    let (added, name, full_witness) = access(|s| {
        let set = set(s);
        (set.add(tile, dir), set.name, set.full_witness)
    });
    if !added {
        SmallSet::<Tdir, ITEMS>::debug_full(name);
        crate::witness::hit(full_witness);
    }
    added
}

/// Current signal block state flags.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SigFlag(u8);
impl SigFlag {
    /// Train found in segment.
    const TRAIN: Self = Self(0);
    /// Exitsignal found.
    const EXIT: Self = Self(1);
    /// Two or more exits found.
    const MULTI_EXIT: Self = Self(2);
    /// Green exitsignal found.
    const GREEN: Self = Self(3);
    /// Two or more green exits found.
    const MULTI_GREEN: Self = Self(4);
    /// Some of buffers was full, do not continue.
    const FULL: Self = Self(5);
    /// PBS signal found.
    const PBS: Self = Self(6);
    /// Track merge/split found.
    const SPLIT: Self = Self(7);
    /// Signal entering the block found.
    const ENTER: Self = Self(8);
    /// Two or more signals entering the block found.
    const MULTI_ENTER: Self = Self(9);
}
/// Original `EnumBitSet<SigFlag, uint16_t>`, including unnamed bits.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct SigFlags(u16);
impl SigFlags {
    const NONE: Self = Self(0);
    const FULL: Self = Self::from_flag(SigFlag::FULL);
    const SPLIT: Self = Self::from_flag(SigFlag::SPLIT);
    const MULTI_ENTER: Self = Self::from_flag(SigFlag::MULTI_ENTER);

    /// The set holding one element.
    const fn from_flag(flag: SigFlag) -> Self {
        Self(1 << flag.0)
    }
    /// `EnumBitSet::Test`.
    const fn test(self, flag: SigFlag) -> bool {
        self.0 & Self::from_flag(flag).0 != 0
    }
    /// `EnumBitSet::Set`.
    fn set(&mut self, flag: SigFlag) {
        self.0 |= Self::from_flag(flag).0;
    }
    /// `EnumBitSet::Any`.
    const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}
crate::base_bitset_type::impl_bitset_ops!(SigFlags);

/// Spatial occupancy and the original assertion policy.
#[repr(C)]
pub struct SignalServices {
    /// Native assertion build policy, independent of Cargo optimization.
    pub assertions: bool,
    /// Native `assert`, for the assertions of `signal.cpp`.
    pub assert_condition: extern "C" fn(bool),
    /// Direct Rust occupancy traversal, shared with the train controller.
    pub vehicles: *const VehicleServices,
}

/// One entry's service tables and its map view.
struct Signals<'a> {
    shared: &'a Services,
    rail: &'a RailMapServices,
    native: &'a SignalServices,
    /// The entry's map view, created at its first map read.
    map: OnceCell<crate::map_func::World<'a>>,
}
macro_rules! read {
    ($w:expr, $group:ident, $field:ident $(, $arg:expr)*) => {
        crate::witness::counted!(before SIGNAL_SERVICES, SIGNAL_BYTES, ($w.$group.$field)($($arg),*))
    };
}
impl<'a> Signals<'a> {
    /// The entry's map view. Queue-only entries read no map and never create it.
    fn map(&self) -> &crate::map_func::World<'a> {
        self.map
            .get_or_init(|| crate::map_func::World::new(self.shared))
    }
    /// The native assertion of a header helper's precondition.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        move |ok| native_assert(self.shared, ok)
    }
    /// `assert(condition)` of `signal.cpp`: evaluated only where the native build asserts.
    fn source_assert(&self, condition: impl FnOnce() -> bool) {
        if self.native.assertions {
            read!(self, native, assert_condition, condition());
        }
    }
    /// `TileOffsByDiagDir`.
    fn tile_offs_by_diag_dir(&self, dir: DiagDirection) -> TileIndexDiff {
        self.map()
            .dimensions
            .tile_offs_by_diag_dir(dir, self.check())
    }
    fn vehicles(&self) -> &VehicleServices {
        // SAFETY: Static services; signal scans delete no vehicle.
        unsafe { &*self.native.vehicles }
    }
    /// `if (!flags.Test(SigFlag::Train) && HasVehicleOnTile(tile, IsTrainAndNotInDepot)) flags.Set(SigFlag::Train);`
    fn train_on_tile(&self, tile: TileIndex, flags: &mut SigFlags) {
        let vehicles = self.vehicles();
        if !flags.test(SigFlag::TRAIN)
            && vehicles.any_on_tile(tile.0, |v| is_train_and_not_in_depot(vehicles, v))
        {
            flags.set(SigFlag::TRAIN);
            crate::witness::hit(crate::witness::SIGNAL_TRAIN);
        }
    }

    /// Perform some operations before adding data into Todo set.
    /// The new and reverse direction is removed from _globset, because we are sure
    /// it doesn't need to be checked again.
    /// Also, remove reverse direction from _tbdset.
    /// This is the 'core' part so the graph searching won't enter any tile twice.
    ///
    /// `t1` and `d1` are the tile and side we are entering, `t2` and `d2` the tile and side
    /// we are leaving. Returns false iff reverse direction was in Todo set.
    fn check_add_to_todo_set(
        &self,
        t1: TileIndex,
        d1: DiagDirection,
        t2: TileIndex,
        d2: DiagDirection,
    ) -> bool {
        access(|s| {
            s.globset.remove(t1, d1); // it can be in Global but not in Todo
            s.globset.remove(t2, d2); // remove in all cases
        });

        self.source_assert(|| access(|s| !s.tbdset.is_in(t1, d1))); // it really shouldn't be there already

        access(|s| !s.tbdset.remove(t2, d2))
    }

    /// Perform some operations before adding data into Todo set.
    /// The new and reverse direction is removed from Global set, because we are sure
    /// it doesn't need to be checked again.
    /// Also, remove reverse direction from Todo set.
    /// This is the 'core' part so the graph searching won't enter any tile twice.
    ///
    /// `t1` and `d1` are the tile and side we are entering, `t2` and `d2` the tile and side
    /// we are leaving. Returns false iff the Todo buffer would be overrun.
    fn maybe_add_to_todo_set(
        &self,
        t1: TileIndex,
        d1: DiagDirection,
        t2: TileIndex,
        d2: DiagDirection,
    ) -> bool {
        if !self.check_add_to_todo_set(t1, d1, t2, d2) {
            return true;
        }

        add_to_set(|s| &mut s.tbdset, t1, d1)
    }

    /// Search signal block of the `owner` whose signals we are updating.
    #[allow(clippy::too_many_lines)]
    fn explore_segment(&self, owner: Owner) -> SigFlags {
        let check = self.check();
        let mut flags = SigFlags::NONE;

        while let Some(SSdata {
            mut tile,
            dir: mut enterdir,
        }) = access(|s| s.tbdset.get())
        {
            crate::witness::hit(crate::witness::SIGNAL_TILES);
            let oldtile = tile; // tile we are leaving
            let mut exitdir = if enterdir == DiagDirection::INVALID_DIAGDIR {
                DiagDirection::INVALID_DIAGDIR
            } else {
                reverse_diag_dir(enterdir, check)
            }; // expected new exit direction (for straight line)

            let map = self.map();
            match map.kind(tile) {
                TileType::MP_RAILWAY => {
                    if map.owner(tile) != owner {
                        crate::witness::hit(crate::witness::SIGNAL_OWNER_BARRIER);
                        continue; // do not propagate signals on others' tiles (remove for tracksharing)
                    }

                    if map.is_rail_depot(tile.0) {
                        if enterdir == DiagDirection::INVALID_DIAGDIR {
                            // from 'inside' - train just entered or left the depot
                            crate::witness::hit(crate::witness::SIGNAL_DEPOT);
                            self.train_on_tile(tile, &mut flags);
                            exitdir = DiagDirection(map.rail_depot_dir(tile.0));
                            tile += self.tile_offs_by_diag_dir(exitdir);
                            enterdir = reverse_diag_dir(exitdir, check);
                        } else if enterdir == DiagDirection(map.rail_depot_dir(tile.0)) {
                            // entered a depot
                            self.train_on_tile(tile, &mut flags);
                            continue;
                        } else {
                            continue;
                        }
                    } else {
                        self.source_assert(|| is_valid_diag_direction(enterdir));
                        let mut tracks = TrackBits(map.tracks(tile.0)); // trackbits of tile
                        let tracks_masked = tracks & ENTERDIR_TO_TRACKBITS[usize::from(enterdir.0)]; // only incidating trackbits

                        if tracks == TrackBits::TRACK_BIT_HORZ
                            || tracks == TrackBits::TRACK_BIT_VERT
                        {
                            // there is exactly one incidating track, no need to check
                            tracks = tracks_masked;
                            // If no train detected yet, and there is not no train -> there is a train -> set the flag
                            if !flags.test(SigFlag::TRAIN)
                                && crate::vehicle::occupancy::ensure_no_train_on_track_bits(
                                    self.vehicles(),
                                    tile,
                                    tracks,
                                )
                                .is_some()
                            {
                                flags.set(SigFlag::TRAIN);
                                crate::witness::hit(crate::witness::SIGNAL_TRAIN);
                            }
                        } else {
                            if tracks_masked == TrackBits::TRACK_BIT_NONE {
                                continue; // no incidating track
                            }
                            self.train_on_tile(tile, &mut flags);
                        }

                        // Is this a track merge or split?
                        if !has_at_most_one_bit(u64::from(tracks.0)) {
                            flags.set(SigFlag::SPLIT);
                            crate::witness::hit(crate::witness::SIGNAL_SPLIT);
                        }

                        if map.has_signals(tile.0) {
                            // there is exactly one track - not zero, because there is exit from this tile
                            let track = Track(track_bits_to_track(tracks_masked.0, check)); // mask TRACK_BIT_X and Y too
                            if map.has_signal_on_track(tile.0, track.0) {
                                // now check whole track, not trackdir
                                let sig = SignalType(map.signal_type(tile.0, track.0));
                                let trackdir = Trackdir(find_first_bit(u64::from(
                                    (u32::from(tracks.0) * 0x101)
                                        & u32::from(
                                            ENTERDIR_TO_TRACKDIRBITS[usize::from(enterdir.0)].0,
                                        ),
                                )));
                                let reversedir = Trackdir(reverse_trackdir(trackdir.0));
                                // add (tile, reversetrackdir) to 'to-be-updated' set when there is
                                // ANY conventional signal in REVERSE direction
                                // (if it is a presignal EXIT and it changes, it will be added to 'to-be-done' set later)
                                if map.has_signal_on_trackdir(tile.0, reversedir.0) {
                                    if is_pbs_signal(sig.0) {
                                        flags.set(SigFlag::PBS);
                                        crate::witness::hit(crate::witness::SIGNAL_PBS);
                                    }
                                    if flags.test(SigFlag::ENTER) {
                                        flags.set(SigFlag::MULTI_ENTER);
                                        crate::witness::hit(crate::witness::SIGNAL_MULTI_ENTER);
                                    }
                                    flags.set(SigFlag::ENTER);

                                    if !add_to_set(|s| &mut s.tbuset, tile, reversedir) {
                                        return flags | SigFlags::FULL;
                                    }
                                }
                                if map.has_signal_on_trackdir(tile.0, trackdir.0)
                                    && !map.oneway_signal(tile.0, track.0)
                                {
                                    flags.set(SigFlag::PBS);
                                }

                                // if it is a presignal EXIT in OUR direction and we haven't found 2 green exits yes, do special check
                                if !flags.test(SigFlag::MULTI_GREEN)
                                    && map.is_presignal_exit(tile.0, track.0)
                                    && map.has_signal_on_trackdir(tile.0, trackdir.0)
                                {
                                    // found presignal exit
                                    if flags.test(SigFlag::EXIT) {
                                        flags.set(SigFlag::MULTI_EXIT); // found two (or more) exits
                                        crate::witness::hit(crate::witness::SIGNAL_MULTI_EXIT);
                                    }
                                    flags.set(SigFlag::EXIT); // found at least one exit - allow for compiler optimizations
                                    crate::witness::hit(crate::witness::SIGNAL_EXIT);
                                    if SignalState(map.signal_state_by_trackdir(tile.0, trackdir.0))
                                        == SignalState::SIGNAL_STATE_GREEN
                                    {
                                        // found green presignal exit
                                        if flags.test(SigFlag::GREEN) {
                                            flags.set(SigFlag::MULTI_GREEN);
                                            crate::witness::hit(crate::witness::SIGNAL_MULTI_GREEN);
                                        }
                                        flags.set(SigFlag::GREEN);
                                        crate::witness::hit(crate::witness::SIGNAL_GREEN_EXIT);
                                    }
                                }

                                continue;
                            }
                        }

                        for dir in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
                            // test all possible exit directions
                            let dir = DiagDirection(dir);
                            if dir != enterdir
                                && (tracks & ENTERDIR_TO_TRACKBITS[usize::from(dir.0)])
                                    != TrackBits::TRACK_BIT_NONE
                            {
                                // any track incidating?
                                let newtile = tile + self.tile_offs_by_diag_dir(dir); // new tile to check
                                let newdir = reverse_diag_dir(dir, check); // direction we are entering from
                                if !self.maybe_add_to_todo_set(newtile, newdir, tile, dir) {
                                    return flags | SigFlags::FULL;
                                }
                            }
                        }

                        continue; // continue the while() loop
                    }
                }

                TileType::MP_STATION => {
                    if !map.has_station_rail(tile) {
                        continue;
                    }
                    if map.owner(tile) != owner {
                        continue;
                    }
                    if diag_dir_to_axis(enterdir, check) != map.get_rail_station_axis(tile) {
                        continue; // different axis
                    }
                    if map.is_station_tile_blocked(tile) {
                        continue; // 'eye-candy' station tile
                    }

                    crate::witness::hit(crate::witness::SIGNAL_STATION);
                    self.train_on_tile(tile, &mut flags);
                    tile += self.tile_offs_by_diag_dir(exitdir);
                }

                TileType::MP_ROAD => {
                    if !map.level_crossing(tile.0) {
                        continue;
                    }
                    if map.owner(tile) != owner {
                        continue;
                    }
                    if diag_dir_to_axis(enterdir, check).0 == map.crossing_road_axis(tile.0) {
                        continue; // different axis
                    }

                    crate::witness::hit(crate::witness::SIGNAL_CROSSING);
                    self.train_on_tile(tile, &mut flags);
                    tile += self.tile_offs_by_diag_dir(exitdir);
                }

                TileType::MP_TUNNELBRIDGE => {
                    if map.owner(tile) != owner {
                        continue;
                    }
                    if TransportType(map.tunnel_bridge_transport(tile.0))
                        != TransportType::TRANSPORT_RAIL
                    {
                        continue;
                    }
                    let dir = DiagDirection(map.tunnel_bridge_dir(tile.0));
                    crate::witness::hit(crate::witness::SIGNAL_WORMHOLE);

                    if enterdir == DiagDirection::INVALID_DIAGDIR {
                        // incoming from the wormhole
                        self.train_on_tile(tile, &mut flags);
                        enterdir = dir;
                        exitdir = reverse_diag_dir(dir, check);
                        tile += self.tile_offs_by_diag_dir(exitdir); // just skip to next tile
                    } else {
                        // NOT incoming from the wormhole!
                        if reverse_diag_dir(enterdir, check) != dir {
                            continue;
                        }
                        self.train_on_tile(tile, &mut flags);
                        tile = TileIndex(read!(self, rail, other_tunnel_bridge_end, tile.0)); // just skip to exit tile
                        enterdir = DiagDirection::INVALID_DIAGDIR;
                        exitdir = DiagDirection::INVALID_DIAGDIR;
                    }
                }

                _ => continue, // continue the while() loop
            }

            if !self.maybe_add_to_todo_set(tile, enterdir, oldtile, exitdir) {
                return flags | SigFlags::FULL;
            }
        }

        flags
    }

    /// Update signals around segment in _tbuset, with the `flags` info about the segment.
    fn update_signals_around_segment(&self, flags: SigFlags) {
        let check = self.check();
        while let Some(SSdata {
            tile,
            dir: trackdir,
        }) = access(|s| s.tbuset.get())
        {
            let map = self.map();
            self.source_assert(|| map.has_signal_on_trackdir(tile.0, trackdir.0));

            let track = Track(trackdir_to_track(trackdir.0));
            let sig = SignalType(map.signal_type(tile.0, track.0));
            let mut newstate = SignalState::SIGNAL_STATE_GREEN;

            // Signal state of reserved path signals is handled by the reserve/unreserve process.
            if is_pbs_signal(sig.0)
                && (read!(self, rail, rail_reservation, tile.0)
                    & TrackBits(track_to_track_bits(track.0, check)))
                    != TrackBits::TRACK_BIT_NONE
            {
                crate::witness::hit(crate::witness::SIGNAL_RESERVED_SKIP);
                continue;
            }

            // determine whether the new state is red
            if flags.test(SigFlag::TRAIN) {
                // train in the segment
                newstate = SignalState::SIGNAL_STATE_RED;
            } else if is_pbs_signal(sig.0) && flags.any(SigFlags::SPLIT | SigFlags::MULTI_ENTER) {
                // Turn path signals red if the segment has a junction or more than one way in.
                newstate = SignalState::SIGNAL_STATE_RED;
            } else if sig == SignalType::SIGTYPE_COMBO
                && map.has_signal_on_trackdir(tile.0, reverse_trackdir(trackdir.0))
            {
                // is it a bidir combo? - then do not count its other signal direction as exit
                crate::witness::hit(crate::witness::SIGNAL_COMBO);
                // at least one more exit
                if flags.test(SigFlag::MULTI_EXIT)
                    // no green exit
                    && (!flags.test(SigFlag::GREEN)
                        // only one green exit, and it is this one - so all other exits are red
                        || (!flags.test(SigFlag::MULTI_GREEN)
                            && SignalState(map.signal_state_by_trackdir(
                                tile.0,
                                reverse_trackdir(trackdir.0),
                            )) == SignalState::SIGNAL_STATE_GREEN))
                {
                    newstate = SignalState::SIGNAL_STATE_RED;
                    crate::witness::hit(crate::witness::SIGNAL_COMBO_RED);
                }
            } else if map.is_presignal_entry(tile.0, trackdir_to_track(trackdir.0))
                && flags.test(SigFlag::EXIT)
                && !flags.test(SigFlag::GREEN)
            {
                // entry, at least one exit, no green exit
                newstate = SignalState::SIGNAL_STATE_RED;
            }

            // only when the state changes
            if newstate != SignalState(map.signal_state_by_trackdir(tile.0, trackdir.0)) {
                if map.is_presignal_exit(tile.0, trackdir_to_track(trackdir.0)) {
                    // for pre-signal exits, add block to the global set
                    let exitdir = DiagDirection(trackdir_to_exitdir(reverse_trackdir(trackdir.0)));
                    add_to_set(|s| &mut s.globset, tile, exitdir); // do not check for full global set, first update all signals
                    crate::witness::hit(crate::witness::SIGNAL_PROPAGATE);
                }
                read!(self, rail, set_signal_state, tile.0, trackdir.0, newstate.0);
                read!(self, shared, mark_dirty, tile.0);
                crate::witness::hit(crate::witness::SIGNAL_CHANGED);
            }
        }
    }

    /// `GetTileTrackStatus(tile, TRANSPORT_RAIL, 0)` has track bits incidating with `dir`.
    fn has_incidating_track(&self, tile: TileIndex, dir: DiagDirection) -> bool {
        let status = crate::movement::table(self.shared).get_tile_track_status_in_world(
            self.map(),
            tile,
            TransportType::TRANSPORT_RAIL,
            0,
            DiagDirection::INVALID_DIAGDIR,
        );
        (TrackBits(track_status_to_track_bits(status)) & ENTERDIR_TO_TRACKBITS[usize::from(dir.0)])
            != TrackBits::TRACK_BIT_NONE
    }

    /// Updates blocks in _globset buffer for the company `owner` whose signals we are updating
    /// (`UpdateSignalsInBuffer(Owner)`). Returns the state of the first block from _globset.
    /// Precondition: `Company::IsValidID(owner)`.
    fn update_signals_in_buffer_owner(&self, owner: Owner) -> SigSegState {
        let check = self.check();
        self.source_assert(|| (self.shared.company_valid)(owner.0));
        crate::witness::hit(crate::witness::SIGNAL_UPDATES);

        let mut first = true; // first block?
        let mut state = SigSegState::SIGSEG_FREE; // value to return

        while let Some(SSdata { mut tile, mut dir }) = access(|s| s.globset.get()) {
            self.source_assert(|| access(|s| s.tbuset.is_empty()));
            self.source_assert(|| access(|s| s.tbdset.is_empty()));

            // After updating signal, data stored are always MP_RAILWAY with signals.
            // Other situations happen when data are from outside functions -
            // modification of railbits (including both rail building and removal),
            // train entering/leaving block, train leaving depot...
            let map = self.map();
            let kind = map.kind(tile);
            if kind == TileType::MP_TUNNELBRIDGE {
                // 'optimization assert' - do not try to update signals when it is not needed
                self.source_assert(|| {
                    TransportType(map.tunnel_bridge_transport(tile.0))
                        == TransportType::TRANSPORT_RAIL
                });
                self.source_assert(|| {
                    dir == DiagDirection::INVALID_DIAGDIR
                        || dir
                            == reverse_diag_dir(DiagDirection(map.tunnel_bridge_dir(tile.0)), check)
                });
                add_to_set(|s| &mut s.tbdset, tile, DiagDirection::INVALID_DIAGDIR); // we can safely start from wormhole centre
                let other = TileIndex(read!(self, rail, other_tunnel_bridge_end, tile.0));
                add_to_set(|s| &mut s.tbdset, other, DiagDirection::INVALID_DIAGDIR);
            } else if kind == TileType::MP_RAILWAY && map.is_rail_depot(tile.0) {
                // 'optimization assert' do not try to update signals in other cases
                self.source_assert(|| {
                    dir == DiagDirection::INVALID_DIAGDIR
                        || dir == DiagDirection(map.rail_depot_dir(tile.0))
                });
                add_to_set(|s| &mut s.tbdset, tile, DiagDirection::INVALID_DIAGDIR); // start from depot inside
            } else if (kind == TileType::MP_RAILWAY
                || kind == TileType::MP_STATION
                || kind == TileType::MP_ROAD)
                && self.has_incidating_track(tile, dir)
            {
                // only add to set when there is some 'interesting' track
                add_to_set(|s| &mut s.tbdset, tile, dir);
                let next = tile + self.tile_offs_by_diag_dir(dir);
                add_to_set(|s| &mut s.tbdset, next, reverse_diag_dir(dir, check));
            } else {
                // jump to next tile
                tile += self.tile_offs_by_diag_dir(dir);
                dir = reverse_diag_dir(dir, check);
                if !self.has_incidating_track(tile, dir) {
                    // happens when removing a rail that wasn't connected at one or both sides
                    crate::witness::hit(crate::witness::SIGNAL_NEIGHBOUR_CONTINUE);
                    continue; // continue the while() loop
                }
                add_to_set(|s| &mut s.tbdset, tile, dir);
            }

            self.source_assert(|| access(|s| !s.tbdset.overflowed())); // it really shouldn't overflow by these one or two items
            self.source_assert(|| access(|s| !s.tbdset.is_empty())); // it wouldn't hurt anyone, but shouldn't happen too

            let flags = self.explore_segment(owner);

            if first {
                first = false;
                // SIGSEG_FREE is set by default
                if flags.test(SigFlag::PBS) {
                    state = SigSegState::SIGSEG_PBS;
                } else if flags.test(SigFlag::TRAIN)
                    || (flags.test(SigFlag::EXIT) && !flags.test(SigFlag::GREEN))
                    || flags.test(SigFlag::FULL)
                {
                    state = SigSegState::SIGSEG_FULL;
                }
            }

            // do not do anything when some buffer was full
            if flags.test(SigFlag::FULL) {
                reset_sets(); // free all sets
                crate::witness::hit(crate::witness::SIGNAL_RESET);
                break;
            }

            self.update_signals_around_segment(flags);
        }

        state
    }

    /// Update signals in buffer. Called from 'outside' (`UpdateSignalsInBuffer()`).
    fn update_signals_in_buffer(&self) {
        if access(|s| !s.globset.is_empty()) {
            self.update_signals_in_buffer_owner(access(|s| s.last_owner));
            access(|s| s.last_owner = Owner::INVALID); // invalidate
        }
    }

    /// Add `track` of `tile` to signal update buffer, to update signals at its ends for `owner`.
    fn add_track_to_signal_buffer(&self, tile: TileIndex, track: Track, owner: Owner) {
        const SEARCH_DIR_1: [DiagDirection; 6] = [
            DiagDirection::DIAGDIR_NE,
            DiagDirection::DIAGDIR_SE,
            DiagDirection::DIAGDIR_NE,
            DiagDirection::DIAGDIR_SE,
            DiagDirection::DIAGDIR_SW,
            DiagDirection::DIAGDIR_SE,
        ];
        const SEARCH_DIR_2: [DiagDirection; 6] = [
            DiagDirection::DIAGDIR_SW,
            DiagDirection::DIAGDIR_NW,
            DiagDirection::DIAGDIR_NW,
            DiagDirection::DIAGDIR_SW,
            DiagDirection::DIAGDIR_NW,
            DiagDirection::DIAGDIR_NE,
        ];

        // do not allow signal updates for two companies in one run
        self.source_assert(|| access(|s| s.globset.is_empty() || owner == s.last_owner));

        access(|s| s.last_owner = owner);

        add_to_set(|s| &mut s.globset, tile, SEARCH_DIR_1[usize::from(track.0)]);
        add_to_set(|s| &mut s.globset, tile, SEARCH_DIR_2[usize::from(track.0)]);

        if access(|s| s.globset.items() >= SIG_GLOB_UPDATE) {
            // too many items, force update
            crate::witness::hit(crate::witness::SIGNAL_FORCED_FLUSH);
            self.update_signals_in_buffer_owner(access(|s| s.last_owner));
            access(|s| s.last_owner = Owner::INVALID);
        }
    }

    /// Add `side` of `tile` to signal update buffer, to update signals of `owner`.
    fn add_side_to_signal_buffer(&self, tile: TileIndex, side: DiagDirection, owner: Owner) {
        // do not allow signal updates for two companies in one run
        self.source_assert(|| access(|s| s.globset.is_empty() || owner == s.last_owner));

        access(|s| s.last_owner = owner);

        add_to_set(|s| &mut s.globset, tile, side);

        if access(|s| s.globset.items() >= SIG_GLOB_UPDATE) {
            // too many items, force update
            crate::witness::hit(crate::witness::SIGNAL_FORCED_FLUSH);
            self.update_signals_in_buffer_owner(access(|s| s.last_owner));
            access(|s| s.last_owner = Owner::INVALID);
        }
    }

    /// Update signals of `owner`, starting at `side` of `tile`.
    /// Will check tile next to this at opposite side too.
    /// Returns the state of the signal segment.
    fn update_signals_on_segment(
        &self,
        tile: TileIndex,
        side: DiagDirection,
        owner: Owner,
    ) -> SigSegState {
        self.source_assert(|| access(|s| s.globset.is_empty()));
        add_to_set(|s| &mut s.globset, tile, side);

        self.update_signals_in_buffer_owner(owner)
    }

    /// Update signals of `owner` at segments that are at both ends of
    /// given (existent or non-existent) `track` of `tile`.
    fn set_signals_on_both_dir(&self, tile: TileIndex, track: Track, owner: Owner) {
        self.source_assert(|| access(|s| s.globset.is_empty()));

        self.add_track_to_signal_buffer(tile, track, owner);
        self.update_signals_in_buffer_owner(owner);
    }
}

/// Check whether there is a train on rail, not in a depot.
fn is_train_and_not_in_depot(vehicles: &VehicleServices, v: VehicleRef) -> bool {
    VehicleType(v.vehicle_type()) == VehicleType::TRAIN && {
        crate::witness::hit(crate::witness::SIGNAL_OCCUPANCY_ACCESSORS);
        TrackBits((vehicles.train_handle)(v.0.as_ptr()).track()) != TrackBits::TRACK_BIT_DEPOT
    }
}

/// Reset all sets after one set overflowed.
fn reset_sets() {
    access(|s| {
        s.tbuset.reset();
        s.tbdset.reset();
        s.globset.reset();
    });
}

// SAFETY for every exported entry: immutable tables outlive the synchronous
// invocation. Game-state mutex is held; callbacks do not retain any Rust owner
// borrow and cannot throw ordinary-play exceptions. No entry draws Random.
/// `UpdateSignalsInBuffer()`: flush queued blocks and invalidate the last owner only when
/// nonempty.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_flush(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    Context {
        shared,
        rail,
        native,
    }
    .update_signals_in_buffer();
}
/// `AddTrackToSignalBuffer`: queue both ends of a track and preserve the 64-entry forced flush.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_add_track(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    tile: u32,
    track: u8,
    owner: u8,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    Context {
        shared,
        rail,
        native,
    }
    .add_track_to_signal_buffer(TileIndex(tile), Track(track), CompanyID(owner));
}
/// `AddSideToSignalBuffer`: queue one side with the original owner lifetime and forced flush.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_add_side(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    tile: u32,
    side: u8,
    owner: u8,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    Context {
        shared,
        rail,
        native,
    }
    .add_side_to_signal_buffer(TileIndex(tile), DiagDirection(side), CompanyID(owner));
}
/// `UpdateSignalsOnSegment`: update a segment, returning the classification of the first
/// explored block.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_segment(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    tile: u32,
    side: u8,
    owner: u8,
) -> u8 {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    let state = Context {
        shared,
        rail,
        native,
    }
    .update_signals_on_segment(TileIndex(tile), DiagDirection(side), CompanyID(owner));
    crate::witness::hit(crate::witness::SIGNAL_CLASSIFICATION_BYTES);
    state.0
}
/// `SetSignalsOnBothDir`: update both track ends with the original private flush owner
/// lifetime.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_both(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    tile: u32,
    track: u8,
    owner: u8,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    Context {
        shared,
        rail,
        native,
    }
    .set_signals_on_both_dir(TileIndex(tile), Track(track), CompanyID(owner));
}

/// Call-scoped diagnostic image of the private signal work sets. No live map or
/// pool state is included; only the ordered active prefixes are meaningful.
#[repr(C)]
pub struct Snapshot {
    /// Active to-be-updated signal count.
    pub tbu_count: c_uint,
    /// Active traversal frontier count.
    pub tbd_count: c_uint,
    /// Active global block frontier count.
    pub global_count: c_uint,
    /// Original overflow flags in tbu/tbd/global order.
    pub overflow: [bool; 3],
    /// Canonical last owner, including its original stale-owner cases.
    pub last_owner: u8,
    /// Ordered active signal prefix; tail is zero-filled diagnostic output.
    pub tbu: [SignalItem; 64],
    /// Ordered active traversal prefix; tail is zero-filled diagnostic output.
    pub tbd: [SignalItem; 256],
    /// Ordered active global prefix; tail is zero-filled diagnostic output.
    pub global: [SignalItem; 128],
}
/// Tile/direction diagnostic element.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SignalItem {
    /// Original `TileIndex`.
    pub tile: u32,
    /// Original `Trackdir` or `DiagDirection`, including sentinel 255.
    pub dir: u8,
}
impl<Tdir: SetDir> From<SSdata<Tdir>> for SignalItem {
    fn from(item: SSdata<Tdir>) -> Self {
        Self {
            tile: item.tile.0,
            dir: item.dir.base(),
        }
    }
}
/// The ordered active prefix of `set`, then zero-filled diagnostic output.
fn inspect_set<Tdir: SetDir, const ITEMS: usize>(
    dest: &mut [SignalItem],
    set: &SmallSet<Tdir, ITEMS>,
) {
    dest.fill(SignalItem::default());
    for (dest, source) in dest.iter_mut().zip(&set.data[..set.n]) {
        *dest = SignalItem::from(*source);
    }
}
/// Inspect pending sets without altering their ordering, counts or lifetime.
/// # Safety
/// `output` is writable for one Snapshot; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_inspect(output: *mut Snapshot) {
    // SAFETY: caller owns this output, which does not alias STATE.
    let out = unsafe { &mut *output };
    access(|s| {
        out.tbu_count = s.tbuset.items() as c_uint;
        out.tbd_count = s.tbdset.items() as c_uint;
        out.global_count = s.globset.items() as c_uint;
        out.overflow = [
            s.tbuset.overflowed(),
            s.tbdset.overflowed(),
            s.globset.overflowed(),
        ];
        out.last_owner = s.last_owner.0;
        inspect_set(&mut out.tbu, &s.tbuset);
        inspect_set(&mut out.tbd, &s.tbdset);
        inspect_set(&mut out.global, &s.globset);
    });
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSignalSetTrace" ["probe"] => SetTrace {
        duplicate_count,
        duplicate_pop,
        full_failed,
        overflow_full,
        first_after_remove,
        refill_succeeded,
        refill_pop,
        drained_count,
        overflow_drained,
        overflow_reset,
    };
    "OpenTTDSignalSnapshot" ["probe"] => Snapshot {
        tbu_count,
        tbd_count,
        global_count,
        overflow,
        last_owner,
        tbu,
        tbd,
        global,
    };
    "OpenTTDSignalItem" ["probe"] => SignalItem {
        tile,
        dir,
    };
    "OpenTTDSignalServices" ["probe"] => SignalServices {
        assertions,
        assert_condition,
        vehicles,
    };
    "OpenTTDSignalContext" ["probe"] => Context {
        shared,
        rail,
        native,
    };
}

/// Call-scoped bridge to the signal entries from other Rust owners. All pointers address
/// immutable process-lifetime native service tables.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Context {
    /// Existing canonical shared map operations.
    pub shared: *const Services,
    /// Canonical rail-map observations.
    pub rail: *const RailMapServices,
    /// Occupancy and source assertions.
    pub native: *const SignalServices,
}
impl Context {
    fn signals(&self) -> Signals<'_> {
        // SAFETY: native acquisition supplied immutable process-lifetime tables;
        // the caller holds the game-state mutex throughout this synchronous call.
        unsafe {
            Signals {
                shared: &*self.shared,
                rail: &*self.rail,
                native: &*self.native,
                map: OnceCell::new(),
            }
        }
    }
    /// `UpdateSignalsInBuffer()`.
    pub(crate) fn update_signals_in_buffer(self) {
        self.signals().update_signals_in_buffer();
    }
    /// `AddTrackToSignalBuffer`.
    pub(crate) fn add_track_to_signal_buffer(self, tile: TileIndex, track: Track, owner: Owner) {
        self.signals()
            .add_track_to_signal_buffer(tile, track, owner);
    }
    /// `AddSideToSignalBuffer`.
    pub(crate) fn add_side_to_signal_buffer(
        self,
        tile: TileIndex,
        side: DiagDirection,
        owner: Owner,
    ) {
        self.signals().add_side_to_signal_buffer(tile, side, owner);
    }
    /// `UpdateSignalsOnSegment`.
    pub(crate) fn update_signals_on_segment(
        self,
        tile: TileIndex,
        side: DiagDirection,
        owner: Owner,
    ) -> SigSegState {
        self.signals().update_signals_on_segment(tile, side, owner)
    }
    /// `SetSignalsOnBothDir`.
    pub(crate) fn set_signals_on_both_dir(self, tile: TileIndex, track: Track, owner: Owner) {
        self.signals().set_signals_on_both_dir(tile, track, owner);
    }
    /// Temporary raw adapter of [`Self::update_signals_on_segment`] for `train.rs` until its pass.
    pub(crate) fn segment(self, tile: u32, side: u8, owner: u8) -> u8 {
        self.update_signals_on_segment(TileIndex(tile), DiagDirection(side), CompanyID(owner))
            .0
    }
    /// Temporary raw adapter of [`Self::set_signals_on_both_dir`] for `train.rs` until its pass.
    pub(crate) fn both(self, tile: u32, track: u8, owner: u8) {
        self.set_signals_on_both_dir(TileIndex(tile), Track(track), CompanyID(owner));
    }
    /// Temporary raw adapter of [`Self::add_track_to_signal_buffer`] for `rail_cmd.rs` until
    /// its pass.
    pub(crate) fn add_track(self, tile: u32, track: u8, owner: u8) {
        self.add_track_to_signal_buffer(TileIndex(tile), Track(track), CompanyID(owner));
    }
    /// Temporary raw adapter of [`Self::add_side_to_signal_buffer`] for `rail_cmd.rs`,
    /// `train_reservation.rs` and `tunnelbridge_cmd.rs` until their passes.
    pub(crate) fn add_side(self, tile: u32, side: u8, owner: u8) {
        self.add_side_to_signal_buffer(TileIndex(tile), DiagDirection(side), CompanyID(owner));
    }
    /// Temporary raw adapter of [`Self::update_signals_in_buffer`] for `train_reservation.rs`
    /// and `water.rs` until their passes.
    pub(crate) fn flush(self) {
        self.update_signals_in_buffer();
    }
}
/// Acquire call-scoped context while counting its real Rust-to-native crossing.
pub(crate) fn acquire_context(get: extern "C" fn() -> Context) -> Context {
    let context = get();
    crate::witness::hit(crate::witness::SIGNAL_CONTEXT_ACQUISITIONS);
    crate::witness::add(
        crate::witness::SIGNAL_CONTEXT_BYTES,
        std::mem::size_of::<Context>() as u64,
    );
    context
}
/// One actual native owner read at a direct signal-call branch (one byte).
pub(crate) fn count_context_owner_read() {
    crate::witness::hit(crate::witness::SIGNAL_CONTEXT_OWNER_READS);
}
/// Narrow unchanged-body probe trace for `SmallSet` operations not reachable from
/// the stock rail layouts (not a production signal entry or second authority).
#[repr(C)]
pub struct SetTrace {
    /// Duplicates remain after one first-match/swap-last removal.
    pub duplicate_count: c_uint,
    /// LIFO drain after the duplicate/reverse removal sequence.
    pub duplicate_pop: [SignalItem; 3],
    /// Add on a full fixed array failed.
    pub full_failed: bool,
    /// Failure set the sticky overflow flag.
    pub overflow_full: bool,
    /// Swap-last preserved the last element at the removed first position.
    pub first_after_remove: SignalItem,
    /// Refill succeeds after removal/drain with sticky overflow; Reset empties it.
    pub refill_succeeded: bool,
    /// First LIFO item after refill.
    pub refill_pop: SignalItem,
    /// Drained set is empty.
    pub drained_count: c_uint,
    /// Drain keeps the overflow flag.
    pub overflow_drained: bool,
    /// Reset clears the overflow flag.
    pub overflow_reset: bool,
}
/// The probe's next popped item; an empty set yields the zero item, which the native
/// comparison rejects.
fn pop_item<Tdir: SetDir, const N: usize>(set: &mut SmallSet<Tdir, N>) -> SignalItem {
    set.get().map_or_else(SignalItem::default, SignalItem::from)
}
fn probe_set<Tdir: SetDir, const N: usize>(name: &'static str, full_witness: usize) -> SetTrace {
    let mut set = SmallSet::<Tdir, N>::new(name, full_witness, Tdir::from_base(0));
    let tile = |t: u32| TileIndex(t);
    let a = (tile(1), Tdir::from_base(0));
    for (t, d) in [
        a,
        (tile(2), Tdir::from_base(DiagDirection::INVALID_DIAGDIR.0)),
        a,
        (tile(1), Tdir::from_base(2)),
    ] {
        set.add(t, d);
    }
    set.remove(a.0, a.1);
    let duplicate_count = set.items() as c_uint;
    let duplicate_pop = std::array::from_fn(|_| pop_item(&mut set));
    set.reset();
    for t in 0..N {
        set.add(tile(t as u32), Tdir::from_base((t & 3) as u8));
    }
    let full_failed = !set.add(
        tile(u32::MAX),
        Tdir::from_base(DiagDirection::INVALID_DIAGDIR.0),
    );
    if full_failed {
        SmallSet::<Tdir, N>::debug_full(set.name);
    }
    let overflow_full = set.overflowed();
    set.remove(tile(0), Tdir::from_base(0));
    let first_after_remove = set.data[0].into();
    let mut refill_succeeded = set.add(
        tile(u32::MAX),
        Tdir::from_base(DiagDirection::INVALID_DIAGDIR.0),
    );
    let refill_pop = pop_item(&mut set);
    while set.get().is_some() {}
    let drained_count = set.items() as c_uint;
    let overflow_drained = set.overflowed();
    refill_succeeded &= set.add(
        tile(u32::MAX),
        Tdir::from_base(DiagDirection::INVALID_DIAGDIR.0),
    );
    set.reset();
    refill_succeeded &= set.is_empty();
    SetTrace {
        duplicate_count,
        duplicate_pop,
        full_failed,
        overflow_full,
        first_after_remove,
        refill_succeeded,
        refill_pop,
        drained_count,
        overflow_drained,
        overflow_reset: set.overflowed(),
    }
}
/// Compare all three fixed capacities against the unchanged source `SmallSet`
/// body. Ordinary saves cannot expose its sticky overflow bookkeeping.
/// # Safety
/// Native services are immutable and output holds three writable `SetTrace` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_workset_probe(output: *mut SetTrace) {
    // SAFETY: caller supplies live immutable callbacks and three output records.
    unsafe {
        output.write(probe_set::<Trackdir, SIG_TBU_SIZE>(
            "_tbuset",
            crate::witness::SIGNAL_TBU_OVERFLOW,
        ));
        output
            .add(1)
            .write(probe_set::<DiagDirection, SIG_TBD_SIZE>(
                "_tbdset",
                crate::witness::SIGNAL_TBD_OVERFLOW,
            ));
        output
            .add(2)
            .write(probe_set::<DiagDirection, SIG_GLOB_SIZE>(
                "_globset",
                crate::witness::SIGNAL_GLOBAL_OVERFLOW,
            ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    thread_local! {
        static ASSERTIONS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
        static COMPANY_READS: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn record_assertion(condition: bool) {
        ASSERTIONS.with(|a| a.borrow_mut().push(condition));
    }
    extern "C" fn invalid_company(_: u8) -> bool {
        COMPANY_READS.set(COMPANY_READS.get() + 1);
        false
    }
    fn item<Tdir>(tile: TileIndex, dir: Tdir) -> SSdata<Tdir> {
        SSdata { tile, dir }
    }

    #[test]
    fn empty_flush_and_queue_only_entries_leave_map_metadata_unused() {
        extern "C" fn check(condition: bool) {
            assert!(condition);
        }
        let _lock = LOCK.lock().unwrap();
        let mut shared = crate::test_tables::SHARED;
        shared.map = std::ptr::null();
        let mut native = crate::test_tables::SIGNALS;
        native.assertions = true;
        native.assert_condition = check;
        let context = Context {
            shared: &raw const shared,
            rail: &crate::test_tables::RAIL_MAP,
            native: &raw const native,
        };
        access(|s| {
            s.globset.reset();
            s.last_owner = Owner::INVALID;
        });
        context.update_signals_in_buffer();
        access(|s| {
            assert!(s.globset.is_empty());
            assert_eq!(s.last_owner, Owner::INVALID);
        });
        context.add_track_to_signal_buffer(TileIndex(1234), Track::TRACK_X, CompanyID(7));
        context.add_side_to_signal_buffer(TileIndex(4321), DiagDirection::DIAGDIR_NE, CompanyID(7));
        access(|s| {
            assert_eq!(s.globset.items(), 3);
            assert_eq!(s.last_owner, CompanyID(7));
            assert_eq!(
                s.globset.get(),
                Some(item(TileIndex(4321), DiagDirection::DIAGDIR_NE))
            );
            assert_eq!(
                s.globset.get(),
                Some(item(TileIndex(1234), DiagDirection::DIAGDIR_SW))
            );
            assert_eq!(
                s.globset.get(),
                Some(item(TileIndex(1234), DiagDirection::DIAGDIR_NE))
            );
            assert!(s.globset.is_empty());
            s.last_owner = Owner::INVALID;
        });
    }

    #[test]
    fn process_owner_is_shared_across_serialized_game_threads() {
        let _lock = LOCK.lock().unwrap();
        access(|s| {
            s.globset.reset();
            s.last_owner = CompanyID(7);
            s.globset.add(TileIndex(1234), DiagDirection::DIAGDIR_SW);
        });
        std::thread::spawn(|| {
            access(|s| {
                assert_eq!(s.last_owner, CompanyID(7));
                assert_eq!(
                    s.globset.get(),
                    Some(item(TileIndex(1234), DiagDirection::DIAGDIR_SW))
                );
            });
        })
        .join()
        .unwrap();
        access(|s| s.last_owner = Owner::INVALID);
    }

    /// `_enterdir_to_trackdirbits` as `signal.cpp` writes it, with the `TrackdirBits` values of
    /// `track_type.h`: `TRACKDIR_BIT_X_NE` 0x1, `Y_SE` 0x2, `UPPER_E` 0x4, `LOWER_E` 0x8,
    /// `LEFT_S` 0x10, `RIGHT_S` 0x20, `X_SW` 0x100, `Y_NW` 0x200, `UPPER_W` 0x400,
    /// `LOWER_W` 0x800, `LEFT_N` 0x1000 and `RIGHT_N` 0x2000.
    #[test]
    fn enterdir_to_trackdirbits_matches_original_table() {
        let expected = [
            0x100 | 0x400 | 0x20,   // X_SW | UPPER_W | RIGHT_S
            0x200 | 0x800 | 0x2000, // Y_NW | LOWER_W | RIGHT_N
            0x1 | 0x8 | 0x1000,     // X_NE | LOWER_E | LEFT_N
            0x2 | 0x4 | 0x10,       // Y_SE | UPPER_E | LEFT_S
        ];
        assert_eq!(ENTERDIR_TO_TRACKDIRBITS.map(|bits| bits.0), expected);
    }

    /// `SmallSet::Add` on a full set prints `Debug(misc, 0, "SignalSegment too complex. Set {}
    /// is full (maximum {})", name, items)` once, with that set's name and capacity.
    #[test]
    fn full_set_add_prints_name_and_capacity() {
        let _lock = LOCK.lock().unwrap();
        crate::debug::tests::take();
        reset_sets();
        access(|s| {
            for t in 0..SIG_TBU_SIZE {
                s.tbuset.add(TileIndex(t as u32), Trackdir::TRACKDIR_X_NE);
            }
            for t in 0..SIG_TBD_SIZE {
                s.tbdset.add(TileIndex(t as u32), DiagDirection::DIAGDIR_NE);
            }
            for t in 0..SIG_GLOB_SIZE {
                s.globset
                    .add(TileIndex(t as u32), DiagDirection::DIAGDIR_NE);
            }
        });
        assert!(!add_to_set(
            |s| &mut s.tbuset,
            TileIndex(0),
            Trackdir::TRACKDIR_X_NE
        ));
        assert!(!add_to_set(
            |s| &mut s.tbdset,
            TileIndex(0),
            DiagDirection::DIAGDIR_NE
        ));
        assert!(!add_to_set(
            |s| &mut s.globset,
            TileIndex(0),
            DiagDirection::DIAGDIR_NE
        ));
        access(|s| {
            assert!(s.tbuset.overflowed() && s.tbdset.overflowed() && s.globset.overflowed());
        });
        reset_sets();
        let line = |text: &str| ("misc".to_owned(), 0, text.as_bytes().to_vec());
        assert_eq!(
            crate::debug::tests::take(),
            [
                line("SignalSegment too complex. Set _tbuset is full (maximum 64)"),
                line("SignalSegment too complex. Set _tbdset is full (maximum 256)"),
                line("SignalSegment too complex. Set _globset is full (maximum 128)"),
            ]
        );
    }

    /// `assert(Company::IsValidID(owner))` reaches the native assertion once, and reads the
    /// company only where the native build asserts.
    #[test]
    fn company_assertion_is_evaluated_only_with_assertions() {
        let _lock = LOCK.lock().unwrap();
        let mut shared = crate::test_tables::SHARED;
        shared.map = std::ptr::null();
        shared.company_valid = invalid_company;
        let mut native = crate::test_tables::SIGNALS;
        native.assert_condition = record_assertion;
        reset_sets();
        for assertions in [true, false] {
            native.assertions = assertions;
            ASSERTIONS.with(|a| a.borrow_mut().clear());
            COMPANY_READS.set(0);
            let signals = Context {
                shared: &raw const shared,
                rail: &crate::test_tables::RAIL_MAP,
                native: &raw const native,
            }
            .signals()
            .update_signals_in_buffer_owner(CompanyID(3));
            assert_eq!(signals, SigSegState::SIGSEG_FREE);
            let expected: &[bool] = if assertions { &[false] } else { &[] };
            assert_eq!(ASSERTIONS.with(|a| a.borrow().clone()), expected);
            assert_eq!(COMPANY_READS.get(), u32::from(assertions));
        }
    }
}
