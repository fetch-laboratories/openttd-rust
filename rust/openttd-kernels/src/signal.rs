/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete signal-block traversal and canonical process-wide fixed work sets.
//! The native game-state mutex serializes entries on main/game threads. Every
//! owner access ends before a native callback, including occupancy and dirtying.
//! Native leaves are noexcept; environmental failures terminate, panics abort.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::collapsible_if)]
use crate::services::{Map, RailMapServices, Services};
use std::cell::UnsafeCell;
use std::ffi::c_uint;

const INVALID: u8 = 255;
const ENTER_TRACKS: [u8; 4] = [0x25, 0x2a, 0x19, 0x16];
const ENTER_TRACKDIRS: [u16; 4] = [0x0520, 0x2a00, 0x1009, 0x0016];
const SEARCH_ONE: [u8; 6] = [0, 1, 0, 1, 2, 1];
const SEARCH_TWO: [u8; 6] = [2, 3, 3, 2, 3, 0];
const TRACKDIR_EXIT: [u8; 14] = [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0];
const TRAIN: u16 = 1 << 0;
const EXIT: u16 = 1 << 1;
const MULTI_EXIT: u16 = 1 << 2;
const GREEN: u16 = 1 << 3;
const MULTI_GREEN: u16 = 1 << 4;
const FULL: u16 = 1 << 5;
const PBS: u16 = 1 << 6;
const SPLIT: u16 = 1 << 7;
const ENTER: u16 = 1 << 8;
const MULTI_ENTER: u16 = 1 << 9;

