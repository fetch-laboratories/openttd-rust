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
#![allow(unsafe_code)]
use crate::bitmath_func::{gb, has_bit, sb};
use crate::cargo_type::{CargoType, is_valid_cargo_type};
use crate::company_type::{CompanyID, Owner};
use crate::economy_type::EconomyType;
use crate::history::{
    get_and_reset_accumulated_average, rotate_history_year, update_valid_history_year,
};
use crate::history_type::{LAST_MONTH, THIS_MONTH};
use crate::industry_type::{
    IndustryAnimationTrigger, IndustryID, IndustryRandomTrigger, IndustryType,
};
use crate::industrytype::{
    INDUSTRYLIFE_BLACK_HOLE, IndustryBehaviour, IndustryBehaviours, IndustryLifeType,
    IndustryLifeTypes, IndustrySpec, get_industry_spec, spec_behaviour, spec_callback_mask,
};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CallbackID, IndustryCallbackMask, IndustryCallbackMasks,
};
use crate::newgrf_industries::IndustryAvailabilityCallType;
use crate::random_func::chance16_i;
use crate::sound_type::SoundFx;
use crate::strings_type::StringID;
use crate::tile_type::{TileIndex, TileType};
use crate::tilearea::TileArea;
use crate::timer_game_tick::{DAY_TICKS, INDUSTRY_CUT_TREE_TICKS, INDUSTRY_PRODUCE_TICKS};
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
const RECORDS: usize = crate::history_type::HISTORY_RECORDS;
const TYPES: usize = IndustryType::NUM_INDUSTRYTYPES.0 as usize;
/// `Industry::ProducedHistory` (`first` is `production`, `second` is `transported`) and
/// `Industry::AcceptedHistory` (`first` is `accepted`, `second` is the average `waiting`).
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Pair {
    pub first: u16,
    pub second: u16,
}
/// `Industry::ProducedCargo`.
#[repr(C)]
#[derive(Clone)]
pub struct Produced {
    /// Cargo type
    pub cargo: u8,
    /// Amount of cargo produced
    pub waiting: u16,
    /// Production rate
    pub rate: u8,
    /// History of cargo produced and transported for this month and 24 previous months
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
/// `Industry::AcceptedCargo`.
#[repr(C)]
#[allow(
    clippy::struct_field_names,
    reason = "Industry::AcceptedCargo keeps the original's field names"
)]
pub struct Accepted {
    /// Cargo type
    pub cargo: u8,
    /// Amount of cargo waiting to processed
    pub waiting: u16,
    /// Accumulated waiting total over the last month, used to calculate average.
    pub accumulated_waiting: u32,
    /// Last day cargo was accepted by this industry
    pub last_accepted: i32,
    /// History of accepted and waiting cargo, created on first use.
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
/// The production and statistics scalars of `Industry`.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Fields {
    /// Mask of valid history records.
    pub valid_history: u64,
    /// last economy year of production
    pub last_prod_year: i32,
    /// used for animation and/or production (if available cargo)
    pub counter: u16,
    /// general production level
    pub prod_level: u8,
    /// flag that indicate this has been the closest industry chosen for cargo delivery by a
    /// station. see `DeliverGoodsToIndustry`
    pub was_cargo_delivered: u8,
    /// flags overriding standard behaviours
    pub ctlflags: u8,
}
/// The identity, location and construction scalars of `Industry`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Metadata {
    /// `location.tile`: location of the industry
    pub tile: u32,
    /// `location.w`
    pub width: u16,
    /// `location.h`
    pub height: u16,
    /// Date of the construction of the industry
    pub construction_date: i32,
    /// Random value used for randomisation of all kinds of things
    pub random: u16,
    /// type of industry.
    pub kind: u8,
    /// owner of the industry.  Which SHOULD always be (imho) `OWNER_NONE`
    pub owner: u8,
    /// randomized colour of the industry, for display purpose
    pub random_colour: u8,
    /// Founder of the industry
    pub founder: u8,
    /// Way the industry was constructed (@see `IndustryConstructionType`)
    pub construction_type: u8,
    /// Which tile layout was used when creating the industry
    pub selected_layout: u8,
    /// Which company has exclusive rights to deliver cargo (`INVALID_OWNER` = anyone)
    pub exclusive_supplier: u8,
    /// Which company has exclusive rights to take cargo (`INVALID_OWNER` = anyone)
    pub exclusive_consumer: u8,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            tile: TileIndex::INVALID.0,
            width: 0,
            height: 0,
            construction_date: 0,
            random: 0,
            kind: 0,
            owner: Owner::INVALID.0,
            random_colour: crate::gfx_type::Colours::COLOUR_BEGIN.0,
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
    (0..accepted_count(owner))
        .any(|slot| is_valid_cargo_type(unsafe { (*accepted(owner, slot)).cargo }))
}
/// `for (const auto &p : i->produced)`: visit each produced slot's cargo in slot order.
pub(crate) fn for_each_cargo_produced(owner: *mut Industry, mut visit: impl FnMut(CargoType)) {
    for slot in 0..produced_count(owner) {
        // SAFETY: Live owner; immediate copy before the visit.
        visit(CargoType(unsafe { (*produced(owner, slot)).cargo }));
    }
}
/// Temporary raw adapter for [`for_each_cargo_produced`], used by `station.rs` and
/// `station_lifecycle.rs`.
pub(crate) fn for_each_produced_cargo(owner: *mut Industry, mut visit: impl FnMut(u8)) {
    for_each_cargo_produced(owner, |cargo| visit(cargo.0));
}
/// `IndustryTypeBuildData`.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct BuildFields {
    /// Relative probability of building this industry.
    pub probability: u32,
    /// Smallest number of industries that should exist (either 0 or 1).
    pub min_number: u8,
    /// Desired number of industries of this type.
    pub target_count: u16,
    /// Starting number of turns to wait (copied to `wait_count`).
    pub max_wait: u16,
    /// Number of turns to wait before trying to build again.
    pub wait_count: u16,
}
/// `IndustryBuildData` (data for managing the number and type of industries in the game),
/// with the industry fields of `_economy` and the lumber mill sound state.
#[repr(C)]
pub struct BuilderFields {
    /// Industry build data for every industry type.
    pub builddata: [BuildFields; TYPES],
    /// Number of wanted industries (bits 31-16), and a fraction (bits 15-0).
    pub wanted_inds: u32,
    /// `_economy.industry_daily_change_counter`: bits 31-16 are number of industry to be
    /// performed, 15-0 are fractional collected daily
    pub daily_counter: u32,
    /// `_economy.industry_daily_increment`: the value which will increment
    /// `industry_daily_change_counter`. Computed value. NOSAVE
    pub daily_increment: u32,
    /// `_industry_sound_tile`
    pub sound_tile: u32,
    /// `_industry_sound_ctr`
    pub sound_ctr: u8,
}
pub struct Builder {
    fields: BuilderFields,
    /// `Industry::industries`: the industry IDs of each type, in ascending order.
    registry: [Vec<IndustryID>; TYPES],
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
/// `AcceptedCargo::GetOrCreateHistory`: get history data, creating it if necessary.
///
/// SAFETY: Live accepted slot with canonical layout. Existing history remains stable.
unsafe fn get_or_create_history(accepted: *mut Accepted) -> *mut [Pair; RECORDS] {
    unsafe {
        if (*accepted).history.is_null() {
            (*accepted).history = Box::into_raw(Box::new([Pair::default(); RECORDS]));
        }
        (*accepted).history
    }
}
/// # Safety
/// Live accepted slot with canonical layout. Existing history remains stable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_history(accepted: *mut Accepted) -> *mut c_void {
    unsafe { get_or_create_history(accepted).cast() }
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
    pub behaviour: IndustryBehaviours,
    pub callbacks: IndustryCallbackMasks,
    pub life: IndustryLifeTypes,
    pub kind: IndustryType,
    pub up_text: StringID,
    pub down_text: StringID,
    pub closure_text: StringID,
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
    /// `_settings_game.game_creation.landscape`, through the service.
    pub(crate) fn landscape_type(&self) -> LandscapeType {
        LandscapeType(unsafe { (self.landscape)() })
    }
}
impl Map {
    fn coordinates(self) -> crate::services::Map {
        crate::services::Map::from_log_x(self.size_x.trailing_zeros())
    }
}
use crate::clear_map::ClearGround;
use crate::direction_func::{diag_dir_to_axis, other_axis};
use crate::direction_type::DiagDirection;
use crate::landscape_type::LandscapeType;
use crate::tree_map::TreeGround;
/// `Industry::type`.
fn industry_type(owner: *const Industry) -> IndustryType {
    IndustryType(unsafe { (*owner).metadata.kind })
}
/// `GetIndustrySpec(i->type)` through the owner's type.
fn spec_of(owner: *mut Industry) -> *const IndustrySpec {
    get_industry_spec(industry_type(owner))
}
/// `Industry::ctlflags`.
fn ctlflags(f: *const Fields) -> IndustryControlFlags {
    IndustryControlFlags(unsafe { (*f).ctlflags })
}
fn observe(owner: *mut Industry) -> Observation {
    let spec = spec_of(owner);
    unsafe {
        Observation {
            owner,
            behaviour: IndustryBehaviours((*spec).behaviour),
            callbacks: IndustryCallbackMasks((*spec).callback_mask),
            life: IndustryLifeTypes((*spec).life_type),
            kind: industry_type(owner),
            up_text: StringID((*spec).production_up_text),
            down_text: StringID((*spec).production_down_text),
            closure_text: StringID((*spec).closure_text),
        }
    }
}
/// `Random()`.
fn random(s: &Services) -> u32 {
    s.shared().random()
}
/// `RandomRange(limit)`.
fn random_range(s: &Services, limit: u32) -> u32 {
    s.shared().random_range(limit)
}
/// `Chance16(a, b)`.
fn chance16(s: &Services, a: u32, b: u32) -> bool {
    chance16_i(a, b, random(s))
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
unsafe fn get_cargo_produced(owner: *const Industry, cargo: CargoType) -> Option<usize> {
    if !is_valid_cargo_type(cargo.0) {
        return None;
    }
    unsafe { &(*owner).produced }
        .iter()
        .position(|p| p.cargo == cargo.0)
}
/// `Industry::GetCargoAccepted`, with the same validity and ordering contract.
pub(crate) unsafe fn get_cargo_accepted(owner: *const Industry, cargo: CargoType) -> Option<usize> {
    if !is_valid_cargo_type(cargo.0) {
        return None;
    }
    unsafe { &(*owner).accepted }
        .iter()
        .position(|a| a.cargo == cargo.0)
}
/// `Industry::exclusive_supplier`.
///
/// SAFETY: Live serialized industry owner; observe after the refusal callback.
pub(crate) unsafe fn exclusive_supplier(owner: *const Industry) -> Owner {
    unsafe { CompanyID((*owner).metadata.exclusive_supplier) }
}
/// The accepted-cargo bookkeeping of `DeliverGoodsToIndustry`: add up to the room left in
/// the slot, count it as accepted this month, and record the delivery date.
///
/// SAFETY: Live accepted slot selected by `GetCargoAccepted`; the date leaf cannot resize slots.
#[allow(
    clippy::cast_possible_truncation,
    reason = "DeliverGoodsToIndustry keeps the original's implicit C++ integer conversions"
)]
pub(crate) unsafe fn accept_delivery(
    owner: *mut Industry,
    slot: usize,
    pieces: u32,
    date: impl FnOnce() -> crate::timer_game_economy::EconomyDate,
) -> u32 {
    let a = accepted(owner, slot);
    let amount = pieces.min(u32::from(u16::MAX - unsafe { (*a).waiting }));
    unsafe {
        (*a).waiting = (*a).waiting.wrapping_add(amount as u16);
        let history = get_or_create_history(a);
        let count = &mut (*history)[usize::from(THIS_MONTH)].first;
        *count = count.wrapping_add(amount as u16);
    }
    let now = date();
    unsafe { (*a).last_accepted = now.0 };
    amount
}
impl Pair {
    /// `Industry::ProducedHistory::PctTransported`: percentage of cargo transported, as a
    /// fraction of 255.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Industry::ProducedHistory::PctTransported keeps the original's implicit C++ integer conversions"
    )]
    fn pct_transported(self) -> u8 {
        let p = self;
        if p.first == 0 {
            0
        } else {
            (u32::from(p.second) * 256 / u32::from(p.first)).min(u32::from(u8::MAX)) as u8
        }
    }
}
/// Helper for `ProduceIndustryGoods` that scales and produces cargos.
/// `scale` tells whether to scale production of this cargo directly.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ProduceIndustryGoodsHelper keeps the original's implicit C++ integer conversions"
)]
fn produce_industry_goods_helper(owner: *mut Industry, s: &Services, scale: bool) {
    for n in 0..produced_count(owner) {
        let p = produced(owner, n);
        unsafe {
            if !is_valid_cargo_type((*p).cargo) {
                continue;
            }
        }
        let mut amount = unsafe { u16::from((*p).rate) };
        if scale {
            amount = unsafe { (s.scale_cargo)(u32::from(amount)) } as u16;
        }
        unsafe {
            (*p).waiting =
                (u32::from((*p).waiting) + u32::from(amount)).min(u32::from(u16::MAX)) as u16;
        }
    }
}
/// `ProduceIndustryGoods`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ProduceIndustryGoods keeps the original's implicit C++ integer conversions"
)]
fn produce_industry_goods(entry: Entry, svc: &Services, builder: *mut Builder) {
    let (i, owner) = (entry.industry, entry.owner);
    let state = fields(owner);
    // play a sound?
    if gb(u32::from(unsafe { (*state).counter }), 0, 6) == 0 {
        let r = random(svc);
        if chance16_i(1, 14, r) {
            let sounds = unsafe { (*spec_of(owner)).random_sounds.len() } as u16;
            if sounds != 0
                && unsafe { svc.shared().settings().ambient.read() }
                && (0..produced_count(owner)).any(|slot| unsafe {
                    (*produced(owner, slot)).history[usize::from(LAST_MONTH)].first > 0
                })
            {
                // Play sound since last month had production
                unsafe { (svc.industry_sound)(i, ((r >> 16) * u32::from(sounds)) >> 16) };
            }
        }
    }
    unsafe {
        (*state).counter = (*state).counter.wrapping_sub(1);
    }
    let callbacks = spec_callback_mask(spec_of(owner));
    let callback = callbacks.test(IndustryCallbackMask::Production256Ticks);
    // If using an industry callback, scale the callback interval by cargo scale percentage.
    if callback
        && unsafe {
            u32::from((*state).counter)
                % crate::economy_func::scale_by_inverse_cargo_scale(
                    INDUSTRY_PRODUCE_TICKS,
                    svc.shared().settings().industry_cargo_scale.read(),
                )
                == 0
        }
    {
        unsafe { (svc.production_callback)(i, 1) };
        produce_industry_goods_helper(owner, svc, false);
    }
    // All other production and special effects happen every 256 ticks, and cargo
    // production is just scaled by the cargo scale percentage. This keeps a slow trickle of
    // production to avoid confusion at low scale factors when the industry seems to be
    // doing nothing for a long period of time.
    if unsafe { u32::from((*state).counter) % INDUSTRY_PRODUCE_TICKS != 0 } {
        return;
    }
    // Handle non-callback cargo production.
    if !callback {
        produce_industry_goods_helper(owner, svc, true);
    }
    let indbehav = spec_behaviour(spec_of(owner));
    // The two special effects share one shape: CBID_INDUSTRY_SPECIAL_EFFECT with parameter
    // 0 for planting fields and 1 for cutting trees.
    for (flag, parameter) in [
        (IndustryBehaviour::PlantFields, 0),
        (IndustryBehaviour::CutTrees, 1),
    ] {
        if !indbehav.test(flag) {
            continue;
        }
        let cb_res = if callbacks.test(IndustryCallbackMask::SpecialEffect) {
            let random = random(svc);
            unsafe { (svc.special_effect)(i, random, parameter) }
        } else {
            CALLBACK_FAILED
        };
        let act = if cb_res != CALLBACK_FAILED {
            (svc.shared().grf_boolean_callback)(
                unsafe { (*spec_of(owner)).grf_prop.grffile },
                CallbackID::CBID_INDUSTRY_SPECIAL_EFFECT.0,
                cb_res,
            )
        } else if parameter == 0 {
            chance16(svc, 1, 8)
        } else {
            unsafe { u32::from((*state).counter) % INDUSTRY_CUT_TREE_TICKS == 0 }
        };
        if act {
            if parameter == 0 {
                plant_random_farm_field_with_settings(i, IndustryID(entry.id), tick_map(svc), svc);
            } else {
                chop_lumber_mill_trees(i, owner, tick_map(svc), svc, builder);
            }
        }
    }
    // TriggerIndustryRandomisation and TriggerIndustryAnimation for IndustryTick.
    unsafe { (svc.tick_trigger)(i) };
}
/// `OnTick_Industry`.
///
/// # Safety
/// Live builder and synchronous services; C++ supplies serial world
/// access and terminating noexcept environmental failures. No borrowed owner
/// reference survives a service, including recursive production on construction.
pub(crate) unsafe fn on_tick_industry(builder: *mut Builder, services: *const Services) {
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
        produce_industry_goods(e, s, builder);
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
#[allow(
    clippy::cast_possible_truncation,
    reason = "TransportIndustryGoods keeps the original's implicit C++ integer conversions"
)]
pub unsafe extern "C" fn openttd_rust_industry_transport_abi(
    i: *mut c_void,
    owner: *mut Industry,
    minimal_cargo: u8,
    recession: bool,
    services: *const Services,
) -> u8 {
    let s = unsafe { &*services };
    let mut moved_cargo = false;
    let mut catchment = None;
    for n in 0..produced_count(owner) {
        let p = produced(owner, n);
        let mut cw = unsafe { u32::from((*p).waiting).min(u32::from(u8::MAX)) };
        if cw <= u32::from(minimal_cargo) || unsafe { !is_valid_cargo_type((*p).cargo) } {
            continue;
        }
        unsafe {
            (*p).waiting = (*p).waiting.wrapping_sub(cw as u16);
        }
        // fluctuating economy?
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
        moved_cargo |= am != 0;
    }
    u8::from(moved_cargo)
}
/// `Industry::RecomputeProductionMultipliers`: recompute `production_rate` for the current
/// `prod_level`. This function is only valid when not using smooth economy.
#[allow(
    clippy::cast_possible_truncation,
    reason = "Industry::RecomputeProductionMultipliers keeps the original's implicit C++ integer conversions"
)]
fn recompute_production_multipliers(owner: *mut Industry) {
    let f = fields(owner);
    // Rates are rounded up, so e.g. oilrig always produces some passengers
    for n in 0..produced_count(owner) {
        let indspec = spec_of(owner);
        let rate = u32::from(unsafe { (*indspec).production_rate[n] });
        let value = rate * unsafe { u32::from((*f).prod_level) };
        unsafe {
            (*produced(owner, n)).rate = value
                .div_ceil(u32::from(PRODLEVEL_DEFAULT))
                .min(u32::from(u8::MAX)) as u8;
        }
    }
}
/// # Safety
/// Live industry and its owner; the original `UsesOriginalEconomy` precondition applies.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_recompute(owner: *mut Industry) {
    recompute_production_multipliers(owner);
}
/// `UpdateIndustryStatistics`.
fn update_industry_statistics(owner: *mut Industry, s: &Services) {
    let f = fields(owner);
    let month = u32::from(unsafe { (s.economy_month)() });
    unsafe {
        (*f).valid_history = update_valid_history_year((*f).valid_history, month);
    }
    for n in 0..produced_count(owner) {
        let p = produced(owner, n);
        unsafe {
            if !is_valid_cargo_type((*p).cargo) {
                continue;
            }
            if (*p).history[usize::from(THIS_MONTH)].first != 0 {
                (*f).last_prod_year = (s.economy_year)();
            }
            rotate_history_year(&mut (*p).history, (*f).valid_history, month);
        }
    }
    for n in 0..accepted_count(owner) {
        let a = accepted(owner, n);
        unsafe {
            if !is_valid_cargo_type((*a).cargo) || (*a).history.is_null() {
                continue;
            }
            let history = (*a).history;
            (*history)[usize::from(THIS_MONTH)].second = get_and_reset_accumulated_average(
                &mut (*a).accumulated_waiting,
                (s.days_since_last_month)(),
            );
            rotate_history_year(&mut *history, (*f).valid_history, month);
        }
    }
}
/// Protects an industry from closure if the appropriate flags and conditions are met.
/// `CanCloseLastInstance` must be set (which, by default, it is not) and the count of
/// industries of this type must one (or lower) in order to be protected against closure.
/// Returns true if protection is on, false otherwise (except for oil wells).
fn check_industry_close_down_protection(o: Observation, s: &Services) -> bool {
    // oil wells (or the industries with that flag set) are always allowed to closedown
    if o.behaviour.test(IndustryBehaviour::DontIncrProd)
        && LandscapeType(unsafe { (s.landscape)() }) == LandscapeType::Temperate
    {
        return false;
    }
    !o.behaviour.test(IndustryBehaviour::CanCloseLastInstance)
        && unsafe { (s.type_count)(o.kind.0) } <= 1
}
const PERCENT_TRANSPORTED_60: u32 = 153;
const PERCENT_TRANSPORTED_80: u32 = 204;

