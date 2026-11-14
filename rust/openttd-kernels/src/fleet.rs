/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical fleet state and complete group and autoreplace-rule control. Exports take
//! live, serialized group, statistics and rule records whose scalar prefix C++ starts;
//! field accesses copy one field and hold no reference across a callback.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::struct_field_names
)]
use crate::command_type::DoCommandFlag;
use crate::company_type::CompanyID;
use crate::debug::debug;
use crate::engine_type::EngineID;
use crate::group::{GroupFlag, GroupFlags};
use crate::group_type::GroupID;
use crate::livery::{Livery, PRIMARY, SECONDARY};
use crate::raw_field::{read_field, write_field};
use crate::vehicle::{OpenTTDVehicle, VehicleRef};
use crate::vehicle_base::is_ground_vehicle;
use crate::vehicle_type::{VehicleID, VehicleType};
use std::collections::{BTreeMap, BTreeSet};
const NONE: u32 = u32::MAX;
/// `VehicleID::Invalid()` (`vehicle_type.h`), the list-selection sentinel.
pub(crate) const INVALID_VEHICLE: u32 = VehicleID::INVALID.0;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GroupFields {
    pub owner: u8,
    pub vehicle_type: u8,
    pub flags: u8,
    pub livery: Livery,
    pub parent: u16,
    pub number: u16,
}
#[repr(C)]
struct Group {
    fields: GroupFields,
    name: Vec<u8>,
    children: BTreeSet<u16>,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StatsFields {
    pub profit: i64,
    pub profit_min_age: i64,
    pub vehicles: u16,
    pub vehicles_min_age: u16,
    pub defined: bool,
    pub finished: bool,
}
#[repr(C)]
struct Stats {
    fields: StatsFields,
    engines: BTreeMap<u16, u16>,
}
crate::abi::shared_records! {
    fn abi_record;
    /// Canonical replacement node; the company owns the list head independently.
    #[derive(Clone, Copy)]
    "FleetRenewFields" ["fleet"] => pub struct Renew {
        pub from: u16 => "EngineID",
        pub to: u16 => "EngineID",
        pub next: *mut () => "EngineRenew *",
        pub group_id: u16 => "GroupID",
        pub replace_when_old: bool,
    }
}
/// Create the canonical group allocation; C++ starts scalar prefix lifetimes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_group_create() -> *mut () {
    Box::into_raw(Box::new(Group {
        fields: GroupFields {
            owner: 255,
            vehicle_type: 255,
            flags: 0,
            livery: Livery::default(),
            parent: 65535,
            number: 0,
        },
        name: Vec::new(),
        children: BTreeSet::new(),
    }))
    .cast()
}
/// Release a unique group owner after shell/scalar accesses end.
/// # Safety
/// Live matching create result, destroyed once on the serial game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_group_destroy(p: *mut ()) {
    unsafe { drop(Box::from_raw(p.cast::<Group>())) }
}
/// Create the canonical stats allocation; C++ starts scalar prefix lifetimes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_stats_create() -> *mut () {
    Box::into_raw(Box::new(Stats {
        fields: StatsFields::default(),
        engines: BTreeMap::new(),
    }))
    .cast()
}
/// Release a unique stats owner after shell/scalar accesses end.
/// # Safety
/// Live matching create result, destroyed once on the serial game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_destroy(p: *mut ()) {
    unsafe { drop(Box::from_raw(p.cast::<Stats>())) }
}
/// Create the canonical renew allocation; C++ starts scalar prefix lifetimes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_renew_create() -> *mut () {
    Box::into_raw(Box::new(Renew {
        from: 65535,
        to: 65535,
        next: std::ptr::null_mut(),
        group_id: 65535,
        replace_when_old: false,
    }))
    .cast()
}
/// Release a unique renew owner after shell/scalar accesses end.
/// # Safety
/// Live matching create result, destroyed once on the serial game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_renew_destroy(p: *mut ()) {
    unsafe { drop(Box::from_raw(p.cast::<Renew>())) }
}
/// Create the canonical head allocation; C++ starts scalar prefix lifetimes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_head_create() -> *mut () {
    Box::into_raw(Box::new(std::ptr::null_mut::<()>())).cast()
}
/// Release a unique head owner after shell/scalar accesses end.
/// # Safety
/// Live matching create result, destroyed once on the serial game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_head_destroy(p: *mut ()) {
    unsafe { drop(Box::from_raw(p.cast::<*mut ()>())) }
}
/// Copy detached statistics without sharing their allocation lifetime.
/// # Safety
/// Live statistics owners, serial access, no active borrows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_copy(to: *mut (), from: *const ()) {
    if std::ptr::eq(to, from.cast_mut()) {
        return;
    }
    let a = unsafe { &*from.cast::<Stats>() };
    let b = unsafe { &mut *to.cast::<Stats>() };
    b.fields = a.fields;
    b.engines.clone_from(&a.engines);
}
/// Preserve the original shallow renewal-list copy while separating head lifetime.
/// # Safety
/// Live head allocations; serial access without active borrows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_head_copy(to: *mut (), from: *const ()) {
    unsafe { to.cast::<*mut ()>().write(from.cast::<*mut ()>().read()) }
}
/// Copy the owned byte name for GUI/save adapters.
/// # Safety
/// Live group and writable output for length bytes, or null output for size query.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_name(
    p: *const (),
    out: *mut u8,
    length: usize,
) -> usize {
    let g = unsafe { &*p.cast::<Group>() };
    if !out.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(g.name.as_ptr(), out, length) }
    }
    g.name.len()
}
/// Replace name at the original mutation point.
/// # Safety
/// Live exclusive group; readable bytes of length, no reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_set_name(p: *mut (), bytes: *const u8, length: usize) {
    let g = unsafe { &mut *p.cast::<Group>() };
    g.name = unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec();
}
/// First ordered child at or above a typed pool ID, without retaining a borrow.
/// # Safety
/// Live group, serial access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_child(p: *const (), n: u32) -> u32 {
    unsafe { &*p.cast::<Group>() }
        .children
        .range(n as u16..)
        .next()
        .map_or(NONE, |&v| u32::from(v))
}
/// Original sorted `FlatSet` insertion/removal.
/// # Safety
/// Live group, exclusive container access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_child_change(p: *mut (), id: u32, insert: bool) {
    let g = unsafe { &mut *p.cast::<Group>() };
    if insert {
        g.children.insert(id as u16);
    } else {
        g.children.remove(&(id as u16));
    }
}
/// Cached engine count, including retained zero entries.
/// # Safety
/// Live statistics, serial access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_engine_count(p: *const (), engine: u16) -> u16 {
    unsafe { &*p.cast::<Stats>() }
        .engines
        .get(&engine)
        .copied()
        .unwrap_or(0)
}
/// Clear the original clear cache fields.
/// # Safety
/// Live statistics allocation, exclusive serial access without callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_clear(p: *mut ()) {
    let s = unsafe { &mut *p.cast::<Stats>() };
    s.fields.profit = 0;
    s.fields.profit_min_age = 0;
    s.fields.vehicles = 0;
    s.fields.vehicles_min_age = 0;
    s.engines.clear();
}
/// Clear the original `clear_profits` cache fields.
/// # Safety
/// Live statistics allocation, exclusive serial access without callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_clear_profits(p: *mut ()) {
    let s = unsafe { &mut *p.cast::<Stats>() };
    s.fields.profit = 0;
    s.fields.profit_min_age = 0;
    s.fields.vehicles_min_age = 0;
}
/// Clear the original `clear_autoreplace` cache fields.
/// # Safety
/// Live statistics allocation, exclusive serial access without callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_clear_autoreplace(p: *mut ()) {
    let s = unsafe { &mut *p.cast::<Stats>() };
    s.fields.defined = false;
    s.fields.finished = false;
}
/// Adjust a retained engine-map entry with the original uint16 truncation.
/// # Safety
/// Live exclusive statistics; game-thread access without callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_engine_change(p: *mut (), engine: u16, delta: i32) {
    let s = unsafe { &mut *p.cast::<Stats>() };
    let value = s.engines.entry(engine).or_default();
    *value = (i32::from(*value).wrapping_add(delta)) as u16;
}

