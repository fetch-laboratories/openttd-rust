/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Signal-block traversal and its process-wide fixed work sets. Occupancy and dirtying
//! callbacks can reenter.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::collapsible_if)]
use crate::debug::debug;
use crate::direction_type::DiagDirection;
use crate::game_state::GameState;
use crate::rail_map::is_pbs_signal as pbs;
use crate::tile_type::TileIndex;
use crate::tile_type::{MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE};

use crate::company_type::INVALID_OWNER;
use crate::direction_func::reverse_diag_dir_raw;
use crate::direction_type::INVALID_DIAGDIR;
use crate::direction_type::{
    Axis, DIAGDIR_BEGIN, DIAGDIR_END, DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW,
};
use crate::map_func::Map;
use crate::movement::native_assert;
use crate::services::{RailMapServices, Services};
use crate::signal_func::SigSegState;
use crate::signal_type::{SignalState, SignalType};
use crate::track_func::track_status_to_track_bits;
use crate::track_type::TrackBits;
use std::{cell::OnceCell, ffi::c_uint};

// Maximums used for updating signal blocks.
const SIG_TBU_SIZE: usize = 64; // Number of signals entering the block.
const SIG_TBD_SIZE: usize = 256; // Number of intersections: open nodes in the current block.
const SIG_GLOB_SIZE: usize = 128; // Number of open blocks; a block can be opened more than once until detected.
const SIG_GLOB_UPDATE: usize = 64; // Items in the global set that force an update.

// Incident track bits for the given entering direction.
const ENTERDIR_TO_TRACKBITS: [TrackBits; 4] = [
    TrackBits::TRACK_BIT_3WAY_NE,
    TrackBits::TRACK_BIT_3WAY_SE,
    TrackBits::TRACK_BIT_3WAY_SW,
    TrackBits::TRACK_BIT_3WAY_NW,
];
const SEARCH_DIR_1: [u8; 6] = [
    DIAGDIR_NE, DIAGDIR_SE, DIAGDIR_NE, DIAGDIR_SE, DIAGDIR_SW, DIAGDIR_SE,
];
const SEARCH_DIR_2: [u8; 6] = [
    DIAGDIR_SW, DIAGDIR_NW, DIAGDIR_NW, DIAGDIR_SW, DIAGDIR_NW, DIAGDIR_NE,
];
/// Current signal block state flags from `signal.cpp`; all raw byte values remain representable.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SigFlag(u8);
impl SigFlag {
    /// `SigFlag::Train`: train found in segment.
    const TRAIN: Self = Self(0);
    /// `SigFlag::Exit`: exit signal found.
    const EXIT: Self = Self(1);
    /// `SigFlag::MultiExit`: two or more exits found.
    const MULTI_EXIT: Self = Self(2);
    /// `SigFlag::Green`: green exit signal found.
    const GREEN: Self = Self(3);
    /// `SigFlag::MultiGreen`: two or more green exits found.
    const MULTI_GREEN: Self = Self(4);
    /// `SigFlag::Full`: some buffer was full; do not continue.
    const FULL: Self = Self(5);
    /// `SigFlag::Pbs`: PBS signal found.
    const PBS: Self = Self(6);
    /// `SigFlag::Split`: track merge or split found.
    const SPLIT: Self = Self(7);
    /// `SigFlag::Enter`: signal entering the block found.
    const ENTER: Self = Self(8);
    /// `SigFlag::MultiEnter`: two or more signals entering the block found.
    const MULTI_ENTER: Self = Self(9);
}
/// Original `EnumBitSet<SigFlag, uint16_t>`, including unnamed bits.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct SigFlags(u16);
impl SigFlags {
    const TRAIN: Self = Self(1 << SigFlag::TRAIN.0);
    const EXIT: Self = Self(1 << SigFlag::EXIT.0);
    const MULTI_EXIT: Self = Self(1 << SigFlag::MULTI_EXIT.0);
    const GREEN: Self = Self(1 << SigFlag::GREEN.0);
    const MULTI_GREEN: Self = Self(1 << SigFlag::MULTI_GREEN.0);
    const FULL: Self = Self(1 << SigFlag::FULL.0);
    const PBS: Self = Self(1 << SigFlag::PBS.0);
    const SPLIT: Self = Self(1 << SigFlag::SPLIT.0);
    const ENTER: Self = Self(1 << SigFlag::ENTER.0);
    const MULTI_ENTER: Self = Self(1 << SigFlag::MULTI_ENTER.0);
}
crate::base_bitset_type::impl_bitset_ops!(SigFlags);

