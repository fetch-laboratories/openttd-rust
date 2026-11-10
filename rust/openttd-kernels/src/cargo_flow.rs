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
use crate::cargo_type::CargoType;
use crate::station_type::StationID;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ops::Bound::{Excluded, Unbounded};

/// `station_base.h::FlowStat`: flow statistics describing how much to send
/// along a link. Each share is the difference between a cumulative map key and
/// its predecessor; a key alone has no meaning. Random upper-bound lookups
/// select shares with probability proportional to their flow.
#[derive(Clone)]
pub struct Flow {
    /// Shares sent via the specified station, or consumed locally.
    shares: BTreeMap<u32, u16>,
    /// Limit for unrestricted shares.
    unrestricted: u32,
}
impl Flow {
    /// Create a `FlowStat` with an initial station, flow and restriction state.
    fn new(st: StationID, flow: u32, restricted: bool) -> Self {
        assert!(flow > 0);
        Self {
            shares: BTreeMap::from([(flow, st.0)]),
            unrestricted: if restricted { 0 } else { flow },
        }
    }
    /// `FlowStat::GetShare`: get flow for a station.
    fn get_share(&self, st: StationID) -> u32 {
        let mut prev = 0;
        for (&key, &station) in &self.shares {
            if station == st.0 {
                return key.wrapping_sub(prev);
            }
            prev = key;
        }
        0
    }
    /// `FlowStat::AppendShare`: append only when the station is not already in
    /// the map; otherwise the shares may become inconsistent.
    fn append_share(&mut self, st: StationID, flow: u32, restricted: bool) {
        assert!(flow > 0);
        let end = *self.shares.last_key_value().unwrap().0;
        self.shares.insert(end.wrapping_add(flow), st.0);
        if !restricted {
            self.unrestricted = self.unrestricted.wrapping_add(flow);
        }
    }
    /// `FlowStat::Invalidate`: reduce flows to minimum capacity so they do not
    /// distort link-usage statistics, but keep routing any remaining cargo.
    fn invalidate(&mut self) {
        assert!(!self.shares.is_empty());
        let mut new_shares = BTreeMap::new();
        let mut i = 0u32;
        for (&key, &st) in &self.shares {
            i = i.wrapping_add(1);
            new_shares.insert(i, st);
            if key == self.unrestricted {
                self.unrestricted = i;
            }
        }
        self.shares = new_shares;
        assert!(self.unrestricted <= *self.shares.last_key_value().unwrap().0);
    }
    /// `FlowStat::ChangeShare`: change a station's share. `INT_MIN` erases the
    /// share; newly added flows are unrestricted.
    fn change_share(&mut self, st: StationID, mut flow: i32) {
        // Assert only before changing: shares may become empty afterwards,
        // in which case the whole FlowStat must then be deleted.
        assert!(!self.shares.is_empty());
        let (mut removed_shares, mut added_shares, mut last_share) = (0u32, 0u32, 0u32);
        let mut new_shares = BTreeMap::new();
        for (&key, &station) in &self.shares {
            if station == st.0 {
                if flow < 0 {
                    let share = key.wrapping_sub(last_share);
                    if flow == i32::MIN || flow.wrapping_neg() as u32 >= share {
                        removed_shares = removed_shares.wrapping_add(share);
                        if key <= self.unrestricted {
                            self.unrestricted = self.unrestricted.wrapping_sub(share);
                        }
                        if flow != i32::MIN {
                            flow = flow.wrapping_add(share as i32);
                        }
                        last_share = key;
                        continue; // Remove the whole share.
                    }
                    removed_shares = removed_shares.wrapping_add(flow.wrapping_neg() as u32);
                } else {
                    added_shares = added_shares.wrapping_add(flow as u32);
                }
                if key <= self.unrestricted {
                    self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
                }
                // Unless the loop continued above, the whole flow has been
                // added or removed.
                flow = 0;
            }
            new_shares.insert(
                key.wrapping_add(added_shares).wrapping_sub(removed_shares),
                station,
            );
            last_share = key;
        }
        if flow > 0 {
            new_shares.insert(last_share.wrapping_add(flow as u32), st.0);
            // ChangeShare calls ReleaseShare on the old map before swapping.
            if self.unrestricted < last_share {
                self.release_share(st);
            } else {
                self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
            }
        }
        self.shares = new_shares;
    }
    /// `FlowStat::RestrictShare`: move a flow to the end of the map and
    /// decrease the unrestricted amount.
    fn restrict_share(&mut self, st: StationID) {
        assert!(!self.shares.is_empty());
        let (mut flow, mut last_share) = (0u32, 0u32);
        let mut new_shares = BTreeMap::new();
        for (&key, &station) in &self.shares {
            if flow == 0 {
                if key > self.unrestricted {
                    return; // Not present or already restricted.
                }
                if station == st.0 {
                    flow = key.wrapping_sub(last_share);
                    self.unrestricted = self.unrestricted.wrapping_sub(flow);
                } else {
                    new_shares.insert(key, station);
                }
            } else {
                new_shares.insert(key.wrapping_sub(flow), station);
            }
            last_share = key;
        }
        if flow == 0 {
            return;
        }
        // The original increases the cumulative tail by flow here.
        new_shares.insert(last_share.wrapping_add(flow), st.0);
        self.shares = new_shares;
    }
    /// `FlowStat::ReleaseShare`: move a flow to the beginning of the map and
    /// increase the unrestricted amount.
    #[allow(clippy::if_not_else)] // Keep the source branch order.
    fn release_share(&mut self, st: StationID) {
        assert!(!self.shares.is_empty());
        let (mut flow, mut next_share) = (0u32, 0u32);
        let mut found = false;
        for (&key, &station) in self.shares.iter().rev() {
            // Not <=: the share may hit the limit.
            if key < self.unrestricted {
                return;
            }
            if found {
                flow = next_share.wrapping_sub(key);
                self.unrestricted = self.unrestricted.wrapping_add(flow);
                break;
            }
            if key == self.unrestricted {
                return; // Not found, so the limit was not hit.
            }
            if station == st.0 {
                found = true;
            }
            next_share = key;
        }
        if flow == 0 {
            return;
        }
        let mut new_shares = BTreeMap::from([(flow, st.0)]);
        for (&key, &station) in &self.shares {
            if station != st.0 {
                new_shares.insert(flow.wrapping_add(key), station);
            } else {
                flow = 0;
            }
        }
        self.shares = new_shares;
    }
    /// `FlowStat::ScaleToMonthly`: scale the uncompressed link-graph runtime
    /// to monthly values. Runtime must be positive to avoid infinite flow.
    fn scale_to_monthly(&mut self, runtime: u32) {
        assert!(runtime > 0);
        let mut new_shares = BTreeMap::new();
        let mut share = 0u32;
        for (&key, &station) in &self.shares {
            share = share.wrapping_add(1).max(key.wrapping_mul(30) / runtime);
            new_shares.insert(share, station);
            if self.unrestricted == key {
                self.unrestricted = share;
            }
        }
        self.shares = new_shares;
    }
    fn upper_bound(&self, key: u32) -> (u32, u16) {
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
    /// Existing dispatch for `FlowStat::GetViaWithRestricted` and both
    /// `GetVia` overloads: select a route in proportion to its flow, optionally
    /// excluding stations or including restricted flows.
    fn get_via(
        &self,
        mode: u8,
        excluded: StationID,
        excluded2: StationID,
        restricted: &mut u8,
        mut random: impl FnMut() -> u32,
    ) -> StationID {
        let mut range = |max: u32| crate::random_func::scale_to_limit(random(), max);
        if mode == 1 {
            // GetViaWithRestricted includes restricted flows and reports
            // whether the selected share crosses the unrestricted limit.
            assert!(!self.shares.is_empty());
            let draw = range(*self.shares.last_key_value().unwrap().0);
            *restricted = u8::from(draw >= self.unrestricted);
            return StationID(self.upper_bound(draw).1);
        }
        if mode == 0 {
            assert!(!self.shares.is_empty());
        }
        if self.unrestricted == 0 {
            return StationID::INVALID;
        }
        assert!(!self.shares.is_empty());
        let (mut end, station) = self.upper_bound(range(self.unrestricted));
        if mode == 0 || (station != excluded.0 && station != excluded2.0) {
            return StationID(station);
        }
        // Hit an excluded station. Draw another share outside its range.
        let mut begin = self.begin_of(end);
        let mut interval = end.wrapping_sub(begin);
        if interval >= self.unrestricted {
            return StationID::INVALID; // Only one station in the map.
        }
        let mut max = self.unrestricted.wrapping_sub(interval);
        let draw = range(max);
        let (mut end2, station2) = self.upper_bound(if draw < begin {
            draw
        } else {
            draw.wrapping_add(interval)
        });
        if station2 != excluded.0 && station2 != excluded2.0 {
            return StationID(station2);
        }
        // Hit the second excluded station. Draw again, excluding both ranges.
        let mut begin2 = self.begin_of(end2);
        let mut interval2 = end2.wrapping_sub(begin2);
        if interval2 >= max {
            return StationID::INVALID; // Only the two excluded stations remain.
        }
        max = max.wrapping_sub(interval2);
        if begin > begin2 {
            std::mem::swap(&mut begin, &mut begin2);
            std::mem::swap(&mut end, &mut end2);
            std::mem::swap(&mut interval, &mut interval2);
        }
        let draw = range(max);
        StationID(
            self.upper_bound(if draw < begin {
                draw
            } else if draw < begin2.wrapping_sub(interval) {
                draw.wrapping_add(interval)
            } else {
                draw.wrapping_add(interval).wrapping_add(interval2)
            })
            .1,
        )
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
/// `station_base.h::FlowStatMap`: flow descriptions by origin station.
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
    /// `FlowStatMap::DeleteFlows`: delete flows to a remote station and return
    /// source IDs whose complete `FlowStat`, rather than just a share, was erased.
    pub(crate) fn deleted(&mut self, via: u16) -> Vec<u16> {
        let mut ret = Vec::new();
        for (&origin, &ptr) in &self.entries {
            let flow = unsafe { &mut *ptr };
            flow.change_share(StationID(via), i32::MIN);
            if flow.shares.is_empty() {
                ret.push(origin);
            }
        }
        for &origin in &ret {
            self.erase(origin);
        }
        ret
    }
    /// Existing combined `FlowStatMap::AddFlow` / `PassOnFlow` dispatch.
    /// Passed flow is remembered under the invalid station for later
    /// subtraction from local consumption: flows cannot be negative, and
    /// sorting them before addition is avoided.
    fn add(&mut self, origin: u16, via: u16, amount: u32, pass: bool) {
        if let Some(&ptr) = self.entries.get(&origin) {
            let flow = unsafe { &mut *ptr };
            flow.change_share(StationID(via), amount as i32);
            if pass {
                flow.change_share(StationID::INVALID, amount as i32);
            }
            assert!(!flow.shares.is_empty());
        } else {
            let mut flow = Flow::new(StationID(via), amount, false);
            if pass {
                flow.append_share(StationID::INVALID, amount, false);
            }
            self.entries.insert(origin, Box::into_raw(Box::new(flow)));
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_flow_new(st: u16, amount: u32, restricted: u8) -> *mut Flow {
    Box::into_raw(Box::new(Flow::new(StationID(st), amount, restricted != 0)))
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
        0 => flow.get_share(StationID(st)),
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
        0 => flow.change_share(StationID(st), amount as i32),
        1 => flow.restrict_share(StationID(st)),
        2 => flow.release_share(StationID(st)),
        3 => flow.scale_to_monthly(amount),
        4 => flow.invalidate(),
        5 | 6 => flow.append_share(StationID(st), amount, op == 6),
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
    unsafe {
        (&*ptr)
            .get_via(
                mode,
                StationID(ex),
                StationID(ex2),
                &mut *restricted,
                random,
            )
            .0
    }
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
        // GetFlow sums all flows; GetFlowVia sums shares via one station.
        1 | 2 => map.entries.values().fold(0u32, |sum, &ptr| {
            sum.wrapping_add(unsafe {
                if op == 1 {
                    *(*ptr).shares.last_key_value().unwrap().0
                } else {
                    (*ptr).get_share(StationID(via))
                }
            })
        }),
        // GetFlowFrom and GetFlowFromVia return zero for a missing origin.
        3 | 4 => map.entries.get(&origin).map_or(0, |&ptr| unsafe {
            if op == 3 {
                *(*ptr).shares.last_key_value().unwrap().0
            } else {
                (*ptr).get_share(StationID(via))
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
        // RestrictFlows / ReleaseFlows apply to every origin at this station.
        2 | 3 => {
            for &ptr in map.entries.values() {
                unsafe {
                    if op == 2 {
                        (*ptr).restrict_share(StationID(via));
                    } else {
                        (*ptr).release_share(StationID(via));
                    }
                }
            }
        }
        4 => {
            // FinalizeLocalConsumption subtracts invalid flows from the
            // locally consumed share at this station.
            for &ptr in map.entries.values() {
                let flow = unsafe { &mut *ptr };
                let mut local = flow.get_share(StationID::INVALID);
                if local > i32::MAX as u32 {
                    // Make sure the remaining amount fits in an int.
                    flow.change_share(StationID(origin), -i32::MAX);
                    flow.change_share(StationID::INVALID, -i32::MAX);
                    local = local.wrapping_sub(i32::MAX as u32);
                }
                flow.change_share(StationID(origin), (local as i32).wrapping_neg());
                flow.change_share(StationID::INVALID, (local as i32).wrapping_neg());
                // If the local share is used up, a remote share must remain.
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
        // LinkGraphJob::~LinkGraphJob: the graph was merged into another one.
        return;
    }
    let snapshot = unsafe { (*s.job).graph };
    let size = unsafe { &*snapshot }.size();
    let graph = unsafe { (*s.job).graph_index.0 };
    let cargo = CargoType(unsafe { (*snapshot).cargo });
    let stations = unsafe { &*s.stations };
    let links = unsafe { &*s.links };
    let job_flows = |node: u16| unsafe { (&(*s.job).nodes)[usize::from(node)].flows };
    let mut edge_begin = 0;
    // Station deletion and link-graph merging can change IDs. Ignore a node
    // unless its current station, graph and node IDs remain consistent.
    let valid_node = |node| {
        let station = unsafe { (&(*snapshot).nodes)[usize::from(node)].station };
        let shell = (stations.get)(station);
        !shell.is_null()
            && (stations.is_station)(shell)
            && (links.graphs.graph_id)(shell, cargo.0) == graph
            && (links.graphs.node)(shell, cargo.0) == node
    };
    for node in 0..size {
        let station = unsafe { &*snapshot }.nodes[usize::from(node)].station;
        let edge_count = unsafe { &*snapshot }.nodes[usize::from(node)].edges.len();
        let first_edge = edge_begin;
        edge_begin += edge_count;
        if !valid_node(node) {
            // The station may have been deleted. Erase all flows from it.
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
                // The edge was removed. Delete its flows, and old flows for
                // source stations removed from the new flows, to avoid cycles
                // between old and new flows.
                let erased = unsafe { (&mut *flows).deleted(to) };
                for origin in erased {
                    unsafe {
                        (&mut *live).erase(origin);
                    }
                }
            } else if observed.unwrap().1 == crate::timer_game_common::INVALID_DATE {
                // The edge is fully restricted.
                unsafe {
                    openttd_rust_flow_map_change(flows, 2, 0, to, 0);
                }
            }
        }
        // Swap shares and invalidate completely deleted ones. Keep them to
        // avoid unroutable cargo, unless automatic distribution was turned
        // off for this cargo; then delete them and reroute the relevant cargo.
        let mut current = unsafe { openttd_rust_flow_map_at(live, 0, 0) };
        while current.found != 0 {
            let next = unsafe { openttd_rust_flow_map_at(live, u32::from(current.origin), 2) };
            let new = unsafe { openttd_rust_flow_map_at(flows, u32::from(current.origin), 1) };
            if new.found == 0 {
                if (links.automatic_distribution)(cargo.0) {
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
        let mut flow = Flow::new(StationID(1), 10, false);
        flow.append_share(StationID(2), 20, false);
        flow.restrict_share(StationID(1));
        assert_eq!(flow.shares, BTreeMap::from([(20, 2), (40, 1)]));
        assert_eq!(flow.unrestricted, 20);
        flow.change_share(StationID(3), 5);
        assert_eq!(flow.shares, BTreeMap::from([(20, 2), (40, 1), (45, 3)]));
        assert_eq!(flow.unrestricted, 20);
        flow.release_share(StationID(1));
        assert_eq!(flow.shares, BTreeMap::from([(20, 1), (40, 2), (45, 3)]));
        assert_eq!(flow.unrestricted, 40);
        flow.invalidate();
        assert_eq!(flow.shares, BTreeMap::from([(1, 1), (2, 2), (3, 3)]));
        assert_eq!(flow.unrestricted, 2);
    }
    #[test]
    fn excluded_selection_draws_once_for_each_hit() {
        let mut flow = Flow::new(StationID(1), 10, false);
        flow.append_share(StationID(2), 20, false);
        flow.append_share(StationID(3), 30, false);
        DRAWS.with(|n| n.set(0));
        let mut restricted = 0;
        assert_eq!(
            flow.get_via(2, StationID(1), StationID(2), &mut restricted, || {
                zero_random()
            })
            .0,
            3
        );
        assert_eq!(DRAWS.with(Cell::get), 3);
        flow.change_share(StationID(3), i32::MIN);
        DRAWS.with(|n| n.set(0));
        assert_eq!(
            flow.get_via(2, StationID(1), StationID(2), &mut restricted, || {
                zero_random()
            })
            .0,
            u16::MAX
        );
        assert_eq!(DRAWS.with(Cell::get), 2);
        let restricted_flow = Flow::new(StationID(1), 5, true);
        DRAWS.with(|n| n.set(0));
        assert_eq!(
            restricted_flow
                .get_via(0, StationID(0), StationID(0), &mut restricted, || {
                    zero_random()
                })
                .0,
            u16::MAX
        );
        assert_eq!(DRAWS.with(Cell::get), 0);
        assert_eq!(
            restricted_flow
                .get_via(1, StationID(0), StationID(0), &mut restricted, || {
                    zero_random()
                })
                .0,
            1
        );
        assert_eq!(restricted, 1);
        assert_eq!(DRAWS.with(Cell::get), 1);
    }
    #[test]
    fn scaling_wraps_before_division_and_nodes_keep_identity() {
        let mut flow = Flow::new(StationID(1), u32::MAX, false);
        flow.scale_to_monthly(30);
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
        assert_eq!(unsafe { (*cloned.entries[&10]).get_share(StationID(1)) }, 5);
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
    extern "C" fn station_get(id: u16) -> *mut crate::station::BaseStation {
        if id == 102 {
            std::ptr::null_mut()
        } else {
            std::ptr::without_provenance_mut(usize::from(id) + 1)
        }
    }
    extern "C" fn is_station(_: *mut crate::station::BaseStation) -> bool {
        true
    }
    extern "C" fn graph_id(_: *mut crate::station::BaseStation, _: u8) -> u16 {
        42
    }
    extern "C" fn node_id(station: *mut crate::station::BaseStation, _: u8) -> u16 {
        (station.addr() - 101) as u16
    }
    extern "C" fn automatic_distribution(_: u8) -> bool {
        !MANUAL.with(Cell::get)
    }
    fn station_tables() -> (crate::station_service::World, crate::station_service::Links) {
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
            station_id: unused!((*const crate::station::BaseStation) -> u16),
            index: unused!((*const crate::linkgraph::live::Graph) -> crate::linkgraph_type::LinkGraphID),
            station: unused!((u16, u8) -> crate::linkgraph::live::StationInput),
            attach: unused!((u16, u8, u16, u16)),
            set_node: unused!((u16, u8, u16)),
            delete_graph: unused!((*mut crate::linkgraph::live::Graph)),
            graph_id,
            node: node_id,
            available: unused!(() -> bool),
            allocate: unused!((u8) -> *mut crate::linkgraph::live::Graph),
        }));
        let world = crate::station_service::World {
            graphs,
            station: unused!((u16) -> crate::station_service::LoadingStation),
            station_owner: unused!((*mut crate::station::BaseStation) -> u8),
            station_exclusivity: unused!((*mut crate::station::BaseStation) -> crate::station_service::StationExclusivity),
            rating_counts: unused!((*mut crate::station::BaseStation, u8) -> crate::station_service::RatingCounts),
            get: station_get,
            owner: unused!((*mut crate::station::BaseStation) -> *mut crate::station_service::Service),
            pool_size: unused!(() -> usize),
            environment: crate::services::EnvironmentServices {
                editor: unused!(() -> bool),
                freeform_edges: unused!(() -> bool),
            },
            facilities: unused!((*mut crate::station::BaseStation) -> u8),
            is_station,
            town_statue: unused!((*mut crate::station::BaseStation) -> bool),
            rating_cheat: unused!(() -> bool),
            available: unused!((*mut crate::station::BaseStation, u8) -> u32),
            cargo_valid: unused!((u8) -> bool),
            rating_callback_enabled: unused!((u8) -> bool),
            passenger_cargo: unused!((u8) -> bool),
            packet_available: unused!(() -> bool),
            rating_redraw: unused!((*mut crate::station::BaseStation, bool)),
            acceptance_news: unused!((*mut crate::station::BaseStation, u64, bool)),
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
            shared: std::ptr::null(),
        };
        let links = crate::station_service::Links {
            graphs,
            automatic_distribution,
            edge: unused!((*mut crate::station::BaseStation, u16) -> crate::station_service::Edge),
            delete_flows: unused!((*mut crate::station::BaseStation, u8, u16)),
            restrict_flows: unused!((*mut crate::station::BaseStation, u8, u16)),
            order_list: unused!((u16) -> *mut crate::orders::NativeOrderList),
            order_pool_size: unused!(() -> usize),
            order_count: unused!((*mut crate::orders::NativeOrderList) -> usize),
            order_destination: unused!((*mut crate::orders::NativeOrderList, usize) -> crate::station_service::OrderDestination),
            order_vehicle: unused!((*mut crate::orders::NativeOrderList) -> Option<crate::vehicle::VehicleRef>),
            next_shared: unused!((crate::vehicle::VehicleRef) -> Option<crate::vehicle::VehicleRef>),
            stopped_in_depot: unused!((crate::vehicle::VehicleRef) -> bool),
            last_service: unused!((crate::vehicle::VehicleRef) -> i32),
            cargo_type: unused!((crate::vehicle::VehicleRef) -> u8),
            goods_entry: unused!((*mut crate::station::BaseStation, u8) -> *const crate::station_base::GoodsEntry),
            station_resolver: std::ptr::null(),
            vehicle_resolver: std::ptr::null(),
        };
        (world, links)
    }
    #[test]
    fn live_application_erases_before_rerouting_and_preserves_node_order() {
        for manual in [false, true] {
            MANUAL.with(|value| value.set(manual));
            let (stations, links) = station_tables();
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