#[repr(C)]
pub struct GroupServices {
    pub(crate) group: unsafe extern "C" fn(u16) -> *mut (),
    pub(crate) next_group: unsafe extern "C" fn(u32) -> u32,
    pub(crate) next_company: unsafe extern "C" fn(u32) -> u32,
    pub(crate) vehicle: unsafe extern "C" fn(u32) -> *mut (),
    /// Resolves shells to their canonical records.
    pub(crate) vehicles: &'static crate::vehicle::VehicleServices,
    pub(crate) vehicle_id: unsafe extern "C" fn(*mut ()) -> u32,
    pub(crate) vehicle_group: unsafe extern "C" fn(*mut ()) -> u16,
    pub(crate) profit: unsafe extern "C" fn(*mut ()) -> i64,
    pub(crate) old_enough: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) primary: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) countable: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) first_shared: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) next_shared: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) set_membership: unsafe extern "C" fn(*mut (), u16),
    pub(crate) invalidate_cache: unsafe extern "C" fn(*mut ()),
    pub(crate) viewport: unsafe extern "C" fn(*mut ()),
    pub(crate) stats: unsafe extern "C" fn(u8, u16, u8) -> *mut (),
    pub(crate) head: unsafe extern "C" fn(u8) -> *mut (),
    pub(crate) renew_state: unsafe extern "C" fn(*mut ()) -> *mut (),
    pub(crate) next_renew: unsafe extern "C" fn(u32) -> *mut (),
    pub(crate) renew_id: unsafe extern "C" fn(*mut ()) -> u32,
    pub(crate) buildable_type: unsafe extern "C" fn(u8) -> bool,
    pub(crate) can_allocate: unsafe extern "C" fn() -> bool,
    pub(crate) allocate: unsafe extern "C" fn(u8, u8) -> u16,
    pub(crate) use_number: unsafe extern "C" fn(u8) -> u16,
    pub(crate) release_number: unsafe extern "C" fn(u8, u16),
    pub(crate) company_livery: unsafe extern "C" fn(u8) -> *const Livery,
    pub(crate) keep_length: unsafe extern "C" fn(u8) -> bool,
    pub(crate) delete_group: unsafe extern "C" fn(u16),
    pub(crate) clear_backup: unsafe extern "C" fn(u16),
    pub(crate) remove_rule: unsafe extern "C" fn(u8, u16, u16, u32),
    pub(crate) remove_vehicles: unsafe extern "C" fn(u16, u32),
    pub(crate) delete_child: unsafe extern "C" fn(u16, u32),
    pub(crate) add_to_group: unsafe extern "C" fn(u16, u32, u32),
    pub(crate) list_dirty: unsafe extern "C" fn(u8, u8),
    pub(crate) list_set_dirty: unsafe extern "C" fn(u8, u8),
    pub(crate) colour_dirty: unsafe extern "C" fn(u8, u8),
    pub(crate) replace_dirty: unsafe extern "C" fn(u8),
    pub(crate) replace_invalidate: unsafe extern "C" fn(u8),
    pub(crate) alter_dirty: unsafe extern "C" fn(u8),
    pub(crate) vehicle_dirty: unsafe extern "C" fn(u32),
    pub(crate) depot_dirty: unsafe extern "C" fn(*mut ()),
    pub(crate) close_replace: unsafe extern "C" fn(u8),
    pub(crate) shared: *const crate::services::Services,
    pub(crate) list_generate: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) list_size: unsafe extern "C" fn(*mut ()) -> usize,
    pub(crate) list_at: unsafe extern "C" fn(*mut (), usize) -> *mut (),
    pub(crate) renew_allocate: unsafe extern "C" fn() -> *mut (),
    pub(crate) renew_can_allocate: unsafe extern "C" fn() -> bool,
    pub(crate) renew_delete: unsafe extern "C" fn(*mut ()),
    pub(crate) recursion_error: u32,
}

// The shared table's canonical vehicle iteration, company and UTF-8 services.
impl GroupServices {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: The immutable shared table outlives every serialized fleet entry.
        unsafe { &*self.shared }
    }
    /// The first vehicle at or after `from`, as `Vehicle::Iterate(from)`.
    pub(crate) fn next_vehicle(&self, from: u32) -> *mut OpenTTDVehicle {
        (self.shared().vehicle_next)(from).map_or(std::ptr::null_mut(), |v| v.0.as_ptr())
    }
    /// `_current_company`.
    pub(crate) fn current_company(&self) -> u8 {
        (self.shared().current_company)()
    }
    /// `Utf8StringLength` of a validated span.
    pub(crate) fn utf8_length(&self, text: *const u8, len: usize) -> usize {
        (self.shared().utf8_length)(text, len)
    }
    pub(crate) fn engine(&self, id: u16) -> crate::vehicle::EngineRef {
        // SAFETY: this serialized entry retains the immutable resolver table and live engine.
        let capabilities = unsafe { (&*(self.vehicles.newgrf)()).capabilities };
        crate::vehicle::EngineRef::of((capabilities.engine)(id))
    }
}

const INVALID_GROUP: u16 = 65535;
pub(crate) const DEFAULT_GROUP: u16 = 65534;
pub(crate) const ALL_GROUP: u16 = 65533;
const OK: u32 = u32::MAX;
const ERROR: u32 = 65535;

/// Canonical record of a live native shell.
pub(crate) fn record(w: &GroupServices, v: *mut ()) -> VehicleRef {
    // SAFETY: Every caller passes a live shell of the serialized game state.
    unsafe { VehicleRef::from_shell(v.cast(), w.vehicles) }
}
/// `v->Next()` as a native shell.
pub(crate) fn next_part(v: VehicleRef) -> *mut () {
    v.next()
        .map_or(std::ptr::null_mut(), |next| next.shell().cast())
}