/// Change industry production or do closure. `monthly` is true for the monthly call and
/// false for the random call.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "ChangeIndustryProduction keeps the original's implicit C++ integer conversions"
)]
#[allow(
    clippy::too_many_lines,
    reason = "ChangeIndustryProduction is ported as the original's single body; its length is tracked in the Clippy debt ledger"
)]
fn change_industry_production(i: *mut c_void, owner: *mut Industry, svc: &Services, monthly: bool) {
    let obs = observe(owner);
    let state = fields(obs.owner);
    let mut str = StringID::STR_NULL;
    let mut closeit = false;
    let mut standard = false;
    let mut suppress_message = false;
    // reinitialize production_rate to match prod_level
    let mut recalculate_multipliers = false;
    // use original economy for industries using production related callbacks
    let original_economy =
        uses_original_economy(obs.callbacks, EconomyType(unsafe { (svc.economy_type)() }));
    let mut div = 0u8;
    let mut mul = 0u8;
    let mut increment = 0i8;
    let callback_enabled = obs.callbacks.test(if monthly {
        IndustryCallbackMask::MonthlyProdChange
    } else {
        IndustryCallbackMask::ProductionChange
    });
    if callback_enabled {
        let random = random(svc);
        let result = unsafe { (svc.change_callback)(i, monthly, random) };
        let mut res = u32::from(result.result);
        // failed callback means "do nothing"
        if res != u32::from(CALLBACK_FAILED) {
            suppress_message = has_bit(res, 7);
            // Get the custom message if any
            if has_bit(res, 8) {
                let grfid = unsafe { (*spec_of(owner)).grf_prop.grfid };
                str = StringID(unsafe {
                    crate::newgrf_text::openttd_rust_grf_text_map(
                        &*svc.text,
                        grfid,
                        gb(result.reg as u32, 0, 16),
                    )
                });
            }
            res = gb(res, 0, 4);
            match res {
                // Do nothing, but show the custom message if any
                0x0 => {}
                // Halve industry production. If production reaches the quarter of the
                // default, the industry is closed instead.
                0x1 => div = 1,
                // Double industry production if it hasn't reached eight times of the
                // original yet.
                0x2 => mul = 1,
                // The industry announces imminent closure, and is physically removed from
                // the map next month.
                0x3 => closeit = true,
                // Do the standard random production change as if this industry was a
                // primary one.
                0x4 => standard = true,
                // Divide production by 4, 8, 16, or 32 for 0x8
                0x5..=0x8 => div = (res - 0x3) as u8,
                // Multiply production by 4, 8, 16, or 32 for 0xC
                0x9..=0xC => mul = (res - 0x7) as u8,
                // decrement production; increment production
                0xD | 0xE => increment = if res == 0x0D { -1 } else { 1 },
                // Set production to third byte of register 0x100
                0xF => {
                    unsafe {
                        (*state).prod_level = gb(result.reg as u32, 16, 8)
                            .clamp(u32::from(PRODLEVEL_MINIMUM), u32::from(PRODLEVEL_MAXIMUM))
                            as u8;
                    }
                    recalculate_multipliers = true;
                }
                _ => unreachable!(),
            }
        }
    } else if monthly == original_economy
        || (!original_economy
            && EconomyType(unsafe { (svc.economy_type)() }) == EconomyType::ET_FROZEN)
        || obs.life == INDUSTRYLIFE_BLACK_HOLE
    {
        return;
    }
    if standard
        || (!callback_enabled
            && obs.life.any(
                IndustryLifeTypes::from_element(IndustryLifeType::Organic).with(
                    IndustryLifeTypes::from_element(IndustryLifeType::Extractive),
                ),
            ))
    {
        // decrease or increase
        let only_decrease = obs.behaviour.test(IndustryBehaviour::DontIncrProd)
            && LandscapeType(unsafe { (svc.landscape)() }) == LandscapeType::Temperate;
        if original_economy {
            if only_decrease || chance16(svc, 1, 3) {
                let pct = u32::from(unsafe {
                    (*get_produced(obs.owner, 0)).history[usize::from(LAST_MONTH)].pct_transported()
                });
                // If more than 60% transported, 66% chance of increase, else 33% chance of
                // increase
                if !only_decrease && (pct > PERCENT_TRANSPORTED_60) != chance16(svc, 1, 3) {
                    // Increase production
                    mul = 1;
                } else {
                    // Decrease production
                    div = 1;
                }
            }
        } else if EconomyType(unsafe { (svc.economy_type)() }) == EconomyType::ET_SMOOTH {
            closeit = !ctlflags(state).any(
                IndustryControlFlags::from_element(IndustryControlFlag::NoClosure).with(
                    IndustryControlFlags::from_element(IndustryControlFlag::NoProductionDecrease),
                ),
            );
            for slot in 0..produced_count(obs.owner) {
                let p = produced(obs.owner, slot);
                if unsafe { !is_valid_cargo_type((*p).cargo) } {
                    continue;
                }
                let r = random(svc);
                let pct =
                    u32::from(unsafe { (*p).history[usize::from(LAST_MONTH)] }.pct_transported());
                // If over 60% is transported, mult is 1, else mult is -1.
                let mut mult = if pct > PERCENT_TRANSPORTED_60 { 1 } else { -1 };
                let old_prod = unsafe { i32::from((*p).rate) };
                let mut new_prod = old_prod;
                // For industries with only_decrease flags (temperate terrain Oil Wells),
                // the multiplier will always be -1 so they will only decrease.
                if only_decrease {
                    mult = -1;
                // For normal industries, if over 60% is transported, 33% chance for decrease.
                // Bonus for very high station ratings (over 80%): 16% chance for decrease.
                } else if chance16_i(1, if pct > PERCENT_TRANSPORTED_80 { 6 } else { 3 }, r) {
                    mult *= -1;
                }
                // 4.5% chance for 3-23% (or 1 unit for very low productions) production
                // change, determined by mult value. If mult = 1 prod. increases, else (-1) it
                // decreases.
                if chance16_i(1, 22, r >> 16) {
                    new_prod = new_prod.wrapping_add(
                        mult * (((random_range(svc, 50) + 10) * old_prod as u32) >> 8).max(1)
                            as i32,
                    );
                }
                // Prevent production to overflow or Oil Rig passengers to be over-"produced"
                new_prod = new_prod.clamp(1, i32::from(u8::MAX));
                if unsafe {
                    (*p).cargo
                        == crate::cargo_registry::openttd_rust_cargo_by_label(
                            crate::cargo_type::CT_PASSENGERS,
                        )
                } && !obs.behaviour.test(IndustryBehaviour::NoPaxProdClamp)
                {
                    new_prod = new_prod.clamp(0, 16);
                }
                // If override flags are set, prevent actually changing production if any was
                // decided on
                if (ctlflags(state).test(IndustryControlFlag::NoProductionDecrease)
                    && new_prod < old_prod)
                    || (ctlflags(state).test(IndustryControlFlag::NoProductionIncrease)
                        && new_prod > old_prod)
                {
                    continue;
                }
                // Do not stop closing the industry when it has the lowest possible production
                // rate
                if new_prod == old_prod && old_prod > 1 {
                    closeit = false;
                    continue;
                }
                let percent = if old_prod == 0 {
                    100
                } else {
                    new_prod * 100 / old_prod - 100
                };
                unsafe {
                    (*p).rate = new_prod as u8;
                }
                // Close the industry when it has the lowest possible production rate
                if new_prod > 1 {
                    closeit = false;
                }
                if percent.abs() >= 10 {
                    // ReportNewsProductionChangeIndustry
                    unsafe {
                        (svc.production_news)(i, (*p).cargo, percent, servicing(i, obs.owner, svc));
                    };
                }
            }
        }
    }
    // If override flags are set, prevent actually changing production if any was decided on
    if (ctlflags(state).test(IndustryControlFlag::NoProductionDecrease)
        && (div > 0 || increment < 0))
        || (ctlflags(state).test(IndustryControlFlag::NoProductionIncrease)
            && (mul > 0 || increment > 0))
    {
        return;
    }
    if ctlflags(state).test(IndustryControlFlag::ExternalProdLevel) {
        div = 0;
        mul = 0;
        increment = 0;
    }
    if !callback_enabled
        && obs.life.test(IndustryLifeType::Processing)
        && unsafe {
            (svc.economy_year)().wrapping_sub((*state).last_prod_year)
                >= PROCESSING_INDUSTRY_ABANDONMENT_YEARS.0
        }
        && chance16(svc, 1, if original_economy { 2 } else { 180 })
    {
        closeit = true;
    }
    // Increase if needed
    while mul != 0 && unsafe { (*state).prod_level < PRODLEVEL_MAXIMUM } {
        mul = mul.wrapping_sub(1);
        unsafe {
            (*state).prod_level =
                (u16::from((*state).prod_level) * 2).min(u16::from(PRODLEVEL_MAXIMUM)) as u8;
        }
        recalculate_multipliers = true;
        if str == StringID::STR_NULL {
            str = obs.up_text;
        }
    }
    // Decrease if needed
    while div != 0 && !closeit {
        div = div.wrapping_sub(1);
        if unsafe { (*state).prod_level == PRODLEVEL_MINIMUM } {
            closeit = true;
            break;
        }
        unsafe {
            (*state).prod_level = ((*state).prod_level / 2).max(PRODLEVEL_MINIMUM);
        }
        recalculate_multipliers = true;
        if str == StringID::STR_NULL {
            str = obs.down_text;
        }
    }
    // Increase or Decreasing the production level if needed
    if increment != 0 {
        if increment < 0 && unsafe { (*state).prod_level == PRODLEVEL_MINIMUM } {
            closeit = true;
        } else {
            unsafe {
                (*state).prod_level = ((i32::from((*state).prod_level) + i32::from(increment))
                    as u32)
                    .clamp(u32::from(PRODLEVEL_MINIMUM), u32::from(PRODLEVEL_MAXIMUM))
                    as u8;
            }
            recalculate_multipliers = true;
        }
    }
    // Recalculate production_rate
    // For non-smooth economy these should always be synchronized with prod_level
    if recalculate_multipliers {
        recompute_production_multipliers(obs.owner);
    }
    // Close if needed and allowed
    if closeit
        && !check_industry_close_down_protection(obs, svc)
        && !ctlflags(state).test(IndustryControlFlag::NoClosure)
    {
        unsafe {
            (*state).prod_level = PRODLEVEL_CLOSURE;
            (svc.set_dirty)(i);
        }
        str = obs.closure_text;
    }
    if !suppress_message && str != StringID::STR_NULL {
        // Compute news category, set the parameters of the news string and report the news
        // to the user.
        unsafe {
            (svc.news)(
                i,
                str.0,
                closeit,
                if closeit {
                    Servicing::Nobody
                } else {
                    servicing(i, obs.owner, svc)
                },
            );
        };
    }
}
/// `IndustryTypeBuildData::GetIndustryTypeData`: set the `probability` and `min_number`
/// fields for the industry type `it` for a running game. Returns whether at least one of
/// the fields has changed value.
fn get_industry_type_data(f: *mut BuildFields, it: IndustryType, s: &Services) -> bool {
    let (probability, min_number) = get_industry_game_play_probability(it, s);
    let probability = u32::from(probability);
    unsafe {
        let changed = min_number != (*f).min_number || probability != (*f).probability;
        (*f).min_number = min_number;
        (*f).probability = probability;
        changed
    }
}
/// `GetIndustryGamePlayProbability`: the probability of `it` during gameplay, with its
/// `min_number`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "GetIndustryGamePlayProbability keeps the original's implicit C++ integer conversions"
)]
fn get_industry_game_play_probability(it: IndustryType, s: &Services) -> (u16, u8) {
    if unsafe { (s.fund_only)() } {
        return (0, 0);
    }
    let ind_spc = get_industry_spec(it);
    let climate = unsafe { s.shared().settings().climate.read() };
    let (behaviour, enabled, layouts, chance) = unsafe {
        (
            IndustryBehaviours((*ind_spc).behaviour),
            (*ind_spc).enabled,
            !(*ind_spc).layouts.is_empty(),
            (*ind_spc).appear_ingame[usize::from(climate)],
        )
    };
    let year = crate::timer_game_common::calendar_year().0;
    if !enabled
        || !layouts
        || (behaviour.test(IndustryBehaviour::Before1950) && year > 1950)
        || (behaviour.test(IndustryBehaviour::After1960) && year < 1960)
    {
        return (0, 0);
    }
    let chance = resolver::get_industry_probability_callback(
        it,
        IndustryAvailabilityCallType::IACT_RANDOMCREATION,
        u32::from(chance),
        unsafe { &*s.resolver },
    ) as u8;
    if chance == 0 {
        return (0, 0);
    }
    (
        u16::from(chance),
        u8::from(behaviour.test(IndustryBehaviour::CanCloseLastInstance)),
    )
}
/// # Safety
/// Live build record; no reference is retained across the resolver service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_type(
    f: *mut BuildFields,
    kind: u8,
    s: *const Services,
) -> u8 {
    u8::from(get_industry_type_data(f, IndustryType(kind), unsafe {
        &*s
    }))
}
fn build_fields(builder: *mut Builder) -> *mut BuilderFields {
    unsafe { ptr::addr_of_mut!((*builder).fields) }
}
/// `IndustryBuildData::builddata[it]`.
fn builddata(b: *mut BuilderFields, it: IndustryType) -> *mut BuildFields {
    unsafe { ptr::addr_of_mut!((*b).builddata[usize::from(it.0)]) }
}
/// Every industry type, `0..NUM_INDUSTRYTYPES`.
fn industry_types() -> impl Iterator<Item = IndustryType> {
    (0..IndustryType::NUM_INDUSTRYTYPES.0).map(IndustryType)
}
impl BuildFields {
    /// `IndustryTypeBuildData::Reset`: reset the entry.
    fn reset(&mut self) {
        self.probability = 0;
        self.min_number = 0;
        self.target_count = 0;
        self.max_wait = 1;
        self.wait_count = 0;
    }
}
/// `IndustryBuildData::Reset`: completely reset the industry build data.
fn industry_build_data_reset(builder: *mut Builder) {
    let b = build_fields(builder);
    unsafe {
        (*b).wanted_inds = total(builder).wrapping_shl(16);
        for it in industry_types() {
            (*builddata(b, it)).reset();
        }
    }
}
/// # Safety
/// Live exclusive builder operation, with source total-count service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_reset(b: *mut Builder, _s: *const Services) {
    industry_build_data_reset(b);
}
/// lower 16 bits is a float fraction, 3.5 industries per decade, divided by 10 * 12 months.
const NEWINDS_PER_MONTH: u32 = 0x38000 / (10 * 12);
/// `IndustryBuildData::EconomyMonthlyLoop`: monthly update of industry build data.
fn industry_build_data_economy_monthly_loop(builder: *mut Builder, s: &Services) {
    // 'no industries' setting.
    if unsafe { (s.fund_only)() } {
        return;
    }
    let b = build_fields(builder);
    // To prevent running out of unused industries for the player to connect,
    // add a fraction of new industries each month, but only if the manager can keep up.
    // At most 2 industries for small maps, and 100 at the biggest map (about 6 months
    // industry build attempts).
    let max_behind = 1 + unsafe { (s.scale_by_map_size)(3) }.min(99);
    if total(builder).wrapping_add(max_behind) >= unsafe { (*b).wanted_inds >> 16 } {
        let increment = unsafe { (s.scale_by_map_size)(NEWINDS_PER_MONTH) };
        unsafe {
            (*b).wanted_inds = (*b).wanted_inds.wrapping_add(increment);
        }
    }
}
/// # Safety
/// Live builder; direct total/map observations preserve the original order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_monthly(b: *mut Builder, s: *const Services) {
    industry_build_data_economy_monthly_loop(b, unsafe { &*s });
}
/// `IndustryBuildData::SetupTargetCount`: decide how many industries of each type are needed.
fn setup_target_count(builder: *mut Builder, s: &Services) {
    let b = build_fields(builder);
    let mut changed = false;
    // Number of industries planned in the industry build data.
    let mut num_planned = 0u32;
    for it in industry_types() {
        let f = builddata(b, it);
        changed |= get_industry_type_data(f, it, s);
        num_planned += unsafe { u32::from((*f).target_count) };
    }
    // Desired total number of industries.
    let mut total_amount = unsafe { (*b).wanted_inds >> 16 };
    changed |= num_planned != total_amount;
    // All industries are still the same, no need to re-randomize.
    if !changed {
        return;
    }
    // Initialize the target counts.
    // Number of industries that should always be available.
    let mut force_build = 0;
    // Sum of probabilities.
    let mut total_prob = 0u32;
    for it in industry_types() {
        let ibd = builddata(b, it);
        unsafe {
            force_build += u32::from((*ibd).min_number);
            (*ibd).target_count = u16::from((*ibd).min_number);
            total_prob = total_prob.wrapping_add((*ibd).probability);
        }
    }
    // No buildable industries.
    if total_prob == 0 {
        return;
    }
    // Subtract forced industries from the number of industries available for construction.
    total_amount = total_amount.saturating_sub(force_build);
    // Assign number of industries that should be aimed for, by using the probability as a
    // weight.
    while total_amount > 0 {
        let mut r = random_range(s, total_prob);
        let mut it = IndustryType(0);
        while r >= unsafe { (*builddata(b, it)).probability } {
            r -= unsafe { (*builddata(b, it)).probability };
            it.0 += 1;
        }
        unsafe {
            (*builddata(b, it)).target_count = (*builddata(b, it)).target_count.wrapping_add(1);
        }
        total_amount -= 1;
    }
}
/// # Safety
/// Live builder; slot fields are read/written only between source services.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_targets(b: *mut Builder, s: *const Services) {
    setup_target_count(b, unsafe { &*s });
}
/// `Industry::GetIndustryTypeCount`.
fn get_industry_type_count(s: &Services, it: IndustryType) -> i32 {
    i32::from(unsafe { (s.type_count)(it.0) })
}
/// Try to place the industry in the game. Since there is no feedback why placement fails,
/// there is no other option than to try a few times before concluding it does not work.
/// `try_hard` tries very hard to find a place (used to place at least one industry per
/// type). Returns the created industry, or null if creation failed.
fn place_industry(
    kind: IndustryType,
    creation_type: IndustryAvailabilityCallType,
    try_hard: bool,
    s: &Services,
) -> *mut c_void {
    for _ in 0..if try_hard { 10000 } else { 2000 } {
        // CreateNewIndustry(RandomTile(), type, creation_type)
        let ind = unsafe { (s.create)(kind.0, creation_type.0) };
        if !ind.is_null() {
            return ind;
        }
    }
    ptr::null_mut()
}
/// `IndustryBuildData::TryBuildNewIndustry`: try to create a random industry, during
/// gameplay.
#[allow(
    clippy::cast_sign_loss,
    reason = "IndustryBuildData::TryBuildNewIndustry keeps the original's implicit C++ integer conversions"
)]
fn try_build_new_industry(builder: *mut Builder, s: &Services) {
    setup_target_count(builder, s);
    let b = build_fields(builder);
    // Number of industries that need to be build.
    let mut missing = 0i32;
    // Number of industry types eligible for build.
    let mut count = 0;
    // Sum of probabilities.
    let mut total_prob = 0u32;
    // Industry type that should be forcibly build.
    let mut forced_build = IndustryType::NUM_INDUSTRYTYPES;
    for it in industry_types() {
        let f = builddata(b, it);
        let difference = unsafe { i32::from((*f).target_count) } - get_industry_type_count(s, it);
        missing = missing.wrapping_add(difference);
        // This type may not be built now.
        if unsafe { (*f).wait_count > 0 } {
            continue;
        }
        if difference > 0 {
            if get_industry_type_count(s, it) == 0
                && unsafe { (*f).min_number > 0 }
                // An industry that should exist at least once, is not available. Force it,
                // trying the most needed one first.
                && (forced_build == IndustryType::NUM_INDUSTRYTYPES
                    || difference
                        > unsafe { i32::from((*builddata(b, forced_build)).target_count) }
                            - get_industry_type_count(s, forced_build))
            {
                forced_build = it;
            }
            total_prob = total_prob.wrapping_add(difference as u32);
            count += 1;
        }
    }
    // Skip creation of an industry.
    if (s.shared().recession)()
        || (forced_build == IndustryType::NUM_INDUSTRYTYPES && (missing <= 0 || total_prob == 0))
    {
        count = 0;
    }
    if count >= 1 {
        // If not forced, pick a weighted random industry to build.
        // For the case that count == 1, there is no need to draw a random number.
        let it = if forced_build == IndustryType::NUM_INDUSTRYTYPES {
            // Non-forced, select an industry type to build (weighted random).
            let mut r = if count > 1 {
                random_range(s, total_prob)
            } else {
                0
            };
            let mut it = IndustryType(0);
            loop {
                let f = builddata(b, it);
                // Type may not be built now; too many of this kind.
                if unsafe { (*f).wait_count == 0 } {
                    let difference =
                        unsafe { i32::from((*f).target_count) } - get_industry_type_count(s, it);
                    if difference > 0 {
                        if count == 1 || r < difference as u32 {
                            break it;
                        }
                        r -= difference as u32;
                    }
                }
                it.0 += 1;
            }
        } else {
            forced_build
        };
        // Try to create the industry.
        let ind = place_industry(
            it,
            IndustryAvailabilityCallType::IACT_RANDOMCREATION,
            false,
            s,
        );
        let f = builddata(b, it);
        if ind.is_null() {
            unsafe {
                // Compensate for decrementing below.
                (*f).wait_count = (*f).max_wait.wrapping_add(1);
                (*f).max_wait = (u32::from((*f).max_wait) + 2).min(1000) as u16;
            }
        } else {
            advertise(ind, s);
            unsafe {
                // Reduce waiting time of the industry type.
                (*f).max_wait = ((*f).max_wait / 2).max(1);
            }
        }
    }
    // Decrement wait counters.
    for it in industry_types() {
        let f = builddata(b, it);
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
    try_build_new_industry(b, unsafe { &*s });
}
/// Every economy day handler for the industry changes (`_economy_industries_daily`).
/// Taking the original map size of 256*256, the number of random changes was always of just
/// one unit. But it cannot be the same on smaller or bigger maps. That number has to be
/// scaled up or down. For small maps, it implies that less than one change per month is
/// required, while on bigger maps, it would be way more. The daily loop handles those
/// changes.
///
/// # Safety
/// Live builder and original ordered daily timer invocation. The company services
/// temporarily change and restore the original current company.
pub(crate) unsafe fn economy_industries_daily(builder: *mut Builder, services: *const Services) {
    let s = unsafe { &*services };
    let b = build_fields(builder);
    unsafe {
        (*b).daily_counter = (*b).daily_counter.wrapping_add((*b).daily_increment);
    }
    // Bits 16-31 of industry_construction_counter contain the number of industries to
    // change/create today, the lower 16 bit are a fractional part that might accumulate
    // over several days until it is sufficient for an industry.
    let change_loop = unsafe { ((*b).daily_counter >> 16) as u16 };
    // Reset the active part of the counter, just keeping the "fractional part"
    unsafe {
        (*b).daily_counter &= 0xFFFF;
    }
    // Nothing to do? get out
    if change_loop == 0 {
        return;
    }
    let cur_company = unsafe { (s.company_none)() };
    // perform the required industry changes for the day
    // Between 3% and 9% chance of creating a new industry.
    let mut perc = 3;
    let wanted = unsafe { (*b).wanted_inds >> 16 };
    if wanted > total(builder) {
        perc = (perc + wanted - total(builder)).min(9);
    }
    for _ in 0..change_loop {
        if chance16(s, perc, 100) {
            try_build_new_industry(builder, s);
        } else {
            let e = random_entry(s);
            if !e.industry.is_null() {
                change_industry_production(e.industry, e.owner, s, false);
                unsafe { (s.set_dirty)(e.industry) };
            }
        }
    }
    unsafe {
        (s.restore_company)(cur_company);
        // production-change
        (s.directory_dirty)(IndustryDirectoryInvalidateWindowData::IDIWD_PRODUCTION_CHANGE.0);
    }
}
/// Every economy month handler for the industries (`_economy_industries_monthly`).
///
/// # Safety
/// Live builder and original ordered monthly invocation. Destruction happens only
/// after statistics and with all Rust owner borrows ended; next ID matches pool order.
pub(crate) unsafe fn economy_industries_monthly(builder: *mut Builder, services: *const Services) {
    let s = unsafe { &*services };
    let cur_company = unsafe { (s.company_none)() };
    industry_build_data_economy_monthly_loop(builder, s);
    let mut from = u32::from(IndustryID::BEGIN.0);
    loop {
        let e = unsafe { (s.next_tick)(from) };
        if e.industry.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        update_industry_statistics(e.owner, s);
        if unsafe { (*fields(e.owner)).prod_level == PRODLEVEL_CLOSURE } {
            unsafe { (s.destroy)(e.industry) };
        } else {
            change_industry_production(e.industry, e.owner, s, true);
            unsafe { (s.set_dirty)(e.industry) };
        }
    }
    unsafe {
        (s.restore_company)(cur_company);
        // production-change
        (s.directory_dirty)(IndustryDirectoryInvalidateWindowData::IDIWD_PRODUCTION_CHANGE.0);
    }
}
/// `TrimIndustryAcceptedProduced`.
///
/// # Safety
/// Live industry with no outstanding iterators. Rust owns removal/history freeing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_trim(i: *mut Industry) {
    unsafe {
        let a = &mut (*i).accepted;
        let n = a
            .iter()
            .rposition(|a| is_valid_cargo_type(a.cargo))
            .map_or(0, |n| n + 1);
        a.truncate(n);
        a.shrink_to_fit();
        let p = &mut (*i).produced;
        let n = p
            .iter()
            .rposition(|p| is_valid_cargo_type(p.cargo))
            .map_or(0, |n| n + 1);
        p.truncate(n);
        p.shrink_to_fit();
    }
}

