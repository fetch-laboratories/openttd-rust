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
pub mod founding;
pub mod names;

use crate::services::{Map, Services, chance16_i};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TownId(pub u16);
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TownRef {
    pub shell: *mut crate::services::TownHandle,
    pub state: *mut State,
    pub id: TownId,
}
use std::{
    cell::{OnceCell, UnsafeCell},
    ffi::c_void,
};

// Original C++ globals are shared by GUI commands on the main thread and the
// simulation on ottd:game. The native game-state mutex serializes both; startup
// accesses occur before the game thread starts. No borrow crosses a callback.
struct GameState<T>(UnsafeCell<T>);
// SAFETY: Every access is serialized by the native game-state mutex (or startup).
unsafe impl<T: Send> Sync for GameState<T> {}
impl<T> GameState<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }
    fn with<R>(&self, access: impl FnOnce(&UnsafeCell<T>) -> R) -> R {
        access(&self.0)
    }
}

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
    simulation: SimulationFields,
    supplied: Vec<Supplied>,
    id_counts: Vec<u16>,
    class_counts: Vec<u16>,
    growth: GrowthFields,
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
static GENERATING_TOWN: GameState<bool> = GameState::new(false);
pub extern "C" fn openttd_rust_town_generating_exchange(generating: bool) -> bool {
    GENERATING_TOWN.with(|flag| {
        // SAFETY: Serialized native entry; the flag is process-wide.
        unsafe { std::ptr::replace(flag.get(), generating) }
    })
}
/// Convert original town tick counters, including tick zero, to game ticks.
fn town_ticks_to_game_ticks(ticks: u16) -> u16 {
    ((u32::from(ticks.min(930)) + 1) * 70 - 1) as u16
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
    if GENERATING_TOWN.with(|flag| {
        // SAFETY: Immediate serialized read ends before presentation callbacks.
        unsafe { *flag.get() }
    }) {
        return;
    }
    (leaves.population_changed)(town);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_change_population(
    leaves: *const Leaves,
    state: *mut State,
    town: u32,
    amount: i32,
) {
    // SAFETY: Immutable table and live owner outlive the synchronous invocation.
    unsafe {
        change_population(&*leaves, state, town, amount);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_world_population(leaves: *const Leaves) -> u32 {
    // SAFETY: Immutable table remains live; no owner access spans pool iteration.
    let leaves = unsafe { &*leaves };
    let mut total = 0_u32;
    let mut town = (leaves.next_town)(0);
    while town != u32::MAX {
        let owner = (leaves.state)(town);
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
    BUILDING_COUNTS.with(|cell| {
        // SAFETY: Game-state-mutex serialized access; no service call or reentry.
        let counts = unsafe { &mut (*cell.get()).ids };
        Slots {
            data: counts.as_mut_ptr().cast(),
            size: counts.len(),
        }
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_id_counts_resize(count: usize) {
    // SAFETY: Game-state-mutex serialized access; no native view survives resizing.
    BUILDING_COUNTS.with(|cell| unsafe {
        (*cell.get()).ids.resize(count, 0);
    });
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_class_counts() -> Slots {
    BUILDING_COUNTS.with(|cell| {
        // SAFETY: Game-state-mutex serialized access; no service call or reentry.
        let counts = unsafe { &mut (*cell.get()).classes };
        Slots {
            data: counts.as_mut_ptr().cast(),
            size: counts.len(),
        }
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_global_class_counts_resize(count: usize) {
    // SAFETY: Game-state-mutex serialized access; no native view survives resizing.
    BUILDING_COUNTS.with(|cell| unsafe {
        (*cell.get()).classes.resize(count, 0);
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_increase_building(
    state: *mut State,
    house: u16,
    class: u16,
) {
    // SAFETY: Original valid spec/count indices; no callback while borrowed.
    let state = unsafe { &mut *state };
    BUILDING_COUNTS.with(|cell| {
        // SAFETY: All counters are serialized by the game-state mutex.
        let global = unsafe { &mut *cell.get() };
        state.id_counts[usize::from(house)] = state.id_counts[usize::from(house)].wrapping_add(1);
        global.ids[usize::from(house)] = global.ids[usize::from(house)].wrapping_add(1);
        if class != 0 {
            state.class_counts[usize::from(class)] =
                state.class_counts[usize::from(class)].wrapping_add(1);
            global.classes[usize::from(class)] = global.classes[usize::from(class)].wrapping_add(1);
        }
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_decrease_building(
    state: *mut State,
    house: u16,
    class: u16,
) {
    // SAFETY: Source valid indices and serialized original lifetime preconditions.
    let state = unsafe { &mut *state };
    BUILDING_COUNTS.with(|cell| {
        // SAFETY: No callback spans the process-wide global count access.
        let global = unsafe { &mut *cell.get() };
        state.id_counts[usize::from(house)] = state.id_counts[usize::from(house)].saturating_sub(1);
        global.ids[usize::from(house)] = global.ids[usize::from(house)].saturating_sub(1);
        if class != 0 {
            state.class_counts[usize::from(class)] =
                state.class_counts[usize::from(class)].saturating_sub(1);
            global.classes[usize::from(class)] =
                global.classes[usize::from(class)].saturating_sub(1);
        }
    });
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
    HOUSE_CANDIDATES.with(|cell| unsafe {
        (*cell.get()).len = 0;
    });
}
fn add_house_candidate(house: u32, weight: u32) {
    HOUSE_CANDIDATES.with(|cell| {
        // SAFETY: HouseSpec's original maximum is NUM_HOUSES=4096. No callback occurs.
        let candidates = unsafe { &mut *cell.get() };
        candidates.slots[candidates.len].write((house, weight));
        candidates.len += 1;
    });
}
fn take_house_candidate(mut random: u32) -> (u32, u32) {
    HOUSE_CANDIDATES.with(|cell| {
        // SAFETY: Source's positive total guarantees an initialized weighted slot.
        // The source's static probability vector is also replaced by reentrant builds.
        let candidates = unsafe { &mut *cell.get() };
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
    })
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
    pub flags: u8,
    pub extra_flags: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseConstruction {
    pub extra_flags: u8,
    pub population: u8,
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
    let town = (s.closest_town)(tile, u32::from((a.authority_distance)()));
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
    pub snow_line: extern "C" fn() -> u8,
    pub editor: extern "C" fn() -> bool,
    pub generating_world: extern "C" fn() -> bool,
    pub allow_crossings: extern "C" fn() -> bool,
    pub climate: extern "C" fn() -> u8,
    pub highest_snow_line: extern "C" fn() -> u8,
    pub calendar_year: extern "C" fn() -> i32,
    pub allow_roads: extern "C" fn() -> bool,
    pub road_flags: extern "C" fn(u8) -> u8,
    pub house_availability: extern "C" fn(u16) -> HouseAvailability,
    pub house_placement: extern "C" fn(u16) -> HousePlacement,
    pub house_construction: extern "C" fn(u16) -> HouseConstruction,
    pub house_flags: extern "C" fn(u16) -> u8,
    pub house_extra_flags: extern "C" fn(u16) -> u8,
    pub house_allows_callback: extern "C" fn(u16) -> bool,
    pub road_info: extern "C" fn(u8) -> RoadChoice,
    pub state: extern "C" fn(u32) -> *mut State,
    pub stations: extern "C" fn(
        u32,
        u32,
        *mut c_void,
        extern "C" fn(*mut c_void, *const crate::station::Station, u32),
    ),
    pub station_activity_owner:
        extern "C" fn(*const crate::station::Station) -> *const crate::station_service::Service,
    pub station_owner: extern "C" fn(*const crate::station::Station) -> u8,
    pub dirty_authority: extern "C" fn(u32),
    pub dirty_town: extern "C" fn(u32),
    pub foundation_slope: extern "C" fn(u32) -> u32,
    pub terraform_price: extern "C" fn() -> i64,
    pub clean_road_bits: extern "C" fn(u32, u32) -> u32,
    pub set_road_town: extern "C" fn(u32, u32),
    pub exchange_company: extern "C" fn(u32) -> u32,
    pub house_class: extern "C" fn(u16) -> u16,
    pub make_house: extern "C" fn(u32, u32, u32, u32),
    pub add_animation: extern "C" fn(u32, bool),
    pub population_changed: extern "C" fn(u32),
    pub nearby_stations: extern "C" fn(u32, u32, u32),
    pub house_count: extern "C" fn() -> u32,
    pub update_max_pass: extern "C" fn(u32),
    pub next_town: extern "C" fn(u32) -> u32,
    pub road_mask: extern "C" fn() -> u64,
    pub build_road: extern "C" fn(u32, u32, u32, u32, u32) -> bool,
    pub clear_land: extern "C" fn(u32, u32) -> bool,
    pub terraform_land: extern "C" fn(u32, u32, u32, u32) -> CommandResult,
    pub build_bridge: extern "C" fn(u32, u32, u32, u32, u32) -> bool,
    pub build_tunnel: extern "C" fn(u32, u32, u32) -> bool,
    pub clear_house_land: extern "C" fn(u32),
    pub allow_construction: extern "C" fn(u32, u32, u32, u32) -> bool,
    pub construction_animation: extern "C" fn(u32),
}
pub type StationActivity = crate::station_service::StationActivity;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandResult {
    pub cost: i64,
    pub success: bool,
}
#[derive(Clone)]
struct Engine<'a> {
    leaves: &'a Leaves,
    services: &'a Services,
    map: OnceCell<Map>,
    owner: *mut State,
}
impl Engine<'_> {
    fn map(&self) -> Map {
        *self.map.get_or_init(|| Map {
            log_x: (self.leaves.map_log_x)() as u8,
        })
    }
    fn size_x(&self) -> u32 {
        self.map().size_x()
    }
    fn position(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { position(self.owner) }
    }
    fn num_houses(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { (*self.owner).simulation.num_houses }
    }
    fn change_house_count(&self, amount: u32) {
        // SAFETY: Immediate owner mutation ends before every native callback.
        unsafe {
            (*self.owner).simulation.num_houses = self.num_houses().wrapping_add(amount);
        }
    }
    fn population(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { population(self.owner) }
    }
    fn layout(&self) -> u8 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { (*self.owner).simulation.layout }
    }
    fn is_city(&self) -> bool {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { (*self.owner).simulation.larger_town }
    }
    fn radius(&self) -> [u32; 5] {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { (*self.owner).simulation.radius }
    }
    fn goals(&self) -> [u32; 6] {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { (*self.owner).simulation.goal }
    }
    fn old_received(&self, at: usize) -> u32 {
        // SAFETY: Source bounds and immediate owner access before any callback.
        unsafe { u32::from((*self.owner).simulation.received[at].old_act) }
    }
    fn bind(mut self, town: u32) -> Self {
        self.owner = (self.leaves.state)(town);
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
    fn road(&self, town: u32, tile: u32, bits: u32, flags: u32) -> bool {
        let rt = self.road_type();
        (self.leaves.build_road)(town, tile, bits, rt, flags)
    }
    fn clear(&self, tile: u32, execute: bool) -> bool {
        (self.leaves.clear_land)(tile, u32::from(execute))
    }
    fn count_stations(&self, _town: u32) -> u32 {
        struct StationCount<'a> {
            engine: Engine<'a>,
            center: u32,
            radius: u32,
            count: u32,
        }
        extern "C" fn visit(ctx: *mut c_void, station: *const crate::station::Station, xy: u32) {
            // SAFETY: Synchronous non-reentrant visitor; context lives on this stack.
            let count = unsafe { &mut *ctx.cast::<StationCount>() };
            if count.engine.distance(xy, count.center) <= count.radius {
                // SAFETY: Synchronous radius visitor supplies a live station owner.
                let activity = unsafe {
                    crate::station_service::activity((count.engine.leaves.station_activity_owner)(
                        station,
                    ))
                };
                if activity.time_since_load <= 20 || activity.time_since_unload <= 20 {
                    count.count = count.count.wrapping_add(1);
                }
            }
        }
        let center = self.position();
        let radius = self.radius()[0];
        let mut count = StationCount {
            engine: {
                let _ = self.map();
                self.clone()
            },
            center,
            radius,
            count: 0,
        };
        (self.leaves.stations)(center, radius / 2, (&raw mut count).cast(), visit);
        count.count
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
                        if (self.services.tile_height)(self.position())
                            >= u32::from((self.leaves.snow_line)())
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
        leaves,
        services,
        map: OnceCell::new(),
        owner: std::ptr::null_mut(),
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
        let mut mask = (self.leaves.road_mask)();
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
            if (0..1_826_212_865).contains(&date) && date > (self.services.calendar_date)() {
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
        if (self.services.distance_from_edge)(tile) == 0
            || ((self.services.bridge_above)(tile)
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
            (self.leaves.foundation_slope)(tile)
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
                    success = (self.leaves.terraform_land)(tile, edges, 0, 1).success;
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
        let CommandResult { success, cost } =
            (self.leaves.terraform_land)(tile, edges, u32::from(up), 0);
        if !success || cost >= ((self.leaves.terraform_price)() + 2) * 8 {
            return false;
        }
        (self.leaves.terraform_land)(tile, edges, u32::from(up), 1);
        true
    }
    fn level(&self, tile: u32) {
        if (self.services.tile_type)(tile) == 3 {
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
        if (self.services.distance_from_edge)(tile) == 0 {
            return false;
        }
        let mut count = 0;
        for dir in 0..4 {
            if matches!((self.services.tile_type)(self.step(tile, dir)), 3 | 7) {
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
            9 => {
                return (self.services.tunnel_transport)(next) == 1
                    && u32::from((self.services.tunnel_bridge_direction)(next)) == dir;
            }
            5 => {
                if (self.services.is_drive_through_stop)(next) {
                    return u32::from((self.services.drive_through_axis)(next)) == dir & 1;
                }
                if (self.services.is_bay_stop)(next) {
                    return u32::from((self.services.bay_stop_direction)(next)) == reverse(dir);
                }
                return false;
            }
            2 => {
                return (self.services.is_road_depot)(next)
                    && u32::from((self.services.road_depot_direction)(next)) == reverse(dir);
            }
            1 => {
                if !(self.leaves.allow_crossings)() {
                    return false;
                }
            }
            _ => {}
        }
        (self.leaves.build_road)(town, next, bit(reverse(dir)), rt, 0)
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
                    && (slope != 0 || !(self.services.is_sea)(end)))
                    || (self.services.is_plain_rail)(end)
                    || ((self.services.is_normal_road)(end)
                        && (self.services.disallowed_road_directions)(end) != 0))
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
                let slope = u32::from((self.services.tile_slope)(test));
                if slope != incline(dir) && slope & 16 == 0 && !matches!(slope, 1 | 2 | 4 | 8) {
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
                || !((self.services.tile_type)(at) == 2 || (self.services.is_any_road_stop)(at))
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
        if (self.services.tile_type)(tile) != 2 {
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
                2 => bits != 0,
                5 => (self.services.is_drive_through_stop)(next),
                9 => (self.services.tunnel_transport)(next) == 1,
                3 | 8 | 10 => false,
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
                || (!(self.leaves.allow_crossings)() && (self.services.tile_type)(at) == 1)
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
                    if (self.services.tile_type)(self.step(at, (dir + 1) & 3)) != 3
                        && (self.services.tile_type)(self.step(at, (dir + 3) & 3)) != 3
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
            if (self.services.tile_type)(at) == 9 {
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
                if (self.services.tile_type)(house_tile) != 3 {
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
        bits = (self.leaves.clean_road_bits)(at, bits);
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
            if (self.services.tile_type)(tile) == 9 {
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
            if (self.services.tile_type)(tile) == 2
                && !(self.services.is_road_depot)(tile)
                && (self.services.has_road_type)(tile)
            {
                let owner = (self.services.road_owner)(tile);
                if owner == 15 && u32::from((self.services.town_index)(tile)) != town {
                    return false;
                }
                if owner == 16 && (self.leaves.editor)() {
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
                if (self.services.tile_type)(tile) != 3
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
                    (self.leaves.build_road)(town, tile, (1 << a) + (1 << b), rt, 2);
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
        let slope = u32::from((self.services.tile_slope)(tile));
        if (noslope && slope != 0) || slope & 16 != 0 {
            return false;
        }
        if !self.roads_allow_house(tile) || (self.services.bridge_above)(tile) {
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
        (self.leaves.clear_house_land)(tile);
        let class = (self.leaves.house_class)(house as u16);
        // SAFETY: Entry resolved the live town once. Count borrows end here,
        // before map writes or callbacks can reenter and mutate the owner.
        unsafe { openttd_rust_town_increase_building(self.owner, house as u16, class) };
        (self.leaves.make_house)(
            tile,
            town,
            house,
            counter | (stage << 8) | (random << 16) | (u32::from(protected) << 24),
        );
        if u32::from((self.leaves.house_flags)(house as u16)) & 32 != 0 {
            (self.leaves.add_animation)(tile, false);
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
        let h = (self.leaves.house_construction)(house as u16);
        let mut counter = 0;
        let mut stage = 0;
        if (self.leaves.generating_world)() || (self.leaves.editor)() || complete {
            let construction = self.services.random();
            stage = 3;
            if (self.leaves.generating_world)() && h.extra_flags & 1 == 0 && self.chance(1, 7) {
                stage = construction & 3;
            }
            if stage == 3 {
                // SAFETY: Owner mutation ends before possible presentation reentry.
                unsafe {
                    change_population(self.leaves, self.owner, town, i32::from(h.population));
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
        (self.leaves.nearby_stations)(town, tile, size);
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe {
            update_radius(self.owner);
        }
        self.update_rate(town);
        let size = u32::from((self.leaves.house_flags)(house as u16));
        (self.leaves.construction_animation)(tile);
        if size & 24 != 0 {
            (self.leaves.construction_animation)(self.offset(tile, 0, 1));
        }
        if size & 20 != 0 {
            (self.leaves.construction_animation)(self.offset(tile, 1, 0));
        }
        if size & 16 != 0 {
            (self.leaves.construction_animation)(self.offset(tile, 1, 1));
        }
    }
    fn try_house(&self, town: u32, base: u32, modes: u32) -> bool {
        if !self.layout_house(town, base, modes, false) || !self.can_house(base, false) {
            return false;
        }
        let slope = u32::from((self.services.tile_slope)(base));
        let maxz = (self.services.tile_max_z)(base);
        let distance = self.distance(self.position(), base);
        let radius = self.radius();
        let mut zone = 0;
        if self.funding() != 0 && distance <= 25 {
            zone = 4;
        } else {
            for (i, r) in radius.iter().enumerate() {
                if distance < *r {
                    zone = i as u32;
                }
            }
        }
        let climate = match (self.leaves.climate)() {
            0 => 12,
            1 => {
                if maxz > i32::from((self.leaves.highest_snow_line)()) {
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
                && !(self.leaves.editor)()
                && h.extra_flags & 1 != 0
            {
                continue;
            }
            let year = (self.leaves.calendar_year)();
            if year < h.min_year || year > h.max_year {
                continue;
            }
            let oneof = if u32::from(h.flags) & 64 != 0 {
                2
            } else if u32::from(h.flags) & 128 != 0 {
                4
            } else {
                0
            };
            if self.flags() & oneof != 0 {
                continue;
            }
            let noslope = u32::from(h.flags) & 2 != 0;
            if noslope && slope != 0 {
                continue;
            }
            if !self.check_house(town, &mut tile, maxz, noslope, u32::from(h.flags), modes) {
                continue;
            }
            let random = u32::from(self.services.random() as u8);
            if (self.leaves.house_allows_callback)(house as u16)
                && !(self.leaves.allow_construction)(town, tile, house, random)
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
        let modes = 1 | if (self.leaves.allow_roads)() { 2 } else { 0 };
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
        (self.leaves.update_max_pass)(town);
    }
    fn tick_all(&self) {
        if !(self.leaves.editor)() {
            let mut id = (self.leaves.next_town)(0);
            while id != u32::MAX {
                self.clone().bind(id).tick(id);
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
        let ticks = 70;
        self.set_counter((counter.min(2 * ticks - (rate - counter) % ticks)) as u16);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_try_house(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
    tile: u32,
    modes: u32,
) -> bool {
    engine(leaves, services)
        .bind(town)
        .try_house(town, tile, modes)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_build_house(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
    tile: u32,
    house: u32,
    random: u32,
    complete: bool,
    protected: bool,
) {
    engine(leaves, services)
        .bind(town)
        .build_house(town, tile, house, random, complete, protected);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_tick_all(
    leaves: *const Leaves,
    services: *const Services,
) {
    engine(leaves, services).tick_all();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_update_growth(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
) {
    engine(leaves, services).bind(town).update_growth(town);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_expand(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
    amount: u32,
    modes: u32,
) {
    engine(leaves, services)
        .bind(town)
        .expand(town, amount, modes);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_custom_rate(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
    rate: u32,
) {
    engine(leaves, services).bind(town).custom_rate(town, rate);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_fund(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
) {
    engine(leaves, services).bind(town).fund(town);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_road_type(
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    engine(leaves, services).road_type()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_tunnel(
    leaves: *const Leaves,
    services: *const Services,
    town: u32,
    tile: u32,
    dir: u32,
) -> bool {
    engine(leaves, services).bind(town).tunnel(town, tile, dir)
}

// Original std::accumulate's literal 0 deduces an int accumulator. Each lambda
// result narrows to int, then the final int sign-extends to the uint64 destination.
fn sum_history(history: &[SuppliedHistory]) -> SuppliedHistory {
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
        struct RatingContext<'a> {
            engine: Engine<'a>,
            center: u32,
            radius: u32,
        }
        extern "C" fn visit(
            context: *mut c_void,
            station: *const crate::station::Station,
            xy: u32,
        ) {
            // SAFETY: Synchronous stack visitor; no reference survives reentry.
            let context = unsafe { &*context.cast::<RatingContext>() };
            let engine = context.engine.clone();
            let center = context.center;
            let radius = context.radius;
            if engine.distance(xy, center) > radius {
                return;
            }
            // SAFETY: Synchronous radius visitor supplies a live station owner.
            let activity = unsafe {
                crate::station_service::activity((engine.leaves.station_activity_owner)(station))
            };
            let active = activity.time_since_load <= 20 || activity.time_since_unload <= 20;
            let owner = (engine.leaves.station_owner)(station);
            if !(engine.services.company_valid)(owner) {
                return;
            }
            // SAFETY: Valid company index and immediate write after all services.
            unsafe {
                let rating = &mut (*engine.owner).simulation.ratings[usize::from(owner)];
                *rating = if active {
                    (i32::from(*rating) + 12).min(i32::from(i16::MAX)) as i16
                } else {
                    (i32::from(*rating) - 15).max(i32::from(i16::MIN)) as i16
                };
            }
        }
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
        let mut context = RatingContext {
            engine: {
                let _ = self.map();
                self.clone()
            },
            center,
            radius,
        };
        (self.leaves.stations)(center, radius / 2, (&raw mut context).cast(), visit);
        // SAFETY: All station visitors returned; no callback spans clamping.
        unsafe {
            for rating in &mut (*self.owner).simulation.ratings {
                *rating = (*rating).clamp(-1000, 1000);
            }
        }
        (self.leaves.dirty_authority)(town);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_monthly_all(
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
    RATING_TEST.with(|cell| {
        // SAFETY: Game-state-mutex serialized scratch; no callback.
        let test = unsafe { &mut *cell.get() };
        if mode {
            if test.depth == 0 {
                test.ratings.clear();
            }
            test.depth = test.depth.wrapping_add(1);
        } else {
            debug_assert!(test.depth > 0);
            test.depth = test.depth.wrapping_sub(1);
        }
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_rating(
    state: *const State,
    shell: usize,
    company: u8,
) -> i32 {
    RATING_TEST.with(|cell| {
        // SAFETY: Serialized immediate accesses; native shell address preserves source map keys and reuse.
        let test = unsafe { &*cell.get() };
        if test.depth != 0 {
            if let Some(rating) = test.ratings.get(&shell) {
                return *rating;
            }
        }
        // SAFETY: Original source invokes this only with live owner and valid company.
        unsafe { i32::from((*state).simulation.ratings[usize::from(company)]) }
    })
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
    let testing = RATING_TEST.with(|cell| {
        // SAFETY: No callback during scratch map insertion.
        let test = unsafe { &mut *cell.get() };
        if test.depth == 0 {
            false
        } else {
            test.ratings.insert(shell, rating);
            true
        }
    });
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

/// House fields consumed only by the yearly traversal, with immediate map access.
#[repr(C)]
pub struct YearlyServices {
    pub completed: extern "C" fn(u32) -> bool,
    pub age: extern "C" fn(u32) -> u8,
    pub set_age: extern "C" fn(u32, u8),
}
/// Original `Map::Iterate` order and completed-house age saturation.
/// SAFETY: Immutable tables live through this serialized native timer entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_yearly(
    services: *const Services,
    houses: *const YearlyServices,
    size: u32,
) {
    // SAFETY: Borrowed tables outlive traversal; map access ends before mutations.
    let (services, houses) = unsafe { (&*services, &*houses) };
    for tile in 0..size {
        if (services.tile_type)(tile) != 3 {
            continue;
        }
        if (houses.completed)(tile) {
            let age = (houses.age)(tile);
            if age < u8::MAX {
                (houses.set_age)(tile, age + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_and_game_threads_share_original_globals() {
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
        assert!(!openttd_rust_town_generating_exchange(true));
        clear_house_candidates();
        add_house_candidate(110, 1);
        openttd_rust_town_rating_test(true);
        let (id_pointer, class_pointer, rating) = std::thread::spawn(move || {
            let owner = address as *mut State;
            assert!(openttd_rust_town_generating_exchange(false));
            assert_eq!(take_house_candidate(0), (110, 1));
            add_house_candidate(4095, 1);
            // SAFETY: Owner lives on the suspended parent stack; spawn/join prevents
            // concurrent reads and every index is valid in both sized owners.
            unsafe {
                openttd_rust_town_increase_building(owner, 110, 15);
            }
            RATING_TEST.with(|cell| {
                // SAFETY: Serialized access; same process-wide test scratch.
                let test = unsafe { &mut *cell.get() };
                assert_eq!(test.depth, 1);
                test.ratings.insert(42, -99);
            });
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
        assert!(!openttd_rust_town_generating_exchange(false));
        // SAFETY: Child has joined; original pointers remain valid without resize.
        unsafe {
            assert_eq!(*ids.data.cast::<u32>().add(110), 1);
            assert_eq!(*classes.data.cast::<u32>().add(15), 1);
            assert_eq!(openttd_rust_town_rating(&raw const owner, 42, 0), -99);
            openttd_rust_town_decrease_building(&raw mut owner, 110, 15);
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