// Raw field access permits native aliases; every access ends before a callback.
unsafe fn group(w: &GroupServices, id: u16) -> *mut GroupFields {
    unsafe { (w.group)(id).cast() }
}
unsafe fn child_after(g: *mut GroupFields, from: u32) -> Option<u16> {
    if from > 65535 {
        return None;
    }
    let id = unsafe { openttd_rust_fleet_child(g.cast(), from) };
    (id != NONE).then_some(id as u16)
}
unsafe fn children_snapshot(g: *mut GroupFields) -> Vec<u16> {
    unsafe { &*g.cast::<()>().cast::<Group>() }
        .children
        .iter()
        .copied()
        .collect()
}
unsafe fn contains(w: &GroupServices, mut search: u16, target: u16) -> bool {
    let mut g = unsafe { group(w, search) };
    if g.is_null() {
        return search == target;
    }
    loop {
        if search == target {
            return true;
        }
        search = read_field!(g, parent);
        if search == INVALID_GROUP {
            return false;
        }
        g = unsafe { group(w, search) };
    }
}
unsafe fn vehicle_stats(w: &GroupServices, v: VehicleRef, all: bool) -> *mut Stats {
    let owner = v.owner();
    let id = if all {
        ALL_GROUP
    } else {
        unsafe { (w.vehicle_group)(v.shell().cast()) }
    };
    let kind = v.vehicle_type();
    unsafe { (w.stats)(owner, id, kind).cast() }
}
unsafe fn count_vehicle(w: &GroupServices, v: *mut (), delta: i32) {
    let old = unsafe { (w.old_enough)(v) };
    let profit = unsafe { (w.profit)(v) }.saturating_mul(i64::from(delta));
    let v = record(w, v);
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    for p in [all, stats] {
        // Money uses the original OverflowSafeInt64 saturation; uint16 counts wrap.
        // No callback while these field-sized accesses are active.
        let s = unsafe { &mut *p };
        s.fields.vehicles = (i32::from(s.fields.vehicles).wrapping_add(delta)) as u16;
        s.fields.profit = s.fields.profit.saturating_add(profit);
        if old {
            s.fields.vehicles_min_age =
                (i32::from(s.fields.vehicles_min_age).wrapping_add(delta)) as u16;
            s.fields.profit_min_age = s.fields.profit_min_age.saturating_add(profit);
        }
    }
}
unsafe fn count_engine(w: &GroupServices, v: *mut (), delta: i32) {
    let v = record(w, v);
    let engine = v.engine_id();
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    unsafe {
        openttd_rust_fleet_engine_change(all.cast(), engine, delta);
        openttd_rust_fleet_engine_change(stats.cast(), engine, delta);
    }
}
unsafe fn add_profit(w: &GroupServices, v: *mut ()) {
    let profit = unsafe { (w.profit)(v) };
    let v = record(w, v);
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    for p in [all, stats] {
        unsafe { (*p).fields.profit = (*p).fields.profit.saturating_add(profit) };
    }
}
unsafe fn min_age(w: &GroupServices, v: *mut ()) {
    let profit = unsafe { (w.profit)(v) };
    let v = record(w, v);
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    for p in [all, stats] {
        let s = unsafe { &mut *p };
        s.fields.vehicles_min_age = s.fields.vehicles_min_age.wrapping_add(1);
        s.fields.profit_min_age = s.fields.profit_min_age.saturating_add(profit);
    }
}
unsafe fn engine_membership(w: &GroupServices, v: VehicleRef, old: u16, new: u16) {
    if old == new {
        return;
    }
    let owner = v.owner();
    let kind = v.vehicle_type();
    let engine = v.engine_id();
    let old_stats = unsafe { (w.stats)(owner, old, kind) };
    unsafe { openttd_rust_fleet_engine_change(old_stats, engine, -1) };
    let new_stats = unsafe { (w.stats)(owner, new, kind) };
    unsafe { openttd_rust_fleet_engine_change(new_stats, engine, 1) };
}
unsafe fn sum_engines(w: &GroupServices, company: u8, id: u16, engine: u16) -> u32 {
    let mut count = 0_u32;
    let g = unsafe { group(w, id) };
    if !g.is_null() {
        let mut from = 0;
        while let Some(child) = unsafe { child_after(g, from) } {
            count = count.wrapping_add(unsafe { sum_engines(w, company, child, engine) });
            from = u32::from(child) + 1;
        }
    }
    let kind = w.engine(engine).kind().0;
    let s = unsafe { (w.stats)(company, id, kind) };
    count.wrapping_add(u32::from(unsafe {
        openttd_rust_fleet_engine_count(s, engine)
    }))
}
unsafe fn sum_vehicles(w: &GroupServices, company: u8, id: u16, kind: u8) -> u32 {
    let mut count = 0_u32;
    let g = unsafe { group(w, id) };
    if !g.is_null() {
        let mut from = 0;
        while let Some(child) = unsafe { child_after(g, from) } {
            count = count.wrapping_add(unsafe { sum_vehicles(w, company, child, kind) });
            from = u32::from(child) + 1;
        }
    }
    let s = unsafe { (w.stats)(company, id, kind).cast::<Stats>() };
    count.wrapping_add(u32::from(unsafe { (*s).fields.vehicles }))
}
unsafe fn sum_min_age(w: &GroupServices, company: u8, id: u16, kind: u8) -> u32 {
    let mut count = 0_u32;
    let g = unsafe { group(w, id) };
    if !g.is_null() {
        let mut from = 0;
        while let Some(child) = unsafe { child_after(g, from) } {
            count = count.wrapping_add(unsafe { sum_min_age(w, company, child, kind) });
            from = u32::from(child) + 1;
        }
    }
    let s = unsafe { (w.stats)(company, id, kind).cast::<Stats>() };
    count.wrapping_add(u32::from(unsafe { (*s).fields.vehicles_min_age }))
}
unsafe fn sum_profit(w: &GroupServices, company: u8, id: u16, kind: u8) -> i64 {
    let mut sum = 0_i64;
    let g = unsafe { group(w, id) };
    if !g.is_null() {
        let mut from = 0;
        while let Some(child) = unsafe { child_after(g, from) } {
            sum = sum.saturating_add(unsafe { sum_profit(w, company, child, kind) });
            from = u32::from(child) + 1;
        }
    }
    let s = unsafe { (w.stats)(company, id, kind).cast::<Stats>() };
    sum.saturating_add(unsafe { (*s).fields.profit_min_age })
}
unsafe fn update_autoreplace(w: &GroupServices, company: u8) {
    for kind in 0..4 {
        let all = unsafe { (w.stats)(company, ALL_GROUP, kind) };
        unsafe { openttd_rust_fleet_stats_clear_autoreplace(all) };
        let default = unsafe { (w.stats)(company, DEFAULT_GROUP, kind) };
        unsafe { openttd_rust_fleet_stats_clear_autoreplace(default) };
    }
    let mut from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        let g = unsafe { group(w, id as u16) };
        if read_field!(g, owner) == company {
            let kind = read_field!(g, vehicle_type);
            let s = unsafe { (w.stats)(company, id as u16, kind) };
            unsafe { openttd_rust_fleet_stats_clear_autoreplace(s) };
        }
        from = id + 1;
    }
    let head = unsafe { (w.head)(company) };
    let mut er = unsafe { head.cast::<*mut ()>().read() };
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let engine = read_field!(p, from);
        let id = read_field!(p, group_id);
        let next = read_field!(p, next);
        let kind = w.engine(engine).kind().0;
        let s = unsafe { (w.stats)(company, id, kind).cast::<Stats>() };
        unsafe {
            if !(*s).fields.defined {
                (*s).fields.defined = true;
                (*s).fields.finished = true;
            }
        }
        let count = unsafe { sum_engines(w, company, id, engine) };
        if count > 0 {
            unsafe {
                (*s).fields.finished = false;
            }
        }
        er = next;
    }
}
unsafe fn train_group(w: &GroupServices, v: *mut (), new: u16, viewport: bool) {
    let v = record(w, v);
    let owner = v.owner();
    let mut part = Some(v);
    while let Some(r) = part {
        let u = r.shell().cast();
        let old = unsafe { (w.vehicle_group)(u) };
        if unsafe { (w.countable)(u) } {
            unsafe { engine_membership(w, r, old, new) };
        }
        unsafe {
            (w.set_membership)(u, new);
            (w.invalidate_cache)(u);
        }
        if viewport {
            unsafe { (w.viewport)(u) };
        }
        part = r.next();
    }
    unsafe {
        update_autoreplace(w, owner);
        (w.replace_dirty)(0);
    }
}
unsafe fn add_vehicle(w: &GroupServices, v: *mut (), new: u16) {
    unsafe { count_vehicle(w, v, -1) };
    let record = record(w, v);
    match record.vehicle_type() {
        0 => unsafe { train_group(w, v, new, true) },
        1..=3 => {
            let old = unsafe { (w.vehicle_group)(v) };
            if unsafe { (w.countable)(v) } {
                unsafe { engine_membership(w, record, old, new) };
            }
            unsafe { (w.set_membership)(v, new) };
            let mut part = Some(record);
            while let Some(r) = part {
                let u = r.shell().cast();
                unsafe {
                    (w.invalidate_cache)(u);
                    (w.viewport)(u);
                }
                part = r.next();
            }
        }
        _ => unreachable!(),
    }
    let id = unsafe { (w.vehicle_id)(v) };
    unsafe {
        (w.vehicle_dirty)(id);
        count_vehicle(w, v, 1);
    }
}
unsafe fn parent_livery(w: &GroupServices, g: *mut GroupFields) -> *const Livery {
    let parent = read_field!(g, parent);
    if parent == INVALID_GROUP {
        let owner = read_field!(g, owner);
        unsafe { (w.company_livery)(owner) }
    } else {
        let p = unsafe { group(w, parent) };
        unsafe { std::ptr::addr_of!((*p).livery) }
    }
}
unsafe fn propagate_livery(w: &GroupServices, id: u16, reset: bool) {
    let g = unsafe { group(w, id) };
    if reset {
        let mut from = 0;
        while let Some(record) = std::ptr::NonNull::new(w.next_vehicle(from)) {
            let record = VehicleRef(record);
            let v = record.shell().cast();
            from = unsafe { (w.vehicle_id)(v) } + 1;
            if unsafe { (w.vehicle_group)(v) } == id
                && (!is_ground_vehicle(record.vehicle_type()) || record.is_front_engine())
            {
                let mut part = Some(record);
                while let Some(r) = part {
                    unsafe { (w.invalidate_cache)(r.shell().cast()) };
                    part = r.next();
                }
            }
        }
    }
    let mut from = 0;
    while let Some(child) = unsafe { child_after(g, from) } {
        let cg = unsafe { group(w, child) };
        if read_field!(cg, livery.in_use) & PRIMARY == 0 {
            write_field!(cg, livery.colour1, read_field!(g, livery.colour1));
        }
        if read_field!(cg, livery.in_use) & SECONDARY == 0 {
            write_field!(cg, livery.colour2, read_field!(g, livery.colour2));
        }
        unsafe { propagate_livery(w, child, reset) };
        from = u32::from(child) + 1;
    }
}
unsafe fn create_group(w: &GroupServices, flags: u32, kind: u8, parent: u16, out: *mut u16) -> u32 {
    unsafe { out.write(INVALID_GROUP) };
    if !unsafe { (w.buildable_type)(kind) } || !unsafe { (w.can_allocate)() } {
        return ERROR;
    }
    let pg = unsafe { group(w, parent) };
    let owner = w.current_company();
    if !pg.is_null() && (read_field!(pg, owner) != owner || read_field!(pg, vehicle_type) != kind) {
        return ERROR;
    }
    if flags & 1 != 0 {
        let id = unsafe { (w.allocate)(owner, kind) };
        let g = unsafe { group(w, id) };
        let number = unsafe { (w.use_number)(owner) };
        write_field!(g, number, number);
        if pg.is_null() {
            let parent = unsafe { (w.company_livery)(owner) };
            write_field!(g, livery.colour1, read_field!(parent, colour1));
            write_field!(g, livery.colour2, read_field!(parent, colour2));
            if unsafe { (w.keep_length)(owner) } {
                write_field!(g, flags, read_field!(g, flags) | 2);
            }
        } else {
            write_field!(g, parent, parent);
            write_field!(g, livery.colour1, read_field!(pg, livery.colour1));
            write_field!(g, livery.colour2, read_field!(pg, livery.colour2));
            write_field!(g, flags, read_field!(pg, flags));
            unsafe { openttd_rust_fleet_child_change(pg.cast(), u32::from(id), true) };
        }
        unsafe {
            (w.list_dirty)(owner, kind);
            (w.colour_dirty)(owner, kind);
            out.write(id);
        }
    }
    OK
}
unsafe fn delete_group(w: &GroupServices, flags: u32, id: u16) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = w.current_company();
    if g.is_null() || read_field!(g, owner) != owner {
        return ERROR;
    }
    unsafe { (w.remove_vehicles)(id, flags) };
    let children = unsafe { children_snapshot(g) };
    for child in children {
        unsafe { (w.delete_child)(child, flags) };
    }
    if flags & 1 != 0 {
        unsafe { (w.clear_backup)(id) };
        let owner = read_field!(g, owner);
        if owner < 15 {
            let mut from = 0;
            loop {
                let er = unsafe { (w.next_renew)(from) };
                if er.is_null() {
                    break;
                }
                from = unsafe { (w.renew_id)(er) } + 1;
                let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
                if read_field!(p, group_id) == id {
                    let engine = read_field!(p, from);
                    unsafe { (w.remove_rule)(owner, engine, id, flags) };
                }
            }
            let number = read_field!(g, number);
            unsafe { (w.release_number)(owner, number) };
        }
        let parent = read_field!(g, parent);
        if parent != INVALID_GROUP {
            let pg = unsafe { group(w, parent) };
            unsafe { openttd_rust_fleet_child_change(pg.cast(), u32::from(id), false) };
        }
        let kind = read_field!(g, vehicle_type);
        unsafe {
            (w.close_replace)(kind);
            (w.delete_group)(id);
            (w.list_dirty)(owner, kind);
            (w.colour_dirty)(owner, kind);
        }
    }
    OK
}
unsafe fn alter_group(
    w: &GroupServices,
    flags: u32,
    mode: u8,
    id: u16,
    parent: u16,
    text: *const u8,
    len: usize,
) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = w.current_company();
    if g.is_null() || read_field!(g, owner) != owner {
        return ERROR;
    }
    if mode == 0 {
        if len != 0 && w.utf8_length(text, len) >= 32 {
            return ERROR;
        }
        if flags & 1 != 0 {
            unsafe { openttd_rust_fleet_set_name(g.cast(), text, len) };
        }
    } else if mode == 1 {
        if read_field!(g, parent) == parent {
            return OK;
        }
        let pg = unsafe { group(w, parent) };
        if !pg.is_null() {
            if read_field!(pg, owner) != owner
                || read_field!(pg, vehicle_type) != read_field!(g, vehicle_type)
            {
                return ERROR;
            }
            if unsafe { contains(w, parent, id) } {
                return w.recursion_error;
            }
        }
        if flags & 1 != 0 {
            let old_parent = read_field!(g, parent);
            if old_parent != INVALID_GROUP {
                let old = unsafe { group(w, old_parent) };
                unsafe { openttd_rust_fleet_child_change(old.cast(), u32::from(id), false) };
            }
            write_field!(g, parent, if pg.is_null() { INVALID_GROUP } else { parent });
            if !pg.is_null() {
                unsafe { openttd_rust_fleet_child_change(pg.cast(), u32::from(id), true) };
            }
            unsafe { update_autoreplace(w, owner) };
            if read_field!(g, livery.in_use) & (PRIMARY | SECONDARY) != (PRIMARY | SECONDARY) {
                let inherited = unsafe { parent_livery(w, g) };
                if read_field!(g, livery.in_use) & PRIMARY == 0 {
                    write_field!(g, livery.colour1, read_field!(inherited, colour1));
                }
                if read_field!(g, livery.in_use) & SECONDARY == 0 {
                    write_field!(g, livery.colour2, read_field!(inherited, colour2));
                }
                unsafe {
                    propagate_livery(w, id, true);
                    ((*w.shared).screen_dirty)();
                }
            }
        }
    } else {
        return ERROR;
    }
    if flags & 1 != 0 {
        let kind = read_field!(g, vehicle_type);
        unsafe {
            (w.alter_dirty)(kind);
            (w.list_dirty)(owner, kind);
            (w.colour_dirty)(owner, kind);
        }
    }
    OK
}
unsafe fn set_flag(w: &GroupServices, id: u16, flag: u8, value: bool, children: bool) {
    let g = unsafe { group(w, id) };
    let bits = read_field!(g, flags);
    write_field!(
        g,
        flags,
        if value {
            bits | (1 << flag)
        } else {
            bits & !(1 << flag)
        }
    );
    if !children {
        return;
    }
    let mut from = 0;
    while let Some(child) = unsafe { child_after(g, from) } {
        unsafe { set_flag(w, child, flag, value, true) };
        from = u32::from(child) + 1;
    }
}

