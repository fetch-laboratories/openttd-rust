/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete town growth policy; canonical map, pools and shared caches remain C++.
//! Commands and `NewGRF` callbacks run directly after all owner access ends.
//! No private-state reference survives a shared service or reentry.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::decimal_bitwise_operands
)]
pub mod commands;
pub mod founding;
pub mod house;
pub mod import;
pub mod names;
pub(crate) mod policy;
pub mod resolver;
pub(crate) mod spatial;

use crate::history_type::LAST_MONTH;
use crate::random_func::chance16_i;
use crate::services::{Map, Services};
use crate::slope_func::{is_slope_with_one_corner_raised, is_steep_slope};
use crate::tile_type::{
    MP_HOUSE, MP_INDUSTRY, MP_OBJECT, MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE, MP_VOID,
};
use crate::timer_game_tick::TOWN_GROWTH_TICKS;

/// Value for custom town number in difficulty settings (`town.h`).
pub const CUSTOM_TOWN_NUMBER_DIFFICULTY: u32 = 4;

/// The town only needs this cargo in the winter (any amount).
pub const TOWN_GROWTH_WINTER: u32 = 0xFFFF_FFFE;
/// The town needs the cargo for growth when on desert (any amount).
pub const TOWN_GROWTH_DESERT: u32 = 0xFFFF_FFFF;

