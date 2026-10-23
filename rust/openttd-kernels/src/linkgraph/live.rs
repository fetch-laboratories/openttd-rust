/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `linkgraph.h` contents owner. Native shells/pools retain identity, not contents.
//! Each job clones this owner before publication and destroys it after joining.
//! Save/load borrows canonical scalar addresses only between native calls. No
//! Rust frame or borrow survives `SlObject`, its pointer cleanup or exceptions.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::linkgraph_type::EdgeUpdateModes;
use std::ptr::addr_of_mut;

pub(crate) const MIN_TIMEOUT_DISTANCE: u32 = 32;
pub(crate) const STALE_LINK_DEPOT_TIMEOUT: i32 = 1024;
pub(crate) const COMPRESSION_INTERVAL: i32 = 256;
const INVALID_DATE: i32 = -1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Edge {
    pub capacity: u32,
    pub usage: u32,
    pub travel_time_sum: u64,
    pub last_unrestricted_update: i32,
    pub last_restricted_update: i32,
    pub dest_node: u16,
}
impl Edge {
    fn new(dest_node: u16) -> Self {
        Self {
            capacity: 0,
            usage: 0,
            travel_time_sum: 0,
            last_unrestricted_update: INVALID_DATE,
            last_restricted_update: INVALID_DATE,
            dest_node,
        }
    }
    pub(crate) fn last_update(self) -> i32 {
        self.last_unrestricted_update
            .max(self.last_restricted_update)
    }
    pub(crate) fn travel_time(self) -> u32 {
        (self.travel_time_sum / u64::from(self.capacity)) as u32
    }
    fn update(&mut self, capacity: u32, usage: u32, time: u32, modes: EdgeUpdateModes, date: i32) {
        if modes.0 & EdgeUpdateModes::INCREASE.0 != 0 {
            if self.travel_time_sum == 0 {
                self.travel_time_sum =
                    u64::from(self.capacity.wrapping_add(capacity)).wrapping_mul(u64::from(time));
            } else if time == 0 {
                self.travel_time_sum = self.travel_time_sum.wrapping_add(
                    (self.travel_time_sum / u64::from(self.capacity))
                        .wrapping_mul(u64::from(capacity)),
                );
            } else {
                self.travel_time_sum = self
                    .travel_time_sum
                    .wrapping_add(u64::from(time) * u64::from(capacity));
            }
            self.capacity = self.capacity.wrapping_add(capacity);
            self.usage = self.usage.wrapping_add(usage);
        } else if modes.0 & EdgeUpdateModes::REFRESH.0 != 0 {
            if self.travel_time_sum == 0 {
                self.capacity = self.capacity.max(capacity);
                self.travel_time_sum = u64::from(time) * u64::from(self.capacity);
            } else if capacity > self.capacity {
                self.travel_time_sum = (self.travel_time_sum / u64::from(self.capacity))
                    .wrapping_mul(u64::from(capacity));
                self.capacity = capacity;
            }
            self.usage = self.usage.max(usage);
        }
        if modes.0 & EdgeUpdateModes::UNRESTRICTED.0 != 0 {
            self.last_unrestricted_update = date;
        }
        if modes.0 & EdgeUpdateModes::RESTRICTED.0 != 0 {
            self.last_restricted_update = date;
        }
    }
}

