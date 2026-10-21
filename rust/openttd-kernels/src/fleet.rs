/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical fleet state and complete group/rule control; validation remains WIP.
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
use crate::vehicle::OpenTTDVehicle;
use std::collections::{BTreeMap, BTreeSet};
const NONE: u32 = u32::MAX;
/// `VehicleID::Invalid()` (`vehicle_type.h`), the list-selection sentinel.
pub(crate) const INVALID_VEHICLE: u32 = 0xF_FFFF;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GroupFields {
    pub owner: u8,
    pub vehicle_type: u8,
    pub flags: u8,
    pub livery: [u8; 3],
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
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Renew {
    pub from: u16,
    pub to: u16,
    pub next: *mut (),
    pub group: u16,
    pub when_old: bool,
}
/// Create the canonical group allocation; C++ starts scalar prefix lifetimes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_group_create() -> *mut () {
    Box::into_raw(Box::new(Group {
        fields: GroupFields {
            owner: 255,
            vehicle_type: 255,
            flags: 0,
            livery: [0; 3],
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
        group: 65535,
        when_old: false,
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
    pub(crate) next_vehicle: unsafe extern "C" fn(u32) -> *mut OpenTTDVehicle,
    pub(crate) vehicle: unsafe extern "C" fn(u32) -> *mut (),
    pub(crate) vehicle_id: unsafe extern "C" fn(*mut ()) -> u32,
    pub(crate) vehicle_type: unsafe extern "C" fn(*mut ()) -> u8,
    pub(crate) vehicle_owner: unsafe extern "C" fn(*mut ()) -> u8,
    pub(crate) vehicle_group: unsafe extern "C" fn(*mut ()) -> u16,
    pub(crate) vehicle_engine: unsafe extern "C" fn(*mut ()) -> u16,
    pub(crate) profit: unsafe extern "C" fn(*mut ()) -> i64,
    pub(crate) old_enough: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) primary: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) countable: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) ground: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) front: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) next_part: unsafe extern "C" fn(*mut ()) -> *mut (),
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
    pub(crate) engine_type: unsafe extern "C" fn(u16) -> u8,
    pub(crate) current_company: unsafe extern "C" fn() -> u8,
    pub(crate) buildable_type: unsafe extern "C" fn(u8) -> bool,
    pub(crate) can_allocate: unsafe extern "C" fn() -> bool,
    pub(crate) allocate: unsafe extern "C" fn(u8, u8) -> u16,
    pub(crate) use_number: unsafe extern "C" fn(u8) -> u16,
    pub(crate) release_number: unsafe extern "C" fn(u8, u16),
    pub(crate) company_livery: unsafe extern "C" fn(u8) -> *const u8,
    pub(crate) keep_length: unsafe extern "C" fn(u8) -> bool,
    pub(crate) delete_group: unsafe extern "C" fn(u16),
    pub(crate) invalid_parent: unsafe extern "C" fn(u16, u16),
    pub(crate) clear_backup: unsafe extern "C" fn(u16),
    pub(crate) remove_rule: unsafe extern "C" fn(u8, u16, u16, u32),
    pub(crate) remove_vehicles: unsafe extern "C" fn(u16, u32),
    pub(crate) delete_child: unsafe extern "C" fn(u16, u32),
    pub(crate) add_to_group: unsafe extern "C" fn(u16, u32, u32),
    pub(crate) utf8_length: unsafe extern "C" fn(*const u8, usize) -> usize,
    pub(crate) list_dirty: unsafe extern "C" fn(u8, u8),
    pub(crate) list_set_dirty: unsafe extern "C" fn(u8, u8),
    pub(crate) colour_dirty: unsafe extern "C" fn(u8, u8),
    pub(crate) replace_dirty: unsafe extern "C" fn(u8),
    pub(crate) replace_invalidate: unsafe extern "C" fn(u8),
    pub(crate) alter_dirty: unsafe extern "C" fn(u8),
    pub(crate) vehicle_dirty: unsafe extern "C" fn(u32),
    pub(crate) depot_dirty: unsafe extern "C" fn(*mut ()),
    pub(crate) close_replace: unsafe extern "C" fn(u8),
    pub(crate) screen_dirty: unsafe extern "C" fn(),
    pub(crate) list_generate: unsafe extern "C" fn(*mut ()) -> bool,
    pub(crate) list_size: unsafe extern "C" fn(*mut ()) -> usize,
    pub(crate) list_at: unsafe extern "C" fn(*mut (), usize) -> *mut (),
    pub(crate) renew_allocate: unsafe extern "C" fn() -> *mut (),
    pub(crate) renew_can_allocate: unsafe extern "C" fn() -> bool,
    pub(crate) renew_delete: unsafe extern "C" fn(*mut ()),
    pub(crate) recursion_error: u32,
}

