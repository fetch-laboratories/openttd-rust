/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete ship YAPF searches and canonical path storage.
// Equal estimates deliberately follow CBinaryHeapT rather than a stable queue.
// Arena indices preserve parent identity when an existing open node is replaced.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::too_many_lines
)]
use crate::node_hash::NodeMap;
use crate::services::Services;
use crate::water_regions::Patch;
use std::cell::UnsafeCell;
use std::ffi::c_void;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Input {
    pub map_x: u32,
    pub map_y: u32,
    pub tile: u32,
    pub dest_tile: u32,
    pub curve90: i32,
    pub curve45: i32,
    pub max_speed: u32,
    pub dest_dirs: u16,
    pub reverse_dirs: u16,
    pub trackdir: u8,
    pub ocean_frac: u8,
    pub canal_frac: u8,
    pub station: u8,
    pub unit_number: u32,
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
pub struct Tile {
    pub ships: u32,
    pub docking: u8,
    pub sea: u8,
    pub lock_middle: u8,
    pub destination: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Result {
    pub direction: u8,
    pub found: u8,
    pub origin: u8,
    pub stats: [u32; 13],
}
#[repr(C)]
pub struct Destination {
    pub tile: u32,
    pub station: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub destination: unsafe extern "C" fn(*const c_void) -> Destination,
    pub follow: unsafe extern "C" fn(*const c_void, u32, u8) -> Follow,
    pub tile: unsafe extern "C" fn(*const c_void, u32, u8) -> Tile,
    pub patch: unsafe extern "C" fn(u32) -> Patch,
    pub visit_new: unsafe extern "C" fn(Patch) -> *mut c_void,
    pub visit_next: unsafe extern "C" fn(*mut c_void, Patch, *mut Patch) -> u8,
    pub visit_destroy: unsafe extern "C" fn(*mut c_void),
    pub debug: unsafe extern "C" fn(u32, u8, u8, u32, u32, u32, i32, i32),
}
#[derive(Clone, Default)]
pub struct Path(Vec<u8>);
pub struct RegionPath(Vec<Patch>);
const INVALID: u8 = 255;
const NEXT: [u8; 14] = [0, 1, 3, 2, 5, 4, 255, 255, 8, 9, 11, 10, 13, 12];
#[derive(Clone, Copy)]
struct Node {
    key: u32,
    tile: u32,
    td: u8,
    patch: Patch,
    parent: Option<usize>,
    cost: i32,
    estimate: i32,
}
// Maps are lookup-only (never iterated): bucket/iteration order never selects a
// node, so the fixed NodeMap hasher cannot change results. Keys are the original tile/exit-dir or CalculateWaterRegionPatchHash bits.
/// Only copied nodes and indices persist; no world pointer or input is cached.
#[derive(Default)]
struct Scratch {
    arena: Vec<Node>,
    open: NodeMap<u32, usize>,
    closed: NodeMap<u32, usize>,
    heap: Vec<usize>,
}
struct SerializedScratch(UnsafeCell<Option<Scratch>>);
// SAFETY: Production entries hold the native game-state mutex. Slot borrows end
// before callbacks; an active search owns its storage, including during reentry.
// Native heap probes use local storage without accessing this process-wide slot.
unsafe impl Sync for SerializedScratch {}
static SCRATCH: SerializedScratch = SerializedScratch(UnsafeCell::new(None));

struct Search {
    arena: Vec<Node>,
    open: NodeMap<u32, usize>,
    closed: NodeMap<u32, usize>,
    heap: Vec<usize>,
    limit: i32,
    reuse_scratch: bool,
}
impl Search {
    fn new(limit: i32) -> Self {
        Self::with_scratch(limit, Scratch::default(), false)
    }
    /// Caller serializes production entries with the native game-state mutex.
    unsafe fn reused(limit: i32) -> Self {
        // SAFETY: Move ownership out before any callback; nested calls get their
        // own storage. First use allocates only when the search grows.
        let scratch = unsafe { (*SCRATCH.0.get()).take().unwrap_or_default() };
        Self::with_scratch(limit, scratch, true)
    }
    fn with_scratch(limit: i32, mut scratch: Scratch, reuse_scratch: bool) -> Self {
        scratch.arena.clear();
        scratch.open.clear();
        scratch.closed.clear();
        scratch.heap.clear();
        scratch.heap.push(usize::MAX);
        Self {
            arena: scratch.arena,
            open: scratch.open,
            closed: scratch.closed,
            heap: scratch.heap,
            limit,
            reuse_scratch,
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
    fn add(&mut self, n: Node) {
        if let Some(&old) = self.open.get(&n.key) {
            if n.estimate < self.arena[old].estimate {
                self.remove(old);
                self.arena[old] = n;
                self.include(old);
            }
            return;
        }
        if let Some(&old) = self.closed.get(&n.key) {
            if n.estimate < self.arena[old].estimate {
                unreachable!("original closed-node improvement NOT_REACHED");
            }
            return;
        }
        let index = self.arena.len();
        self.arena.push(n);
        self.open.insert(n.key, index);
        self.include(index);
    }
    fn startup(&mut self, n: Node) {
        if !self.open.contains_key(&n.key) {
            self.add(n);
        }
    }
    fn best(&self) -> Option<usize> {
        self.heap.get(1).copied()
    }
    fn report(&self, input: &Input, leaves: &Leaves, kind: u8, rounds: u32, found: Option<usize>) {
        let (cost, distance) = found.map_or((-1, -1), |index| {
            let node = self.arena[index];
            (node.cost, node.estimate.wrapping_sub(node.cost))
        });
        // SAFETY: the diagnostic leaf only formats copied scalars. It cannot
        // reenter this owner; environmental output failures terminate in C++.
        unsafe {
            (leaves.debug)(
                input.unit_number,
                kind,
                u8::from(found.is_some()),
                rounds,
                self.open.len() as u32,
                self.closed.len() as u32,
                cost,
                distance,
            );
        }
    }
    fn close(&mut self, index: usize) -> bool {
        if self.limit != 0 && self.closed.len() as i32 >= self.limit {
            return false;
        }
        self.remove(index);
        let key = self.arena[index].key;
        self.open.remove(&key);
        self.closed.insert(key, index);
        true
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
            // SAFETY: Only serialized entries enable reuse. No slot borrow spans
            // callbacks; an outer search may replace a nested call's cache.
            unsafe { *SCRATCH.0.get() = Some(scratch) };
        }
    }
}
fn blank(key: u32, tile: u32, td: u8, patch: Patch, parent: Option<usize>) -> Node {
    Node {
        key,
        tile,
        td,
        patch,
        parent,
        cost: 0,
        estimate: 0,
    }
}
fn patch_hash(input: &Input, p: Patch) -> u32 {
    ((input.map_x / 16)
        .wrapping_mul(p.y as u32)
        .wrapping_add(p.x as u32)
        << 8)
        | u32::from(p.label)
}
fn distance(a: Patch, b: Patch) -> i32 {
    (a.x.wrapping_sub(b.x).abs() + a.y.wrapping_sub(b.y).abs()).wrapping_mul(100)
}
fn diag(n: Node, parent: Node) -> u8 {
    let dx = n.patch.x - parent.patch.x;
    let dy = n.patch.y - parent.patch.y;
    if dx > 0 && dy == 0 {
        2
    } else if dx < 0 && dy == 0 {
        0
    } else if dx == 0 && dy > 0 {
        1
    } else if dx == 0 && dy < 0 {
        3
    } else {
        255
    }
}
fn region_path(
    input: &Input,
    leaves: &Leaves,
    start: u32,
    max_length: i32,
    origins: &[u32],
    stats: &mut [u32; 13],
) -> Vec<Patch> {
    // SAFETY: copied world leaves do not reenter this search. Water cache is a distinct owner.
    let start_patch = unsafe { (leaves.patch)(start) };
    let limit = ((input.map_x.wrapping_mul(input.map_y).wrapping_mul(4) as i32) / 256).min(65536);
    // SAFETY: Production calls hold the native game-state mutex.
    let mut search = unsafe { Search::reused(limit) };
    for &tile in origins {
        let patch = unsafe { (leaves.patch)(tile) };
        if patch.label != 0 {
            search.startup(blank(patch_hash(input, patch), 0, 0, patch, None));
        }
    }
    let mut path = vec![start_patch];
    let goal = patch_hash(input, start_patch);
    if search.open.contains_key(&goal) {
        return path;
    }
    let mut rounds = 0_u32;
    let best = loop {
        rounds = rounds.wrapping_add(1);
        let Some(index) = search.best() else {
            search.report(input, leaves, 0, rounds, None);
            return Vec::new();
        };
        let node = search.arena[index];
        if node.key == goal {
            search.report(input, leaves, 0, rounds, Some(index));
            break index;
        }
        stats[0] += 1;
        let cursor = unsafe { (leaves.visit_new)(node.patch) };
        loop {
            let mut patch = Patch {
                x: 0,
                y: 0,
                label: 0,
            };
            if unsafe { (leaves.visit_next)(cursor, node.patch, &raw mut patch) } == 0 {
                break;
            }
            let mut n = blank(patch_hash(input, patch), 0, 0, patch, Some(index));
            n.cost = node.cost.wrapping_add(distance(patch, node.patch));
            if let Some(grandparent) = node.parent {
                let a = diag(node, search.arena[grandparent]);
                let b = diag(n, node);
                let diff = a.wrapping_sub(b) & 3;
                if diff != 1 && diff != 3 {
                    n.cost = n.cost.wrapping_add(1);
                }
            }
            n.estimate = if n.key == goal {
                n.cost
            } else {
                n.cost.wrapping_add(distance(patch, start_patch))
            };
            search.add(n);
        }
        unsafe { (leaves.visit_destroy)(cursor) };
        if !search.close(index) {
            stats[11] += 1;
            search.report(input, leaves, 0, rounds, None);
            return Vec::new();
        }
    };
    let mut node = Some(best);
    for _ in 0..max_length - 1 {
        if let Some(index) = node {
            node = search.arena[index].parent;
            if let Some(parent) = node {
                path.push(search.arena[parent].patch);
            }
        }
    }
    path
}
fn coords(input: &Input, tile: u32) -> (i32, i32) {
    ((tile % input.map_x) as i32, (tile / input.map_x) as i32)
}
fn region(input: &Input, tile: u32) -> (i32, i32) {
    let (x, y) = coords(input, tile);
    (x / 16, y / 16)
}
fn matches(
    input: &Input,
    leaves: &Leaves,
    context: *const c_void,
    node: Node,
    intermediate: Option<Patch>,
) -> bool {
    if let Some(patch) = intermediate {
        return region(input, node.tile) == (patch.x, patch.y)
            && unsafe { (leaves.patch)(node.tile) } == patch;
    }
    if input.station != 0 {
        return unsafe { (leaves.tile)(context, node.tile, 0) }.destination != 0;
    }
    node.tile == input.dest_tile && input.dest_dirs & (1 << node.td) != 0
}
fn octile(input: &Input, tile: u32, td: u8, destination: u32) -> i32 {
    let (x, y) = coords(input, tile);
    let (dx, dy) = coords(input, destination);
    let exit = usize::from(crate::track_func::trackdir_to_exitdir(td));
    let x = (2 * x + [-1, 0, 1, 0][exit] - 2 * dx).abs();
    let y = (2 * y + [0, 1, 0, -1][exit] - 2 * dy).abs();
    x.min(y) * 71 + ((x - y).abs() - 1) * 50
}
fn preferred(input: &Input, tile: u32, td: u8) -> bool {
    let (x, y) = coords(input, tile);
    let x = x & 1 != 0;
    let y = y & 1 != 0;
    match td {
        0 => y,
        8 => !y,
        9 => x,
        1 => !x,
        _ => (x ^ y) ^ (((1 << 13) | (1 << 4) | (1 << 10) | (1 << 3)) & (1 << td) != 0),
    }
}
fn low_search(
    services: &Services,
    input: &Input,
    leaves: &Leaves,
    context: *const c_void,
    dirs: u16,
    path: &[Patch],
    restricted: bool,
    stats: &mut [u32; 13],
) -> Option<(Search, usize, u32)> {
    let mut observations = *input;
    // Destination observations occur only when the region search succeeds.
    let dest = unsafe { (leaves.destination)(context) };
    observations.dest_tile = dest.tile;
    observations.dest_dirs = if dest.station {
        u16::MAX
    } else {
        crate::track_func::track_status_to_trackdir_bits(crate::movement::track_status(
            services,
            dest.tile,
            crate::transport_type::TRANSPORT_WATER,
            0,
            crate::direction_type::INVALID_DIAGDIR,
        ))
    };
    let input = &observations;
    let intermediate = if path.len() >= 5 {
        path.last().copied()
    } else {
        None
    };
    // SAFETY: Production calls hold the native game-state mutex.
    let mut search = unsafe { Search::reused(5120) };
    let mut dirs = dirs;
    while dirs != 0 {
        let td = dirs.trailing_zeros() as u8;
        dirs &= dirs - 1;
        let key = (input.tile << 2) | u32::from(crate::track_func::trackdir_to_exitdir(td));
        search.startup(blank(
            key,
            input.tile,
            td,
            Patch {
                x: 0,
                y: 0,
                label: 0,
            },
            None,
        ));
    }
    let mut rounds = 0_u32;
    loop {
        rounds = rounds.wrapping_add(1);
        let Some(index) = search.best() else {
            search.report(input, leaves, 1, rounds, None);
            return None;
        };
        let parent = search.arena[index];
        if matches(input, leaves, context, parent, intermediate) {
            search.report(input, leaves, 1, rounds, Some(index));
            return Some((search, index, input.dest_tile));
        }
        stats[1] += 1;
        let f = unsafe { (leaves.follow)(context, parent.tile, parent.td) };
        if f.followed != 0
            && (!restricted || path.iter().any(|p| (p.x, p.y) == region(input, f.tile)))
        {
            let mut dirs = f.dirs;
            while dirs != 0 {
                let td = dirs.trailing_zeros() as u8;
                dirs &= dirs - 1;
                let mut n = blank(
                    (f.tile << 2) | u32::from(crate::track_func::trackdir_to_exitdir(td)),
                    f.tile,
                    td,
                    Patch {
                        x: 0,
                        y: 0,
                        label: 0,
                    },
                    Some(index),
                );
                let mut c: i32 = if td & 7 < 2 { 100 } else { 71 };
                c = c.wrapping_add(
                    if crate::track_func::trackdir_crosses_trackdirs(parent.td) & (1 << td) != 0 {
                        input.curve90
                    } else if td != NEXT[usize::from(parent.td)] {
                        input.curve45
                    } else {
                        0
                    },
                );
                let tile = unsafe { (leaves.tile)(context, n.tile, 1) };
                if tile.docking != 0 {
                    c = c.wrapping_add(tile.ships.wrapping_mul(300) as i32);
                }
                if !preferred(input, n.tile, td) {
                    c = c.wrapping_add(100);
                }
                c = c.wrapping_add(100_i32.wrapping_mul(f.skipped));
                let frac = i32::from(if tile.sea != 0 {
                    input.ocean_frac
                } else {
                    input.canal_frac
                });
                if frac > 0 {
                    c = c.wrapping_add(
                        100_i32
                            .wrapping_mul(1_i32.wrapping_add(f.skipped))
                            .wrapping_mul(frac)
                            / (256 - frac),
                    );
                }
                if tile.lock_middle != 0 {
                    c = c.wrapping_add(
                        800_u32
                            .wrapping_mul(
                                input
                                    .max_speed
                                    .wrapping_mul(256 - u32::from(input.canal_frac))
                                    / 256,
                            )
                            .wrapping_div(128) as i32,
                    );
                }
                n.cost = parent.cost.wrapping_add(c);
                let destination = intermediate.map_or(input.dest_tile, |p| {
                    ((p.y as u32 * 16 + 8) * input.map_x) + (p.x as u32 * 16 + 8)
                });
                n.estimate = if matches(input, leaves, context, n, intermediate) {
                    n.cost
                } else {
                    n.cost.wrapping_add(octile(input, n.tile, td, destination))
                };
                search.add(n);
            }
        }
        if !search.close(index) {
            stats[3] += 1;
            search.report(input, leaves, 1, rounds, None);
            return None;
        }
    }
}
fn random_dir(services: &Services, mut dirs: u16) -> u8 {
    // RandomRange consumes one complete shared word, including for a zero maximum.
    let strip = services.random_range(dirs.count_ones());
    for _ in 0..strip {
        dirs &= dirs - 1;
    }
    if dirs == 0 {
        INVALID
    } else {
        dirs.trailing_zeros() as u8
    }
}
fn random_path(
    cache: &mut Path,
    input: &Input,
    leaves: &Leaves,
    services: &Services,
    context: *const c_void,
    length: i32,
    stats: &mut [u32; 13],
) -> u8 {
    let mut tile = input.tile;
    let mut td = input.trackdir;
    for _ in 0..length {
        let f = unsafe { (leaves.follow)(context, tile, td) };
        tile = f.tile;
        if f.followed == 0 {
            break;
        }
        let mut dirs = f.dirs;
        let straight = dirs & !crate::track_func::trackdir_crosses_trackdirs(td);
        if straight != 0 {
            dirs = straight;
        }
        stats[8] += 1;
        td = random_dir(services, dirs);
        cache.0.push(td);
    }
    if cache.0.is_empty() {
        return INVALID;
    }
    cache.0.reverse();
    cache.0.pop().unwrap()
}
fn choose(
    cache: &mut Path,
    input: &Input,
    leaves: &Leaves,
    services: &Services,
    context: *const c_void,
    start: u32,
    forward: u16,
    reverse: u16,
    origins: &[u32],
) -> Result {
    let mut result = Result {
        direction: INVALID,
        found: 0,
        origin: INVALID,
        stats: [0; 13],
    };
    let path = region_path(input, leaves, start, 5, origins, &mut result.stats);
    if path.len() >= 5 {
        result.stats[4] += 1;
    }
    if path.is_empty() {
        result.stats[7] += 1;
        result.direction = random_path(
            cache,
            input,
            leaves,
            services,
            context,
            8,
            &mut result.stats,
        );
        return result;
    }
    for attempt in 0..2 {
        if attempt > 0 {
            result.stats[2] += 1;
        }
        let Some((search, mut index, closest)) = low_search(
            services,
            input,
            leaves,
            context,
            forward | reverse,
            &path,
            attempt > 0,
            &mut result.stats,
        ) else {
            if attempt == 0 {
                continue;
            }
            result.stats[7] += 1;
            result.direction = random_path(
                cache,
                input,
                leaves,
                services,
                context,
                8,
                &mut result.stats,
            );
            return result;
        };
        result.found = 1;
        if input.station != 0 && path.len() < 5 && search.arena[index].tile != closest {
            result.stats[12] += 1;
        }
        let end = unsafe { (leaves.patch)(search.arena[index].tile) };
        while let Some(parent) = search.arena[index].parent {
            let node = search.arena[index];
            let patch = unsafe { (leaves.patch)(node.tile) };
            if (path.len() < 5 && patch != end) || !path.contains(&patch) || patch == path[0] {
                cache.0.push(node.td);
            } else {
                result.stats[5] += 1;
                cache.0.clear();
            }
            index = parent;
        }
        result.origin = search.arena[index].td;
        if forward & (1 << result.origin) == 0 {
            result.stats[9] += 1;
            cache.0.clear();
            return result;
        }
        if cache.0.is_empty() {
            result.direction = random_path(
                cache,
                input,
                leaves,
                services,
                context,
                1,
                &mut result.stats,
            );
            return result;
        }
        result.direction = cache.0.pop().unwrap();
        if path[0] == end {
            result.stats[6] += 1;
            cache.0.clear();
        }
        return result;
    }
    unreachable!()
}
// SAFETY for all exports: nonnull owner/input/table pointers are live, exclusively
// accessible for the call; copied input spans are readable and <=isize::MAX.
// No pointer is retained except opaque owners; destroy each owner exactly once.
// C++ leaves never throw or reenter these owners. Panics/OOM abort at the ABI.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ship_path_new() -> *mut Path {
    Box::into_raw(Box::default())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_clone(p: *const Path) -> *mut Path {
    Box::into_raw(Box::new(unsafe { &*p }.clone()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_destroy(p: *mut Path) {
    if !p.is_null() {
        drop(unsafe { Box::from_raw(p) });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_size(p: *const Path) -> usize {
    unsafe { &*p }.0.len()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_get(p: *const Path, index: usize) -> u8 {
    unsafe { &*p }.0[index]
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_set(p: *mut Path, index: usize, td: u8) {
    unsafe { &mut *p }.0[index] = td;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_push(p: *mut Path, td: u8) {
    unsafe { &mut *p }.0.push(td);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_pop(p: *mut Path) {
    unsafe { &mut *p }.0.pop();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_clear(p: *mut Path) {
    unsafe { &mut *p }.0.clear();
}
unsafe fn origin_slice<'a>(p: *const u32, len: usize) -> &'a [u32] {
    if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(p, len) }
    }
}
/// Branch witnesses: the 13 search statistics, the call kind and reverse choice.
fn witness(result: &Result, call: usize) {
    if crate::witness::enabled() {
        for (index, &count) in result.stats.iter().enumerate() {
            crate::witness::add(
                [
                    crate::witness::SHIP_YAPF_REGION_NODES,
                    crate::witness::SHIP_YAPF_TRACK_NODES,
                    crate::witness::SHIP_YAPF_RETRIES,
                    crate::witness::SHIP_YAPF_TRACK_LIMITS,
                    crate::witness::SHIP_YAPF_INTERMEDIATE,
                    crate::witness::SHIP_YAPF_CACHE_TRUNCATIONS,
                    crate::witness::SHIP_YAPF_FINAL_REGION_CLEARS,
                    crate::witness::SHIP_YAPF_LOST,
                    crate::witness::SHIP_YAPF_RANDOM_DRAWS,
                    crate::witness::SHIP_YAPF_REVERSE_ORIGINS,
                    crate::witness::SHIP_YAPF_BLOCKED_FALLBACKS,
                    crate::witness::SHIP_YAPF_REGION_LIMITS,
                    crate::witness::SHIP_YAPF_ALTERNATE_DOCKING,
                    crate::witness::SHIP_YAPF_CHOOSE_CALLS,
                ][index],
                u64::from(count),
            );
        }
        crate::witness::hit(call);
        if call == crate::witness::SHIP_YAPF_REVERSE_CALLS && result.found != 0 {
            crate::witness::hit(crate::witness::SHIP_YAPF_REVERSE_CHOSEN);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_choose(
    cache: *mut Path,
    input: *const Input,
    leaves: *const Leaves,
    services: *const Services,
    context: *const c_void,
    start: u32,
    forward: u16,
    reverse: u16,
    origins: *const u32,
    len: usize,
) -> Result {
    let result = unsafe {
        choose(
            &mut *cache,
            &*input,
            &*leaves,
            &*services,
            context,
            start,
            forward,
            reverse,
            origin_slice(origins, len),
        )
    };
    witness(&result, crate::witness::SHIP_YAPF_CHOOSE_CALLS);
    result
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_reverse(
    input: *const Input,
    leaves: *const Leaves,
    services: *const Services,
    context: *const c_void,
    blocked: u8,
    origins: *const u32,
    len: usize,
) -> Result {
    let input = unsafe { &*input };
    let services = unsafe { &*services };
    let mut cache = Path::default();
    let reverse = input.trackdir ^ 8;
    let reverse_dirs = if blocked != 0 {
        input.reverse_dirs
    } else {
        1 << reverse
    };
    let forward = if blocked != 0 { 0 } else { 1 << input.trackdir };
    let mut result = unsafe {
        choose(
            &mut cache,
            input,
            &*leaves,
            services,
            context,
            input.tile,
            forward,
            reverse_dirs,
            origin_slice(origins, len),
        )
    };
    if blocked != 0 {
        result.direction = if result.found != 0 && result.origin != INVALID {
            result.origin
        } else {
            result.stats[10] += 1;
            result.stats[8] += 1;
            random_dir(services, reverse_dirs)
        };
        result.found = 1;
    } else {
        result.found = u8::from(result.found != 0 && result.origin == reverse);
    }
    witness(
        &result,
        if blocked != 0 {
            crate::witness::SHIP_YAPF_BLOCKED_CALLS
        } else {
            crate::witness::SHIP_YAPF_REVERSE_CALLS
        },
    );
    result
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_regions(
    input: *const Input,
    leaves: *const Leaves,
    start: u32,
    max: i32,
    origins: *const u32,
    len: usize,
) -> *mut RegionPath {
    Box::into_raw(Box::new(RegionPath(unsafe {
        region_path(
            &*input,
            &*leaves,
            start,
            max,
            origin_slice(origins, len),
            &mut [0; 13],
        )
    })))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_regions_size(p: *const RegionPath) -> usize {
    unsafe { &*p }.0.len()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_regions_get(p: *const RegionPath, i: usize) -> Patch {
    unsafe { &*p }.0[i]
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_regions_destroy(p: *mut RegionPath) {
    drop(unsafe { Box::from_raw(p) });
}

/// Narrow evidence hook for the heap tie/removal gap in saved-state scenarios.
/// Commands use bits 31..30: include, remove, update; low bits are arena indices.
/// This calls the actual search heap; the native test independently uses the
/// unchanged original `CBinaryHeapT`, never a second Rust implementation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_heap_probe(
    costs: *const i32,
    nodes: usize,
    commands: *const u32,
    values: *const i32,
    count: usize,
    out: *mut u32,
) {
    let mut search = Search::new(0);
    for index in 0..nodes {
        let mut node = blank(
            index as u32,
            0,
            0,
            Patch {
                x: 0,
                y: 0,
                label: 0,
            },
            None,
        );
        node.estimate = unsafe { costs.add(index).read() };
        search.arena.push(node);
    }
    for step in 0..count {
        let command = unsafe { commands.add(step).read() };
        let index = (command & 0x3fff_ffff) as usize;
        match command >> 30 {
            0 => search.include(index),
            1 => search.remove(index),
            2 => {
                search.remove(index);
                search.arena[index].estimate = unsafe { values.add(step).read() };
                search.include(index);
            }
            _ => unreachable!(),
        }
        unsafe {
            out.add(step)
                .write(search.best().map_or(u32::MAX, |i| i as u32));
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDShipYapfDestination" ["probe"] => Destination { tile, station, };
    "OpenTTDShipYapfInput" ["probe"] => Input {
        map_x,
        map_y,
        tile,
        dest_tile,
        curve90,
        curve45,
        max_speed,
        dest_dirs,
        reverse_dirs,
        trackdir,
        ocean_frac,
        canal_frac,
        station,
        unit_number,
    };
    "OpenTTDShipYapfLeaves" ["probe"] => Leaves {
        destination,
        follow,
        tile,
        patch,
        visit_new,
        visit_next,
        visit_destroy,
        debug,
    };
    "OpenTTDShipFollow" ["probe"] => Follow {
        tile,
        skipped,
        dirs,
        followed,
    };
    "OpenTTDShipTile" ["probe"] => Tile {
        ships,
        docking,
        sea,
        lock_middle,
        destination,
    };
    "OpenTTDShipYapfResult" ["probe"] => Result {
        direction,
        found,
        origin,
        stats,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_retains_capacity_across_nested_searches_and_threads() {
        // SAFETY: This is the only Rust test accessing SCRATCH. Nested calls
        // are sequential and the spawned thread starts after the outer drops.
        let mut outer = unsafe { Search::reused(7) };
        let patch = Patch {
            x: 1,
            y: 2,
            label: 3,
        };
        for key in 0..64 {
            outer.add(blank(key, key, 0, patch, None));
        }
        assert!(outer.close(0));
        let capacities = |s: &Search| {
            (
                s.arena.capacity(),
                s.open.capacity(),
                s.closed.capacity(),
                s.heap.capacity(),
            )
        };
        let retained = capacities(&outer);
        let arena = outer.arena.as_ptr() as usize;
        let nested = unsafe { Search::reused(1) };
        assert!(nested.arena.is_empty());
        assert_ne!(nested.arena.as_ptr() as usize, arena);
        drop(nested);
        assert_eq!(outer.arena.len(), 64);
        drop(outer);
        std::thread::spawn(move || {
            let mut next = unsafe { Search::reused(2) };
            assert_eq!(capacities(&next), retained);
            assert_eq!(next.arena.as_ptr() as usize, arena);
            assert!(next.arena.is_empty() && next.open.is_empty() && next.closed.is_empty());
            assert_eq!(next.heap, [usize::MAX]);
            assert_eq!(next.limit, 2);
            for key in 0..64 {
                next.add(blank(key, key, 0, patch, None));
            }
            assert!(next.close(0));
            assert_eq!(capacities(&next), retained);
        })
        .join()
        .unwrap();
    }
}
