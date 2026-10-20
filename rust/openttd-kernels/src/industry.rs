/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical industry cargo/history and production/builder control.
//!
//! The C++ shell owns one Box handle. Its typed views alias only field-sized raw
//! accesses here. No reference into an owner survives a world service: construction,
//! destruction, cargo distribution and `NewGRF` resolution may revisit live state.
//! Slot growth invalidates views exactly as the original vector; optional history
//! allocations remain stable until slot removal. Panics abort; no unwind crosses FFI.
// Original fixed-width conversion and variable names are retained for audit.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::if_not_else
)]
use std::ffi::c_void;
use std::ptr;
mod transaction;
pub use transaction::*;
mod commands;
pub use commands::*;
mod tile;
pub use tile::*;
mod construction;
pub use construction::*;
mod generation;
pub use generation::*;
mod lifecycle;
pub use lifecycle::*;
const RECORDS: usize = 61;
const TYPES: usize = 240;
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Pair {
    pub first: u16,
    pub second: u16,
}
#[repr(C)]
#[derive(Clone)]
pub struct Produced {
    pub cargo: u8,
    pub waiting: u16,
    pub rate: u8,
    pub history: [Pair; RECORDS],
}
impl Default for Produced {
    fn default() -> Self {
        Self {
            cargo: 0,
            waiting: 0,
            rate: 0,
            history: [Pair::default(); RECORDS],
        }
    }
}
#[repr(C)]
pub struct Accepted {
    pub cargo: u8,
    pub waiting: u16,
    pub accumulated_waiting: u32,
    pub last_accepted: i32,
    pub history: *mut [Pair; RECORDS],
}
impl Default for Accepted {
    fn default() -> Self {
        Self {
            cargo: 0,
            waiting: 0,
            accumulated_waiting: 0,
            last_accepted: 0,
            history: ptr::null_mut(),
        }
    }
}
impl Drop for Accepted {
    fn drop(&mut self) {
        if !self.history.is_null() {
            unsafe {
                drop(Box::from_raw(self.history));
            }
        }
    }
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Fields {
    pub valid_history: u64,
    pub last_prod_year: i32,
    pub counter: u16,
    pub prod_level: u8,
    pub was_cargo_delivered: u8,
    pub ctlflags: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Metadata {
    pub tile: u32,
    pub width: u16,
    pub height: u16,
    pub construction_date: i32,
    pub random: u16,
    pub kind: u8,
    pub owner: u8,
    pub random_colour: u8,
    pub founder: u8,
    pub construction_type: u8,
    pub selected_layout: u8,
    pub exclusive_supplier: u8,
    pub exclusive_consumer: u8,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            tile: u32::MAX,
            width: 0,
            height: 0,
            construction_date: 0,
            random: 0,
            kind: 0,
            owner: 255,
            random_colour: 0,
            founder: 255,
            construction_type: 0,
            selected_layout: 0,
            exclusive_supplier: 255,
            exclusive_consumer: 255,
        }
    }
}
#[derive(Default)]
pub struct Industry {
    fields: Fields,
    metadata: Metadata,
    text: Vec<u8>,
    produced: Vec<Produced>,
    accepted: Vec<Accepted>,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct BuildFields {
    pub probability: u32,
    pub min_number: u8,
    pub target_count: u16,
    pub max_wait: u16,
    pub wait_count: u16,
}
#[repr(C)]
pub struct BuilderFields {
    pub builddata: [BuildFields; TYPES],
    pub wanted_inds: u32,
    pub daily_counter: u32,
    pub daily_increment: u32,
    pub sound_tile: u32,
    pub sound_ctr: u8,
}
pub struct Builder {
    fields: BuilderFields,
    registry: [Vec<u16>; TYPES],
}
#[repr(C)]
pub struct Slots {
    pub data: *mut c_void,
    pub size: usize,
}
/// Allocate a canonical owner. Allocation failure aborts.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_new() -> *mut Industry {
    Box::into_raw(Box::default())
}
/// # Safety
/// Handle is live and uniquely owned; all C++ slot/history/scalar views have ended.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_destroy(owner: *mut Industry) {
    unsafe {
        drop(Box::from_raw(owner));
    }
}
/// # Safety
/// Live shell handle. Returned fields are nonowning; no concurrent accesses.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_fields(owner: *mut Industry) -> *mut Fields {
    unsafe { ptr::addr_of_mut!((*owner).fields) }
}
fn view<T>(v: &mut Vec<T>) -> Slots {
    Slots {
        data: v.as_mut_ptr().cast(),
        size: v.len(),
    }
}
/// `std::vector::reserve`: grow capacity to at least `count`; never shrinks.
fn reserve<T>(v: &mut Vec<T>, count: usize) {
    if count > v.len() {
        v.reserve(count - v.len());
    }
}
fn emplace_back<T: Default>(v: &mut Vec<T>) -> Slots {
    v.push(T::default());
    view(v)
}
// One typed entry per vector operation. Each borrows only the addressed vector
// for its own duration; no callback runs. Mutations invalidate outstanding views.
/// # Safety
/// Live handle; the returned view is valid until the next mutation of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_produced_view(owner: *mut Industry) -> Slots {
    unsafe { view(&mut (*owner).produced) }
}
/// # Safety
/// Live handle; the returned view is valid until the next mutation of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_accepted_view(owner: *mut Industry) -> Slots {
    unsafe { view(&mut (*owner).accepted) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_produced_reserve(
    owner: *mut Industry,
    count: usize,
) {
    unsafe { reserve(&mut (*owner).produced, count) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_accepted_reserve(
    owner: *mut Industry,
    count: usize,
) {
    unsafe { reserve(&mut (*owner).accepted, count) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_produced_resize(owner: *mut Industry, count: usize) {
    unsafe { (*owner).produced.resize_with(count, Produced::default) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector; removed slots free history.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_accepted_resize(owner: *mut Industry, count: usize) {
    unsafe { (*owner).accepted.resize_with(count, Accepted::default) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector. Returns the new view.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_produced_emplace_back(
    owner: *mut Industry,
) -> Slots {
    unsafe { emplace_back(&mut (*owner).produced) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector. Returns the new view.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_accepted_emplace_back(
    owner: *mut Industry,
) -> Slots {
    unsafe { emplace_back(&mut (*owner).accepted) }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_produced_shrink_to_fit(owner: *mut Industry) {
    unsafe { (*owner).produced.shrink_to_fit() }
}
/// # Safety
/// Live handle with no outstanding C++ view of this vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_accepted_shrink_to_fit(owner: *mut Industry) {
    unsafe { (*owner).accepted.shrink_to_fit() }
}
/// # Safety
/// Live accepted slot with canonical layout. Existing history remains stable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_history(accepted: *mut Accepted) -> *mut c_void {
    unsafe {
        if (*accepted).history.is_null() {
            (*accepted).history = Box::into_raw(Box::new([Pair::default(); RECORDS]));
        }
        (*accepted).history.cast()
    }
}
/// Allocate the single builder owner for its C++ shell.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_builder_new() -> *mut Builder {
    Box::into_raw(Box::new(Builder {
        registry: std::array::from_fn(|_| Vec::new()),
        fields: BuilderFields {
            builddata: [BuildFields::default(); TYPES],
            wanted_inds: 0,
            daily_counter: 0,
            daily_increment: 0,
            sound_tile: 0,
            sound_ctr: 0,
        },
    }))
}
/// # Safety
/// Unique live builder handle; all nonowning views have ended.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_builder_destroy(owner: *mut Builder) {
    unsafe {
        drop(Box::from_raw(owner));
    }
}
/// # Safety
/// Live builder shell, serial access only. No allocation or callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_builder_fields(
    owner: *mut Builder,
) -> *mut BuilderFields {
    unsafe { ptr::addr_of_mut!((*owner).fields) }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Observation {
    pub owner: *mut Industry,
    pub behaviour: u32,
    pub callbacks: u16,
    pub life: u8,
    pub original: u8,
    pub kind: u8,
    pub up_text: u32,
    pub down_text: u32,
    pub closure_text: u32,
}
/// Map facts constant for one entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Map {
    pub size_x: u32,
    pub size_y: u32,
    pub landscape: u8,
}
/// Values the original tick reads inline; constant for the whole tick.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TickRecord {
    pub counter: u64,
    pub map: Map,
    pub interval: u32,
    pub ambient: u8,
    pub editor: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entry {
    pub industry: *mut c_void,
    pub owner: *mut Industry,
    pub id: u16,
    pub callbacks: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Location {
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FarmTile {
    pub kind: u8,
    pub snow: u8,
    pub ground: u8,
    pub grown: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Change {
    pub result: u16,
    pub reg: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TypeInfo {
    pub behaviour: u32,
    pub enabled: bool,
    pub layouts: bool,
    pub appear: u8,
}
/// Typed `noexcept` C++ services in `industry_ffi.h` order. Each is a world leaf;
/// construction and commands return values at the original call site.
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub next_tick: unsafe extern "C" fn(u32) -> Entry,
    pub sound_count: unsafe extern "C" fn(*mut c_void) -> u16,
    pub behaviour: unsafe extern "C" fn(*mut c_void) -> u32,
    pub location: unsafe extern "C" fn(*mut c_void) -> Location,
    pub industry_sound: unsafe extern "C" fn(*mut c_void, u32),
    pub play_sound: unsafe extern "C" fn(u16, u32),
    pub special_effect: unsafe extern "C" fn(*mut c_void, u32, u32) -> u16,
    pub tick_trigger: unsafe extern "C" fn(*mut c_void),
    pub production_callback: unsafe extern "C" fn(*mut c_void, u8),
    pub scale_cargo: unsafe extern "C" fn(u32) -> u32,
    pub move_goods: unsafe extern "C" fn(*mut c_void, u32, u32) -> u32,
    pub tile_add_wrap: unsafe extern "C" fn(u32, i32, i32) -> u32,
    pub farm_tile: unsafe extern "C" fn(u32) -> FarmTile,
    pub snow_line: unsafe extern "C" fn() -> u8,
    pub make_field: unsafe extern "C" fn(u32, u8, u8, u16),
    pub fence_wanted: unsafe extern "C" fn(u32, u8) -> bool,
    pub tile_completed: unsafe extern "C" fn(*mut c_void, u32) -> bool,
    pub harvest: unsafe extern "C" fn(u32),
    pub observe: unsafe extern "C" fn(*mut c_void, *mut Observation),
    pub production_rate: unsafe extern "C" fn(*mut c_void, u32) -> u8,
    pub change_callback: unsafe extern "C" fn(*mut c_void, bool, u32) -> Change,
    pub custom_text: unsafe extern "C" fn(*mut c_void, u16) -> u32,
    pub news: unsafe extern "C" fn(*mut c_void, u32, bool),
    pub rate_news: unsafe extern "C" fn(*mut c_void, u8, i32),
    pub callback_error: unsafe extern "C" fn(*mut c_void, bool),
    pub set_dirty: unsafe extern "C" fn(*mut c_void),
    pub destroy: unsafe extern "C" fn(*mut c_void),
    pub advertise: unsafe extern "C" fn(*mut c_void),
    pub create: unsafe extern "C" fn(u8, u8) -> *mut c_void,
    pub random_industry: unsafe extern "C" fn() -> *mut c_void,
    pub get: unsafe extern "C" fn(u16) -> *mut c_void,
    pub type_count: unsafe extern "C" fn(u8) -> u16,
    pub total: unsafe extern "C" fn() -> u32,
    pub type_info: unsafe extern "C" fn(u8) -> TypeInfo,
    pub probability_callback: unsafe extern "C" fn(u8, u8, u32) -> u32,
    pub scale_by_map_size: unsafe extern "C" fn(u32) -> u32,
    pub company_none: unsafe extern "C" fn() -> u8,
    pub restore_company: unsafe extern "C" fn(u8),
    pub directory_dirty: unsafe extern "C" fn(u8),
    pub recession: unsafe extern "C" fn() -> bool,
    pub economy_month: unsafe extern "C" fn() -> u8,
    pub economy_year: unsafe extern "C" fn() -> i32,
    pub days_since_last_month: unsafe extern "C" fn() -> u32,
    pub landscape: unsafe extern "C" fn() -> u8,
    pub economy_type: unsafe extern "C" fn() -> u8,
    pub passengers: unsafe extern "C" fn() -> u8,
    pub fund_only: unsafe extern "C" fn() -> bool,
    pub calendar_year: unsafe extern "C" fn() -> i32,
    pub deity: unsafe extern "C" fn() -> bool,
}
impl Services {
    /// The immutable shared table outlives every synchronous industry entry.
    fn shared(&self) -> &crate::services::Services {
        unsafe { &*self.shared }
    }
}
impl Map {
    fn coordinates(self) -> crate::services::Map {
        crate::services::Map {
            log_x: self.size_x.trailing_zeros() as u8,
        }
    }
}
const SND_36_LUMBER_MILL_3: u16 = 0x36;
const SND_37_LUMBER_MILL_2: u16 = 0x37;
const LANDSCAPE_ARCTIC: u8 = 1;
const LANDSCAPE_TROPIC: u8 = 2;
const MP_CLEAR: u8 = 0;
const MP_TREES: u8 = 4;
const INVALID_TILE: u32 = u32::MAX;
fn observe(s: &Services, i: *mut c_void) -> Observation {
    let mut o = Observation::default();
    unsafe {
        (s.observe)(i, &raw mut o);
    }
    o
}
fn rand(s: &Services) -> u32 {
    s.shared().random()
}
fn range(s: &Services, n: u32) -> u32 {
    s.shared().random_range(n)
}
fn chance_value(a: u32, b: u32, value: u32) -> bool {
    (((value & 0xffff).wrapping_mul(b).wrapping_add(b / 2)) >> 16) < a
}
fn chance(s: &Services, a: u32, b: u32) -> bool {
    chance_value(a, b, rand(s))
}
fn fields(owner: *mut Industry) -> *mut Fields {
    unsafe { ptr::addr_of_mut!((*owner).fields) }
}
fn produced(owner: *mut Industry, slot: usize) -> *mut Produced {
    unsafe { (*owner).produced.as_mut_ptr().add(slot) }
}
fn accepted(owner: *mut Industry, slot: usize) -> *mut Accepted {
    unsafe { (*owner).accepted.as_mut_ptr().add(slot) }
}
fn produced_count(owner: *mut Industry) -> usize {
    unsafe { (*owner).produced.len() }
}
fn accepted_count(owner: *mut Industry) -> usize {
    unsafe { (*owner).accepted.len() }
}
fn valid(cargo: u8) -> bool {
    cargo != 255
}
fn transported(p: Pair) -> u32 {
    if p.first == 0 {
        0
    } else {
        (u32::from(p.second) * 256 / u32::from(p.first)).min(255)
    }
}
fn helper(owner: *mut Industry, s: &Services, scale: bool) {
    for n in 0..produced_count(owner) {
        let p = produced(owner, n);
        unsafe {
            if !valid((*p).cargo) {
                continue;
            }
        }
        let mut amount = unsafe { u32::from((*p).rate) };
        if scale {
            amount = u32::from(unsafe { (s.scale_cargo)(amount) } as u16);
        }
        unsafe {
            (*p).waiting = (u32::from((*p).waiting) + amount).min(65535) as u16;
        }
    }
}
fn produce(e: Entry, t: &TickRecord, s: &Services, builder: *mut Builder) {
    let (i, owner) = (e.industry, e.owner);
    let f = fields(owner);
    if unsafe { (*f).counter.is_multiple_of(64) } {
        let r = rand(s);
        if chance_value(1, 14, r) {
            let sounds = unsafe { (s.sound_count)(i) };
            if sounds != 0
                && t.ambient != 0
                && (0..produced_count(owner))
                    .any(|n| unsafe { (*produced(owner, n)).history[1].first > 0 })
            {
                unsafe { (s.industry_sound)(i, ((r >> 16) * u32::from(sounds)) >> 16) };
            }
        }
    }
    unsafe {
        (*f).counter = (*f).counter.wrapping_sub(1);
    }
    let callback = e.callbacks & 4 != 0;
    if callback && unsafe { u32::from((*f).counter) % t.interval == 0 } {
        unsafe { (s.production_callback)(i, 1) };
        helper(owner, s, false);
    }
    if unsafe { !(*f).counter.is_multiple_of(256) } {
        return;
    }
    if !callback {
        helper(owner, s, true);
    }
    let behaviour = unsafe { (s.behaviour)(i) };
    for (flag, parameter) in [(1, 0), (2, 1)] {
        if behaviour & flag == 0 {
            continue;
        }
        let cb = if e.callbacks & (1 << 9) != 0 {
            let random = rand(s);
            unsafe { (s.special_effect)(i, random, parameter) }
        } else {
            0xffff
        };
        let act = if cb != 0xffff {
            cb != 0
        } else if parameter == 0 {
            chance(s, 1, 8)
        } else {
            unsafe { (*f).counter.is_multiple_of(512) }
        };
        if act {
            if parameter == 0 {
                farm(i, e.id, t.map, s);
            } else {
                chop(i, owner, t.map, s, builder);
            }
        }
    }
    unsafe { (s.tick_trigger)(i) };
}
/// # Safety
/// Live builder, tick record and synchronous services; C++ supplies serial world
/// access and terminating noexcept environmental failures. No borrowed owner
/// reference survives a service, including recursive production on construction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tick(
    builder: *mut Builder,
    record: *const TickRecord,
    services: *const Services,
) {
    let s = unsafe { &*services };
    let t = unsafe { *record };
    let b = build_fields(builder);
    unsafe {
        if (*b).sound_ctr != 0 {
            (*b).sound_ctr = (*b).sound_ctr.wrapping_add(1);
            if (*b).sound_ctr == 75 {
                if t.ambient != 0 {
                    (s.play_sound)(SND_37_LUMBER_MILL_2, (*b).sound_tile);
                }
            } else if (*b).sound_ctr == 160 {
                (*b).sound_ctr = 0;
                if t.ambient != 0 {
                    (s.play_sound)(SND_36_LUMBER_MILL_3, (*b).sound_tile);
                }
            }
        }
    }
    if t.editor != 0 {
        return;
    }
    let mut from = 0;
    loop {
        let e = unsafe { (s.next_tick)(from) };
        if e.industry.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        produce(e, &t, s, builder);
        if t.counter.wrapping_add(u64::from(e.id)) % 74 == 0 {
            for n in 0..accepted_count(e.owner) {
                let a = accepted(e.owner, n);
                unsafe {
                    (*a).accumulated_waiting = (*a)
                        .accumulated_waiting
                        .wrapping_add(u32::from((*a).waiting));
                }
            }
        }
    }
}
/// # Safety
/// Industry, owner and services follow the tick contract; distribution can reenter
/// industry access, so history/waiting writes occur before/after with raw fields.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_transport(
    i: *mut c_void,
    owner: *mut Industry,
    minimal_cargo: u8,
    recession: bool,
    services: *const Services,
) -> bool {
    let s = unsafe { &*services };
    let mut moved = false;
    for n in 0..produced_count(owner) {
        let p = produced(owner, n);
        let mut cw = unsafe { u32::from((*p).waiting).min(255) };
        if cw <= u32::from(minimal_cargo) || unsafe { !valid((*p).cargo) } {
            continue;
        }
        unsafe {
            (*p).waiting = (*p).waiting.wrapping_sub(cw as u16);
        }
        if recession {
            cw = cw.div_ceil(2);
        }
        unsafe {
            (*p).history[0].first = (*p).history[0].first.wrapping_add(cw as u16);
        }
        let am = unsafe { (s.move_goods)(i, n as u32, cw) };
        unsafe {
            (*p).history[0].second = (*p).history[0].second.wrapping_add(am as u16);
        }
        moved |= am != 0;
    }
    moved
}
fn recompute(i: *mut c_void, owner: *mut Industry, s: &Services) {
    let f = fields(owner);
    for n in 0..produced_count(owner) {
        let rate = u32::from(unsafe { (s.production_rate)(i, n as u32) });
        let value = rate * unsafe { u32::from((*f).prod_level) };
        unsafe {
            (*produced(owner, n)).rate = value.div_ceil(16).min(255) as u8;
        }
    }
}
/// # Safety
/// Live industry and its owner; the original `UsesOriginalEconomy` precondition applies.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_recompute(
    i: *mut c_void,
    owner: *mut Industry,
    s: *const Services,
) {
    recompute(i, owner, unsafe { &*s });
}
fn update_history(mask: &mut u64, month: u32) {
    for (first, last, division, total) in [(1, 25, 1, 1), (25, 42, 3, 3), (42, 61, 4, 12)] {
        if *mask & (1 << (last - 1)) != 0
            || !month.is_multiple_of(total)
            || (division != 1 && *mask & (1 << (first - division)) == 0)
        {
            continue;
        }
        let bits = ((1u64 << (last - first)) - 1) << first;
        *mask = (*mask & !bits) | (((*mask & bits) << 1 | (1 << first)) & bits);
    }
}
fn rotate(history: &mut [Pair; RECORDS], mask: u64, month: u32) {
    for (first, last, division, total) in [(1, 25, 1, 1), (25, 42, 3, 3), (42, 61, 4, 12)] {
        if !month.is_multiple_of(total) {
            continue;
        }
        history.copy_within(first..last - 1, first + 1);
        if total == 1 {
            history[first] = history[0];
            history[0] = Pair::default();
        } else if mask & (1 << (first - division)) != 0 {
            let mut a = 0u32;
            let mut b = 0u32;
            for pair in &history[first - division..first] {
                a += u32::from(pair.first);
                b += u32::from(pair.second);
            }
            history[first] = Pair {
                first: (a / division as u32).min(65535) as u16,
                second: (b / division as u32).min(65535) as u16,
            };
        }
    }
}
fn statistics(owner: *mut Industry, s: &Services) {
    let f = fields(owner);
    let month = u32::from(unsafe { (s.economy_month)() });
    unsafe {
        update_history(&mut (*f).valid_history, month);
    }
    for n in 0..produced_count(owner) {
        let p = produced(owner, n);
        unsafe {
            if !valid((*p).cargo) {
                continue;
            }
            if (*p).history[0].first != 0 {
                (*f).last_prod_year = (s.economy_year)();
            }
            rotate(&mut (*p).history, (*f).valid_history, month);
        }
    }
    for n in 0..accepted_count(owner) {
        let a = accepted(owner, n);
        unsafe {
            if !valid((*a).cargo) || (*a).history.is_null() {
                continue;
            }
            let history = (*a).history;
            (*history)[0].second =
                ((*a).accumulated_waiting / (s.days_since_last_month)().max(1)).min(65535) as u16;
            (*a).accumulated_waiting = 0;
            rotate(&mut *history, (*f).valid_history, month);
        }
    }
}
fn protected(o: Observation, s: &Services) -> bool {
    if o.behaviour & (1 << 7) != 0 && unsafe { (s.landscape)() } == 0 {
        return false;
    }
    o.behaviour & (1 << 17) == 0 && unsafe { (s.type_count)(o.kind) } <= 1
}
fn change(i: *mut c_void, s: &Services, monthly: bool) {
    let o = observe(s, i);
    let f = fields(o.owner);
    let mut str = 0;
    let mut close = false;
    let mut standard = false;
    let mut suppress = false;
    let mut recalc = false;
    let original = o.original != 0;
    let mut div = 0u8;
    let mut mul = 0u8;
    let mut increment = 0i8;
    let callback = o.callbacks & (1 << if monthly { 5 } else { 4 }) != 0;
    if callback {
        let random = rand(s);
        let result = unsafe { (s.change_callback)(i, monthly, random) };
        let mut res = result.result;
        if res != 0xffff {
            suppress = res & 128 != 0;
            if res & 256 != 0 {
                str = unsafe { (s.custom_text)(i, result.reg as u16) };
            }
            res &= 15;
            match res {
                0 => {}
                1 => div = 1,
                2 => mul = 1,
                3 => close = true,
                4 => standard = true,
                5..=8 => div = (res - 3) as u8,
                9..=12 => mul = (res - 7) as u8,
                13 => increment = -1,
                14 => increment = 1,
                15 => {
                    unsafe {
                        (*f).prod_level = ((result.reg as u32 >> 16) & 255).clamp(4, 128) as u8;
                    }
                    recalc = true;
                }
                _ => unreachable!(),
            }
        }
    } else if monthly == original
        || (!original && unsafe { (s.economy_type)() } == 2)
        || o.life == 0
    {
        return;
    }
    if standard || (!callback && o.life & 3 != 0) {
        let only_decrease = o.behaviour & (1 << 7) != 0 && unsafe { (s.landscape)() } == 0;
        if original {
            if only_decrease || chance(s, 1, 3) {
                let pct = if produced_count(o.owner) > 0 {
                    unsafe { transported((*produced(o.owner, 0)).history[1]) }
                } else {
                    0
                };
                if !only_decrease && (pct > 153) != chance(s, 1, 3) {
                    mul = 1;
                } else {
                    div = 1;
                }
            }
        } else if unsafe { (s.economy_type)() } == 1 {
            close = unsafe { (*f).ctlflags & 5 == 0 };
            for n in 0..produced_count(o.owner) {
                let p = produced(o.owner, n);
                if unsafe { !valid((*p).cargo) } {
                    continue;
                }
                let random = rand(s);
                let old = unsafe { i32::from((*p).rate) };
                let pct = unsafe { transported((*p).history[1]) };
                let mut mult = if pct > 153 { 1 } else { -1 };
                if only_decrease {
                    mult = -1;
                } else if chance_value(1, if pct > 204 { 6 } else { 3 }, random) {
                    mult *= -1;
                }
                let mut new = old;
                if chance_value(1, 22, random >> 16) {
                    new = new.wrapping_add(
                        mult * (((range(s, 50) + 10) * old as u32) >> 8).max(1) as i32,
                    );
                }
                new = new.clamp(1, 255);
                if unsafe { (*p).cargo == (s.passengers)() } && o.behaviour & (1 << 19) == 0 {
                    new = new.clamp(0, 16);
                }
                if unsafe {
                    ((*f).ctlflags & 1 != 0 && new < old) || ((*f).ctlflags & 2 != 0 && new > old)
                } {
                    continue;
                }
                if new == old && old > 1 {
                    close = false;
                    continue;
                }
                let percent = if old == 0 { 100 } else { new * 100 / old - 100 };
                unsafe {
                    (*p).rate = new as u8;
                }
                if new > 1 {
                    close = false;
                }
                if percent.abs() >= 10 {
                    unsafe { (s.rate_news)(i, (*p).cargo, percent) };
                }
            }
        }
    }
    if unsafe {
        ((*f).ctlflags & 1 != 0 && (div > 0 || increment < 0))
            || ((*f).ctlflags & 2 != 0 && (mul > 0 || increment > 0))
    } {
        return;
    }
    if unsafe { (*f).ctlflags & 8 != 0 } {
        div = 0;
        mul = 0;
        increment = 0;
    }
    if !callback
        && o.life & 4 != 0
        && unsafe { (s.economy_year)().wrapping_sub((*f).last_prod_year) >= 5 }
        && chance(s, 1, if original { 2 } else { 180 })
    {
        close = true;
    }
    while mul != 0 && unsafe { (*f).prod_level < 128 } {
        mul = mul.wrapping_sub(1);
        unsafe {
            (*f).prod_level = (u16::from((*f).prod_level) * 2).min(128) as u8;
        }
        recalc = true;
        if str == 0 {
            str = o.up_text;
        }
    }
    while div != 0 && !close {
        div = div.wrapping_sub(1);
        if unsafe { (*f).prod_level == 4 } {
            close = true;
            break;
        }
        unsafe {
            (*f).prod_level = ((*f).prod_level / 2).max(4);
        }
        recalc = true;
        if str == 0 {
            str = o.down_text;
        }
    }
    if increment != 0 {
        if increment < 0 && unsafe { (*f).prod_level == 4 } {
            close = true;
        } else {
            unsafe {
                (*f).prod_level = ((i32::from((*f).prod_level) + i32::from(increment)) as u32)
                    .clamp(4, 128) as u8;
            }
            recalc = true;
        }
    }
    if recalc {
        recompute(i, o.owner, s);
    }
    if close && !protected(o, s) && unsafe { (*f).ctlflags & 4 == 0 } {
        unsafe {
            (*f).prod_level = 0;
            (s.set_dirty)(i);
        }
        str = o.closure_text;
    }
    if !suppress && str != 0 {
        unsafe { (s.news)(i, str, close) };
    }
}
fn type_data(f: *mut BuildFields, kind: u8, s: &Services) -> bool {
    let probability = game_probability(kind, s);
    let p = probability as u32;
    let min = (probability >> 32) as u8;
    unsafe {
        let changed = (*f).probability != p || (*f).min_number != min;
        (*f).probability = p;
        (*f).min_number = min;
        changed
    }
}
fn game_probability(kind: u8, s: &Services) -> u64 {
    if unsafe { (s.fund_only)() } {
        return 0;
    }
    let info = unsafe { (s.type_info)(kind) };
    let behavior = info.behaviour;
    let year = unsafe { (s.calendar_year)() };
    if !info.enabled
        || !info.layouts
        || (behavior & (1 << 8) != 0 && year > 1950)
        || (behavior & (1 << 9) != 0 && year < 1960)
    {
        return 0;
    }
    let probability = unsafe { (s.probability_callback)(kind, 1, u32::from(info.appear)) } as u8;
    if probability == 0 {
        return 0;
    }
    u64::from(probability) | ((u64::from(behavior & (1 << 17) != 0)) << 32)
}
/// # Safety
/// Live build record; no reference is retained across the resolver service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_type(
    f: *mut BuildFields,
    kind: u8,
    s: *const Services,
) -> u8 {
    u8::from(type_data(f, kind, unsafe { &*s }))
}
fn build_fields(builder: *mut Builder) -> *mut BuilderFields {
    unsafe { ptr::addr_of_mut!((*builder).fields) }
}
fn build_type(b: *mut BuilderFields, n: usize) -> *mut BuildFields {
    unsafe { ptr::addr_of_mut!((*b).builddata[n]) }
}
fn build_reset(builder: *mut Builder, s: &Services) {
    let b = build_fields(builder);
    unsafe {
        (*b).wanted_inds = (s.total)().wrapping_shl(16);
        for n in 0..TYPES {
            *build_type(b, n) = BuildFields {
                max_wait: 1,
                ..BuildFields::default()
            };
        }
    }
}
/// # Safety
/// Live exclusive builder operation, with source total-count service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_reset(b: *mut Builder, s: *const Services) {
    build_reset(b, unsafe { &*s });
}
fn build_monthly(builder: *mut Builder, s: &Services) {
    if unsafe { (s.fund_only)() } {
        return;
    }
    let b = build_fields(builder);
    let max = 1 + unsafe { (s.scale_by_map_size)(3) }.min(99);
    if unsafe { (s.total)() }.wrapping_add(max) >= unsafe { (*b).wanted_inds >> 16 } {
        let increment = unsafe { (s.scale_by_map_size)(0x38000 / (10 * 12)) };
        unsafe {
            (*b).wanted_inds = (*b).wanted_inds.wrapping_add(increment);
        }
    }
}
/// # Safety
/// Live builder; direct total/map observations preserve the original order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_monthly(b: *mut Builder, s: *const Services) {
    build_monthly(b, unsafe { &*s });
}
fn targets(builder: *mut Builder, s: &Services) {
    let b = build_fields(builder);
    let mut changed = false;
    let mut planned = 0u32;
    for n in 0..TYPES {
        let f = build_type(b, n);
        changed |= type_data(f, n as u8, s);
        planned += unsafe { u32::from((*f).target_count) };
    }
    let mut amount = unsafe { (*b).wanted_inds >> 16 };
    changed |= planned != amount;
    if !changed {
        return;
    }
    let mut forced = 0;
    let mut probability = 0u32;
    for n in 0..TYPES {
        let f = build_type(b, n);
        unsafe {
            forced += u32::from((*f).min_number);
            (*f).target_count = u16::from((*f).min_number);
            probability = probability.wrapping_add((*f).probability);
        }
    }
    if probability == 0 {
        return;
    }
    amount = amount.saturating_sub(forced);
    while amount > 0 {
        let mut random = range(s, probability);
        let mut n = 0;
        while random >= unsafe { (*build_type(b, n)).probability } {
            random -= unsafe { (*build_type(b, n)).probability };
            n += 1;
        }
        unsafe {
            (*build_type(b, n)).target_count = (*build_type(b, n)).target_count.wrapping_add(1);
        }
        amount -= 1;
    }
}
/// # Safety
/// Live builder; slot fields are read/written only between source services.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_targets(b: *mut Builder, s: *const Services) {
    targets(b, unsafe { &*s });
}
fn count(s: &Services, n: usize) -> i32 {
    i32::from(unsafe { (s.type_count)(n as u8) })
}
fn place(kind: u8, creation: u8, hard: bool, s: &Services) -> *mut c_void {
    for _ in 0..if hard { 10000 } else { 2000 } {
        let i = unsafe { (s.create)(kind, creation) };
        if !i.is_null() {
            return i;
        }
    }
    ptr::null_mut()
}
fn build_try(builder: *mut Builder, s: &Services) {
    targets(builder, s);
    let b = build_fields(builder);
    let mut missing = 0i32;
    let mut eligible = 0;
    let mut probability = 0u32;
    let mut forced = TYPES;
    for n in 0..TYPES {
        let f = build_type(b, n);
        let difference = unsafe { i32::from((*f).target_count) } - count(s, n);
        missing = missing.wrapping_add(difference);
        if unsafe { (*f).wait_count > 0 } {
            continue;
        }
        if difference > 0 {
            if count(s, n) == 0
                && unsafe { (*f).min_number > 0 }
                && (forced == TYPES
                    || difference
                        > unsafe { i32::from((*build_type(b, forced)).target_count) }
                            - count(s, forced))
            {
                forced = n;
            }
            probability = probability.wrapping_add(difference as u32);
            eligible += 1;
        }
    }
    if unsafe { (s.recession)() } || (forced == TYPES && (missing <= 0 || probability == 0)) {
        eligible = 0;
    }
    if eligible >= 1 {
        let n = if forced != TYPES {
            forced
        } else {
            let mut random = if eligible > 1 {
                range(s, probability)
            } else {
                0
            };
            let mut n = 0;
            loop {
                let f = build_type(b, n);
                if unsafe { (*f).wait_count == 0 } {
                    let difference = unsafe { i32::from((*f).target_count) } - count(s, n);
                    if difference > 0 {
                        if eligible == 1 || random < difference as u32 {
                            break n;
                        }
                        random -= difference as u32;
                    }
                }
                n += 1;
            }
        };
        let i = place(n as u8, 1, false, s);
        let f = build_type(b, n);
        if i.is_null() {
            unsafe {
                (*f).wait_count = (*f).max_wait.wrapping_add(1);
                (*f).max_wait = (u32::from((*f).max_wait) + 2).min(1000) as u16;
            }
        } else {
            unsafe { (s.advertise)(i) };
            unsafe {
                (*f).max_wait = ((*f).max_wait / 2).max(1);
            }
        }
    }
    for n in 0..TYPES {
        let f = build_type(b, n);
        unsafe {
            if (*f).wait_count > 0 {
                (*f).wait_count -= 1;
            }
        }
    }
}
/// # Safety
/// Live builder. Construction and news services run with raw owner fields only;
/// recursive industry calls cannot alias a borrowed Rust owner or vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_try(b: *mut Builder, s: *const Services) {
    build_try(b, unsafe { &*s });
}
/// # Safety
/// Live builder and original ordered daily timer invocation. The company services
/// temporarily change and restore the original current company.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_daily(
    builder: *mut Builder,
    services: *const Services,
) {
    let s = unsafe { &*services };
    let b = build_fields(builder);
    unsafe {
        (*b).daily_counter = (*b).daily_counter.wrapping_add((*b).daily_increment);
    }
    let loops = unsafe { ((*b).daily_counter >> 16) as u16 };
    unsafe {
        (*b).daily_counter &= 65535;
    }
    if loops == 0 {
        return;
    }
    let company = unsafe { (s.company_none)() };
    let mut percentage = 3;
    let wanted = unsafe { (*b).wanted_inds >> 16 };
    if wanted > unsafe { (s.total)() } {
        percentage = (percentage + wanted - unsafe { (s.total)() }).min(9);
    }
    for _ in 0..loops {
        if chance(s, percentage, 100) {
            build_try(builder, s);
        } else {
            let i = unsafe { (s.random_industry)() };
            if !i.is_null() {
                change(i, s, false);
                unsafe { (s.set_dirty)(i) };
            }
        }
    }
    unsafe {
        (s.restore_company)(company);
        (s.directory_dirty)(1);
    }
}
/// # Safety
/// Live builder and original ordered monthly invocation. Destruction happens only
/// after statistics and with all Rust owner borrows ended; next ID matches pool order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_monthly(
    builder: *mut Builder,
    services: *const Services,
) {
    let s = unsafe { &*services };
    let company = unsafe { (s.company_none)() };
    build_monthly(builder, s);
    let mut from = 0;
    loop {
        let e = unsafe { (s.next_tick)(from) };
        if e.industry.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        statistics(e.owner, s);
        if unsafe { (*fields(e.owner)).prod_level == 0 } {
            unsafe { (s.destroy)(e.industry) };
        } else {
            change(e.industry, s, true);
            unsafe { (s.set_dirty)(e.industry) };
        }
    }
    unsafe {
        (s.restore_company)(company);
        (s.directory_dirty)(1);
    }
}
/// # Safety
/// Live industry with no outstanding iterators. Rust owns removal/history freeing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_trim(i: *mut Industry) {
    unsafe {
        let a = &mut (*i).accepted;
        let n = a.iter().rposition(|a| valid(a.cargo)).map_or(0, |n| n + 1);
        a.truncate(n);
        a.shrink_to_fit();
        let p = &mut (*i).produced;
        let n = p.iter().rposition(|p| valid(p.cargo)).map_or(0, |n| n + 1);
        p.truncate(n);
        p.shrink_to_fit();
    }
}

