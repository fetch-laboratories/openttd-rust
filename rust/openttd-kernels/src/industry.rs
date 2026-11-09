/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry cargo, history, production and builder control. The C++ shell owns one boxed
//! owner; construction, destruction, cargo distribution and `NewGRF` resolution may revisit
//! it, so no reference survives a world service. Slot growth invalidates views exactly as the
//! original vector does; history allocations stay stable until their slot is removed.
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
use crate::company_type::{CompanyID, Owner};
use crate::history_type::{LAST_MONTH, THIS_MONTH};
use crate::industry_type::{IndustryID, IndustryType};
use crate::industrytype::{
    INDUSTRYLIFE_BLACK_HOLE, IndustryBehaviour, IndustryBehaviours, IndustryLifeType,
    IndustryLifeTypes,
};
use crate::newgrf_callbacks::{CALLBACK_FAILED, IndustryCallbackMask, IndustryCallbackMasks};
use crate::newgrf_industries::IndustryAvailabilityCallType;
use crate::random_func::chance16_i;
use crate::sound_type::SoundFx;
use crate::tile_type::{INVALID_TILE, MP_CLEAR, MP_TREES};
use crate::tilearea::TileArea;
use crate::timer_game_tick::DAY_TICKS;
use std::ffi::c_void;
use std::ptr;
pub(crate) mod definitions;
pub mod resolver;
mod transaction;
pub use transaction::*;
mod commands;
pub use commands::*;
pub(crate) mod tile;
pub use tile::*;
mod construction;
pub use construction::*;
mod generation;
pub use generation::*;
pub(crate) mod policy;
pub use policy::*;
mod lifecycle;
pub use lifecycle::*;
const RECORDS: usize = 61;
const TYPES: usize = crate::industry_type::NUM_INDUSTRYTYPES as usize;
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
            owner: Owner::INVALID.0,
            random_colour: 0,
            founder: CompanyID::INVALID.0,
            construction_type: 0,
            selected_layout: 0,
            exclusive_supplier: CompanyID::INVALID.0,
            exclusive_consumer: CompanyID::INVALID.0,
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
    stations_near: crate::station::StationList,
}
/// The industry's nearby station set, for immediate borrows only.
pub(crate) fn stations_near(owner: *mut Industry) -> *mut crate::station::StationList {
    // SAFETY: Live owner supplied by the caller's resolved shell.
    unsafe { &raw mut (*owner).stations_near }
}
/// Native `Industry::stations_near` facade storage.
/// # Safety
/// `owner` is live; the alias ends with the owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_stations_near(
    owner: *mut Industry,
) -> *mut crate::station::StationList {
    stations_near(owner)
}
/// `Industry::location`, from the canonical metadata.
pub(crate) fn location(owner: *mut Industry) -> TileArea {
    // SAFETY: Live owner supplied by the caller's resolved shell.
    unsafe { lifecycle::location(owner) }
}
/// `Industry::IsCargoAccepted()`: any slot holds a valid cargo.
pub(crate) fn is_cargo_accepted(owner: *mut Industry) -> bool {
    (0..accepted_count(owner)).any(|slot| valid(unsafe { (*accepted(owner, slot)).cargo }))
}
/// Visit each produced slot's cargo in slot order.
pub(crate) fn for_each_produced_cargo(owner: *mut Industry, mut visit: impl FnMut(u8)) {
    for slot in 0..produced_count(owner) {
        // SAFETY: Live owner; immediate copy before the visit.
        visit(unsafe { (*produced(owner, slot)).cargo });
    }
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

/// Spec facts of one industry's type, read together as the original did.
#[derive(Clone, Copy)]
pub struct Observation {
    pub owner: *mut Industry,
    pub behaviour: u32,
    pub callbacks: u16,
    pub life: u8,
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
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entry {
    pub industry: *mut c_void,
    pub owner: *mut Industry,
    pub id: u16,
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
/// Typed `noexcept` C++ services in `industry_ffi.h` order. Each is a world leaf;
/// construction and commands return values at the original call site.
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub next_tick: unsafe extern "C" fn(u32) -> Entry,
    pub location: unsafe extern "C" fn(*mut c_void) -> TileArea,
    pub industry_sound: unsafe extern "C" fn(*mut c_void, u32),
    pub play_sound: unsafe extern "C" fn(u16, u32),
    pub special_effect: unsafe extern "C" fn(*mut c_void, u32, u32) -> u16,
    pub tick_trigger: unsafe extern "C" fn(*mut c_void),
    pub production_callback: unsafe extern "C" fn(*mut c_void, u8),
    pub scale_cargo: unsafe extern "C" fn(u32) -> u32,
    pub environment: *const crate::services::EnvironmentServices,
    pub farm_tile: unsafe extern "C" fn(u32) -> FarmTile,
    pub make_field: unsafe extern "C" fn(u32, u8, u8, u16),
    pub fence_wanted: unsafe extern "C" fn(u32, u8) -> bool,
    pub tile_completed: unsafe extern "C" fn(*mut c_void, u32) -> bool,
    pub harvest: unsafe extern "C" fn(u32),
    pub change_callback: unsafe extern "C" fn(*mut c_void, bool, u32) -> Change,
    /// Shared text owner services for `MapGRFStringID`.
    pub text: *const crate::newgrf_text::Services,
    pub news: unsafe extern "C" fn(*mut c_void, u32, bool, Servicing),
    pub production_news: unsafe extern "C" fn(*mut c_void, u8, i32, Servicing),
    pub callback_error: unsafe extern "C" fn(*mut c_void, bool),
    pub set_dirty: unsafe extern "C" fn(*mut c_void),
    pub destroy: unsafe extern "C" fn(*mut c_void),
    pub opening_news: unsafe extern "C" fn(*mut c_void),
    pub opening_events: unsafe extern "C" fn(*mut c_void),
    pub create: unsafe extern "C" fn(u8, u8) -> *mut c_void,
    pub num_items: unsafe extern "C" fn() -> u32,
    pub get: unsafe extern "C" fn(u16) -> *mut c_void,
    pub type_count: unsafe extern "C" fn(u8) -> u16,
    pub resolver: *const resolver::ResolverServices,
    pub scale_by_map_size: unsafe extern "C" fn(u32) -> u32,
    pub company_none: unsafe extern "C" fn() -> u8,
    pub restore_company: unsafe extern "C" fn(u8),
    pub directory_dirty: unsafe extern "C" fn(u8),
    pub economy_month: unsafe extern "C" fn() -> u8,
    pub economy_year: unsafe extern "C" fn() -> i32,
    pub days_since_last_month: unsafe extern "C" fn() -> u32,
    pub landscape: unsafe extern "C" fn() -> u8,
    pub economy_type: unsafe extern "C" fn() -> u8,
    pub fund_only: unsafe extern "C" fn() -> bool,
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
        crate::services::Map::from_log_x(self.size_x.trailing_zeros())
    }
}
use crate::clear_map::ClearGround;
use crate::direction_type::DiagDirection;
use crate::landscape_type::{ARCTIC, LandscapeType, TROPIC};
use crate::tree_map::TreeGround;
/// `GetIndustrySpec(i->type)` through the owner's type.
fn spec_of(owner: *mut Industry) -> *const crate::industrytype::IndustrySpec {
    crate::industrytype::industry(u16::from(unsafe { (*owner).metadata.kind }))
}
fn observe(owner: *mut Industry) -> Observation {
    let spec = spec_of(owner);
    unsafe {
        Observation {
            owner,
            behaviour: (*spec).behaviour,
            callbacks: (*spec).callback_mask,
            life: (*spec).life_type,
            kind: (*owner).metadata.kind,
            up_text: (*spec).production_up_text,
            down_text: (*spec).production_down_text,
            closure_text: (*spec).closure_text,
        }
    }
}
fn rand(s: &Services) -> u32 {
    s.shared().random()
}
fn range(s: &Services, n: u32) -> u32 {
    s.shared().random_range(n)
}
fn chance(s: &Services, a: u32, b: u32) -> bool {
    chance16_i(a, b, rand(s))
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
    crate::cargo_type::is_valid_cargo_type(cargo)
}
/// `Industry::GetProduced`, including its invalid-cargo empty slot.
unsafe fn get_produced(owner: *const Industry, slot: usize) -> *const Produced {
    static EMPTY: Produced = Produced {
        cargo: crate::cargo_type::INVALID_CARGO,
        waiting: 0,
        rate: 0,
        history: [Pair {
            first: 0,
            second: 0,
        }; RECORDS],
    };
    let produced = unsafe { &(*owner).produced };
    produced.get(slot).map_or(&raw const EMPTY, ptr::from_ref)
}
/// `Industry::GetAccepted`. The shared empty slot is immutable, as in the header.
unsafe fn get_accepted(owner: *const Industry, slot: usize) -> *const Accepted {
    struct Empty(Accepted);
    // SAFETY: immutable static, null history pointer; no mutation is exposed.
    unsafe impl Sync for Empty {}
    static EMPTY: Empty = Empty(Accepted {
        cargo: crate::cargo_type::INVALID_CARGO,
        waiting: 0,
        accumulated_waiting: 0,
        last_accepted: 0,
        history: ptr::null_mut(),
    });
    let accepted = unsafe { &(*owner).accepted };
    accepted.get(slot).map_or(&raw const EMPTY.0, ptr::from_ref)
}

/// `Industry::GetCargoProduced`: invalid cargo never matches, then first slot wins.
unsafe fn get_cargo_produced(owner: *const Industry, cargo: u8) -> Option<usize> {
    if !valid(cargo) {
        return None;
    }
    unsafe { &(*owner).produced }
        .iter()
        .position(|p| p.cargo == cargo)
}
/// `Industry::GetCargoAccepted`, with the same validity and ordering contract.
pub(crate) unsafe fn get_cargo_accepted(owner: *const Industry, cargo: u8) -> Option<usize> {
    if !valid(cargo) {
        return None;
    }
    unsafe { &(*owner).accepted }
        .iter()
        .position(|a| a.cargo == cargo)
}
/// SAFETY: Live serialized industry owner; observe after the refusal callback.
pub(crate) unsafe fn exclusive_supplier(owner: *const Industry) -> crate::company_type::CompanyID {
    unsafe { crate::company_type::CompanyID((*owner).metadata.exclusive_supplier) }
}
/// SAFETY: Live accepted slot selected by `GetCargoAccepted`; the date leaf cannot resize slots.
pub(crate) unsafe fn accept_delivery(
    owner: *mut Industry,
    slot: usize,
    pieces: u32,
    date: impl FnOnce() -> i32,
) -> u32 {
    let a = accepted(owner, slot);
    let amount = pieces.min(u32::from(u16::MAX - unsafe { (*a).waiting }));
    unsafe {
        (*a).waiting = (*a).waiting.wrapping_add(amount as u16);
        let history = openttd_rust_industry_history(a).cast::<[Pair; RECORDS]>();
        let count = &mut (*history)[usize::from(THIS_MONTH)].first;
        *count = count.wrapping_add(amount as u16);
    }
    let now = date();
    unsafe { (*a).last_accepted = now };
    amount
}
fn transported(p: Pair) -> u32 {
    u32::from(p.pct_transported())
}
impl Pair {
    /// `Industry::ProducedHistory::PctTransported`, shared by production and resolution.
    fn pct_transported(self) -> u8 {
        let p = self;
        if p.first == 0 {
            0
        } else {
            (u32::from(p.second) * 256 / u32::from(p.first)).min(255) as u8
        }
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
fn produce(e: Entry, s: &Services, builder: *mut Builder) {
    let (i, owner) = (e.industry, e.owner);
    let f = fields(owner);
    if unsafe { (*f).counter.is_multiple_of(64) } {
        let r = rand(s);
        if chance16_i(1, 14, r) {
            let kind = unsafe { (*owner).metadata.kind };
            let sounds = unsafe {
                (*crate::industrytype::industry(u16::from(kind)))
                    .random_sounds
                    .len()
            } as u16;
            if sounds != 0
                && unsafe { s.shared().settings().ambient.read() }
                && (0..produced_count(owner)).any(|n| unsafe {
                    (*produced(owner, n)).history[usize::from(LAST_MONTH)].first > 0
                })
            {
                unsafe { (s.industry_sound)(i, ((r >> 16) * u32::from(sounds)) >> 16) };
            }
        }
    }
    unsafe {
        (*f).counter = (*f).counter.wrapping_sub(1);
    }
    let callbacks = unsafe { (*spec_of(owner)).callback_mask };
    let callback = IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::Production256Ticks);
    // Scale the callback interval by the cargo scale percentage.
    if callback
        && unsafe {
            u32::from((*f).counter)
                % crate::economy_func::scale_by_inverse_cargo_scale(
                    crate::timer_game_tick::INDUSTRY_PRODUCE_TICKS,
                    s.shared().settings().industry_cargo_scale.read(),
                )
                == 0
        }
    {
        unsafe { (s.production_callback)(i, 1) };
        helper(owner, s, false);
    }
    // Other production and special effects run every 256 ticks. Scaling their
    // cargo amount preserves a trickle of production at low scale factors.
    if unsafe { !(*f).counter.is_multiple_of(256) } {
        return;
    }
    if !callback {
        helper(owner, s, true);
    }
    let behaviour = unsafe { (*spec_of(owner)).behaviour };
    for (flag, parameter) in [
        (IndustryBehaviour::PlantFields, 0),
        (IndustryBehaviour::CutTrees, 1),
    ] {
        if !IndustryBehaviours(behaviour).test(flag) {
            continue;
        }
        let cb = if IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::SpecialEffect) {
            let random = rand(s);
            unsafe { (s.special_effect)(i, random, parameter) }
        } else {
            CALLBACK_FAILED
        };
        let act = if cb != CALLBACK_FAILED {
            (s.shared().grf_boolean_callback)(
                unsafe { (*spec_of(owner)).grf_prop.grffile },
                crate::newgrf_callbacks::CBID_INDUSTRY_SPECIAL_EFFECT,
                cb,
            )
        } else if parameter == 0 {
            chance(s, 1, 8)
        } else {
            unsafe {
                (*f).counter
                    .is_multiple_of(crate::timer_game_tick::INDUSTRY_CUT_TREE_TICKS as u16)
            }
        };
        if act {
            if parameter == 0 {
                farm(i, e.id, tick_map(s), s);
            } else {
                chop(i, owner, tick_map(s), s, builder);
            }
        }
    }
    unsafe { (s.tick_trigger)(i) };
}
/// # Safety
/// Live builder and synchronous services; C++ supplies serial world
/// access and terminating noexcept environmental failures. No borrowed owner
/// reference survives a service, including recursive production on construction.
pub(crate) unsafe fn tick(builder: *mut Builder, services: *const Services) {
    let s = unsafe { &*services };
    let b = build_fields(builder);
    unsafe {
        if (*b).sound_ctr != 0 {
            (*b).sound_ctr = (*b).sound_ctr.wrapping_add(1);
            if (*b).sound_ctr == 75 {
                if s.shared().settings().ambient.read() {
                    (s.play_sound)(u16::from(SoundFx::SND_37_LUMBER_MILL_2.0), (*b).sound_tile);
                }
            } else if (*b).sound_ctr == 160 {
                (*b).sound_ctr = 0;
                if s.shared().settings().ambient.read() {
                    (s.play_sound)(u16::from(SoundFx::SND_36_LUMBER_MILL_3.0), (*b).sound_tile);
                }
            }
        }
    }
    if crate::session::mode() == crate::openttd::GameMode::EDITOR {
        return;
    }
    let mut from = u32::from(IndustryID::BEGIN.0);
    loop {
        let e = unsafe { (s.next_tick)(from) };
        if e.industry.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        produce(e, s, builder);
        if (s.shared().tick_counter)().wrapping_add(u64::from(e.id)) % DAY_TICKS as u64 == 0 {
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
pub unsafe extern "C" fn openttd_rust_industry_transport_abi(
    i: *mut c_void,
    owner: *mut Industry,
    minimal_cargo: u8,
    recession: bool,
    services: *const Services,
) -> u8 {
    let s = unsafe { &*services };
    let mut moved = false;
    let mut catchment = None;
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
            (*p).history[usize::from(THIS_MONTH)].first = (*p).history[usize::from(THIS_MONTH)]
                .first
                .wrapping_add(cw as u16);
        }
        let c = catchment.get_or_insert_with(|| crate::station::Ctx::from_shared(s.shared()));
        let source = crate::cargo_storage::Source {
            id: (c.c.industry_id)(i.cast()).0,
            source_type: crate::source_type::Source::INDUSTRY,
        };
        // SAFETY: Live owner; the station set is re-read between observations.
        let am = unsafe {
            crate::station::move_goods_to_station(
                crate::cargo_type::CargoType((*p).cargo),
                cw,
                source,
                stations_near(owner),
                crate::company_type::CompanyID((*owner).metadata.exclusive_consumer),
                c,
            )
        };
        unsafe {
            (*p).history[usize::from(THIS_MONTH)].second = (*p).history[usize::from(THIS_MONTH)]
                .second
                .wrapping_add(am as u16);
        }
        moved |= am != 0;
    }
    u8::from(moved)
}
fn recompute(owner: *mut Industry) {
    // Round rates up so an oilrig always produces some passengers.
    let f = fields(owner);
    for n in 0..produced_count(owner) {
        let spec = crate::industrytype::industry(u16::from(unsafe { (*owner).metadata.kind }));
        let rate = u32::from(unsafe { (*spec).production_rate[n] });
        let value = rate * unsafe { u32::from((*f).prod_level) };
        unsafe {
            (*produced(owner, n)).rate =
                value.div_ceil(u32::from(PRODLEVEL_DEFAULT)).min(255) as u8;
        }
    }
}
/// # Safety
/// Live industry and its owner; the original `UsesOriginalEconomy` precondition applies.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_recompute(owner: *mut Industry) {
    recompute(owner);
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
            history[first] = policy::sum_history(&history[first - division..first]);
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
            if (*p).history[usize::from(THIS_MONTH)].first != 0 {
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
    if IndustryBehaviours(o.behaviour).test(IndustryBehaviour::DontIncrProd)
        && LandscapeType(unsafe { (s.landscape)() }) == LandscapeType::Temperate
    {
        return false;
    }
    !IndustryBehaviours(o.behaviour).test(IndustryBehaviour::CanCloseLastInstance)
        && unsafe { (s.type_count)(o.kind) } <= 1
}
const PERCENT_TRANSPORTED_60: u32 = 153;
const PERCENT_TRANSPORTED_80: u32 = 204;

fn change(i: *mut c_void, owner: *mut Industry, s: &Services, monthly: bool) {
    let o = observe(owner);
    let f = fields(o.owner);
    let mut str = 0;
    let mut close = false;
    let mut standard = false;
    let mut suppress = false;
    let mut recalc = false;
    // Industries with production callbacks use the original economy.
    let original = original_economy(o.callbacks, unsafe { (s.economy_type)() });
    let mut div = 0u8;
    let mut mul = 0u8;
    let mut increment = 0i8;
    let callback = o.callbacks
        & (1 << if monthly {
            IndustryCallbackMask::MonthlyProdChange.0
        } else {
            IndustryCallbackMask::ProductionChange.0
        })
        != 0;
    if callback {
        let random = rand(s);
        let result = unsafe { (s.change_callback)(i, monthly, random) };
        let mut res = result.result;
        if res != CALLBACK_FAILED {
            // A failed callback does nothing; a successful one may supply a custom message.
            suppress = res & 128 != 0;
            if res & 256 != 0 {
                let grfid = unsafe { (*spec_of(owner)).grf_prop.grfid };
                str = unsafe {
                    crate::newgrf_text::openttd_rust_grf_text_map(
                        &*s.text,
                        grfid,
                        u32::from(result.reg as u16),
                    )
                };
            }
            res &= 15;
            match res {
                0 => {}
                1 => div = 1,
                2 => mul = 1,
                // Announce closure now; physical removal happens next month.
                3 => close = true,
                // Apply the standard random change as for a primary industry.
                4 => standard = true,
                5..=8 => div = (res - 3) as u8,
                9..=12 => mul = (res - 7) as u8,
                13 => increment = -1,
                14 => increment = 1,
                // Set the level from the third byte of register 0x100.
                15 => {
                    unsafe {
                        (*f).prod_level = ((result.reg as u32 >> 16) & 255)
                            .clamp(u32::from(PRODLEVEL_MINIMUM), u32::from(PRODLEVEL_MAXIMUM))
                            as u8;
                    }
                    recalc = true;
                }
                _ => unreachable!(),
            }
        }
    } else if monthly == original
        || (!original && unsafe { (s.economy_type)() } == 2)
        || o.life == INDUSTRYLIFE_BLACK_HOLE.0
    {
        return;
    }
    if standard
        || (!callback
            && o.life
                & (IndustryLifeTypes::from_element(IndustryLifeType::Extractive).0
                    | IndustryLifeTypes::from_element(IndustryLifeType::Organic).0)
                != 0)
    {
        let only_decrease = IndustryBehaviours(o.behaviour).test(IndustryBehaviour::DontIncrProd)
            && LandscapeType(unsafe { (s.landscape)() }) == LandscapeType::Temperate;
        if original {
            if only_decrease || chance(s, 1, 3) {
                let pct = if produced_count(o.owner) > 0 {
                    unsafe { transported((*produced(o.owner, 0)).history[usize::from(LAST_MONTH)]) }
                } else {
                    0
                };
                // Above 60% transported, increase with 66% probability; otherwise 33%.
                if !only_decrease && (pct > PERCENT_TRANSPORTED_60) != chance(s, 1, 3) {
                    mul = 1;
                } else {
                    div = 1;
                }
            }
        } else if unsafe { (s.economy_type)() } == 1 {
            close = unsafe {
                (*f).ctlflags
                    & (IndustryControlFlags::from_element(
                        IndustryControlFlag::NoProductionDecrease,
                    )
                    .0 | IndustryControlFlags::from_element(IndustryControlFlag::NoClosure).0)
                    == 0
            };
            for n in 0..produced_count(o.owner) {
                let p = produced(o.owner, n);
                if unsafe { !valid((*p).cargo) } {
                    continue;
                }
                let random = rand(s);
                let old = unsafe { i32::from((*p).rate) };
                let pct = unsafe { transported((*p).history[usize::from(LAST_MONTH)]) };
                // Above 60% transported, the multiplier is 1; otherwise it is -1.
                let mut mult = if pct > PERCENT_TRANSPORTED_60 { 1 } else { -1 };
                // Temperate oil wells always decrease. Other industries have a 33%
                // decrease chance, reduced to 16% above 80% transported.
                if only_decrease {
                    mult = -1;
                } else if chance16_i(1, if pct > PERCENT_TRANSPORTED_80 { 6 } else { 3 }, random) {
                    mult *= -1;
                }
                let mut new = old;
                // A 4.5% chance changes production by 3-23%, or at least one unit.
                if chance16_i(1, 22, random >> 16) {
                    new = new.wrapping_add(
                        mult * (((range(s, 50) + 10) * old as u32) >> 8).max(1) as i32,
                    );
                }
                // Prevent overflow and excessive oil-rig passenger production.
                new = new.clamp(1, 255);
                if unsafe {
                    (*p).cargo
                        == crate::cargo_registry::openttd_rust_cargo_by_label(
                            crate::cargo_type::CT_PASSENGERS,
                        )
                } && !IndustryBehaviours(o.behaviour).test(IndustryBehaviour::NoPaxProdClamp)
                {
                    new = new.clamp(0, 16);
                }
                // Overrides prevent applying a change in the forbidden direction.
                if unsafe {
                    (IndustryControlFlags((*f).ctlflags)
                        .test(IndustryControlFlag::NoProductionDecrease)
                        && new < old)
                        || (IndustryControlFlags((*f).ctlflags)
                            .test(IndustryControlFlag::NoProductionIncrease)
                            && new > old)
                } {
                    continue;
                }
                // The lowest production rate must not cancel a pending closure.
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
                    unsafe {
                        (s.production_news)(i, (*p).cargo, percent, servicing(i, o.owner, s));
                    };
                }
            }
        }
    }
    if unsafe {
        (IndustryControlFlags((*f).ctlflags).test(IndustryControlFlag::NoProductionDecrease)
            && (div > 0 || increment < 0))
            || (IndustryControlFlags((*f).ctlflags).test(IndustryControlFlag::NoProductionIncrease)
                && (mul > 0 || increment > 0))
    } {
        return;
    }
    if unsafe { IndustryControlFlags((*f).ctlflags).test(IndustryControlFlag::ExternalProdLevel) } {
        div = 0;
        mul = 0;
        increment = 0;
    }
    if !callback
        && IndustryLifeTypes(o.life).test(IndustryLifeType::Processing)
        && unsafe {
            (s.economy_year)().wrapping_sub((*f).last_prod_year)
                >= PROCESSING_INDUSTRY_ABANDONMENT_YEARS.0
        }
        && chance(s, 1, if original { 2 } else { 180 })
    {
        close = true;
    }
    while mul != 0 && unsafe { (*f).prod_level < PRODLEVEL_MAXIMUM } {
        mul = mul.wrapping_sub(1);
        unsafe {
            (*f).prod_level =
                (u16::from((*f).prod_level) * 2).min(u16::from(PRODLEVEL_MAXIMUM)) as u8;
        }
        recalc = true;
        if str == 0 {
            str = o.up_text;
        }
    }
    while div != 0 && !close {
        div = div.wrapping_sub(1);
        if unsafe { (*f).prod_level == PRODLEVEL_MINIMUM } {
            close = true;
            break;
        }
        unsafe {
            (*f).prod_level = ((*f).prod_level / 2).max(PRODLEVEL_MINIMUM);
        }
        recalc = true;
        if str == 0 {
            str = o.down_text;
        }
    }
    if increment != 0 {
        if increment < 0 && unsafe { (*f).prod_level == PRODLEVEL_MINIMUM } {
            close = true;
        } else {
            unsafe {
                (*f).prod_level = ((i32::from((*f).prod_level) + i32::from(increment)) as u32)
                    .clamp(u32::from(PRODLEVEL_MINIMUM), u32::from(PRODLEVEL_MAXIMUM))
                    as u8;
            }
            recalc = true;
        }
    }
    // Outside the smooth economy, production rates must match the production level.
    if recalc {
        recompute(o.owner);
    }
    if close
        && !protected(o, s)
        && unsafe { !IndustryControlFlags((*f).ctlflags).test(IndustryControlFlag::NoClosure) }
    {
        unsafe {
            (*f).prod_level = PRODLEVEL_CLOSURE;
            (s.set_dirty)(i);
        }
        str = o.closure_text;
    }
    if !suppress && str != 0 {
        unsafe {
            (s.news)(
                i,
                str,
                close,
                if close {
                    Servicing::Nobody
                } else {
                    servicing(i, o.owner, s)
                },
            );
        };
    }
}
fn type_data(f: *mut BuildFields, kind: IndustryType, s: &Services) -> bool {
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
fn game_probability(kind: IndustryType, s: &Services) -> u64 {
    if unsafe { (s.fund_only)() } {
        return 0;
    }
    let spec = crate::industrytype::industry(u16::from(kind.0));
    let climate = unsafe { s.shared().settings().climate.read() };
    let (behavior, enabled, layouts, appear) = unsafe {
        (
            (*spec).behaviour,
            (*spec).enabled,
            !(*spec).layouts.is_empty(),
            (*spec).appear_ingame[usize::from(climate)],
        )
    };
    let year = crate::timer_game_common::calendar_year().0;
    if !enabled
        || !layouts
        || (IndustryBehaviours(behavior).test(IndustryBehaviour::Before1950) && year > 1950)
        || (IndustryBehaviours(behavior).test(IndustryBehaviour::After1960) && year < 1960)
    {
        return 0;
    }
    let probability = unsafe {
        resolver::openttd_rust_industry_probability(
            kind.0,
            IndustryAvailabilityCallType::IACT_RANDOMCREATION.0,
            u32::from(appear),
            s.resolver,
        )
    } as u8;
    if probability == 0 {
        return 0;
    }
    u64::from(probability)
        | ((u64::from(IndustryBehaviours(behavior).test(IndustryBehaviour::CanCloseLastInstance)))
            << 32)
}
/// # Safety
/// Live build record; no reference is retained across the resolver service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_type(
    f: *mut BuildFields,
    kind: u8,
    s: *const Services,
) -> u8 {
    u8::from(type_data(f, IndustryType(kind), unsafe { &*s }))
}
fn build_fields(builder: *mut Builder) -> *mut BuilderFields {
    unsafe { ptr::addr_of_mut!((*builder).fields) }
}
fn build_type(b: *mut BuilderFields, n: usize) -> *mut BuildFields {
    unsafe { ptr::addr_of_mut!((*b).builddata[n]) }
}
fn build_reset(builder: *mut Builder, _s: &Services) {
    let b = build_fields(builder);
    unsafe {
        (*b).wanted_inds = total(builder).wrapping_shl(16);
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
    // Avoid running out of unconnected industries by adding a fraction each
    // month, but only if the builder can keep up. At most two industries behind
    // on small maps and 100 on the largest (about six months of attempts).
    let max = 1 + unsafe { (s.scale_by_map_size)(3) }.min(99);
    if total(builder).wrapping_add(max) >= unsafe { (*b).wanted_inds >> 16 } {
        // Lower 16 bits are fractional: 3.5 industries per decade / (10 * 12 months).
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
    let mut planned = 0u32; // Number of industries planned in the build data.
    for n in 0..TYPES {
        let f = build_type(b, n);
        changed |= type_data(f, IndustryType(n as u8), s);
        planned += unsafe { u32::from((*f).target_count) };
    }
    let mut amount = unsafe { (*b).wanted_inds >> 16 }; // Desired total number.
    changed |= planned != amount;
    if !changed {
        return; // Nothing changed; do not re-randomize the target counts.
    }
    // Initialize target counts, including industries that should always exist.
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
    // Subtract forced industries from the number available for construction.
    amount = amount.saturating_sub(forced);
    // Assign remaining target counts using probabilities as weights.
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
fn place(
    kind: IndustryType,
    creation: IndustryAvailabilityCallType,
    hard: bool,
    s: &Services,
) -> *mut c_void {
    for _ in 0..if hard { 10000 } else { 2000 } {
        let i = unsafe { (s.create)(kind.0, creation.0) };
        if !i.is_null() {
            return i;
        }
    }
    ptr::null_mut()
}
fn build_try(builder: *mut Builder, s: &Services) {
    targets(builder, s);
    let b = build_fields(builder);
    let mut missing = 0i32; // Number of industries still needed.
    let mut eligible = 0; // Number of industry types eligible for construction.
    let mut probability = 0u32;
    let mut forced = TYPES; // Type to build forcibly, or no forced type.
    for n in 0..TYPES {
        let f = build_type(b, n);
        let difference = unsafe { i32::from((*f).target_count) } - count(s, n);
        missing = missing.wrapping_add(difference);
        if unsafe { (*f).wait_count > 0 } {
            continue;
        }
        if difference > 0 {
            // Force a missing type that must exist, trying the most-needed first.
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
    if (s.shared().recession)() || (forced == TYPES && (missing <= 0 || probability == 0)) {
        eligible = 0;
    }
    if eligible >= 1 {
        // Unless forced, select by weight. With one eligible type, do not draw RNG.
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
        let i = place(
            IndustryType(n as u8),
            IndustryAvailabilityCallType::IACT_RANDOMCREATION,
            false,
            s,
        );
        let f = build_type(b, n);
        if i.is_null() {
            unsafe {
                // Compensate for the decrement below.
                (*f).wait_count = (*f).max_wait.wrapping_add(1);
                (*f).max_wait = (u32::from((*f).max_wait) + 2).min(1000) as u16;
            }
        } else {
            advertise(i, s);
            unsafe {
                // Reduce the waiting time after a successful construction.
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
pub(crate) unsafe fn openttd_rust_industry_daily(builder: *mut Builder, services: *const Services) {
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
    if wanted > total(builder) {
        percentage = (percentage + wanted - total(builder)).min(9);
    }
    for _ in 0..loops {
        if chance(s, percentage, 100) {
            build_try(builder, s);
        } else {
            let e = random_entry(s);
            if !e.industry.is_null() {
                change(e.industry, e.owner, s, false);
                unsafe { (s.set_dirty)(e.industry) };
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
pub(crate) unsafe fn openttd_rust_industry_monthly(
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
            change(e.industry, e.owner, s, true);
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

fn suitable(tile: u32, fields: bool, rough: bool, s: &Services) -> bool {
    let info = unsafe { (s.farm_tile)(tile) };
    match info.kind {
        MP_CLEAR => {
            if info.snow != 0 {
                return false;
            }
            match ClearGround(info.ground) {
                ClearGround::CLEAR_DESERT => false,
                ClearGround::CLEAR_ROUGH => rough,
                ClearGround::CLEAR_FIELDS => fields,
                _ => true,
            }
        }
        MP_TREES => {
            TreeGround(info.ground) != TreeGround::TREE_GROUND_SHORE
                && (rough || TreeGround(info.ground) != TreeGround::TREE_GROUND_ROUGH)
        }
        _ => false,
    }
}
fn fence(mut tile: u32, mut size: u32, kind: u8, side: DiagDirection, map: Map, s: &Services) {
    let stride = if side == DiagDirection::DIAGDIR_NE || side == DiagDirection::DIAGDIR_SW {
        map.size_x
    } else {
        1
    };
    let mask = map.size_x.wrapping_mul(map.size_y).wrapping_sub(1);
    loop {
        tile &= mask;
        if unsafe { (s.fence_wanted)(tile, side.0) } {
            let mut kind = kind;
            if kind == 1 && chance(s, 1, 7) {
                kind = 2;
            }
            (s.shared().set_fence)(tile, side, u32::from(kind));
        }
        tile = tile.wrapping_add(stride);
        size -= 1;
        if size == 0 {
            break;
        }
    }
}
fn plant(tile: u32, industry: u16, map: Map, s: &Services, landscape: &impl Fn() -> u8) {
    if landscape() == ARCTIC
        && (s.shared().tile_z)(tile) + 2 >= i32::from(crate::snow_line::get(s.shared()))
    {
        return;
    }
    let mut r = (rand(s) & 0x303) + 0x404;
    if landscape() == ARCTIC {
        r += 0x404;
    }
    let (width, height) = (map.size_x, map.size_y);
    let tiles = crate::map_func::Map::from_log_x(width.trailing_zeros());
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
            count += u32::from(suitable(tiles.tile_xy(xx, yy), false, false, s));
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
            let tile = tiles.tile_xy(xx, yy);
            if suitable(tile, true, true, s) {
                unsafe { (s.make_field)(tile, field, counter, industry) };
            }
        }
    }
    let mut kind = 3;
    if landscape() != ARCTIC && landscape() != TROPIC {
        kind = [1, 1, 1, 1, 1, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6][(rand(s) & 15) as usize];
    }
    fence(
        tiles.tile_xy(x, y),
        sy,
        kind,
        DiagDirection::DIAGDIR_NE,
        map,
        s,
    );
    fence(
        tiles.tile_xy(x, y),
        sx,
        kind,
        DiagDirection::DIAGDIR_NW,
        map,
        s,
    );
    fence(
        tiles.tile_xy(x + sx - 1, y),
        sy,
        kind,
        DiagDirection::DIAGDIR_SW,
        map,
        s,
    );
    fence(
        tiles.tile_xy(x, y + sy - 1),
        sx,
        kind,
        DiagDirection::DIAGDIR_SE,
        map,
        s,
    );
}
fn farm_with_landscape(
    i: *mut c_void,
    id: u16,
    map: Map,
    s: &Services,
    landscape: &impl Fn() -> u8,
) {
    let l = unsafe { (s.location)(i) };
    let x = (u32::from(l.w) / 2)
        .wrapping_add(rand(s) % 31)
        .wrapping_sub(16) as i32;
    let y = (u32::from(l.h) / 2)
        .wrapping_add(rand(s) % 31)
        .wrapping_sub(16) as i32;
    let tile = map.coordinates().tile_add_wrap(
        l.tile,
        x,
        y,
        map.size_y,
        (unsafe { &*s.environment }.freeform_edges)(),
    );
    if tile != INVALID_TILE {
        plant(tile, id, map, s, landscape);
    }
}
fn tick_map(s: &Services) -> Map {
    let world = crate::map_func::World::new(s.shared());
    Map {
        size_x: world.dimensions.size_x(),
        size_y: world.size_y(),
        landscape: unsafe { s.shared().settings().climate.read() },
    }
}
fn farm(i: *mut c_void, id: u16, map: Map, s: &Services) {
    farm_with_landscape(i, id, map, s, &|| unsafe {
        s.shared().settings().climate.read()
    });
}
/// # Safety
/// Live industry, synchronous map/RNG services. No owner references cross calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_farm(i: *mut c_void, id: u16, s: *const Services) {
    let s = unsafe { &*s };
    farm(i, id, tick_map(s), s);
}
fn chop(i: *mut c_void, owner: *mut Industry, map: Map, s: &Services, builder: *mut Builder) {
    if produced_count(owner) == 0 || unsafe { !valid((*produced(owner, 0)).cargo) } {
        return;
    }
    let l = unsafe { (s.location)(i) };
    let width = map.size_x;
    let tiles = crate::map_func::Map::from_log_x(width.trailing_zeros());
    for y in l.tile / width..l.tile / width + u32::from(l.h) {
        for x in l.tile % width..l.tile % width + u32::from(l.w) {
            if unsafe { !(s.tile_completed)(i, tiles.tile_xy(x, y)) } {
                return;
            }
        }
    }
    let mut spiral = crate::spiral::square(l.tile % width, l.tile / width, 40, width, map.size_y);
    while !spiral.is_end() {
        let tile = tiles.tile_xy(spiral.x, spiral.y);
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
/// Native `IndustriesResolverObject` in caller-owned stack storage; checked by C++.
#[repr(C, align(16))]
pub struct ProductionStorage {
    storage: [std::mem::MaybeUninit<u8>; 1024],
}
/// One live native production resolver. Rust never inspects its representation.
#[repr(C)]
pub struct ProductionContext {
    _private: [u8; 0],
}
#[repr(C)]
pub struct DeliveryServices {
    pub construct:
        unsafe extern "C" fn(*mut ProductionStorage, *mut ShellHandle) -> *mut ProductionContext,
    pub destroy: unsafe extern "C" fn(*mut ProductionContext),
    pub resolve: unsafe extern "C" fn(*mut ProductionContext, u32, u32, *mut ProductionResult),
}
/// SAFETY: Live serialized industry, immutable services and spec; callbacks retain no owner borrows.
pub(crate) unsafe fn trigger_delivery(
    shell: *mut ShellHandle,
    owner: *mut Industry,
    id: crate::industry_type::IndustryID,
    resolver: &resolver::ResolverServices,
    services: &Services,
    delivery: &DeliveryServices,
) {
    use crate::newgrf_callbacks::{
        INDUSTRY_PRODUCTION_256_TICKS, INDUSTRY_PRODUCTION_CARGO_ARRIVAL,
    };
    let spec = resolver::ResolverServices::spec(unsafe { (*owner).metadata.kind });
    let mask = unsafe { (*spec).callback_mask };
    unsafe { (*owner).fields.was_cargo_delivered = 1 };
    if mask & (INDUSTRY_PRODUCTION_CARGO_ARRIVAL | INDUSTRY_PRODUCTION_256_TICKS) != 0 {
        if mask & INDUSTRY_PRODUCTION_CARGO_ARRIVAL != 0 {
            let mut storage = std::mem::MaybeUninit::<ProductionStorage>::uninit();
            let context = unsafe { (delivery.construct)(storage.as_mut_ptr(), shell) };
            let behaviour = resolver::ResolverServices::behaviour(spec);
            production_callback(
                shell.cast(),
                owner,
                behaviour,
                0,
                services,
                |random, parameter, result| unsafe {
                    (delivery.resolve)(context, random, parameter, result);
                },
            );
            unsafe { (delivery.destroy)(context) };
        } else {
            unsafe { (services.set_dirty)(shell.cast()) };
        }
    } else {
        for input in 0..accepted_count(owner) {
            let a = accepted(owner, input);
            let waiting = unsafe { (*a).waiting };
            if waiting == 0 || !valid(unsafe { (*a).cargo }) {
                continue;
            }
            for output in 0..produced_count(owner) {
                let p = produced(owner, output);
                if !valid(unsafe { (*p).cargo }) {
                    continue;
                }
                let old = unsafe { (*p).waiting };
                let multiplier = unsafe { (*spec).input_cargo_multiplier[input][output] };
                let product = i32::from(waiting).wrapping_mul(i32::from(multiplier));
                unsafe {
                    (*p).waiting =
                        i32::from(old).wrapping_add(product / 256).clamp(0, 65535) as u16;
                };
            }
            unsafe { (*a).waiting = 0 };
        }
    }
    unsafe {
        openttd_rust_industry_randomise(
            shell.cast(),
            owner,
            id.0,
            crate::industry_type::RANDOM_TRIGGER_CARGO_RECEIVED,
            services.shared().map().size_x(),
            services,
            resolver.lifecycle(),
        );
        openttd_rust_industry_animate_abi(
            owner,
            id.0,
            crate::industry_type::ANIMATION_TRIGGER_CARGO_RECEIVED,
            services.shared().map().size_x(),
            services,
            resolver.lifecycle(),
        );
    }
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
    production_callback(
        i,
        owner,
        behaviour,
        reason,
        unsafe { &*services },
        |random, parameter, result| unsafe {
            resolve(context, random, parameter, result);
        },
    );
}
fn production_callback(
    i: *mut c_void,
    owner: *mut Industry,
    behaviour: u32,
    reason: u8,
    s: &Services,
    mut resolve: impl FnMut(u32, u32, *mut ProductionResult),
) {
    let random = if IndustryBehaviours(behaviour).test(IndustryBehaviour::ProdCallbackRandom) {
        rand(s)
    } else {
        0
    };
    let multiplier = if IndustryBehaviours(behaviour).test(IndustryBehaviour::ProdMultiHandling) {
        unsafe { i32::from((*fields(owner)).prod_level) }
    } else {
        1
    };
    let mut parameter = u32::from(reason);
    // The loop number occupies 16 NewGRF bits. Limit calls to break infinite loops.
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
        resolve(random, parameter, &raw mut result);
        if result.present == 0 {
            break;
        }
        // An invalid-on-load result is noncritical: report it and keep the game running.
        if result.version == 255 {
            unsafe { (s.callback_error)(i, true) };
            break;
        }
        for n in 0..usize::from(result.num_input) {
            // Old callback parameters are slot indices; newer versions list cargoes
            // whose industry slots must be looked up.
            let slot = if result.version < 2 {
                if n >= accepted_count(owner) {
                    break;
                }
                Some(n)
            } else {
                let cargo = result.cargo_input[n];
                unsafe { get_cargo_accepted(owner, cargo) }
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
            // Old callback parameters are slot indices; newer versions list cargoes
            // whose industry slots must be looked up.
            let slot = if result.version < 2 {
                if n >= produced_count(owner) {
                    break;
                }
                Some(n)
            } else {
                let cargo = result.cargo_output[n];
                unsafe { get_cargo_produced(owner, cargo) }
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
        // Divide the scaled map size by 31 days. Fractional parts make the leftover
        // days negligible, so a reset for the actual month length is unnecessary.
        (*fields).daily_increment = (1u32 << map_bits) / 31;
        // New games and saves from older versions need the counter initialized.
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

/// Subsidy search reads only the original industry location.
/// SAFETY: Live serialized industry owner, immediate read with no callbacks.
pub(crate) unsafe fn subsidy_position(owner: *const Industry) -> u32 {
    unsafe { (*owner).metadata.tile }
}
/// SAFETY: Live serialized owner; no callback occurs while reading its cargoes.
pub(crate) unsafe fn subsidy_produced_count(owner: *const Industry) -> u32 {
    unsafe { &(*owner).produced }
        .iter()
        .filter(|p| crate::cargo_type::is_valid_cargo_type(p.cargo))
        .count() as u32
}
/// SAFETY: Original random cargo ordinal is less than the valid cargo count.
pub(crate) unsafe fn subsidy_produced(owner: *const Industry, number: u32) -> (u8, u16, u8) {
    let p = unsafe { &(*owner).produced }
        .iter()
        .filter(|p| crate::cargo_type::is_valid_cargo_type(p.cargo))
        .nth(number as usize)
        .unwrap();
    let history = p.history[usize::from(LAST_MONTH)];
    let percent = transported(history) as u8;
    (p.cargo, history.first, percent)
}
/// SAFETY: Live serialized owner; same accepted-slot predicate as `IsCargoAccepted`.
pub(crate) unsafe fn subsidy_accepts(owner: *const Industry, cargo: u8) -> bool {
    unsafe { &(*owner).accepted }
        .iter()
        .any(|a| a.cargo == cargo)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryLocationResult" ["industry"] => crate::industry::LocationResult {
        callback,
        registers,
    };
    "OpenTTDIndustryAllocation" ["industry"] => crate::industry::Allocation {
        shell,
        owner,
        id,
    };
    "OpenTTDIndustryConflictKinds" ["industry"] => crate::industry::ConflictKinds {
        kinds,
    };
    "OpenTTDIndustryConstructionErrors" ["industry"] => crate::industry::ConstructionErrors {
        site,
        forest,
        positioned,
        desert,
        rainforest,
        lowland,
        one_per_town,
        town,
        population,
        near_town,
        conflict,
        too_many,
    };
    "OpenTTDIndustryTransactionServices" ["industry"] => crate::industry::TransactionServices {
        raw_setting,
        multiple_per_town,
        town,
        population,
        town_position,
        industry,

        edge_distance,
        refinery_limit,
        tile_height,
        set_company,
        temporary_shell,
        resolve_location,
        resolve_shape,
        location_error,
        shape_error,
        terragenesis,
        ignore_restrictions,
        platform,
        can_allocate,
        allocate,
        local_company,
        prospect_no_place,
        prospect_unlucky,
    };
    "OpenTTDIndustryTransactionContext" ["industry"] => crate::industry::TransactionContext {
        map,
        builder,
        production,
        generation,
        construction,
        lifecycle,
        tiles,
        tile_policy,
        commands,
        command_checks,
        costs,
        transaction,
        errors,
    };
    "OpenTTDIndustryTilePolicyServices" ["industry"] => crate::industry::TilePolicyServices {
        current_company,
        magic_bulldozer,
        object_in_way,
        close_ai,
        close_game,
        destroy,
        tile_max_z,
        command_checks,
    };
    "OpenTTDIndustryCostServices" ["industry"] => crate::industry::IndustryCostServices {
        zero,
        error,
        expense_money,
        success,
    };
    "OpenTTDIndustryCommandServices" ["industry"] => crate::industry::IndustryCommandServices {
        deity,
        get,
        owner,
        valid_company,
        invalidate_view,
    };
    "OpenTTDIndustryTileAcceptance" ["industry"] => crate::industry::TileAcceptance {
        cargo,
        amount,
        accepts_all,
        accepts_callback,
        amount_callback,
    };
    "OpenTTDIndustryTileOwner" ["industry"] => crate::industry::TileIndustry {
        shell,
        owner,
    };
    "OpenTTDIndustryTileServices" ["industry"] => crate::industry::TileServices {
        shared,
        gfx,
        set_gfx,
        frame,
        set_frame,
        animation_loop,
        set_animation_loop,
        construction_counter,
        construction_stage,
        completed,
        set_completed,
        reset_construction,
        industry,
        industry_index,
        toffee_offset,
        on_water,
        transport,
        build_oilrig,
        pixel_max_z,
        callback,
        };
    "OpenTTDIndustryPair" ["industry"] => Pair { first, second, };
    "Industry::ProducedHistory" ["industry"] => Pair { first => production, second => transported, };
    "Industry::AcceptedHistory" ["industry"] => Pair { first => accepted, second => waiting, };
    "OpenTTDIndustryCargoLists" ["industry"] => crate::industry::CargoLists {
        produced,
        accepted,
    };
    "OpenTTDIndustryLayoutTile" ["industry"] => crate::industry::LayoutTile {
        x,
        y,
        gfx,
    };
    "OpenTTDIndustryConstructionRequest" ["industry"] => crate::industry::ConstructionRequest {
        layout,
        town,
        layout_index,
        tile,
        size_x,
        size_y,
        id,
        random_bits,
        kind => type,
        founder,
    };
    "OpenTTDIndustryConstructionServices" ["industry"] => crate::industry::ConstructionServices {
        set_town,
        generating,
        callback,
        callback_error,
        water_class,
        clear_tile,
        make_industry,
        set_construction_counter,
        set_construction_stage,
        translated_gfx,
        build_window_dirty,
        serve_neutral,
    };
    "OpenTTDIndustryGenerationInfo" ["industry"] => crate::industry::GenerationInfo {
        appear,
        enabled,
        layouts,
    };
    "OpenTTDIndustryGenerationServices" ["industry"] => crate::industry::GenerationServices {
        environment,
        density,
        custom_number,
        scale_by_size_1d,
        scale_by_land_proportion,
        set_progress,
        increase_progress,
        missing_warning,
    };
    "OpenTTDIndustryMetadata" ["industry"] => Metadata {
        tile,
        width,
        height,
        construction_date,
        random,
        kind => type,
        owner,
        random_colour,
        founder,
        construction_type,
        selected_layout,
        exclusive_supplier,
        exclusive_consumer,
    };
    "OpenTTDIndustryCleanupTile" ["industry"] => crate::industry::CleanupTile {
        industry,
        kind => type,
        oilrig,
    };
    "OpenTTDIndustryReseed" ["industry"] => crate::industry::Reseed {
        own => self,
        parent,
        used,
    };
    "OpenTTDIndustryRandomisationSpec" ["industry"] => crate::industry::RandomisationSpec {
        gfx,
        enabled,
    };
    "OpenTTDIndustryAnimationSpec" ["industry"] => crate::industry::AnimationSpec {
        spec,
        triggers,
    };
    "OpenTTDIndustryLifecycleServices" ["industry"] => crate::industry::LifecycleServices {
        neutral_station,
        cleanup_tile,
        delete_tile_inspect,
        restoration,
        delete_oilrig,
        clear_docking,
        industry_field,
        disassociate_field,
        release_disasters,
        delete_storage,
        delete_news,
        close_view,
        close_production,
        delete_inspect,
        subsidies,
        invalidate_cargo,
        belongs,
        randomisation_spec,
        random_triggers,
        set_random_triggers,
        resolve_reseed,
        random_bits,
        set_random_bits,
        animation_spec,

    };
    "OpenTTDIndustryFields" ["industry"] => Fields {
        valid_history,
        last_prod_year,
        counter,
        prod_level,
        was_cargo_delivered,
        ctlflags,
    };
    "OpenTTDIndustryBuildFields" ["industry"] => BuildFields {
        probability,
        min_number,
        target_count,
        max_wait,
        wait_count,
    };
    "OpenTTDIndustryBuilderFields" ["industry"] => BuilderFields {
        builddata,
        wanted_inds,
        daily_counter,
        daily_increment,
        sound_tile,
        sound_ctr,
    };
    "OpenTTDIndustrySlots" ["industry"] => Slots {
        data,
        size,
    };
    "Industry::ProducedCargo" ["industry"] => Produced {
        cargo,
        waiting,
        rate,
        history,
    };
    "Industry::AcceptedCargo" ["industry"] => Accepted {
        cargo,
        waiting,
        accumulated_waiting,
        last_accepted,
        history,
    };
    "OpenTTDIndustryServices" ["industry"] => Services {
        shared,
        next_tick,
        location,
        industry_sound,
        play_sound,
        special_effect,
        tick_trigger,
        production_callback,
        scale_cargo,
        environment,
        farm_tile,

        make_field,
        fence_wanted,
        tile_completed,
        harvest,
        change_callback,
        text,
        news,
        production_news,
        callback_error,
        set_dirty,
        destroy,
        opening_news,
        opening_events,
        create,
        num_items,
        get,
        type_count,
        resolver,
        scale_by_map_size,
        company_none,
        restore_company,
        directory_dirty,
        economy_month,
        economy_year,
        days_since_last_month,
        landscape,
        economy_type,
        fund_only,
        deity,
    };
    "OpenTTDIndustryProductionStorage" ["industry"] => ProductionStorage { storage, };
    "OpenTTDIndustryDeliveryServices" ["industry"] => DeliveryServices { construct, destroy, resolve, };
    "OpenTTDIndustryProductionResult" ["industry"] => ProductionResult {
        subtract,
        add,
        again,
        cargo_input,
        cargo_output,
        version,
        num_input,
        num_output,
        present,
    };
    "OpenTTDIndustryMap" ["industry"] => Map {
        size_x,
        size_y,
        landscape,
    };
    "OpenTTDIndustryEntry" ["industry"] => Entry {
        industry,
        owner,
        id,
    };
    "OpenTTDIndustryLocation" ["industry"] => TileArea {
        tile,
        w => width,
        h => height,
    };
    "OpenTTDIndustryFarmTile" ["industry"] => FarmTile {
        kind => type,
        snow,
        ground,
        grown,
    };
    "OpenTTDIndustryChange" ["industry"] => Change {
        result,
        reg,
    };
}

#[cfg(test)]
mod delivery_tests {
    use super::*;

    #[test]
    fn delivery_uses_first_valid_slot_and_preserves_history() {
        let mut industry = Industry {
            accepted: vec![
                Accepted {
                    cargo: 7,
                    waiting: 65530,
                    ..Accepted::default()
                },
                Accepted {
                    cargo: 7,
                    waiting: 42,
                    ..Accepted::default()
                },
                Accepted {
                    cargo: 255,
                    ..Accepted::default()
                },
            ],
            ..Industry::default()
        };
        let owner = &raw mut industry;
        unsafe {
            assert_eq!(get_cargo_accepted(owner, 7), Some(0));
            assert_eq!(get_cargo_accepted(owner, 255), None);
            assert_eq!(get_cargo_accepted(owner, 8), None);
            // Saturated stockpiles still create history and update last_accepted.
            for (pieces, amount, waiting, date) in [(20, 5, 65535, -12), (20, 0, 65535, 73)] {
                assert_eq!(accept_delivery(owner, 0, pieces, || date), amount);
                assert_eq!(industry.accepted[0].waiting, waiting);
                assert_eq!(industry.accepted[0].last_accepted, date);
                assert_eq!(
                    (*industry.accepted[0].history)[usize::from(THIS_MONTH)].first,
                    5
                );
                assert_eq!(industry.accepted[1].waiting, 42);
                assert!(industry.accepted[1].history.is_null());
            }
            industry.accepted[0].waiting = 0;
            (*industry.accepted[0].history)[usize::from(THIS_MONTH)].first = 65530;
            let history = industry.accepted[0].history;
            assert_eq!(accept_delivery(owner, 0, 10, || 99), 10);
            assert_eq!(industry.accepted[0].history, history);
            assert_eq!((*history)[usize::from(THIS_MONTH)].first, 4);
            assert_eq!((*history)[usize::from(LAST_MONTH)].first, 0);
        }
    }
}

/// Native `industry.h::IndustryControlFlag`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryControlFlag(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryControlFlag {
    /// Ignore rolls to decrease when evaluating industry production changes.
    pub const NoProductionDecrease: Self = Self(0);
    /// Ignore rolls to increase when evaluating industry production changes.
    pub const NoProductionIncrease: Self = Self(1);
    /// Prevent closure for production level or time since delivery, except an
    /// already announced closure.
    pub const NoClosure: Self = Self(2);
    /// Production level is externally controlled.
    pub const ExternalProdLevel: Self = Self(3);
    /// Native `IndustryControlFlag::End`.
    pub const End: Self = Self(4);
}

/// Native `IndustryControlFlags`; elements are `IndustryControlFlag` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryControlFlags(pub u8);
impl IndustryControlFlags {
    /// Native valid control bits.
    pub const MASK: u8 = (1 << IndustryControlFlag::End.0) - 1;
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native bit set containing one element.
    #[must_use]
    pub const fn from_element(element: IndustryControlFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: IndustryControlFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native `BaseBitSet::IsValid`: no bit outside the defined flags is set.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 & !Self::MASK == 0
    }
}

/// Consecutive economy years without production before a processing industry may close.
pub const PROCESSING_INDUSTRY_ABANDONMENT_YEARS: crate::timer_game_economy::EconomyYear =
    crate::timer_game_economy::EconomyYear(5);
/// Production level indicating that the industry will actually close.
pub const PRODLEVEL_CLOSURE: u8 = 0;
/// Below this production level, the industry is set to be closing.
pub const PRODLEVEL_MINIMUM: u8 = 4;
/// Default level of a newly created industry.
pub const PRODLEVEL_DEFAULT: u8 = 16;
/// Industry running at full speed.
pub const PRODLEVEL_MAXIMUM: u8 = 128;

/// Native industry-directory invalidation reasons from `industry.h`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryDirectoryInvalidateWindowData(pub u8);
impl IndustryDirectoryInvalidateWindowData {
    /// Rebuild the directory.
    pub const IDIWD_FORCE_REBUILD: Self = Self(0);
    /// Update for a production change.
    pub const IDIWD_PRODUCTION_CHANGE: Self = Self(1);
    /// Resort the directory.
    pub const IDIWD_FORCE_RESORT: Self = Self(2);
}

#[cfg(test)]
mod production_policy_tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static RNG: Cell<u32> = const { Cell::new(0) };
        static ERRORS: Cell<u32> = const { Cell::new(0) };
        static DIRTY: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn random(_: *mut c_void) -> u32 {
        let value = RNG.get();
        RNG.set(value + 1);
        value
    }
    extern "C" fn unused_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        unreachable!()
    }
    extern "C" fn error(_: *mut c_void, _: bool) {
        ERRORS.set(ERRORS.get() + 1);
    }
    extern "C" fn dirty(_: *mut c_void) {
        DIRTY.set(DIRTY.get() + 1);
    }
    extern "C" fn unexpected_next_tick(_: u32) -> Entry {
        unreachable!("unexpected Services::next_tick")
    }
    extern "C" fn unexpected_location(_: *mut c_void) -> TileArea {
        unreachable!("unexpected Services::location")
    }
    extern "C" fn unexpected_industry_sound(_: *mut c_void, _: u32) {
        unreachable!("unexpected Services::industry_sound")
    }
    extern "C" fn unexpected_play_sound(_: u16, _: u32) {
        unreachable!("unexpected Services::play_sound")
    }
    extern "C" fn unexpected_special_effect(_: *mut c_void, _: u32, _: u32) -> u16 {
        unreachable!("unexpected Services::special_effect")
    }
    extern "C" fn unexpected_tick_trigger(_: *mut c_void) {
        unreachable!("unexpected Services::tick_trigger")
    }
    extern "C" fn unexpected_production_callback(_: *mut c_void, _: u8) {
        unreachable!("unexpected Services::production_callback")
    }
    extern "C" fn unexpected_scale_cargo(_: u32) -> u32 {
        unreachable!("unexpected Services::scale_cargo")
    }
    extern "C" fn unexpected_farm_tile(_: u32) -> FarmTile {
        unreachable!("unexpected Services::farm_tile")
    }
    extern "C" fn unexpected_make_field(_: u32, _: u8, _: u8, _: u16) {
        unreachable!("unexpected Services::make_field")
    }
    extern "C" fn unexpected_fence_wanted(_: u32, _: u8) -> bool {
        unreachable!("unexpected Services::fence_wanted")
    }
    extern "C" fn unexpected_tile_completed(_: *mut c_void, _: u32) -> bool {
        unreachable!("unexpected Services::tile_completed")
    }
    extern "C" fn unexpected_harvest(_: u32) {
        unreachable!("unexpected Services::harvest")
    }
    extern "C" fn unexpected_change_callback(_: *mut c_void, _: bool, _: u32) -> Change {
        unreachable!("unexpected Services::change_callback")
    }
    extern "C" fn unexpected_news(_: *mut c_void, _: u32, _: bool, _: Servicing) {
        unreachable!("unexpected Services::news")
    }
    extern "C" fn unexpected_production_news(_: *mut c_void, _: u8, _: i32, _: Servicing) {
        unreachable!("unexpected Services::production_news")
    }
    extern "C" fn unexpected_destroy(_: *mut c_void) {
        unreachable!("unexpected Services::destroy")
    }
    extern "C" fn unexpected_opening_news(_: *mut c_void) {
        unreachable!("unexpected Services::opening_news")
    }
    extern "C" fn unexpected_opening_events(_: *mut c_void) {
        unreachable!("unexpected Services::opening_events")
    }
    extern "C" fn unexpected_create(_: u8, _: u8) -> *mut c_void {
        unreachable!("unexpected Services::create")
    }
    extern "C" fn unexpected_num_items() -> u32 {
        unreachable!("unexpected Services::num_items")
    }
    extern "C" fn unexpected_get(_: u16) -> *mut c_void {
        unreachable!("unexpected Services::get")
    }
    extern "C" fn unexpected_type_count(_: u8) -> u16 {
        unreachable!("unexpected Services::type_count")
    }
    extern "C" fn unexpected_scale_by_map_size(_: u32) -> u32 {
        unreachable!("unexpected Services::scale_by_map_size")
    }
    extern "C" fn unexpected_company_none() -> u8 {
        unreachable!("unexpected Services::company_none")
    }
    extern "C" fn unexpected_restore_company(_: u8) {
        unreachable!("unexpected Services::restore_company")
    }
    extern "C" fn unexpected_directory_dirty(_: u8) {
        unreachable!("unexpected Services::directory_dirty")
    }
    extern "C" fn unexpected_economy_month() -> u8 {
        unreachable!("unexpected Services::economy_month")
    }
    extern "C" fn unexpected_economy_year() -> i32 {
        unreachable!("unexpected Services::economy_year")
    }
    extern "C" fn unexpected_days_since_last_month() -> u32 {
        unreachable!("unexpected Services::days_since_last_month")
    }
    extern "C" fn unexpected_landscape() -> u8 {
        unreachable!("unexpected Services::landscape")
    }
    extern "C" fn unexpected_economy_type() -> u8 {
        unreachable!("unexpected Services::economy_type")
    }
    extern "C" fn unexpected_fund_only() -> bool {
        unreachable!("unexpected Services::fund_only")
    }
    extern "C" fn unexpected_deity() -> bool {
        unreachable!("unexpected Services::deity")
    }
    fn services(shared: &crate::services::Services) -> Services {
        Services {
            shared,
            next_tick: unexpected_next_tick,
            location: unexpected_location,
            industry_sound: unexpected_industry_sound,
            play_sound: unexpected_play_sound,
            special_effect: unexpected_special_effect,
            tick_trigger: unexpected_tick_trigger,
            production_callback: unexpected_production_callback,
            scale_cargo: unexpected_scale_cargo,
            environment: ptr::null(),
            farm_tile: unexpected_farm_tile,
            make_field: unexpected_make_field,
            fence_wanted: unexpected_fence_wanted,
            tile_completed: unexpected_tile_completed,
            harvest: unexpected_harvest,
            change_callback: unexpected_change_callback,
            text: ptr::null(),
            news: unexpected_news,
            production_news: unexpected_production_news,
            callback_error: error,
            set_dirty: dirty,
            destroy: unexpected_destroy,
            opening_news: unexpected_opening_news,
            opening_events: unexpected_opening_events,
            create: unexpected_create,
            num_items: unexpected_num_items,
            get: unexpected_get,
            type_count: unexpected_type_count,
            resolver: ptr::null(),
            scale_by_map_size: unexpected_scale_by_map_size,
            company_none: unexpected_company_none,
            restore_company: unexpected_restore_company,
            directory_dirty: unexpected_directory_dirty,
            economy_month: unexpected_economy_month,
            economy_year: unexpected_economy_year,
            days_since_last_month: unexpected_days_since_last_month,
            landscape: unexpected_landscape,
            economy_type: unexpected_economy_type,
            fund_only: unexpected_fund_only,
            deity: unexpected_deity,
        }
    }

    // A run stores count, first pair, and per-pair uint32 deltas. Every pair is
    // expanded and checked; this is lossless compression, not a digest/sample.
    fn parameters(tokens: &[&str]) -> Vec<(u32, u32)> {
        let mut pairs = Vec::new();
        for token in tokens {
            let fields: Vec<u32> = token.split(':').map(|v| v.parse().unwrap()).collect();
            assert_eq!(fields.len(), 5);
            assert!(fields[0] != 0);
            for n in 0..fields[0] {
                pairs.push((
                    fields[1].wrapping_add(n.wrapping_mul(fields[3])),
                    fields[2].wrapping_add(n.wrapping_mul(fields[4])),
                ));
            }
        }
        pairs
    }
    fn resolve_result(
        out: &mut ProductionResult,
        version: u8,
        amount: i32,
        cargo: u8,
        limit: usize,
        calls: usize,
    ) {
        out.present = u8::from(calls <= limit);
        if out.present == 0 {
            return;
        }
        out.version = version;
        if version == 255 {
            return;
        }
        out.num_input = 3;
        out.num_output = 3;
        for n in 0..3 {
            out.subtract[n] = if version >= 1 { amount } else { n as i32 };
            out.add[n] = if version >= 1 { amount } else { n as i32 + 3 };
        }
        out.cargo_input[..3].copy_from_slice(&[cargo, 255, 7]);
        out.cargo_output[..3].copy_from_slice(&[cargo, 255, 9]);
        out.again = if version == 0 {
            6
        } else if limit == 1 {
            0
        } else {
            0x123
        };
    }
    #[test]
    fn production_callback_matches_complete_original_corpus() {
        let shared = crate::services::fixture(ptr::null_mut(), random, unused_industry);
        let services = services(&shared);
        let mut oracle = include_str!("industry/production-oracle.txt")
            .lines()
            .filter(|line| line.starts_with("I "));
        // Mixed-radix enumeration preserves the complete 4 x 5 x 4 x 4 x 2 x 7
        // input nest, including its exact original order.
        for checked in 0..4480 {
            let version = [0, 1, 2, 255][checked / 1120];
            let level = [0, 4, 16, 128, 255][(checked / 224) % 5];
            let behaviour = [0, 1 << 14, 1 << 15, 3 << 14][(checked / 56) % 4];
            let limit = [0, 1, 3, 65536][(checked / 14) % 4];
            let reason = [0, 1][(checked / 7) % 2];
            let amount = [i32::MIN, -65536, -1, 0, 1, 65536, i32::MAX][checked % 7];
            let line = oracle.next().expect("missing Industry policy case");
            let tokens: Vec<_> = line.split_whitespace().collect();
            let input = format!("I {version} {level} {behaviour} {limit} {reason} {amount} ");
            assert!(
                line.starts_with(&input),
                "case {checked}: dimensions changed"
            );
            let expected: Vec<u32> = tokens[7..17].iter().map(|v| v.parse().unwrap()).collect();
            let pairs = parameters(&tokens[17..]);
            assert_eq!(pairs.len(), expected[9] as usize);
            let cargo = if amount == i32::MAX {
                254
            } else if amount == i32::MIN {
                64
            } else {
                2
            };
            let mut industry = Industry::default();
            industry.fields.prod_level = level;
            for (cargo, waiting) in [(cargo, 65535), (255, 15), (3, 1)] {
                industry.accepted.push(Accepted {
                    cargo,
                    waiting,
                    ..Accepted::default()
                });
            }
            for (cargo, waiting) in [(cargo, 65535), (255, 15), (8, 1)] {
                industry.produced.push(Produced {
                    cargo,
                    waiting,
                    ..Produced::default()
                });
            }
            RNG.set(0x1234_5678);
            ERRORS.set(0);
            DIRTY.set(0);
            let mut calls = 0;
            production_callback(
                ptr::null_mut(),
                &raw mut industry,
                behaviour,
                reason,
                &services,
                |random, parameter, out| {
                    assert_eq!(
                        pairs.get(calls),
                        Some(&(random, parameter)),
                        "case {checked} callback {calls}"
                    );
                    calls += 1;
                    // SAFETY: Production supplies one initialized result for this synchronous resolver.
                    let out = unsafe { &mut *out };
                    resolve_result(out, version, amount, cargo, limit, calls);
                },
            );
            assert_eq!(calls, pairs.len(), "case {checked}: truncated callbacks");
            let actual = [
                RNG.get(),
                ERRORS.get(),
                DIRTY.get(),
                u32::from(industry.accepted[0].waiting),
                u32::from(industry.accepted[1].waiting),
                u32::from(industry.accepted[2].waiting),
                u32::from(industry.produced[0].waiting),
                u32::from(industry.produced[1].waiting),
                u32::from(industry.produced[2].waiting),
            ];
            assert_eq!(actual, expected[..9], "case {checked}: state/effects");
        }
        assert!(
            oracle.next().is_none(),
            "unexpected extra Industry policy case"
        );
    }
}