const INVALID_GROUP: u16 = 65535;
const DEFAULT_GROUP: u16 = 65534;
pub(crate) const ALL_GROUP: u16 = 65533;
const NEW_GROUP: u16 = 65532;
const OK: u32 = u32::MAX;
const ERROR: u32 = 65535;

// Raw field access permits native aliases; every access ends before a callback.
macro_rules! field {
    ($p:expr, $t:ty, $f:ident) => {
        unsafe { std::ptr::addr_of!((*$p.cast::<$t>()).$f).read() }
    };
}
macro_rules! set {
    ($p:expr, $t:ty, $f:ident, $v:expr) => {{
        let value = $v;
        unsafe { std::ptr::addr_of_mut!((*$p.cast::<$t>()).$f).write(value) }
    }};
}
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
        search = field!(g, GroupFields, parent);
        if search == INVALID_GROUP {
            return false;
        }
        g = unsafe { group(w, search) };
    }
}
unsafe fn vehicle_stats(w: &GroupServices, v: *mut (), all: bool) -> *mut Stats {
    let owner = unsafe { (w.vehicle_owner)(v) };
    let id = if all {
        ALL_GROUP
    } else {
        unsafe { (w.vehicle_group)(v) }
    };
    let kind = unsafe { (w.vehicle_type)(v) };
    unsafe { (w.stats)(owner, id, kind).cast() }
}
unsafe fn count_vehicle(w: &GroupServices, v: *mut (), delta: i32) {
    let old = unsafe { (w.old_enough)(v) };
    let profit = unsafe { (w.profit)(v) }.saturating_mul(i64::from(delta));
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
    let engine = unsafe { (w.vehicle_engine)(v) };
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    unsafe {
        openttd_rust_fleet_engine_change(all.cast(), engine, delta);
        openttd_rust_fleet_engine_change(stats.cast(), engine, delta);
    }
}
unsafe fn add_profit(w: &GroupServices, v: *mut ()) {
    let profit = unsafe { (w.profit)(v) };
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    for p in [all, stats] {
        unsafe { (*p).fields.profit = (*p).fields.profit.saturating_add(profit) };
    }
}
unsafe fn min_age(w: &GroupServices, v: *mut ()) {
    let profit = unsafe { (w.profit)(v) };
    let all = unsafe { vehicle_stats(w, v, true) };
    let stats = unsafe { vehicle_stats(w, v, false) };
    for p in [all, stats] {
        let s = unsafe { &mut *p };
        s.fields.vehicles_min_age = s.fields.vehicles_min_age.wrapping_add(1);
        s.fields.profit_min_age = s.fields.profit_min_age.saturating_add(profit);
    }
}
unsafe fn engine_membership(w: &GroupServices, v: *mut (), old: u16, new: u16) {
    if old == new {
        return;
    }
    let owner = unsafe { (w.vehicle_owner)(v) };
    let kind = unsafe { (w.vehicle_type)(v) };
    let engine = unsafe { (w.vehicle_engine)(v) };
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
    let kind = unsafe { (w.engine_type)(engine) };
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
        if field!(g, GroupFields, owner) == company {
            let kind = field!(g, GroupFields, vehicle_type);
            let s = unsafe { (w.stats)(company, id as u16, kind) };
            unsafe { openttd_rust_fleet_stats_clear_autoreplace(s) };
        }
        from = id + 1;
    }
    let head = unsafe { (w.head)(company) };
    let mut er = unsafe { head.cast::<*mut ()>().read() };
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let engine = field!(p, Renew, from);
        let id = field!(p, Renew, group);
        let next = field!(p, Renew, next);
        let kind = unsafe { (w.engine_type)(engine) };
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
    let owner = unsafe { (w.vehicle_owner)(v) };
    let mut u = v;
    while !u.is_null() {
        let old = unsafe { (w.vehicle_group)(u) };
        if unsafe { (w.countable)(u) } {
            unsafe { engine_membership(w, u, old, new) };
        }
        unsafe {
            (w.set_membership)(u, new);
            (w.invalidate_cache)(u);
        }
        if viewport {
            unsafe { (w.viewport)(u) };
        }
        u = unsafe { (w.next_part)(u) };
    }
    unsafe {
        update_autoreplace(w, owner);
        (w.replace_dirty)(0);
    }
}
unsafe fn add_vehicle(w: &GroupServices, v: *mut (), new: u16) {
    unsafe { count_vehicle(w, v, -1) };
    match unsafe { (w.vehicle_type)(v) } {
        0 => unsafe { train_group(w, v, new, true) },
        1..=3 => {
            let old = unsafe { (w.vehicle_group)(v) };
            if unsafe { (w.countable)(v) } {
                unsafe { engine_membership(w, v, old, new) };
            }
            unsafe { (w.set_membership)(v, new) };
            let mut u = v;
            while !u.is_null() {
                unsafe {
                    (w.invalidate_cache)(u);
                    (w.viewport)(u);
                }
                u = unsafe { (w.next_part)(u) };
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
unsafe fn parent_livery(w: &GroupServices, g: *mut GroupFields) -> [u8; 3] {
    let parent = field!(g, GroupFields, parent);
    if parent == INVALID_GROUP {
        let owner = field!(g, GroupFields, owner);
        let p = unsafe { (w.company_livery)(owner) };
        unsafe { p.cast::<[u8; 3]>().read() }
    } else {
        let p = unsafe { group(w, parent) };
        field!(p, GroupFields, livery)
    }
}
unsafe fn propagate_livery(w: &GroupServices, id: u16, reset: bool) {
    let g = unsafe { group(w, id) };
    if reset {
        let mut from = 0;
        loop {
            let v = unsafe { (w.next_vehicle)(from) }.cast::<()>();
            if v.is_null() {
                break;
            }
            from = unsafe { (w.vehicle_id)(v) } + 1;
            if unsafe { (w.vehicle_group)(v) } == id
                && (!unsafe { (w.ground)(v) } || unsafe { (w.front)(v) })
            {
                let mut u = v;
                while !u.is_null() {
                    unsafe { (w.invalidate_cache)(u) };
                    u = unsafe { (w.next_part)(u) };
                }
            }
        }
    }
    let mut from = 0;
    while let Some(child) = unsafe { child_after(g, from) } {
        let cg = unsafe { group(w, child) };
        let mut livery = field!(cg, GroupFields, livery);
        let parent = field!(g, GroupFields, livery);
        if livery[0] & 1 == 0 {
            livery[1] = parent[1];
        }
        if livery[0] & 2 == 0 {
            livery[2] = parent[2];
        }
        set!(cg, GroupFields, livery, livery);
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
    let owner = unsafe { (w.current_company)() };
    if !pg.is_null()
        && (field!(pg, GroupFields, owner) != owner
            || field!(pg, GroupFields, vehicle_type) != kind)
    {
        return ERROR;
    }
    if flags & 1 != 0 {
        let id = unsafe { (w.allocate)(owner, kind) };
        let g = unsafe { group(w, id) };
        let number = unsafe { (w.use_number)(owner) };
        set!(g, GroupFields, number, number);
        let mut livery = field!(g, GroupFields, livery);
        if pg.is_null() {
            let parent = unsafe { (w.company_livery)(owner).cast::<[u8; 3]>().read() };
            livery[1] = parent[1];
            livery[2] = parent[2];
            if unsafe { (w.keep_length)(owner) } {
                set!(g, GroupFields, flags, field!(g, GroupFields, flags) | 2);
            }
        } else {
            set!(g, GroupFields, parent, parent);
            let parent = field!(pg, GroupFields, livery);
            livery[1] = parent[1];
            livery[2] = parent[2];
            set!(g, GroupFields, flags, field!(pg, GroupFields, flags));
            unsafe { openttd_rust_fleet_child_change(pg.cast(), u32::from(id), true) };
        }
        set!(g, GroupFields, livery, livery);
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
    let owner = unsafe { (w.current_company)() };
    if g.is_null() || field!(g, GroupFields, owner) != owner {
        return ERROR;
    }
    unsafe { (w.remove_vehicles)(id, flags) };
    let children = unsafe { children_snapshot(g) };
    for child in children {
        unsafe { (w.delete_child)(child, flags) };
    }
    if flags & 1 != 0 {
        unsafe { (w.clear_backup)(id) };
        let owner = field!(g, GroupFields, owner);
        if owner < 15 {
            let mut from = 0;
            loop {
                let er = unsafe { (w.next_renew)(from) };
                if er.is_null() {
                    break;
                }
                from = unsafe { (w.renew_id)(er) } + 1;
                let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
                if field!(p, Renew, group) == id {
                    let engine = field!(p, Renew, from);
                    unsafe { (w.remove_rule)(owner, engine, id, flags) };
                }
            }
            let number = field!(g, GroupFields, number);
            unsafe { (w.release_number)(owner, number) };
        }
        let parent = field!(g, GroupFields, parent);
        if parent != INVALID_GROUP {
            let pg = unsafe { group(w, parent) };
            unsafe { openttd_rust_fleet_child_change(pg.cast(), u32::from(id), false) };
        }
        let kind = field!(g, GroupFields, vehicle_type);
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
    let owner = unsafe { (w.current_company)() };
    if g.is_null() || field!(g, GroupFields, owner) != owner {
        return ERROR;
    }
    if mode == 0 {
        if len != 0 && unsafe { (w.utf8_length)(text, len) } >= 32 {
            return ERROR;
        }
        if flags & 1 != 0 {
            unsafe { openttd_rust_fleet_set_name(g.cast(), text, len) };
        }
    } else if mode == 1 {
        if field!(g, GroupFields, parent) == parent {
            return OK;
        }
        let pg = unsafe { group(w, parent) };
        if !pg.is_null() {
            if field!(pg, GroupFields, owner) != owner
                || field!(pg, GroupFields, vehicle_type) != field!(g, GroupFields, vehicle_type)
            {
                return ERROR;
            }
            if unsafe { contains(w, parent, id) } {
                return w.recursion_error;
            }
        }
        if flags & 1 != 0 {
            let old_parent = field!(g, GroupFields, parent);
            if old_parent != INVALID_GROUP {
                let old = unsafe { group(w, old_parent) };
                unsafe { openttd_rust_fleet_child_change(old.cast(), u32::from(id), false) };
            }
            set!(
                g,
                GroupFields,
                parent,
                if pg.is_null() { INVALID_GROUP } else { parent }
            );
            if !pg.is_null() {
                unsafe { openttd_rust_fleet_child_change(pg.cast(), u32::from(id), true) };
            }
            unsafe { update_autoreplace(w, owner) };
            let mut livery = field!(g, GroupFields, livery);
            if livery[0] & 3 != 3 {
                let inherited = unsafe { parent_livery(w, g) };
                if livery[0] & 1 == 0 {
                    livery[1] = inherited[1];
                }
                if livery[0] & 2 == 0 {
                    livery[2] = inherited[2];
                }
                set!(g, GroupFields, livery, livery);
                unsafe {
                    propagate_livery(w, id, true);
                    (w.screen_dirty)();
                }
            }
        }
    } else {
        return ERROR;
    }
    if flags & 1 != 0 {
        let kind = field!(g, GroupFields, vehicle_type);
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
    let bits = field!(g, GroupFields, flags);
    set!(
        g,
        GroupFields,
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
        let parent = field!(g, GroupFields, parent);
        if parent == INVALID_GROUP {
            continue;
        }
        let pg = unsafe { group(w, parent) };
        if pg.is_null()
            || field!(pg, GroupFields, owner) != field!(g, GroupFields, owner)
            || field!(pg, GroupFields, vehicle_type) != field!(g, GroupFields, vehicle_type)
        {
            unsafe { (w.invalid_parent)(id as u16, parent) };
            set!(g, GroupFields, parent, INVALID_GROUP);
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
        let owner = field!(g, GroupFields, owner);
        let kind = field!(g, GroupFields, vehicle_type);
        let p = unsafe { (w.stats)(owner, id as u16, kind) };
        unsafe { openttd_rust_fleet_stats_clear(p) };
    }
    from = 0;
    loop {
        let v = unsafe { (w.next_vehicle)(from) }.cast::<()>();
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
        let owner = field!(g, GroupFields, owner);
        let kind = field!(g, GroupFields, vehicle_type);
        let p = unsafe { (w.stats)(owner, id as u16, kind) };
        unsafe { openttd_rust_fleet_stats_clear_profits(p) };
    }
    from = 0;
    loop {
        let v = unsafe { (w.next_vehicle)(from) }.cast::<()>();
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
unsafe fn company_liveries(w: &GroupServices, company: u8) {
    let mut from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        from = id + 1;
        let g = unsafe { group(w, id as u16) };
        if field!(g, GroupFields, owner) == company
            && field!(g, GroupFields, parent) == INVALID_GROUP
        {
            let inherited = unsafe { (w.company_livery)(company).cast::<[u8; 3]>().read() };
            let mut livery = field!(g, GroupFields, livery);
            if livery[0] & 1 == 0 {
                livery[1] = inherited[1];
            }
            if livery[0] & 2 == 0 {
                livery[2] = inherited[2];
            }
            set!(g, GroupFields, livery, livery);
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
    unsafe { out.write(INVALID_GROUP) };
    let mut new = id;
    if unsafe { group(w, new) }.is_null() && new != DEFAULT_GROUP && new != NEW_GROUP {
        return ERROR;
    }
    // The original's command-local `VehicleList`: generated from the
    // identifier, or holding the single named vehicle.
    let list = if veh_id == INVALID_VEHICLE && vli_valid {
        if !unsafe { (w.list_generate)(context) } || unsafe { (w.list_size)(context) } == 0 {
            return ERROR;
        }
        VehicleList::Generated(context)
    } else {
        let v = unsafe { (w.vehicle)(veh_id) };
        if v.is_null() {
            return ERROR;
        }
        VehicleList::Single(v)
    };
    let first = unsafe { list.at(w, 0) };
    let kind = unsafe { (w.vehicle_type)(first) };
    let owner = unsafe { (w.current_company)() };
    let size = unsafe { list.size(w) };
    for i in 0..size {
        let v = unsafe { list.at(w, i) };
        if unsafe { (w.vehicle_owner)(v) } != owner || !unsafe { (w.primary)(v) } {
            return ERROR;
        }
    }
    let g = unsafe { group(w, new) };
    if !g.is_null()
        && (field!(g, GroupFields, owner) != owner || field!(g, GroupFields, vehicle_type) != kind)
    {
        return ERROR;
    }
    if new == NEW_GROUP {
        let ret = unsafe { create_group(w, flags, kind, INVALID_GROUP, &raw mut new) };
        if ret != OK {
            unsafe { out.write(new) };
            return ret;
        }
    }
    if flags & 1 != 0 {
        for i in 0..size {
            let vc = unsafe { list.at(w, i) };
            let vehicle_id = unsafe { (w.vehicle_id)(vc) };
            let v = unsafe { (w.vehicle)(vehicle_id) };
            unsafe { add_vehicle(w, v, new) };
            if shared {
                let mut v2 = unsafe { (w.first_shared)(v) };
                while !v2.is_null() {
                    if unsafe { (w.vehicle_group)(v2) } != new {
                        unsafe { add_vehicle(w, v2, new) };
                    }
                    v2 = unsafe { (w.next_shared)(v2) };
                }
            }
            unsafe { (w.depot_dirty)(v) };
        }
        unsafe {
            update_autoreplace(w, owner);
            (w.replace_dirty)(kind);
            (w.list_dirty)(owner, kind);
        }
    }
    unsafe { out.write(new) };
    OK
}
unsafe fn add_shared_group(w: &GroupServices, flags: u32, id: u16, kind: u8) -> u32 {
    if unsafe { group(w, id) }.is_null() || !unsafe { (w.buildable_type)(kind) } {
        return ERROR;
    }
    if flags & 1 != 0 {
        let mut from = 0;
        loop {
            let v = unsafe { (w.next_vehicle)(from) }.cast::<()>();
            if v.is_null() {
                break;
            }
            from = unsafe { (w.vehicle_id)(v) } + 1;
            if unsafe { (w.vehicle_type)(v) } != kind
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
        let owner = unsafe { (w.current_company)() };
        unsafe { (w.list_dirty)(owner, kind) };
    }
    OK
}
unsafe fn remove_vehicles_group(w: &GroupServices, flags: u32, id: u16) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = unsafe { (w.current_company)() };
    if g.is_null() || field!(g, GroupFields, owner) != owner {
        return ERROR;
    }
    if flags & 1 != 0 {
        let mut from = 0;
        loop {
            let v = unsafe { (w.next_vehicle)(from) }.cast::<()>();
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
        let kind = field!(g, GroupFields, vehicle_type);
        unsafe { (w.list_dirty)(owner, kind) };
    }
    OK
}
unsafe fn set_livery(w: &GroupServices, flags: u32, id: u16, primary: bool, mut colour: u8) -> u32 {
    let g = unsafe { group(w, id) };
    let owner = unsafe { (w.current_company)() };
    if g.is_null() || field!(g, GroupFields, owner) != owner || (colour >= 16 && colour != 255) {
        return ERROR;
    }
    if flags & 1 != 0 {
        let mut livery = field!(g, GroupFields, livery);
        let mask = if primary { 1 } else { 2 };
        let index = if primary { 1 } else { 2 };
        if colour == 255 {
            livery[0] &= !mask;
            colour = unsafe { parent_livery(w, g) }[index];
        } else {
            livery[0] |= mask;
        }
        livery[index] = colour;
        set!(g, GroupFields, livery, livery);
        unsafe {
            propagate_livery(w, id, true);
            (w.screen_dirty)();
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
    let owner = unsafe { (w.current_company)() };
    if g.is_null() || field!(g, GroupFields, owner) != owner || flag > 1 {
        return ERROR;
    }
    if flags & 1 != 0 {
        unsafe { set_flag(w, id, flag, value, recursive) };
        let kind = field!(g, GroupFields, vehicle_type);
        unsafe {
            (w.list_set_dirty)(owner, kind);
            (w.replace_invalidate)(kind);
        }
    }
    OK
}
unsafe fn remove_company_groups(w: &GroupServices, company: u8) {
    let mut from = 0;
    loop {
        let id = unsafe { (w.next_group)(from) };
        if id == NONE {
            break;
        }
        from = id + 1;
        let g = unsafe { group(w, id as u16) };
        if field!(g, GroupFields, owner) == company {
            unsafe { (w.delete_group)(id as u16) };
        }
    }
}
unsafe fn get_replacement(w: &GroupServices, mut er: *mut (), engine: u16, id: u16) -> *mut Renew {
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let target = field!(p, Renew, group);
        if field!(p, Renew, from) == engine && unsafe { contains(w, id, target) } {
            return p;
        }
        er = field!(p, Renew, next);
    }
    std::ptr::null_mut()
}
unsafe fn replacement(
    w: &GroupServices,
    er: *mut (),
    engine: u16,
    id: u16,
    when_old: *mut bool,
) -> u16 {
    let mut p = unsafe { get_replacement(w, er, engine, id) };
    if p.is_null() {
        let g = unsafe { group(w, id) };
        if id == DEFAULT_GROUP || (!g.is_null() && field!(g, GroupFields, flags) & 1 == 0) {
            p = unsafe { get_replacement(w, er, engine, ALL_GROUP) };
        }
    }
    if !when_old.is_null() {
        let old = !p.is_null() && (field!(p, Renew, to) == engine || field!(p, Renew, when_old));
        unsafe { when_old.write(old) };
    }
    if p.is_null() {
        INVALID_GROUP
    } else {
        field!(p, Renew, to)
    }
}
unsafe fn add_replacement(
    w: &GroupServices,
    head: *mut *mut (),
    from: u16,
    to: u16,
    id: u16,
    old: bool,
    flags: u32,
) -> u32 {
    let p = unsafe { get_replacement(w, head.read(), from, id) };
    if !p.is_null() {
        if flags & 1 != 0 {
            set!(p, Renew, to, to);
            set!(p, Renew, when_old, old);
        }
        return OK;
    }
    if !unsafe { (w.renew_can_allocate)() } {
        return ERROR;
    }
    if flags & 1 != 0 {
        let shell = unsafe { (w.renew_allocate)() };
        let p = unsafe { (w.renew_state)(shell).cast::<Renew>() };
        set!(p, Renew, from, from);
        set!(p, Renew, to, to);
        set!(p, Renew, group, id);
        set!(p, Renew, when_old, old);
        set!(p, Renew, next, unsafe { head.read() });
        unsafe { head.write(shell) };
    }
    OK
}
unsafe fn remove_replacement(
    w: &GroupServices,
    head: *mut *mut (),
    engine: u16,
    id: u16,
    flags: u32,
) -> u32 {
    let mut er = unsafe { head.read() };
    let mut prev: *mut Renew = std::ptr::null_mut();
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let next = field!(p, Renew, next);
        if field!(p, Renew, from) == engine && field!(p, Renew, group) == id {
            if flags & 1 != 0 {
                if prev.is_null() {
                    unsafe { head.write(next) };
                } else {
                    set!(prev, Renew, next, next);
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
unsafe fn remove_all_replacements(w: &GroupServices, head: *mut *mut ()) {
    let mut er = unsafe { head.read() };
    while !er.is_null() {
        let p = unsafe { (w.renew_state)(er).cast::<Renew>() };
        let next = field!(p, Renew, next);
        unsafe { (w.renew_delete)(er) };
        er = next;
    }
    unsafe { head.write(std::ptr::null_mut()) };
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
/// Synchronous original fleet `add_profit` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_add_profit(services: *const GroupServices, v: *mut ()) {
    let w = unsafe { &*services };
    unsafe { add_profit(w, v) }
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
/// Synchronous original fleet `company_liveries` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_company_liveries(
    services: *const GroupServices,
    company: u8,
) {
    let w = unsafe { &*services };
    unsafe { company_liveries(w, company) }
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
/// Synchronous original fleet `set_train_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_set_train_group(
    services: *const GroupServices,
    v: *mut (),
    id: u16,
) {
    let w = unsafe { &*services };
    unsafe {
        if !group(w, id).is_null() || id == DEFAULT_GROUP {
            train_group(w, v, id, true);
        }
    }
}
/// Synchronous original fleet `update_train_group` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_update_train_group(
    services: *const GroupServices,
    v: *mut (),
) {
    let w = unsafe { &*services };
    unsafe {
        {
            let id = if (w.front)(v) {
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
/// Synchronous original fleet `remove_company_groups` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_remove_company_groups(
    services: *const GroupServices,
    company: u8,
) {
    let w = unsafe { &*services };
    unsafe { remove_company_groups(w, company) }
}
/// Synchronous original fleet contains control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_contains(
    services: *const GroupServices,
    search: u16,
    target: u16,
) -> bool {
    let w = unsafe { &*services };
    unsafe { contains(w, search, target) }
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
    unsafe { replacement(w, head, engine, id, old) }
}
/// Synchronous original fleet `add_replacement` control.
/// # Safety
/// Static typed noexcept services; live native arguments and game-thread owners.
/// Field accesses end before callbacks. Panics or escaping exceptions abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_add_replacement(
    services: *const GroupServices,
    head: *mut *mut (),
    from: u16,
    to: u16,
    id: u16,
    old: bool,
    flags: u32,
) -> u32 {
    let w = unsafe { &*services };
    unsafe { add_replacement(w, head, from, to, id, old, flags) }
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
    unsafe { remove_replacement(w, head, engine, id, flags) }
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
    unsafe { remove_all_replacements(w, head) }
}

pub(crate) mod pending;
pub(crate) mod transactions;