unsafe fn update_children(w: &GroupServices) {
    let mut from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        from = id + 1;
        let g = unsafe { group(w, id as u16) };
        let parent = read_field!(g, parent);
        if parent == INVALID_GROUP {
            continue;
        }
        let pg = unsafe { group(w, parent) };
        if pg.is_null()
            || read_field!(pg, owner) != read_field!(g, owner)
            || read_field!(pg, vehicle_type) != read_field!(g, vehicle_type)
        {
            debug!(misc, 2, "Group {} has invalid parent {}", id as u16, parent);
            write_field!(g, parent, INVALID_GROUP);
        } else {
            unsafe { openttd_rust_fleet_child_change(pg.cast(), id, true) };
        }
    }
}
unsafe fn update_afterload(w: &GroupServices) {
    let mut from = 0;
    loop {
        let company = unsafe { (w.next_company)(from) };
        if company == NONE {
            break;
        }
        from = company + 1;
        for kind in 0..4 {
            let p = unsafe { (w.stats)(company as u8, ALL_GROUP, kind) };
            unsafe { openttd_rust_fleet_stats_clear(p) };
            let p = unsafe { (w.stats)(company as u8, DEFAULT_GROUP, kind) };
            unsafe { openttd_rust_fleet_stats_clear(p) };
        }
    }
    from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        from = id + 1;
        let g = unsafe { group(w, id as u16) };
        let owner = read_field!(g, owner);
        let kind = read_field!(g, vehicle_type);
        let p = unsafe { (w.stats)(owner, id as u16, kind) };
        unsafe { openttd_rust_fleet_stats_clear(p) };
    }
    from = 0;
    loop {
        let v = crate::vehicle::VehicleRef::shell_from_record(w.next_vehicle(from));
        if v.is_null() {
            break;
        }
        from = unsafe { (w.vehicle_id)(v) } + 1;
        if !unsafe { (w.countable)(v) } {
            continue;
        }
        unsafe { count_engine(w, v, 1) };
        if unsafe { (w.primary)(v) } {
            unsafe { count_vehicle(w, v, 1) };
        }
    }
    from = 0;
    loop {
        let company = unsafe { (w.next_company)(from) };
        if company == NONE {
            break;
        }
        from = company + 1;
        unsafe { update_autoreplace(w, company as u8) };
    }
}
unsafe fn update_profits(w: &GroupServices) {
    let mut from = 0;
    loop {
        let company = unsafe { (w.next_company)(from) };
        if company == NONE {
            break;
        }
        from = company + 1;
        for kind in 0..4 {
            let p = unsafe { (w.stats)(company as u8, ALL_GROUP, kind) };
            unsafe { openttd_rust_fleet_stats_clear_profits(p) };
            let p = unsafe { (w.stats)(company as u8, DEFAULT_GROUP, kind) };
            unsafe { openttd_rust_fleet_stats_clear_profits(p) };
        }
    }
    from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        from = id + 1;
        let g = unsafe { group(w, id as u16) };
        let owner = read_field!(g, owner);
        let kind = read_field!(g, vehicle_type);
        let p = unsafe { (w.stats)(owner, id as u16, kind) };
        unsafe { openttd_rust_fleet_stats_clear_profits(p) };
    }
    from = 0;
    loop {
        let v = crate::vehicle::VehicleRef::shell_from_record(w.next_vehicle(from));
        if v.is_null() {
            break;
        }
        from = unsafe { (w.vehicle_id)(v) } + 1;
        if unsafe { (w.primary)(v) } {
            unsafe { add_profit(w, v) };
            if unsafe { (w.old_enough)(v) } {
                unsafe { min_age(w, v) };
            }
        }
    }
}
pub(crate) unsafe fn company_liveries(w: &GroupServices, company: u8) {
    let mut from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        from = id + 1;
        let g = unsafe { group(w, id as u16) };
        if read_field!(g, owner) == company && read_field!(g, parent) == INVALID_GROUP {
            let inherited = unsafe { (w.company_livery)(company) };
            if read_field!(g, livery.in_use) & PRIMARY == 0 {
                write_field!(g, livery.colour1, read_field!(inherited, colour1));
            }
            if read_field!(g, livery.in_use) & SECONDARY == 0 {
                write_field!(g, livery.colour2, read_field!(inherited, colour2));
            }
            unsafe { propagate_livery(w, id as u16, false) };
        }
    }
}
enum VehicleList {
    Generated(*mut ()),
    Single(*mut ()),
}
impl VehicleList {
    unsafe fn size(&self, w: &GroupServices) -> usize {
        match *self {
            Self::Generated(context) => unsafe { (w.list_size)(context) },
            Self::Single(_) => 1,
        }
    }
    unsafe fn at(&self, w: &GroupServices, index: usize) -> *mut () {
        match *self {
            Self::Generated(context) => unsafe { (w.list_at)(context, index) },
            Self::Single(v) => v,
        }
    }
}
unsafe fn add_vehicle_group(
    w: &GroupServices,
    flags: u32,
    id: u16,
    veh_id: u32,
    shared: bool,
    context: *mut (),
    vli_valid: bool,
    out: *mut u16,
) -> u32 {
    unsafe { out.write(GroupID::INVALID.0) };
    let veh_id = VehicleID(veh_id);
    let mut new = GroupID(id);
    if unsafe { group(w, new.0) }.is_null()
        && new != GroupID::DEFAULT_GROUP
        && new != GroupID::NEW_GROUP
    {
        return ERROR;
    }
    // The original's command-local `VehicleList`: generated from the
    // identifier, or holding the single named vehicle.
    let list = if veh_id == VehicleID::INVALID && vli_valid {
        if !unsafe { (w.list_generate)(context) } || unsafe { (w.list_size)(context) } == 0 {
            return ERROR;
        }
        VehicleList::Generated(context)
    } else {
        let v = unsafe { (w.vehicle)(veh_id.0) };
        if v.is_null() {
            return ERROR;
        }
        VehicleList::Single(v)
    };
    let first = unsafe { list.at(w, 0) };
    let kind = VehicleType(record(w, first).vehicle_type());
    let owner = CompanyID(w.current_company());
    let size = unsafe { list.size(w) };
    for i in 0..size {
        let v = unsafe { list.at(w, i) };
        if CompanyID(record(w, v).owner()) != owner || !unsafe { (w.primary)(v) } {
            return ERROR;
        }
    }
    let g = unsafe { group(w, new.0) };
    if !g.is_null()
        && (CompanyID(read_field!(g, owner)) != owner
            || VehicleType(read_field!(g, vehicle_type)) != kind)
    {
        return ERROR;
    }
    if new == GroupID::NEW_GROUP {
        // Create new group.
        let ret = unsafe { create_group(w, flags, kind.0, GroupID::INVALID.0, &raw mut new.0) };
        if ret != OK {
            unsafe { out.write(new.0) };
            return ret;
        }
    }
    if flags & u32::from(DoCommandFlag::EXECUTE) != 0 {
        for i in 0..size {
            let vc = unsafe { list.at(w, i) };
            // VehicleList is const but we need to modify the vehicle.
            let vehicle_id = VehicleID(unsafe { (w.vehicle_id)(vc) });
            let v = unsafe { (w.vehicle)(vehicle_id.0) };
            unsafe { add_vehicle(w, v, new.0) };
            if shared {
                // Add vehicles in the shared order list as well.
                let mut v2 = unsafe { (w.first_shared)(v) };
                while !v2.is_null() {
                    if GroupID(unsafe { (w.vehicle_group)(v2) }) != new {
                        unsafe { add_vehicle(w, v2, new.0) };
                    }
                    v2 = unsafe { (w.next_shared)(v2) };
                }
            }
            unsafe { (w.depot_dirty)(v) };
        }
        unsafe {
            update_autoreplace(w, owner.0);
            // Update the Replace Vehicle Windows.
            (w.replace_dirty)(kind.0);
            (w.list_dirty)(owner.0, kind.0);
        }
    }
    unsafe { out.write(new.0) };
    OK
}
unsafe fn add_shared_group(w: &GroupServices, flags: u32, id: u16, kind: u8) -> u32 {
    if unsafe { group(w, id) }.is_null() || !unsafe { (w.buildable_type)(kind) } {
        return ERROR;
    }
    if flags & 1 != 0 {
        let mut from = 0;
        while let Some(record) = std::ptr::NonNull::new(w.next_vehicle(from)) {
            let record = VehicleRef(record);
            let v = record.shell().cast();
            from = unsafe { (w.vehicle_id)(v) } + 1;
            if record.vehicle_type() != kind
                || !unsafe { (w.primary)(v) }
                || unsafe { (w.vehicle_group)(v) } != id
            {
                continue;
            }
            let mut v2 = unsafe { (w.first_shared)(v) };
            while !v2.is_null() {
                if unsafe { (w.vehicle_group)(v2) } != id {
                    let vehicle_id = unsafe { (w.vehicle_id)(v2) };
                    unsafe { (w.add_to_group)(id, vehicle_id, flags) };
                }
                v2 = unsafe { (w.next_shared)(v2) };
            }
        }
        let owner = w.current_company();
        unsafe { (w.list_dirty)(owner, kind) };
    }
    OK
}
unsafe fn remove_vehicles_group(w: &GroupServices, flags: u32, id: u16) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = w.current_company();
    if g.is_null() || read_field!(g, owner) != owner {
        return ERROR;
    }
    if flags & 1 != 0 {
        let mut from = 0;
        loop {
            let v = crate::vehicle::VehicleRef::shell_from_record(w.next_vehicle(from));
            if v.is_null() {
                break;
            }
            from = unsafe { (w.vehicle_id)(v) } + 1;
            if !unsafe { (w.primary)(v) } || unsafe { (w.vehicle_group)(v) } != id {
                continue;
            }
            let vehicle_id = unsafe { (w.vehicle_id)(v) };
            unsafe { (w.add_to_group)(DEFAULT_GROUP, vehicle_id, flags) };
        }
        let kind = read_field!(g, vehicle_type);
        unsafe { (w.list_dirty)(owner, kind) };
    }
    OK
}
unsafe fn set_livery(w: &GroupServices, flags: u32, id: u16, primary: bool, mut colour: u8) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = w.current_company();
    if g.is_null() || read_field!(g, owner) != owner || (colour >= 16 && colour != 255) {
        return ERROR;
    }
    if flags & 1 != 0 {
        let mask = if primary { PRIMARY } else { SECONDARY };
        if colour == 255 {
            write_field!(g, livery.in_use, read_field!(g, livery.in_use) & !mask);
            let inherited = unsafe { parent_livery(w, g) };
            colour = if primary {
                read_field!(inherited, colour1)
            } else {
                read_field!(inherited, colour2)
            };
        } else {
            write_field!(g, livery.in_use, read_field!(g, livery.in_use) | mask);
        }
        if primary {
            write_field!(g, livery.colour1, colour);
        } else {
            write_field!(g, livery.colour2, colour);
        }
        unsafe {
            propagate_livery(w, id, true);
            ((*w.shared).screen_dirty)();
        }
    }
    OK
}
unsafe fn flag_command(
    w: &GroupServices,
    flags: u32,
    id: u16,
    flag: u8,
    value: bool,
    recursive: bool,
) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = w.current_company();
    if g.is_null() || read_field!(g, owner) != owner || flag > 1 {
        return ERROR;
    }
    if flags & 1 != 0 {
        unsafe { set_flag(w, id, flag, value, recursive) };
        let kind = read_field!(g, vehicle_type);
        unsafe {
            (w.list_set_dirty)(owner, kind);
            (w.replace_invalidate)(kind);
        }
    }
    OK
}
/// Retrieves the `EngineRenew` that specifies the replacement of the given
/// engine type from the given renewlist
unsafe fn get_engine_replacement(
    w: &GroupServices,
    mut er: *mut (),
    engine: u16,
    id: u16,
) -> *mut Renew {
    let engine = EngineID(engine);
    let id = GroupID(id);
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let target = read_field!(p, group_id);
        if EngineID(read_field!(p, from)) == engine && unsafe { contains(w, id.0, target) } {
            return p;
        }
        er = read_field!(p, next);
    }
    std::ptr::null_mut()
}
/// Retrieve the engine replacement in a given renewlist for an original engine type.
/// `when_old` is set if the replacement should be done when old.
/// Returns the engine type to replace with, or `EngineID::INVALID` if no
/// replacement is in the list.
unsafe fn engine_replacement(
    w: &GroupServices,
    er: *mut (),
    engine: u16,
    id: u16,
    when_old: *mut bool,
) -> u16 {
    let engine = EngineID(engine);
    let id = GroupID(id);
    let mut p = unsafe { get_engine_replacement(w, er, engine.0, id.0) };
    if p.is_null() {
        let g = unsafe { group(w, id.0) };
        if id == GroupID::DEFAULT_GROUP
            || (!g.is_null()
                && !GroupFlags(read_field!(g, flags)).test(GroupFlag::ReplaceProtection))
        {
            // We didn't find anything useful in the vehicle's own group so we will try ALL_GROUP
            p = unsafe { get_engine_replacement(w, er, engine.0, GroupID::ALL_GROUP.0) };
        }
    }
    if !when_old.is_null() {
        // Not replacing: false. When replacing with same model, only ever do it when old.
        // Otherwise use player setting.
        let old = !p.is_null()
            && (EngineID(read_field!(p, to)) == engine || read_field!(p, replace_when_old));
        unsafe { when_old.write(old) };
    }
    if p.is_null() {
        EngineID::INVALID.0
    } else {
        read_field!(p, to)
    }
}
/// Add an engine replacement to the given renewlist.
/// `from` is the original engine type, `to` the replacement engine type, `id` the
/// related group, `old` selects replacement when old or always, and `flags` are
/// the calling command flags. Returns success or command error.
unsafe fn add_engine_replacement(
    w: &GroupServices,
    head: *mut *mut (),
    from: u16,
    to: u16,
    id: u16,
    old: bool,
    flags: u32,
) -> u32 {
    let from = EngineID(from);
    let to = EngineID(to);
    let id = GroupID(id);
    // Check if the old vehicle is already in the list
    let p = unsafe { get_engine_replacement(w, head.read(), from.0, id.0) };
    if !p.is_null() {
        if flags & u32::from(DoCommandFlag::EXECUTE) != 0 {
            write_field!(p, to, to.0);
            write_field!(p, replace_when_old, old);
        }
        return OK;
    }
    if !unsafe { (w.renew_can_allocate)() } {
        return ERROR;
    }
    if flags & u32::from(DoCommandFlag::EXECUTE) != 0 {
        // Insert before the first element
        let shell = unsafe { (w.renew_allocate)() };
        let p = unsafe { (w.renew_state)(shell).cast::<Renew>() };
        write_field!(p, from, from.0);
        write_field!(p, to, to.0);
        write_field!(p, group_id, id.0);
        write_field!(p, replace_when_old, old);
        write_field!(p, next, unsafe { head.read() });
        unsafe { head.write(shell) };
    }
    OK
}
/// Remove an engine replacement from a given renewlist.
/// `engine` is the original engine type and `id` the related group.
/// `flags` are the calling command flags. Returns success or command error.
unsafe fn remove_engine_replacement(
    w: &GroupServices,
    head: *mut *mut (),
    engine: u16,
    id: u16,
    flags: u32,
) -> u32 {
    let engine = EngineID(engine);
    let id = GroupID(id);
    let mut er = unsafe { head.read() };
    let mut prev: *mut Renew = std::ptr::null_mut();
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let next = read_field!(p, next);
        if EngineID(read_field!(p, from)) == engine && GroupID(read_field!(p, group_id)) == id {
            if flags & u32::from(DoCommandFlag::EXECUTE) != 0 {
                if prev.is_null() {
                    // First element
                    // The second becomes the new first element
                    unsafe { head.write(next) };
                } else {
                    // Cut this element out
                    write_field!(prev, next, next);
                }
                unsafe { (w.renew_delete)(er) };
            }
            return OK;
        }
        prev = p;
        er = next;
    }
    ERROR
}
/// Remove all engine replacement settings for the company.
/// The renewlist is empty on return.
unsafe fn remove_all_engine_replacement(w: &GroupServices, head: *mut *mut ()) {
    let mut er = unsafe { head.read() };
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let next = read_field!(p, next);
        unsafe { (w.renew_delete)(er) };
        er = next;
    }
    unsafe { head.write(std::ptr::null_mut()) }; // Empty list
}