fn tile_xy(x: u32, y: u32, map: Map) -> u32 {
    y.wrapping_mul(map.size_x).wrapping_add(x)
}
fn suitable(tile: u32, fields: bool, rough: bool, s: &Services) -> bool {
    let info = unsafe { (s.farm_tile)(tile) };
    match info.kind {
        MP_CLEAR => {
            if info.snow != 0 {
                return false;
            }
            match info.ground {
                5 => false,
                1 => rough,
                3 => fields,
                _ => true,
            }
        }
        MP_TREES => info.ground != 3 && (rough || info.ground != 1),
        _ => false,
    }
}
fn fence(mut tile: u32, mut size: u32, kind: u8, side: u8, map: Map, s: &Services) {
    let stride = if side == 0 || side == 2 {
        map.size_x
    } else {
        1
    };
    let mask = map.size_x.wrapping_mul(map.size_y).wrapping_sub(1);
    loop {
        tile &= mask;
        if unsafe { (s.fence_wanted)(tile, side) } {
            let mut kind = kind;
            if kind == 1 && chance(s, 1, 7) {
                kind = 2;
            }
            (s.shared().set_fence)(tile, crate::services::DiagDirection(side), u32::from(kind));
        }
        tile = tile.wrapping_add(stride);
        size -= 1;
        if size == 0 {
            break;
        }
    }
}
fn plant(tile: u32, industry: u16, map: Map, s: &Services, landscape: &impl Fn() -> u8) {
    if landscape() == LANDSCAPE_ARCTIC
        && (s.shared().tile_z)(tile) + 2 >= i32::from(unsafe { (s.snow_line)() })
    {
        return;
    }
    let mut r = (rand(s) & 0x303) + 0x404;
    if landscape() == LANDSCAPE_ARCTIC {
        r += 0x404;
    }
    let (width, height) = (map.size_x, map.size_y);
    let sx = r & 255;
    let sy = (r >> 8) & 255;
    let x = tile % width;
    let y = tile / width;
    let x = x - x.min(sx / 2);
    let y = y - y.min(sy / 2);
    let sx = sx.min(width - x);
    let sy = sy.min(height - y);
    if sx == 0 || sy == 0 {
        return;
    }
    let mut count = 0;
    for yy in y..y + sy {
        for xx in x..x + sx {
            count += u32::from(suitable(tile_xy(xx, yy, map), false, false, s));
        }
    }
    if count * 2 < sx * sy {
        return;
    }
    r = rand(s);
    let counter = ((r >> 5) & 7) as u8;
    let field = ((((r >> 8) & 255) * 9) >> 8) as u8;
    for yy in y..y + sy {
        for xx in x..x + sx {
            let tile = tile_xy(xx, yy, map);
            if suitable(tile, true, true, s) {
                unsafe { (s.make_field)(tile, field, counter, industry) };
            }
        }
    }
    let mut kind = 3;
    if landscape() != LANDSCAPE_ARCTIC && landscape() != LANDSCAPE_TROPIC {
        kind = [1, 1, 1, 1, 1, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6][(rand(s) & 15) as usize];
    }
    fence(tile_xy(x, y, map), sy, kind, 0, map, s);
    fence(tile_xy(x, y, map), sx, kind, 3, map, s);
    fence(tile_xy(x + sx - 1, y, map), sy, kind, 2, map, s);
    fence(tile_xy(x, y + sy - 1, map), sx, kind, 1, map, s);
}
fn farm_with_landscape(
    i: *mut c_void,
    id: u16,
    map: Map,
    s: &Services,
    landscape: &impl Fn() -> u8,
) {
    let l = unsafe { (s.location)(i) };
    let x = (u32::from(l.width) / 2)
        .wrapping_add(rand(s) % 31)
        .wrapping_sub(16) as i32;
    let y = (u32::from(l.height) / 2)
        .wrapping_add(rand(s) % 31)
        .wrapping_sub(16) as i32;
    let tile = unsafe { (s.tile_add_wrap)(l.tile, x, y) };
    if tile != INVALID_TILE {
        plant(tile, id, map, s, landscape);
    }
}
fn farm(i: *mut c_void, id: u16, map: Map, s: &Services) {
    // Preserve the earlier production tick's captured landscape until #168.
    farm_with_landscape(i, id, map, s, &|| map.landscape);
}
/// # Safety
/// Live industry, synchronous map/RNG services. No owner references cross calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_farm(
    i: *mut c_void,
    id: u16,
    map: Map,
    s: *const Services,
) {
    farm(i, id, map, unsafe { &*s });
}
fn chop(i: *mut c_void, owner: *mut Industry, map: Map, s: &Services, builder: *mut Builder) {
    if produced_count(owner) == 0 || unsafe { !valid((*produced(owner, 0)).cargo) } {
        return;
    }
    let l = unsafe { (s.location)(i) };
    let width = map.size_x;
    for y in l.tile / width..l.tile / width + u32::from(l.height) {
        for x in l.tile % width..l.tile % width + u32::from(l.width) {
            if unsafe { !(s.tile_completed)(i, tile_xy(x, y, map)) } {
                return;
            }
        }
    }
    let mut spiral = crate::spiral::square(l.tile % width, l.tile / width, 40, width, map.size_y);
    while !spiral.is_end() {
        let tile = tile_xy(spiral.x, spiral.y, map);
        let info = unsafe { (s.farm_tile)(tile) };
        if info.kind == MP_TREES && info.grown != 0 {
            let b = build_fields(builder);
            unsafe {
                (*b).sound_ctr = 1;
                (*b).sound_tile = tile;
                (s.harvest)(tile);
            }
            let amount = unsafe { (s.scale_cargo)(45) };
            unsafe {
                let p = produced(owner, 0);
                (*p).waiting = (u32::from((*p).waiting) + amount).min(65535) as u16;
            }
            break;
        }
        spiral = spiral.advance(width, map.size_y);
    }
}
#[repr(C)]
pub struct ProductionResult {
    pub subtract: [i32; 256],
    pub add: [i32; 256],
    pub again: i32,
    pub cargo_input: [u8; 256],
    pub cargo_output: [u8; 256],
    pub version: u8,
    pub num_input: u8,
    pub num_output: u8,
    pub present: u8,
}
pub type Resolve = unsafe extern "C" fn(*mut c_void, u32, u32, *mut ProductionResult);
/// # Safety
/// Live industry and local bounded C++ resolver context for this call. Resolution
/// observes live canonical fields but cannot execute scripts or commands. It
/// copies only one result; no group or register references survive the service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_production_callback(
    i: *mut c_void,
    owner: *mut Industry,
    context: *mut c_void,
    behaviour: u32,
    reason: u8,
    resolve: Resolve,
    services: *const Services,
) {
    let s = unsafe { &*services };
    let random = if behaviour & (1 << 15) != 0 {
        rand(s)
    } else {
        0
    };
    let multiplier = if behaviour & (1 << 14) != 0 {
        unsafe { i32::from((*fields(owner)).prod_level) }
    } else {
        1
    };
    let mut parameter = u32::from(reason);
    for n in 0..=65536 {
        if n == 65536 {
            unsafe { (s.callback_error)(i, false) };
            break;
        }
        parameter = (parameter & !0x00ff_ff00) | (n << 8);
        let mut result = ProductionResult {
            subtract: [0; 256],
            add: [0; 256],
            again: 0,
            cargo_input: [0; 256],
            cargo_output: [0; 256],
            version: 0,
            num_input: 0,
            num_output: 0,
            present: 0,
        };
        unsafe {
            resolve(context, random, parameter, &raw mut result);
        }
        if result.present == 0 {
            break;
        }
        if result.version == 255 {
            unsafe { (s.callback_error)(i, true) };
            break;
        }
        for n in 0..usize::from(result.num_input) {
            let slot = if result.version < 2 {
                if n >= accepted_count(owner) {
                    break;
                }
                Some(n)
            } else {
                let cargo = result.cargo_input[n];
                if !valid(cargo) {
                    None
                } else {
                    (0..accepted_count(owner))
                        .find(|&n| unsafe { (*accepted(owner, n)).cargo == cargo })
                }
            };
            if let Some(nslot) = slot {
                let a = accepted(owner, nslot);
                unsafe {
                    if result.version < 2 && !valid((*a).cargo) {
                        continue;
                    }
                    (*a).waiting = i32::from((*a).waiting)
                        .wrapping_sub(result.subtract[n].wrapping_mul(multiplier))
                        .clamp(0, 65535) as u16;
                }
            }
        }
        for n in 0..usize::from(result.num_output) {
            let slot = if result.version < 2 {
                if n >= produced_count(owner) {
                    break;
                }
                Some(n)
            } else {
                let cargo = result.cargo_output[n];
                if !valid(cargo) {
                    None
                } else {
                    (0..produced_count(owner))
                        .find(|&n| unsafe { (*produced(owner, n)).cargo == cargo })
                }
            };
            if let Some(nslot) = slot {
                let p = produced(owner, nslot);
                unsafe {
                    if result.version < 2 && !valid((*p).cargo) {
                        continue;
                    }
                    (*p).waiting = i32::from((*p).waiting)
                        .wrapping_add(result.add[n].max(0).wrapping_mul(multiplier))
                        .clamp(0, 65535) as u16;
                }
            }
        }
        if result.again == 0 {
            break;
        }
        parameter = (parameter & 0x00ff_ffff) | ((result.again as u32 & 255) << 24);
    }
    unsafe { (s.set_dirty)(i) };
}