/// Spatial occupancy, diagnostics and original Company assertion services.
#[repr(C)]
pub struct SignalServices {
    /// Native assertion build policy, independent of Cargo optimization.
    pub assertions: bool,
    /// Original work-set/direction/owner assertion predicate.
    pub assert_condition: extern "C" fn(bool),
    /// Original tunnel transport and entering-direction assertion sites.
    pub assert_wormhole_start: extern "C" fn(u32, u8),
    /// Direct Rust occupancy traversal, shared with the train controller.
    pub vehicles: *const crate::vehicle::VehicleServices,
    /// Original `Company::IsValidID` assertion, retaining native build policy.
    pub assert_company: extern "C" fn(u8),
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Item {
    tile: u32,
    dir: u8,
}
/// Set of tile/direction items. No tree structure is used because it would
/// cause slowdowns in the most usual cases (`signal.cpp::SmallSet`).
struct SmallSet<const N: usize> {
    count: usize,     // Actual number of units.
    overflowed: bool, // Did we try to overflow the set?
    data: [Item; N],
}
impl<const N: usize> SmallSet<N> {
    const fn new() -> Self {
        Self {
            count: 0,
            overflowed: false,
            data: [Item { tile: 0, dir: 0 }; N],
        }
    }
    /// Reset variables to default values.
    fn reset(&mut self) {
        self.count = 0;
        self.overflowed = false;
    }
    /// Try to remove the first instance of the given tile and direction.
    fn remove(&mut self, item: Item) -> bool {
        for i in 0..self.count {
            if self.data[i] == item {
                self.count -= 1;
                self.data[i] = self.data[self.count];
                return true;
            }
        }
        false
    }
    /// Find whether the tile/direction element is in the set (`IsIn`).
    fn is_in(&self, item: Item) -> bool {
        self.data[..self.count].contains(&item)
    }
    /// Add a tile/direction; set the overflow flag if the set was full.
    fn add(&mut self, item: Item) -> bool {
        if self.count == N {
            self.overflowed = true;
            return false;
        }
        self.data[self.count] = item;
        self.count += 1;
        true
    }
    /// Read the last added element; return no element if the set was empty.
    fn get(&mut self) -> Option<Item> {
        if self.count == 0 {
            return None;
        }
        self.count -= 1;
        Some(self.data[self.count])
    }
}
struct State {
    tbu: SmallSet<SIG_TBU_SIZE>,
    tbd: SmallSet<SIG_TBD_SIZE>,
    global: SmallSet<SIG_GLOB_SIZE>,
    last_owner: u8,
}
static STATE: GameState<State> = GameState::new(State {
    tbu: SmallSet::new(),
    tbd: SmallSet::new(),
    global: SmallSet::new(),
    last_owner: INVALID_OWNER,
});
fn access<T>(f: impl FnOnce(&mut State) -> T) -> T {
    // SAFETY: caller holds the native game-state mutex; closure never reenters.
    unsafe { f(&mut *STATE.get()) }
}
struct World<'a> {
    shared: &'a Services,
    rail: &'a RailMapServices,
    native: &'a SignalServices,
    map: OnceCell<Map>,
    log_x: u32,
}
macro_rules! read {
    ($w:expr, $group:ident, $field:ident $(, $arg:expr)*) => {
        crate::witness::counted!(before SIGNAL_SERVICES, SIGNAL_BYTES, ($w.$group.$field)($($arg),*))
    };
}
/// Signal facts consumed together; later fields are read only after their guards.
#[allow(clippy::struct_excessive_bools)] // The original reads four independent signal facts.
#[derive(Clone, Copy)]
struct RailSignalPresence {
    signals: bool,
    /// `TrackBitsToTrack` of the masked tracks, read only with signals.
    track: u8,
    /// `HasSignalOnTrack` for that track.
    on_track: bool,
    kind: SignalType,
    forward: bool,
    reverse: bool,
}

#[derive(Clone, Copy)]
struct RailSignalUpdate {
    kind: SignalType,
    reservations: TrackBits,
}

