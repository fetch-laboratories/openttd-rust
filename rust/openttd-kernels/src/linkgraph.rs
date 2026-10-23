/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete link graph computation over an immutable copied input graph.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

pub(crate) mod live;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Opaque live native `LinkGraph`; never dereferenced by Rust.
#[repr(C)]
pub struct NativeGraph {
    _private: [u8; 0],
}
use std::ffi::c_void;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InputNode {
    pub supply: u32,
    pub demand: u32,
    pub station: u32,
    pub x: u32,
    pub y: u32,
    pub edge_begin: u32,
    pub edge_count: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InputEdge {
    pub capacity: u32,
    pub travel_time: u32,
    pub dest: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Settings {
    pub accuracy: u32,
    pub demand_distance: u32,
    pub demand_size: u32,
    pub saturation: u32,
    pub distribution: u32,
    pub express: u32,
    pub map_max_x: u32,
    pub map_max_y: u32,
    pub runtime: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct OutputShare {
    pub node: u32,
    pub origin: u32,
    pub via: u32,
    pub cumulative: u32,
    pub unrestricted: u32,
    pub has_share: u32,
}

pub struct Result {
    shares: Vec<OutputShare>,
    edge_flows: Vec<u32>,
}

#[derive(Clone, Copy, Default)]
struct Demand {
    demand: u32,
    unsatisfied: u32,
}

#[derive(Default)]
struct FlowStat {
    shares: BTreeMap<u32, u32>,
    unrestricted: u32,
}

impl FlowStat {
    fn new(via: u32, flow: u32) -> Self {
        Self {
            shares: BTreeMap::from([(flow, via)]),
            unrestricted: flow,
        }
    }

    // Original ChangeShare rebuilds the cumulative-key map in its existing order.
    fn change(&mut self, via: u32, mut flow: i32) {
        let mut removed = 0_u32;
        let mut added = 0_u32;
        let mut last = 0_u32;
        let mut shares = BTreeMap::new();
        for (&cumulative, &station) in &self.shares {
            if station == via {
                if flow < 0 {
                    let share = cumulative.wrapping_sub(last);
                    if flow == i32::MIN || flow.wrapping_neg() as u32 >= share {
                        removed = removed.wrapping_add(share);
                        if cumulative <= self.unrestricted {
                            self.unrestricted = self.unrestricted.wrapping_sub(share);
                        }
                        if flow != i32::MIN {
                            flow = flow.wrapping_add(share as i32);
                        }
                        last = cumulative;
                        continue;
                    }
                    removed = removed.wrapping_add(flow.wrapping_neg() as u32);
                } else {
                    added = added.wrapping_add(flow as u32);
                }
                if cumulative <= self.unrestricted {
                    self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
                }
                flow = 0;
            }
            shares.insert(
                cumulative.wrapping_add(added).wrapping_sub(removed),
                station,
            );
            last = cumulative;
        }
        if flow > 0 {
            shares.insert(last.wrapping_add(flow as u32), via);
            if self.unrestricted < last {
                self.release(via);
            } else {
                self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
            }
        }
        self.shares = shares;
    }

    // Original ChangeShare calls ReleaseShare on its OLD map before swapping.
    // Normally job flows are unrestricted; retain the branch for wrapping inputs.
    fn release(&mut self, station: u32) {
        let mut flow = 0_u32;
        let mut next = 0_u32;
        let mut found = false;
        for (&cumulative, &via) in self.shares.iter().rev() {
            if cumulative < self.unrestricted {
                return;
            }
            if found {
                flow = next.wrapping_sub(cumulative);
                self.unrestricted = self.unrestricted.wrapping_add(flow);
                break;
            }
            if cumulative == self.unrestricted {
                return;
            }
            if via == station {
                found = true;
            }
            next = cumulative;
        }
        if flow == 0 {
            return;
        }
        let mut shares = BTreeMap::from([(flow, station)]);
        for (&cumulative, &via) in &self.shares {
            if via == station {
                flow = 0;
            } else {
                shares.insert(flow.wrapping_add(cumulative), via);
            }
        }
        self.shares = shares;
    }

    fn get_share(&self, station: u32) -> u32 {
        let mut last = 0;
        for (&share, &via) in &self.shares {
            if via == station {
                return share.wrapping_sub(last);
            }
            last = share;
        }
        0
    }

    fn finalize(&mut self, station: u32) {
        let mut local = self.get_share(u32::from(u16::MAX));
        if local > i32::MAX as u32 {
            self.change(station, -i32::MAX);
            self.change(u32::from(u16::MAX), -i32::MAX);
            local -= i32::MAX as u32;
        }
        self.change(station, (local as i32).wrapping_neg());
        self.change(u32::from(u16::MAX), (local as i32).wrapping_neg());
    }

    fn scale(&mut self, runtime: u32) {
        let mut shares = BTreeMap::new();
        let mut share = 0_u32;
        for (&old, &via) in &self.shares {
            share = share.wrapping_add(1).max(old.wrapping_mul(30) / runtime);
            shares.insert(share, via);
            if self.unrestricted == old {
                self.unrestricted = share;
            }
        }
        self.shares = shares;
    }
}

struct Node {
    base: InputNode,
    undelivered: u32,
    demands: Vec<Demand>,
    paths: VecDeque<usize>,
    flows: BTreeMap<u32, FlowStat>,
}

#[derive(Clone, Copy)]
struct Path {
    node: usize,
    origin: usize,
    distance: u32,
    capacity: u32,
    free_capacity: i32,
    flow: u32,
    children: u32,
    parent: Option<usize>,
}

impl Path {
    fn new(node: usize, source: bool) -> Self {
        Self {
            node,
            origin: if source { node } else { usize::MAX },
            distance: if source { 0 } else { u32::MAX },
            capacity: if source { u32::MAX } else { 0 },
            free_capacity: if source { i32::MAX } else { i32::MIN },
            flow: 0,
            children: 0,
            parent: None,
        }
    }
}

// C++ promotes the signed numerator to uint for division. This is deliberately
// unsigned division, even for a negative clamped free capacity.
fn capacity_ratio(free: i32, total: u32) -> i32 {
    (free
        .clamp((i32::MIN + 1) / 16, (i32::MAX - 1) / 16)
        .wrapping_mul(16) as u32
        / total.max(1)) as i32
}

fn distance(a: InputNode, b: InputNode) -> u32 {
    let dx = a.x.abs_diff(b.x);
    let dy = a.y.abs_diff(b.y);
    dx.max(dy).wrapping_mul(2).wrapping_add(dx.min(dy))
}

struct Job {
    nodes: Vec<Node>,
    edges: Vec<InputEdge>,
    edge_flows: Vec<u32>,
    paths: Vec<Option<Path>>,
    free_paths: Vec<usize>,
    settings: Settings,
}

impl Job {
    fn deliver(&mut self, from: usize, to: usize, amount: u32) {
        let node = &mut self.nodes[from];
        node.undelivered = node.undelivered.wrapping_sub(amount);
        node.demands[to].demand = node.demands[to].demand.wrapping_add(amount);
        node.demands[to].unsatisfied = node.demands[to].unsatisfied.wrapping_add(amount);
    }

    #[allow(clippy::too_many_lines)] // Keep the original queue loop and mutation order visible.
    fn demands(&mut self) {
        let settings = self.settings;
        if settings.distribution != 1 && settings.distribution != 2 {
            return;
        }
        let symmetric = settings.distribution == 2;
        let mut supplies = VecDeque::new();
        let mut demands = VecDeque::new();
        let mut supply_sum = 0_u32;
        for (id, node) in self.nodes.iter().enumerate() {
            supply_sum = supply_sum.wrapping_add(node.base.supply);
            if node.base.supply > 0 {
                supplies.push_back(id);
            }
            if node.base.demand > 0 {
                demands.push_back(id);
            }
        }
        let mut num_supplies = supplies.len() as u32;
        let mut num_demands = demands.len() as u32;
        if num_supplies == 0 || num_demands == 0 {
            return;
        }
        let demand_per_node = (supply_sum / num_demands).max(1);
        let base_distance = crate::math::int_sqrt(
            settings.map_max_x.max(settings.map_max_y) * 2
                + settings.map_max_x.min(settings.map_max_y),
        ) as i32;
        let mut mod_dist = settings.demand_distance as i32;
        if mod_dist > 100 {
            let over = mod_dist - 100;
            mod_dist = 100 + over * over / 12;
        }
        let accuracy = settings.accuracy as i32;
        let mut chance = 0_u32;
        while !supplies.is_empty() && !demands.is_empty() {
            let from = supplies.pop_front().unwrap();
            // num_demands is modified inside the original for loop.
            let mut i = 0;
            while i < num_demands {
                let to = demands.pop_front().unwrap();
                if from == to {
                    if demands.is_empty() && supplies.is_empty() {
                        return;
                    }
                    demands.push_back(to);
                    i += 1;
                    continue;
                }
                let supply = if symmetric {
                    (self.nodes[from]
                        .base
                        .supply
                        .wrapping_mul(self.nodes[to].base.supply.max(1))
                        .wrapping_mul(settings.demand_size)
                        / 100
                        / demand_per_node)
                        .max(1)
                } else {
                    self.nodes[from].base.supply
                } as i32;
                let mut scaled_distance = base_distance;
                if mod_dist > 0 {
                    let edge_distance = distance(self.nodes[from].base, self.nodes[to].base) as i32;
                    scaled_distance = 0.max(
                        base_distance.wrapping_add(
                            edge_distance
                                .wrapping_sub(base_distance)
                                .wrapping_mul(mod_dist)
                                / 1024,
                        ),
                    );
                }
                let divisor = 16_i32.wrapping_add(
                    accuracy.wrapping_mul(scaled_distance).wrapping_mul(16) / (base_distance * 2),
                );
                let mut forward = 0;
                if divisor <= supply.wrapping_mul(16) {
                    forward = (supply.wrapping_mul(16) / divisor) as u32;
                } else {
                    chance = chance.wrapping_add(1);
                    if chance
                        > settings
                            .accuracy
                            .wrapping_mul(num_demands)
                            .wrapping_mul(num_supplies)
                    {
                        forward = 1;
                    }
                }
                forward = forward.min(self.nodes[from].undelivered);
                if symmetric && self.nodes[from].base.demand > 0 {
                    let mut back = forward.wrapping_mul(settings.demand_size) / 100;
                    if back > self.nodes[to].undelivered {
                        back = self.nodes[to].undelivered;
                        forward = (back.wrapping_mul(100) / settings.demand_size).max(1);
                    }
                    self.deliver(to, from, back);
                }
                self.deliver(from, to, forward);
                let demand_left = self.nodes[to].base.demand > 0
                    && (!symmetric
                        || self.nodes[to].base.supply == 0
                        || self.nodes[to].undelivered > 0);
                if demand_left {
                    demands.push_back(to);
                } else {
                    num_demands -= 1;
                }
                if self.nodes[from].undelivered == 0 {
                    break;
                }
                i += 1;
            }
            if self.nodes[from].undelivered != 0 {
                supplies.push_back(from);
            } else {
                num_supplies -= 1;
            }
        }
    }

    fn allocate_path(&mut self, path: Path) -> usize {
        if let Some(id) = self.free_paths.pop() {
            self.paths[id] = Some(path);
            id
        } else {
            let id = self.paths.len();
            self.paths.push(Some(path));
            id
        }
    }

    fn delete_path(&mut self, id: usize) {
        self.paths[id] = None;
        self.free_paths.push(id);
    }

    fn path(&self, id: usize) -> Path {
        self.paths[id].unwrap()
    }

    fn edge(&self, from: usize, to: usize) -> usize {
        let node = self.nodes[from].base;
        let start = node.edge_begin as usize;
        start
            + self.edges[start..start + node.edge_count as usize]
                .iter()
                .position(|e| e.dest as usize == to)
                .unwrap()
    }

    fn detach(&mut self, id: usize) {
        if let Some(parent) = self.path(id).parent {
            self.paths[parent].as_mut().unwrap().children -= 1;
            self.paths[id].as_mut().unwrap().parent = None;
        }
    }

    fn fork(&mut self, dest: usize, source: usize, cap: u32, free: i32, edge_distance: u32) {
        let base = self.path(source);
        let mut path = self.path(dest);
        path.capacity = base.capacity.min(cap);
        path.free_capacity = base.free_capacity.min(free);
        path.distance = base.distance.wrapping_add(edge_distance);
        if path.parent != Some(source) {
            self.detach(dest);
            path.parent = Some(source);
            self.paths[source].as_mut().unwrap().children += 1;
        }
        path.origin = base.origin;
        self.paths[dest] = Some(path);
    }

    #[allow(clippy::too_many_lines)] // Mirrors the two original annotation/edge specializations.
    fn dijkstra(
        &mut self,
        source: usize,
        capacity_mode: bool,
        saturation: u32,
    ) -> Vec<Option<usize>> {
        let size = self.nodes.len();
        let mut paths = Vec::with_capacity(size);
        // The C++ set orders distance ascending/node ascending, or capacity
        // descending/node descending. Remove old keys before changing a path.
        let mut queue = BTreeSet::new();
        for node in 0..size {
            let path = Path::new(node, node == source);
            let id = self.allocate_path(path);
            let key = if capacity_mode {
                -i64::from(capacity_ratio(path.free_capacity, path.capacity))
            } else {
                i64::from(path.distance)
            };
            queue.insert((
                key,
                if capacity_mode {
                    -(node as i64)
                } else {
                    node as i64
                },
                id,
            ));
            paths.push(Some(id));
        }
        let station_to_node: BTreeMap<u32, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(n, node)| (node.base.station, n))
            .collect();
        while let Some((_, _, id)) = queue.pop_first() {
            let base = self.path(id);
            let from = base.node;
            let destinations: Vec<usize> = if capacity_mode {
                self.nodes[from]
                    .flows
                    .get(&self.nodes[source].base.station)
                    .map_or_else(Vec::new, |flow| {
                        flow.shares
                            .values()
                            .map(|via| station_to_node[via])
                            .collect()
                    })
            } else {
                let node = self.nodes[from].base;
                self.edges[node.edge_begin as usize..(node.edge_begin + node.edge_count) as usize]
                    .iter()
                    .map(|edge| edge.dest as usize)
                    .collect()
            };
            for to in destinations {
                if to == from {
                    continue;
                }
                let edge_id = self.edge(from, to);
                let edge = self.edges[edge_id];
                let mut capacity = edge.capacity;
                if saturation != u32::MAX {
                    capacity = (capacity.wrapping_mul(saturation) / 100).max(1);
                }
                let edge_distance =
                    distance(self.nodes[from].base, self.nodes[to].base).wrapping_add(1);
                let time = if edge.travel_time != 0 {
                    edge.travel_time.wrapping_add(74)
                } else {
                    edge_distance.wrapping_mul(74)
                };
                let edge_distance = if self.settings.express != 0 {
                    time
                } else {
                    edge_distance
                };
                let free = capacity.wrapping_sub(self.edge_flows[edge_id]) as i32;
                let dest_id = paths[to].unwrap();
                let dest = self.path(dest_id);
                let better = if capacity_mode {
                    let min_cap =
                        capacity_ratio(base.free_capacity.min(free), base.capacity.min(capacity));
                    let this_cap = capacity_ratio(dest.free_capacity, dest.capacity);
                    if min_cap == this_cap {
                        base.distance != u32::MAX
                            && base.distance.wrapping_add(edge_distance) < dest.distance
                    } else {
                        min_cap > this_cap
                    }
                } else if base.distance == u32::MAX {
                    false
                } else if dest.distance == u32::MAX {
                    true
                } else if free > 0 && base.free_capacity > 0 {
                    dest.free_capacity <= 0
                        || base.distance.wrapping_add(edge_distance) < dest.distance
                } else {
                    dest.free_capacity <= 0
                        && base.distance.wrapping_add(edge_distance) < dest.distance
                };
                if better {
                    let key = if capacity_mode {
                        -i64::from(capacity_ratio(dest.free_capacity, dest.capacity))
                    } else {
                        i64::from(dest.distance)
                    };
                    let node_key = if capacity_mode {
                        -(to as i64)
                    } else {
                        to as i64
                    };
                    queue.remove(&(key, node_key, dest_id));
                    self.fork(dest_id, id, capacity, free, edge_distance);
                    let dest = self.path(dest_id);
                    let key = if capacity_mode {
                        -i64::from(capacity_ratio(dest.free_capacity, dest.capacity))
                    } else {
                        i64::from(dest.distance)
                    };
                    queue.insert((key, node_key, dest_id));
                }
            }
        }
        paths
    }

    fn add_path_flow(&mut self, id: usize, mut flow: u32, saturation: u32) -> u32 {
        let path = self.path(id);
        if let Some(parent) = path.parent {
            let parent_node = self.path(parent).node;
            let edge = self.edge(parent_node, path.node);
            if saturation != u32::MAX {
                let usable = self.edges[edge].capacity.wrapping_mul(saturation) / 100;
                if usable > self.edge_flows[edge] {
                    flow = flow.min(usable - self.edge_flows[edge]);
                } else {
                    return 0;
                }
            }
            flow = self.add_path_flow(parent, flow, saturation);
            if path.flow == 0 && flow > 0 {
                self.nodes[parent_node].paths.push_front(id);
            }
            self.edge_flows[edge] = self.edge_flows[edge].wrapping_add(flow);
        }
        self.paths[id].as_mut().unwrap().flow = path.flow.wrapping_add(flow);
        flow
    }

    fn push_flow(&mut self, source: usize, dest: usize, path: usize, saturation: u32) -> u32 {
        let demand = self.nodes[source].demands[dest];
        let flow = (demand.demand / self.settings.accuracy).clamp(1, demand.unsatisfied);
        let flow = self.add_path_flow(path, flow, saturation);
        self.nodes[source].demands[dest].unsatisfied -= flow;
        flow
    }

    fn cleanup(&mut self, source: usize, paths: &mut [Option<usize>]) {
        let source_id = paths[source].take().unwrap();
        for node in 0..paths.len() {
            let Some(id) = paths[node] else {
                continue;
            };
            if self.path(id).parent == Some(source_id) {
                self.detach(id);
            }
            let mut at = Some(id);
            while let Some(id) = at {
                if id == source_id || self.path(id).flow != 0 {
                    break;
                }
                let path = self.path(id);
                self.detach(id);
                if self.path(id).children == 0 {
                    paths[path.node] = None;
                    self.delete_path(id);
                }
                at = path.parent;
            }
        }
        self.delete_path(source_id);
    }

    fn eliminate_cycle(&mut self, paths: &[Option<usize>], begin: usize, flow: u32) {
        let mut current = begin;
        loop {
            let path = self.path(current);
            self.paths[current].as_mut().unwrap().flow -= flow;
            if self.path(current).flow == 0 {
                let parent_node = self.path(path.parent.unwrap()).node;
                let list = &mut self.nodes[parent_node].paths;
                if let Some(pos) = list.iter().position(|&id| id == current) {
                    list.remove(pos);
                    list.push_back(current);
                }
            }
            current = paths[path.node].unwrap();
            let edge = self.edge(path.node, self.path(current).node);
            self.edge_flows[edge] = self.edge_flows[edge].wrapping_sub(flow);
            if current == begin {
                break;
            }
        }
    }

    fn eliminate_from(&mut self, paths: &mut [Option<usize>], origin: usize, next: usize) -> bool {
        match paths[next] {
            Some(usize::MAX) => false,
            None => {
                let mut next_hops = BTreeMap::new();
                let mut i = 0;
                while i < self.nodes[next].paths.len() {
                    let child_id = self.nodes[next].paths[i];
                    let child = self.path(child_id);
                    if child.flow == 0 {
                        break;
                    }
                    if child.origin == origin {
                        if let Some(&existing) = next_hops.get(&child.node) {
                            let existing_flow = self.path(existing).flow;
                            self.paths[existing].as_mut().unwrap().flow =
                                existing_flow.wrapping_add(child.flow);
                            self.paths[child_id].as_mut().unwrap().flow = 0;
                            self.nodes[next].paths.remove(i);
                            self.nodes[next].paths.push_back(child_id);
                        } else {
                            next_hops.insert(child.node, child_id);
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                let mut found = false;
                for (&node, &child) in &next_hops {
                    if self.path(child).flow > 0 {
                        paths[next] = Some(child);
                        found = self.eliminate_from(paths, origin, node) || found;
                    }
                }
                paths[next] = if found { None } else { Some(usize::MAX) };
                found
            }
            Some(begin) => {
                let mut flow = u32::MAX;
                let mut current = begin;
                loop {
                    let path = self.path(current);
                    flow = flow.min(path.flow);
                    current = paths[path.node].unwrap();
                    if current == begin {
                        break;
                    }
                }
                if flow > 0 {
                    self.eliminate_cycle(paths, begin, flow);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn eliminate_cycles(&mut self) -> bool {
        let size = self.nodes.len();
        let mut found = false;
        let mut paths = vec![None; size];
        for node in 0..size {
            paths.fill(None);
            found |= self.eliminate_from(&mut paths, node, node);
        }
        found
    }

    fn first_pass(&mut self, aborted: &impl Fn() -> bool) {
        let size = self.nodes.len();
        let saturation = self.settings.saturation;
        let mut finished = vec![false; size];
        loop {
            let mut more = false;
            for (source, done) in finished.iter_mut().enumerate() {
                if *done {
                    continue;
                }
                let mut paths = self.dijkstra(source, false, saturation);
                let mut left = false;
                for (dest, path) in paths.iter().enumerate() {
                    if self.nodes[source].demands[dest].unsatisfied > 0 {
                        let id = path.unwrap();
                        let free = self.path(id).free_capacity;
                        if free > 0 && self.push_flow(source, dest, id, saturation) > 0 {
                            more |= self.nodes[source].demands[dest].unsatisfied > 0;
                        } else if self.nodes[source].demands[dest].unsatisfied
                            == self.nodes[source].demands[dest].demand
                            && free > i32::MIN
                        {
                            self.push_flow(source, dest, id, u32::MAX);
                        }
                        left |= self.nodes[source].demands[dest].unsatisfied > 0;
                    }
                }
                *done = !left;
                self.cleanup(source, &mut paths);
            }
            // Preserve the original short circuit: cycle elimination only runs
            // if the demand loop did not already request another iteration.
            if !(more || self.eliminate_cycles()) || aborted() {
                break;
            }
        }
    }

    fn second_pass(&mut self, aborted: &impl Fn() -> bool) {
        let size = self.nodes.len();
        let mut finished = vec![false; size];
        let mut left = true;
        while left && !aborted() {
            left = false;
            for (source, done) in finished.iter_mut().enumerate() {
                if *done {
                    continue;
                }
                let mut paths = self.dijkstra(source, true, u32::MAX);
                let mut source_left = false;
                for (dest, path) in paths.iter().enumerate() {
                    let id = path.unwrap();
                    if self.nodes[source].demands[dest].unsatisfied > 0
                        && self.path(id).free_capacity > i32::MIN
                    {
                        self.push_flow(source, dest, id, u32::MAX);
                        if self.nodes[source].demands[dest].unsatisfied > 0 {
                            left = true;
                            source_left = true;
                        }
                    }
                }
                *done = !source_left;
                self.cleanup(source, &mut paths);
            }
        }
    }

    fn add_flow(&mut self, node: usize, origin: u32, via: u32, flow: u32) {
        self.nodes[node]
            .flows
            .entry(origin)
            .and_modify(|fs| fs.change(via, flow as i32))
            .or_insert_with(|| FlowStat::new(via, flow));
    }

    fn pass_on_flow(&mut self, node: usize, origin: u32, via: u32, flow: u32) {
        use std::collections::btree_map::Entry;
        match self.nodes[node].flows.entry(origin) {
            Entry::Vacant(entry) => {
                let mut fs = FlowStat::new(via, flow);
                fs.shares
                    .insert(flow.wrapping_add(flow), u32::from(u16::MAX));
                fs.unrestricted = fs.unrestricted.wrapping_add(flow);
                entry.insert(fs);
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().change(via, flow as i32);
                entry.get_mut().change(u32::from(u16::MAX), flow as i32);
            }
        }
    }

    fn map_flows(&mut self, scale: bool) {
        for previous in 0..self.nodes.len() {
            let prev_station = self.nodes[previous].base.station;
            let paths: Vec<usize> = self.nodes[previous].paths.iter().copied().collect();
            for id in paths {
                let path = self.path(id);
                if path.flow == 0 {
                    break;
                }
                let via = self.nodes[path.node].base.station;
                let origin = self.nodes[path.origin].base.station;
                self.add_flow(path.node, origin, via, path.flow);
                if prev_station == origin {
                    self.add_flow(previous, origin, via, path.flow);
                } else {
                    self.pass_on_flow(previous, origin, via, path.flow);
                }
            }
        }
        for node in &mut self.nodes {
            for fs in node.flows.values_mut() {
                fs.finalize(node.base.station);
                if scale {
                    fs.scale(self.settings.runtime);
                }
            }
            node.paths.clear();
        }
        self.paths.clear();
        self.free_paths.clear();
    }

    fn run(&mut self, aborted: impl Fn() -> bool) {
        if aborted() {
            return;
        }
        self.demands();
        if aborted() {
            return;
        }
        self.first_pass(&aborted);
        if aborted() {
            return;
        }
        self.map_flows(false);
        if aborted() {
            return;
        }
        self.second_pass(&aborted);
        if aborted() {
            return;
        }
        self.map_flows(true);
    }
}

/// Copy an input snapshot and compute all demands, paths and final flows.
///
/// # Safety
/// Input arrays are readable, aligned, disjoint and live for this call; lengths
/// fit `isize::MAX` and edge ranges/destinations satisfy original graph invariants.
/// Settings is readable and aligned. Empty arrays permit null. The nonthrowing
/// abort callback reads only the job's atomic abort flag and cannot reenter Rust.
/// No pointer or reference to C++ storage survives the call. The opaque returned
/// owner must be destroyed exactly once. Panics abort; no unwind crosses FFI.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_linkgraph_run(
    nodes: *const InputNode,
    node_count: usize,
    edges: *const InputEdge,
    edge_count: usize,
    settings: *const Settings,
    context: *const c_void,
    aborted: unsafe extern "C" fn(*const c_void) -> u8,
) -> *mut Result {
    // SAFETY: the caller supplies the documented live input arrays and settings.
    let nodes = if node_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(nodes, node_count) }
    };
    // SAFETY: same array contract; null is accepted only for empty arrays.
    let edges = if edge_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(edges, edge_count) }
    };
    // SAFETY: settings is live and aligned for the complete call.
    let settings = unsafe { *settings };
    let mut job = Job {
        nodes: nodes
            .iter()
            .map(|&base| Node {
                base,
                undelivered: base.supply,
                demands: vec![Demand::default(); node_count],
                paths: VecDeque::new(),
                flows: BTreeMap::new(),
            })
            .collect(),
        edges: edges.to_vec(),
        edge_flows: vec![0; edge_count],
        paths: Vec::new(),
        free_paths: Vec::new(),
        settings,
    };
    // SAFETY: caller promises a synchronous nonthrowing atomic-only callback.
    job.run(|| unsafe { aborted(context) } != 0);
    let mut shares = Vec::new();
    for (node_id, node) in job.nodes.iter().enumerate() {
        for (&origin, fs) in &node.flows {
            if fs.shares.is_empty() {
                shares.push(OutputShare {
                    node: node_id as u32,
                    origin,
                    via: 0,
                    cumulative: 0,
                    unrestricted: fs.unrestricted,
                    has_share: 0,
                });
            }
            for (&cumulative, &via) in &fs.shares {
                shares.push(OutputShare {
                    node: node_id as u32,
                    origin,
                    via,
                    cumulative,
                    unrestricted: fs.unrestricted,
                    has_share: 1,
                });
            }
        }
    }
    Box::into_raw(Box::new(Result {
        shares,
        edge_flows: job.edge_flows,
    }))
}

/// Read the final shares, ordered by node, origin and cumulative share.
/// # Safety
/// `owner` is a live result from run. Output count is aligned writable storage.
/// Returned storage stays readable until owner destruction; no mutation allowed.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_linkgraph_shares(
    owner: *const Result,
    count: *mut usize,
) -> *const OutputShare {
    // SAFETY: the caller owns the live immutable result and writable count.
    let result = unsafe { &*owner };
    // SAFETY: count is exclusive and live during this call.
    unsafe {
        *count = result.shares.len();
    }
    result.shares.as_ptr()
}

/// Read edge flows in the input edge order, borrowing the live result.
/// # Safety
/// `owner` must be a live result; returned storage outlives all reads and must
/// never be written. The input edge count determines its extent.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_linkgraph_edges(owner: *const Result) -> *const u32 {
    // SAFETY: the caller guarantees the live result borrow.
    unsafe { &*owner }.edge_flows.as_ptr()
}

/// Destroy the opaque result, including all Rust allocations.
/// # Safety
/// `owner` must be the live result returned by run, used exactly once; all
/// output borrows must have ended. No C++ allocator or exception enters Rust.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_linkgraph_destroy(owner: *mut Result) {
    // SAFETY: caller transfers sole ownership of the result back to Rust.
    unsafe {
        drop(Box::from_raw(owner));
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDLinkGraphNode" ["probe"] => InputNode {
        supply,
        demand,
        station,
        x,
        y,
        edge_begin,
        edge_count,
    };
    "OpenTTDLinkGraphEdge" ["probe"] => InputEdge {
        capacity,
        travel_time,
        dest,
    };
    "OpenTTDLinkGraphSettings" ["probe"] => Settings {
        accuracy,
        demand_distance,
        demand_size,
        saturation,
        distribution,
        express,
        map_max_x,
        map_max_y,
        runtime,
    };
    "OpenTTDLinkGraphShare" ["probe"] => OutputShare {
        node,
        origin,
        via,
        cumulative,
        unrestricted,
        has_share,
    };
}

#[cfg(test)]
mod tests {
    use super::{FlowStat, capacity_ratio};

    #[test]
    fn signed_capacity_numerator_is_promoted_before_division() {
        assert_eq!(capacity_ratio(-1, 100), 42_949_672);
        assert_eq!(capacity_ratio(i32::MIN, u32::MAX), 0);
        assert_eq!(capacity_ratio(2, 0), 32);
    }

    #[test]
    fn shares_keep_unsigned_key_wrap_and_empty_origin_state() {
        let mut fs = FlowStat::new(1, u32::MAX);
        fs.change(2, 2);
        assert_eq!(
            fs.shares.into_iter().collect::<Vec<_>>(),
            vec![(1, 2), (u32::MAX, 1)]
        );
        assert_eq!(fs.unrestricted, 1);
        let mut fs = FlowStat::new(1, 1);
        fs.change(1, -1);
        assert!(fs.shares.is_empty());
        assert_eq!(fs.unrestricted, 0);
    }
}