/// # Safety
/// Live builder record; resetting has no callback or allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_type_reset(fields: *mut BuildFields) {
    unsafe {
        *fields = BuildFields {
            max_wait: 1,
            ..BuildFields::default()
        };
    }
}
/// # Safety
/// Live builder; native map-size preconditions constrain the original shift.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_daily_start(
    builder: *mut Builder,
    map_bits: u32,
    init_counter: u8,
) {
    let fields = build_fields(builder);
    unsafe {
        (*fields).daily_increment = (1u32 << map_bits) / 31;
        if init_counter != 0 {
            (*fields).daily_counter = 0;
        }
    }
}

/// # Safety
/// Live owner with serial field-sized accesses. The facade placement-constructs
/// its native `TileArea` prefix once with pinned scalar layout; no reference spans a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_metadata(owner: *mut Industry) -> *mut Metadata {
    unsafe { ptr::addr_of_mut!((*owner).metadata) }
}
/// # Safety
/// Live owner. The returned full byte span stays valid until text mutation or
/// owner destruction. Copy or inspect it before any callback that can mutate it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_text(
    owner: *const Industry,
) -> crate::services::ByteSpan {
    let text = unsafe { &(*owner).text };
    crate::services::ByteSpan {
        data: text.as_ptr(),
        size: text.len(),
    }
}
/// # Safety
/// Live owner; text points to length initialized bytes, disjoint from this owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_text(
    owner: *mut Industry,
    text: *const u8,
    length: usize,
) {
    let target = unsafe { &mut (*owner).text };
    target.clear();
    if length != 0 {
        target.extend_from_slice(unsafe { std::slice::from_raw_parts(text, length) });
    }
}
/// # Safety
/// Live builder and valid original type. View ends before registry mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_registry_view(
    owner: *mut Builder,
    kind: u8,
) -> Slots {
    unsafe { view(&mut (*owner).registry[usize::from(kind)]) }
}
/// # Safety
/// Live builder and valid original type. No registry view survives this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_registry_insert(
    owner: *mut Builder,
    kind: u8,
    id: u16,
) {
    let registry = unsafe { &mut (*owner).registry[usize::from(kind)] };
    if let Err(index) = registry.binary_search(&id) {
        registry.insert(index, id);
    }
}
/// # Safety
/// Live builder and valid original type. No registry view survives this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_registry_erase(
    owner: *mut Builder,
    kind: u8,
    id: u16,
) {
    let registry = unsafe { &mut (*owner).registry[usize::from(kind)] };
    if let Ok(index) = registry.binary_search(&id) {
        registry.remove(index);
    }
}
/// # Safety
/// Live builder and valid original type. No registry view survives this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_registry_clear(owner: *mut Builder, kind: u8) {
    unsafe { (*owner).registry[usize::from(kind)].clear() };
}