pub(crate) struct Node {
    pub supply: u32,
    pub demand: u32,
    pub station: u16,
    pub xy: u32,
    pub last_update: i32,
    pub edges: Vec<Edge>,
}
impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            supply: self.supply,
            demand: self.demand,
            station: self.station,
            xy: self.xy,
            last_update: self.last_update,
            edges: self.edges.clone(),
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.supply = source.supply;
        self.demand = source.demand;
        self.station = source.station;
        self.xy = source.xy;
        self.last_update = source.last_update;
        self.edges.clone_from(&source.edges);
    }
}
impl Node {
    fn new(xy: u32, station: u16, demand: u32) -> Self {
        Self {
            supply: 0,
            demand,
            station,
            xy,
            last_update: INVALID_DATE,
            edges: Vec::new(),
        }
    }
    pub(crate) fn edge(&self, dest: u16) -> Option<&Edge> {
        self.edges
            .binary_search_by_key(&dest, |e| e.dest_node)
            .ok()
            .map(|i| &self.edges[i])
    }
    pub(crate) fn update_edge(
        &mut self,
        to: u16,
        capacity: u32,
        usage: u32,
        time: u32,
        modes: EdgeUpdateModes,
        date: i32,
    ) {
        match self.edges.binary_search_by_key(&to, |e| e.dest_node) {
            Ok(index) => self.edges[index].update(capacity, usage, time, modes, date),
            Err(index) => {
                let mut edge = Edge::new(to);
                edge.capacity = capacity;
                edge.usage = usage;
                edge.travel_time_sum = u64::from(time) * u64::from(capacity);
                if modes.0 & EdgeUpdateModes::UNRESTRICTED.0 != 0 {
                    edge.last_unrestricted_update = date;
                }
                if modes.0 & EdgeUpdateModes::RESTRICTED.0 != 0 {
                    edge.last_restricted_update = date;
                }
                self.edges.insert(index, edge);
            }
        }
    }
    pub(crate) fn remove_edge(&mut self, to: u16) {
        let first = self.edges.partition_point(|e| e.dest_node < to);
        let last = self.edges.partition_point(|e| e.dest_node <= to);
        self.edges.drain(first..last);
    }
}

#[derive(Clone)]
pub struct Graph {
    pub(crate) cargo: u8,
    pub(crate) last_compression: i32,
    pub(crate) nodes: Vec<Node>,
}
impl Graph {
    pub(crate) fn size(&self) -> u16 {
        self.nodes.len() as u16
    }
    pub(crate) fn add_node(&mut self, xy: u32, station: u16, demand: u32) -> u16 {
        let id = self.size();
        self.nodes.push(Node::new(xy, station, demand));
        id
    }
    pub(crate) fn compress(&mut self, date: i32) {
        self.last_compression = date.wrapping_add(self.last_compression) / 2;
        let size = usize::from(self.size());
        for n in self.nodes.iter_mut().take(size) {
            n.supply /= 2;
            for e in &mut n.edges {
                if e.capacity > 0 {
                    let capacity = 1.max(e.capacity / 2);
                    if e.capacity < (1 << 16) {
                        e.travel_time_sum = e.travel_time_sum.wrapping_mul(u64::from(capacity))
                            / u64::from(e.capacity);
                    } else if e.travel_time_sum != 0 {
                        e.travel_time_sum = 1.max(e.travel_time_sum / 2);
                    }
                    e.capacity = capacity;
                    e.usage /= 2;
                }
            }
        }
    }
    fn remove_node(&mut self, id: u16) {
        let last = self.size().wrapping_sub(1);
        // Copy assignment, including for the last node, matches the source's
        // replacement semantics; existing node/edge storage can be reused.
        if id != last {
            let (before, after) = self.nodes.split_at_mut(usize::from(last));
            before[usize::from(id)].clone_from(&after[0]);
        }
        self.nodes.pop();
        for n in &mut self.nodes {
            n.remove_edge(id);
            if n.edges.last().is_some_and(|e| e.dest_node == last) {
                let mut edge = *n.edges.last().unwrap();
                edge.dest_node = id;
                let insert = n.edges.partition_point(|e| e.dest_node < id);
                n.edges.insert(insert, edge);
                n.edges.pop();
            }
        }
    }
}

pub(crate) fn scale(value: u32, target_age: i32, orig_age: i32) -> u32 {
    if value > 0 {
        1.max(value.wrapping_mul(target_age as u32) / orig_age as u32)
    } else {
        0
    }
}