/// Spatial occupancy, diagnostics and original Company assertion services.
#[repr(C)]
pub struct SignalServices {
    /// Native assertion build policy, independent of Cargo optimization.
    pub assertions: bool,
    /// Original work-set/direction/owner assertion predicate.
    pub assert_condition: extern "C" fn(bool),
    /// Original tunnel transport and entering-direction assertion sites.
    pub assert_wormhole_start: extern "C" fn(u32, u8),
    /// Original `HasVehicleOnTile(IsTrainAndNotInDepot)` spatial query.
    pub train_on_tile: extern "C" fn(u32) -> bool,
    /// Original `EnsureNoTrainOnTrackBits(...).Failed()` spatial query.
    pub train_on_tracks: extern "C" fn(u32, u8) -> bool,
    /// Original to-be-updated set overflow log.
    pub overflow_tbu: extern "C" fn(),
    /// Original to-be-done set overflow log.
    pub overflow_tbd: extern "C" fn(),
    /// Original global set overflow log.
    pub overflow_global: extern "C" fn(),
    /// Original `Company::IsValidID` assertion, retaining native build policy.
    pub assert_company: extern "C" fn(u8),
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Item {
    tile: u32,
    dir: u8,
}
struct SmallSet<const N: usize> {
    count: usize,
    overflowed: bool,
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
    fn reset(&mut self) {
        self.count = 0;
        self.overflowed = false;
    }
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
    fn contains(&self, item: Item) -> bool {
        self.data[..self.count].contains(&item)
    }
    fn add(&mut self, item: Item) -> bool {
        if self.count == N {
            self.overflowed = true;
            return false;
        }
        self.data[self.count] = item;
        self.count += 1;
        true
    }
    fn get(&mut self) -> Option<Item> {
        if self.count == 0 {
            return None;
        }
        self.count -= 1;
        Some(self.data[self.count])
    }
}
struct State {
    tbu: SmallSet<64>,
    tbd: SmallSet<256>,
    global: SmallSet<128>,
    last_owner: u8,
}
struct ProcessState(UnsafeCell<State>);
// SAFETY: game-state mutex serializes calls. `access` cannot call native code;
// all its borrows end before callbacks, allowing same-thread native reentry.
unsafe impl Sync for ProcessState {}
static STATE: ProcessState = ProcessState(UnsafeCell::new(State {
    tbu: SmallSet::new(),
    tbd: SmallSet::new(),
    global: SmallSet::new(),
    last_owner: INVALID,
}));
fn access<T>(f: impl FnOnce(&mut State) -> T) -> T {
    // SAFETY: caller holds the native game-state mutex; closure never reenters.
    unsafe { f(&mut *STATE.0.get()) }
}
struct World<'a> {
    shared: &'a Services,
    rail: &'a RailMapServices,
    native: &'a SignalServices,
    map: Map,
}
macro_rules! read {
    ($w:expr, $group:ident, $field:ident $(, $arg:expr)*) => {{
        crate::witness::hit(crate::witness::SIGNAL_SERVICES);
        let result = ($w.$group.$field)($($arg),*);
        crate::witness::add(crate::witness::SIGNAL_BYTES, std::mem::size_of_val(&result) as u64);
        result
    }};
}
fn track_bits(status: u32) -> u8 {
    ((status | (status >> 8)) & 0x3f) as u8
}
fn reverse(dir: u8) -> u8 {
    dir ^ 2
}
fn pbs(kind: u8) -> bool {
    kind == 4 || kind == 5
}
fn presignal_exit(kind: u8) -> bool {
    kind == 2 || kind == 3
}
fn add_tbu(w: &World<'_>, item: Item) -> bool {
    let added = access(|s| s.tbu.add(item));
    if !added {
        read!(w, native, overflow_tbu);
        crate::witness::hit(crate::witness::SIGNAL_TBU_OVERFLOW);
    }
    added
}
fn add_tbd(w: &World<'_>, item: Item) -> bool {
    let added = access(|s| s.tbd.add(item));
    if !added {
        read!(w, native, overflow_tbd);
        crate::witness::hit(crate::witness::SIGNAL_TBD_OVERFLOW);
    }
    added
}
fn add_global(w: &World<'_>, item: Item) -> bool {
    let added = access(|s| s.global.add(item));
    if !added {
        read!(w, native, overflow_global);
        crate::witness::hit(crate::witness::SIGNAL_GLOBAL_OVERFLOW);
    }
    added
}
fn source_assert(w: &World<'_>, condition: bool) {
    if w.native.assertions {
        read!(w, native, assert_condition, condition);
    }
}
fn maybe_add(w: &World<'_>, next: Item, previous: Item) -> bool {
    access(|s| {
        s.global.remove(next);
        s.global.remove(previous);
    });
    source_assert(w, access(|s| !s.tbd.contains(next)));
    let add = access(|s| !s.tbd.remove(previous));
    !add || add_tbd(w, next)
}
fn train_on_tile(w: &World<'_>, tile: u32, flags: &mut u16) {
    if *flags & TRAIN == 0 && read!(w, native, train_on_tile, tile) {
        *flags |= TRAIN;
        crate::witness::hit(crate::witness::SIGNAL_TRAIN);
    }
}
#[allow(clippy::too_many_lines)]
fn explore(w: &World<'_>, owner: u8) -> u16 {
    let mut flags = 0;
    while let Some(Item { mut tile, mut dir }) = access(|s| s.tbd.get()) {
        crate::witness::hit(crate::witness::SIGNAL_TILES);
        let old_tile = tile;
        let mut exitdir = if dir == INVALID {
            INVALID
        } else {
            reverse(dir)
        };
        match read!(w, shared, tile_type, tile) {
            1 => {
                if read!(w, shared, tile_owner, tile) != owner {
                    crate::witness::hit(crate::witness::SIGNAL_OWNER_BARRIER);
                    continue;
                }
                let geometry = read!(w, rail, traversal, tile);
                if geometry.depot {
                    if dir == INVALID {
                        crate::witness::hit(crate::witness::SIGNAL_DEPOT);
                        train_on_tile(w, tile, &mut flags);
                        exitdir = read!(w, rail, rail_depot_direction, tile).0;
                        tile = w.map.add_diag(tile, exitdir);
                        dir = reverse(exitdir);
                    } else if dir == read!(w, rail, rail_depot_direction, tile).0 {
                        train_on_tile(w, tile, &mut flags);
                        continue;
                    } else {
                        continue;
                    }
                } else {
                    source_assert(w, dir < 4);
                    let mut tracks = geometry.tracks.0;
                    let masked = tracks & ENTER_TRACKS[usize::from(dir)];
                    if tracks == 0x0c || tracks == 0x30 {
                        tracks = masked;
                        if flags & TRAIN == 0 && read!(w, native, train_on_tracks, tile, tracks) {
                            flags |= TRAIN;
                            crate::witness::hit(crate::witness::SIGNAL_TRAIN);
                        }
                    } else {
                        if masked == 0 {
                            continue;
                        }
                        train_on_tile(w, tile, &mut flags);
                    }
                    if tracks.count_ones() > 1 {
                        flags |= SPLIT;
                        crate::witness::hit(crate::witness::SIGNAL_SPLIT);
                    }
                    let td = ((u32::from(tracks) * 0x101)
                        & u32::from(ENTER_TRACKDIRS[usize::from(dir)]))
                    .trailing_zeros() as u8;
                    let signal = read!(w, rail, signal_presence, tile, masked, td);
                    if signal.signals {
                        if signal.track {
                            let kind = signal.kind.0;
                            let back = td ^ 8;
                            if signal.reverse {
                                if pbs(kind) {
                                    flags |= PBS;
                                    crate::witness::hit(crate::witness::SIGNAL_PBS);
                                }
                                if flags & ENTER != 0 {
                                    flags |= MULTI_ENTER;
                                    crate::witness::hit(crate::witness::SIGNAL_MULTI_ENTER);
                                }
                                flags |= ENTER;
                                if !add_tbu(w, Item { tile, dir: back }) {
                                    return flags | FULL;
                                }
                            }
                            if signal.forward && kind == 4 {
                                flags |= PBS;
                            }
                            if flags & MULTI_GREEN == 0 && presignal_exit(kind) && signal.forward {
                                if flags & EXIT != 0 {
                                    flags |= MULTI_EXIT;
                                    crate::witness::hit(crate::witness::SIGNAL_MULTI_EXIT);
                                }
                                flags |= EXIT;
                                crate::witness::hit(crate::witness::SIGNAL_EXIT);
                                if read!(w, rail, signal_state, tile, td).0 == 1 {
                                    if flags & GREEN != 0 {
                                        flags |= MULTI_GREEN;
                                        crate::witness::hit(crate::witness::SIGNAL_MULTI_GREEN);
                                    }
                                    flags |= GREEN;
                                    crate::witness::hit(crate::witness::SIGNAL_GREEN_EXIT);
                                }
                            }
                            continue;
                        }
                    }
                    for exit in 0..4 {
                        if exit != dir && tracks & ENTER_TRACKS[usize::from(exit)] != 0 {
                            let next = Item {
                                tile: w.map.add_diag(tile, exit),
                                dir: reverse(exit),
                            };
                            if !maybe_add(w, next, Item { tile, dir: exit }) {
                                return flags | FULL;
                            }
                        }
                    }
                    continue;
                }
            }
            5 => {
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
                tile = w.map.add_diag(tile, exitdir);
            }
            2 => {
                let crossing = read!(w, rail, crossing_admission, tile, owner);
                if !crossing.crossing || crossing.owner != owner || (dir & 1) == crossing.axis.0 {
                    continue;
                }
                crate::witness::hit(crate::witness::SIGNAL_CROSSING);
                train_on_tile(w, tile, &mut flags);
                tile = w.map.add_diag(tile, exitdir);
            }
            9 => {
                let wormhole = read!(w, rail, wormhole_admission, tile, owner);
                if wormhole.owner != owner || wormhole.transport.0 != 0 {
                    continue;
                }
                crate::witness::hit(crate::witness::SIGNAL_WORMHOLE);
                let direction = wormhole.direction.0;
                if dir == INVALID {
                    train_on_tile(w, tile, &mut flags);
                    dir = direction;
                    exitdir = reverse(direction);
                    tile = w.map.add_diag(tile, exitdir);
                } else {
                    if reverse(dir) != direction {
                        continue;
                    }
                    train_on_tile(w, tile, &mut flags);
                    tile = read!(w, rail, other_tunnel_bridge_end, tile);
                    dir = INVALID;
                    exitdir = INVALID;
                }
            }
            _ => continue,
        }
        if !maybe_add(
            w,
            Item { tile, dir },
            Item {
                tile: old_tile,
                dir: exitdir,
            },
        ) {
            return flags | FULL;
        }
    }
    flags
}
fn update_around(w: &World<'_>, flags: u16) {
    while let Some(Item { tile, dir: td }) = access(|s| s.tbu.get()) {
        let track = td & 7;
        let observation = read!(w, rail, signal_update, tile, td);
        let kind = observation.kind.0;
        let mut new_state = 1;
        if pbs(kind) && observation.reservations.0 & (1 << track) != 0 {
            crate::witness::hit(crate::witness::SIGNAL_RESERVED_SKIP);
            continue;
        }
        if flags & TRAIN != 0 || (pbs(kind) && flags & (SPLIT | MULTI_ENTER) != 0) {
            new_state = 0;
        } else if kind == 3 && read!(w, rail, has_signal_trackdir, tile, td ^ 8) {
            crate::witness::hit(crate::witness::SIGNAL_COMBO);
            if flags & MULTI_EXIT != 0
                && (flags & GREEN == 0
                    || (flags & MULTI_GREEN == 0
                        && read!(w, rail, signal_state, tile, td ^ 8).0 == 1))
            {
                new_state = 0;
                crate::witness::hit(crate::witness::SIGNAL_COMBO_RED);
            }
        } else if (kind == 1 || kind == 3) && flags & EXIT != 0 && flags & GREEN == 0 {
            new_state = 0;
        }
        if new_state != read!(w, rail, signal_state, tile, td).0 {
            if presignal_exit(kind) {
                add_global(
                    w,
                    Item {
                        tile,
                        dir: TRACKDIR_EXIT[usize::from(td ^ 8)],
                    },
                );
                crate::witness::hit(crate::witness::SIGNAL_PROPAGATE);
            }
            read!(w, rail, set_signal_state, tile, td, new_state);
            read!(w, shared, mark_dirty, tile);
            crate::witness::hit(crate::witness::SIGNAL_CHANGED);
        }
    }
}
fn reset_sets() {
    access(|s| {
        s.tbu.reset();
        s.tbd.reset();
        s.global.reset();
    });
}
fn update_buffer(w: &World<'_>, owner: u8) -> u8 {
    read!(w, native, assert_company, owner);
    crate::witness::hit(crate::witness::SIGNAL_UPDATES);
    let mut first = true;
    let mut state = 0; // SIGSEG_FREE
    while let Some(Item { mut tile, mut dir }) = access(|s| s.global.get()) {
        source_assert(w, access(|s| s.tbu.count == 0));
        source_assert(w, access(|s| s.tbd.count == 0));
        let kind = read!(w, shared, tile_type, tile);
        if kind == 9 {
            if w.native.assertions {
                read!(w, native, assert_wormhole_start, tile, dir);
            }
            add_tbd(w, Item { tile, dir: INVALID });
            let other = read!(w, rail, other_tunnel_bridge_end, tile);
            add_tbd(
                w,
                Item {
                    tile: other,
                    dir: INVALID,
                },
            );
        } else if kind == 1 && read!(w, rail, is_rail_depot, tile) {
            if w.native.assertions {
                source_assert(
                    w,
                    dir == INVALID || dir == read!(w, rail, rail_depot_direction, tile).0,
                );
            }
            add_tbd(w, Item { tile, dir: INVALID });
        } else if (kind == 1 || kind == 5 || kind == 2)
            && (track_bits(read!(w, rail, rail_track_status, tile, INVALID))
                & ENTER_TRACKS[usize::from(dir)])
                != 0
        {
            add_tbd(w, Item { tile, dir });
            add_tbd(
                w,
                Item {
                    tile: w.map.add_diag(tile, dir),
                    dir: reverse(dir),
                },
            );
        } else {
            tile = w.map.add_diag(tile, dir);
            dir = reverse(dir);
            if (track_bits(read!(w, rail, rail_track_status, tile, INVALID))
                & ENTER_TRACKS[usize::from(dir)])
                == 0
            {
                crate::witness::hit(crate::witness::SIGNAL_NEIGHBOUR_CONTINUE);
                continue;
            }
            add_tbd(w, Item { tile, dir });
        }
        source_assert(w, access(|s| !s.tbd.overflowed));
        source_assert(w, access(|s| s.tbd.count != 0));
        let flags = explore(w, owner);
        if first {
            first = false;
            if flags & PBS != 0 {
                state = 2;
            }
            // SIGSEG_PBS
            else if flags & TRAIN != 0
                || (flags & EXIT != 0 && flags & GREEN == 0)
                || flags & FULL != 0
            {
                state = 1;
            }
        }
        if flags & FULL != 0 {
            reset_sets();
            crate::witness::hit(crate::witness::SIGNAL_RESET);
            break;
        }
        update_around(w, flags);
    }
    state
}
fn add_track(w: &World<'_>, tile: u32, track: u8, owner: u8) {
    source_assert(w, access(|s| s.global.count == 0 || owner == s.last_owner));
    access(|s| s.last_owner = owner);
    add_global(
        w,
        Item {
            tile,
            dir: SEARCH_ONE[usize::from(track)],
        },
    );
    add_global(
        w,
        Item {
            tile,
            dir: SEARCH_TWO[usize::from(track)],
        },
    );
    force_flush(w);
}
fn force_flush(w: &World<'_>) {
    if access(|s| s.global.count >= 64) {
        crate::witness::hit(crate::witness::SIGNAL_FORCED_FLUSH);
        update_buffer(w, access(|s| s.last_owner));
        access(|s| s.last_owner = INVALID);
    }
}
fn flush(w: &World<'_>) {
    if access(|s| s.global.count != 0) {
        update_buffer(w, access(|s| s.last_owner));
        access(|s| s.last_owner = INVALID);
    }
}
fn add_side(w: &World<'_>, tile: u32, side: u8, owner: u8) {
    source_assert(w, access(|s| s.global.count == 0 || owner == s.last_owner));
    access(|s| s.last_owner = owner);
    add_global(w, Item { tile, dir: side });
    force_flush(w);
}
fn segment(w: &World<'_>, tile: u32, side: u8, owner: u8) -> u8 {
    source_assert(w, access(|s| s.global.count == 0));
    add_global(w, Item { tile, dir: side });
    update_buffer(w, owner)
}
fn both(w: &World<'_>, tile: u32, track: u8, owner: u8) {
    source_assert(w, access(|s| s.global.count == 0));
    add_track(w, tile, track, owner);
    update_buffer(w, owner);
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
            // Native allocation constrains LogX to 6..=12 (or zero before allocation);
            // adapt once to the existing shared Map representation without guards.
            map: Map { log_x: log_x as u8 },
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
    flush(&w);
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
    add_track(&w, tile, track, owner);
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
    add_side(&w, tile, side, owner);
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
    let state = segment(&w, tile, side, owner);
    crate::witness::hit(crate::witness::SIGNAL_CLASSIFICATION_BYTES);
    state
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
    both(&w, tile, track, owner);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_owner_is_shared_across_serialized_game_threads() {
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
        access(|s| s.last_owner = INVALID);
    }
}