/// `IsSuitableForFarmField`.
fn is_suitable_for_farm_field(
    tile: TileIndex,
    allow_fields: bool,
    allow_rough: bool,
    s: &Services,
) -> bool {
    let info = unsafe { (s.farm_tile)(tile.0) };
    match TileType(info.kind) {
        TileType::MP_CLEAR => {
            if info.snow != 0 {
                return false;
            }
            match ClearGround(info.ground) {
                ClearGround::CLEAR_DESERT => false,
                ClearGround::CLEAR_ROUGH => allow_rough,
                ClearGround::CLEAR_FIELDS => allow_fields,
                _ => true,
            }
        }
        TileType::MP_TREES => {
            TreeGround(info.ground) != TreeGround::TREE_GROUND_SHORE
                && (allow_rough || TreeGround(info.ground) != TreeGround::TREE_GROUND_ROUGH)
        }
        _ => false,
    }
}
/// Build farm field fence. `tile` is the tile to position the fence on, `size` the size of
/// the field being planted in tiles, `kind` the type of fence to set and `side` the side of
/// the tile to attempt placement.
#[allow(
    clippy::similar_names,
    reason = "SetupFarmFieldFence keeps the original's local variable names"
)]
fn setup_farm_field_fence(
    mut tile: TileIndex,
    mut size: u32,
    kind: u8,
    side: DiagDirection,
    map: Map,
    s: &Services,
) {
    let world = crate::map_func::World::new(s.shared());
    let check = |ok| world.check(ok);
    let diff = map
        .coordinates()
        .offs_by_axis(other_axis(diag_dir_to_axis(side.0, check), check));
    loop {
        tile = TileIndex(map.coordinates().wrap_to_map(tile.0, map.size_y));
        // The tile is a field whose neighbour in `side` is not a field with a fence in the
        // same position.
        if unsafe { (s.fence_wanted)(tile.0, side.0) } {
            // Add fence as long as neighbouring tile does not already have a fence in the
            // same position.
            let mut or = kind;
            if or == 1 && chance16(s, 1, 7) {
                or = 2;
            }
            (s.shared().set_fence)(tile.0, side, u32::from(or));
        }
        tile = TileIndex(tile.0.wrapping_add(diff));
        size -= 1;
        if size == 0 {
            break;
        }
    }
}
/// `_plantfarmfield_type`.
const PLANTFARMFIELD_TYPE: [u8; 16] = [1, 1, 1, 1, 1, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6];
/// `PlantFarmField`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "PlantFarmField keeps the original's implicit C++ integer conversions"
)]
fn plant_farm_field(
    tile: TileIndex,
    industry: IndustryID,
    map: Map,
    svc: &Services,
    landscape: &impl Fn() -> LandscapeType,
) {
    if landscape() == LandscapeType::Arctic
        && (svc.shared().tile_z)(tile.0) + 2 >= i32::from(crate::snow_line::get(svc.shared()))
    {
        return;
    }
    // determine field size
    let mut r = (random(svc) & 0x303) + 0x404;
    if landscape() == LandscapeType::Arctic {
        r += 0x404;
    }
    let size_x = gb(r, 0, 8);
    let size_y = gb(r, 8, 8);
    let tiles = map.coordinates();
    let (tile_x, tile_y) = (tiles.tile_x(tile.0), tiles.tile_y(tile.0));
    let mut ta = TileArea::new(
        tiles.tile_xy(
            tile_x - tile_x.min(size_x / 2),
            tile_y - tile_y.min(size_y / 2),
        ),
        size_x as u16,
        size_y as u16,
    );
    let world = crate::map_func::World::new(svc.shared());
    ta.clamp_to_map(tiles, map.size_x.wrapping_mul(map.size_y), |ok| {
        world.check(ok);
    });
    if ta.w == 0 || ta.h == 0 {
        return;
    }
    // check the amount of bad tiles
    let mut count = 0;
    for cur_tile in ta.tiles(tiles) {
        count += u32::from(is_suitable_for_farm_field(
            TileIndex(cur_tile),
            false,
            false,
            svc,
        ));
    }
    if count * 2 < u32::from(ta.w) * u32::from(ta.h) {
        return;
    }
    // determine type of field
    r = random(svc);
    let counter = gb(r, 5, 3) as u8;
    let field_type = ((gb(r, 8, 8) * 9) >> 8) as u8;
    // make field
    for cur_tile in ta.tiles(tiles) {
        if is_suitable_for_farm_field(TileIndex(cur_tile), true, true, svc) {
            unsafe { (svc.make_field)(cur_tile, field_type, counter, industry.0) };
        }
    }
    let mut kind = 3;
    if landscape() != LandscapeType::Arctic && landscape() != LandscapeType::Tropic {
        kind = PLANTFARMFIELD_TYPE[(random(svc) & 0xF) as usize];
    }
    let (width, height) = (u32::from(ta.w), u32::from(ta.h));
    let north = TileIndex(ta.tile);
    setup_farm_field_fence(north, height, kind, DiagDirection::DIAGDIR_NE, map, svc);
    setup_farm_field_fence(north, width, kind, DiagDirection::DIAGDIR_NW, map, svc);
    setup_farm_field_fence(
        TileIndex(ta.tile.wrapping_add(width - 1)),
        height,
        kind,
        DiagDirection::DIAGDIR_SW,
        map,
        svc,
    );
    setup_farm_field_fence(
        TileIndex(ta.tile.wrapping_add((height - 1).wrapping_mul(map.size_x))),
        width,
        kind,
        DiagDirection::DIAGDIR_SE,
        map,
        svc,
    );
}
/// `PlantRandomFarmField`, with the landscape read at each original observation.
#[allow(
    clippy::cast_possible_wrap,
    reason = "PlantRandomFarmField keeps the original's implicit C++ integer conversions"
)]
fn plant_random_farm_field(
    i: *mut c_void,
    id: IndustryID,
    map: Map,
    s: &Services,
    landscape: &impl Fn() -> LandscapeType,
) {
    let location = unsafe { (s.location)(i) };
    let x = (u32::from(location.w) / 2)
        .wrapping_add(random(s) % 31)
        .wrapping_sub(16) as i32;
    let y = (u32::from(location.h) / 2)
        .wrapping_add(random(s) % 31)
        .wrapping_sub(16) as i32;
    let tile = TileIndex(map.coordinates().tile_add_wrap(
        location.tile,
        x,
        y,
        map.size_y,
        (unsafe { &*s.environment }.freeform_edges)(),
    ));
    if tile != TileIndex::INVALID {
        plant_farm_field(tile, id, map, s, landscape);
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
/// `PlantRandomFarmField`, reading the landscape from the game settings.
fn plant_random_farm_field_with_settings(i: *mut c_void, id: IndustryID, map: Map, s: &Services) {
    plant_random_farm_field(i, id, map, s, &|| {
        LandscapeType(unsafe { s.shared().settings().climate.read() })
    });
}
/// # Safety
/// Live industry, synchronous map/RNG services. No owner references cross calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_farm(i: *mut c_void, id: u16, s: *const Services) {
    let s = unsafe { &*s };
    plant_random_farm_field_with_settings(i, IndustryID(id), tick_map(s), s);
}
/// Perform a circular search around the Lumber Mill in order to find trees to cut.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ChopLumberMillTrees keeps the original's implicit C++ integer conversions"
)]
fn chop_lumber_mill_trees(
    i: *mut c_void,
    owner: *mut Industry,
    map: Map,
    s: &Services,
    builder: *mut Builder,
) {
    // Don't process lumber mill if cargo is not set up correctly.
    if produced_count(owner) == 0 || unsafe { !is_valid_cargo_type((*produced(owner, 0)).cargo) } {
        return;
    }
    let location = unsafe { (s.location)(i) };
    let tiles = map.coordinates();
    // We only want to cut trees if all tiles are completed.
    for tile_cur in location.tiles(tiles) {
        // TileBelongsToIndustry(tile_cur) && !IsIndustryCompleted(tile_cur)
        if unsafe { !(s.tile_completed)(i, tile_cur) } {
            return;
        }
    }
    let width = map.size_x;
    // 40x40 tiles to search.
    let mut spiral = crate::spiral::square(
        tiles.tile_x(location.tile),
        tiles.tile_y(location.tile),
        40,
        width,
        map.size_y,
    );
    while !spiral.is_end() {
        let tile = tiles.tile_xy(spiral.x, spiral.y);
        let info = unsafe { (s.farm_tile)(tile) };
        if TileType(info.kind) == TileType::MP_TREES && info.grown != 0 {
            // found a tree
            let b = build_fields(builder);
            unsafe {
                (*b).sound_ctr = 1;
                (*b).sound_tile = tile;
                // SND_38_LUMBER_MILL_1 and the CMD_LANDSCAPE_CLEAR as OWNER_NONE.
                (s.harvest)(tile);
            }
            // Add according value to waiting cargo.
            let amount = unsafe { (s.scale_cargo)(45) };
            unsafe {
                let p = produced(owner, 0);
                (*p).waiting = (u32::from((*p).waiting) + amount).min(u32::from(u16::MAX)) as u16;
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
/// `TriggerIndustryProduction`.
///
/// SAFETY: Live serialized industry, immutable services and spec; callbacks retain no owner borrows.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "TriggerIndustryProduction keeps the original's implicit C++ integer conversions"
)]
pub(crate) unsafe fn trigger_industry_production(
    shell: *mut ShellHandle,
    owner: *mut Industry,
    id: IndustryID,
    resolver: &resolver::ResolverServices,
    services: &Services,
    delivery: &DeliveryServices,
) {
    let indspec = spec_of(owner);
    let cbm = spec_callback_mask(indspec);
    unsafe { (*owner).fields.was_cargo_delivered = 1 };
    if cbm.test(IndustryCallbackMask::ProductionCargoArrival)
        || cbm.test(IndustryCallbackMask::Production256Ticks)
    {
        if cbm.test(IndustryCallbackMask::ProductionCargoArrival) {
            // IndustryProductionCallback(i, 0)
            let mut storage = std::mem::MaybeUninit::<ProductionStorage>::uninit();
            let context = unsafe { (delivery.construct)(storage.as_mut_ptr(), shell) };
            industry_production_callback(
                shell.cast(),
                owner,
                spec_behaviour(indspec),
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
        for ita in 0..accepted_count(owner) {
            let a = accepted(owner, ita);
            let waiting = unsafe { (*a).waiting };
            if waiting == 0 || !is_valid_cargo_type(unsafe { (*a).cargo }) {
                continue;
            }
            for itp in 0..produced_count(owner) {
                let p = produced(owner, itp);
                if !is_valid_cargo_type(unsafe { (*p).cargo }) {
                    continue;
                }
                let old = unsafe { (*p).waiting };
                let multiplier = unsafe { (*indspec).input_cargo_multiplier[ita][itp] };
                let product = i32::from(waiting).wrapping_mul(i32::from(multiplier));
                unsafe {
                    (*p).waiting = i32::from(old)
                        .wrapping_add(product / 256)
                        .clamp(0, i32::from(u16::MAX)) as u16;
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
            IndustryRandomTrigger::CargoReceived.0,
            services.shared().map().size_x(),
            services,
            resolver.lifecycle(),
        );
        openttd_rust_industry_animate_abi(
            owner,
            id.0,
            IndustryAnimationTrigger::CargoReceived.0,
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
    industry_production_callback(
        i,
        owner,
        IndustryBehaviours(behaviour),
        reason,
        unsafe { &*services },
        |random, parameter, result| unsafe {
            resolve(context, random, parameter, result);
        },
    );
}
/// `IndustryProductionCallback`: get the industry production callback and apply it to the
/// industry. `reason` is the reason it is called (0 = incoming cargo, 1 = periodic tick
/// callback).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "IndustryProductionCallback keeps the original's implicit C++ integer conversions"
)]
fn industry_production_callback(
    i: *mut c_void,
    owner: *mut Industry,
    behaviour: IndustryBehaviours,
    reason: u8,
    s: &Services,
    mut resolve: impl FnMut(u32, u32, *mut ProductionResult),
) {
    let callback_param1 = if behaviour.test(IndustryBehaviour::ProdCallbackRandom) {
        random(s)
    } else {
        0
    };
    let multiplier = if behaviour.test(IndustryBehaviour::ProdMultiHandling) {
        unsafe { i32::from((*fields(owner)).prod_level) }
    } else {
        1
    };
    let mut callback_param2 = u32::from(reason);
    for loop_ in 0..=0x10000 {
        // limit the number of calls to break infinite loops.
        // 'loop' is provided as 16 bits to the newgrf, so abort when those are exceeded.
        if loop_ >= 0x10000 {
            // display error message; abort the function early, this error isn't critical and
            // will allow the game to continue to run
            unsafe { (s.callback_error)(i, false) };
            break;
        }
        sb(&mut callback_param2, 8, 16, loop_);
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
        resolve(callback_param1, callback_param2, &raw mut result);
        if result.present == 0 {
            break;
        }
        if result.version == 0xFF {
            // Result was marked invalid on load, display error message; abort the function
            // early, this error isn't critical and will allow the game to continue to run
            unsafe { (s.callback_error)(i, true) };
            break;
        }
        // Version < 2: callback parameters map directly to industry cargo slot indices.
        // Otherwise the callback receives a list of cargos to apply for, which need to have
        // their cargo slots in industry looked up.
        for n in 0..usize::from(result.num_input) {
            let slot = if result.version < 2 {
                if n >= accepted_count(owner) {
                    break;
                }
                Some(n)
            } else {
                unsafe { get_cargo_accepted(owner, CargoType(result.cargo_input[n])) }
            };
            if let Some(nslot) = slot {
                let a = accepted(owner, nslot);
                unsafe {
                    if result.version < 2 && !is_valid_cargo_type((*a).cargo) {
                        continue;
                    }
                    (*a).waiting = i32::from((*a).waiting)
                        .wrapping_sub(result.subtract[n].wrapping_mul(multiplier))
                        .clamp(0, i32::from(u16::MAX)) as u16;
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
                unsafe { get_cargo_produced(owner, CargoType(result.cargo_output[n])) }
            };
            if let Some(nslot) = slot {
                let p = produced(owner, nslot);
                unsafe {
                    if result.version < 2 && !is_valid_cargo_type((*p).cargo) {
                        continue;
                    }
                    (*p).waiting = i32::from((*p).waiting)
                        .wrapping_add(result.add[n].max(0).wrapping_mul(multiplier))
                        .clamp(0, i32::from(u16::MAX)) as u16;
                }
            }
        }
        if result.again == 0 {
            break;
        }
        sb(&mut callback_param2, 24, 8, result.again as u32);
    }
    // SetWindowDirty(WC_INDUSTRY_VIEW, ind->index)
    unsafe { (s.set_dirty)(i) };
}

/// # Safety
/// Live builder record; resetting has no callback or allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_build_type_reset(fields: *mut BuildFields) {
    unsafe { (*fields).reset() };
}
/// `StartupIndustryDailyChanges`: initialize the variables that will maintain the daily
/// industry change system. `init_counter` specifies if the counter is required to be
/// initialized.
///
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
        // After getting map size, it needs to be scaled appropriately and divided by 31,
        // which stands for the days in a month.
        // Using just 31 will make it so that a monthly reset (based on the real number of
        // days of that month) would not be needed.
        // Since it is based on "fractional parts", the leftover days will not make much of
        // a difference on the overall total number of changes performed
        (*fields).daily_increment = (1u32 << map_bits) / 31;
        // A new game or a savegame from an older version will require the counter to be
        // initialized
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
    let id = IndustryID(id);
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
    if let Ok(index) = registry.binary_search(&IndustryID(id)) {
        registry.remove(index);
    }
}
/// # Safety
/// Live builder and valid original type. No registry view survives this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_registry_clear(owner: *mut Builder, kind: u8) {
    unsafe { (*owner).registry[usize::from(kind)].clear() };
}

/// `Industry::location.tile`, the only industry location the subsidy search reads.
/// SAFETY: Live serialized industry owner, immediate read with no callbacks.
pub(crate) unsafe fn subsidy_position(owner: *const Industry) -> TileIndex {
    TileIndex(unsafe { (*owner).metadata.tile })
}
/// SAFETY: Live serialized owner; no callback occurs while reading its cargoes.
#[allow(
    clippy::cast_possible_truncation,
    reason = "FindSubsidyIndustryCargoRoute counts the valid produced cargoes as uint"
)]
pub(crate) unsafe fn subsidy_produced_count(owner: *const Industry) -> u32 {
    unsafe { &(*owner).produced }
        .iter()
        .filter(|p| is_valid_cargo_type(p.cargo))
        .count() as u32
}
/// SAFETY: Original random cargo ordinal is less than the valid cargo count.
/// The cargo, last-month production and transported fraction of the valid produced slot
/// with ordinal `number`.
///
/// SAFETY: Original random cargo ordinal is less than the valid cargo count.
pub(crate) unsafe fn subsidy_produced(owner: *const Industry, number: u32) -> (CargoType, u16, u8) {
    let p = unsafe { &(*owner).produced }
        .iter()
        .filter(|p| is_valid_cargo_type(p.cargo))
        .nth(number as usize)
        .unwrap();
    let history = p.history[usize::from(LAST_MONTH)];
    (CargoType(p.cargo), history.first, history.pct_transported())
}
/// `Industry::IsCargoAccepted(cargo)`.
///
/// SAFETY: Live serialized owner; same accepted-slot predicate as `IsCargoAccepted`.
pub(crate) unsafe fn subsidy_accepts(owner: *const Industry, cargo: CargoType) -> bool {
    unsafe { &(*owner).accepted }
        .iter()
        .any(|a| a.cargo == cargo.0)
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
            assert_eq!(get_cargo_accepted(owner, CargoType(7)), Some(0));
            assert_eq!(get_cargo_accepted(owner, CargoType::INVALID_CARGO), None);
            assert_eq!(get_cargo_accepted(owner, CargoType(8)), None);
            // Saturated stockpiles still create history and update last_accepted.
            for (pieces, amount, waiting, date) in [(20, 5, 65535, -12), (20, 0, 65535, 73)] {
                assert_eq!(
                    accept_delivery(owner, 0, pieces, || {
                        crate::timer_game_economy::EconomyDate(date)
                    }),
                    amount
                );
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
            assert_eq!(
                accept_delivery(owner, 0, 10, || {
                    crate::timer_game_economy::EconomyDate(99)
                }),
                10
            );
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
    #[allow(
        clippy::cast_possible_truncation,
        reason = "EnumBitSet<IndustryControlFlag, uint8_t> stores the element bit in uint8_t"
    )]
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
    /// Native `BaseBitSet::Any`: whether any of the given values are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
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
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "the test fixture narrows its IndustryProductionSpriteGroup values as the native result does"
    )]
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
            industry_production_callback(
                ptr::null_mut(),
                &raw mut industry,
                IndustryBehaviours(behaviour),
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
