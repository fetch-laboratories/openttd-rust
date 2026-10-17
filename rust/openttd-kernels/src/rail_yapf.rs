/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Rail YAPF owns all search, segment-cache and reservation progression.
//! No persistent state enters a save. The game thread serializes cache access.
//! Station callbacks return through `step`, with no owner/cache borrow retained.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    clippy::if_not_else,
    clippy::collapsible_else_if,
    clippy::similar_names
)]
use crate::node_hash::NodeMap;
use crate::witness;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Settings {
    pub max_nodes: u32,
    pub firstred: u32,
    pub firstred_exit: u32,
    pub lastred: u32,
    pub lastred_exit: u32,
    pub station: u32,
    pub slope: u32,
    pub curve45: u32,
    pub curve90: u32,
    pub depot_reverse: u32,
    pub crossing: u32,
    pub lookahead: u32,
    pub p0: i32,
    pub p1: i32,
    pub p2: i32,
    pub pbs_cross: u32,
    pub pbs_station: u32,
    pub pbs_back: u32,
    pub doubleslip: u32,
    pub longer: u32,
    pub longer_tile: u32,
    pub shorter: u32,
    pub shorter_tile: u32,
    pub firstred_eol: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Tile {
    pub flags: u32,
    pub other_end: u32,
    pub station: u16,
    pub railtype: u8,
    pub tracks: u8,
    pub reserved: u8,
    pub station_track: u8,
    pub tunnel_dir: u8,
    pub uphill: u8,
    pub flat_ramp: u8,
    pub signal_along: u8,
    pub signal_against: u8,
    pub signal_green: u8,
    pub signal_type: u8,
    pub oneway: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Follow {
    pub tile: u32,
    pub skipped: i32,
    pub min_speed: i32,
    pub max_speed: i32,
    pub dirs: u16,
    pub followed: u8,
    pub error: u8,
    pub station: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Train {
    pub compatible: u64,
    pub all_compatible: u64,
    pub tile: u32,
    pub rear_tile: u32,
    pub virtual_tile: u32,
    pub rear_virtual_tile: u32,
    pub dest_tile: u32,
    pub length: u32,
    pub speed: u32,
    pub order_destination: u16,
    pub td: u8,
    pub rear_td: u8,
    pub wormhole: u8,
    pub rear_wormhole: u8,
    pub order: u8,
    pub nearest_depot: u8,
    pub complex_waypoint: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Input {
    pub context: *mut c_void,
    pub vehicle: *const crate::services::Train,
    pub shared: *const crate::services::Services,
    pub settings: Settings,
    pub map_x: u32,
    pub tile: u32,
    pub max_cost: i32,
    pub desync: i32,
    pub kind: u8,
    pub td: u8,
    pub override_railtype: u8,
    pub forbid90: u8,
    pub reserve: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Step {
    pub tile: u32,
    pub destination: u32,
    pub target_tile: u32,
    pub best_length: u32,
    pub action: u8,
    pub td: u8,
    pub found: u8,
    pub reverse: u8,
    pub value: u8,
    pub target_td: u8,
    pub target_okay: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub train: unsafe extern "C" fn(*mut c_void) -> Train,
    pub tile: unsafe extern "C" fn(u32, u8) -> Tile,
    pub follow: unsafe extern "C" fn(*mut c_void, u32, u8, u64, u8, u8) -> Follow,
    pub free: unsafe extern "C" fn(*mut c_void, u32, u8, u8) -> u8,
    pub compatible_station: unsafe extern "C" fn(u32, u32) -> u8,
    pub platform_length: unsafe extern "C" fn(u32, u8) -> u32,
    pub closest_station: unsafe extern "C" fn(*mut c_void, u8) -> u32,
    pub destination_dirs: unsafe extern "C" fn(u32) -> u16,
    pub origin: unsafe extern "C" fn(*mut c_void, *mut u32, *mut u8),
    pub write: unsafe extern "C" fn(u32, u8, u8) -> u8,
    pub output: unsafe extern "C" fn(*mut c_void, u8, *const Step),
    pub debug: unsafe extern "C" fn(*mut c_void, u8, *const u32, u32),
}
const INVALID: u32 = u32::MAX;
const INVALID_TD: u8 = 255;
const RAILWAY: u32 = 1;
const ROAD: u32 = 2;
const STATION_RAIL: u32 = 4;
const RAIL_STATION: u32 = 8;
const WAYPOINT: u32 = 16;
const DEPOT: u32 = 32;
const PLAIN: u32 = 64;
const CROSSING: u32 = 128;
const BRIDGE: u32 = 256;
const TUNNEL: u32 = 512;
const PLATFORM_RESERVED: u32 = 1024;
const ONEWAY_BLOCKING: u32 = 2048;
const DEAD: u16 = 1;
const RAILTYPE: u16 = 1 << 1;
const LOOP: u16 = 1 << 2;
const TOO_LONG: u16 = 1 << 3;
const CHOICE: u16 = 1 << 4;
const E_DEPOT: u16 = 1 << 5;
const E_WAYPOINT: u16 = 1 << 6;
const E_STATION: u16 = 1 << 7;
const SAFE: u16 = 1 << 8;
const PATH_TOO_LONG: u16 = 1 << 9;
const FIRST_RED: u16 = 1 << 10;
const POSSIBLE_TARGET: u16 = E_DEPOT | E_WAYPOINT | E_STATION | SAFE;
const CACHED: u16 = (1 << 9) - 1;
const ABORT: u16 = DEAD | PATH_TOO_LONG | LOOP | FIRST_RED;
const EXIT: [u8; 14] = [0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0];
const NEXT: [u8; 14] = [0, 1, 3, 2, 5, 4, 255, 255, 8, 9, 11, 10, 13, 12];
const CROSS: [u16; 6] = [0x202, 0x101, 0x3030, 0x3030, 0x0c0c, 0x0c0c];
const REACH_TRACKS: [u8; 4] = [0x19, 0x16, 0x25, 0x2a];
fn exit(td: u8) -> u8 {
    EXIT[usize::from(td)]
}
fn diagonal(td: u8) -> bool {
    td & 7 < 2
}
fn pbs(signal: u8) -> bool {
    signal == 4 || signal == 5
}
fn first(dirs: u16) -> u8 {
    dirs.trailing_zeros() as u8
}
fn key(tile: u32, td: u8) -> u32 {
    tile.wrapping_shl(4) | u32::from(td)
}
fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}
fn mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}
fn overlaps(reserved: u8, td: u8) -> bool {
    let bit = 1 << (td & 7);
    if reserved & bit != 0 {
        return true;
    }
    let bits = reserved | bit;
    bits & bits.wrapping_sub(1) != 0 && bits != 0x0c && bits != 0x30
}
fn offset(tile: u32, dir: u8, map_x: u32) -> u32 {
    match dir {
        0 => tile.wrapping_sub(1),
        1 => tile.wrapping_add(map_x),
        2 => tile.wrapping_add(1),
        _ => tile.wrapping_sub(map_x),
    }
}

#[derive(Clone, Copy)]
struct Segment {
    last_tile: u32,
    last_td: u8,
    cost: i32,
    signal_tile: u32,
    signal_td: u8,
    reasons: u16,
}
impl Segment {
    fn new(tile: u32, td: u8) -> Self {
        Self {
            last_tile: tile,
            last_td: td,
            cost: -1,
            signal_tile: INVALID,
            signal_td: INVALID_TD,
            reasons: 0,
        }
    }
}
// Stable shared segments retain identity for every node attached to a bank. No
// RefCell/reference survives a service or station callback. Rc also keeps an
// active search's segments alive if a callback constructs a new search/flushes.
type SegmentRef = Rc<UnsafeCell<Segment>>;
struct Bank {
    seen: i32,
    // Lookup-only cache (get/insert/clear); never iterated.
    segments: NodeMap<u32, SegmentRef>,
}
struct Globals {
    counter: i32,
    banks: [Bank; 6],
}
struct SerializedGlobals(UnsafeCell<Option<Globals>>);
// SAFETY: the C++ facade enters only on the serialized simulation thread. It
// never exposes references; synchronous leaves do not reenter this owner.
unsafe impl Sync for SerializedGlobals {}
static GLOBALS: SerializedGlobals = SerializedGlobals(UnsafeCell::new(None));
unsafe fn globals() -> *mut Globals {
    let slot = GLOBALS.0.get();
    if unsafe { (*slot).is_none() } {
        unsafe {
            *slot = Some(Globals {
                counter: 0,
                banks: std::array::from_fn(|_| Bank {
                    seen: 0,
                    segments: NodeMap::default(),
                }),
            });
        }
    }
    unsafe { (*slot).as_mut().unwrap() }
}
fn invalidate() {
    unsafe {
        let state = globals();
        (*state).counter = (*state).counter.wrapping_add(1);
    }
}
#[derive(Clone)]
struct Node {
    tile: u32,
    td: u8,
    parent: Option<usize>,
    cost: i32,
    estimate: i32,
    segment: SegmentRef,
    signals: u16,
    target: bool,
    choice: bool,
    red: bool,
    red_type: u8,
    signal_type: u8,
}
impl Node {
    fn segment(&self) -> Segment {
        unsafe { *self.segment.get() }
    }
    fn set_segment(&self, value: Segment) {
        unsafe {
            self.segment.get().write(value);
        }
    }
    fn last(&self) -> (u32, u8) {
        let s = self.segment();
        (s.last_tile, s.last_td)
    }
}
struct Search {
    input: Input,
    leaves: Leaves,
    train: Train,
    compatible: u64,
    bank: usize,
    disabled: bool,
    lookahead: Vec<i32>,
    max_cost: i32,
    treat_first_red: bool,
    stopped: bool,
    dest_tile: u32,
    dest_dirs: u16,
    dest_station: u16,
    any_depot: bool,
    arena: Vec<Node>,
    scratch: Option<usize>,
    // Lookup-only (never iterated); order never selects a node.
    open: NodeMap<(u32, u8), usize>,
    closed: NodeMap<(u32, u8), usize>,
    heap: Vec<usize>,
    best_dest: Option<usize>,
    intermediate: Option<usize>,
    rounds: u32,
    hits: u32,
    calcs: u32,
}
impl Search {
    fn new(input: Input, leaves: Leaves, disabled: bool) -> Self {
        let kind = if input.kind == 1 { 0 } else { input.kind };
        let bank = usize::from(if kind == 0 {
            0_u8
        } else if kind == 2 {
            2
        } else {
            4
        }) + usize::from(input.forbid90 != 0);
        let flushed = unsafe {
            let state = globals();
            let c = (*state).counter;
            let b = &mut (*state).banks[bank];
            if b.seen != c {
                b.seen = c;
                b.segments.clear();
                true
            } else {
                false
            }
        };
        if flushed {
            witness::hit(witness::RAIL_CACHE_FLUSHES);
            unsafe {
                (leaves.debug)(input.context, 2, std::ptr::null(), 0);
            }
        }
        let s = input.settings;
        // Original uint i promotes both signed polynomial operands to unsigned.
        let lookahead = (0..s.lookahead)
            .map(|i| {
                (s.p0 as u32).wrapping_add(
                    i.wrapping_mul((s.p1 as u32).wrapping_add(i.wrapping_mul(s.p2 as u32))),
                ) as i32
            })
            .collect();
        let train = unsafe { (leaves.train)(input.context) };
        Self {
            input,
            leaves,
            train,
            compatible: train.compatible,
            bank,
            disabled,
            lookahead,
            max_cost: if input.kind == 2 { input.max_cost } else { 0 },
            treat_first_red: input.kind != 1,
            stopped: false,
            dest_tile: INVALID,
            dest_dirs: u16::MAX,
            dest_station: u16::MAX,
            any_depot: false,
            arena: Vec::new(),
            scratch: None,
            open: NodeMap::default(),
            closed: NodeMap::default(),
            heap: vec![usize::MAX],
            best_dest: None,
            intermediate: None,
            rounds: 0,
            hits: 0,
            calcs: 0,
        }
    }
    fn global_for(&self, node: &Node) -> bool {
        !self.disabled
            && node
                .parent
                .is_some_and(|p| usize::from(self.arena[p].signals) >= self.lookahead.len())
    }
    fn cache(&mut self, n: &mut Node) -> bool {
        if !self.global_for(n) {
            return false;
        }
        let k = key(n.tile, n.td);
        let (segment, found) = unsafe {
            let state = globals();
            let bank = &mut (*state).banks[self.bank];
            if let Some(segment) = bank.segments.get(&k) {
                (Rc::clone(segment), true)
            } else {
                let segment = Rc::new(UnsafeCell::new(Segment::new(n.tile, n.td)));
                bank.segments.insert(k, Rc::clone(&segment));
                (segment, false)
            }
        };
        n.segment = segment;
        if n.segment().cost < 0 {
            let mut s = n.segment();
            s.last_tile = n.tile;
            s.last_td = n.td;
            n.set_segment(s);
        }
        found
    }
    fn node(&self, parent: Option<usize>, tile: u32, td: u8, choice: bool) -> Node {
        let mut n = if let Some(p) = parent {
            self.arena[p].clone()
        } else {
            Node {
                tile,
                td,
                parent,
                cost: 0,
                estimate: 0,
                segment: Rc::new(UnsafeCell::new(Segment::new(tile, td))),
                signals: 0,
                target: false,
                choice: false,
                red: false,
                red_type: 0,
                signal_type: 4,
            }
        };
        n.tile = tile;
        n.td = td;
        n.parent = parent;
        n.cost = 0;
        n.estimate = 0;
        n.segment = Rc::new(UnsafeCell::new(Segment::new(tile, td)));
        n.choice |= choice;
        n
    }
    fn allocate(&mut self, n: Node) -> usize {
        if let Some(i) = self.scratch {
            self.arena[i] = n;
            i
        } else {
            let i = self.arena.len();
            self.arena.push(n);
            self.scratch = Some(i);
            i
        }
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
    fn include(&mut self, i: usize) {
        self.open.insert((self.arena[i].tile, self.arena[i].td), i);
        self.heap.push(0);
        let gap = self.up(self.heap.len() - 1, i);
        self.heap[gap] = i;
        if self.scratch == Some(i) {
            self.scratch = None;
        }
    }
    fn remove(&mut self, i: usize) {
        let index = self.heap.iter().position(|n| *n == i).unwrap();
        let last = self.heap.pop().unwrap();
        if index < self.heap.len() {
            let gap = self.up(index, last);
            let gap = self.down(gap, last);
            self.heap[gap] = last;
        }
        self.open.remove(&(self.arena[i].tile, self.arena[i].td));
    }
    fn startup(&mut self, tile: u32, td: u8, cost: i32) {
        if tile == INVALID || td == INVALID_TD {
            return;
        }
        let mut node = self.node(None, tile, td, false);
        node.cost = cost;
        self.cache(&mut node);
        let i = self.allocate(node);
        if !self.open.contains_key(&(tile, td)) {
            self.include(i);
        }
    }
    fn tile(&self, tile: u32, td: u8) -> Tile {
        unsafe { (self.leaves.tile)(tile, td) }
    }
    fn follow(&self, tile: u32, td: u8, no90: u8, compatible: u64, mask: bool) -> Follow {
        unsafe {
            (self.leaves.follow)(
                self.input.context,
                tile,
                td,
                compatible,
                no90,
                u8::from(mask),
            )
        }
    }
    fn safe(&self, tile: u32, td: u8, forbid: u8) -> bool {
        // SAFETY: The immutable shared table and native train outlive this synchronous search.
        unsafe {
            ((*self.input.shared).safe_waiting_position)(self.input.vehicle, tile, td, forbid != 0)
        }
    }
    fn free(&self, tile: u32, td: u8, forbid: u8) -> bool {
        unsafe { (self.leaves.free)(self.input.context, tile, td, forbid) != 0 }
    }
    fn platform_length(&self, tile: u32, dir: u8) -> u32 {
        unsafe { (self.leaves.platform_length)(tile, dir) }
    }
    fn set_destination(&mut self) {
        if self.input.kind == 3 {
            if self.input.override_railtype != 0 {
                self.compatible |= self.train.all_compatible;
            }
            return;
        }
        if self.input.kind == 2 {
            return;
        }
        match self.train.order {
            1 | 6 => {
                if self.train.order == 6 && self.train.complex_waypoint != 0 {
                    self.disabled = true;
                }
                self.dest_tile = unsafe {
                    (self.leaves.closest_station)(
                        self.input.context,
                        u8::from(self.train.order == 1),
                    )
                };
                self.dest_station = self.train.order_destination;
            }
            _ => {
                self.any_depot = self.train.order == 2 && self.train.nearest_depot != 0;
                self.dest_tile = self.train.dest_tile;
                self.dest_dirs = unsafe { (self.leaves.destination_dirs)(self.dest_tile) };
            }
        }
    }
    fn destination(&self, tile: u32, td: u8) -> bool {
        if self.input.kind == 3 {
            return self.safe(tile, td, self.input.forbid90)
                && self.free(tile, td, self.input.forbid90);
        }
        let t = self.tile(tile, td);
        if self.input.kind == 2 {
            return t.flags & DEPOT != 0;
        }
        if self.dest_station != u16::MAX {
            return t.flags & STATION_RAIL != 0
                && t.station == self.dest_station
                && t.station_track == td & 7;
        }
        if self.any_depot {
            return t.flags & DEPOT != 0;
        }
        tile == self.dest_tile && self.dest_dirs & (1 << td) != 0
    }
    fn distance(&self, tile: u32, td: u8) -> i32 {
        let x1 = 2 * (tile % self.input.map_x) as i32 + [-1, 0, 1, 0][usize::from(exit(td))];
        let y1 = 2 * (tile / self.input.map_x) as i32 + [0, 1, 0, -1][usize::from(exit(td))];
        let x2 = 2 * (self.dest_tile % self.input.map_x) as i32;
        let y2 = 2 * (self.dest_tile / self.input.map_x) as i32;
        let dx = (x1 - x2).abs();
        let dy = (y1 - y2).abs();
        add(mul(dx.min(dy), 71), mul((dx - dy).abs() - 1, 50))
    }
    fn prune(&mut self, i: usize, n: &Node) {
        let mut node = Some(i);
        let mut intermediate = false;
        while let Some(index) = node {
            let current = if index == i { n } else { &self.arena[index] };
            if current.segment().reasons & CHOICE != 0 {
                break;
            }
            if self.intermediate == Some(index) {
                intermediate = true;
            }
            node = current.parent;
        }
        if intermediate {
            self.intermediate = node;
        }
    }
    fn signal_cost(&mut self, i: usize, n: &mut Node, tile: u32, td: u8, t: Tile) -> i32 {
        if t.flags & RAILWAY == 0 {
            return 0;
        }
        let mut segment = n.segment();
        if t.signal_against != 0 && t.signal_along == 0 && t.oneway != 0 {
            segment.reasons |= DEAD;
            n.set_segment(segment);
            return 0;
        }
        let mut cost = 0_i32;
        if t.signal_along != 0 {
            n.signal_type = t.signal_type;
            let lookahead = self
                .lookahead
                .get(usize::from(n.signals))
                .copied()
                .unwrap_or(0);
            if t.signal_green != 0 {
                n.red = false;
                if lookahead < 0 {
                    cost = cost.wrapping_sub(lookahead);
                }
            } else {
                if !pbs(t.signal_type)
                    && self.input.settings.firstred_eol != 0
                    && self.treat_first_red
                    && n.choice
                    && t.signal_against != 0
                    && n.signals == 0
                {
                    self.prune(i, n);
                    segment.reasons |= DEAD;
                    n.set_segment(segment);
                    self.stopped = true;
                    return -1;
                }
                n.red_type = t.signal_type;
                n.red = true;
                if !pbs(t.signal_type) && lookahead > 0 {
                    cost = add(cost, lookahead);
                }
                if n.signals == 0 {
                    cost = add(
                        cost,
                        match t.signal_type {
                            2 | 3 => self.input.settings.firstred_exit as i32,
                            0 | 1 => self.input.settings.firstred as i32,
                            _ => 0,
                        },
                    );
                }
            }
            n.signals = n.signals.wrapping_add(1);
            segment.signal_tile = tile;
            segment.signal_td = td;
            n.set_segment(segment);
        }
        if t.signal_against != 0
            && pbs(t.signal_type)
            && u32::from(n.signals) < self.input.settings.lookahead
        {
            cost = add(cost, self.input.settings.pbs_back as i32);
        }
        cost
    }
    fn reservation_cost(&self, n: &Node, tile: u32, td: u8, skipped: i32, t: Tile) -> i32 {
        if usize::from(n.signals) >= self.lookahead.len() / 2 || !pbs(n.signal_type) {
            return 0;
        }
        if t.flags & RAIL_STATION != 0 {
            let mut cur = tile;
            let mut left = skipped;
            while left >= 0 {
                if self.tile(cur, td).flags & PLATFORM_RESERVED != 0 {
                    return mul(self.input.settings.pbs_station as i32, add(skipped, 1));
                }
                left -= 1;
                cur = offset(cur, exit(td ^ 8), self.input.map_x);
            }
        }
        if overlaps(t.reserved, td) {
            let mut cost = self.input.settings.pbs_cross as i32;
            if !diagonal(td) {
                cost = mul(cost, 71) / 100;
            }
            return mul(cost, add(skipped, 1));
        }
        0
    }
    fn waypoint_cost(&self, tile: u32, td: u8) -> i32 {
        let mut cur = tile;
        let mut dir = td;
        let mut max_tiles = 20_u32;
        loop {
            // The original waypoint lookahead uses CFollowTrackRail even in No90.
            let f = self.follow(cur, dir, 0, self.train.compatible, false);
            if f.followed == 0 {
                break;
            }
            cur = f.tile;
            max_tiles -= 1;
            if cur == tile || max_tiles == 0 || f.dirs & f.dirs.wrapping_sub(1) != 0 {
                dir = INVALID_TD;
                break;
            }
            dir = first(f.dirs);
            if self.safe(cur, dir, self.input.forbid90) {
                break;
            }
        }
        if dir == INVALID_TD
            || !self.safe(cur, dir, self.input.forbid90)
            || !self.free(cur, dir, self.input.forbid90)
        {
            self.input.settings.lastred as i32
        } else {
            0
        }
    }
    fn calc_cost(&mut self, i: usize, n: &mut Node, mut follower: Follow) -> bool {
        let s = self.input.settings;
        let cached = n.segment().cost >= 0;
        let parent_cost = n.parent.map_or(0, |p| self.arena[p].cost);
        let (mut cur, mut td) = (n.tile, n.td);
        let mut cur_info = self.tile(cur, td);
        let (mut prev, mut prev_td) = n
            .parent
            .map_or((INVALID, INVALID_TD), |p| self.arena[p].last());
        let mut prev_info = if prev == INVALID {
            Tile::default()
        } else {
            self.tile(prev, prev_td)
        };
        let mut entry = 0_i32;
        let mut segment_cost = 0_i32;
        let mut extra = 0_i32;
        let mut reasons;
        let mut skip_entry = n.parent.is_none();
        loop {
            if !skip_entry {
                let mut transition = if self.input.forbid90 == 0
                    && CROSS[usize::from(prev_td & 7)] & (1 << td) != 0
                {
                    s.curve90 as i32
                } else if td != NEXT[usize::from(prev_td)] {
                    s.curve45 as i32
                } else {
                    0
                };
                if prev_info.flags & PLAIN != 0 && cur_info.flags & PLAIN != 0 {
                    let dir = exit(prev_td);
                    let t1 = prev_info.tracks & REACH_TRACKS[usize::from(dir ^ 2)];
                    let t2 = cur_info.tracks & REACH_TRACKS[usize::from(dir)];
                    if t1 & t1.wrapping_sub(1) != 0 && t2 & t2.wrapping_sub(1) != 0 {
                        transition = add(transition, s.doubleslip as i32);
                    }
                }
                if segment_cost == 0 {
                    entry = transition;
                    if cached {
                        let segment = n.segment();
                        segment_cost = segment.cost;
                        reasons = segment.reasons;
                        if segment.signal_tile != INVALID {
                            let signal = self.tile(segment.signal_tile, segment.signal_td);
                            n.red = signal.signal_green == 0;
                            if n.red {
                                n.red_type = signal.signal_type;
                            }
                        }
                        (cur, td) = n.last();
                        break;
                    }
                } else {
                    segment_cost = add(segment_cost, transition);
                }
            }
            skip_entry = false;
            segment_cost = add(
                segment_cost,
                if diagonal(td) {
                    add(
                        100,
                        if cur_info.flags & (ROAD | CROSSING) == ROAD | CROSSING {
                            s.crossing as i32
                        } else {
                            0
                        },
                    )
                } else {
                    71
                },
            );
            segment_cost = add(segment_cost, mul(100, follower.skipped));
            if diagonal(td) {
                let slope = if cur_info.flags & BRIDGE != 0 {
                    cur_info.tunnel_dir == exit(td) && cur_info.flat_ramp == 0
                } else {
                    cur_info.flags & TUNNEL == 0 && cur_info.uphill != 0
                };
                if slope {
                    segment_cost = add(segment_cost, s.slope as i32);
                }
            }
            segment_cost = add(segment_cost, self.signal_cost(i, n, cur, td, cur_info));
            segment_cost = add(
                segment_cost,
                self.reservation_cost(n, cur, td, follower.skipped, cur_info),
            );
            reasons = n.segment().reasons;
            if cur == prev {
                segment_cost = add(segment_cost, s.depot_reverse as i32);
            } else if cur_info.flags & DEPOT != 0 {
                reasons |= E_DEPOT;
            } else if cur_info.flags & WAYPOINT != 0 {
                if self.train.order == 6
                    && cur_info.station == self.train.order_destination
                    && self.train.complex_waypoint != 0
                {
                    extra = add(extra, self.waypoint_cost(cur, td));
                }
                reasons |= E_WAYPOINT;
            } else if follower.station != 0 {
                segment_cost = add(
                    segment_cost,
                    (s.station
                        .wrapping_mul((follower.skipped as u32).wrapping_add(1)))
                        as i32,
                );
                reasons |= E_STATION;
            } else if self.input.kind == 3
                && cur_info.flags & RAILWAY != 0
                && cur_info.signal_along != 0
                && !pbs(cur_info.signal_type)
            {
                reasons |= SAFE;
            }
            if usize::from(n.signals) < self.lookahead.len() {
                let speed = self.train.speed as i32;
                if follower.max_speed < speed {
                    // tiles_skipped is signed in CFollowTrackT; the source's
                    // int products precede division. Keep each width explicit.
                    extra = add(
                        extra,
                        mul(
                            mul(100, speed.wrapping_sub(follower.max_speed)),
                            add(4, follower.skipped),
                        ) / speed,
                    );
                }
                if follower.min_speed > speed {
                    extra = add(extra, mul(100, follower.min_speed.wrapping_sub(speed)));
                }
            }
            if self.max_cost > 0 && add(add(parent_cost, entry), segment_cost) > self.max_cost {
                reasons |= PATH_TOO_LONG;
            }
            follower = self.follow(cur, td, self.input.forbid90, self.compatible, false);
            if follower.followed == 0 {
                reasons |= if follower.error == 2 { RAILTYPE } else { DEAD };
                if self.input.kind == 3 && cur_info.flags & ONEWAY_BLOCKING == 0 {
                    reasons |= SAFE;
                }
                break;
            }
            if follower.dirs & follower.dirs.wrapping_sub(1) != 0 {
                reasons |= CHOICE;
                break;
            }
            let next_td = first(follower.dirs);
            let next_info = self.tile(follower.tile, next_td);
            if self.input.kind == 3 && next_info.flags & RAILWAY != 0 {
                if next_info.signal_along != 0 && pbs(next_info.signal_type) {
                    reasons |= SAFE;
                } else if next_info.signal_against != 0 && next_info.signal_type == 5 {
                    reasons |= SAFE | DEAD;
                    extra = add(extra, s.lastred_exit as i32);
                }
            }
            if next_info.railtype != cur_info.railtype {
                reasons |= RAILTYPE;
                break;
            }
            if follower.tile == n.tile && next_td == n.td {
                reasons |= LOOP;
                break;
            }
            if segment_cost > 10_000 && next_info.flags & RAILWAY != 0 {
                reasons |= TOO_LONG;
                break;
            }
            if reasons != 0 {
                break;
            }
            prev = cur;
            prev_td = td;
            prev_info = cur_info;
            cur = follower.tile;
            td = next_td;
            cur_info = next_info;
        }
        if reasons & PATH_TOO_LONG != 0 {
            return false;
        }
        let target = reasons & POSSIBLE_TARGET != 0 && self.destination(cur, td);
        if !cached {
            let mut segment = n.segment();
            segment.cost = segment_cost;
            segment.reasons = reasons & CACHED;
            segment.last_tile = cur;
            segment.last_td = td;
            n.set_segment(segment);
        }
        if !target && reasons & ABORT != 0 {
            return false;
        }
        if target {
            n.target = true;
            if n.red {
                if n.red_type == 2 {
                    extra = add(extra, s.lastred_exit as i32);
                } else if !pbs(n.red_type) {
                    extra = add(extra, s.lastred as i32);
                }
            }
            if reasons & E_STATION != 0 {
                let platform = self.platform_length(cur, exit(td) ^ 2);
                extra = extra.wrapping_sub(s.station.wrapping_mul(platform) as i32);
                let missing = self.train.length.div_ceil(16) as i32 - platform as i32;
                if missing < 0 {
                    extra = add(
                        extra,
                        add(
                            s.longer as i32,
                            mul(s.longer_tile as i32, missing.wrapping_neg()),
                        ),
                    );
                } else if missing > 0 {
                    extra = add(
                        extra,
                        add(s.shorter as i32, mul(s.shorter_tile as i32, missing)),
                    );
                }
            }
        }
        n.cost = add(add(add(parent_cost, entry), segment_cost), extra);
        true
    }
    fn add_node(&mut self, parent: usize, follower: Follow, td: u8, choice: bool) {
        let mut node = self.node(Some(parent), follower.tile, td, choice);
        let i = self.allocate(node.clone());
        if self.cache(&mut node) {
            self.hits = self.hits.wrapping_add(1);
        } else {
            self.calcs = self.calcs.wrapping_add(1);
        }
        if !self.calc_cost(i, &mut node, follower) {
            self.arena[i] = node;
            return;
        }
        node.estimate = if self.input.kind >= 2 || self.destination(node.last().0, node.last().1) {
            node.cost
        } else {
            add(node.cost, self.distance(node.last().0, node.last().1))
        };
        self.arena[i] = node;
        let n = &self.arena[i];
        let intermediate = (self.input.settings.max_nodes as i32) > 0
            && self.intermediate.is_none_or(|old| {
                self.arena[old].estimate.wrapping_sub(self.arena[old].cost)
                    > n.estimate.wrapping_sub(n.cost)
            });
        let k = (n.tile, n.td);
        if let Some(&old) = self.open.get(&k) {
            if n.estimate < self.arena[old].estimate {
                self.remove(old);
                self.arena[old] = self.arena[i].clone();
                self.include(old);
                if intermediate {
                    self.intermediate = Some(old);
                }
            }
            return;
        }
        if let Some(&old) = self.closed.get(&k) {
            if n.estimate < self.arena[old].estimate {
                unreachable!("original closed-node improvement NOT_REACHED");
            }
            return;
        }
        self.include(i);
        if intermediate {
            self.intermediate = Some(i);
        }
    }
    fn find_path(&mut self) -> bool {
        loop {
            self.rounds = self.rounds.wrapping_add(1);
            let Some(&i) = self.heap.get(1) else {
                break;
            };
            let (tile, td) = self.arena[i].last();
            if self.destination(tile, td) {
                self.best_dest = Some(i);
                break;
            }
            let f = self.follow(
                tile,
                td,
                self.input.forbid90,
                if self.input.kind == 3 {
                    self.compatible
                } else {
                    self.train.compatible
                },
                self.input.kind == 3,
            );
            if f.followed != 0 {
                let choice = f.dirs & f.dirs.wrapping_sub(1) != 0;
                let mut dirs = f.dirs;
                while dirs != 0 {
                    let td = first(dirs);
                    dirs &= dirs.wrapping_sub(1);
                    self.add_node(i, f, td, choice);
                }
            }
            let max = self.input.settings.max_nodes as i32;
            // ClosedCount is checked AFTER following, BEFORE closing the node.
            if max != 0 && self.closed.len() as i32 >= max {
                witness::hit(witness::RAIL_LIMIT_STOPS);
                let p = [0_u32];
                unsafe {
                    (self.leaves.debug)(self.input.context, 5, p.as_ptr(), 1);
                }
                break;
            }
            self.remove(i);
            self.closed
                .insert((self.arena[i].tile, self.arena[i].td), i);
        }
        let found = self.best_dest.is_some();
        let (cost, distance) = self.best_dest.map_or((-1, -1), |i| {
            (
                self.arena[i].cost,
                self.arena[i].estimate.wrapping_sub(self.arena[i].cost),
            )
        });
        let stats = [
            self.rounds,
            self.open.len() as u32,
            self.closed.len() as u32,
            self.hits,
            self.calcs,
            u32::from(found),
            cost as u32,
            distance as u32,
        ];
        unsafe {
            (self.leaves.debug)(self.input.context, 0, stats.as_ptr(), stats.len() as u32);
        }
        if witness::enabled() {
            witness::hit(
                [
                    witness::RAIL_CHOOSE,
                    witness::RAIL_REVERSE,
                    witness::RAIL_DEPOT,
                    witness::RAIL_SAFE,
                ][usize::from(self.input.kind)],
            );
            witness::hit(if self.disabled {
                witness::RAIL_UNCACHED
            } else {
                witness::RAIL_CACHED
            });
            witness::add(
                witness::RAIL_PARTIAL_PATHS,
                u64::from(!found && self.intermediate.is_some()),
            );
            witness::add(witness::RAIL_BOUNDED_DEPOT, u64::from(self.max_cost != 0));
            witness::add(witness::RAIL_FOUND, u64::from(found));
            witness::hit(if self.input.forbid90 != 0 {
                witness::RAIL_FORBID90
            } else {
                witness::RAIL_ALLOW90
            });
        }
        found
    }
    fn dump(&self, file: u32) {
        let mut words = vec![file, self.rounds];
        let mut identities = HashMap::new();
        for (i, n) in self.arena.iter().enumerate() {
            let segment = n.segment();
            let next = identities.len() as u32;
            let identity = *identities.entry(Rc::as_ptr(&n.segment)).or_insert(next);
            words.extend_from_slice(&[
                i as u32,
                n.tile,
                u32::from(n.td),
                n.parent.map_or(INVALID, |p| p as u32),
                n.cost as u32,
                n.estimate as u32,
                segment.last_tile,
                u32::from(segment.last_td),
                segment.cost as u32,
                segment.signal_tile,
                u32::from(segment.signal_td),
                u32::from(segment.reasons),
                u32::from(n.signals),
                u32::from(n.target) | (u32::from(n.choice) << 1) | (u32::from(n.red) << 2),
                u32::from(n.red_type),
                u32::from(n.signal_type),
                identity,
            ]);
        }
        unsafe {
            (self.leaves.debug)(self.input.context, 6, words.as_ptr(), words.len() as u32);
        }
    }
    fn best(&self) -> Option<usize> {
        self.best_dest.or(self.intermediate)
    }
    fn origin_of(&self, mut i: usize) -> usize {
        while let Some(p) = self.arena[i].parent {
            i = p;
        }
        i
    }
    fn init(&mut self) {
        match self.input.kind {
            0 | 2 => {
                let (mut tile, mut td) = (INVALID, INVALID_TD);
                unsafe {
                    (self.leaves.origin)(self.input.context, &raw mut tile, &raw mut td);
                }
                self.startup(tile, td, 0);
                if self.input.kind == 2 {
                    self.startup(self.train.rear_tile, self.train.rear_td ^ 8, 100_000);
                }
            }
            1 => {
                let mut tile = self.train.tile;
                let mut rear = self.train.rear_tile;
                let td = self.train.td;
                let rear_td = self.train.rear_td ^ 8;
                let mut penalty = 0_i32;
                if self.train.wormhole != 0 {
                    let t = self.tile(tile, td);
                    if exit(td) == t.tunnel_dir {
                        tile = t.other_end;
                    }
                    penalty = penalty
                        .wrapping_sub(mul(self.manhattan(self.train.virtual_tile, tile), 100));
                }
                if self.train.rear_wormhole != 0 {
                    let t = self.tile(rear, rear_td);
                    if exit(rear_td) == t.tunnel_dir {
                        rear = t.other_end;
                    }
                    penalty = add(
                        penalty,
                        mul(self.manhattan(self.train.rear_virtual_tile, rear), 100),
                    );
                }
                if penalty == 0 {
                    penalty = 1;
                }
                self.startup(tile, td, 0);
                self.startup(rear, rear_td, penalty);
            }
            _ => self.startup(self.input.tile, self.input.td, 0),
        }
        self.set_destination();
    }
    fn manhattan(&self, a: u32, b: u32) -> i32 {
        ((a % self.input.map_x).abs_diff(b % self.input.map_x)
            + (a / self.input.map_x).abs_diff(b / self.input.map_x)) as i32
    }
    fn write(&self, tile: u32, td: u8, op: u8) -> bool {
        unsafe { (self.leaves.write)(tile, td, op) != 0 }
    }
    fn output(&self, mask: u8, result: &Step) {
        unsafe {
            (self.leaves.output)(self.input.context, mask, result);
        }
    }
}

struct TileCursor {
    node: usize,
    tile: u32,
    td: u8,
    stop: bool,
}
impl TileCursor {
    fn new(search: &Search, node: usize) -> Self {
        let n = &search.arena[node];
        Self {
            node,
            tile: n.tile,
            td: n.td,
            stop: false,
        }
    }
    fn advance(&mut self, search: &Search) -> bool {
        if self.stop || (self.tile, self.td) == search.arena[self.node].last() {
            return false;
        }
        let f = search.follow(
            self.tile,
            self.td,
            search.input.forbid90,
            search.compatible,
            false,
        );
        if f.followed == 0 {
            // IterateTiles calls the functor once more on the same current tile
            // when Follow fails; the original breaks, then calls it at exit.
            self.stop = true;
        } else {
            self.tile = f.tile;
            self.td = first(f.dirs);
        }
        true
    }
}
struct Reservation {
    target_node: usize,
    target_tile: u32,
    target_td: u8,
    origin: u32,
    fail_tile: u32,
    fail_td: u8,
    signals: Vec<(u32, u8)>,
    cursor: TileCursor,
    resume_callback: bool,
    resume_continue: bool,
    callback_tile: u32,
    initialized: bool,
    output_target: bool,
}
impl Reservation {
    fn new(search: &Search, best: usize, output_target: bool) -> Self {
        let (mut target_tile, mut target_td) = search.arena[best].last();
        let mut target_node = best;
        let mut node = best;
        while let Some(parent) = search.arena[node].parent {
            if search.arena[parent].signals < 2 {
                let mut cursor = TileCursor::new(search, node);
                loop {
                    if search.safe(cursor.tile, cursor.td, search.input.forbid90) {
                        target_node = node;
                        target_tile = cursor.tile;
                        target_td = cursor.td;
                        break;
                    }
                    if !cursor.advance(search) {
                        break;
                    }
                }
            }
            node = parent;
        }
        Self {
            target_node,
            target_tile,
            target_td,
            origin: search.arena[node].last().0,
            fail_tile: INVALID,
            fail_td: INVALID_TD,
            signals: Vec::new(),
            cursor: TileCursor::new(search, target_node),
            resume_callback: false,
            resume_continue: false,
            callback_tile: INVALID,
            initialized: false,
            output_target,
        }
    }
    fn target_output(&self, okay: bool) -> Step {
        let mut out = empty_step();
        out.target_tile = self.target_tile;
        out.target_td = self.target_td;
        out.target_okay = u8::from(okay);
        out
    }
    fn rollback_tile(&self, search: &Search, mut tile: u32, td: u8, fail: u32) -> bool {
        let info = search.tile(tile, td);
        if info.flags & RAIL_STATION != 0 {
            let start = tile;
            while (tile != fail || td != self.fail_td)
                && unsafe { (search.leaves.compatible_station)(tile, start) != 0 }
            {
                search.write(tile, td, 4);
                tile = offset(tile, exit(td ^ 8), search.input.map_x);
            }
        } else if tile != fail || td != self.fail_td {
            search.write(tile, td, 2);
        }
        (tile != self.target_tile || td != self.target_td) && (tile != fail || td != self.fail_td)
    }
    fn rollback(&self, search: &Search, failed_node: usize) {
        let mut fail_node = self.target_node;
        loop {
            let fail = if fail_node == failed_node {
                self.fail_tile
            } else {
                INVALID
            };
            let mut cursor = TileCursor::new(search, fail_node);
            loop {
                if !self.rollback_tile(search, cursor.tile, cursor.td, fail)
                    || !cursor.advance(search)
                {
                    break;
                }
            }
            if fail_node == failed_node {
                break;
            }
            let Some(parent) = search.arena[fail_node].parent else {
                break;
            };
            fail_node = parent;
        }
        // The original restores in insertion order, without dirtying again.
        for &(tile, td) in &self.signals {
            search.write(tile, td, 6);
            witness::hit(witness::RAIL_SIGNALS_RESTORED);
            let p = [2_u32];
            unsafe {
                (search.leaves.debug)(search.input.context, 5, p.as_ptr(), 1);
            }
        }
    }
    // None means station callback pending. Some(bool) completes reservation.
    fn step(&mut self, search: &Search) -> Option<bool> {
        if !self.initialized {
            self.initialized = true;
            if self.output_target {
                search.output(8, &self.target_output(false));
            }
            // TryReservePath uses the default forbid90=false free-position query.
            witness::hit(witness::RAIL_RESERVE_ATTEMPTS);
            if !search.free(self.target_tile, self.target_td, 0) {
                witness::hit(witness::RAIL_TARGET_BUSY);
                return Some(false);
            }
        }
        loop {
            if search.arena[self.cursor.node].parent.is_none() {
                if self.output_target {
                    search.output(8, &self.target_output(true));
                }
                if search.global_for(&search.arena[self.target_node]) {
                    invalidate();
                }
                witness::hit(witness::RAIL_RESERVE_SUCCESS);
                return Some(true);
            }
            let (tile, td) = (self.cursor.tile, self.cursor.td);
            let mut continue_segment = tile != self.target_tile || td != self.target_td;
            if self.resume_callback {
                self.resume_callback = false;
                continue_segment = self.resume_continue;
            } else {
                let info = search.tile(tile, td);
                if info.flags & RAIL_STATION != 0 {
                    let mut platform_tile = tile;
                    loop {
                        if search.tile(platform_tile, td).flags & PLATFORM_RESERVED != 0 {
                            // ReserveSingleTrack historically records platform
                            // failure, yet returns the TARGET test (not false).
                            self.fail_tile = platform_tile;
                            self.fail_td = td;
                            continue_segment =
                                platform_tile != self.target_tile || td != self.target_td;
                            break;
                        }
                        search.write(platform_tile, td, 3);
                        platform_tile = offset(platform_tile, exit(td ^ 8), search.input.map_x);
                        if unsafe { (search.leaves.compatible_station)(platform_tile, tile) == 0 }
                            || platform_tile == self.origin
                        {
                            self.resume_callback = true;
                            self.resume_continue =
                                platform_tile != self.target_tile || td != self.target_td;
                            self.callback_tile = tile;
                            return None;
                        }
                    }
                } else {
                    if !search.write(tile, td, 1) {
                        self.fail_tile = tile;
                        self.fail_td = td;
                        continue_segment = false;
                    } else {
                        let reverse = search.tile(tile, td ^ 8);
                        if reverse.signal_along != 0
                            && pbs(reverse.signal_type)
                            && reverse.signal_green != 0
                        {
                            self.signals.push((tile, td ^ 8));
                            search.write(tile, td ^ 8, 5);
                            witness::hit(witness::RAIL_SIGNALS_RED);
                            let p = [1_u32];
                            unsafe {
                                (search.leaves.debug)(search.input.context, 5, p.as_ptr(), 1);
                            }
                        }
                        if info.flags & WAYPOINT != 0 {
                            self.resume_callback = true;
                            self.resume_continue = continue_segment;
                            self.callback_tile = tile;
                            return None;
                        }
                    }
                }
            }
            if continue_segment && self.cursor.advance(search) {
                continue;
            }
            if self.fail_tile != INVALID {
                self.rollback(search, self.cursor.node);
                witness::hit(witness::RAIL_ROLLBACK);
                return Some(false);
            }
            let parent = search.arena[self.cursor.node].parent.unwrap();
            self.cursor = TileCursor::new(search, parent);
        }
    }
}
fn empty_step() -> Step {
    Step {
        tile: INVALID,
        destination: INVALID,
        target_tile: INVALID,
        best_length: u32::MAX,
        action: 0,
        td: INVALID_TD,
        found: 0,
        reverse: 0,
        value: 0,
        target_td: INVALID_TD,
        target_okay: 0,
    }
}
pub struct Owner {
    input: Input,
    leaves: Leaves,
    search: Option<Search>,
    first_search: Option<Search>,
    reservation: Option<Reservation>,
    stage: u8,
    first: Option<Step>,
    result: Step,
    done: bool,
}
impl Owner {
    fn new(input: Input, leaves: Leaves) -> Self {
        Self {
            input,
            leaves,
            search: None,
            first_search: None,
            reservation: None,
            stage: 0,
            first: None,
            result: empty_step(),
            done: false,
        }
    }
    fn prepare(&mut self) {
        let disabled = self.stage != 0 || (self.input.kind == 2 && self.input.max_cost != 0);
        let mut search = Search::new(self.input, self.leaves, disabled);
        let actual_outputs = self.input.desync < 2 || self.stage != 0;
        if self.input.kind == 0 && actual_outputs {
            search.output(3, &empty_step());
        }
        search.init();
        let found = search.find_path();
        let mut result = empty_step();
        result.found = u8::from(found);
        if self.input.kind == 0 {
            search.output(4, &result);
        }
        match self.input.kind {
            0 => {
                if let Some(best) = search.best() {
                    // Safe-target traversal occurs even when reservation wasn't
                    // requested, before extracting the first path trackdir.
                    let reservation = Reservation::new(&search, best, actual_outputs);
                    let mut node = best;
                    let mut prev = None;
                    while let Some(parent) = search.arena[node].parent {
                        prev = Some(node);
                        node = parent;
                    }
                    if let Some(next) = prev {
                        result.td = search.arena[next].td;
                        if self.input.reserve != 0 && found && actual_outputs {
                            result.destination = search.arena[best].last().0;
                            search.output(2, &result);
                            self.reservation = Some(reservation);
                        }
                        result.found |= u8::from(search.stopped);
                    }
                    // Original early return when best has no parent skips the
                    // stopped_on_first_two_way_signal OR as well.
                } else {
                    result.found |= u8::from(search.stopped);
                }
                if self.reservation.is_none() {
                    search.output(4, &result);
                }
            }
            1 => {
                if found {
                    let origin = search.origin_of(search.best().unwrap());
                    result.value = u8::from(search.arena[origin].cost != 0);
                }
            }
            2 => {
                if found {
                    let best = search.best().unwrap();
                    let origin = search.origin_of(best);
                    result.tile = search.arena[best].last().0;
                    result.best_length = search.arena[best].cost as u32;
                    result.reverse = u8::from(search.arena[origin].cost != 0);
                }
            }
            _ => {
                result.value = u8::from(found);
                if found {
                    let reservation = Reservation::new(&search, search.best().unwrap(), false);
                    if self.input.desync < 2 || self.stage != 0 {
                        self.reservation = Some(reservation);
                    }
                }
            }
        }
        self.result = result;
        self.search = Some(search);
    }
    fn complete_stage(&mut self) {
        if self.input.desync >= 2 && self.stage == 0 {
            self.first = Some(self.result);
            self.first_search = self.search.take();
            self.stage = 1;
            return;
        }
        if let Some(first) = self.first {
            let mismatch = match self.input.kind {
                0 => first.td != self.result.td,
                1 | 3 => first.value != self.result.value,
                _ => first.tile != self.result.tile || first.reverse != self.result.reverse,
            };
            if mismatch {
                let data = [
                    u32::from(self.input.kind),
                    if self.input.kind == 0 {
                        u32::from(first.td)
                    } else if self.input.kind == 2 {
                        u32::from(first.tile != INVALID)
                    } else {
                        u32::from(first.value)
                    },
                    if self.input.kind == 0 {
                        u32::from(self.result.td)
                    } else if self.input.kind == 2 {
                        u32::from(self.result.tile != INVALID)
                    } else {
                        u32::from(self.result.value)
                    },
                ];
                unsafe {
                    (self.leaves.debug)(self.input.context, 3, data.as_ptr(), 3);
                }
                self.first_search.as_ref().unwrap().dump(1);
                self.search.as_ref().unwrap().dump(2);
            }
            if self.input.kind == 0 {
                self.result.td = first.td;
            }
            if self.input.kind == 1 || self.input.kind == 2 {
                self.result = first;
            }
        }
        self.done = true;
    }
    fn step(&mut self) -> Step {
        if self.done {
            return self.result;
        }
        loop {
            if self.search.is_none() {
                self.prepare();
            }
            if let Some(reservation) = &mut self.reservation {
                let search = self.search.as_ref().unwrap();
                let Some(okay) = reservation.step(search) else {
                    let mut callback = empty_step();
                    callback.action = 1;
                    callback.tile = reservation.callback_tile;
                    return callback;
                };
                if self.input.kind == 3 {
                    self.result.value = u8::from(okay);
                }
                if self.input.kind == 0 {
                    search.output(4, &self.result);
                }
                self.reservation = None;
            }
            self.complete_stage();
            if self.done {
                return self.result;
            }
        }
    }
}
/// # Safety
/// Input/leaves are initialized readable records. Their opaque context lives
/// until destroy, all synchronous leaves obey the header contract. Entry occurs
/// on the serialized simulation thread; no concurrent cache access is allowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_new(
    input: *const Input,
    leaves: *const Leaves,
) -> *mut Owner {
    Box::into_raw(Box::new(Owner::new(unsafe { *input }, unsafe { *leaves })))
}
/// # Safety
/// Owner is uniquely accessed live storage from new; callbacks from the previous
/// returned action have completed. No owner borrow survives this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_step(owner: *mut Owner) -> Step {
    let owner = unsafe { &mut *owner };
    let step = owner.step();
    if step.action == 0 {
        let chosen = match owner.input.kind {
            1 | 3 => step.value != 0,
            2 => step.tile != INVALID,
            _ => false,
        };
        if chosen {
            witness::hit(
                [
                    witness::RAIL_INVALIDATIONS,
                    witness::RAIL_REVERSE_CHOSEN,
                    witness::RAIL_DEPOT_FOUND,
                    witness::RAIL_SAFE_FOUND,
                ][usize::from(owner.input.kind)],
            );
        }
    }
    step
}
/// # Safety
/// Destroy exactly once the live pointer returned by new, with no active call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_destroy(owner: *mut Owner) {
    drop(unsafe { Box::from_raw(owner) });
}
/// Serialized game-thread notification; increment only, lazy bank invalidation.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_invalidate() {
    invalidate();
    witness::hit(witness::RAIL_INVALIDATIONS);
}