/// Call-scoped bridge to a direct Rust signal entry from other Rust owners.
/// All pointers address immutable process-lifetime native service tables; map
/// width is captured for this invocation. This contains no signal-state mirror.
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
        segment(&self.world(), tile, side, owner)
    }
    pub(crate) fn both(self, tile: u32, track: u8, owner: u8) {
        both(&self.world(), tile, track, owner);
    }
    pub(crate) fn add_track(self, tile: u32, track: u8, owner: u8) {
        add_track(&self.world(), tile, track, owner);
    }
    pub(crate) fn add_side(self, tile: u32, side: u8, owner: u8) {
        add_side(&self.world(), tile, side, owner);
    }
    pub(crate) fn flush(self) {
        flush(&self.world());
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
/// Bounded report from the scenario adapter's occupancy accessor link wrapper.
/// The ordinary game never calls this diagnostic entry.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_signal_count_occupancy(reads: u64) {
    crate::witness::add(crate::witness::SIGNAL_OCCUPANCY_ACCESSORS, reads);
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
fn probe_set<const N: usize>(overflow: extern "C" fn()) -> SetTrace {
    let mut set = SmallSet::<N>::new();
    let a = Item { tile: 1, dir: 0 };
    for item in [
        a,
        Item {
            tile: 2,
            dir: INVALID,
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
        dir: INVALID,
    });
    if full_failed {
        overflow();
    }
    let overflow_full = set.overflowed;
    set.remove(Item { tile: 0, dir: 0 });
    let first_after_remove = set.data[0].into();
    let mut refill_succeeded = set.add(Item {
        tile: u32::MAX,
        dir: INVALID,
    });
    let refill_pop = set.get().unwrap().into();
    while set.get().is_some() {}
    let drained_count = set.count as c_uint;
    let overflow_drained = set.overflowed;
    refill_succeeded &= set.add(Item {
        tile: u32::MAX,
        dir: INVALID,
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
pub unsafe extern "C" fn openttd_rust_signal_workset_probe(
    native: *const SignalServices,
    output: *mut SetTrace,
) {
    // SAFETY: caller supplies live immutable callbacks and three output records.
    unsafe {
        output.write(probe_set::<64>((*native).overflow_tbu));
        output
            .add(1)
            .write(probe_set::<256>((*native).overflow_tbd));
        output
            .add(2)
            .write(probe_set::<128>((*native).overflow_global));
    }
}