#[derive(Clone, Copy)]
struct RailCrossingAdmission {
    crossing: bool,
    owner: u8,
    axis: Axis,
}
impl World<'_> {
    fn map(&self) -> Map {
        *self.map.get_or_init(|| {
            let mut map = self.shared.map();
            map.log_x = self.log_x;
            map
        })
    }
    /// The canonical map view, loaded at each observation.
    fn view(&self) -> crate::map_func::World<'_> {
        crate::map_func::World::new(self.shared)
    }
}
/// `UpdateSignalsAroundSegment` signal facts: `HasSignals`, then `GetPresentSignals`
/// with `SignalOnTrack`, then `GetSignalType` and both `SignalAlongTrackdir` tests.
fn signal_presence(w: &World<'_>, tile: u32, masked_tracks: u8, td: u8) -> RailSignalPresence {
    let view = w.view();
    let mut r = RailSignalPresence {
        signals: view.has_signals(tile),
        track: crate::track_type::INVALID_TRACK,
        on_track: false,
        kind: SignalType::SIGTYPE_BLOCK,
        forward: false,
        reverse: false,
    };
    if r.signals {
        r.track = crate::track_func::track_bits_to_track(masked_tracks, |ok| view.check(ok));
        let present = view.present_signals(tile);
        r.on_track = present & crate::signal_func::signal_on_track(r.track) != 0;
        if r.on_track {
            r.kind = SignalType(view.signal_type(tile, r.track));
            r.forward = present & crate::signal_func::signal_along_trackdir(td) != 0;
            r.reverse = present
                & crate::signal_func::signal_along_trackdir(crate::track_func::reverse_trackdir(
                    td,
                ))
                != 0;
        }
    }
    r
}
/// Signal update facts: the signal type, then reservations only for path signals.
fn signal_update(w: &World<'_>, tile: u32, td: u8) -> RailSignalUpdate {
    let view = w.view();
    view.check(view.has_signal_on_trackdir(tile, td));
    let kind = view.signal_type(tile, crate::track_func::trackdir_to_track(td));
    let mut r = RailSignalUpdate {
        kind: SignalType(kind),
        reservations: TrackBits::TRACK_BIT_NONE,
    };
    if pbs(kind) {
        r.reservations = read!(w, rail, rail_reservation, tile);
    }
    r
}
/// Level-crossing admission: `IsLevelCrossing`, then the owner, then the road axis.
fn crossing_admission(w: &World<'_>, tile: u32, owner: u8) -> RailCrossingAdmission {
    let view = w.view();
    let mut r = RailCrossingAdmission {
        crossing: view.level_crossing(tile),
        owner: INVALID_OWNER,
        axis: Axis(crate::direction_type::AXIS_X),
    };
    if r.crossing {
        r.owner = view.owner(tile);
        if r.owner == owner {
            r.axis = Axis(view.crossing_road_axis(tile));
        }
    }
    r
}
fn overflow_tbu() {
    debug!(
        misc,
        0, "SignalSegment too complex. Set _tbuset is full (maximum 64)"
    );
}
fn overflow_tbd() {
    debug!(
        misc,
        0, "SignalSegment too complex. Set _tbdset is full (maximum 256)"
    );
}
fn overflow_global() {
    debug!(
        misc,
        0, "SignalSegment too complex. Set _globset is full (maximum 128)"
    );
}
fn add_tbu(item: Item) -> bool {
    let added = access(|s| s.tbu.add(item));
    if !added {
        overflow_tbu();
        crate::witness::hit(crate::witness::SIGNAL_TBU_OVERFLOW);
    }
    added
}
fn add_tbd(item: Item) -> bool {
    let added = access(|s| s.tbd.add(item));
    if !added {
        overflow_tbd();
        crate::witness::hit(crate::witness::SIGNAL_TBD_OVERFLOW);
    }
    added
}
fn add_global(item: Item) -> bool {
    let added = access(|s| s.global.add(item));
    if !added {
        overflow_global();
        crate::witness::hit(crate::witness::SIGNAL_GLOBAL_OVERFLOW);
    }
    added
}
fn source_assert(w: &World<'_>, condition: bool) {
    if w.native.assertions {
        read!(w, native, assert_condition, condition);
    }
}
/// Remove the new and reverse directions from the global set: they need not
/// be checked again. Remove the reverse direction from the todo set too. This
/// is the core of avoiding a second visit to a tile. Return false only if adding
/// the new direction would overrun the todo buffer (`MaybeAddToTodoSet` and
/// its in-place `CheckAddToTodoSet` operations).
fn maybe_add_to_todo_set(w: &World<'_>, next: Item, previous: Item) -> bool {
    access(|s| {
        s.global.remove(next); // It can be in Global but not in Todo.
        s.global.remove(previous); // Remove in all cases.
    });
    source_assert(w, access(|s| !s.tbd.is_in(next)));
    let add = access(|s| !s.tbd.remove(previous));
    !add || add_tbd(next)
}
// The unfiltered branch is IsTrainAndNotInDepot: a train on rail, outside a
// depot. The filtered branch retains EnsureNoTrainOnTrackBits occupancy policy.
fn occupied(w: &World<'_>, tile: u32, bits: Option<u8>) -> bool {
    // SAFETY: Static services and live shells; signal scans do not delete vehicles.
    let vs = unsafe { &*w.native.vehicles };
    vs.any_on_tile(tile, |v| {
        if v.vehicle_type() != crate::vehicle_type::VehicleType::TRAIN.0 {
            return false;
        }
        crate::witness::hit(crate::witness::SIGNAL_OCCUPANCY_ACCESSORS);
        let track = (vs.train_handle)(v.0.as_ptr()).track();
        bits.map_or(track != crate::track_type::TRACK_BIT_DEPOT, |bits| {
            track == bits || crate::track_func::tracks_overlap(track | bits)
        })
    })
}
fn train_on_tile(w: &World<'_>, tile: u32, flags: &mut SigFlags) {
    if *flags & SigFlags::TRAIN == SigFlags(0) && occupied(w, tile, None) {
        *flags |= SigFlags::TRAIN;
        crate::witness::hit(crate::witness::SIGNAL_TRAIN);
    }
}
/// Search the signal block belonging to the owner (`ExploreSegment`).
#[allow(clippy::too_many_lines)]
fn explore_segment(w: &World<'_>, owner: u8) -> SigFlags {
    let mut flags = SigFlags(0);
    while let Some(Item { mut tile, mut dir }) = access(|s| s.tbd.get()) {
        crate::witness::hit(crate::witness::SIGNAL_TILES);
        let old_tile = tile; // Tile we are leaving.
        // Expected new exit direction for a straight line.
        let mut exitdir = if dir == INVALID_DIAGDIR {
            INVALID_DIAGDIR
        } else {
            reverse_diag_dir_raw(dir, |ok| native_assert(w.shared, ok))
        };
        match read!(w, shared, tile_type, tile) {
            MP_RAILWAY => {
                // Do not propagate signals on others' tiles (remove for track sharing).
                if read!(w, shared, tile_owner, tile) != owner {
                    crate::witness::hit(crate::witness::SIGNAL_OWNER_BARRIER);
                    continue;
                }
                let geometry = read!(w, rail, traversal, tile);
                if geometry.depot {
                    if dir == INVALID_DIAGDIR {
                        // From inside: a train just entered or left the depot.
                        crate::witness::hit(crate::witness::SIGNAL_DEPOT);
                        train_on_tile(w, tile, &mut flags);
                        exitdir = w.view().rail_depot_dir(tile);
                        tile = (TileIndex(tile)
                            + w.map().tile_offs_by_diag_dir(
                                DiagDirection(exitdir),
                                crate::map_func::native_check,
                            ))
                        .0;
                        dir = reverse_diag_dir_raw(exitdir, |ok| native_assert(w.shared, ok));
                    } else if dir == w.view().rail_depot_dir(tile) {
                        // Entered a depot.
                        train_on_tile(w, tile, &mut flags);
                        continue;
                    } else {
                        continue;
                    }
                } else {
                    source_assert(w, dir < DIAGDIR_END);
                    let mut tracks = geometry.tracks; // Track bits of the tile.
                    let masked = tracks & ENTERDIR_TO_TRACKBITS[usize::from(dir)]; // Only incident track bits.
                    // There is exactly one incident track, no need to check.
                    if tracks == TrackBits::TRACK_BIT_HORZ || tracks == TrackBits::TRACK_BIT_VERT {
                        tracks = masked;
                        // If no train was detected yet and the track is not clear, set the train flag.
                        if flags & SigFlags::TRAIN == SigFlags(0)
                            && occupied(w, tile, Some(tracks.0))
                        {
                            flags |= SigFlags::TRAIN;
                            crate::witness::hit(crate::witness::SIGNAL_TRAIN);
                        }
                    } else {
                        if masked == TrackBits::TRACK_BIT_NONE {
                            // No incident track.
                            continue;
                        }
                        train_on_tile(w, tile, &mut flags);
                    }
                    // Is this a track merge or split?
                    if tracks.0.count_ones() > 1 {
                        flags |= SigFlags::SPLIT;
                        crate::witness::hit(crate::witness::SIGNAL_SPLIT);
                    }
                    let td = ((u32::from(tracks.0) * 0x101)
                        & u32::from(crate::track_func::diagdir_reaches_trackdirs(
                            reverse_diag_dir_raw(dir, |ok| native_assert(w.shared, ok)),
                        )))
                    .trailing_zeros() as u8;
                    let signal = signal_presence(w, tile, masked.0, td);
                    if signal.signals {
                        // Exactly one track, not zero: there is an exit from this tile.
                        if signal.on_track {
                            // Check the whole track, rather than only the trackdir.
                            let kind = signal.kind;
                            let back = crate::track_func::reverse_trackdir(td);
                            // Add the reverse direction to the to-be-updated set for any
                            // conventional signal. A changed pre-signal exit will later add
                            // its block to the to-be-done set.
                            if signal.reverse {
                                if pbs(kind.0) {
                                    flags |= SigFlags::PBS;
                                    crate::witness::hit(crate::witness::SIGNAL_PBS);
                                }
                                if flags & SigFlags::ENTER != SigFlags(0) {
                                    flags |= SigFlags::MULTI_ENTER;
                                    crate::witness::hit(crate::witness::SIGNAL_MULTI_ENTER);
                                }
                                flags |= SigFlags::ENTER;
                                if !add_tbu(Item { tile, dir: back }) {
                                    return flags | SigFlags::FULL;
                                }
                            }
                            if signal.forward && kind == SignalType::SIGTYPE_PBS {
                                flags |= SigFlags::PBS;
                            }
                            // Check a pre-signal exit in our direction until two green exits are found.
                            if flags & SigFlags::MULTI_GREEN == SigFlags(0)
                                && w.view().is_presignal_exit(tile, signal.track)
                                && signal.forward
                            {
                                if flags & SigFlags::EXIT != SigFlags(0) {
                                    flags |= SigFlags::MULTI_EXIT;
                                    crate::witness::hit(crate::witness::SIGNAL_MULTI_EXIT);
                                }
                                flags |= SigFlags::EXIT;
                                crate::witness::hit(crate::witness::SIGNAL_EXIT);
                                if w.view().signal_state_by_trackdir(tile, td)
                                    == SignalState::SIGNAL_STATE_GREEN.0
                                {
                                    if flags & SigFlags::GREEN != SigFlags(0) {
                                        flags |= SigFlags::MULTI_GREEN;
                                        crate::witness::hit(crate::witness::SIGNAL_MULTI_GREEN);
                                    }
                                    flags |= SigFlags::GREEN;
                                    crate::witness::hit(crate::witness::SIGNAL_GREEN_EXIT);
                                }
                            }
                            continue;
                        }
                    }
                    for exit in DIAGDIR_BEGIN..DIAGDIR_END {
                        // Test all possible exit directions.
                        if exit != dir
                            && (tracks & ENTERDIR_TO_TRACKBITS[usize::from(exit)])
                                != TrackBits::TRACK_BIT_NONE
                        {
                            let next = Item {
                                tile: (TileIndex(tile)
                                    + w.map().tile_offs_by_diag_dir(
                                        DiagDirection(exit),
                                        crate::map_func::native_check,
                                    ))
                                .0,
                                dir: reverse_diag_dir_raw(exit, |ok| native_assert(w.shared, ok)),
                            };
                            if !maybe_add_to_todo_set(w, next, Item { tile, dir: exit }) {
                                return flags | SigFlags::FULL;
                            }
                        }
                    }
                    continue;
                }
            }
            MP_STATION => {
                // Reject a different axis or a blocked (eye-candy) station tile.
                let station = read!(w, rail, station_admission, tile, owner, dir);
                if !station.rail
                    || station.owner != owner
                    || (dir & 1) != station.axis.0
                    || station.blocked
                {
                    continue;
                }
                crate::witness::hit(crate::witness::SIGNAL_STATION);
                train_on_tile(w, tile, &mut flags);
                tile = (TileIndex(tile)
                    + w.map().tile_offs_by_diag_dir(
                        DiagDirection(exitdir),
                        crate::map_func::native_check,
                    ))
                .0;
            }
            MP_ROAD => {
                // The crossing road axis must differ from our entering axis.
                let crossing = crossing_admission(w, tile, owner);
                if !crossing.crossing || crossing.owner != owner || (dir & 1) == crossing.axis.0 {
                    continue;
                }
                crate::witness::hit(crate::witness::SIGNAL_CROSSING);
                train_on_tile(w, tile, &mut flags);
                tile = (TileIndex(tile)
                    + w.map().tile_offs_by_diag_dir(
                        DiagDirection(exitdir),
                        crate::map_func::native_check,
                    ))
                .0;
            }
            MP_TUNNELBRIDGE => {
                let wormhole = read!(w, rail, wormhole_admission, tile, owner);
                if wormhole.owner != owner
                    || wormhole.transport != crate::transport_type::TransportType::TRANSPORT_RAIL
                {
                    continue;
                }
                crate::witness::hit(crate::witness::SIGNAL_WORMHOLE);
                let direction = wormhole.direction.0;
                if dir == INVALID_DIAGDIR {
                    // Incoming from the wormhole.
                    train_on_tile(w, tile, &mut flags);
                    dir = direction;
                    exitdir = reverse_diag_dir_raw(direction, |ok| native_assert(w.shared, ok));
                    tile = (TileIndex(tile)
                        + w.map().tile_offs_by_diag_dir(
                            DiagDirection(exitdir),
                            crate::map_func::native_check,
                        ))
                    .0;
                } else {
                    if reverse_diag_dir_raw(dir, |ok| native_assert(w.shared, ok)) != direction {
                        continue;
                    }
                    train_on_tile(w, tile, &mut flags);
                    tile = read!(w, rail, other_tunnel_bridge_end, tile); // Skip to the exit tile.
                    dir = INVALID_DIAGDIR;
                    exitdir = INVALID_DIAGDIR;
                }
            }
            _ => continue,
        }
        if !maybe_add_to_todo_set(
            w,
            Item { tile, dir },
            Item {
                tile: old_tile,
                dir: exitdir,
            },
        ) {
            return flags | SigFlags::FULL;
        }
    }
    flags
}
/// Update signals around the segment in the to-be-updated set.
fn update_signals_around_segment(w: &World<'_>, flags: SigFlags) {
    while let Some(Item { tile, dir: td }) = access(|s| s.tbu.get()) {
        let track = crate::track_func::trackdir_to_track(td);
        let observation = signal_update(w, tile, td);
        let kind = observation.kind;
        let mut new_state = SignalState::SIGNAL_STATE_GREEN;
        // The reserve/unreserve process handles the state of reserved path signals.
        if pbs(kind.0) && observation.reservations.0 & (1 << track) != 0 {
            crate::witness::hit(crate::witness::SIGNAL_RESERVED_SKIP);
            continue;
        }
        // Turn red for a train, or a path signal with a junction or multiple ways in.
        if flags & SigFlags::TRAIN != SigFlags(0)
            || (pbs(kind.0) && flags & (SigFlags::SPLIT | SigFlags::MULTI_ENTER) != SigFlags(0))
        {
            new_state = SignalState::SIGNAL_STATE_RED;
            // For a bidirectional combo, do not count the other direction as an exit.
        } else if kind == SignalType::SIGTYPE_COMBO
            && w.view()
                .has_signal_on_trackdir(tile, crate::track_func::reverse_trackdir(td))
        {
            crate::witness::hit(crate::witness::SIGNAL_COMBO);
            // At least one more exit, with no green exit, or this is the only green exit.
            if flags & SigFlags::MULTI_EXIT != SigFlags(0)
                && (flags & SigFlags::GREEN == SigFlags(0)
                    || (flags & SigFlags::MULTI_GREEN == SigFlags(0)
                        && w.view().signal_state_by_trackdir(
                            tile,
                            crate::track_func::reverse_trackdir(td),
                        ) == SignalState::SIGNAL_STATE_GREEN.0))
            {
                new_state = SignalState::SIGNAL_STATE_RED;
                crate::witness::hit(crate::witness::SIGNAL_COMBO_RED);
            }
            // Entry signal, at least one exit, no green exit.
        } else if w.view().is_presignal_entry(tile, track)
            && flags & SigFlags::EXIT != SigFlags(0)
            && flags & SigFlags::GREEN == SigFlags(0)
        {
            new_state = SignalState::SIGNAL_STATE_RED;
        }
        // Only when the state changes.
        if new_state != SignalState(w.view().signal_state_by_trackdir(tile, td)) {
            // For pre-signal exits, add the block globally without checking for a
            // full global set: update all signals first.
            if w.view().is_presignal_exit(tile, track) {
                add_global(Item {
                    tile,
                    dir: crate::track_func::trackdir_to_exitdir(
                        crate::track_func::reverse_trackdir(td),
                    ),
                });
                crate::witness::hit(crate::witness::SIGNAL_PROPAGATE);
            }
            read!(w, rail, set_signal_state, tile, td, new_state.0);
            read!(w, shared, mark_dirty, tile);
            crate::witness::hit(crate::witness::SIGNAL_CHANGED);
        }
    }
}
/// Reset all sets after one set overflowed.
fn reset_sets() {
    access(|s| {
        s.tbu.reset();
        s.tbd.reset();
        s.global.reset();
    });
}
/// Update global buffered blocks and return the state of the first block.
fn update_signals_in_buffer(w: &World<'_>, owner: u8) -> SigSegState {
    read!(w, native, assert_company, owner);
    crate::witness::hit(crate::witness::SIGNAL_UPDATES);
    let mut first = true;
    let mut state = SigSegState::SIGSEG_FREE; // Value to return.
    while let Some(Item { mut tile, mut dir }) = access(|s| s.global.get()) {
        source_assert(w, access(|s| s.tbu.count == 0));
        source_assert(w, access(|s| s.tbd.count == 0));
        // Signal updates leave railway tiles with signals here. Other entries
        // come from rail construction/removal, trains entering/leaving a block,
        // or a train leaving a depot.
        let kind = read!(w, shared, tile_type, tile);
        if kind == MP_TUNNELBRIDGE {
            // Optimization assertions: do not update signals when unnecessary.
            if w.native.assertions {
                read!(w, native, assert_wormhole_start, tile, dir);
            }
            add_tbd(Item {
                tile,
                dir: INVALID_DIAGDIR,
            });
            // Safely start from the wormhole centre at both ends.
            let other = read!(w, rail, other_tunnel_bridge_end, tile);
            add_tbd(Item {
                tile: other,
                dir: INVALID_DIAGDIR,
            });
        } else if kind == MP_RAILWAY && w.view().is_rail_depot(tile) {
            if w.native.assertions {
                source_assert(
                    w,
                    dir == INVALID_DIAGDIR || dir == w.view().rail_depot_dir(tile),
                );
            }
            add_tbd(Item {
                tile,
                dir: INVALID_DIAGDIR,
            });
        } else if (kind == MP_RAILWAY || kind == MP_STATION || kind == MP_ROAD)
            && (track_status_to_track_bits(crate::movement::track_status(
                w.shared,
                tile,
                crate::transport_type::TRANSPORT_RAIL,
                0,
                INVALID_DIAGDIR,
            )) & ENTERDIR_TO_TRACKBITS[usize::from(dir)].0)
                != 0
        {
            add_tbd(Item { tile, dir });
            add_tbd(Item {
                tile: (TileIndex(tile)
                    + w.map()
                        .tile_offs_by_diag_dir(DiagDirection(dir), crate::map_func::native_check))
                .0,
                dir: reverse_diag_dir_raw(dir, |ok| native_assert(w.shared, ok)),
            });
        } else {
            tile = (TileIndex(tile)
                + w.map()
                    .tile_offs_by_diag_dir(DiagDirection(dir), crate::map_func::native_check))
            .0;
            dir = reverse_diag_dir_raw(dir, |ok| native_assert(w.shared, ok));
            if (track_status_to_track_bits(crate::movement::track_status(
                w.shared,
                tile,
                crate::transport_type::TRANSPORT_RAIL,
                0,
                INVALID_DIAGDIR,
            )) & ENTERDIR_TO_TRACKBITS[usize::from(dir)].0)
                == 0
            {
                // Removing a rail unconnected at one or both ends reaches here.
                crate::witness::hit(crate::witness::SIGNAL_NEIGHBOUR_CONTINUE);
                continue;
            }
            add_tbd(Item { tile, dir });
        }
        // These one or two items should neither overflow nor leave the set empty.
        source_assert(w, access(|s| !s.tbd.overflowed));
        source_assert(w, access(|s| s.tbd.count != 0));
        let flags = explore_segment(w, owner);
        if first {
            first = false;
            if flags & SigFlags::PBS != SigFlags(0) {
                state = SigSegState::SIGSEG_PBS;
            } else if flags & SigFlags::TRAIN != SigFlags(0)
                || (flags & SigFlags::EXIT != SigFlags(0) && flags & SigFlags::GREEN == SigFlags(0))
                || flags & SigFlags::FULL != SigFlags(0)
            {
                state = SigSegState::SIGSEG_FULL;
            }
        }
        // Do nothing more when any buffer was full.
        if flags & SigFlags::FULL != SigFlags(0) {
            reset_sets();
            crate::witness::hit(crate::witness::SIGNAL_RESET);
            break;
        }
        update_signals_around_segment(w, flags);
    }
    state
}
/// Add a track to the signal update buffer at both ends.
fn add_track_to_signal_buffer(w: &World<'_>, tile: u32, track: u8, owner: u8) {
    // Do not allow signal updates for two companies in one run.
    source_assert(w, access(|s| s.global.count == 0 || owner == s.last_owner));
    access(|s| s.last_owner = owner);
    add_global(Item {
        tile,
        dir: SEARCH_DIR_1[usize::from(track)],
    });
    add_global(Item {
        tile,
        dir: SEARCH_DIR_2[usize::from(track)],
    });
    force_flush(w);
}
fn force_flush(w: &World<'_>) {
    if access(|s| s.global.count >= SIG_GLOB_UPDATE) {
        // Too many items: force an update.
        crate::witness::hit(crate::witness::SIGNAL_FORCED_FLUSH);
        update_signals_in_buffer(w, access(|s| s.last_owner));
        access(|s| s.last_owner = INVALID_OWNER);
    }
}
/// Update signals in the buffer, called from outside (`UpdateSignalsInBuffer()`).
fn flush_signal_buffer(w: &World<'_>) {
    if access(|s| s.global.count != 0) {
        update_signals_in_buffer(w, access(|s| s.last_owner));
        access(|s| s.last_owner = INVALID_OWNER);
    }
}
/// Add one side of a tile to the signal update buffer.
fn add_side_to_signal_buffer(w: &World<'_>, tile: u32, side: u8, owner: u8) {
    // Do not allow signal updates for two companies in one run.
    source_assert(w, access(|s| s.global.count == 0 || owner == s.last_owner));
    access(|s| s.last_owner = owner);
    add_global(Item { tile, dir: side });
    force_flush(w);
}
/// Update from one side of a tile, checking the neighbouring opposite side too.
fn update_signals_on_segment(w: &World<'_>, tile: u32, side: u8, owner: u8) -> SigSegState {
    source_assert(w, access(|s| s.global.count == 0));
    add_global(Item { tile, dir: side });
    update_signals_in_buffer(w, owner)
}
/// Update segments at both ends of an existent or non-existent track.
fn set_signals_on_both_dir(w: &World<'_>, tile: u32, track: u8, owner: u8) {
    source_assert(w, access(|s| s.global.count == 0));
    add_track_to_signal_buffer(w, tile, track, owner);
    update_signals_in_buffer(w, owner);
}
// SAFETY for every exported entry: immutable tables outlive the synchronous
// invocation. Game-state mutex is held; callbacks do not retain any Rust owner
// borrow and cannot throw ordinary-play exceptions. No entry draws Random.
unsafe fn world<'a>(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    log_x: c_uint,
) -> World<'a> {
    // SAFETY: caller supplies valid immutable tables for the invocation.
    unsafe {
        World {
            shared: &*shared,
            rail: &*rail,
            native: &*native,
            map: OnceCell::new(),
            log_x,
        }
    }
}
/// Flush queued blocks and invalidate last owner only when nonempty.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_flush(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    log_x: c_uint,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    let w = unsafe { world(shared, rail, native, log_x) };
    flush_signal_buffer(&w);
}
/// Queue both ends of a track and preserve the 64-entry forced flush.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_add_track(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    log_x: c_uint,
    tile: u32,
    track: u8,
    owner: u8,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    let w = unsafe { world(shared, rail, native, log_x) };
    add_track_to_signal_buffer(&w, tile, track, owner);
}
/// Queue one side with the original owner lifetime and forced flush.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_add_side(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    log_x: c_uint,
    tile: u32,
    side: u8,
    owner: u8,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    let w = unsafe { world(shared, rail, native, log_x) };
    add_side_to_signal_buffer(&w, tile, side, owner);
}
/// Update a segment, returning the classification of the first explored block.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_segment(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    log_x: c_uint,
    tile: u32,
    side: u8,
    owner: u8,
) -> u8 {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    let w = unsafe { world(shared, rail, native, log_x) };
    let state = update_signals_on_segment(&w, tile, side, owner);
    crate::witness::hit(crate::witness::SIGNAL_CLASSIFICATION_BYTES);
    state.0
}
/// Update both track ends with the original private flush owner lifetime.
/// # Safety
/// Immutable tables are valid for the call; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_both(
    shared: *const Services,
    rail: *const RailMapServices,
    native: *const SignalServices,
    log_x: c_uint,
    tile: u32,
    track: u8,
    owner: u8,
) {
    crate::witness::hit(crate::witness::SIGNAL_ENTRIES);
    let w = unsafe { world(shared, rail, native, log_x) };
    set_signals_on_both_dir(&w, tile, track, owner);
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
/// Inspect pending sets without altering their ordering, counts or lifetime.
/// # Safety
/// `output` is writable for one Snapshot; caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_signal_inspect(output: *mut Snapshot) {
    // SAFETY: caller owns this output, which does not alias STATE.
    let out = unsafe { &mut *output };
    access(|s| {
        out.tbu_count = s.tbu.count as c_uint;
        out.tbd_count = s.tbd.count as c_uint;
        out.global_count = s.global.count as c_uint;
        out.overflow = [s.tbu.overflowed, s.tbd.overflowed, s.global.overflowed];
        out.last_owner = s.last_owner;
        for (dest, source) in [
            (&mut out.tbu[..], &s.tbu.data[..s.tbu.count]),
            (&mut out.tbd[..], &s.tbd.data[..s.tbd.count]),
            (&mut out.global[..], &s.global.data[..s.global.count]),
        ] {
            dest.fill(SignalItem::default());
            for (dest, source) in dest.iter_mut().zip(source) {
                *dest = SignalItem {
                    tile: source.tile,
                    dir: source.dir,
                };
            }
        }
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
        assert_wormhole_start,
        vehicles,
        assert_company,
    };
    "OpenTTDSignalContext" ["probe"] => Context {
        shared,
        rail,
        native,
        log_x,
    } width log_x: std::ffi::c_uint;
}

/// Call-scoped bridge to a direct Rust signal entry from other Rust owners.
/// All pointers address immutable process-lifetime native service tables; map
/// width is captured for this invocation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Context {
    /// Existing canonical shared map operations.
    pub shared: *const Services,
    /// Canonical rail-map observations.
    pub rail: *const RailMapServices,
    /// Occupancy, diagnostics and source assertions.
    pub native: *const SignalServices,
    /// Current `Map::LogX`.
    pub log_x: c_uint,
}
impl Context {
    fn world(&self) -> World<'_> {
        // SAFETY: native acquisition supplied immutable process-lifetime tables;
        // the caller holds the game-state mutex throughout this synchronous call.
        unsafe { world(self.shared, self.rail, self.native, self.log_x) }
    }
    pub(crate) fn segment(self, tile: u32, side: u8, owner: u8) -> u8 {
        update_signals_on_segment(&self.world(), tile, side, owner).0
    }
    pub(crate) fn both(self, tile: u32, track: u8, owner: u8) {
        set_signals_on_both_dir(&self.world(), tile, track, owner);
    }
    pub(crate) fn add_track(self, tile: u32, track: u8, owner: u8) {
        add_track_to_signal_buffer(&self.world(), tile, track, owner);
    }
    pub(crate) fn add_side(self, tile: u32, side: u8, owner: u8) {
        add_side_to_signal_buffer(&self.world(), tile, side, owner);
    }
    pub(crate) fn flush(self) {
        flush_signal_buffer(&self.world());
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
impl From<Item> for SignalItem {
    fn from(item: Item) -> Self {
        Self {
            tile: item.tile,
            dir: item.dir,
        }
    }
}
fn probe_set<const N: usize>(overflow: fn()) -> SetTrace {
    let mut set = SmallSet::<N>::new();
    let a = Item { tile: 1, dir: 0 };
    for item in [
        a,
        Item {
            tile: 2,
            dir: INVALID_DIAGDIR,
        },
        a,
        Item { tile: 1, dir: 2 },
    ] {
        set.add(item);
    }
    set.remove(a);
    let duplicate_count = set.count as c_uint;
    let duplicate_pop = std::array::from_fn(|_| SignalItem::from(set.get().unwrap()));
    set.reset();
    for tile in 0..N {
        set.add(Item {
            tile: tile as u32,
            dir: (tile & 3) as u8,
        });
    }
    let full_failed = !set.add(Item {
        tile: u32::MAX,
        dir: INVALID_DIAGDIR,
    });
    if full_failed {
        overflow();
    }
    let overflow_full = set.overflowed;
    set.remove(Item { tile: 0, dir: 0 });
    let first_after_remove = set.data[0].into();
    let mut refill_succeeded = set.add(Item {
        tile: u32::MAX,
        dir: INVALID_DIAGDIR,
    });
    let refill_pop = set.get().unwrap().into();
    while set.get().is_some() {}
    let drained_count = set.count as c_uint;
    let overflow_drained = set.overflowed;
    refill_succeeded &= set.add(Item {
        tile: u32::MAX,
        dir: INVALID_DIAGDIR,
    });
    set.reset();
    refill_succeeded &= set.count == 0;
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
        overflow_reset: set.overflowed,
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
        output.write(probe_set::<64>(overflow_tbu));
        output.add(1).write(probe_set::<256>(overflow_tbd));
        output.add(2).write(probe_set::<128>(overflow_global));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
            log_x: 6,
        };
        access(|s| {
            s.global.reset();
            s.last_owner = INVALID_OWNER;
        });
        context.flush();
        access(|s| {
            assert_eq!(s.global.count, 0);
            assert_eq!(s.last_owner, INVALID_OWNER);
        });
        context.add_track(1234, crate::track_type::TRACK_X, 7);
        context.add_side(4321, DIAGDIR_NE, 7);
        access(|s| {
            assert_eq!(s.global.count, 3);
            assert_eq!(s.last_owner, 7);
            assert_eq!(
                s.global.get(),
                Some(Item {
                    tile: 4321,
                    dir: DIAGDIR_NE
                })
            );
            assert_eq!(
                s.global.get(),
                Some(Item {
                    tile: 1234,
                    dir: DIAGDIR_SW
                })
            );
            assert_eq!(
                s.global.get(),
                Some(Item {
                    tile: 1234,
                    dir: DIAGDIR_NE
                })
            );
            assert_eq!(s.global.count, 0);
            s.last_owner = INVALID_OWNER;
        });
    }

    #[test]
    fn process_owner_is_shared_across_serialized_game_threads() {
        let _lock = LOCK.lock().unwrap();
        access(|s| {
            s.global.reset();
            s.last_owner = 7;
            s.global.add(Item { tile: 1234, dir: 2 });
        });
        std::thread::spawn(|| {
            access(|s| {
                assert_eq!(s.last_owner, 7);
                assert_eq!(s.global.get(), Some(Item { tile: 1234, dir: 2 }));
            });
        })
        .join()
        .unwrap();
        access(|s| s.last_owner = INVALID_OWNER);
    }
}
