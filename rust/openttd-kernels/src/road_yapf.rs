/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete road track/depot searches; #121 retains the sole persistent path.
//! Lookup maps are never iterated; arena identity survives open-node replacement.
//! Heap ties follow `CBinaryHeapT`, including removal's upward then downward repair.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::similar_names
)]
use crate::map_func::Map;
use crate::node_hash::NodeMap;
use crate::road::{PathElement, State};
use crate::services::Services;
use std::cell::UnsafeCell;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Input {
    pub map_x: u32,
    pub map_y: u32,
    pub tile: u32,
    pub dest_tile: u32,
    pub max_nodes: i32,
    pub slope: i32,
    pub crossing: i32,
    pub stop: i32,
    pub occupied: i32,
    pub bay: i32,
    pub curve: i32,
    pub order_destination: u16,
    pub order_type: u8,
    pub bus: u8,
    pub articulated: u8,
    pub trackdir: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tile {
    pub station: u16,
    pub r#type: u8,
    pub station_type: u8,
    pub depot: u8,
    pub depot_dir: u8,
    pub crossing: u8,
    pub waypoint: u8,
    pub drive_through: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Follow {
    pub tile: u32,
    pub skipped: i32,
    pub dirs: u16,
    pub followed: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(clippy::struct_field_names)] // Existing native ABI names distinguish the four speed observations.
pub struct Traversal {
    pub display_speed: i32,
    pub max_speed: i32,
    pub min_speed: i32,
    pub order_speed: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Area {
    pub tile: u32,
    pub width: u16,
    pub height: u16,
    pub valid: u8,
    pub stop: u8,
    pub drive_through: u8,
    pub next: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    /// Shared table for the road stop owner's cost observations.
    pub shared: *const Services,
    pub road_kind: unsafe extern "C" fn(*const c_void) -> crate::services::RoadTramType,
    pub tile: unsafe extern "C" fn(*const c_void, u32, u8, bool) -> Tile,
    pub follow: unsafe extern "C" fn(*const c_void, u32, u8) -> Follow,
    pub traversal: unsafe extern "C" fn(*const c_void, u32) -> Traversal,
    pub closest: unsafe extern "C" fn(*const c_void, u16, u8) -> u32,
    pub area: unsafe extern "C" fn(*const c_void, u16) -> Area,
}
#[repr(C)]
pub struct Result {
    pub tile: u32,
    pub cost: i32,
    pub direction: u8,
    pub found: u8,
    pub rounds: i32,
    pub open: i32,
    pub closed: i32,
    pub calcs: i32,
    pub distance: i32,
}
const EXIT: [u8; 14] = [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0];
const REACHES: [u16; 4] = [
    (1 << 0) | (1 << 3) | (1 << 12),
    (1 << 1) | (1 << 4) | (1 << 2),
    (1 << 8) | (1 << 10) | (1 << 5),
    (1 << 9) | (1 << 13) | (1 << 11),
];
const INVALID: u8 = 255;
#[derive(Clone, Copy)]
struct Node {
    tile: u32,
    td: u8,
    last_tile: u32,
    last_td: u8,
    parent: Option<usize>,
    cost: i32,
    estimate: i32,
    choice: bool,
    destination: Option<bool>,
}
impl Node {
    fn key(self) -> (u32, u8) {
        (self.tile, EXIT[usize::from(self.td)])
    }
    fn new(tile: u32, td: u8, parent: Option<usize>, choice: bool) -> Self {
        Self {
            tile,
            td,
            last_tile: tile,
            last_td: td,
            parent,
            cost: 0,
            estimate: 0,
            choice,
            destination: None,
        }
    }
}

/// Only value nodes and indices persist; vehicle pointers and input descriptors
/// remain in the call-local Search. Capacity grows only for a larger search.
#[derive(Default)]
struct Scratch {
    arena: Vec<Node>,
    open: NodeMap<(u32, u8), usize>,
    closed: NodeMap<(u32, u8), usize>,
    heap: Vec<usize>,
}
struct SerializedScratch(UnsafeCell<Option<Scratch>>);
// SAFETY: Production searches run under the native game-state mutex. Taking or
// returning scratch ends its global borrow before any callback; active searches
// own their storage, so a nested call cannot alias it. Native heap probes keep
// local storage and do not access this process-wide cache.
unsafe impl Sync for SerializedScratch {}
static SCRATCH: SerializedScratch = SerializedScratch(UnsafeCell::new(None));

struct Search {
    input: Input,
    leaves: Leaves,
    context: *const c_void,
    depot: bool,
    max_cost: i32,
    destination: u32,
    dest_dirs: u16,
    dest_station: Option<u16>,
    station_type: u8,
    arena: Vec<Node>,
    // Lookup-only (never iterated); order never selects a node.
    open: NodeMap<(u32, u8), usize>,
    closed: NodeMap<(u32, u8), usize>,
    heap: Vec<usize>,
    intermediate: Option<usize>,
    best_dest: Option<usize>,
    rounds: i32,
    calcs: i32,
    reuse_scratch: bool,
}
impl Search {
    fn new(
        input: Input,
        leaves: Leaves,
        context: *const c_void,
        depot: bool,
        max_cost: i32,
    ) -> Self {
        Self::with_scratch(
            input,
            leaves,
            context,
            depot,
            max_cost,
            Scratch::default(),
            false,
        )
    }
    /// The caller holds the game-state mutex for the complete search. No global
    /// borrow survives this acquisition or any of the search's native queries.
    unsafe fn reused(
        input: Input,
        leaves: Leaves,
        context: *const c_void,
        depot: bool,
        max_cost: i32,
    ) -> Self {
        // SAFETY: Native production entries serialize access. Ownership moves
        // out of the slot before any query; first use has no allocation yet.
        let scratch = unsafe { (*SCRATCH.0.get()).take().unwrap_or_default() };
        Self::with_scratch(input, leaves, context, depot, max_cost, scratch, true)
    }
    fn with_scratch(
        input: Input,
        leaves: Leaves,
        context: *const c_void,
        depot: bool,
        max_cost: i32,
        mut scratch: Scratch,
        reuse_scratch: bool,
    ) -> Self {
        scratch.arena.clear();
        scratch.open.clear();
        scratch.closed.clear();
        scratch.heap.clear();
        scratch.heap.push(usize::MAX);
        Self {
            input,
            leaves,
            context,
            depot,
            max_cost,
            destination: 0,
            dest_dirs: 0,
            dest_station: None,
            station_type: 0,
            arena: scratch.arena,
            open: scratch.open,
            closed: scratch.closed,
            heap: scratch.heap,
            intermediate: None,
            best_dest: None,
            rounds: 0,
            calcs: 0,
            reuse_scratch,
        }
    }
    fn tracks(&self, tile: u32, status: u8) -> u16 {
        // SAFETY: The native search keeps the same live road vehicle context.
        let shared = unsafe { &*self.leaves.shared };
        let tram = unsafe { (self.leaves.road_kind)(self.context) }.0;
        if status != 0 {
            crate::track_func::track_status_to_trackdir_bits(crate::movement::track_status(
                shared,
                tile,
                crate::transport_type::TRANSPORT_ROAD,
                u32::from(tram),
                crate::direction_type::INVALID_DIAGDIR,
            ))
        } else {
            crate::pathfinder_func::road_trackdirs(shared, tile, tram)
        }
    }
    fn tile(&self, tile: u32, td: u8, cost: bool) -> Tile {
        // SAFETY: Shared map/pool query has no owner access or reentry.
        unsafe { (self.leaves.tile)(self.context, tile, td, cost) }
    }
    fn follow(&self, tile: u32, td: u8) -> Follow {
        // SAFETY: Existing shared follower, opaque live context, copied output.
        unsafe { (self.leaves.follow)(self.context, tile, td) }
    }
    fn destination(&mut self) {
        if self.input.order_type == 1 || self.input.order_type == 6 {
            self.dest_station = Some(self.input.order_destination);
            self.station_type = if self.input.order_type == 6 {
                8
            } else if self.input.bus != 0 {
                3
            } else {
                2
            };
            // SAFETY: Shared station-area query cannot reenter or modify owners.
            self.destination = unsafe {
                (self.leaves.closest)(
                    self.context,
                    self.input.order_destination,
                    self.station_type,
                )
            };
            self.dest_dirs = 0xffff;
        } else {
            self.destination = self.input.dest_tile;
            self.dest_dirs = self.tracks(self.destination, 1);
        }
    }
    fn detected(&self, tile: u32, td: u8, t: Tile) -> bool {
        if self.depot {
            return t.depot != 0;
        }
        if let Some(station) = self.dest_station {
            return t.r#type == 5
                && t.station == station
                && t.station_type == self.station_type
                && (self.input.articulated == 0 || t.drive_through != 0);
        }
        tile == self.destination && self.dest_dirs & (1 << td) != 0
    }
    fn endpoint_detected(&self, n: Node) -> bool {
        n.destination.unwrap_or_else(|| {
            self.detected(
                n.last_tile,
                n.last_td,
                self.tile(n.last_tile, n.last_td, false),
            )
        })
    }
    fn less(&self, a: usize, b: usize) -> bool {
        self.arena[a].estimate < self.arena[b].estimate
    }
    fn up(&mut self, mut gap: usize, item: usize) -> usize {
        while gap > 1 {
            let parent = gap / 2;
            if !self.less(item, self.heap[parent]) {
                break;
            }
            self.heap[gap] = self.heap[parent];
            gap = parent;
        }
        gap
    }
    fn down(&mut self, mut gap: usize, item: usize) -> usize {
        let mut child = gap * 2;
        while child < self.heap.len() {
            if child + 1 < self.heap.len() && self.less(self.heap[child + 1], self.heap[child]) {
                child += 1;
            }
            if !self.less(self.heap[child], item) {
                break;
            }
            self.heap[gap] = self.heap[child];
            gap = child;
            child = gap * 2;
        }
        gap
    }
    fn include(&mut self, item: usize) {
        self.heap.push(0);
        let gap = self.up(self.heap.len() - 1, item);
        self.heap[gap] = item;
    }
    fn remove(&mut self, item: usize) {
        let index = self.heap.iter().position(|i| *i == item).unwrap();
        let last = self.heap.pop().unwrap();
        if index < self.heap.len() {
            let gap = self.up(index, last);
            let gap = self.down(gap, last);
            self.heap[gap] = last;
        }
    }
    fn startup(&mut self, tile: u32, mut dirs: u16) {
        let choice = dirs.count_ones() > 1;
        while dirs != 0 {
            let td = dirs.trailing_zeros() as u8;
            dirs &= dirs - 1;
            let n = Node::new(tile, td, None, choice);
            if !self.open.contains_key(&n.key()) {
                let i = self.arena.len();
                self.arena.push(n);
                self.open.insert(n.key(), i);
                self.include(i);
            }
        }
    }
    fn one_tile(&self, tile: u32, t: Tile, td: u8) -> i32 {
        if td & 7 > 1 {
            return 71_i32.wrapping_add(self.input.curve);
        }
        let mut cost: i32 = 100;
        if t.r#type == 2 && t.crossing != 0 {
            cost = cost.wrapping_add(self.input.crossing);
        }
        if t.r#type == 5 && t.waypoint == 0 {
            // SAFETY: Production leaves carry the static shared table.
            let shared = unsafe { &*self.leaves.shared };
            let rs = crate::road_stop::at(shared, tile);
            if t.drive_through != 0 {
                cost = cost.wrapping_add(self.input.stop);
                let dir = EXIT[usize::from(td)];
                let map = Map {
                    log_x: self.input.map_x.trailing_zeros() as u8,
                };
                if !crate::road_stop::continuation(
                    shared,
                    tile,
                    tile.wrapping_sub(map.offs_by_diag_dir(dir)),
                ) {
                    let entry = crate::road_stop::entry_read(rs, dir);
                    cost = cost.wrapping_add(
                        (u32::from(entry.occupied).wrapping_mul(self.input.occupied as u32)
                            / u32::from(entry.length)) as i32,
                    );
                }
            } else {
                let busy_bays = u32::from(!crate::road_stop::is_free_bay(rs, 0))
                    + u32::from(!crate::road_stop::is_free_bay(rs, 1));
                cost = cost.wrapping_add(
                    (self.input.bay as u32)
                        .wrapping_mul(busy_bays)
                        .wrapping_div(2) as i32,
                );
            }
        }
        cost
    }
    fn calc_cost(&self, n: &mut Node) -> bool {
        let parent_cost = n.parent.map_or(0, |i| self.arena[i].cost);
        let (mut tile, mut td, mut cost, mut tiles) = (n.tile, n.td, 0_i32, 0_u32);
        loop {
            let t = self.tile(tile, td, true);
            cost = cost.wrapping_add(self.one_tile(tile, t, td));
            let destination = self.detected(tile, td, t);
            n.destination = Some(destination);
            if destination {
                break;
            }
            if self.max_cost > 0 && parent_cost.wrapping_add(cost) > self.max_cost {
                return false;
            }
            if t.depot != 0
                && td
                    == crate::track_func::diag_dir_to_diag_trackdir((t.depot_dir + 2) & 3, |ok| {
                        crate::movement::native_assert(unsafe { &*self.leaves.shared }, ok);
                    })
            {
                break;
            }
            let f = self.follow(tile, td);
            if f.followed == 0 || f.dirs.count_ones() > 1 {
                break;
            }
            let new_td = f.dirs.trailing_zeros() as u8;
            if f.tile == n.tile && new_td == n.td {
                return false;
            }
            cost = cost.wrapping_add(f.skipped.wrapping_mul(100));
            tiles = tiles.wrapping_add(f.skipped.wrapping_add(1) as u32);
            // SAFETY: After accepted continuation, observe current/next height,
            // then vehicle and old-tile follower limits, without world mutation.
            let shared = unsafe { &*self.leaves.shared };
            let map = crate::services::Map {
                log_x: self.input.map_x.trailing_zeros() as u8,
            };
            let center_z = |tile| {
                crate::movement::pixel_z(
                    shared,
                    (map.tile_x(tile) * crate::tile_type::TILE_SIZE
                        + crate::tile_type::TILE_SIZE / 2)
                        .cast_signed(),
                    (map.tile_y(tile) * crate::tile_type::TILE_SIZE
                        + crate::tile_type::TILE_SIZE / 2)
                        .cast_signed(),
                    true,
                )
            };
            let current_z = center_z(tile);
            let next_z = center_z(f.tile);
            let facts = unsafe { (self.leaves.traversal)(self.context, tile) };
            if next_z.wrapping_sub(current_z) > 1 {
                cost = cost.wrapping_add(self.input.slope);
            }
            let speed = facts.display_speed.min(i32::from(facts.order_speed) * 2);
            if facts.max_speed < speed {
                cost = cost.wrapping_add(
                    100_i32
                        .wrapping_mul(speed.wrapping_sub(facts.max_speed))
                        .wrapping_mul(4_i32.wrapping_add(f.skipped))
                        / speed,
                );
            }
            if facts.min_speed > speed {
                cost = cost.wrapping_add(100_i32.wrapping_mul(facts.min_speed.wrapping_sub(speed)));
            }
            n.destination = None; // The advanced endpoint has not been observed.
            tile = f.tile;
            td = new_td;
            if tiles > 4096 {
                break;
            }
        }
        n.last_tile = tile;
        n.last_td = td;
        n.cost = parent_cost.wrapping_add(cost);
        true
    }
    fn estimate(&self, n: Node) -> i32 {
        if self.depot || self.endpoint_detected(n) {
            return n.cost;
        }
        let dir = usize::from(EXIT[usize::from(n.last_td)]);
        let x = (2 * (n.last_tile % self.input.map_x)) as i32 + [-1, 0, 1, 0][dir];
        let y = (2 * (n.last_tile / self.input.map_x)) as i32 + [0, 1, 0, -1][dir];
        let dx = (x - (2 * (self.destination % self.input.map_x)) as i32).abs();
        let dy = (y - (2 * (self.destination / self.input.map_x)) as i32).abs();
        let estimate = n
            .cost
            .wrapping_add(dx.min(dy).wrapping_mul(71))
            .wrapping_add((dx - dy).abs().wrapping_sub(1).wrapping_mul(50));
        assert!(estimate >= self.arena[n.parent.unwrap()].estimate);
        estimate
    }
    fn add(&mut self, mut n: Node) {
        self.calcs = self.calcs.wrapping_add(1);
        if !self.calc_cost(&mut n) {
            return;
        }
        n.estimate = self.estimate(n);
        let set_intermediate = self.input.max_nodes > 0
            && self.intermediate.is_none_or(|i| {
                self.arena[i].estimate.wrapping_sub(self.arena[i].cost)
                    > n.estimate.wrapping_sub(n.cost)
            });
        if let Some(&old) = self.open.get(&n.key()) {
            if n.estimate < self.arena[old].estimate {
                self.remove(old);
                self.arena[old] = n;
                self.include(old);
                if set_intermediate {
                    self.intermediate = Some(old);
                }
            }
            return;
        }
        if let Some(&old) = self.closed.get(&n.key()) {
            if n.estimate < self.arena[old].estimate {
                unreachable!("original closed-node improvement NOT_REACHED");
            }
            return;
        }
        let i = self.arena.len();
        self.arena.push(n);
        self.open.insert(n.key(), i);
        self.include(i);
        if set_intermediate {
            self.intermediate = Some(i);
        }
    }
    #[allow(clippy::while_let_loop)]
    fn find(&mut self) -> bool {
        loop {
            self.rounds = self.rounds.wrapping_add(1);
            let Some(&best) = self.heap.get(1) else {
                break;
            };
            let n = self.arena[best];
            if self.endpoint_detected(n) {
                self.best_dest = Some(best);
                break;
            }
            let f = self.follow(n.last_tile, n.last_td);
            if f.followed != 0 {
                let choice = f.dirs.count_ones() > 1;
                let mut dirs = f.dirs;
                while dirs != 0 {
                    let td = dirs.trailing_zeros() as u8;
                    dirs &= dirs - 1;
                    self.add(Node::new(f.tile, td, Some(best), choice));
                }
            }
            if self.input.max_nodes != 0 && self.closed.len() as i32 >= self.input.max_nodes {
                break;
            }
            self.remove(best);
            self.open.remove(&n.key());
            self.closed.insert(n.key(), best);
        }
        self.best_dest.is_some()
    }
    fn best(&self) -> Option<usize> {
        self.best_dest.or(self.intermediate)
    }
    fn result(&self, direction: u8, found: u8) -> Result {
        let best = self.best().map(|i| self.arena[i]);
        Result {
            tile: best.map_or(u32::MAX, |n| n.last_tile),
            cost: best.map_or(0, |n| n.cost),
            direction,
            found,
            rounds: self.rounds,
            open: self.open.len() as i32,
            closed: self.closed.len() as i32,
            calcs: self.calcs,
            distance: best.map_or(0, |n| n.estimate.wrapping_sub(n.cost)),
        }
    }
    unsafe fn reconstruct(&self, state: *mut State) -> u8 {
        let Some(mut index) = self.best() else {
            return INVALID;
        };
        let mut steps = 0;
        let mut cursor = index;
        while let Some(parent) = self.arena[cursor].parent {
            steps += 1;
            cursor = parent;
        }
        while let Some(parent) = self.arena[index].parent {
            steps -= 1;
            let n = self.arena[index];
            if n.choice && steps < 8 {
                // SAFETY: No service runs during this short exclusive cache access.
                unsafe {
                    (*state).path.push(PathElement {
                        trackdir: n.td,
                        tile: n.tile,
                    });
                }
            }
            index = parent;
        }
        if let Some(station) = self.dest_station {
            // SAFETY: No road owner reference exists during the shared station query.
            let a = unsafe { (self.leaves.area)(self.context, station) };
            if a.valid != 0 && a.stop != 0 && (a.drive_through != 0 || a.next != 0) {
                let x = (a.tile % self.input.map_x) as i32;
                let y = (a.tile / self.input.map_x) as i32;
                let sx = (x - 8).max(0);
                let sy = (y - 8).max(0);
                let ex = (x + i32::from(a.width) + 8).min(self.input.map_x as i32);
                let ey = (y + i32::from(a.height) + 8).min(self.input.map_y as i32);
                // SAFETY: Shared query has returned; no callback overlaps the Vec borrow.
                let path = unsafe { &mut (*state).path };
                let end = path
                    .iter()
                    .position(|p| {
                        let tx = (p.tile % self.input.map_x) as i32;
                        let ty = (p.tile / self.input.map_x) as i32;
                        !(sx <= tx && tx < ex && sy <= ty && ty < ey)
                    })
                    .unwrap_or(path.len());
                path.drain(..end);
            }
        }
        self.arena[index].td
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        if self.reuse_scratch {
            let scratch = Scratch {
                arena: std::mem::take(&mut self.arena),
                open: std::mem::take(&mut self.open),
                closed: std::mem::take(&mut self.closed),
                heap: std::mem::take(&mut self.heap),
            };
            // SAFETY: Only serialized production entries set reuse_scratch.
            // No borrow of the slot spans callbacks, and no native pointer is
            // stored here. An outer search can replace a nested call's cache.
            unsafe { *SCRATCH.0.get() = Some(scratch) };
        }
    }
}

/// Caller supplies initialized descriptors, live opaque vehicle and sole road
/// owner under the game-state mutex; services cannot reenter either owner.
/// No pointer escapes. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_yapf_choose(
    state: *mut State,
    input: *const Input,
    leaves: *const Leaves,
    context: *const c_void,
    tile: u32,
    entry: u8,
    dirs: u16,
    found: u8,
) -> Result {
    // SAFETY: Caller-owned descriptors remain live and immutable for this call.
    let (input, leaves) = unsafe { (*input, *leaves) };
    if tile == input.dest_tile && input.order_type != 1 {
        return Result {
            tile,
            cost: 0,
            direction: crate::track_func::diag_dir_to_diag_trackdir(entry, |ok| {
                crate::movement::native_assert(unsafe { &*leaves.shared }, ok);
            }),
            found,
            rounds: 0,
            open: 0,
            closed: 0,
            calcs: 0,
            distance: 0,
        };
    }
    // SAFETY: The production caller serializes searches with the game-state mutex.
    let mut search = unsafe { Search::reused(input, leaves, context, false, 0) };
    search.startup(tile, search.tracks(tile, 0) & REACHES[usize::from(entry)]);
    search.destination();
    let found = u8::from(search.find());
    // SAFETY: Canonical road owner live and no borrow overlaps world services.
    let mut direction = unsafe { search.reconstruct(state) };
    if direction == INVALID {
        direction = dirs.trailing_zeros() as u8;
    }
    search.result(direction, found)
}
/// Same synchronous descriptor/lifetime contract as choose; no path is mutated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_yapf_depot(
    input: *const Input,
    leaves: *const Leaves,
    context: *const c_void,
    max_cost: i32,
) -> Result {
    // SAFETY: Caller supplies live initialized immutable scalar descriptors.
    let (input, leaves) = unsafe { (*input, *leaves) };
    // SAFETY: Same serialized production entry contract as choose.
    let mut search = unsafe { Search::reused(input, leaves, context, true, max_cost) };
    let dirs = search.tracks(input.tile, 0);
    // HasTrackdir has the original valid trackdir precondition.
    if dirs & (1 << input.trackdir) == 0 {
        return Result {
            tile: u32::MAX,
            cost: 0,
            direction: INVALID,
            found: 0,
            rounds: 0,
            open: 0,
            closed: 0,
            calcs: 0,
            distance: 0,
        };
    }
    search.startup(input.tile, 1 << input.trackdir);
    let found = u8::from(search.find());
    search.result(INVALID, found)
}

