/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Cumulative flow shares, origin maps and post-join live application. Original unsigned
//! arithmetic wraps explicitly. Operation numbers are in `cargo_flow_ffi.h`.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ops::Bound::{Excluded, Unbounded};

#[derive(Clone)]
pub struct Flow {
    shares: BTreeMap<u32, u16>,
    unrestricted: u32,
}
impl Flow {
    fn new(st: u16, amount: u32, restricted: bool) -> Self {
        assert!(amount > 0);
        Self {
            shares: BTreeMap::from([(amount, st)]),
            unrestricted: if restricted { 0 } else { amount },
        }
    }
    fn share(&self, station: u16) -> u32 {
        let mut previous = 0;
        for (&key, &st) in &self.shares {
            if st == station {
                return key.wrapping_sub(previous);
            }
            previous = key;
        }
        0
    }
    fn append(&mut self, st: u16, amount: u32, restricted: bool) {
        assert!(amount > 0);
        let end = *self.shares.last_key_value().unwrap().0;
        self.shares.insert(end.wrapping_add(amount), st);
        if !restricted {
            self.unrestricted = self.unrestricted.wrapping_add(amount);
        }
    }
    fn invalidate(&mut self) {
        assert!(!self.shares.is_empty());
        let mut shares = BTreeMap::new();
        let mut index = 0u32;
        for (&key, &st) in &self.shares {
            index = index.wrapping_add(1);
            shares.insert(index, st);
            if key == self.unrestricted {
                self.unrestricted = index;
            }
        }
        self.shares = shares;
        assert!(self.unrestricted <= *self.shares.last_key_value().unwrap().0);
    }
    fn change(&mut self, st: u16, mut flow: i32) {
        assert!(!self.shares.is_empty());
        let (mut removed, mut added, mut last) = (0u32, 0u32, 0u32);
        let mut shares = BTreeMap::new();
        for (&key, &station) in &self.shares {
            if station == st {
                if flow < 0 {
                    let share = key.wrapping_sub(last);
                    if flow == i32::MIN || flow.wrapping_neg() as u32 >= share {
                        removed = removed.wrapping_add(share);
                        if key <= self.unrestricted {
                            self.unrestricted = self.unrestricted.wrapping_sub(share);
                        }
                        if flow != i32::MIN {
                            flow = flow.wrapping_add(share as i32);
                        }
                        last = key;
                        continue;
                    }
                    removed = removed.wrapping_add(flow.wrapping_neg() as u32);
                } else {
                    added = added.wrapping_add(flow as u32);
                }
                if key <= self.unrestricted {
                    self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
                }
                flow = 0;
            }
            shares.insert(key.wrapping_add(added).wrapping_sub(removed), station);
            last = key;
        }
        if flow > 0 {
            shares.insert(last.wrapping_add(flow as u32), st);
            // Preserve the source's ReleaseShare call on the OLD map, before swap.
            if self.unrestricted < last {
                self.release(st);
            } else {
                self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
            }
        }
        self.shares = shares;
    }
    fn restrict(&mut self, st: u16) {
        assert!(!self.shares.is_empty());
        let (mut flow, mut last) = (0u32, 0u32);
        let mut shares = BTreeMap::new();
        for (&key, &station) in &self.shares {
            if flow == 0 {
                if key > self.unrestricted {
                    return;
                }
                if station == st {
                    flow = key.wrapping_sub(last);
                    self.unrestricted = self.unrestricted.wrapping_sub(flow);
                } else {
                    shares.insert(key, station);
                }
            } else {
                shares.insert(key.wrapping_sub(flow), station);
            }
            last = key;
        }
        if flow == 0 {
            return;
        }
        // The original increases the cumulative tail by flow here.
        shares.insert(last.wrapping_add(flow), st);
        self.shares = shares;
    }
    #[allow(clippy::if_not_else)] // Keep the source branch order.
    fn release(&mut self, st: u16) {
        assert!(!self.shares.is_empty());
        let (mut flow, mut next) = (0u32, 0u32);
        let mut found = false;
        for (&key, &station) in self.shares.iter().rev() {
            if key < self.unrestricted {
                return;
            }
            if found {
                flow = next.wrapping_sub(key);
                self.unrestricted = self.unrestricted.wrapping_add(flow);
                break;
            }
            if key == self.unrestricted {
                return;
            }
            if station == st {
                found = true;
            }
            next = key;
        }
        if flow == 0 {
            return;
        }
        let mut shares = BTreeMap::from([(flow, st)]);
        for (&key, &station) in &self.shares {
            if station != st {
                shares.insert(flow.wrapping_add(key), station);
            } else {
                flow = 0;
            }
        }
        self.shares = shares;
    }
    fn scale(&mut self, runtime: u32) {
        assert!(runtime > 0);
        let mut shares = BTreeMap::new();
        let mut share = 0u32;
        for (&key, &station) in &self.shares {
            share = share.wrapping_add(1).max(key.wrapping_mul(30) / runtime);
            shares.insert(share, station);
            if self.unrestricted == key {
                self.unrestricted = share;
            }
        }
        self.shares = shares;
    }
    fn upper(&self, key: u32) -> (u32, u16) {
        let (&key, &station) = self
            .shares
            .range((Excluded(key), Unbounded))
            .next()
            .unwrap();
        (key, station)
    }
    fn begin_of(&self, end: u32) -> u32 {
        self.shares
            .range(..end)
            .next_back()
            .map_or(0, |(&key, _)| key)
    }
    fn via(
        &self,
        mode: u8,
        excluded: u16,
        excluded2: u16,
        restricted: &mut u8,
        mut random: impl FnMut() -> u32,
    ) -> u16 {
        let mut range = |max: u32| crate::random_func::scale_to_limit(random(), max);
        if mode == 1 {
            assert!(!self.shares.is_empty());
            let draw = range(*self.shares.last_key_value().unwrap().0);
            *restricted = u8::from(draw >= self.unrestricted);
            return self.upper(draw).1;
        }
        if mode == 0 {
            assert!(!self.shares.is_empty());
        }
        if self.unrestricted == 0 {
            return u16::MAX;
        }
        assert!(!self.shares.is_empty());
        let (mut end, station) = self.upper(range(self.unrestricted));
        if mode == 0 || (station != excluded && station != excluded2) {
            return station;
        }
        let mut begin = self.begin_of(end);
        let mut interval = end.wrapping_sub(begin);
        if interval >= self.unrestricted {
            return u16::MAX;
        }
        let mut max = self.unrestricted.wrapping_sub(interval);
        let draw = range(max);
        let (mut end2, station2) = self.upper(if draw < begin {
            draw
        } else {
            draw.wrapping_add(interval)
        });
        if station2 != excluded && station2 != excluded2 {
            return station2;
        }
        let mut begin2 = self.begin_of(end2);
        let mut interval2 = end2.wrapping_sub(begin2);
        if interval2 >= max {
            return u16::MAX;
        }
        max = max.wrapping_sub(interval2);
        if begin > begin2 {
            std::mem::swap(&mut begin, &mut begin2);
            std::mem::swap(&mut end, &mut end2);
            std::mem::swap(&mut interval, &mut interval2);
        }
        let draw = range(max);
        self.upper(if draw < begin {
            draw
        } else if draw < begin2.wrapping_sub(interval) {
            draw.wrapping_add(interval)
        } else {
            draw.wrapping_add(interval).wrapping_add(interval2)
        })
        .1
    }
}
#[repr(C)]
#[derive(Default)]
pub struct Share {
    pub cumulative: u32,
    pub station: u16,
    pub found: u8,
}
#[repr(C)]
#[allow(clippy::struct_field_names)] // Native origin-record vocabulary.
pub struct Origin {
    pub flow: *mut Flow,
    pub origin: u16,
    pub found: u8,
}
impl Default for Origin {
    fn default() -> Self {
        Self {
            flow: std::ptr::null_mut(),
            origin: 0,
            found: 0,
        }
    }
}
/// Raw-owned nodes retain address identity across unrelated map mutations.
#[derive(Default)]
pub struct FlowMap {
    entries: BTreeMap<u16, *mut Flow>,
}
impl Drop for FlowMap {
    fn drop(&mut self) {
        for ptr in self.entries.values() {
            unsafe {
                drop(Box::from_raw(*ptr));
            }
        }
    }
}
impl Clone for FlowMap {
    fn clone(&self) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .map(|(&key, &ptr)| (key, Box::into_raw(Box::new(unsafe { (&*ptr).clone() }))))
                .collect(),
        }
    }
}
impl FlowMap {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub(crate) fn import_shares(&mut self, shares: &[crate::linkgraph::OutputShare]) {
        let first = shares.first().unwrap();
        let flow = Flow {
            shares: shares
                .iter()
                .filter(|s| s.has_share != 0)
                .map(|s| (s.cumulative, s.via as u16))
                .collect(),
            unrestricted: first.unrestricted,
        };
        self.entries
            .entry(first.origin as u16)
            .or_insert_with(|| Box::into_raw(Box::new(flow)));
    }
    fn erase(&mut self, key: u16) {
        if let Some(ptr) = self.entries.remove(&key) {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }
    pub(crate) fn deleted(&mut self, via: u16) -> Vec<u16> {
        let mut ret = Vec::new();
        for (&origin, &ptr) in &self.entries {
            let flow = unsafe { &mut *ptr };
            flow.change(via, i32::MIN);
            if flow.shares.is_empty() {
                ret.push(origin);
            }
        }
        for &origin in &ret {
            self.erase(origin);
        }
        ret
    }
    fn add(&mut self, origin: u16, via: u16, amount: u32, pass: bool) {
        if let Some(&ptr) = self.entries.get(&origin) {
            let flow = unsafe { &mut *ptr };
            flow.change(via, amount as i32);
            if pass {
                flow.change(u16::MAX, amount as i32);
            }
            assert!(!flow.shares.is_empty());
        } else {
            let mut flow = Flow::new(via, amount, false);
            if pass {
                flow.append(u16::MAX, amount, false);
            }
            self.entries.insert(origin, Box::into_raw(Box::new(flow)));
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_flow_new(st: u16, amount: u32, restricted: u8) -> *mut Flow {
    Box::into_raw(Box::new(Flow::new(st, amount, restricted != 0)))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_clone(ptr: *const Flow) -> *mut Flow {
    Box::into_raw(Box::new(unsafe { (&*ptr).clone() }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_destroy(ptr: *mut Flow) {
    unsafe {
        drop(Box::from_raw(ptr));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_read(ptr: *const Flow, op: u8, st: u16) -> u32 {
    let flow = unsafe { &*ptr };
    match op {
        0 => flow.share(st),
        1 => flow.unrestricted,
        2 => flow.shares.len() as u32,
        3 => *flow.shares.last_key_value().unwrap().0,
        _ => unreachable!(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_change(ptr: *mut Flow, op: u8, st: u16, amount: u32) {
    let flow = unsafe { &mut *ptr };
    match op {
        0 => flow.change(st, amount as i32),
        1 => flow.restrict(st),
        2 => flow.release(st),
        3 => flow.scale(amount),
        4 => flow.invalidate(),
        5 | 6 => flow.append(st, amount, op == 6),
        // Temporary imported job map is reset before its cumulative records arrive.
        7 => {
            flow.shares.clear();
            flow.unrestricted = amount;
        }
        8 => {
            flow.shares.entry(amount).or_insert(st);
        }
        _ => unreachable!(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_swap(a: *mut Flow, b: *mut Flow) {
    if a != b {
        unsafe {
            std::ptr::swap(a, b);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_share(ptr: *const Flow, key: u64, op: u8) -> Share {
    let flow = unsafe { &*ptr };
    let entry = match op {
        0 => flow.shares.first_key_value(),
        1 => {
            if key > u64::from(u32::MAX) {
                None
            } else {
                flow.shares.range((Excluded(key as u32), Unbounded)).next()
            }
        }
        2 => {
            if key > u64::from(u32::MAX) {
                flow.shares.last_key_value()
            } else {
                flow.shares.range(..key as u32).next_back()
            }
        }
        3 => flow.shares.get_key_value(&(key as u32)),
        _ => unreachable!(),
    };
    entry.map_or_else(Share::default, |(&cumulative, &station)| Share {
        cumulative,
        station,
        found: 1,
    })
}
/// Direct owner selection with the caller's shared RNG operation.
/// # Safety
/// The flow is live and serialized; random draws cannot mutate its storage.
pub(crate) unsafe fn flow_via(
    ptr: *const Flow,
    mode: u8,
    ex: u16,
    ex2: u16,
    restricted: *mut u8,
    random: impl FnMut() -> u32,
) -> u16 {
    unsafe { (&*ptr).via(mode, ex, ex2, &mut *restricted, random) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_via(
    ptr: *const Flow,
    mode: u8,
    ex: u16,
    ex2: u16,
    restricted: *mut u8,
    random: extern "C" fn() -> u32,
) -> u16 {
    // RNG mutates only the shared RNG, never this flow. No map borrow is retained.
    unsafe { flow_via(ptr, mode, ex, ex2, restricted, || random()) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_flow_map_new() -> *mut FlowMap {
    Box::into_raw(Box::default())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_clone(ptr: *const FlowMap) -> *mut FlowMap {
    Box::into_raw(Box::new(unsafe { (&*ptr).clone() }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_destroy(ptr: *mut FlowMap) {
    unsafe {
        drop(Box::from_raw(ptr));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_at(ptr: *const FlowMap, key: u32, op: u8) -> Origin {
    let map = unsafe { &*ptr };
    let entry = match op {
        0 => map.entries.first_key_value(),
        1 => map.entries.get_key_value(&(key as u16)),
        2 => {
            if key > u32::from(u16::MAX) {
                None
            } else {
                map.entries.range((Excluded(key as u16), Unbounded)).next()
            }
        }
        _ => unreachable!(),
    };
    entry.map_or_else(Origin::default, |(&origin, &flow)| Origin {
        flow,
        origin,
        found: 1,
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_insert(
    ptr: *mut FlowMap,
    origin: u16,
    flow: *const Flow,
) -> u8 {
    let map = unsafe { &mut *ptr };
    if let std::collections::btree_map::Entry::Vacant(entry) = map.entries.entry(origin) {
        entry.insert(Box::into_raw(Box::new(unsafe { (&*flow).clone() })));
        1
    } else {
        0
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_erase(ptr: *mut FlowMap, origin: u16) {
    unsafe {
        (&mut *ptr).erase(origin);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_read(
    ptr: *const FlowMap,
    op: u8,
    origin: u16,
    via: u16,
) -> u32 {
    let map = unsafe { &*ptr };
    match op {
        0 => map.entries.len() as u32,
        1 | 2 => map.entries.values().fold(0u32, |sum, &ptr| {
            sum.wrapping_add(unsafe {
                if op == 1 {
                    *(*ptr).shares.last_key_value().unwrap().0
                } else {
                    (*ptr).share(via)
                }
            })
        }),
        3 | 4 => map.entries.get(&origin).map_or(0, |&ptr| unsafe {
            if op == 3 {
                *(*ptr).shares.last_key_value().unwrap().0
            } else {
                (*ptr).share(via)
            }
        }),
        _ => unreachable!(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_change(
    ptr: *mut FlowMap,
    op: u8,
    origin: u16,
    via: u16,
    amount: u32,
) {
    let map = unsafe { &mut *ptr };
    match op {
        0 | 1 => map.add(origin, via, amount, op == 1),
        2 | 3 => {
            for &ptr in map.entries.values() {
                unsafe {
                    if op == 2 {
                        (*ptr).restrict(via);
                    } else {
                        (*ptr).release(via);
                    }
                }
            }
        }
        4 => {
            for &ptr in map.entries.values() {
                let flow = unsafe { &mut *ptr };
                let mut local = flow.share(u16::MAX);
                if local > i32::MAX as u32 {
                    flow.change(origin, -i32::MAX);
                    flow.change(u16::MAX, -i32::MAX);
                    local = local.wrapping_sub(i32::MAX as u32);
                }
                flow.change(origin, (local as i32).wrapping_neg());
                flow.change(u16::MAX, (local as i32).wrapping_neg());
                assert!(!flow.shares.is_empty());
            }
        }
        5 => {
            map.entries.retain(|_, ptr| {
                unsafe {
                    drop(Box::from_raw(*ptr));
                }
                false
            });
        }
        _ => unreachable!(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_map_delete(
    ptr: *mut FlowMap,
    via: u16,
    ctx: *mut c_void,
    emit: extern "C" fn(*mut c_void, u16),
) {
    let origins = unsafe { (&mut *ptr).deleted(via) };
    for origin in origins {
        emit(ctx, origin);
    }
}
#[repr(C)]
pub struct Services {
    pub job: *mut crate::linkgraph::job::JobRecord,
    pub live_graph: *const crate::linkgraph::live::Graph,
    pub context: *mut c_void,
    pub stations: *const crate::station_service::World,
    pub links: *const crate::station_service::Links,
    pub data_is_empty: extern "C" fn(*mut c_void, u16) -> bool,
    pub live_flows: extern "C" fn(*mut c_void, u16) -> *mut FlowMap,
    pub reroute: extern "C" fn(*mut c_void, u16, u16),
    pub finish: extern "C" fn(*mut c_void, u16, u8),
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flow_apply(services: *const Services) {
    let s = unsafe { &*services };
    if s.live_graph.is_null() {
        return;
    }
    let snapshot = unsafe { (*s.job).graph };
    let size = unsafe { &*snapshot }.size();
    let graph = unsafe { (*s.job).graph_index.0 };
    let cargo = unsafe { (*snapshot).cargo };
    let stations = unsafe { &*s.stations };
    let links = unsafe { &*s.links };
    let job_flows = |node: u16| unsafe { (&(*s.job).nodes)[usize::from(node)].flows };
    let mut edge_begin = 0;
    let valid_node = |node| {
        let station = unsafe { (&(*snapshot).nodes)[usize::from(node)].station };
        let record = crate::station_record::get_station_record_if_valid(
            unsafe { &*stations.shared },
            crate::station_type::StationID(station),
        );
        !record.is_null()
            && unsafe {
                let goods = &(*record).goods[usize::from(cargo)];
                goods.link_graph == graph && goods.node == node
            }
    };
    for node in 0..size {
        let station = unsafe { &*snapshot }.nodes[usize::from(node)].station;
        let edge_count = unsafe { &*snapshot }.nodes[usize::from(node)].edges.len();
        let first_edge = edge_begin;
        edge_begin += edge_count;
        if !valid_node(node) {
            for index in 0..size {
                let map = job_flows(index);
                unsafe {
                    (&mut *map).erase(station);
                }
            }
            continue;
        }
        let flows = job_flows(node);
        let live = (s.live_flows)(s.context, node);
        for edge in 0..edge_count {
            if unsafe { (&(*s.job).edge_flows)[first_edge + edge] } == 0 {
                continue;
            }
            let dest = unsafe { &*snapshot }.nodes[usize::from(node)].edges[edge].dest_node;
            let to = unsafe { &*snapshot }.nodes[usize::from(dest)].station;
            let observed = if valid_node(dest) {
                // No owner borrow crosses a native world leaf, including reroute.
                unsafe { &*s.live_graph }.nodes[usize::from(node)]
                    .edge(dest)
                    .map(|edge| (edge.last_update(), edge.last_unrestricted_update))
            } else {
                None
            };
            if observed.is_none_or(|(last, _)| last == crate::timer_game_common::INVALID_DATE) {
                let erased = unsafe { (&mut *flows).deleted(to) };
                for origin in erased {
                    unsafe {
                        (&mut *live).erase(origin);
                    }
                }
            } else if observed.unwrap().1 == crate::timer_game_common::INVALID_DATE {
                unsafe {
                    openttd_rust_flow_map_change(flows, 2, 0, to, 0);
                }
            }
        }
        let mut current = unsafe { openttd_rust_flow_map_at(live, 0, 0) };
        while current.found != 0 {
            let next = unsafe { openttd_rust_flow_map_at(live, u32::from(current.origin), 2) };
            let new = unsafe { openttd_rust_flow_map_at(flows, u32::from(current.origin), 1) };
            if new.found == 0 {
                if (links.automatic_distribution)(cargo) {
                    unsafe {
                        (*current.flow).invalidate();
                    }
                } else {
                    // Remove the live entry before routing, as in erase(it++).
                    let removed = unsafe { (*live).entries.remove(&current.origin).unwrap() };
                    let shares = unsafe { Box::from_raw(removed) };
                    // The audited reroute changes station/loading-vehicle cargo only:
                    // it cannot replace data/flows, delete the station or reset/yield.
                    // Like the source iterator, raw live/next handles remain valid;
                    // no borrowed station/map/entry reference crosses the callback.
                    for &via in shares.shares.values() {
                        (s.reroute)(s.context, node, via);
                    }
                }
            } else {
                unsafe {
                    std::ptr::swap(current.flow, new.flow);
                    (&mut *flows).erase(current.origin);
                }
            }
            current = next;
        }
        // Original insert copies remaining new flows; the job keeps its entries.
        unsafe {
            for (&origin, &flow) in &(*flows).entries {
                openttd_rust_flow_map_insert(live, origin, flow);
            }
        }
        let empty = u8::from((s.data_is_empty)(s.context, node));
        // finish may clear station data: no live map or entry is accessed afterward.
        (s.finish)(s.context, node, empty);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoShare" ["probe"] => Share {
        cumulative,
        station,
        found,
    };
    "OpenTTDCargoOrigin" ["probe"] => Origin {
        flow,
        origin,
        found,
    };
    "OpenTTDCargoFlowServices" ["probe"] => Services {
        job,
        live_graph,
        context,
        stations,
        links,
        data_is_empty,
        live_flows,
        reroute,
        finish,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    thread_local! { static DRAWS: Cell<u32> = const { Cell::new(0) }; }
    extern "C" fn zero_random() -> u32 {
        DRAWS.with(|n| n.set(n.get() + 1));
        0
    }

    #[test]
    fn source_restriction_tail_and_old_map_release_are_preserved() {
        let mut flow = Flow::new(1, 10, false);
        flow.append(2, 20, false);
        flow.restrict(1);
        assert_eq!(flow.shares, BTreeMap::from([(20, 2), (40, 1)]));
        assert_eq!(flow.unrestricted, 20);
        flow.change(3, 5);
        assert_eq!(flow.shares, BTreeMap::from([(20, 2), (40, 1), (45, 3)]));
        assert_eq!(flow.unrestricted, 20);
        flow.release(1);
        assert_eq!(flow.shares, BTreeMap::from([(20, 1), (40, 2), (45, 3)]));
        assert_eq!(flow.unrestricted, 40);
        flow.invalidate();
        assert_eq!(flow.shares, BTreeMap::from([(1, 1), (2, 2), (3, 3)]));
        assert_eq!(flow.unrestricted, 2);
    }
    #[test]
    fn excluded_selection_draws_once_for_each_hit() {
        let mut flow = Flow::new(1, 10, false);
        flow.append(2, 20, false);
        flow.append(3, 30, false);
        DRAWS.with(|n| n.set(0));
        let mut restricted = 0;
        assert_eq!(flow.via(2, 1, 2, &mut restricted, || zero_random()), 3);
        assert_eq!(DRAWS.with(Cell::get), 3);
        flow.change(3, i32::MIN);
        DRAWS.with(|n| n.set(0));
        assert_eq!(
            flow.via(2, 1, 2, &mut restricted, || zero_random()),
            u16::MAX
        );
        assert_eq!(DRAWS.with(Cell::get), 2);
        let restricted_flow = Flow::new(1, 5, true);
        DRAWS.with(|n| n.set(0));
        assert_eq!(
            restricted_flow.via(0, 0, 0, &mut restricted, || zero_random()),
            u16::MAX
        );
        assert_eq!(DRAWS.with(Cell::get), 0);
        assert_eq!(
            restricted_flow.via(1, 0, 0, &mut restricted, || zero_random()),
            1
        );
        assert_eq!(restricted, 1);
        assert_eq!(DRAWS.with(Cell::get), 1);
    }
    #[test]
    fn scaling_wraps_before_division_and_nodes_keep_identity() {
        let mut flow = Flow::new(1, u32::MAX, false);
        flow.scale(30);
        assert_eq!(flow.unrestricted, u32::MAX.wrapping_mul(30) / 30);
        let mut map = FlowMap::default();
        map.add(10, 1, 5, false);
        let first = map.entries[&10];
        map.add(5, 2, 8, false);
        map.add(20, 3, 9, false);
        map.erase(5);
        assert_eq!(map.entries[&10], first);
        let cloned = map.clone();
        assert_ne!(cloned.entries[&10], first);
        assert_eq!(unsafe { (*cloned.entries[&10]).share(1) }, 5);
    }
    struct World {
        job: [*mut FlowMap; 3],
        live: [*mut FlowMap; 3],
        events: Vec<(u8, u16, u16)>,
    }
    impl Drop for World {
        fn drop(&mut self) {
            for ptr in self.live {
                unsafe {
                    drop(Box::from_raw(ptr));
                }
            }
        }
    }
    extern "C" fn data_is_empty(ctx: *mut c_void, node: u16) -> bool {
        unsafe {
            (*(*ctx.cast::<World>()).live[usize::from(node)])
                .entries
                .is_empty()
        }
    }
    extern "C" fn live_map(ctx: *mut c_void, node: u16) -> *mut FlowMap {
        unsafe { (*ctx.cast::<World>()).live[usize::from(node)] }
    }
    extern "C" fn reroute(ctx: *mut c_void, node: u16, via: u16) {
        let world = unsafe { &mut *ctx.cast::<World>() };
        assert!(
            !unsafe { &*world.live[usize::from(node)] }
                .entries
                .contains_key(&7)
        );
        world.events.push((0, node, via));
    }
    extern "C" fn finish(ctx: *mut c_void, node: u16, empty: u8) {
        unsafe {
            (*ctx.cast::<World>())
                .events
                .push((1, node, u16::from(empty)));
        }
    }
    thread_local! { static MANUAL: Cell<bool> = const { Cell::new(false) }; }
    struct StationFixture {
        _records: [Box<crate::station_record::StationRecord>; 2],
        pool: crate::pool::PoolDescriptor,
    }
    thread_local! {
        static STATIONS: StationFixture = {
            let records = std::array::from_fn(|node| {
                let mut record = Box::new(crate::station_record::StationRecord::new(
                    std::ptr::null_mut(), crate::tile_type::TileIndex(0),
                ));
                record.goods[0].link_graph = 42;
                record.goods[0].node = node as u16;
                record
            });
            let mut slots = vec![crate::pool::PoolSlot { shell: std::ptr::null_mut(), record: std::ptr::null_mut() }; 103];
            for (node, record) in records.iter().enumerate() {
                slots[100 + node].record = std::ptr::from_ref(&**record).cast_mut().cast();
            }
            StationFixture {
                _records: records,
                pool: crate::pool::PoolDescriptor {
                    slots: crate::industrytype::NativeVec::from_vec(slots),
                    used_bitmap: crate::industrytype::NativeVec::new(),
                    first_free: 0, first_unused: 103, items: 2, checked: 0,
                    cleaning: false, alloc_cache: std::ptr::null_mut(),
                },
            }
        };
    }
    extern "C" fn station_pool() -> crate::station_record::StationPool {
        STATIONS.with(|fixture| &raw const fixture.pool)
    }
    extern "C" fn automatic_distribution(_: u8) -> bool {
        !MANUAL.with(Cell::get)
    }
    fn station_tables() -> (
        Box<crate::services::Services>,
        crate::station_service::World,
        crate::station_service::Links,
    ) {
        // Unused leaves terminate if application unexpectedly reaches another operation.
        macro_rules! unused {
            (($($arg:ty),*) $(-> $result:ty)?) => {{
                extern "C" fn leaf($(_: $arg),*) $(-> $result)? { unreachable!() }
                leaf
            }};
        }
        let graphs = Box::leak(Box::new(crate::linkgraph::live::Services {
            assertions: true,
            pool: std::ptr::null(),
            shared: std::ptr::null(),
            station_id: unused!((*const crate::station::BaseStation) -> u16),
            index: unused!((*const crate::linkgraph::live::Graph) -> crate::linkgraph_type::LinkGraphID),
            delete_graph: unused!((*mut crate::linkgraph::live::Graph)),
            available: unused!(() -> bool),
            allocate: unused!((u8) -> *mut crate::linkgraph::live::Graph),
        }));
        let mut shared = Box::new(crate::services::fixture(
            std::ptr::null_mut(),
            unused!((*mut c_void) -> u32),
            unused!((i32, i32, *mut u32) -> u32),
        ));
        shared.station_pool = station_pool;
        let world = crate::station_service::World {
            company: std::ptr::null(),
            graphs,
            station_exclusivity: unused!((*mut crate::station::BaseStation) -> crate::station_service::StationExclusivity),
            environment: crate::services::EnvironmentServices {
                editor: unused!(() -> bool),
                freeform_edges: unused!(() -> bool),
            },
            town_statue: unused!((*mut crate::station::BaseStation) -> bool),
            select_goods: unused!(() -> bool),
            rating_cheat: unused!(() -> bool),
            cargo_valid: unused!((u8) -> bool),
            rating_callback_enabled: unused!((u8) -> bool),
            passenger_cargo: unused!((u8) -> bool),
            packet_available: unused!(() -> bool),
            rating_redraw: unused!((*mut crate::station::BaseStation, bool)),
            acceptance_news: unused!((*mut crate::station::BaseStation, u64, bool)),
            acceptance_redraw: unused!((*mut crate::station::BaseStation)),
            delete_station: unused!((*mut crate::station::BaseStation)),
            append_packet: unused!((
                *mut crate::station::BaseStation,
                u8,
                u32,
                crate::cargo_storage::Source
            )),
            station_list_invalidation: unused!((*mut crate::station::BaseStation, i32)),
            waiting_redraw: unused!((*mut crate::station::BaseStation)),
            cargo_registry: std::ptr::null(),
            with_truncation: unused!((
                *mut crate::station::BaseStation,
                u8,
                u32,
                *mut c_void,
                crate::station_service::Scope
            )),
            truncate_next: unused!((*mut c_void) -> crate::station_service::TruncatedSource),
            shared: &raw const *shared,
        };
        let links = crate::station_service::Links {
            graphs,
            automatic_distribution,
            order_list: unused!((u16) -> *mut crate::orders::NativeOrderList),
            order_pool_size: unused!(() -> usize),
            order_count: unused!((*mut crate::orders::NativeOrderList) -> usize),
            order_destination: unused!((*mut crate::orders::NativeOrderList, usize) -> crate::station_service::OrderDestination),
            order_vehicle: unused!((*mut crate::orders::NativeOrderList) -> Option<crate::vehicle::VehicleRef>),
            next_shared: unused!((crate::vehicle::VehicleRef) -> Option<crate::vehicle::VehicleRef>),
            stopped_in_depot: unused!((crate::vehicle::VehicleRef) -> bool),
            last_service: unused!((crate::vehicle::VehicleRef) -> i32),
            cargo_type: unused!((crate::vehicle::VehicleRef) -> u8),
            station_resolver: std::ptr::null(),
            vehicle_resolver: std::ptr::null(),
        };
        (shared, world, links)
    }
    #[test]
    fn live_application_erases_before_rerouting_and_preserves_node_order() {
        for manual in [false, true] {
            MANUAL.with(|value| value.set(manual));
            let (_shared, stations, links) = station_tables();
            let mut world = World {
                job: std::array::from_fn(|_| openttd_rust_flow_map_new()),
                live: std::array::from_fn(|_| openttd_rust_flow_map_new()),
                events: Vec::new(),
            };
            unsafe {
                (*world.job[0]).add(1, 102, 10, false);
                (*world.job[0]).add(3, 100, 4, false);
                (*world.job[1]).add(102, 101, 8, false);
                (*world.live[0]).add(1, 105, 9, false);
                (*world.live[0]).add(7, 106, 4, false);
                (*world.live[0]).add(7, 107, 6, false);
            }
            let snapshot = crate::linkgraph::live::openttd_rust_live_graph_new(0);
            unsafe {
                (*snapshot).last_compression = 0;
                crate::linkgraph::live::openttd_rust_live_graph_init(snapshot, 3);
                for node in 0..3 {
                    (&mut (*snapshot).nodes)[usize::from(node)].station = 100 + node;
                }
            }
            let settings = crate::linkgraph::job::SnapshotSettings {
                recalc_time: 60,
                recalc_interval: 4,
                distribution_pax: 1,
                distribution_mail: 1,
                distribution_armoured: 1,
                distribution_default: 1,
                accuracy: 2,
                demand_size: 100,
                demand_distance: 100,
                short_path_saturation: 80,
            };
            unsafe {
                crate::linkgraph::live::openttd_rust_live_node_add_edge(
                    snapshot,
                    0,
                    2,
                    20,
                    0,
                    0,
                    crate::linkgraph_type::EdgeUpdateModes::UNRESTRICTED,
                );
            }
            let job = unsafe {
                crate::linkgraph::job::openttd_rust_linkgraph_job_new(
                    snapshot,
                    crate::linkgraph_type::LinkGraphID(42),
                    &raw const settings,
                )
            };
            unsafe {
                (*job).nodes = crate::industrytype::NativeVec::from_vec(
                    world
                        .job
                        .map(|flows| crate::linkgraph::job::JobNode { flows })
                        .into(),
                );
                (*job).edge_flows = crate::industrytype::NativeVec::from_vec(vec![10]);
            }
            let services = Services {
                job,
                // The sole annotated edge targets invalid station 102.
                live_graph: snapshot,
                context: (&raw mut world).cast(),
                stations: &raw const stations,
                links: &raw const links,
                data_is_empty,
                live_flows: live_map,
                reroute,
                finish,
            };
            unsafe {
                openttd_rust_flow_apply(&raw const services);
                crate::linkgraph::live::openttd_rust_live_graph_delete(snapshot);
            }
            assert!(!unsafe { &*world.live[0] }.entries.contains_key(&1));
            assert!(unsafe { &*world.live[0] }.entries.contains_key(&3));
            // Node 1 already applied its flow when deleted node 2 removes origin 102.
            assert!(!unsafe { &*world.job[1] }.entries.contains_key(&102));
            assert!(unsafe { &*world.live[1] }.entries.contains_key(&102));
            if manual {
                assert_eq!(
                    world.events,
                    [(0, 0, 106), (0, 0, 107), (1, 0, 0), (1, 1, 0)]
                );
                assert!(!unsafe { &*world.live[0] }.entries.contains_key(&7));
            } else {
                assert_eq!(world.events, [(1, 0, 0), (1, 1, 0)]);
                let flow = unsafe { &*(&(*world.live[0]).entries)[&7] };
                assert_eq!(flow.shares, BTreeMap::from([(1, 106), (2, 107)]));
            }
            unsafe {
                crate::linkgraph::job::openttd_rust_linkgraph_job_delete(job);
            }
        }
    }
}