pub use crate::town_type::TownId;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TownRef {
    pub shell: *mut crate::services::TownHandle,
    pub state: *mut State,
    pub id: TownId,
}
use crate::game_state::GameState;
use crate::raw_field::{read_field, write_field};
use std::{cell::OnceCell, ffi::c_void};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Received {
    pub old_max: u16,
    pub new_max: u16,
    pub old_act: u16,
    pub new_act: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SimulationFields {
    pub valid_history: u64,
    pub xy: u32,
    pub num_houses: u32,
    pub population: u32,
    pub radius: [u32; 5],
    pub goal: [u32; 6],
    pub ratings: [i16; 15],
    pub received: [Received; 6],
    pub noise_reached: u16,
    pub time_until_rebuild: u16,
    pub statues: u16,
    pub have_ratings: u16,
    pub unwanted: [u8; 15],
    pub exclusivity: u8,
    pub exclusive_counter: u8,
    pub larger_town: bool,
    pub layout: u8,
    pub townnamegrfid: u32,
    pub townnametype: u16,
    pub townnameparts: u32,
}
impl Default for SimulationFields {
    fn default() -> Self {
        Self {
            valid_history: 0,
            xy: u32::MAX,
            num_houses: 0,
            population: 0,
            radius: [0; 5],
            goal: [0; 6],
            ratings: [0; 15],
            received: [Received::default(); 6],
            noise_reached: 0,
            time_until_rebuild: 0,
            statues: 0,
            have_ratings: 0,
            unwanted: [0; 15],
            exclusivity: u8::MAX,
            exclusive_counter: 0,
            larger_town: false,
            layout: 0,
            townnamegrfid: 0,
            townnametype: 0,
            townnameparts: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SuppliedHistory {
    pub production: u32,
    pub transported: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Supplied {
    pub cargo: u8,
    pub history: [SuppliedHistory; 61],
}
impl Default for Supplied {
    fn default() -> Self {
        Self {
            cargo: u8::MAX,
            history: [SuppliedHistory::default(); 61],
        }
    }
}
#[repr(C)]
pub struct Slots {
    pub data: *mut c_void,
    pub size: usize,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GrowthFields {
    pub counter: u16,
    pub rate: u16,
    pub funding: u8,
    pub road: u8,
    pub flags: u8,
}
#[derive(Default)]
pub struct State {
    name: Vec<u8>,
    text: Vec<u8>,
    pub(crate) simulation: SimulationFields,
    supplied: Vec<Supplied>,
    id_counts: Vec<u16>,
    class_counts: Vec<u16>,
    growth: GrowthFields,
    stations_near: crate::station::StationList,
}
/// SAFETY: Live serialized town owner and original cargo acceptance effect.
pub(crate) unsafe fn record_delivery(owner: *mut State, effect: u8, amount: u32) {
    unsafe {
        let count = &mut (*owner).simulation.received[usize::from(effect)].new_act;
        *count = count.wrapping_add(amount as u16);
    }
}
/// The town's nearby station set, for immediate borrows only.
pub(crate) fn stations_near(state: *mut State) -> *mut crate::station::StationList {
    // SAFETY: Live owner supplied by the caller's resolved town.
    unsafe { &raw mut (*state).stations_near }
}
/// Native `Town::stations_near` facade storage.
/// # Safety
/// `state` is live; the alias ends with the owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_stations_near(
    state: *mut State,
) -> *mut crate::station::StationList {
    stations_near(state)
}
/// `Town::GetCargoSupplied` followed by the original history-period lookup.
/// SAFETY: Live serialized canonical owner; the returned value is a copy.
unsafe fn supplied_history(state: *const State, cargo: u8, period: u8) -> Option<SuppliedHistory> {
    if !crate::cargo_type::is_valid_cargo_type(cargo) {
        return None;
    }
    let index = unsafe { openttd_rust_town_supplied_find(state, cargo) };
    unsafe {
        let rows = &(*state).supplied;
        rows.get(index).map(|s| s.history[usize::from(period)])
    }
}
/// Read only the canonical position at the original observation point.
/// SAFETY: The caller supplies a live owner; this immediate copied read retains no borrow.
pub(crate) unsafe fn position(state: *const State) -> u32 {
    // SAFETY: Caller establishes owner lifetime and serialized access.
    unsafe { (*state).simulation.xy }
}
/// Read only the canonical population at the original observation point.
/// SAFETY: The caller supplies a live owner; this immediate copied read retains no borrow.
pub(crate) unsafe fn population(state: *const State) -> u32 {
    // SAFETY: Caller establishes owner lifetime and serialized access.
    unsafe { (*state).simulation.population }
}
/// Read only the canonical house count at the original observation point.
/// SAFETY: The caller supplies a live owner; this immediate copied read retains no borrow.
pub(crate) unsafe fn num_houses(state: *const State) -> u32 {
    // SAFETY: Caller establishes owner lifetime and serialized access.
    unsafe { (*state).simulation.num_houses }
}
fn radius_group(funded: bool, radii: &[u32; 5], distance: u32) -> u8 {
    if funded && distance <= 25 {
        return 4;
    }
    let mut zone = 0;
    for (index, radius) in radii.iter().enumerate() {
        if distance < *radius {
            zone = index as u8;
        }
    }
    zone
}
/// `GetTownRadiusGroup` over live canonical town fields. No owner borrow crosses
/// a callback. The caller supplies the original live town and valid tile.
pub(crate) unsafe fn get_radius_group(
    state: *const State,
    tile: u32,
    map: crate::map_func::Map,
) -> u8 {
    let state = unsafe { &*state };
    radius_group(
        state.growth.funding != 0,
        &state.simulation.radius,
        map.distance_square(tile, state.simulation.xy),
    )
}
/// `GetTownRadiusGroup` for a live native shell, resolving its canonical owner once.
pub(crate) unsafe fn radius_from_shell(
    s: &Services,
    town: *const crate::services::TownHandle,
    tile: u32,
) -> u8 {
    unsafe { get_radius_group((s.town_owner)(town), tile, s.map()) }
}
/// Copied canonical road-work policy inputs; no owner reference spans a service.
/// SAFETY: Caller supplies a live, serialized owner.
pub(crate) unsafe fn road_context(state: *const State, tile: u32, size_x: u32) -> (u8, u8, u32) {
    // SAFETY: Immediate reads only, caller establishes owner lifetime.
    let state = unsafe { &*state };
    let xy = state.simulation.xy;
    // Native TileX/TileY operate on the power-of-two map dimensions.
    let mask = size_x - 1;
    let shift = size_x.trailing_zeros();
    let dx = (tile & mask).abs_diff(xy & mask);
    let dy = (tile >> shift).abs_diff(xy >> shift);
    let zone = unsafe { get_radius_group(state, tile, crate::map_func::Map::from_log_x(shift)) };
    (state.growth.road, zone, dx + dy)
}
static GENERATING_TOWN: GameState<bool> = GameState::new(false);
pub extern "C" fn openttd_rust_town_generating_exchange_abi(generating: bool) -> u8 {
    // SAFETY: Serialized native entry; the flag is process-wide.
    u8::from(unsafe { std::ptr::replace(GENERATING_TOWN.get(), generating) })
}
/// Convert original town tick counters, including tick zero, to game ticks.
fn town_ticks_to_game_ticks(ticks: u16) -> u16 {
    ((u32::from(ticks.min(930)) + 1) * u32::from(TOWN_GROWTH_TICKS) - 1) as u16
}

unsafe fn update_radius(state: *mut State) {
    const RADII: [[u32; 5]; 23] = [
        [4, 0, 0, 0, 0],
        [16, 0, 0, 0, 0],
        [25, 0, 0, 0, 0],
        [36, 0, 0, 0, 0],
        [49, 0, 4, 0, 0],
        [64, 0, 4, 0, 0],
        [64, 0, 9, 0, 1],
        [64, 0, 9, 0, 4],
        [64, 0, 16, 0, 4],
        [81, 0, 16, 0, 4],
        [81, 0, 16, 0, 4],
        [81, 0, 25, 0, 9],
        [81, 36, 25, 0, 9],
        [81, 36, 25, 16, 9],
        [81, 49, 0, 25, 9],
        [81, 64, 0, 25, 9],
        [81, 64, 0, 36, 9],
        [81, 64, 0, 36, 16],
        [100, 81, 0, 49, 16],
        [100, 81, 0, 49, 25],
        [121, 81, 0, 49, 25],
        [121, 81, 0, 49, 25],
        [121, 81, 0, 49, 36],
    ];
    // SAFETY: Immediate field accesses on live owner; this helper invokes no services.
    let houses = unsafe { (*state).simulation.num_houses };
    let radius = if houses < 92 {
        RADII[(houses / 4) as usize]
    } else {
        let mass = (houses / 8) as i32;
        [
            mass.wrapping_mul(15).wrapping_sub(40) as u32,
            mass.wrapping_mul(9).wrapping_sub(15) as u32,
            0,
            mass.wrapping_mul(5).wrapping_sub(5) as u32,
            mass.wrapping_mul(3).wrapping_add(5) as u32,
        ]
    };
    // SAFETY: No callback or competing native access during this owner mutation.
    unsafe {
        (*state).simulation.radius = radius;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_update_radius(state: *mut State) {
    // SAFETY: Native shell supplies its live single owner.
    unsafe {
        update_radius(state);
    }
}
unsafe fn change_population(leaves: &Leaves, state: *mut State, town: u32, amount: i32) {
    // SAFETY: Field mutation ends before the presentation service can reenter.
    unsafe {
        (*state).simulation.population = population(state).wrapping_add(amount as u32);
    }
    // SAFETY: Immediate serialized read ends before presentation callbacks.
    if unsafe { *GENERATING_TOWN.get() } {
        return;
    }
    (leaves.population_changed)(town);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_world_population(leaves: *const Leaves) -> u32 {
    // SAFETY: Immutable table remains live; no owner access spans pool iteration.
    let leaves = unsafe { &*leaves };
    let mut total = 0_u32;
    let mut town = (leaves.next_town)(0);
    while town != u32::MAX {
        let owner = (leaves.resolve_owner)(town);
        // SAFETY: Iterator supplied a live shell; copy read ends before next callback.
        total = total.wrapping_add(unsafe { population(owner) });
        town = (leaves.next_town)(town.wrapping_add(1));
    }
    total
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_new() -> *mut State {
    Box::into_raw(Box::default())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_destroy(state: *mut State) {
    // SAFETY: The shell owns this allocation, destroying it exactly once.
    unsafe {
        drop(Box::from_raw(state));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_growth(state: *mut State) -> *mut GrowthFields {
    // SAFETY: Stable owner allocation; native trivial aliases live until destruction.
    unsafe { &raw mut (*state).growth }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_simulation(state: *mut State) -> *mut SimulationFields {
    // SAFETY: Stable shell-owned allocation. C++ starts the matching trivial native
    // object's lifetime here; aliases remain valid until owner destruction.
    unsafe { &raw mut (*state).simulation }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_supplied_view(state: *mut State) -> Slots {
    // SAFETY: Serialized shell-owned vector. Views end before reserve/insert.
    let supplied = unsafe { &mut (*state).supplied };
    Slots {
        data: supplied.as_mut_ptr().cast(),
        size: supplied.len(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_supplied_reserve(state: *mut State, capacity: usize) {
    // SAFETY: No native vector views survive mutation, matching std::vector.
    let supplied = unsafe { &mut (*state).supplied };
    if capacity > supplied.len() {
        supplied.reserve(capacity - supplied.len());
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_supplied_insert(
    state: *mut State,
    at: usize,
    cargo: u8,
) -> *mut Supplied {
    // SAFETY: Native facade supplies the source's valid insertion position.
    let supplied = unsafe { &mut (*state).supplied };
    supplied.insert(
        at,
        Supplied {
            cargo,
            ..Supplied::default()
        },
    );
    supplied.as_mut_ptr().wrapping_add(at)
}
/// Copied insertion result: native starts a matching trivial lifetime only for
/// a newly allocated row, preserving histories when the cargo already exists.
#[repr(C)]
pub struct SuppliedEntry {
    pub entry: *mut Supplied,
    pub created: bool,
}
unsafe fn supplied_entry(state: *mut State, cargo: u8) -> SuppliedEntry {
    // SAFETY: Live sole owner, no native vector views survive its mutation.
    let rows = unsafe { &mut (*state).supplied };
    let at = rows.partition_point(|row| row.cargo < cargo);
    let created = at == rows.len() || rows[at].cargo != cargo;
    if created {
        rows.insert(
            at,
            Supplied {
                cargo,
                ..Supplied::default()
            },
        );
    }
    SuppliedEntry {
        entry: rows.as_mut_ptr().wrapping_add(at),
        created,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_supplied_entry(
    state: *mut State,
    cargo: u8,
) -> SuppliedEntry {
    // SAFETY: Native facade has ended every vector view before this call.
    unsafe { supplied_entry(state, cargo) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_supplied_find(state: *const State, cargo: u8) -> usize {
    // SAFETY: Immediate shared owner access, with no callbacks or mutation.
    let rows = unsafe { &(*state).supplied };
    let at = rows.partition_point(|row| row.cargo < cargo);
    if at == rows.len() || rows[at].cargo != cargo {
        rows.len()
    } else {
        at
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_percent_transported(
    history: *const SuppliedHistory,
) -> u8 {
    // SAFETY: Native supplies one matching live history; both scalar reads end here.
    let (production, transported) = unsafe { ((*history).production, (*history).transported) };
    if production == 0 {
        return 0;
    }
    // Original uint32 multiplication wraps before division and uint8 clamping.
    (transported.wrapping_mul(256) / production).min(u32::from(u8::MAX)) as u8
}
#[derive(Default)]
struct GlobalCounts {
    ids: Vec<u32>,
    classes: Vec<u32>,
}
static BUILDING_COUNTS: GameState<GlobalCounts> = GameState::new(GlobalCounts {
    ids: Vec::new(),
    classes: Vec::new(),
});
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_id_counts(state: *mut State) -> Slots {
    // SAFETY: Canonical serialized vector. Native views end before resize.
    let counts = unsafe { &mut (*state).id_counts };
    Slots {
        data: counts.as_mut_ptr().cast(),
        size: counts.len(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_id_counts_resize(state: *mut State, count: usize) {
    // SAFETY: No native view survives a structural vector mutation.
    unsafe {
        (*state).id_counts.resize(count, 0);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_class_counts(state: *mut State) -> Slots {
    // SAFETY: Canonical serialized vector. Native views end before resize.
    let counts = unsafe { &mut (*state).class_counts };
    Slots {
        data: counts.as_mut_ptr().cast(),
        size: counts.len(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_class_counts_resize(state: *mut State, count: usize) {
    // SAFETY: No native view survives a structural vector mutation.
    unsafe {
        (*state).class_counts.resize(count, 0);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_id_counts() -> Slots {
    // SAFETY: Game-state-mutex serialized access; no service call or reentry.
    let counts = unsafe { &mut (*BUILDING_COUNTS.get()).ids };
    Slots {
        data: counts.as_mut_ptr().cast(),
        size: counts.len(),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_id_counts_resize(count: usize) {
    // SAFETY: Game-state-mutex serialized access; no native view survives resizing.
    unsafe {
        (*BUILDING_COUNTS.get()).ids.resize(count, 0);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_class_counts() -> Slots {
    // SAFETY: Game-state-mutex serialized access; no service call or reentry.
    let counts = unsafe { &mut (*BUILDING_COUNTS.get()).classes };
    Slots {
        data: counts.as_mut_ptr().cast(),
        size: counts.len(),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_class_counts_resize(count: usize) {
    // SAFETY: Game-state-mutex serialized access; no native view survives resizing.
    unsafe {
        (*BUILDING_COUNTS.get()).classes.resize(count, 0);
    }
}
/// Count one built house part in its town and the process-wide totals.
unsafe fn increase_building(state: *mut State, house: u16, class: u16) {
    // SAFETY: Original valid spec/count indices; no callback while borrowed.
    let state = unsafe { &mut *state };
    // SAFETY: All counters are serialized by the game-state mutex.
    let global = unsafe { &mut *BUILDING_COUNTS.get() };
    state.id_counts[usize::from(house)] = state.id_counts[usize::from(house)].wrapping_add(1);
    global.ids[usize::from(house)] = global.ids[usize::from(house)].wrapping_add(1);
    if class != 0 {
        state.class_counts[usize::from(class)] =
            state.class_counts[usize::from(class)].wrapping_add(1);
        global.classes[usize::from(class)] = global.classes[usize::from(class)].wrapping_add(1);
    }
}
/// Saturating removal of one house part from town and process-wide totals.
unsafe fn decrease_building(state: *mut State, house: u16, class: u16) {
    // SAFETY: Source valid indices and serialized original lifetime preconditions.
    let state = unsafe { &mut *state };
    // SAFETY: No callback spans the process-wide global count access.
    let global = unsafe { &mut *BUILDING_COUNTS.get() };
    state.id_counts[usize::from(house)] = state.id_counts[usize::from(house)].saturating_sub(1);
    global.ids[usize::from(house)] = global.ids[usize::from(house)].saturating_sub(1);
    if class != 0 {
        state.class_counts[usize::from(class)] =
            state.class_counts[usize::from(class)].saturating_sub(1);
        global.classes[usize::from(class)] = global.classes[usize::from(class)].saturating_sub(1);
    }
}
struct HouseCandidates {
    slots: [std::mem::MaybeUninit<(u32, u32)>; 4096],
    len: usize,
}
static HOUSE_CANDIDATES: GameState<HouseCandidates> = GameState::new(HouseCandidates {
    slots: [std::mem::MaybeUninit::uninit(); 4096],
    len: 0,
});
fn clear_house_candidates() {
    unsafe {
        (*HOUSE_CANDIDATES.get()).len = 0;
    }
}
fn add_house_candidate(house: u32, weight: u32) {
    // SAFETY: HouseSpec's original maximum is NUM_HOUSES=4096. No callback occurs.
    let candidates = unsafe { &mut *HOUSE_CANDIDATES.get() };
    candidates.slots[candidates.len].write((house, weight));
    candidates.len += 1;
}
fn take_house_candidate(mut random: u32) -> (u32, u32) {
    // SAFETY: Source's positive total guarantees an initialized weighted slot.
    // The source's static probability vector is also replaced by reentrant builds.
    let candidates = unsafe { &mut *HOUSE_CANDIDATES.get() };
    let index = (0..candidates.len)
        .find(|&at| {
            // SAFETY: Every slot below len was written while collecting specs.
            let (_, weight) = unsafe { candidates.slots[at].assume_init() };
            if weight > random {
                true
            } else {
                random -= weight;
                false
            }
        })
        .unwrap();
    // SAFETY: Selected and final slots are occupied. Match source swap/pop order.
    let selected = unsafe { candidates.slots[index].assume_init() };
    candidates.len -= 1;
    candidates.slots[index] = candidates.slots[candidates.len];
    selected
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoadChoice {
    pub introduction_date: i32,
    pub max_speed: u16,
    pub town_build: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseAvailability {
    pub zones: u16,
    pub override_id: u16,
    pub class_id: u16,
    pub probability: u8,
    pub enabled: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HousePlacement {
    pub min_year: i32,
    pub max_year: i32,
}
/// Narrow authority settings are read only at their source short-circuit points.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AuthorityServices {
    pub council_tolerance: extern "C" fn() -> u8,
    pub bulldozer: extern "C" fn() -> bool,
    pub authority_distance: extern "C" fn() -> u8,
    pub town_index: extern "C" fn(*const crate::services::TownHandle) -> u16,
    pub refusal: u32,
}

/// Native stack result and immutable tables remain live through this invocation.
/// No Rust owner borrow crosses a service; errors retain full native parameters.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_authority_station(
    authority: *const AuthorityServices,
    services: *const Services,
    costs: *const crate::services::CommandCostServices,
    result: *mut crate::services::CommandCost,
    tile: u32,
    no_test: bool,
) {
    // SAFETY: Native immutable tables live for this synchronous invocation.
    let (a, s, c) = unsafe { (&*authority, &*services, &*costs) };
    if (a.council_tolerance)() == 3 {
        (c.zero)(result);
        return;
    }
    let company = (s.current_company)();
    if !(s.company_valid)(company) || no_test {
        (c.zero)(result);
        return;
    }
    let town = crate::town::spatial::closest(s, tile, u32::from((a.authority_distance)()));
    if town.is_null() {
        (c.zero)(result);
        return;
    }
    let owner = (s.town_owner)(town);
    // SAFETY: Live single owner and valid company; immediate copy ends here.
    let rating = unsafe { (*owner).simulation.ratings[usize::from(company)] };
    if rating > -200 {
        (c.zero)(result);
    } else {
        (c.error_param)(result, a.refusal, u64::from((a.town_index)(town)));
    }
}

/// Original destructive-action rating check, including scratch-rating precedence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_authority_remove(
    authority: *const AuthorityServices,
    services: *const Services,
    costs: *const crate::services::CommandCostServices,
    result: *mut crate::services::CommandCost,
    town: *const crate::services::TownHandle,
    no_test: bool,
    kind: u8,
) {
    // SAFETY: Native immutable tables and stack-owned result outlive this call.
    let (a, s, c) = unsafe { (&*authority, &*services, &*costs) };
    if town.is_null() {
        (c.zero)(result);
        return;
    }
    let company = (s.current_company)();
    if !(s.company_valid)(company) || (a.bulldozer)() || no_test {
        (c.zero)(result);
        return;
    }
    let needed = [[16, 144], [64, 208], [112, 400], [-1000, -1000]]
        [usize::from((a.council_tolerance)())][usize::from(kind)];
    let owner = (s.town_owner)(town);
    // SAFETY: Live owner, valid company; scratch/native rating access ends here.
    let rating = unsafe { openttd_rust_town_rating(owner, town as usize, company) };
    if rating < needed {
        (c.error_param)(result, a.refusal, u64::from((a.town_index)(town)));
    } else {
        (c.zero)(result);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub map_log_x: extern "C" fn() -> u32,
    pub growth_rate: extern "C" fn() -> u8,
    pub environment: crate::services::EnvironmentServices,
    pub generating_world: extern "C" fn() -> bool,
    pub allow_crossings: extern "C" fn() -> bool,
    pub climate: extern "C" fn() -> u8,
    pub road_flags: extern "C" fn(u8) -> u8,
    pub house_availability: extern "C" fn(u16) -> HouseAvailability,
    pub house_placement: extern "C" fn(u16) -> HousePlacement,
    pub house_population: extern "C" fn(u16) -> u8,
    pub house_flags: extern "C" fn(u16) -> u8,
    pub house_extra_flags: extern "C" fn(u16) -> u8,
    pub house_callbacks: extern "C" fn(u16) -> u16,
    pub road_info: extern "C" fn(u8) -> RoadChoice,
    pub resolve_owner: extern "C" fn(u32) -> *mut State,
    pub dirty_authority: extern "C" fn(u32),
    pub dirty_town: extern "C" fn(u32),
    pub terraform_price: extern "C" fn() -> i64,
    pub set_road_town: extern "C" fn(u32, u32),
    pub exchange_company: extern "C" fn(u32) -> u32,
    pub house_class: extern "C" fn(u16) -> u16,
    pub make_house: extern "C" fn(u32, u32, u32, u32),

    pub population_changed: extern "C" fn(u32),
    pub house_count: extern "C" fn() -> u32,
    pub road_type_error: extern "C" fn(i32),
    pub next_town: extern "C" fn(u32) -> u32,
    pub build_bridge: extern "C" fn(u32, u32, u32, u32, u32) -> bool,
    pub build_tunnel: extern "C" fn(u32, u32, u32) -> bool,
    pub assert_clear_house: extern "C" fn(bool),
    pub houses: *const house::HouseServices,
}
pub type StationActivity = crate::station_service::StationActivity;
#[derive(Clone, Copy)]
pub struct CommandResult {
    pub cost: i64,
    pub success: bool,
}
/// `owner` is the town's live state for the entry; field accesses end before services.
#[derive(Clone)]
struct Engine<'a> {
    road: *const crate::road_cmd::CommandContext,
    leaves: &'a Leaves,
    services: &'a Services,
    map: OnceCell<Map>,
    owner: *mut State,
    house_shell: OnceCell<*mut crate::services::TownHandle>,
}
impl Engine<'_> {
    fn map(&self) -> Map {
        *self
            .map
            .get_or_init(|| Map::from_log_x((self.leaves.map_log_x)()))
    }
    fn size_x(&self) -> u32 {
        self.map().size_x()
    }
    fn distance_from_edge(&self, tile: u32) -> u32 {
        self.map()
            .distance_from_edge(tile, (self.services.map_max_y)() + 1)
    }
    fn position(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { position(self.owner) }
    }
    fn num_houses(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { num_houses(self.owner) }
    }
    fn change_house_count(&self, amount: u32) {
        write_field!(
            self.owner,
            simulation.num_houses,
            self.num_houses().wrapping_add(amount)
        );
    }
    fn population(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { population(self.owner) }
    }
    fn layout(&self) -> u8 {
        read_field!(self.owner, simulation.layout)
    }
    fn is_city(&self) -> bool {
        read_field!(self.owner, simulation.larger_town)
    }
    fn radius(&self) -> [u32; 5] {
        read_field!(self.owner, simulation.radius)
    }
    fn goals(&self) -> [u32; 6] {
        read_field!(self.owner, simulation.goal)
    }
    fn old_received(&self, at: usize) -> u32 {
        // SAFETY: Source bounds and immediate owner access before any callback.
        unsafe { u32::from((*self.owner).simulation.received[at].old_act) }
    }
    fn bind(mut self, town: u32) -> Self {
        self.owner = (self.leaves.resolve_owner)(town);
        self
    }
    fn modify_flags(&self, clear: u16, set: u16) {
        self.set_flags((self.flags() & !clear) | set);
    }
    fn counter(&self) -> u16 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { (*self.owner).growth.counter }
    }
    fn set_counter(&self, value: u16) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.counter = value;
        }
    }
    fn rate(&self) -> u16 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { (*self.owner).growth.rate }
    }
    fn set_rate(&self, value: u16) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.rate = value;
        }
    }
    fn funding(&self) -> u16 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { u16::from((*self.owner).growth.funding) }
    }
    fn set_funding(&self, value: u16) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.funding = value as u8;
        }
    }
    fn flags(&self) -> u16 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { u16::from((*self.owner).growth.flags) }
    }
    fn set_flags(&self, value: u16) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.flags = value as u8;
        }
    }
    fn chance(&self, a: u32, b: u32) -> bool {
        chance16_i(a, b, self.services.random())
    }
    fn dir(&self) -> u32 {
        self.services.random_range(4)
    }
    fn offset(&self, tile: u32, x: i32, y: i32) -> u32 {
        tile.wrapping_add(self.map().diff(x, y))
    }
    fn step(&self, tile: u32, dir: u32) -> u32 {
        self.map().add_diag(tile, dir as u8)
    }
    fn valid(&self, tile: u32) -> bool {
        (self.services.tile_valid)(tile)
    }
    fn rb(&self, tile: u32) -> u32 {
        if (self.services.is_road_depot)(tile) || (self.services.is_bay_stop)(tile) {
            0
        } else {
            u32::from((self.services.road_bits)(tile, 0, true))
        }
    }
    fn gridpos(&self, _town: u32, tile: u32) -> (i32, i32) {
        let center = self.position();
        let sx = self.size_x();
        (
            (center % sx) as i32 - (tile % sx) as i32,
            (center / sx) as i32 - (tile / sx) as i32,
        )
    }
    fn distance(&self, a: u32, b: u32) -> u32 {
        self.map().distance_square(a, b)
    }
    fn build_road(&self, town: u32, tile: u32, bits: u32, rt: u32, mode: u32) -> bool {
        let mut flags = 2;
        if mode != 2 {
            flags |= 8;
        }
        if mode != 0 {
            flags |= 1;
        }
        // SAFETY: growth entries supply a live outer native stack context.
        unsafe { (&*self.road).build(flags, tile, bits as u8, rt as u8, town as u16) }
    }
    fn road(&self, town: u32, tile: u32, bits: u32, flags: u32) -> bool {
        let rt = self.road_type();
        self.build_road(town, tile, bits, rt, flags)
    }
    fn landscape(
        &self,
        tile: u32,
        slope: u8,
        up: bool,
        execute: bool,
        clear: bool,
    ) -> CommandResult {
        let commands = unsafe { &*(*self.leaves.houses).commands };
        let table = unsafe { &*(commands.landscape_commands)() };
        let costs = unsafe { &*table.costs };
        unsafe {
            /* SAFETY: synchronous costs never escape or outlive this call. */
            crate::command::with_cost(costs, |out| {
                let flags = crate::command_type::DoCommandFlag::AUTO
                    | crate::command_type::DoCommandFlag::NO_WATER
                    | u16::from(execute);
                if clear {
                    crate::landscape_cmd::clear_command(commands, out.0, tile, flags);
                } else {
                    crate::landscape_cmd::terraform_command(
                        commands, out.0, tile, slope, up, flags,
                    );
                }
                CommandResult {
                    cost: out.money(),
                    success: out.ok(),
                }
            })
        }
    }
    fn clear(&self, tile: u32, execute: bool) -> bool {
        self.landscape(tile, 0, false, execute, true).success
    }
    fn count_stations(&self, _town: u32) -> u32 {
        let center = self.position();
        let radius = self.radius()[0];
        let c = unsafe { &*(self.services.station_catchment)() };
        let mut count = 0_u32;
        crate::station::spatial::radius(self.services, center, radius / 2, |st| {
            if self.distance((c.location)(st), center) <= radius {
                let activity = unsafe { crate::station_service::activity((c.owner)(st)) };
                if activity.time_since_load <= 20 || activity.time_since_unload <= 20 {
                    count = count.wrapping_add(1);
                }
            }
        });
        count
    }
    fn update_rate(&self, town: u32) {
        if self.flags() & 8 != 0 {
            return;
        }
        let old = self.rate();
        let rates = [
            [120_u32, 120, 120, 100, 80, 60],
            [320, 420, 300, 220, 160, 100],
        ];
        let funding = self.funding() != 0;
        let mut m = rates[usize::from(!funding)][self.count_stations(town).min(5) as usize];
        m >>= if u32::from((self.leaves.growth_rate)()) == 0 {
            1
        } else {
            u32::from((self.leaves.growth_rate)()) - 1
        };
        if self.is_city() {
            m /= 2;
        }
        let rate = town_ticks_to_game_ticks((m / (self.num_houses() / 50 + 1)) as u16);
        self.set_rate(rate);
        if rate != u16::MAX {
            let counter = self.counter();
            self.set_counter(if old == u16::MAX {
                rate.min(counter)
            } else {
                let numerator = u32::from(counter) * (u32::from(rate) + 1);
                round_div(numerator as i32, u32::from(old) + 1) as u16
            });
        }
        (self.leaves.dirty_town)(town);
    }
    fn update_growth(&self, town: u32) {
        self.update_rate(town);
        self.modify_flags(1, 0);
        (self.leaves.dirty_town)(town);
        let funding = self.funding() != 0;
        if u32::from((self.leaves.growth_rate)()) == 0 && !funding {
            return;
        }
        if !funding {
            for i in 0..6 {
                match self.goals()[i] {
                    0xffff_fffe => {
                        if crate::tile_map::tile_height_in(
                            &crate::map_func::World::new(self.services),
                            self.position(),
                        ) >= u32::from(crate::snow_line::get(self.services))
                            && self.old_received(i) == 0
                            && self.population() > 90
                        {
                            return;
                        }
                    }
                    u32::MAX => {
                        if (self.services.tropic_zone)(self.position()) == 1
                            && self.old_received(i) == 0
                            && self.population() > 60
                        {
                            return;
                        }
                    }
                    goal => {
                        if goal > self.old_received(i) {
                            return;
                        }
                    }
                }
            }
        }
        if self.flags() & 8 != 0 {
            if self.rate() != u16::MAX {
                self.modify_flags(0, 1);
            }
            (self.leaves.dirty_town)(town);
            return;
        }
        if !funding && self.count_stations(town) == 0 && !self.chance(1, 12) {
            return;
        }
        self.modify_flags(0, 1);
        (self.leaves.dirty_town)(town);
    }
}
fn reverse(dir: u32) -> u32 {
    (dir + 2) & 3
}
fn bit(dir: u32) -> u32 {
    1 << (3 - dir)
}
fn incline(dir: u32) -> u32 {
    [12, 6, 3, 9][dir as usize]
}

fn engine<'a>(leaves: *const Leaves, services: *const Services) -> Engine<'a> {
    // SAFETY: Immutable native service tables remain live for each synchronous entry.
    let leaves = unsafe { &*leaves };
    let services = unsafe { &*services };
    Engine {
        road: std::ptr::null(),
        leaves,
        services,
        map: OnceCell::new(),
        owner: std::ptr::null_mut(),
        house_shell: OnceCell::new(),
    }
}
fn round_div(a: i32, b: u32) -> i32 {
    if a > 0 {
        (a + (b as i32) / 2) / (b as i32)
    } else {
        (a - ((b as i32) - 1) / 2) / (b as i32)
    }
}

impl Engine<'_> {
    fn road_type(&self) -> u32 {
        let mut mask = crate::road::get_mask_for_road_tram_type(crate::road_type::RTT_ROAD);
        let mut best = 0;
        let mut best_speed = None;
        while mask != 0 {
            let rt = mask.trailing_zeros();
            mask &= mask - 1;
            let rti = (self.leaves.road_info)(rt as u8);
            if !rti.town_build {
                continue;
            }
            let date = rti.introduction_date;
            if (0..1_826_212_865).contains(&date)
                && date > crate::timer_game_common::calendar_date().0
            {
                continue;
            }
            let speed = if rti.max_speed == 0 {
                50
            } else {
                rti.max_speed
            };
            if best_speed.is_some_and(|previous| speed < previous) {
                continue;
            }
            best = rt;
            best_speed = Some(speed);
        }
        best
    }
    fn neighbour_road(&self, tile: u32, dir: u32, distance: u32) -> bool {
        if !self.valid(tile) {
            return false;
        }
        let left = self.step(tile, (dir + 1) & 3).wrapping_sub(tile);
        let right = self.step(tile, (dir + 3) & 3).wrapping_sub(tile);
        let back = self.step(tile, reverse(dir)).wrapping_sub(tile);
        for pos in 4..(distance + 1) * 4 {
            let mut diff = if pos & 1 != 0 { left } else { right }.wrapping_mul(pos / 4);
            if pos & 2 != 0 {
                diff = diff.wrapping_add(back);
            }
            let test = tile.wrapping_add(diff);
            if self.valid(test)
                && self.rb(test) & bit(if pos & 2 != 0 { dir } else { reverse(dir) }) != 0
            {
                return true;
            }
        }
        false
    }
    fn road_allowed(&self, town: u32, tile: u32, dir: u32) -> bool {
        if self.distance_from_edge(tile) == 0
            || (crate::map_func::World::new(self.services).bridge_above(tile)
                && u32::from((self.services.bridge_axis)(tile)) == (dir & 1))
        {
            return false;
        }
        if self.rb(tile) == 0
            && !self.road(town, tile, if dir & 1 != 0 { 5 } else { 10 }, 0)
            && !self.clear(tile, false)
        {
            return false;
        }
        let slope = if (self.services.build_on_slopes)() {
            u32::from(
                crate::movement::foundation_slope(self.services, tile)
                    .slope
                    .0,
            )
        } else {
            u32::from((self.services.tile_slope)(tile))
        };
        let ret = !self.neighbour_road(tile, dir, if self.layout() == 0 { 1 } else { 2 });
        if slope == 0 {
            return ret;
        }
        let desired = if dir & 1 != 0 { 9 } else { 12 };
        if desired != slope && (desired ^ 15) != slope {
            if self.chance(1, 8) {
                let mut success = false;
                if !(self.leaves.generating_world)() && self.chance(1, 10) {
                    let edges = if self.chance(1, 16) {
                        slope
                    } else {
                        slope ^ 15
                    };
                    success = self
                        .landscape(tile, edges as u8, false, true, false)
                        .success;
                }
                if !success && self.chance(1, 3) {
                    return ret;
                }
            }
            return false;
        }
        ret
    }
    fn terraform(&self, tile: u32, edges: u32, up: bool) -> bool {
        let CommandResult { success, cost } = self.landscape(tile, edges as u8, up, false, false);
        if !success || cost >= ((self.leaves.terraform_price)() + 2) * 8 {
            return false;
        }
        self.landscape(tile, edges as u8, up, true, false);
        true
    }
    fn level(&self, tile: u32) {
        if (self.services.tile_type)(tile) == MP_HOUSE {
            return;
        }
        let slope = u32::from((self.services.tile_slope)(tile));
        if slope == 0 {
            return;
        }
        if !self.terraform(tile, (!slope) & 15, true) {
            self.terraform(tile, slope & 15, false);
        }
    }
    fn grid(&self, town: u32, tile: u32, dir: u32) -> u32 {
        let (x, y) = self.gridpos(town, tile);
        let spacing = if self.layout() == 2 { 3 } else { 4 };
        let bits = if x % spacing == 0 { 5 } else { 0 } | if y % spacing == 0 { 10 } else { 0 };
        if bits != 15 {
            return bits;
        }
        let template = match u32::from((self.services.tile_slope)(tile)) {
            1 => 3,
            3 => 7,
            2 => 6,
            6 => 14,
            4 => 12,
            12 => 13,
            8 => 9,
            9 => 11,
            23 | 27 | 29 | 30 => 0,
            _ => 15,
        };
        if bit(reverse(dir)) & template != 0 {
            template
        } else {
            bit(dir) | bit(reverse(dir))
        }
    }
    fn extra_house(&self, town: u32, tile: u32, modes: u32) -> bool {
        if self.distance_from_edge(tile) == 0 {
            return false;
        }
        let mut count = 0;
        for dir in 0..4 {
            if matches!(
                (self.services.tile_type)(self.step(tile, dir)),
                MP_HOUSE | MP_VOID
            ) {
                count += 1;
            }
            if count >= 3 {
                return self.try_house(town, tile, modes);
            }
        }
        false
    }
    fn road_continue(&self, town: u32, tile: u32, dir: u32) -> bool {
        let next = self.step(tile, dir);
        let rt = self.road_type();
        if !self.valid(next) {
            return false;
        }
        match (self.services.tile_type)(next) {
            MP_TUNNELBRIDGE => {
                return (self.services.tunnel_transport)(next) == 1
                    && u32::from((self.services.tunnel_bridge_direction)(next)) == dir;
            }
            MP_STATION => {
                if (self.services.is_drive_through_stop)(next) {
                    return u32::from((self.services.drive_through_axis)(next)) == dir & 1;
                }
                if (self.services.is_bay_stop)(next) {
                    return u32::from((self.services.bay_stop_direction)(next)) == reverse(dir);
                }
                return false;
            }
            MP_ROAD => {
                return (self.services.is_road_depot)(next)
                    && u32::from((self.services.road_depot_direction)(next)) == reverse(dir);
            }
            MP_RAILWAY => {
                if !(self.leaves.allow_crossings)() {
                    return false;
                }
            }
            _ => {}
        }
        self.build_road(town, next, bit(reverse(dir)), rt, 0)
    }
    fn bridge(&self, town: u32, tile: u32, dir: u32) -> bool {
        let slope = u32::from((self.services.tile_slope)(tile));
        if slope != 0 && slope & incline(dir) != 0 {
            return false;
        }
        if self.rb(self.step(tile, reverse(dir))) & bit(dir) == 0 {
            return false;
        }
        let max = (self.population() / 1000 + 5).min(11);
        let mut length = 0;
        let mut end = tile;
        loop {
            let old = length;
            length += 1;
            if old >= if slope == 0 { 5 } else { max } {
                return false;
            }
            end = self.step(end, dir);
            if !self.valid(end)
                || !(((self.services.is_water_tile)(end)
                    && (slope != 0 || !crate::map_func::World::new(self.services).is_sea(end)))
                    || (self.services.is_plain_rail)(end)
                    || (crate::map_func::World::new(self.services).is_normal_road_tile(end)
                        && crate::map_func::World::new(self.services)
                            .disallowed_road_directions(end)
                            != 0))
            {
                break;
            }
        }
        if length == 1 || !self.road_continue(town, end, dir) {
            return false;
        }
        if slope != 0 {
            let mut spiral = crate::spiral::hole(
                tile % self.size_x(),
                tile / self.size_x(),
                length,
                0,
                0,
                self.size_x(),
                (self.services.map_max_y)().wrapping_add(1),
            );
            while !spiral.is_end() {
                let at = spiral.x + spiral.y * self.size_x();
                if (self.services.is_bridge)(at)
                    && (self.services.tunnel_transport)(at) == 1
                    && u32::from((self.services.tile_slope)(at)) & incline(reverse(dir)) != 0
                {
                    return false;
                }
                spiral = spiral.advance(self.size_x(), (self.services.map_max_y)().wrapping_add(1));
            }
        }
        for _ in 0..=22 {
            let bridge = self.services.random_range(13 - 1);
            let rt = self.road_type();
            if (self.leaves.build_bridge)(tile, end, bridge, rt, 0) {
                (self.leaves.build_bridge)(tile, end, bridge, rt, 1);
                return true;
            }
        }
        false
    }
    fn tunnel(&self, town: u32, tile: u32, dir: u32) -> bool {
        if u32::from((self.services.tile_slope)(tile)) != incline(dir)
            || self.rb(self.step(tile, reverse(dir))) & bit(dir) == 0
        {
            return false;
        }
        let max = if self.road_continue(town, tile, dir) {
            let mut test = tile;
            for _ in 0..4 {
                if !self.valid(test) {
                    return false;
                }
                let slope = (self.services.tile_slope)(test);
                if u32::from(slope) != incline(dir)
                    && !is_steep_slope(slope)
                    && !is_slope_with_one_corner_raised(slope)
                {
                    return false;
                }
                test = self.step(test, dir);
            }
            self.population() / 1000 + 7
        } else {
            5
        };
        let mut length = 0_u8;
        let mut end = tile;
        loop {
            let old = length;
            length = length.wrapping_add(1);
            if u32::from(old) >= max {
                return false;
            }
            end = self.step(end, dir);
            if !self.valid(end) || (self.services.tile_z)(tile) == (self.services.tile_z)(end) {
                break;
            }
        }
        if length == 1 || !self.road_continue(town, end, dir) {
            return false;
        }
        let rt = self.road_type();
        if (self.leaves.build_tunnel)(tile, rt, 0) {
            (self.leaves.build_tunnel)(tile, rt, 1);
            return true;
        }
        false
    }
    fn roads_allow_house(&self, tile: u32) -> bool {
        let mut found = false;
        for (x, y) in [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ] {
            let at = self.offset(tile, x, y);
            if !self.valid(at)
                || !((self.services.tile_type)(at) == MP_ROAD
                    || crate::map_func::World::new(self.services).any_road_stop_tile(at))
            {
                continue;
            }
            found = true;
            let road_type = (self.services.road_type)(at);
            if road_type != 63 && (self.leaves.road_flags)(road_type) & 4 == 0 {
                return true;
            }
        }
        !found
    }
    fn grow_road(&self, tile: u32) -> bool {
        if (self.services.tile_type)(tile) != MP_ROAD {
            return true;
        }
        let road_type = (self.services.road_type)(tile);
        (self.leaves.road_flags)(road_type) & 16 != 0 || u32::from(road_type) == self.road_type()
    }
    fn can_follow(&self, tile: u32, dir: u32, modes: u32) -> bool {
        let next = self.step(tile, dir);
        if !self.valid(next) || (self.services.has_water_ground)(next) {
            return false;
        }
        let bits = self.rb(next);
        if modes & 2 != 0 {
            match (self.services.tile_type)(next) {
                MP_ROAD => bits != 0,
                MP_STATION => (self.services.is_drive_through_stop)(next),
                MP_TUNNELBRIDGE => (self.services.tunnel_transport)(next) == 1,
                MP_HOUSE | MP_INDUSTRY | MP_OBJECT => false,
                _ => true,
            }
        } else {
            let back = bit(reverse(dir));
            bits & back != 0 && bits & !back != 0
        }
    }
}

impl Engine<'_> {
    fn in_tile(&self, town: u32, tile: &mut u32, cur: u32, mut dir: u32, modes: u32) -> u32 {
        let at = *tile;
        let layout = self.layout();
        let mut bits;
        if cur == 0 {
            if modes & 2 == 0
                || (!(self.leaves.allow_crossings)() && (self.services.tile_type)(at) == MP_RAILWAY)
            {
                return 1;
            }
            if !(self.services.build_on_slopes)() || self.chance(1, 6) {
                self.level(at);
            }
            if layout >= 2 {
                bits = self.grid(town, at, dir);
                if bits == 0 {
                    return 1;
                }
            } else {
                if !self.road_allowed(town, at, dir) {
                    return 1;
                }
                let source = reverse(dir);
                if self.chance(1, 4) {
                    loop {
                        dir = self.dir();
                        if dir != source {
                            break;
                        }
                    }
                }
                if !self.road_allowed(town, self.step(at, dir), dir) {
                    if dir != reverse(source) {
                        return 1;
                    }
                    if (self.services.tile_type)(self.step(at, (dir + 1) & 3)) != MP_HOUSE
                        && (self.services.tile_type)(self.step(at, (dir + 3) & 3)) != MP_HOUSE
                    {
                        return 1;
                    }
                }
                bits = bit(dir) | bit(source);
            }
        } else if dir < 4 && cur & bit(reverse(dir)) == 0 {
            if !self.grow_road(at) {
                return 2;
            }
            if modes & 2 == 0 {
                return 1;
            }
            bits = if layout >= 2 {
                self.grid(town, at, dir)
            } else {
                bit(reverse(dir))
            };
        } else {
            let mut allow = true;
            if (self.services.tile_type)(at) == MP_TUNNELBRIDGE {
                if (self.services.tunnel_transport)(at) == 1 && (dir != 4 || self.chance(1, 2)) {
                    *tile = (self.services.other_tunnel_end)(at);
                }
                return 2;
            }
            dir = self.dir();
            let target = bit(dir);
            let house_tile = if cur & target != 0 {
                if cur & 10 != target {
                    return 2;
                }
                let house = match cur {
                    9 => self.offset(at, 1, 1),
                    6 => self.offset(at, -1, -1),
                    12 => self.offset(at, 1, -1),
                    3 => self.offset(at, -1, 1),
                    _ => return 2,
                };
                dir = 4;
                house
            } else {
                self.step(at, dir)
            };
            if (self.services.has_water_ground)(house_tile) {
                return 2;
            }
            if !self.valid(house_tile) {
                return 2;
            }
            let mut result = 2;
            bits = 0;
            if dir != 4 && modes & 2 != 0 {
                if layout == 3 || layout == 1 {
                    if self.extra_house(town, self.step(house_tile, dir), modes) {
                        result = 0;
                    }
                }
                if layout >= 2 {
                    bits = self.grid(town, at, dir);
                    allow = bits & target == 0;
                } else {
                    bits = target;
                    allow = !self.road_allowed(town, house_tile, dir) || self.chance(6, 10);
                }
            }
            allow &= self.roads_allow_house(house_tile);
            if allow {
                if (self.services.tile_type)(house_tile) != MP_HOUSE {
                    if self.chance(1, 6) {
                        self.level(house_tile);
                    }
                    if self.try_house(town, house_tile, modes) {
                        result = 0;
                    }
                }
                return result;
            }
            if !self.grow_road(at) {
                return result;
            }
        }
        if (self.services.has_water_ground)(at) {
            return 1;
        }
        bits = u32::from(
            crate::road_cleanup::clean_up_road_bits(
                self.services,
                at,
                crate::services::RoadBits(bits as u8),
            )
            .0,
        );
        if bits == 0 {
            return 1;
        }
        if self.bridge(town, at, dir) || self.tunnel(town, at, dir) || self.road(town, at, bits, 1)
        {
            return 0;
        }
        1
    }
    fn at_road(&self, town: u32, mut tile: u32, modes: u32) -> bool {
        let factor = match self.layout() {
            1 => 2,
            2 | 3 => 1,
            _ => 4,
        };
        let mut iterations = (10 + self.num_houses().wrapping_mul(factor) / 9) as i32;
        let mut dir = 4;
        loop {
            let mut cur = self.rb(tile);
            match self.in_tile(town, &mut tile, cur, dir, modes) {
                0 => return true,
                1 => iterations = 0,
                _ => {}
            }
            if dir < 4 {
                cur &= !bit(reverse(dir));
            }
            if cur == 0 {
                return false;
            }
            if (self.services.tile_type)(tile) == MP_TUNNELBRIDGE {
                dir = reverse(u32::from((self.services.tunnel_bridge_direction)(tile)));
            } else {
                loop {
                    if cur == 0 {
                        return false;
                    }
                    let target = loop {
                        dir = self.dir();
                        let bits = bit(dir);
                        if cur & bits != 0 {
                            break bits;
                        }
                    };
                    cur &= !target;
                    if self.can_follow(tile, dir, modes) {
                        break;
                    }
                }
            }
            tile = self.step(tile, dir);
            if (self.services.tile_type)(tile) == MP_ROAD
                && !(self.services.is_road_depot)(tile)
                && (self.services.has_road_type)(tile)
            {
                let owner = crate::map_func::World::new(self.services).road_owner(tile, false);
                if owner == 15 && u32::from((self.services.town_index)(tile)) != town {
                    return false;
                }
                if owner == 16 && (self.leaves.environment.editor)() {
                    (self.leaves.set_road_town)(tile, town);
                }
            }
            iterations -= 1;
            if iterations < 0 {
                break;
            }
        }
        false
    }
    fn grow(&self, town: u32, modes: u32) -> bool {
        const OFFSETS: [(i32, i32); 13] = [
            (-1, 0),
            (1, 1),
            (1, -1),
            (-1, -1),
            (-1, 0),
            (0, 2),
            (2, 0),
            (0, -2),
            (-1, -1),
            (-2, 2),
            (2, 2),
            (2, -2),
            (0, 0),
        ];
        // Ordinary shared commands are noexcept; restore the original company on each return.
        let company = (self.leaves.exchange_company)(15);
        let mut tile = self.position();
        for (x, y) in OFFSETS {
            if self.rb(tile) != 0 {
                let result = self.at_road(town, tile, modes);
                (self.leaves.exchange_company)(company);
                return result;
            }
            tile = self.offset(tile, x, y);
        }
        if modes & 2 != 0 {
            tile = self.position();
            for (x, y) in OFFSETS {
                if (self.services.tile_type)(tile) != MP_HOUSE
                    && (self.services.tile_slope)(tile) == 0
                    && self.clear(tile, false)
                {
                    let rt = self.road_type();
                    let r = self.services.random();
                    let a = r & 3;
                    let mut b = (r >> 8) & 3;
                    if a == b {
                        b ^= 2;
                    }
                    self.build_road(town, tile, (1 << a) + (1 << b), rt, 2);
                    (self.leaves.exchange_company)(company);
                    return true;
                }
                tile = self.offset(tile, x, y);
            }
        }
        (self.leaves.exchange_company)(company);
        false
    }
    fn layout_house(&self, town: u32, tile: u32, modes: u32, square: bool) -> bool {
        if modes & 1 == 0 {
            return false;
        }
        if modes & 2 == 0 {
            return true;
        }
        let (x, y) = self.gridpos(town, tile);
        match self.layout() {
            2 => {
                if square {
                    matches!(x % 3, 2 | -1) && matches!(y % 3, 2 | -1)
                } else {
                    x % 3 != 0 && y % 3 != 0
                }
            }
            3 => {
                if square {
                    x & 3 >= 2 && y & 3 >= 2
                } else {
                    x % 4 != 0 && y % 4 != 0
                }
            }
            _ => true,
        }
    }
    fn can_house(&self, tile: u32, noslope: bool) -> bool {
        let slope = (self.services.tile_slope)(tile);
        if (noslope && slope != 0) || is_steep_slope(slope) {
            return false;
        }
        if !self.roads_allow_house(tile)
            || crate::map_func::World::new(self.services).bridge_above(tile)
        {
            return false;
        }
        self.clear(tile, false)
    }
    fn same_z(&self, tile: u32, z: i32, noslope: bool) -> bool {
        self.can_house(tile, noslope) && (self.services.tile_max_z)(tile) == z
    }
    fn free_square(&self, mut tile: u32, z: i32, noslope: bool) -> bool {
        if !self.same_z(tile, z, noslope) {
            return false;
        }
        for dir in 1..4 {
            tile = self.step(tile, dir);
            if !self.same_z(tile, z, noslope) {
                return false;
            }
        }
        true
    }
    fn check_house(
        &self,
        town: u32,
        tile: &mut u32,
        z: i32,
        noslope: bool,
        size: u32,
        modes: u32,
    ) -> bool {
        if size & 16 != 0 {
            let mut test = *tile;
            for dir in 1..=4 {
                if self.layout_house(town, test, modes, true) && self.free_square(test, z, noslope)
                {
                    *tile = test;
                    return true;
                }
                if dir == 4 {
                    break;
                }
                test = self.step(test, reverse(dir));
            }
            false
        } else if size & 12 != 0 {
            let dir = if size & 4 != 0 { 2 } else { 1 };
            let mut test = self.step(*tile, dir);
            if self.layout_house(town, test, modes, false) && self.same_z(test, z, noslope) {
                return true;
            }
            test = self.step(*tile, reverse(dir));
            if self.layout_house(town, test, modes, false) && self.same_z(test, z, noslope) {
                *tile = test;
                return true;
            }
            false
        } else {
            true
        }
    }
    fn make_house_tile(
        &self,
        town: u32,
        tile: u32,
        counter: u32,
        stage: u32,
        house: u32,
        random: u32,
        protected: bool,
    ) {
        (self.leaves.assert_clear_house)(self.landscape(tile, 0, false, true, true).success);
        let class = (self.leaves.house_class)(house as u16);
        // SAFETY: Entry resolved the live town once. Count borrows end here,
        // before map writes or callbacks can reenter and mutate the owner.
        unsafe { increase_building(self.owner, house as u16, class) };
        (self.leaves.make_house)(
            tile,
            town,
            house,
            counter | (stage << 8) | (random << 16) | (u32::from(protected) << 24),
        );
        if u32::from((self.leaves.house_flags)(house as u16)) & 32 != 0 {
            crate::animated_tile::openttd_rust_animation_add(tile, false);
        }
        (self.services.mark_dirty)(tile);
    }
    fn build_house(
        &self,
        town: u32,
        tile: u32,
        house: u32,
        random: u32,
        complete: bool,
        protected: bool,
    ) {
        // SAFETY: Entry bound its owner once; this mutation ends before spec queries.
        unsafe {
            (*self.owner).simulation.num_houses = self.num_houses().wrapping_add(1);
        }

        let mut counter = 0;
        let mut stage = 0;
        if (self.leaves.generating_world)() || (self.leaves.environment.editor)() || complete {
            let construction = self.services.random();
            stage = 3;
            if (self.leaves.generating_world)()
                && (self.leaves.house_extra_flags)(house as u16) & 1 == 0
                && self.chance(1, 7)
            {
                stage = construction & 3;
            }
            if stage == 3 {
                // SAFETY: Owner mutation ends before possible presentation reentry.
                unsafe {
                    change_population(
                        self.leaves,
                        self.owner,
                        town,
                        i32::from((self.leaves.house_population)(house as u16)),
                    );
                }
            } else {
                counter = (construction >> 2) & 3;
            }
        }
        let size = u32::from((self.leaves.house_flags)(house as u16));
        let mut part = house;
        self.make_house_tile(town, tile, counter, stage, part, random, protected);
        if size & 24 != 0 {
            part += 1;
            self.make_house_tile(
                town,
                self.offset(tile, 0, 1),
                counter,
                stage,
                part,
                random,
                protected,
            );
        }
        if size & 20 != 0 {
            part += 1;
            self.make_house_tile(
                town,
                self.offset(tile, 1, 0),
                counter,
                stage,
                part,
                random,
                protected,
            );
        }
        if size & 16 != 0 {
            part += 1;
            self.make_house_tile(
                town,
                self.offset(tile, 1, 1),
                counter,
                stage,
                part,
                random,
                protected,
            );
        }
        let flags = size as u8;
        let area = crate::tilearea::TileArea {
            tile,
            w: if flags & crate::house::BUILDING_2_TILES_X != 0 {
                2
            } else {
                1
            },
            h: if flags & crate::house::BUILDING_2_TILES_Y != 0 {
                2
            } else {
                1
            },
        };
        let list = stations_near(self.owner);
        crate::station::for_all_stations_around_tiles(
            area,
            &crate::station::Ctx::from_shared(self.services),
            |near, _| {
                // SAFETY: Live town owner; immediate mutation.
                unsafe { (*list).insert(near.station, near.id) };
                true
            },
        );
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe {
            update_radius(self.owner);
        }
        self.update_rate(town);
        let size = u32::from((self.leaves.house_flags)(house as u16));
        // Resolve only the native shell at the first construction trigger; the
        // entry already resolved its owner. House lifecycle callers also supplied
        // their shell. No owner reference spans any construction callback.
        let shell = *self.house_shell.get_or_init(|| {
            // SAFETY: Static house table referenced by the town leaves.
            (self.services.town_by_id)(town)
        });
        let town_ref = TownRef {
            shell,
            state: self.owner,
            id: TownId(town as u16),
        };
        house::stage_changed(self.leaves, self.services, town_ref, tile);
        if size & 24 != 0 {
            house::stage_changed(
                self.leaves,
                self.services,
                town_ref,
                self.offset(tile, 0, 1),
            );
        }
        if size & 20 != 0 {
            house::stage_changed(
                self.leaves,
                self.services,
                town_ref,
                self.offset(tile, 1, 0),
            );
        }
        if size & 16 != 0 {
            house::stage_changed(
                self.leaves,
                self.services,
                town_ref,
                self.offset(tile, 1, 1),
            );
        }
    }
    fn try_house(&self, town: u32, base: u32, modes: u32) -> bool {
        if !self.layout_house(town, base, modes, false) || !self.can_house(base, false) {
            return false;
        }
        let slope = u32::from((self.services.tile_slope)(base));
        let maxz = (self.services.tile_max_z)(base);
        let distance = self.distance(self.position(), base);
        let zone = u32::from(radius_group(self.funding() != 0, &self.radius(), distance));
        let climate = match (self.leaves.climate)() {
            0 => 12,
            1 => {
                if maxz > i32::from(crate::snow_line::highest(self.services)) {
                    11
                } else {
                    13
                }
            }
            2 => 14,
            _ => 15,
        };
        let zones = (1 << zone) | (1 << climate);
        clear_house_candidates();
        let mut max = 0_u32;
        for house in 0..(self.leaves.house_count)() {
            let h = (self.leaves.house_availability)(house as u16);
            if u32::from(h.zones) & zones != zones || !h.enabled || h.override_id != u16::MAX {
                continue;
            }
            // SAFETY: Original valid spec/count indices; immediate read ends
            // before the next spec query or any construction callback.
            let count = unsafe {
                if h.class_id != 0 {
                    (&(*self.owner).class_counts)[usize::from(h.class_id)]
                } else {
                    (&(*self.owner).id_counts)[house as usize]
                }
            };
            if count == u16::MAX {
                continue;
            }
            max = max.wrapping_add(u32::from(h.probability));
            add_house_candidate(house, u32::from(h.probability));
        }
        while max > 0 {
            let mut tile = base;
            let (house, weight) = take_house_candidate(self.services.random_range(max));
            max -= weight;
            let h = (self.leaves.house_placement)(house as u16);
            if !(self.leaves.generating_world)()
                && !(self.leaves.environment.editor)()
                && (self.leaves.house_extra_flags)(house as u16) & 1 != 0
            {
                continue;
            }
            let year = crate::timer_game_common::calendar_year().0;
            if year < h.min_year || year > h.max_year {
                continue;
            }
            let flags = (self.leaves.house_flags)(house as u16);
            let oneof = if u32::from(flags) & 64 != 0 {
                2
            } else if u32::from(flags) & 128 != 0 {
                4
            } else {
                0
            };
            if self.flags() & oneof != 0 {
                continue;
            }
            let noslope = u32::from(flags) & 2 != 0;
            if noslope && slope != 0 {
                continue;
            }
            if !self.check_house(town, &mut tile, maxz, noslope, u32::from(flags), modes) {
                continue;
            }
            let random = u32::from(self.services.random() as u8);
            if (self.leaves.house_callbacks)(house as u16) & 1 != 0
                && !unsafe {
                    resolver::allow_construction(
                        self.leaves,
                        town as u16,
                        tile,
                        house as u16,
                        random as u8,
                    )
                }
            {
                continue;
            }
            self.modify_flags(0, oneof);
            self.build_house(
                town,
                tile,
                house,
                random,
                false,
                (self.leaves.house_extra_flags)(house as u16) & 2 != 0,
            );
            return true;
        }
        false
    }
    fn tick(&self, town: u32) {
        if self.flags() & 1 == 0 {
            return;
        }
        let modes = 1 | if unsafe { self.services.settings().allow_town_roads.read() } {
            2
        } else {
            0
        };
        let mut counter = i32::from(self.counter()) - 1;
        if counter < 0 {
            counter = i32::from(if self.grow(town, modes) {
                self.rate()
            } else {
                self.rate().min((70 - 1) as u16)
            });
        }
        self.set_counter(counter as u16);
    }
    fn expand(&self, town: u32, amount: u32, modes: u32) {
        if amount == 0 {
            let amount = self
                .services
                .random_range((self.num_houses() / 10).min(u32::from(u16::MAX)))
                + 3;
            self.change_house_count(amount);
            // SAFETY: Pure immediate owner mutation, without native callbacks.
            unsafe {
                update_radius(self.owner);
            }
            for _ in 0..amount * 10 {
                self.grow(town, modes);
            }
            self.change_house_count(amount.wrapping_neg());
        } else {
            for _ in 0..amount {
                for _ in 0..25 {
                    if self.grow(town, modes) {
                        break;
                    }
                }
            }
        }
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe {
            update_radius(self.owner);
        }
        policy::update_town_max_pass(self.leaves, self.owner);
    }
    fn tick_all(mut self) {
        if !(self.leaves.environment.editor)() {
            let mut id = (self.leaves.next_town)(0);
            while id != u32::MAX {
                // Each town starts with an empty map/shell cache, as the previous
                // per-town engine did, without copying that engine each tick.
                self.map.take();
                self.house_shell.take();
                self.owner = (self.leaves.resolve_owner)(id);
                self.tick(id);
                id = (self.leaves.next_town)(id + 1);
            }
        }
    }
    fn custom_rate(&self, town: u32, a: u32) {
        if a == 0 {
            self.modify_flags(8, 0);
        } else {
            let old = u32::from(self.rate());
            let counter = u32::from(self.counter());
            self.set_counter(if counter >= old {
                a as u16
            } else {
                (counter * a / old) as u16
            });
            self.set_rate(a as u16);
            self.modify_flags(0, 8);
        }
        self.update_growth(town);
    }
    fn fund(&self, town: u32) {
        self.set_funding(3);
        self.update_growth(town);
        let counter = i32::from(self.counter());
        let rate = i32::from(self.rate());
        let ticks = i32::from(TOWN_GROWTH_TICKS);
        self.set_counter((counter.min(2 * ticks - (rate - counter) % ticks)) as u16);
    }
}
pub(crate) unsafe fn tick_all(
    leaves: *const Leaves,
    services: *const Services,
    road: *const crate::road_cmd::CommandContext,
) {
    let mut engine = engine(leaves, services);
    engine.road = road;
    engine.tick_all();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_expand(
    leaves: *const Leaves,
    services: *const Services,
    road: *const crate::road_cmd::CommandContext,
    town: u32,
    amount: u32,
    modes: u32,
) {
    let mut engine = engine(leaves, services);
    engine.road = road;
    engine.bind(town).expand(town, amount, modes);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_road_type(
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    engine(leaves, services).road_type()
}
#[cfg(feature = "native-test-support")]
pub(crate) unsafe fn tunnel(
    leaves: *const Leaves,
    services: *const Services,
    road: *const crate::road_cmd::CommandContext,
    town: u32,
    tile: u32,
    dir: u32,
) -> bool {
    let mut engine = engine(leaves, services);
    engine.road = road;
    engine.bind(town).tunnel(town, tile, dir)
}

// Original std::accumulate's literal 0 deduces an int accumulator. Each lambda
// result narrows to int, then the final int sign-extends to the uint64 destination.
pub(crate) fn sum_history(history: &[SuppliedHistory]) -> SuppliedHistory {
    let production = history.iter().fold(0_i32, |sum, h| {
        (sum as u64).wrapping_add(u64::from(h.production)) as i32
    }) as u64;
    let transported = history.iter().fold(0_i32, |sum, h| {
        (sum as u64).wrapping_add(u64::from(h.transported)) as i32
    }) as u64;
    SuppliedHistory {
        production: (production / history.len() as u64).min(u64::from(u32::MAX)) as u32,
        transported: (transported / history.len() as u64).min(u64::from(u32::MAX)) as u32,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_sum_history(
    history: *const SuppliedHistory,
    len: usize,
) -> SuppliedHistory {
    // SAFETY: Native supplied-history layout is checked; original nonempty range.
    sum_history(unsafe { std::slice::from_raw_parts(history, len) })
}
fn history_ranges() -> [crate::history::Descriptor; 3] {
    let descriptor = |first, last, division, total_division| crate::history::Descriptor {
        child: 0,
        periods: 24,
        records: last - first,
        first,
        last,
        division,
        total_division,
        child_periods: 0,
        child_division: 0,
    };
    [
        descriptor(1, 25, 1, 1),
        descriptor(25, 42, 3, 3),
        descriptor(42, 61, 4, 12),
    ]
}
impl Engine<'_> {
    fn monthly_statistics(&self, month: u8) {
        // SAFETY: No callback or service spans this borrow of canonical histories.
        let state = unsafe { &mut *self.owner };
        if state.growth.road != 0 {
            state.growth.road -= 1;
        }
        if state.growth.funding != 0 {
            state.growth.funding -= 1;
        }
        if state.simulation.exclusive_counter != 0 {
            state.simulation.exclusive_counter -= 1;
            if state.simulation.exclusive_counter == 0 {
                state.simulation.exclusivity = u8::MAX;
            }
        }
        // Company traversal is external, so this borrow ends before it.
        let mut company = (self.services.next_company)(0);
        while company != u8::MAX {
            // SAFETY: Live canonical owner; no reference spans the next pool callback.
            unsafe {
                let unwanted = &mut (*self.owner).simulation.unwanted[usize::from(company)];
                if *unwanted != 0 {
                    *unwanted -= 1;
                }
            }
            company = (self.services.next_company)(company + 1);
        }
        // SAFETY: Reacquire only after traversal; no callback spans history writes.
        let state = unsafe { &mut *self.owner };
        for range in history_ranges() {
            state.simulation.valid_history =
                crate::history::update(state.simulation.valid_history, range, u32::from(month));
        }
        for supplied in &mut state.supplied {
            for range in history_ranges() {
                if !month.is_multiple_of(range.total_division) {
                    continue;
                }
                let first = usize::from(range.first);
                let last = usize::from(range.last);
                supplied.history.copy_within(first..last - 1, first + 1);
                if range.total_division == 1 {
                    supplied.history[first] = supplied.history[first - 1];
                    supplied.history[0] = SuppliedHistory::default();
                } else if state.simulation.valid_history & (1 << (range.first - range.division))
                    != 0
                {
                    supplied.history[first] =
                        sum_history(&supplied.history[first - usize::from(range.division)..first]);
                }
            }
        }
        for received in &mut state.simulation.received {
            received.old_max = received.new_max;
            received.new_max = 0;
            received.old_act = received.new_act;
            received.new_act = 0;
        }
    }
    fn update_rating(&self, town: u32) {
        let mut company = (self.services.next_company)(0);
        while company != u8::MAX {
            // SAFETY: Immediate field update; borrow ends before pool traversal.
            unsafe {
                let rating = &mut (*self.owner).simulation.ratings[usize::from(company)];
                if *rating < 200 {
                    *rating = (i32::from(*rating) + 5).min(200) as i16;
                }
            }
            company = (self.services.next_company)(company + 1);
        }
        let center = self.position();
        let radius = self.radius()[0];
        let c = unsafe { &*(self.services.station_catchment)() };
        crate::station::spatial::radius(self.services, center, radius / 2, |st| {
            if self.distance((c.location)(st), center) > radius {
                return;
            }
            let activity = unsafe { crate::station_service::activity((c.owner)(st)) };
            let active = activity.time_since_load <= 20 || activity.time_since_unload <= 20;
            let commands = unsafe { &*(self.services.station_commands)() };
            let world = unsafe { &*commands.station };
            let owner = (world.station_owner)(st);
            if !(self.services.company_valid)(owner) {
                return;
            }
            unsafe {
                let rating = &mut (*self.owner).simulation.ratings[usize::from(owner)];
                *rating = if active {
                    (i32::from(*rating) + 12).min(i32::from(i16::MAX)) as i16
                } else {
                    (i32::from(*rating) - 15).max(i32::from(i16::MIN)) as i16
                };
            }
        });
        // SAFETY: All station visitors returned; no callback spans clamping.
        unsafe {
            for rating in &mut (*self.owner).simulation.ratings {
                *rating = (*rating).clamp(-1000, 1000);
            }
        }
        (self.leaves.dirty_authority)(town);
    }
}
pub(crate) unsafe fn openttd_rust_town_monthly_all(
    leaves: *const Leaves,
    services: *const Services,
    month: u8,
) {
    let engine = engine(leaves, services);
    let mut town = (engine.leaves.next_town)(0);
    while town != u32::MAX {
        let owner = engine.clone().bind(town);
        owner.monthly_statistics(month);
        owner.update_growth(town);
        owner.update_rating(town);
        (owner.leaves.dirty_town)(town);
        town = (engine.leaves.next_town)(town + 1);
    }
}
/// Native fixture observes the real station-count and monthly-rating consumers.
#[cfg(feature = "native-test-support")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_town_stations(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
) -> u32 {
    let e = engine(leaves, services).bind(town);
    let count = e.count_stations(town);
    e.update_rating(town);
    count
}
#[derive(Default)]
struct RatingTest {
    depth: i32,
    ratings: std::collections::BTreeMap<usize, i32>,
}
static RATING_TEST: GameState<RatingTest> = GameState::new(RatingTest {
    depth: 0,
    ratings: std::collections::BTreeMap::new(),
});
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_rating_test(mode: bool) {
    // SAFETY: Game-state-mutex serialized scratch; no callback.
    let test = unsafe { &mut *RATING_TEST.get() };
    if mode {
        if test.depth == 0 {
            test.ratings.clear();
        }
        test.depth = test.depth.wrapping_add(1);
    } else {
        debug_assert!(test.depth > 0);
        test.depth = test.depth.wrapping_sub(1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_rating(
    state: *const State,
    shell: usize,
    company: u8,
) -> i32 {
    // SAFETY: Serialized immediate accesses; native shell address preserves source map keys and reuse.
    let test = unsafe { &*RATING_TEST.get() };
    if test.depth != 0 {
        if let Some(rating) = test.ratings.get(&shell) {
            return *rating;
        }
    }
    // SAFETY: Original source invokes this only with live owner and valid company.
    unsafe { i32::from((*state).simulation.ratings[usize::from(company)]) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_change_rating(
    leaves: *const Leaves,
    services: *const Services,
    state: *mut State,
    town: u32,
    shell: usize,
    company: u8,
    add: i32,
    maximum: i32,
    no_modify: bool,
    bulldozer: bool,
) {
    if state.is_null() || no_modify {
        return;
    }
    // SAFETY: Immutable call tables, live for invocation; no owner borrow.
    let valid = (unsafe { (*services).company_valid })(company);
    if !valid || (bulldozer && add < 0) {
        return;
    }
    // SAFETY: Valid company, immediate read through original scratch precedence.
    let mut rating = unsafe { openttd_rust_town_rating(state, shell, company) };
    if add < 0 {
        if rating > maximum {
            rating = rating.wrapping_add(add).max(maximum);
        }
    } else if rating < maximum {
        rating = rating.wrapping_add(add).min(maximum);
    }
    let testing = {
        // SAFETY: No callback during scratch map insertion.
        let test = unsafe { &mut *RATING_TEST.get() };
        if test.depth == 0 {
            false
        } else {
            test.ratings.insert(shell, rating);
            true
        }
    };
    if !testing {
        // SAFETY: Field writes finish before native window invalidation.
        unsafe {
            (*state).simulation.have_ratings |= 1 << company;
            (*state).simulation.ratings[usize::from(company)] = rating as i16;
        }
        // SAFETY: Live immutable typed service table.
        (unsafe { (*leaves).dirty_authority })(town);
    }
}

/// Read the original last-month transported percentage from the sole town owner.
/// SAFETY: Serialized live owner; no callback or owner borrow survives this call.
pub(crate) unsafe fn subsidy_percent_transported(state: *const State, cargo: u8) -> u8 {
    if !crate::cargo_type::is_valid_cargo_type(cargo) {
        return 0;
    }
    let rows = unsafe { &(*state).supplied };
    let at = unsafe { openttd_rust_town_supplied_find(state, cargo) };
    if at == rows.len() {
        return 0;
    }
    unsafe {
        openttd_rust_town_percent_transported(&raw const rows[at].history[usize::from(LAST_MONTH)])
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownSuppliedEntry" ["probe"] => SuppliedEntry {
        entry,
        created,
    };
    "OpenTTDTownAuthorityServices" ["probe"] => AuthorityServices {
        council_tolerance,
        bulldozer,
        authority_distance,
        town_index,
        refusal,
    };
    "OpenTTDTownRef" ["probe"] => TownRef {
        shell,
        state,
        id,
    };
    "OpenTTDTownSimulationFields" ["probe"] => SimulationFields {
        valid_history,
        xy,
        num_houses,
        population,
        radius,
        goal,
        ratings,
        received,
        noise_reached,
        time_until_rebuild,
        statues,
        have_ratings,
        unwanted,
        exclusivity,
        exclusive_counter,
        larger_town,
        layout,
        townnamegrfid,
        townnametype,
        townnameparts,
    };
    "OpenTTDTownReceived" ["probe"] => Received {
        old_max,
        new_max,
        old_act,
        new_act,
    };
    "OpenTTDTownSuppliedHistory" ["probe"] => SuppliedHistory {
        production,
        transported,
    };
    "OpenTTDTownSupplied" ["probe"] => Supplied {
        cargo,
        history,
    };
    "OpenTTDTownRoadChoice" ["probe"] => RoadChoice {
        introduction_date,
        max_speed,
        town_build,
    };
    "OpenTTDTownHouseAvailability" ["probe"] => HouseAvailability {
        zones,
        override_id,
        class_id,
        probability,
        enabled,
    };
    "OpenTTDTownHousePlacement" ["probe"] => HousePlacement {
        min_year,
        max_year,
    };
    "OpenTTDTownStationActivity" ["probe"] => StationActivity {
        time_since_load,
        time_since_unload,
    };
    "OpenTTDTownGrowthFields" ["probe"] => GrowthFields {
        counter,
        rate,
        funding,
        road,
        flags,
    };
    "OpenTTDTownSlots" ["probe"] => Slots {
        data,
        size,
    };

    "OpenTTDTownLeaves" ["probe"] => crate::town::Leaves {
        map_log_x,
        growth_rate,

        environment,
        generating_world,
        allow_crossings,
        climate,

        road_flags,
        house_availability,
        house_placement,
        house_population,
        house_flags,
        house_extra_flags,
        house_callbacks,
        road_info,
        resolve_owner,
        dirty_authority,
        dirty_town,
        terraform_price,
        set_road_town,
        exchange_company,
        house_class,
        make_house,

        population_changed,
        house_count,
        road_type_error,
        next_town,
        build_bridge,
        build_tunnel,
        assert_clear_house,
        houses,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn gui_and_game_threads_share_original_globals() {
        let _guard = GLOBAL_LOCK.lock().unwrap();
        // Spawn/join serializes each access like the native game-state mutex.
        // Size a NewGRF-range house ID on the command thread before ottd:game.
        openttd_rust_town_global_id_counts_resize(4096);
        openttd_rust_town_global_class_counts_resize(16);
        let ids = openttd_rust_town_global_id_counts();
        let classes = openttd_rust_town_global_class_counts();
        let mut owner = State {
            id_counts: vec![0; 4096],
            class_counts: vec![0; 16],
            ..State::default()
        };
        owner.simulation.ratings[0] = 77;
        let address = (&raw mut owner) as usize;
        assert_eq!(openttd_rust_town_generating_exchange_abi(true), 0);
        clear_house_candidates();
        add_house_candidate(110, 1);
        openttd_rust_town_rating_test(true);
        let (id_pointer, class_pointer, rating) = std::thread::spawn(move || {
            let owner = address as *mut State;
            assert!((openttd_rust_town_generating_exchange_abi(false) != 0));
            assert_eq!(take_house_candidate(0), (110, 1));
            add_house_candidate(4095, 1);
            // SAFETY: Owner lives on the suspended parent stack; spawn/join prevents
            // concurrent reads and every index is valid in both sized owners.
            unsafe {
                increase_building(owner, 110, 15);
            }
            {
                // SAFETY: Serialized access; same process-wide test scratch.
                let test = unsafe { &mut *RATING_TEST.get() };
                assert_eq!(test.depth, 1);
                test.ratings.insert(42, -99);
            };
            // SAFETY: The parent does not touch its live owner until this join.
            let rating = unsafe { openttd_rust_town_rating(owner, 42, 0) };
            (
                openttd_rust_town_global_id_counts().data as usize,
                openttd_rust_town_global_class_counts().data as usize,
                rating,
            )
        })
        .join()
        .unwrap();
        assert_eq!(ids.data as usize, id_pointer);
        assert_eq!(classes.data as usize, class_pointer);
        assert_eq!(rating, -99);
        assert_eq!(take_house_candidate(0), (4095, 1));
        assert_eq!(openttd_rust_town_generating_exchange_abi(false), 0);
        // SAFETY: Child has joined; original pointers remain valid without resize.
        unsafe {
            assert_eq!(*ids.data.cast::<u32>().add(110), 1);
            assert_eq!(*classes.data.cast::<u32>().add(15), 1);
            assert_eq!(openttd_rust_town_rating(&raw const owner, 42, 0), -99);
            decrease_building(&raw mut owner, 110, 15);
            assert_eq!(*ids.data.cast::<u32>().add(110), 0);
            assert_eq!(*classes.data.cast::<u32>().add(15), 0);
        }
        assert_eq!(owner.id_counts[110], 0);
        openttd_rust_town_rating_test(false);
        // SAFETY: Test scratch is disabled; the live owner's rating is restored.
        assert_eq!(
            unsafe { openttd_rust_town_rating(&raw const owner, 42, 0) },
            77
        );
        openttd_rust_town_global_id_counts_resize(0);
        openttd_rust_town_global_class_counts_resize(0);
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use crate::services::ExpenseType;
    use crate::services::{CommandCost, CommandCostStorage};
    use std::cell::Cell;
    use std::fmt::Write;
    use std::ptr;

    // Process-wide rating scratch also belongs to the existing cross-thread test.
    // Callback state is local to this fixture thread; no production storage changes.
    thread_local! {
        static COMPANY: Cell<u8> = const { Cell::new(0) };
        static VALID: Cell<u32> = const { Cell::new(0) };
        static DIRTY: Cell<u32> = const { Cell::new(0) };
        static CLOSEST: Cell<u32> = const { Cell::new(0) };
        static COUNCIL: Cell<u8> = const { Cell::new(0) };
        static CHEAT: Cell<bool> = const { Cell::new(false) };
        static ABSENT: Cell<bool> = const { Cell::new(false) };
        static OWNER: Cell<*mut State> = const { Cell::new(ptr::null_mut()) };
    }
    extern "C" fn valid(company: u8) -> bool {
        VALID.set(VALID.get() + 1);
        company == 0 || company == 4
    }
    extern "C" fn dirty(town: u32) {
        assert_eq!(town, 17);
        DIRTY.set(DIRTY.get() + 1);
    }
    extern "C" fn unused_random(_: *mut std::ffi::c_void) -> u32 {
        unreachable!()
    }
    extern "C" fn unused_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        unreachable!()
    }
    extern "C" fn company() -> u8 {
        COMPANY.get()
    }
    extern "C" fn council_tolerance() -> u8 {
        COUNCIL.get()
    }
    extern "C" fn bulldozer() -> bool {
        CHEAT.get()
    }
    extern "C" fn distance() -> u8 {
        20
    }
    extern "C" fn town_index(_: *const crate::services::TownHandle) -> u16 {
        17
    }
    extern "C" fn tile_type(tile: u32) -> u8 {
        assert_eq!(tile, 129);
        CLOSEST.set(CLOSEST.get() + 1);
        if ABSENT.get() {
            0
        } else {
            crate::tile_type::MP_HOUSE
        }
    }
    extern "C" fn no_towns() -> usize {
        0
    }
    extern "C" fn index(tile: u32) -> u16 {
        assert_eq!(tile, 129);
        17
    }
    extern "C" fn by_id(id: u32) -> *mut crate::services::TownHandle {
        assert_eq!(id, 17);
        assert!(!ABSENT.get());
        OWNER.get().cast()
    }
    extern "C" fn owner(town: *const crate::services::TownHandle) -> *mut State {
        assert_eq!(town.cast_mut().cast::<State>(), OWNER.get());
        OWNER.get()
    }
    fn shared() -> Services {
        let mut services =
            crate::services::fixture(ptr::null_mut(), unused_random, unused_industry);
        services.company_valid = valid;
        services.current_company = company;
        services.tile_type = tile_type;
        services.town_count = no_towns;
        services.town_index = index;
        services.town_by_id = by_id;
        services.town_owner = owner;
        services.map = crate::map_func::fixture::statics();
        services
    }
    extern "C" fn unexpected_map_log_x() -> u32 {
        unreachable!("unexpected Leaves::map_log_x")
    }
    extern "C" fn unexpected_growth_rate() -> u8 {
        unreachable!("unexpected Leaves::growth_rate")
    }
    extern "C" fn unexpected_generating_world() -> bool {
        unreachable!("unexpected Leaves::generating_world")
    }
    extern "C" fn unexpected_allow_crossings() -> bool {
        unreachable!("unexpected Leaves::allow_crossings")
    }
    extern "C" fn unexpected_climate() -> u8 {
        unreachable!("unexpected Leaves::climate")
    }
    extern "C" fn unexpected_road_flags(_: u8) -> u8 {
        unreachable!("unexpected Leaves::road_flags")
    }
    extern "C" fn unexpected_house_availability(_: u16) -> HouseAvailability {
        unreachable!("unexpected Leaves::house_availability")
    }
    extern "C" fn unexpected_house_placement(_: u16) -> HousePlacement {
        unreachable!("unexpected Leaves::house_placement")
    }
    extern "C" fn unexpected_house_population(_: u16) -> u8 {
        unreachable!("unexpected Leaves::house_population")
    }
    extern "C" fn unexpected_house_flags(_: u16) -> u8 {
        unreachable!("unexpected Leaves::house_flags")
    }
    extern "C" fn unexpected_house_extra_flags(_: u16) -> u8 {
        unreachable!("unexpected Leaves::house_extra_flags")
    }
    extern "C" fn unexpected_house_callbacks(_: u16) -> u16 {
        unreachable!("unexpected Leaves::house_callbacks")
    }
    extern "C" fn unexpected_road_info(_: u8) -> RoadChoice {
        unreachable!("unexpected Leaves::road_info")
    }
    extern "C" fn unexpected_resolve_owner(_: u32) -> *mut State {
        unreachable!("unexpected Leaves::resolve_owner")
    }
    extern "C" fn unexpected_dirty_town(_: u32) {
        unreachable!("unexpected Leaves::dirty_town")
    }
    extern "C" fn unexpected_terraform_price() -> i64 {
        unreachable!("unexpected Leaves::terraform_price")
    }
    extern "C" fn unexpected_set_road_town(_: u32, _: u32) {
        unreachable!("unexpected Leaves::set_road_town")
    }
    extern "C" fn unexpected_exchange_company(_: u32) -> u32 {
        unreachable!("unexpected Leaves::exchange_company")
    }
    extern "C" fn unexpected_house_class(_: u16) -> u16 {
        unreachable!("unexpected Leaves::house_class")
    }
    extern "C" fn unexpected_make_house(_: u32, _: u32, _: u32, _: u32) {
        unreachable!("unexpected Leaves::make_house")
    }
    extern "C" fn unexpected_population_changed(_: u32) {
        unreachable!("unexpected Leaves::population_changed")
    }
    extern "C" fn unexpected_house_count() -> u32 {
        unreachable!("unexpected Leaves::house_count")
    }
    extern "C" fn unexpected_road_type_error(_: i32) {
        unreachable!("unexpected Leaves::road_type_error")
    }
    extern "C" fn unexpected_next_town(_: u32) -> u32 {
        unreachable!("unexpected Leaves::next_town")
    }
    extern "C" fn unexpected_build_bridge(_: u32, _: u32, _: u32, _: u32, _: u32) -> bool {
        unreachable!("unexpected Leaves::build_bridge")
    }
    extern "C" fn unexpected_build_tunnel(_: u32, _: u32, _: u32) -> bool {
        unreachable!("unexpected Leaves::build_tunnel")
    }
    extern "C" fn unexpected_assert_clear_house(_: bool) {
        unreachable!("unexpected Leaves::assert_clear_house")
    }
    fn leaves() -> Leaves {
        Leaves {
            map_log_x: unexpected_map_log_x,
            growth_rate: unexpected_growth_rate,
            environment: crate::services::EnvironmentServices {
                editor: unexpected_generating_world,
                freeform_edges: unexpected_generating_world,
            },
            generating_world: unexpected_generating_world,
            allow_crossings: unexpected_allow_crossings,
            climate: unexpected_climate,
            road_flags: unexpected_road_flags,
            house_availability: unexpected_house_availability,
            house_placement: unexpected_house_placement,
            house_population: unexpected_house_population,
            house_flags: unexpected_house_flags,
            house_extra_flags: unexpected_house_extra_flags,
            house_callbacks: unexpected_house_callbacks,
            road_info: unexpected_road_info,
            resolve_owner: unexpected_resolve_owner,
            dirty_authority: dirty,
            dirty_town: unexpected_dirty_town,
            terraform_price: unexpected_terraform_price,
            set_road_town: unexpected_set_road_town,
            exchange_company: unexpected_exchange_company,
            house_class: unexpected_house_class,
            make_house: unexpected_make_house,
            population_changed: unexpected_population_changed,
            house_count: unexpected_house_count,
            road_type_error: unexpected_road_type_error,
            next_town: unexpected_next_town,
            build_bridge: unexpected_build_bridge,
            build_tunnel: unexpected_build_tunnel,
            assert_clear_house: unexpected_assert_clear_house,
            houses: ptr::null(),
        }
    }

    fn check_rows(kind: char, actual: &[String], count: usize) {
        let prefix = format!("{kind} ");
        let expected: Vec<_> = include_str!("town/policy-oracle.txt")
            .lines()
            .filter(|line| line.starts_with(&prefix))
            .collect();
        assert_eq!(actual.len(), count);
        assert_eq!(expected.len(), count);
        for (case, (a, e)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(a, e, "{kind} case {case}");
        }
    }
    fn supplied_history(
        count: usize,
        production: u32,
        transported: u32,
        stride: u32,
    ) -> Vec<SuppliedHistory> {
        (0..count)
            .map(|i| SuppliedHistory {
                production: production.wrapping_add((i as u32).wrapping_mul(stride)),
                transported: transported.wrapping_add((i as u32).wrapping_mul(stride)),
            })
            .collect()
    }
    #[test]
    fn histories_match_original_policy_corpus() {
        let mut rows = Vec::new();
        for count in [1, 3, 4, 61] {
            for production in [0_u32, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
                for transported in [0_u32, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
                    for stride in [0_u32, 17, 0x4000_0000, 0x8000_0000] {
                        let history = supplied_history(count, production, transported, stride);
                        let result = sum_history(&history);
                        rows.push(format!(
                            "H {count} {production} {transported} {stride} {} {}",
                            result.production, result.transported
                        ));
                    }
                }
            }
        }
        check_rows('H', &rows, 400);
    }
    #[test]
    fn radii_match_original_policy_corpus() {
        let mut town = State::default();
        let mut rows = Vec::new();
        for houses in (0..=100).chain([1000, 10000, 0x100_0000, 0x7fff_ffff, 0x8000_0000, u32::MAX])
        {
            town.simulation.num_houses = houses;
            // SAFETY: Live local canonical owner, no callbacks.
            unsafe {
                update_radius(&raw mut town);
            }
            let mut row = format!("Z {houses}");
            for value in town.simulation.radius {
                write!(row, " {value}").unwrap();
            }
            rows.push(row);
        }
        check_rows('Z', &rows, 107);
    }
    #[test]
    fn percent_transported_matches_original_policy_corpus() {
        let mut rows = Vec::new();
        for production in [0, 1, 255, 256, 0x100_0000, u32::MAX] {
            for transported in [0, 1, 255, 256, 0x100_0000, u32::MAX] {
                let history = SuppliedHistory {
                    production,
                    transported,
                };
                // SAFETY: Live typed supplied-history record.
                let result = unsafe { openttd_rust_town_percent_transported(&raw const history) };
                rows.push(format!("P {production} {transported} {result}"));
            }
        }
        check_rows('P', &rows, 36);
    }
    struct Ratings {
        town: Box<State>,
        leaves: Leaves,
        shared: Services,
        rows: Vec<String>,
        depth: u32,
    }
    impl Ratings {
        fn new() -> Self {
            Self {
                town: Box::default(),
                leaves: leaves(),
                shared: shared(),
                rows: Vec::new(),
                depth: 0,
            }
        }
        fn reset(&mut self, initial: i16) {
            self.town.simulation.ratings.fill(initial);
            self.town.simulation.have_ratings = 0;
        }
        fn mode(&mut self, enabled: bool) {
            openttd_rust_town_rating_test(enabled);
            if enabled {
                self.depth += 1;
            } else {
                self.depth -= 1;
            }
        }
        fn change(&mut self, add: i32, maximum: i32, company: u8, flags: [bool; 3]) {
            let [no_modify, bulldozer, null_town] = flags;
            VALID.set(0);
            DIRTY.set(0);
            let state = if null_town {
                ptr::null_mut()
            } else {
                &raw mut *self.town
            };
            // SAFETY: Local owner/tables live throughout; 42 models the single native
            // shell address, deliberately stable across canonical owner replacement.
            unsafe {
                openttd_rust_town_change_rating(
                    &raw const self.leaves,
                    &raw const self.shared,
                    state,
                    17,
                    42,
                    company,
                    add,
                    maximum,
                    no_modify,
                    bulldozer,
                );
            }
            let mut row = format!(
                "R {} {add} {maximum} {company} {} {} {} {} {} {} {}",
                self.rows.len(),
                u8::from(self.depth != 0),
                u8::from(no_modify),
                u8::from(bulldozer),
                u8::from(null_town),
                VALID.get(),
                DIRTY.get(),
                self.town.simulation.have_ratings
            );
            for value in self.town.simulation.ratings {
                write!(row, " {value}").unwrap();
            }
            // SAFETY: Only the original fixture's two valid companies read ratings.
            let rating = if company == 0 || company == 4 {
                unsafe { openttd_rust_town_rating(&raw const *self.town, 42, company) }
            } else {
                0
            };
            write!(row, " {rating}").unwrap();
            self.rows.push(row);
        }
    }
    #[test]
    fn ratings_match_original_policy_corpus() {
        let _guard = super::tests::GLOBAL_LOCK.lock().unwrap();
        let mut r = Ratings::new();
        for initial in [-32768, -1000, 0, 1000, 32767] {
            for add in [-40000, -1, 0, 1, 40000] {
                for maximum in [-50000, -1000, 0, 1000, 50000] {
                    r.reset(initial);
                    r.change(add, maximum, 0, [false; 3]);
                    r.change(add, maximum, 4, [false; 3]);
                    r.reset(initial);
                    r.mode(true);
                    r.change(add, maximum, 0, [false; 3]);
                    r.mode(true);
                    r.change(add, maximum, 4, [false; 3]);
                    r.mode(false);
                    r.change(-add, maximum, 0, [false; 3]);
                    r.mode(false);
                    r.change(add, maximum, 0, [true, false, false]);
                    r.change(add, maximum, 255, [false; 3]);
                    r.change(add, maximum, 4, [false, true, false]);
                    r.change(add, maximum, 0, [false, false, true]);
                }
            }
        }
        r.reset(10);
        r.mode(true);
        r.change(100, 1000, 0, [false; 3]);
        r.mode(true);
        // Replace the canonical owner allocation while retaining its native shell key.
        drop(std::mem::take(&mut r.town));
        r.reset(-100);
        r.change(1, 1000, 4, [false; 3]);
        r.mode(false);
        r.change(-1, -1000, 0, [false; 3]);
        r.mode(false);
        r.mode(true);
        r.change(1, 1000, 4, [false; 3]);
        r.mode(false);
        check_rows('R', &r.rows, 1129);
    }
    #[repr(C)]
    #[derive(Default)]
    struct Cost {
        ok: bool,
        error: u32,
        parameter: u64,
    }
    extern "C" fn zero(out: *mut crate::services::CommandCost) {
        // SAFETY: Stack-owned typed result passed synchronously to the real entry.
        unsafe {
            *out.cast::<Cost>() = Cost {
                ok: true,
                ..Cost::default()
            };
        }
    }
    extern "C" fn error(out: *mut crate::services::CommandCost, error: u32, parameter: u64) {
        // SAFETY: Same typed result lifetime as zero.
        unsafe {
            *out.cast::<Cost>() = Cost {
                ok: false,
                error,
                parameter,
            };
        }
    }
    fn costs() -> crate::services::CommandCostServices {
        extern "C" fn unexpected_construct(_: *mut CommandCostStorage) -> *mut CommandCost {
            unreachable!("unexpected CommandCostServices::construct")
        }
        extern "C" fn unexpected_destroy(_: *mut CommandCost) {
            unreachable!("unexpected CommandCostServices::destroy")
        }
        extern "C" fn unexpected_error(_: *mut CommandCost, _: u32) {
            unreachable!("unexpected CommandCostServices::error")
        }
        extern "C" fn unexpected_expense_money(_: *mut CommandCost, _: u8, _: i64) {
            unreachable!("unexpected CommandCostServices::expense_money")
        }
        extern "C" fn unexpected_add(_: *mut CommandCost, _: *mut CommandCost) {
            unreachable!("unexpected CommandCostServices::add")
        }
        extern "C" fn unexpected_move_cost(_: *mut CommandCost, _: *mut CommandCost) {
            unreachable!("unexpected CommandCostServices::move_cost")
        }
        extern "C" fn unexpected_amount(_: *mut CommandCost, _: i64) {
            unreachable!("unexpected CommandCostServices::amount")
        }
        extern "C" fn unexpected_success(_: *const CommandCost) -> bool {
            unreachable!("unexpected CommandCostServices::success")
        }
        extern "C" fn unexpected_error_message(_: *const CommandCost) -> u32 {
            unreachable!("unexpected CommandCostServices::error_message")
        }
        extern "C" fn unexpected_money(_: *const CommandCost) -> i64 {
            unreachable!("unexpected CommandCostServices::money")
        }
        extern "C" fn unexpected_expense(_: *const CommandCost) -> ExpenseType {
            unreachable!("unexpected CommandCostServices::expense")
        }
        extern "C" fn unexpected_make_error(_: *mut CommandCost, _: u32) {
            unreachable!("unexpected CommandCostServices::make_error")
        }
        extern "C" fn unexpected_encode_owned_by(
            _: *mut CommandCost,
            _: crate::company_type::Owner,
            _: crate::tile_type::TileIndex,
        ) {
            unreachable!("unexpected CommandCostServices::encode_owned_by")
        }
        extern "C" fn unexpected_error_owner(_: *mut CommandCost, _: crate::company_type::Owner) {
            unreachable!("unexpected CommandCostServices::error_owner")
        }
        crate::services::CommandCostServices {
            construct: unexpected_construct,
            destroy: unexpected_destroy,
            zero,
            error: unexpected_error,
            expense_money: unexpected_expense_money,
            error_param: error,
            add: unexpected_add,
            move_cost: unexpected_move_cost,
            amount: unexpected_amount,
            success: unexpected_success,
            error_message: unexpected_error_message,
            money: unexpected_money,
            expense: unexpected_expense,
            make_error: unexpected_make_error,
            encode_owned_by: unexpected_encode_owned_by,
            error_owner: unexpected_error_owner,
        }
    }

    fn authority_result(
        authority: &AuthorityServices,
        services: &Services,
        costs: &crate::services::CommandCostServices,
        args: (u8, *const crate::services::TownHandle, bool),
    ) -> Cost {
        let (kind, shell, no_test) = args;
        let mut cost = Cost::default();
        let out = (&raw mut cost).cast();
        // SAFETY: Real production dispatch; typed result and immutable tables
        // outlive this call. No reference to the owner crosses a callback.
        unsafe {
            if kind == 2 {
                openttd_rust_town_authority_station(authority, services, costs, out, 129, no_test);
            } else {
                openttd_rust_town_authority_remove(
                    authority, services, costs, out, shell, no_test, kind,
                );
            }
        }
        cost
    }
    fn authority_case(town: *mut State, rows: &mut Vec<String>, case: usize) {
        // Mixed-radix order is the retiring 3 x 4 x 2 x 2 x 2 x 26 x 2
        // input nest. All three action kinds are still observed for every input.
        let company = [0, 4, 255][case / 1664];
        let council = ((case / 416) % 4) as u8;
        let no_test = (case / 208) & 1 != 0;
        let cheat = (case / 104) & 1 != 0;
        let absent = (case / 52) & 1 != 0;
        let rating = [
            -32768, -1001, -1000, -999, -201, -200, -199, 15, 16, 17, 63, 64, 65, 111, 112, 113,
            143, 144, 145, 207, 208, 209, 399, 400, 401, 32767,
        ][(case / 2) % 26];
        let testing = case & 1 != 0;
        let services = shared();
        let leaves = leaves();
        let costs = costs();
        let authority = AuthorityServices {
            council_tolerance,
            bulldozer,
            authority_distance: distance,
            town_index,
            refusal: 19,
        };
        COMPANY.set(company);
        COUNCIL.set(council);
        CHEAT.set(cheat);
        ABSENT.set(absent);
        // SAFETY: Serialized local canonical owner; the field borrow ends here.
        unsafe {
            (*town).simulation.ratings.fill(rating);
        }
        if testing {
            openttd_rust_town_rating_test(true);
            openttd_rust_town_rating_test(true);
            // SAFETY: Local owner/tables remain live, shell key is the owner address.
            unsafe {
                openttd_rust_town_change_rating(
                    &raw const leaves,
                    &raw const services,
                    town,
                    17,
                    town as usize,
                    company,
                    -40000,
                    -1000,
                    false,
                    cheat,
                );
            }
        }
        for kind in 0..3 {
            VALID.set(0);
            CLOSEST.set(0);
            let shell = if absent {
                ptr::null()
            } else {
                town.cast_const().cast()
            };
            let cost = authority_result(&authority, &services, &costs, (kind, shell, no_test));
            rows.push(format!(
                "A {} {company} {council} {} {} {} {rating} {} {kind} {} {} {} {} {}",
                rows.len(),
                u8::from(no_test),
                u8::from(cheat),
                u8::from(absent),
                u8::from(testing),
                u8::from(cost.ok),
                cost.error,
                cost.parameter,
                VALID.get(),
                CLOSEST.get()
            ));
        }
        if testing {
            openttd_rust_town_rating_test(false);
            openttd_rust_town_rating_test(false);
        }
    }
    #[test]
    fn authority_matches_original_policy_corpus() {
        let _guard = super::tests::GLOBAL_LOCK.lock().unwrap();
        let mut town = State::default();
        OWNER.set(&raw mut town);
        // An unallocated descriptor uses the existing controlled tile-type service,
        // just like the retiring native authority fixture's map descriptor.
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: ptr::null(),
                extended: ptr::null(),
                count: 0,
                assertions: false,
            },
            6,
        );
        let mut rows = Vec::new();
        for case in 0..4992 {
            authority_case(&raw mut town, &mut rows, case);
        }
        OWNER.set(ptr::null_mut());
        check_rows('A', &rows, 14976);
    }
}