/// Evidence-only entry for the exact heap tie/removal gap absent from save fields.
/// Spans are initialized, disjoint, live, and at most `isize::MAX`; indices and
/// membership follow Include/Remove preconditions. Uses the actual search heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_yapf_heap_probe(
    input: *const Input,
    leaves: *const Leaves,
    costs: *const i32,
    nodes: usize,
    commands: *const u32,
    values: *const i32,
    count: usize,
    output: *mut u32,
) {
    // SAFETY: Native fixture supplies initialized descriptors and valid spans.
    let mut search = unsafe { Search::new(*input, *leaves, std::ptr::null(), false, 0) };
    for i in 0..nodes {
        let mut node = Node::new(i as u32, 0, None, false);
        // SAFETY: costs spans exactly nodes initialized elements.
        node.estimate = unsafe { costs.add(i).read() };
        search.arena.push(node);
    }
    for i in 0..count {
        // SAFETY: commands and values both have count initialized elements.
        let (command, value) = unsafe { (commands.add(i).read(), values.add(i).read()) };
        match command {
            0 => search.include(value as usize),
            1 => search.remove(value as usize),
            2 => {
                let node = search.heap[1];
                search.remove(node);
                // SAFETY: output spans count exclusive writable elements.
                unsafe {
                    output.add(i).write(node as u32);
                }
                continue;
            }
            _ => unreachable!(),
        }
        // SAFETY: Same exclusive output span; no pointer is retained.
        unsafe {
            output
                .add(i)
                .write(search.heap.get(1).map_or(u32::MAX, |n| *n as u32));
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadYapfInput" ["probe"] => Input {
        map_x,
        map_y,
        tile,
        dest_tile,
        max_nodes,
        slope,
        crossing,
        stop,
        occupied,
        bay,
        curve,
        order_destination,
        order_type,
        bus,
        articulated,
        trackdir,
    };
    "OpenTTDRoadYapfTile" ["probe"] => crate::road_yapf::Tile {
        station,
        r#type => type,
        station_type,
        depot,
        depot_dir,
        crossing,
        waypoint,
        drive_through,
    };
    "OpenTTDRoadYapfFollow" ["probe"] => Follow {
        tile,
        skipped,
        dirs,
        followed,
    };
    "OpenTTDRoadYapfArea" ["probe"] => Area {
        tile,
        width,
        height,
        valid,
        stop,
        drive_through,
        next,
    };
    "OpenTTDRoadYapfTraversal" ["probe"] => Traversal {
        display_speed,
        max_speed,
        min_speed,
        order_speed,
    };
    "OpenTTDRoadYapfLeaves" ["probe"] => crate::road_yapf::Leaves {
        shared,
        road_kind,
        tile,
        follow,
        traversal,
        closest,
        area,
    };
    "OpenTTDRoadYapfResult" ["probe"] => Result {
        tile,
        cost,
        direction,
        found,
        rounds,
        open,
        closed,
        calcs,
        distance,
    };
}