// All entries require live uniquely owned graph pointers for mutation, valid
// source indices and original graph invariants. They retain no native pointer.
// Allocation/panics abort; entries never call throwing C++ services.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_live_graph_new(cargo: u8, date: i32) -> *mut Graph {
    Box::into_raw(Box::new(Graph {
        cargo,
        last_compression: date,
        nodes: Vec::new(),
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_copy(graph: *const Graph) -> *mut Graph {
    Box::into_raw(Box::new(unsafe { &*graph }.clone()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_delete(graph: *mut Graph) {
    drop(unsafe { Box::from_raw(graph) });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_size(graph: *const Graph) -> u16 {
    unsafe { &*graph }.size()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_cargo(graph: *mut Graph) -> *mut u8 {
    unsafe { addr_of_mut!((*graph).cargo) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_date(graph: *mut Graph) -> *mut i32 {
    unsafe { addr_of_mut!((*graph).last_compression) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_init(graph: *mut Graph, size: u32) {
    unsafe { &mut *graph }
        .nodes
        .resize_with(size as usize, || Node::new(u32::MAX, u16::MAX, 0));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_shift(graph: *mut Graph, interval: i32) {
    let graph = unsafe { &mut *graph };
    graph.last_compression = graph.last_compression.wrapping_add(interval);
    let size = usize::from(graph.size());
    for n in graph.nodes.iter_mut().take(size) {
        if n.last_update != INVALID_DATE {
            n.last_update = n.last_update.wrapping_add(interval);
        }
        for e in &mut n.edges {
            if e.last_unrestricted_update != INVALID_DATE {
                e.last_unrestricted_update = e.last_unrestricted_update.wrapping_add(interval);
            }
            if e.last_restricted_update != INVALID_DATE {
                e.last_restricted_update = e.last_restricted_update.wrapping_add(interval);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_compress(graph: *mut Graph, date: i32) {
    unsafe { &mut *graph }.compress(date);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_live_graph_scale(value: u32, target: i32, orig: i32) -> u32 {
    scale(value, target, orig)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_monthly(
    graph: *const Graph,
    value: u32,
    date: i32,
) -> u32 {
    value.wrapping_mul(30)
        / date
            .wrapping_sub(unsafe { &*graph }.last_compression)
            .wrapping_add(1) as u32
}

/// Save-only address carrier; these point to the canonical fields, never a mirror.
#[repr(C)]
pub struct NodeFields {
    pub supply: *mut u32,
    pub demand: *mut u32,
    pub station: *mut u16,
    pub xy: *mut u32,
    pub last_update: *mut i32,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_fields(graph: *mut Graph, id: u16) -> NodeFields {
    let n = unsafe { (*graph).nodes.as_ptr().add(usize::from(id)).cast_mut() };
    NodeFields {
        supply: unsafe { addr_of_mut!((*n).supply) },
        demand: unsafe { addr_of_mut!((*n).demand) },
        station: unsafe { addr_of_mut!((*n).station) },
        xy: unsafe { addr_of_mut!((*n).xy) },
        last_update: unsafe { addr_of_mut!((*n).last_update) },
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_supply(graph: *const Graph, id: u16) -> u32 {
    unsafe { &*graph }.nodes[usize::from(id)].supply
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_station(graph: *const Graph, id: u16) -> u16 {
    unsafe { &*graph }.nodes[usize::from(id)].station
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_location(graph: *mut Graph, id: u16, xy: u32) {
    unsafe { &mut *graph }.nodes[usize::from(id)].xy = xy;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_demand(graph: *mut Graph, id: u16, demand: u32) {
    unsafe { &mut *graph }.nodes[usize::from(id)].demand = demand;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_update_supply(
    graph: *mut Graph,
    id: u16,
    supply: u32,
    date: i32,
) {
    let n = &mut unsafe { &mut *graph }.nodes[usize::from(id)];
    n.supply = n.supply.wrapping_add(supply);
    n.last_update = date;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_edges(
    graph: *const Graph,
    id: u16,
    len: *mut usize,
) -> *const Edge {
    let edges = &unsafe { &*graph }.nodes[usize::from(id)].edges;
    unsafe {
        *len = edges.len();
    }
    edges.as_ptr()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_edge(
    graph: *const Graph,
    id: u16,
    to: u16,
) -> *const Edge {
    unsafe { &*graph }.nodes[usize::from(id)]
        .edge(to)
        .map_or(std::ptr::null(), std::ptr::from_ref)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_update_edge(
    graph: *mut Graph,
    id: u16,
    to: u16,
    capacity: u32,
    usage: u32,
    time: u32,
    modes: EdgeUpdateModes,
    date: i32,
) {
    unsafe { &mut *graph }.nodes[usize::from(id)]
        .update_edge(to, capacity, usage, time, modes, date);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_remove_edge(graph: *mut Graph, id: u16, to: u16) {
    unsafe { &mut *graph }.nodes[usize::from(id)].remove_edge(to);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_node_append_edge(
    graph: *mut Graph,
    id: u16,
) -> *mut Edge {
    let edges = &mut unsafe { &mut *graph }.nodes[usize::from(id)].edges;
    edges.push(Edge::new(u16::MAX));
    edges.last_mut().unwrap()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_edge_init(edge: *mut Edge, dest: u16) {
    unsafe {
        edge.write(Edge::new(dest));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_edge_update(
    edge: *mut Edge,
    capacity: u32,
    usage: u32,
    time: u32,
    modes: EdgeUpdateModes,
    date: i32,
) {
    unsafe { &mut *edge }.update(capacity, usage, time, modes, date);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_edge_restrict(edge: *mut Edge) {
    unsafe {
        (*edge).last_unrestricted_update = INVALID_DATE;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_edge_release(edge: *mut Edge) {
    unsafe {
        (*edge).last_restricted_update = INVALID_DATE;
    }
}

#[repr(C)]
pub struct StationInput {
    pub xy: u32,
    pub demand: u32,
}
#[repr(C)]
pub struct Access {
    pub owner: *mut std::ffi::c_void,
    pub index: u16,
}
unsafe extern "C" {
    fn openttd_native_linkgraph_access(graph: *mut super::NativeGraph) -> Access;
    fn openttd_native_linkgraph_station(station: u16, cargo: u8) -> StationInput;
    fn openttd_native_linkgraph_attach(station: u16, cargo: u8, graph: u16, node: u16);
    fn openttd_native_linkgraph_set_node(station: u16, cargo: u8, node: u16);
    fn openttd_native_linkgraph_delete(graph: *mut super::NativeGraph);
}
struct GraphAccess {
    owner: *mut Graph,
    index: u16,
}
/// Resolve once per Rust caller entry; the shell remains live under the game mutex.
pub(crate) unsafe fn resolve(graph: *mut super::NativeGraph) -> *mut Graph {
    unsafe { openttd_native_linkgraph_access(graph) }
        .owner
        .cast()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_add_station(
    graph: *mut super::NativeGraph,
    station: u16,
) -> u16 {
    let native = unsafe { openttd_native_linkgraph_access(graph) };
    let access = GraphAccess {
        owner: native.owner.cast(),
        index: native.index,
    };
    let cargo = unsafe { (*access.owner).cargo };
    let input = unsafe { openttd_native_linkgraph_station(station, cargo) };
    unsafe { &mut *access.owner }.add_node(input.xy, station, input.demand)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_remove(graph: *mut super::NativeGraph, id: u16) {
    let owner = unsafe { resolve(graph) };
    let (station, cargo) = {
        let g = unsafe { &*owner };
        (
            g.nodes[usize::from(g.size().wrapping_sub(1))].station,
            g.cargo,
        )
    };
    unsafe {
        openttd_native_linkgraph_set_node(station, cargo, id);
    }
    unsafe { &mut *owner }.remove_node(id);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_merge(
    graph: *mut super::NativeGraph,
    other: *mut super::NativeGraph,
    date: i32,
) {
    let native = unsafe { openttd_native_linkgraph_access(graph) };
    let access = GraphAccess {
        owner: native.owner.cast(),
        index: native.index,
    };
    let source = unsafe { resolve(other) };
    let (cargo, first, age, other_age, size) = {
        let g = unsafe { &*access.owner };
        let other = unsafe { &*source };
        (
            g.cargo,
            g.size(),
            date.wrapping_sub(g.last_compression).wrapping_add(1),
            date.wrapping_sub(other.last_compression).wrapping_add(1),
            other.size(),
        )
    };
    for id in 0..size {
        let station = unsafe { &*source }.nodes[usize::from(id)].station;
        let input = unsafe { openttd_native_linkgraph_station(station, cargo) };
        let new = unsafe { &mut *access.owner }.add_node(input.xy, station, input.demand);
        let supply = unsafe { &*source }.nodes[usize::from(id)].supply;
        unsafe { &mut *access.owner }.nodes[usize::from(new)].supply =
            scale(supply, age, other_age);
        // No graph borrow crosses station mutation or deletion/reentry.
        unsafe {
            openttd_native_linkgraph_attach(station, cargo, access.index, new);
        }
        let from = &unsafe { &*source }.nodes[usize::from(id)];
        let to = &mut unsafe { &mut *access.owner }.nodes[usize::from(new)];
        for edge in &from.edges {
            let mut added = Edge::new(first.wrapping_add(edge.dest_node));
            added.capacity = scale(edge.capacity, age, other_age);
            added.usage = scale(edge.usage, age, other_age);
            // Scale takes uint, so the uint64 travel sum narrows BEFORE scaling.
            added.travel_time_sum = u64::from(scale(edge.travel_time_sum as u32, age, other_age));
            to.edges.push(added);
        }
    }
    unsafe {
        openttd_native_linkgraph_delete(other);
    }
}

/// Run the existing computation from its distinct immutable snapshot. This
/// replaces native packing of the same two temporary input arrays, not the job
/// algorithm or its publication/join policy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_live_graph_run(
    graph: *const Graph,
    settings: *const super::Settings,
    context: *const std::ffi::c_void,
    aborted: unsafe extern "C" fn(*const std::ffi::c_void) -> u8,
) -> *mut super::Result {
    let g = unsafe { &*graph };
    let s = unsafe { &*settings };
    let map = crate::map_func::Map {
        log_x: (s.map_max_x + 1).ilog2() as u8,
    };
    let mut nodes = Vec::with_capacity(usize::from(g.size()));
    let mut edges = Vec::new();
    for n in g.nodes.iter().take(usize::from(g.size())) {
        nodes.push(super::InputNode {
            supply: n.supply,
            demand: n.demand,
            station: u32::from(n.station),
            x: map.tile_x(n.xy),
            y: map.tile_y(n.xy),
            edge_begin: edges.len() as u32,
            edge_count: n.edges.len() as u32,
        });
        for e in &n.edges {
            edges.push(super::InputEdge {
                capacity: e.capacity,
                travel_time: e.travel_time(),
                dest: u32::from(e.dest_node),
            });
        }
    }
    unsafe {
        super::openttd_rust_linkgraph_run(
            nodes.as_ptr(),
            nodes.len(),
            edges.as_ptr(),
            edges.len(),
            settings,
            context,
            aborted,
        )
    }
}

#[repr(C)]
pub struct Membership {
    pub graph: *mut super::NativeGraph,
    pub node: u16,
    pub station: u16,
}
unsafe extern "C" {
    fn openttd_native_linkgraph_membership(
        station: *mut crate::station::BaseStation,
        cargo: u8,
    ) -> Membership;
    fn openttd_native_linkgraph_available() -> bool;
    fn openttd_native_linkgraph_allocate(cargo: u8) -> *mut super::NativeGraph;
    fn openttd_native_linkgraph_allocation_failure();
    pub(crate) fn openttd_native_linkgraph_dequeue(graph: *mut super::NativeGraph);
}
pub(crate) unsafe fn join(graph: *mut super::NativeGraph, station: u16) {
    let native = unsafe { openttd_native_linkgraph_access(graph) };
    let access = GraphAccess {
        owner: native.owner.cast(),
        index: native.index,
    };
    let cargo = unsafe { (*access.owner).cargo };
    let input = unsafe { openttd_native_linkgraph_station(station, cargo) };
    let node = unsafe { &mut *access.owner }.add_node(input.xy, station, input.demand);
    unsafe {
        openttd_native_linkgraph_attach(station, cargo, access.index, node);
    }
}
pub(crate) unsafe fn station_demand(
    station: *mut crate::station::BaseStation,
    cargo: u8,
    demand: u32,
) {
    let m = unsafe { openttd_native_linkgraph_membership(station, cargo) };
    if !m.graph.is_null() {
        let owner = unsafe { resolve(m.graph) };
        unsafe { &mut *owner }.nodes[usize::from(m.node)].demand = demand;
    }
}
pub(crate) unsafe fn station_supply(
    station: *mut crate::station::BaseStation,
    cargo: u8,
    supply: u32,
    date: i32,
) {
    let mut m = unsafe { openttd_native_linkgraph_membership(station, cargo) };
    if m.graph.is_null() {
        if unsafe { openttd_native_linkgraph_available() } {
            m.graph = unsafe { openttd_native_linkgraph_allocate(cargo) };
            unsafe {
                join(m.graph, m.station);
            }
            m = unsafe { openttd_native_linkgraph_membership(station, cargo) };
        } else {
            unsafe {
                openttd_native_linkgraph_allocation_failure();
            }
        }
    }
    if !m.graph.is_null() {
        let owner = unsafe { resolve(m.graph) };
        let n = &mut unsafe { &mut *owner }.nodes[usize::from(m.node)];
        n.supply = n.supply.wrapping_add(supply);
        n.last_update = date;
    }
}