/// Synchronous original fleet `update_children` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_update_children(services: *const GroupServices) {
    let w = unsafe { &*services };
    unsafe { update_children(w) }
}
/// Synchronous original fleet `count_vehicle` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_count_vehicle(
    services: *const GroupServices,
    v: *mut (),
    delta: i32,
) {
    let w = unsafe { &*services };
    unsafe { count_vehicle(w, v, delta) }
}
/// Synchronous original fleet `count_engine` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_count_engine(
    services: *const GroupServices,
    v: *mut (),
    delta: i32,
) {
    let w = unsafe { &*services };
    unsafe { count_engine(w, v, delta) }
}
/// Synchronous original fleet `min_age` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_min_age(services: *const GroupServices, v: *mut ()) {
    let w = unsafe { &*services };
    unsafe { min_age(w, v) }
}
/// Synchronous original fleet `update_afterload` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_update_afterload(services: *const GroupServices) {
    let w = unsafe { &*services };
    unsafe { update_afterload(w) }
}
/// Synchronous original fleet `update_profits` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_update_profits(services: *const GroupServices) {
    let w = unsafe { &*services };
    unsafe { update_profits(w) }
}
/// Synchronous original fleet `update_autoreplace` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_update_autoreplace(
    services: *const GroupServices,
    company: u8,
) {
    let w = unsafe { &*services };
    unsafe { update_autoreplace(w, company) }
}
/// Synchronous original fleet `create_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_create_group(
    services: *const GroupServices,
    flags: u32,
    kind: u8,
    parent: u16,
    out: *mut u16,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { create_group(w, flags, kind, parent, out) }
}
/// Synchronous original fleet `delete_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_delete_group(
    services: *const GroupServices,
    flags: u32,
    id: u16,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { delete_group(w, flags, id) }
}
/// Synchronous original fleet `alter_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_alter_group(
    services: *const GroupServices,
    flags: u32,
    mode: u8,
    id: u16,
    parent: u16,
    text: *const u8,
    len: usize,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { alter_group(w, flags, mode, id, parent, text, len) }
}
/// Synchronous original fleet `add_vehicle_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_add_vehicle_group(
    services: *const GroupServices,
    flags: u32,
    id: u16,
    vehicle: u32,
    shared: bool,
    context: *mut (),
    vli_valid: bool,
    out: *mut u16,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { add_vehicle_group(w, flags, id, vehicle, shared, context, vli_valid, out) }
}
/// Synchronous original fleet `add_shared_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_add_shared_group(
    services: *const GroupServices,
    flags: u32,
    id: u16,
    kind: u8,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { add_shared_group(w, flags, id, kind) }
}
/// Synchronous original fleet `remove_vehicles_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_remove_vehicles_group(
    services: *const GroupServices,
    flags: u32,
    id: u16,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { remove_vehicles_group(w, flags, id) }
}
/// Synchronous original fleet `set_livery` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_set_livery(
    services: *const GroupServices,
    flags: u32,
    id: u16,
    primary: bool,
    colour: u8,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { set_livery(w, flags, id, primary, colour) }
}
/// Synchronous original fleet `flag_command` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_flag_command(
    services: *const GroupServices,
    flags: u32,
    id: u16,
    flag: u8,
    value: bool,
    recursive: bool,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { flag_command(w, flags, id, flag, value, recursive) }
}
/// `SetTrainGroupID`: affect the groupID of a train to `new_g`.
/// `v` is the first vehicle of the chain, `new_g` the index of array group.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
pub(crate) unsafe fn set_train_group_id(w: &GroupServices, v: *mut (), new_g: GroupID) {
    let id = new_g.0;
    unsafe {
        if !group(w, id).is_null() || id == DEFAULT_GROUP {
            train_group(w, v, id, true);
        }
    }
}
/// `UpdateTrainGroupID`: recalculates the groupID of a train. Should be called each time a
/// vehicle is added to/removed from the chain. This needs to be called too for 'wagon chains'
/// (in the depot, without an engine). `v` is the first vehicle of the chain.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
pub(crate) unsafe fn update_train_group_id(w: &GroupServices, v: *mut ()) {
    unsafe {
        {
            let id = if record(w, v).is_front_engine() {
                (w.vehicle_group)(v)
            } else {
                DEFAULT_GROUP
            };
            train_group(w, v, id, false);
        }
    }
}
/// Synchronous original fleet `sum_engines` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_sum_engines(
    services: *const GroupServices,
    company: u8,
    id: u16,
    engine: u16,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { sum_engines(w, company, id, engine) }
}
/// Synchronous original fleet `sum_vehicles` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_sum_vehicles(
    services: *const GroupServices,
    company: u8,
    id: u16,
    kind: u8,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { sum_vehicles(w, company, id, kind) }
}
/// Synchronous original fleet `sum_min_age` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_sum_min_age(
    services: *const GroupServices,
    company: u8,
    id: u16,
    kind: u8,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { sum_min_age(w, company, id, kind) }
}
/// Synchronous original fleet `sum_profit` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_sum_profit(
    services: *const GroupServices,
    company: u8,
    id: u16,
    kind: u8,
) -> i64 {
    let w = unsafe { &*services };
    unsafe { sum_profit(w, company, id, kind) }
}
/// Synchronous original fleet contains control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_contains_abi(
    services: *const GroupServices,
    search: u16,
    target: u16,
) -> u8 {
    let w = unsafe { &*services };
    u8::from(unsafe { contains(w, search, target) })
}
/// Synchronous original fleet replacement control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_replacement(
    services: *const GroupServices,
    head: *mut (),
    engine: u16,
    id: u16,
    old: *mut bool,
) -> u16 {
    let w = unsafe { &*services };
    unsafe { engine_replacement(w, head, engine, id, old) }
}
/// Synchronous original fleet `remove_replacement` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_remove_replacement(
    services: *const GroupServices,
    head: *mut *mut (),
    engine: u16,
    id: u16,
    flags: u32,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { remove_engine_replacement(w, head, engine, id, flags) }
}
/// Synchronous original fleet `remove_all_replacements` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_remove_all_replacements(
    services: *const GroupServices,
    head: *mut *mut (),
) {
    let w = unsafe { &*services };
    unsafe { remove_all_engine_replacement(w, head) }
}

pub(crate) mod pending;
pub(crate) mod transactions;

crate::abi::layouts! {
    fn abi_layout;
    "FleetGroupFields" ["fleet"] => GroupFields {
        owner,
        vehicle_type,
        flags,
        livery,
        parent,
        number,
    };
    "FleetStatisticsFields" ["fleet"] => StatsFields {
        profit => profit_last_year,
        profit_min_age => profit_last_year_min_age,
        vehicles => num_vehicle,
        vehicles_min_age => num_vehicle_min_age,
        defined => autoreplace_defined,
        finished => autoreplace_finished,
    };
    "OpenTTDFleetGroupServices" ["fleet"] => GroupServices {
        group,
        next_group,
        next_company,
        vehicle,
        vehicles,
        vehicle_id,
        vehicle_group,
        profit,
        old_enough,
        primary,
        countable,
        first_shared,
        next_shared,
        set_membership,
        invalidate_cache,
        viewport,
        stats,
        head,
        renew_state,
        next_renew,
        renew_id,
        buildable_type,
        can_allocate,
        allocate,
        use_number,
        release_number,
        company_livery,
        keep_length,
        delete_group,
        clear_backup,
        remove_rule,
        remove_vehicles,
        delete_child,
        add_to_group,
        list_dirty,
        list_set_dirty,
        colour_dirty,
        replace_dirty,
        replace_invalidate,
        alter_dirty,
        vehicle_dirty,
        depot_dirty,
        close_replace,
        shared,
        list_generate,
        list_size,
        list_at,
        renew_allocate,
        renew_can_allocate,
        renew_delete,
        recursion_error,
    };
}
