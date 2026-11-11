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

use crate::bitmath_func::gb;
use crate::bridge::MAX_BRIDGES;
use crate::cargo_type::{CargoType, is_valid_cargo_type};
use crate::cargotype::TownAcceptanceEffect;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, CompanyMask, MAX_COMPANIES, OWNER_NONE, OWNER_TOWN};
use crate::direction_func::{
    change_diag_dir, diag_dir_to_axis, is_valid_diag_direction, reverse_diag_dir,
};
use crate::direction_type::{Axis, DiagDirDiff, DiagDirection, Direction};
use crate::history_type::{HISTORY_RECORDS, LAST_MONTH};
use crate::house::{
    BuildingFlag, BuildingFlags, HOUSE_NO_CLASS, HouseExtraFlag, HouseExtraFlags, HouseZone,
    HouseZones, INVALID_HOUSE_ID, NUM_HOUSE_ZONES, NUM_HOUSES, TOWN_HOUSE_COMPLETED,
};
use crate::house_type::{HouseClassID, HouseID};
use crate::landscape_type::LandscapeType;
use crate::map_type::TileIndexDiffC;
use crate::math::{clamp_to_u8, clamp_to_u16, clamp_to_u32, round_div_su};
use crate::newgrf_callbacks::{HouseCallbackMask, HouseCallbackMasks};
use crate::random_func::chance16;
use crate::road::{RoadTypeFlag, RoadTypeFlags};
use crate::road_func::diag_dir_to_road_bits;
use crate::road_type::{DisallowedRoadDirections, RoadBits, RoadTramType, RoadType};
use crate::services::{Map, Services};
use crate::slope_func::{
    complement_slope, inclined_slope, is_slope_with_one_corner_raised, is_steep_slope,
};
use crate::slope_type::Slope;
use crate::tile_type::{TROPICZONE_DESERT, TileIndex, TileType};
use crate::timer_game_common::MAX_DATE;
use crate::timer_game_tick::TOWN_GROWTH_TICKS;
use crate::town_type::{
    RATING_GROWTH_MAXIMUM, RATING_GROWTH_UP_STEP, RATING_MAXIMUM, RATING_MINIMUM,
    RATING_ROAD_NEEDED_HOSTILE, RATING_ROAD_NEEDED_LENIENT, RATING_ROAD_NEEDED_NEUTRAL,
    RATING_ROAD_NEEDED_PERMISSIVE, RATING_STATION_DOWN_STEP, RATING_STATION_UP_STEP,
    RATING_TUNNEL_BRIDGE_NEEDED_HOSTILE, RATING_TUNNEL_BRIDGE_NEEDED_LENIENT,
    RATING_TUNNEL_BRIDGE_NEEDED_NEUTRAL, RATING_TUNNEL_BRIDGE_NEEDED_PERMISSIVE, RATING_VERYPOOR,
    TownExpandMode, TownExpandModes, TownID, TownLayout,
};
use crate::transport_type::TransportType;

/// `NUM_TAE`, as the length of the per-effect town arrays.
const NUM_TAE: usize = TownAcceptanceEffect::NUM_TAE.0 as usize;

/// Value for custom town number in difficulty settings (`town.h`).
pub const CUSTOM_TOWN_NUMBER_DIFFICULTY: u32 = 4;

/// The town only needs this cargo in the winter (any amount).
pub const TOWN_GROWTH_WINTER: u32 = 0xFFFF_FFFE;
/// The town needs the cargo for growth when on desert (any amount).
pub const TOWN_GROWTH_DESERT: u32 = 0xFFFF_FFFF;
/// Special value for `Town::growth_rate` to disable town growth.
pub const TOWN_GROWTH_RATE_NONE: u16 = 0xFFFF;
/// Max amount of original town ticks that still fit into `uint16_t`, about equal to
/// `UINT16_MAX / TOWN_GROWTH_TICKS` but slightly less to simplify calculations.
pub const MAX_TOWN_GROWTH_TICKS: u16 = 930;

/// Town flags (`town.h`), as bit positions, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownFlag(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "original town.h TownFlag enumerators"
)]
impl TownFlag {
    /// Conditions for town growth are met. Grow according to `Town::growth_rate`.
    pub const IsGrowing: Self = Self(0);
    /// There can be only one church by town.
    pub const HasChurch: Self = Self(1);
    /// There can be only one stadium by town.
    pub const HasStadium: Self = Self(2);
    /// Growth rate is controlled by GS.
    pub const CustomGrowth: Self = Self(3);
}
/// Set of `TownFlag`s (`town.h`), retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownFlags(pub u8);
impl TownFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native element-to-set conversion.
    #[must_use]
    pub const fn from_element(element: TownFlag) -> Self {
        Self(1 << element.0)
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: TownFlag) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
    /// Native `Reset` of one element.
    pub fn reset(&mut self, element: TownFlag) {
        self.0 &= !Self::from_element(element).0;
    }
    /// Native `Set` of one element.
    pub fn set(&mut self, element: TownFlag) {
        self.0 |= Self::from_element(element).0;
    }
    /// Native `Any` of a set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}
crate::base_bitset_type::impl_bitset_ops!(TownFlags);

/// Settings for town council attitudes (`town.h`), retaining every underlying byte.
/// Rating tables index by the attitude's value; the lenient, tolerant and hostile
/// values join with their first named Rust user.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownCouncilAttitudes(pub u8);
impl TownCouncilAttitudes {
    /// Permissive (local authority disabled).
    pub const TOWN_COUNCIL_PERMISSIVE: Self = Self(3);
}

/// Action types that a company must ask permission for to a town authority (`town.h`).
/// Native callers pass the element; Rust indexes the needed-rating table by it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownRatingCheckType(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "original town.h TownRatingCheckType enumerators"
)]
impl TownRatingCheckType {
    /// Removal of a tunnel or bridge owned by the town.
    pub const TunnelBridgeRemove: Self = Self(1);
    /// End marker.
    pub const End: Self = Self(2);
}

/// Town actions of a company (`town.h`), retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownAction(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "original town.h TownAction enumerators"
)]
impl TownAction {
    /// Small advertising campaign.
    pub const AdvertiseSmall: Self = Self(0);
    /// Medium advertising campaign.
    pub const AdvertiseMedium: Self = Self(1);
    /// Large advertising campaign.
    pub const AdvertiseLarge: Self = Self(2);
    /// Rebuild the roads.
    pub const RoadRebuild: Self = Self(3);
    /// Build a statue.
    pub const BuildStatue: Self = Self(4);
    /// Fund new buildings.
    pub const FundBuildings: Self = Self(5);
    /// Buy exclusive transport rights.
    pub const BuyRights: Self = Self(6);
    /// Try to bribe the council.
    pub const Bribe: Self = Self(7);
    /// End marker.
    pub const End: Self = Self(8);
}

/// Native `TownActions`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownActions(pub u8);
impl TownActions {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `Set` of one element.
    pub fn set(&mut self, element: TownAction) {
        self.0 |= 1 << element.0;
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: TownAction) -> bool {
        self.0 & (1 << element.0) != 0
    }
}

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
    pub radius: [u32; NUM_HOUSE_ZONES as usize],
    pub goal: [u32; NUM_TAE],
    pub ratings: [i16; MAX_COMPANIES as usize],
    pub received: [Received; NUM_TAE],
    pub noise_reached: u16,
    pub time_until_rebuild: u16,
    pub statues: u16,
    pub have_ratings: u16,
    pub unwanted: [u8; MAX_COMPANIES as usize],
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
            xy: TileIndex::INVALID.0,
            num_houses: 0,
            population: 0,
            radius: [0; NUM_HOUSE_ZONES as usize],
            goal: [0; NUM_TAE],
            ratings: [0; MAX_COMPANIES as usize],
            received: [Received::default(); NUM_TAE],
            noise_reached: 0,
            time_until_rebuild: 0,
            statues: 0,
            have_ratings: 0,
            unwanted: [0; MAX_COMPANIES as usize],
            exclusivity: CompanyID::INVALID.0,
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
    pub history: [SuppliedHistory; HISTORY_RECORDS],
}
impl Default for Supplied {
    fn default() -> Self {
        Self {
            cargo: CargoType::INVALID_CARGO.0,
            history: [SuppliedHistory::default(); HISTORY_RECORDS],
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
/// `t->received[effect].new_act += amount` (`economy.cpp`'s `DeliverGoods`).
/// SAFETY: Live serialized town owner and original cargo acceptance effect.
pub(crate) unsafe fn add_received(owner: *mut State, effect: TownAcceptanceEffect, amount: u32) {
    unsafe {
        let count = &mut (*owner).simulation.received[usize::from(effect.0)].new_act;
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
unsafe fn supplied_history(
    state: *const State,
    cargo: CargoType,
    period: u8,
) -> Option<SuppliedHistory> {
    if !is_valid_cargo_type(cargo.0) {
        return None;
    }
    let index = unsafe { find_supplied(state, cargo) };
    unsafe {
        let rows = &(*state).supplied;
        rows.get(index).map(|s| s.history[usize::from(period)])
    }
}
/// Read only the canonical `Town::xy` at the original observation point.
/// SAFETY: The caller supplies a live owner; this immediate copied read retains no borrow.
pub(crate) unsafe fn position(state: *const State) -> TileIndex {
    // SAFETY: Caller establishes owner lifetime and serialized access.
    TileIndex(unsafe { (*state).simulation.xy })
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
/// `GetTownRadiusGroup` over live canonical town fields. No owner borrow crosses
/// a callback. The caller supplies the original live town and valid tile.
pub(crate) unsafe fn get_town_radius_group(
    state: *const State,
    tile: TileIndex,
    map: crate::map_func::Map,
) -> HouseZone {
    let state = unsafe { &*state };
    let dist = map.distance_square(tile, TileIndex(state.simulation.xy));
    if state.growth.funding != 0 && dist <= 25 {
        return HouseZone::TownCentre;
    }
    let mut smallest = HouseZone::TownEdge;
    for i in HouseZones::HZ_ZONE_ALL.iter() {
        if dist < state.simulation.radius[usize::from(i.0)] {
            smallest = i;
        }
    }
    smallest
}
/// `GetTownRadiusGroup` for a live native shell, resolving its canonical owner once.
/// Raw for its callers outside workset 01 (`objects.rs`, `industry/resolver.rs`,
/// `newgrf_roadstop.rs`).
pub(crate) unsafe fn radius_from_shell(
    s: &Services,
    town: *const crate::services::TownHandle,
    tile: u32,
) -> u8 {
    unsafe { get_town_radius_group((s.town_owner)(town), TileIndex(tile), s.map()) }.0
}
/// The road-work inputs of `TileLoop_Road` for a town: `road_build_months`, the
/// tile's `GetTownRadiusGroup` and `DistanceManhattan(t->xy, tile)`, copied so that no
/// owner reference spans a service.
/// SAFETY: Caller supplies a live, serialized owner.
pub(crate) unsafe fn road_context(
    state: *const State,
    tile: TileIndex,
    map: crate::map_func::Map,
) -> (u8, HouseZone, u32) {
    // SAFETY: Immediate reads only, caller establishes owner lifetime.
    let state = unsafe { &*state };
    let distance = map.distance_manhattan(tile, TileIndex(state.simulation.xy));
    let zone = unsafe { get_town_radius_group(state, tile, map) };
    (state.growth.road, zone, distance)
}
static GENERATING_TOWN: GameState<bool> = GameState::new(false);
pub extern "C" fn openttd_rust_town_generating_exchange_abi(generating: bool) -> u8 {
    // SAFETY: Serialized native entry; the flag is process-wide.
    u8::from(unsafe { std::ptr::replace(GENERATING_TOWN.get(), generating) })
}
/// `TownTicksToGameTicks`: convert original town tick counters, including tick
/// zero, to game ticks.
fn town_ticks_to_game_ticks(ticks: u16) -> u16 {
    ((u32::from(ticks.min(MAX_TOWN_GROWTH_TICKS)) + 1) * u32::from(TOWN_GROWTH_TICKS) - 1) as u16
}

/// `UpdateTownRadius`: update the cached town zone radii of a town, based on the
/// number of houses.
unsafe fn update_town_radius(state: *mut State) {
    const TOWN_SQUARED_TOWN_ZONE_RADIUS_DATA: [[u32; NUM_HOUSE_ZONES as usize]; 23] = [
        [4, 0, 0, 0, 0], // 0
        [16, 0, 0, 0, 0],
        [25, 0, 0, 0, 0],
        [36, 0, 0, 0, 0],
        [49, 0, 4, 0, 0],
        [64, 0, 4, 0, 0], // 20
        [64, 0, 9, 0, 1],
        [64, 0, 9, 0, 4],
        [64, 0, 16, 0, 4],
        [81, 0, 16, 0, 4],
        [81, 0, 16, 0, 4], // 40
        [81, 0, 25, 0, 9],
        [81, 36, 25, 0, 9],
        [81, 36, 25, 16, 9],
        [81, 49, 0, 25, 9],
        [81, 64, 0, 25, 9], // 60
        [81, 64, 0, 36, 9],
        [81, 64, 0, 36, 16],
        [100, 81, 0, 49, 16],
        [100, 81, 0, 49, 25],
        [121, 81, 0, 49, 25], // 80
        [121, 81, 0, 49, 25],
        [121, 81, 0, 49, 36], // 88
    ];
    // SAFETY: Immediate field accesses on live owner; this helper invokes no services.
    let num_houses = unsafe { (*state).simulation.num_houses };
    let radius = if (num_houses as usize) < TOWN_SQUARED_TOWN_ZONE_RADIUS_DATA.len() * 4 {
        TOWN_SQUARED_TOWN_ZONE_RADIUS_DATA[(num_houses / 4) as usize]
    } else {
        let mass = (num_houses / 8) as i32;
        /* Actually we are proportional to sqrt() but that's right because we are covering an area.
         * The offsets are to make sure the radii do not decrease in size when going from the table
         * to the calculated value.*/
        let mut radius = [0; NUM_HOUSE_ZONES as usize];
        radius[usize::from(HouseZone::TownEdge.0)] = mass.wrapping_mul(15).wrapping_sub(40) as u32;
        radius[usize::from(HouseZone::TownOutskirt.0)] =
            mass.wrapping_mul(9).wrapping_sub(15) as u32;
        radius[usize::from(HouseZone::TownOuterSuburb.0)] = 0;
        radius[usize::from(HouseZone::TownInnerSuburb.0)] =
            mass.wrapping_mul(5).wrapping_sub(5) as u32;
        radius[usize::from(HouseZone::TownCentre.0)] = mass.wrapping_mul(3).wrapping_add(5) as u32;
        radius
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
        update_town_radius(state);
    }
}
/// `ChangePopulation`.
unsafe fn change_population(leaves: &Leaves, state: *mut State, t: TownID, amount: i32) {
    // SAFETY: Field mutation ends before the presentation service can reenter.
    unsafe {
        (*state).simulation.population = population(state).wrapping_add(amount as u32);
    }
    // SAFETY: Immediate serialized read ends before presentation callbacks.
    if unsafe { *GENERATING_TOWN.get() } {
        return;
    }
    // Cargo requirements may appear/vanish for small populations.
    (leaves.population_changed)(u32::from(t.0));
}
/// `Town::Iterate()`: the valid towns in pool order from `TownID::BEGIN`. `next(from)`
/// is the pool's next valid index at or after `from`, or `u32::MAX` past the end; it is
/// called only when the iteration advances, after the previous town's visit.
pub(crate) fn iterate(mut next: impl FnMut(u32) -> u32) -> impl Iterator<Item = TownId> {
    let mut from = Some(u32::from(TownId::BEGIN.0));
    std::iter::from_fn(move || {
        let id = next(from?);
        if id == u32::MAX {
            from = None;
            return None;
        }
        from = Some(id.wrapping_add(1));
        Some(TownId(id as u16))
    })
}
/// `Town::Iterate()` over a lookup that resolves each town, ending at a null shell.
pub(crate) fn iterate_refs(mut next: impl FnMut(u32) -> TownRef) -> impl Iterator<Item = TownRef> {
    let mut from = Some(u32::from(TownId::BEGIN.0));
    std::iter::from_fn(move || {
        let town = next(from?);
        if town.shell.is_null() {
            from = None;
            return None;
        }
        from = Some(u32::from(town.id.0).wrapping_add(1));
        Some(town)
    })
}
/// `GetWorldPopulation`: the total population of the world.
pub(crate) fn get_world_population(leaves: &Leaves) -> u32 {
    let mut total = 0_u32;
    for town in iterate(|from| leaves.next_town(from)) {
        let owner = (leaves.resolve_owner)(u32::from(town.0));
        // SAFETY: Iterator supplied a live shell; copy read ends before next callback.
        total = total.wrapping_add(unsafe { population(owner) });
    }
    total
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_world_population(leaves: *const Leaves) -> u32 {
    // SAFETY: Immutable table remains live; no owner access spans pool iteration.
    get_world_population(unsafe { &*leaves })
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
/// `Town::GetOrCreateCargoSupplied`.
pub(crate) unsafe fn supplied_entry(state: *mut State, cargo: CargoType) -> SuppliedEntry {
    // SAFETY: Live sole owner, no native vector views survive its mutation.
    let rows = unsafe { &mut (*state).supplied };
    let at = rows.partition_point(|row| row.cargo < cargo.0);
    let created = at == rows.len() || rows[at].cargo != cargo.0;
    if created {
        rows.insert(
            at,
            Supplied {
                cargo: cargo.0,
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
    unsafe { supplied_entry(state, CargoType(cargo)) }
}
/// `Town::GetCargoSupplied`'s lookup: the index of the cargo's supplied row, or the
/// row count when the town supplies none of it.
/// SAFETY: Live serialized owner; no callback or mutation during the read.
pub(crate) unsafe fn find_supplied(state: *const State, cargo: CargoType) -> usize {
    // SAFETY: Immediate shared owner access, with no callbacks or mutation.
    let rows = unsafe { &(*state).supplied };
    let at = rows.partition_point(|row| row.cargo < cargo.0);
    if at == rows.len() || rows[at].cargo != cargo.0 {
        rows.len()
    } else {
        at
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_supplied_find(state: *const State, cargo: u8) -> usize {
    unsafe { find_supplied(state, CargoType(cargo)) }
}
impl SuppliedHistory {
    /// `Town::SuppliedHistory::PercentTransported`.
    pub(crate) fn percent_transported(self) -> u8 {
        if self.production == 0 {
            return 0;
        }
        // Original uint32 multiplication wraps before division and uint8 clamping.
        clamp_to_u8(i64::from(
            self.transported.wrapping_mul(256) / self.production,
        ))
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_percent_transported(
    history: *const SuppliedHistory,
) -> u8 {
    // SAFETY: Native supplies one matching live history; the copy ends the reads.
    unsafe { *history }.percent_transported()
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
pub unsafe extern "C" fn openttd_rust_town_class_counts(state: *mut State) -> Slots {
    // SAFETY: Canonical serialized vector. Native views end before resize.
    let counts = unsafe { &mut (*state).class_counts };
    Slots {
        data: counts.as_mut_ptr().cast(),
        size: counts.len(),
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
/// `IncreaseBuildingCount`: count one built house part in its town and the
/// process-wide totals.
unsafe fn increase_building(state: *mut State, house: HouseID, class: HouseClassID) {
    // SAFETY: Original valid spec/count indices; no callback while borrowed.
    let state = unsafe { &mut *state };
    // SAFETY: All counters are serialized by the game-state mutex.
    let global = unsafe { &mut *BUILDING_COUNTS.get() };
    state.id_counts[usize::from(house)] = state.id_counts[usize::from(house)].wrapping_add(1);
    global.ids[usize::from(house)] = global.ids[usize::from(house)].wrapping_add(1);
    if class != HOUSE_NO_CLASS {
        state.class_counts[usize::from(class)] =
            state.class_counts[usize::from(class)].wrapping_add(1);
        global.classes[usize::from(class)] = global.classes[usize::from(class)].wrapping_add(1);
    }
}
/// `DecreaseBuildingCount`: saturating removal of one house part from town and
/// process-wide totals.
unsafe fn decrease_building(state: *mut State, house: HouseID, class: HouseClassID) {
    // SAFETY: Source valid indices and serialized original lifetime preconditions.
    let state = unsafe { &mut *state };
    // SAFETY: No callback spans the process-wide global count access.
    let global = unsafe { &mut *BUILDING_COUNTS.get() };
    state.id_counts[usize::from(house)] = state.id_counts[usize::from(house)].saturating_sub(1);
    global.ids[usize::from(house)] = global.ids[usize::from(house)].saturating_sub(1);
    if class != HOUSE_NO_CLASS {
        state.class_counts[usize::from(class)] =
            state.class_counts[usize::from(class)].saturating_sub(1);
        global.classes[usize::from(class)] = global.classes[usize::from(class)].saturating_sub(1);
    }
}
/// `TryBuildTownHouse`'s static probability vector of `(HouseID, probability)`.
struct HouseCandidates {
    slots: [std::mem::MaybeUninit<(HouseID, u32)>; NUM_HOUSES as usize],
    len: usize,
}
static HOUSE_CANDIDATES: GameState<HouseCandidates> = GameState::new(HouseCandidates {
    slots: [std::mem::MaybeUninit::uninit(); NUM_HOUSES as usize],
    len: 0,
});
fn clear_house_candidates() {
    unsafe {
        (*HOUSE_CANDIDATES.get()).len = 0;
    }
}
fn add_house_candidate(house: HouseID, weight: u32) {
    // SAFETY: HouseSpec's original maximum is NUM_HOUSES=4096. No callback occurs.
    let candidates = unsafe { &mut *HOUSE_CANDIDATES.get() };
    candidates.slots[candidates.len].write((house, weight));
    candidates.len += 1;
}
fn take_house_candidate(mut random: u32) -> (HouseID, u32) {
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
    pub bulldozer: extern "C" fn() -> bool,
    pub town_index: extern "C" fn(*const crate::services::TownHandle) -> u16,
    pub refusal: u32,
}

/// `_settings_game.difficulty.town_council_tolerance`, from the settings catalog.
pub(crate) fn council_tolerance(s: &Services) -> u8 {
    // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
    unsafe { s.settings().town_council_tolerance.read() }
}
/// `_settings_game.economy.dist_local_authority`, from the settings catalog.
pub(crate) fn authority_distance(s: &Services) -> u8 {
    // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
    unsafe { s.settings().dist_local_authority.read() }
}

/// `CheckIfAuthorityAllowsNewStation`: checks whether the local authority allows
/// construction of a new station (rail, road, airport, dock) on the given tile.
/// No Rust owner borrow crosses a service; errors retain full native parameters.
/// # Safety
/// `result` is live native result storage for the call.
pub(crate) unsafe fn check_if_authority_allows_new_station(
    a: &AuthorityServices,
    s: &Services,
    c: &crate::services::CommandCostServices,
    result: *mut crate::services::CommandCost,
    tile: TileIndex,
    flags: DoCommandFlags,
) {
    /* The required rating is hardcoded to RATING_VERYPOOR (see below), not the authority attitude setting, so we can bail out like this. */
    if TownCouncilAttitudes(council_tolerance(s)) == TownCouncilAttitudes::TOWN_COUNCIL_PERMISSIVE {
        (c.zero)(result);
        return;
    }
    let company = (s.current_company)();
    if !(s.company_valid)(company) || flags.test(DoCommandFlag::NoTestTownRating) {
        (c.zero)(result);
        return;
    }
    let town = spatial::closest_town_from_tile(s, tile, u32::from(authority_distance(s)));
    if town.is_null() {
        (c.zero)(result);
        return;
    }
    let owner = (s.town_owner)(town);
    // SAFETY: Live single owner and valid company; immediate copy ends here.
    let rating = unsafe { (*owner).simulation.ratings[usize::from(company)] };
    if i32::from(rating) > RATING_VERYPOOR {
        (c.zero)(result);
    } else {
        (c.error_param)(result, a.refusal, u64::from((a.town_index)(town)));
    }
}
/// The command flags of a positional export's `NoTestTownRating` boolean.
fn no_test_flags(no_test: bool) -> DoCommandFlags {
    if no_test {
        DoCommandFlags::from_element(DoCommandFlag::NoTestTownRating)
    } else {
        DoCommandFlags(0)
    }
}
/// Native stack result and immutable tables remain live through this invocation.
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
    unsafe {
        check_if_authority_allows_new_station(
            &*authority,
            &*services,
            &*costs,
            result,
            TileIndex(tile),
            no_test_flags(no_test),
        );
    }
}

/// `CheckforTownRating`: does the town authority allow the (destructive) action of the
/// current company? A null town always allows it.
/// # Safety
/// `town` is null or a live shell; `result` is live native result storage.
pub(crate) unsafe fn check_for_town_rating(
    a: &AuthorityServices,
    s: &Services,
    c: &crate::services::CommandCostServices,
    result: *mut crate::services::CommandCost,
    flags: DoCommandFlags,
    town: *const crate::services::TownHandle,
    kind: TownRatingCheckType,
) {
    /* minimum rating needed to be allowed to remove stuff */
    const NEEDED_RATING: [[i32; TownRatingCheckType::End.0 as usize]; 4] = [
        /* RoadRemove, TunnelBridgeRemove */
        [
            RATING_ROAD_NEEDED_LENIENT,
            RATING_TUNNEL_BRIDGE_NEEDED_LENIENT,
        ], // Lenient
        [
            RATING_ROAD_NEEDED_NEUTRAL,
            RATING_TUNNEL_BRIDGE_NEEDED_NEUTRAL,
        ], // Neutral
        [
            RATING_ROAD_NEEDED_HOSTILE,
            RATING_TUNNEL_BRIDGE_NEEDED_HOSTILE,
        ], // Hostile
        [
            RATING_ROAD_NEEDED_PERMISSIVE,
            RATING_TUNNEL_BRIDGE_NEEDED_PERMISSIVE,
        ], // Permissive
    ];
    /* if magic_bulldozer cheat is active, town doesn't restrict your destructive actions */
    if town.is_null() {
        (c.zero)(result);
        return;
    }
    let company = (s.current_company)();
    if !(s.company_valid)(company) || (a.bulldozer)() || flags.test(DoCommandFlag::NoTestTownRating)
    {
        (c.zero)(result);
        return;
    }
    /* check if you're allowed to remove the road/bridge/tunnel
     * owned by a town no removal if rating is lower than ... depends now on
     * difficulty setting. Minimum town rating selected by difficulty level
     */
    let needed = NEEDED_RATING[usize::from(council_tolerance(s))][usize::from(kind.0)];
    let owner = (s.town_owner)(town);
    // SAFETY: Live owner, valid company; scratch/native rating access ends here.
    let rating = unsafe { get_rating(owner, town, CompanyID(company)) };
    if rating < needed {
        (c.error_param)(result, a.refusal, u64::from((a.town_index)(town)));
    } else {
        (c.zero)(result);
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
    unsafe {
        check_for_town_rating(
            &*authority,
            &*services,
            &*costs,
            result,
            no_test_flags(no_test),
            town,
            TownRatingCheckType(kind),
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub shared: *const Services,
    pub map_log_x: extern "C" fn() -> u32,
    pub environment: crate::services::EnvironmentServices,
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
    pub build_bridge: extern "C" fn(u32, u32, u32, u32, u32) -> bool,
    pub build_tunnel: extern "C" fn(u32, u32, u32) -> bool,
    pub assert_clear_house: extern "C" fn(bool),
    pub houses: *const house::HouseServices,
}
impl Leaves {
    pub(crate) fn shared(&self) -> &Services {
        // SAFETY: Every town table points at its live immutable shared services.
        unsafe { &*self.shared }
    }
    pub(crate) fn generating_world(&self) -> bool {
        (self.shared().clear_square_services.generating_world)()
    }
    pub(crate) fn next_town(&self, from: u32) -> u32 {
        (self.shared().town_next_id)(from)
    }
}
pub type StationActivity = crate::station_service::StationActivity;
#[derive(Clone, Copy)]
pub struct CommandResult {
    pub cost: i64,
    pub success: bool,
}
/// The possible states of town growth (`town_cmd.cpp`), over its `int` underlying type.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct TownGrowthResult(i32);
#[allow(
    non_upper_case_globals,
    reason = "original town_cmd.cpp TownGrowthResult enumerators"
)]
impl TownGrowthResult {
    const Succeed: Self = Self(0);
    const SearchStopped: Self = Self(1);
    const Continue: Self = Self(2);
}
/// The construction state `MakeTownHouse` writes into every part of a house.
#[derive(Clone, Copy)]
struct HouseConstruction {
    /// The counter of the construction stage.
    counter: u8,
    /// The current construction stage.
    stage: u8,
    /// Random bits for newgrf houses to use.
    random_bits: u8,
    /// Whether the house is protected from the town upgrading it.
    is_protected: bool,
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
/// Map geometry and tile accessors in the original vocabulary. Each delegates to
/// the shared map arithmetic or one native tile service.
impl Engine<'_> {
    /// `_settings_game.economy.town_growth_rate`, from the settings catalog.
    fn growth_rate_setting(&self) -> u8 {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe { self.services.settings().town_growth_rate.read() }
    }
    /// `_settings_game.economy.allow_town_level_crossings`, from the settings catalog.
    fn allow_town_level_crossings(&self) -> bool {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe { self.services.settings().allow_town_level_crossings.read() }
    }
    fn map(&self) -> Map {
        *self.map.get_or_init(|| {
            let mut map = self.services.map();
            map.log_x = (self.leaves.map_log_x)();
            map
        })
    }
    /// `Map::SizeY()`, read at each original observation point.
    fn size_y(&self) -> u32 {
        crate::map_func::World::new(self.services).size_y()
    }
    /// The native assertion handler for the shared header preconditions.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| crate::movement::native_assert(self.services, ok)
    }
    fn is_valid_tile(&self, tile: TileIndex) -> bool {
        (self.services.tile_valid)(tile.0)
    }
    fn get_tile_type(&self, tile: TileIndex) -> TileType {
        TileType((self.services.tile_type)(tile.0))
    }
    fn is_tile_type(&self, tile: TileIndex, kind: TileType) -> bool {
        self.get_tile_type(tile) == kind
    }
    fn get_tile_slope(&self, tile: TileIndex) -> Slope {
        Slope((self.services.tile_slope)(tile.0))
    }
    fn get_tile_z(&self, tile: TileIndex) -> i32 {
        (self.services.tile_z)(tile.0)
    }
    fn get_tile_max_z(&self, tile: TileIndex) -> i32 {
        (self.services.tile_max_z)(tile.0)
    }
    fn has_tile_water_ground(&self, tile: TileIndex) -> bool {
        (self.services.has_water_ground)(tile.0)
    }
    fn is_bridge_above(&self, tile: TileIndex) -> bool {
        crate::map_func::World::new(self.services).bridge_above(tile.0)
    }
    fn get_tunnel_bridge_transport_type(&self, tile: TileIndex) -> TransportType {
        TransportType((self.services.tunnel_transport)(tile.0))
    }
    fn get_tunnel_bridge_direction(&self, tile: TileIndex) -> DiagDirection {
        DiagDirection((self.services.tunnel_bridge_direction)(tile.0))
    }
    fn get_road_type_road(&self, tile: TileIndex) -> RoadType {
        RoadType((self.services.road_type)(tile.0))
    }
    /// `GetRoadTypeInfo(rt)->flags`.
    fn road_type_flags(&self, rt: RoadType) -> RoadTypeFlags {
        RoadTypeFlags((self.leaves.road_flags)(rt.0))
    }
    /// Return the `RoadBits` of a tile, ignoring depot and bay road stops (`GetTownRoadBits`).
    fn get_town_road_bits(&self, tile: TileIndex) -> RoadBits {
        if (self.services.is_road_depot)(tile.0) || (self.services.is_bay_stop)(tile.0) {
            return RoadBits::ROAD_NONE;
        }
        RoadBits((self.services.road_bits)(
            tile.0,
            u8::from(RoadTramType::RTT_ROAD.0),
            true,
        ))
    }
    /// `RandomDiagDir`.
    fn random_diag_dir(&self) -> DiagDirection {
        DiagDirection(
            self.services
                .random_range(u32::from(DiagDirection::DIAGDIR_END.0)) as u8,
        )
    }
}
/// The bound town's fields, each read or written at its original observation point.
impl Engine<'_> {
    fn bind(mut self, town: TownID) -> Self {
        self.owner = (self.leaves.resolve_owner)(u32::from(town.0));
        self
    }
    /// `Town::xy`.
    fn xy(&self) -> TileIndex {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { position(self.owner) }
    }
    /// `Town::cache.num_houses`.
    fn num_houses(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { num_houses(self.owner) }
    }
    fn set_num_houses(&self, value: u32) {
        write_field!(self.owner, simulation.num_houses, value);
    }
    /// `Town::cache.population`.
    fn population(&self) -> u32 {
        // SAFETY: Stable live owner; each immediate field read ends before services.
        unsafe { population(self.owner) }
    }
    /// `assert(tile < Map::Size())`, under the native build's assertion policy.
    fn assert_tile_in_map(&self, tile: TileIndex) {
        let world = crate::map_func::World::new(self.services);
        world.assert_with(|| tile.0 < self.services.map_size());
    }
    /// `NOT_REACHED()`: the native fatal error; it does not return.
    fn not_reached(&self) -> ! {
        (self.services.not_reached)();
        unreachable!("NOT_REACHED returned")
    }
    fn layout(&self) -> TownLayout {
        TownLayout(read_field!(self.owner, simulation.layout))
    }
    fn larger_town(&self) -> bool {
        read_field!(self.owner, simulation.larger_town)
    }
    /// `Town::cache.squared_town_zone_radius`.
    fn squared_town_zone_radius(&self) -> [u32; NUM_HOUSE_ZONES as usize] {
        read_field!(self.owner, simulation.radius)
    }
    fn goal(&self) -> [u32; NUM_TAE] {
        read_field!(self.owner, simulation.goal)
    }
    /// `Town::received[i].old_act`.
    fn received_old_act(&self, i: usize) -> u32 {
        // SAFETY: Source bounds and immediate owner access before any callback.
        unsafe { u32::from((*self.owner).simulation.received[i].old_act) }
    }
    fn grow_counter(&self) -> u16 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { (*self.owner).growth.counter }
    }
    fn set_grow_counter(&self, value: u16) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.counter = value;
        }
    }
    fn growth_rate(&self) -> u16 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { (*self.owner).growth.rate }
    }
    fn set_growth_rate(&self, value: u16) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.rate = value;
        }
    }
    fn fund_buildings_months(&self) -> u8 {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        unsafe { (*self.owner).growth.funding }
    }
    fn set_fund_buildings_months(&self, value: u8) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.funding = value;
        }
    }
    fn flags(&self) -> TownFlags {
        // SAFETY: Entry resolves its live owner once; immediate access ends here.
        TownFlags(unsafe { (*self.owner).growth.flags })
    }
    fn set_flags(&self, value: TownFlags) {
        // SAFETY: No owner reference spans a callback or possible reentry.
        unsafe {
            (*self.owner).growth.flags = value.0;
        }
    }
    /// `t->flags.Set(flag)`.
    fn flags_set(&self, flag: TownFlag) {
        let mut flags = self.flags();
        flags.set(flag);
        self.set_flags(flags);
    }
    /// `t->flags.Reset(flag)`.
    fn flags_reset(&self, flag: TownFlag) {
        let mut flags = self.flags();
        flags.reset(flag);
        self.set_flags(flags);
    }
}
/// Commands the growth engine issues; each completes before the next owner access.
impl Engine<'_> {
    /// `Command<CMD_BUILD_ROAD>::Do(flags, tile, rcmd, rt, DRD_NONE, town).Succeeded()`.
    fn cmd_build_road(
        &self,
        flags: DoCommandFlags,
        tile: TileIndex,
        rcmd: RoadBits,
        rt: RoadType,
        town: TownID,
    ) -> bool {
        // SAFETY: growth entries supply a live outer native stack context.
        unsafe { (&*self.road).build(flags.0, tile.0, rcmd.0, rt.0, town.0) }
    }
    /// `CMD_LANDSCAPE_CLEAR` or, with its slope and direction, `CMD_TERRAFORM_LAND`.
    fn landscape_command(
        &self,
        flags: DoCommandFlags,
        tile: TileIndex,
        terraform: Option<(Slope, bool)>,
    ) -> CommandResult {
        let commands = unsafe { &*(*self.leaves.houses).commands };
        let table = unsafe { &*(commands.landscape_commands)() };
        let costs = unsafe { &*table.costs };
        unsafe {
            /* SAFETY: synchronous costs never escape or outlive this call. */
            crate::command::with_cost(costs, |out| {
                if let Some((edges, dir)) = terraform {
                    crate::landscape_cmd::terraform_command(
                        commands, out.0, tile.0, edges.0, dir, flags.0,
                    );
                } else {
                    crate::landscape_cmd::clear_command(commands, out.0, tile.0, flags.0);
                }
                CommandResult {
                    cost: out.money(),
                    success: out.ok(),
                }
            })
        }
    }
    /// `Command<CMD_LANDSCAPE_CLEAR>::Do(flags, tile)`.
    fn cmd_landscape_clear(&self, flags: DoCommandFlags, tile: TileIndex) -> CommandResult {
        self.landscape_command(flags, tile, None)
    }
    /// `Command<CMD_TERRAFORM_LAND>::Do(flags, tile, edges, dir)`.
    fn cmd_terraform_land(
        &self,
        flags: DoCommandFlags,
        tile: TileIndex,
        edges: Slope,
        dir: bool,
    ) -> CommandResult {
        self.landscape_command(flags, tile, Some((edges, dir)))
    }
}
/// `{DoCommandFlag::Auto, DoCommandFlag::NoWater}`.
const DC_AUTO_NO_WATER: DoCommandFlags =
    DoCommandFlags(DoCommandFlag::AUTO | DoCommandFlag::NO_WATER);
/// `{DoCommandFlag::Execute, DoCommandFlag::Auto, DoCommandFlag::NoWater}`.
const DC_EXECUTE_AUTO_NO_WATER: DoCommandFlags =
    DoCommandFlags(DoCommandFlag::EXECUTE | DoCommandFlag::AUTO | DoCommandFlag::NO_WATER);
/// `{DoCommandFlag::Execute, DoCommandFlag::Auto}`.
const DC_EXECUTE_AUTO: DoCommandFlags =
    DoCommandFlags(DoCommandFlag::EXECUTE | DoCommandFlag::AUTO);
/// Growth rates, station ratings and growth state (`town_cmd.cpp`).
impl Engine<'_> {
    /// `ForAllStationsNearTown`: `func` receives the station catchment table with
    /// each station within the town edge zone.
    fn for_all_stations_near_town(
        &self,
        mut func: impl FnMut(&crate::station::CatchmentServices, *mut crate::station::BaseStation),
    ) {
        let xy = self.xy();
        let radius = self.squared_town_zone_radius()[usize::from(HouseZone::TownEdge.0)];
        /* Ideally the search radius should be close to the actual town zone 0 radius.
         * The true radius is not stored or calculated anywhere, only the squared radius. */
        /* The efficiency of this search might be improved for large towns and many stations on the map,
         * by using an integer square root approximation giving a value not less than the true square root. */
        let search_radius = radius / 2;
        let c = unsafe { &*(self.services.station_catchment)() };
        crate::station::spatial::for_all_stations_radius(self.services, xy, search_radius, |st| {
            if self.map().distance_square(TileIndex((c.location)(st)), xy) <= radius {
                func(c, st);
            }
        });
    }
    /// `CountActiveStations`: the amount of active stations in the range of the
    /// town (`HZB_TOWN_EDGE`).
    fn count_active_stations(&self) -> u32 {
        let mut n = 0_u32;
        self.for_all_stations_near_town(|c, st| {
            let activity = unsafe { crate::station_service::activity((c.owner)(st)) };
            if activity.time_since_load <= 20 || activity.time_since_unload <= 20 {
                n = n.wrapping_add(1);
            }
        });
        n
    }
    /// `UpdateTownGrowCounter`: update the grow counter after a growth rate change.
    /// Preserves relative house building progress whenever it can.
    fn update_town_grow_counter(&self, prev_growth_rate: u16) {
        if self.growth_rate() == TOWN_GROWTH_RATE_NONE {
            return;
        }
        if prev_growth_rate == TOWN_GROWTH_RATE_NONE {
            self.set_grow_counter(self.growth_rate().min(self.grow_counter()));
            return;
        }
        let numerator = u32::from(self.grow_counter()) * (u32::from(self.growth_rate()) + 1);
        self.set_grow_counter(
            round_div_su(numerator as i32, u32::from(prev_growth_rate) + 1) as u16,
        );
    }
    /// `GetNormalGrowthRate`: the growth rate in normal conditions (custom growth
    /// rate not set). If town growth speed is set to None(0) returns the same rate
    /// as if it was Normal(2).
    fn get_normal_growth_rate(&self) -> u16 {
        /*
         * Note:
         * Unserviced+unfunded towns get an additional malus in UpdateTownGrowth(),
         * so the "320" is actually not better than the "420".
         */
        const GROW_COUNT_VALUES: [[u32; 6]; 2] = [
            [120, 120, 120, 100, 80, 60],   // Fund new buildings has been activated
            [320, 420, 300, 220, 160, 100], // Normal values
        ];
        let n = self.count_active_stations();
        let mut m =
            GROW_COUNT_VALUES[usize::from(self.fund_buildings_months() == 0)][n.min(5) as usize];
        let growth_multiplier = if u32::from(self.growth_rate_setting()) == 0 {
            1
        } else {
            u32::from(self.growth_rate_setting()) - 1
        };
        m >>= growth_multiplier;
        if self.larger_town() {
            m /= 2;
        }
        town_ticks_to_game_ticks((m / (self.num_houses() / 50 + 1)) as u16)
    }
    /// `UpdateTownGrowthRate`.
    fn update_town_growth_rate(&self, t: TownID) {
        if self.flags().test(TownFlag::CustomGrowth) {
            return;
        }
        let old_rate = self.growth_rate();
        self.set_growth_rate(self.get_normal_growth_rate());
        self.update_town_grow_counter(old_rate);
        (self.leaves.dirty_town)(u32::from(t.0));
    }
    /// `UpdateTownGrowth`: update the growth state (whether the town is growing or not).
    fn update_town_growth(&self, t: TownID) {
        self.update_town_growth_rate(t);
        self.flags_reset(TownFlag::IsGrowing);
        (self.leaves.dirty_town)(u32::from(t.0));
        if u32::from(self.growth_rate_setting()) == 0 && self.fund_buildings_months() == 0 {
            return;
        }
        if self.fund_buildings_months() == 0 {
            /* Check if all goals are reached for this town to grow (given we are not funding it) */
            for tae in TownAcceptanceEffect::TAE_BEGIN.0..TownAcceptanceEffect::TAE_END.0 {
                let i = usize::from(tae);
                match self.goal()[i] {
                    TOWN_GROWTH_WINTER => {
                        if crate::tile_map::tile_height_in(
                            &crate::map_func::World::new(self.services),
                            self.xy().0,
                        ) >= u32::from(crate::snow_line::get(self.services))
                            && self.received_old_act(i) == 0
                            && self.population() > 90
                        {
                            return;
                        }
                    }
                    TOWN_GROWTH_DESERT => {
                        if (self.services.tropic_zone)(self.xy().0) == TROPICZONE_DESERT
                            && self.received_old_act(i) == 0
                            && self.population() > 60
                        {
                            return;
                        }
                    }
                    goal => {
                        if goal > self.received_old_act(i) {
                            return;
                        }
                    }
                }
            }
        }
        if self.flags().test(TownFlag::CustomGrowth) {
            if self.growth_rate() != TOWN_GROWTH_RATE_NONE {
                self.flags_set(TownFlag::IsGrowing);
            }
            (self.leaves.dirty_town)(u32::from(t.0));
            return;
        }
        if self.fund_buildings_months() == 0
            && self.count_active_stations() == 0
            && !chance16(self.services, 1, 12)
        {
            return;
        }
        self.flags_set(TownFlag::IsGrowing);
        (self.leaves.dirty_town)(u32::from(t.0));
    }
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
/// Town road building (`town_cmd.cpp`).
impl Engine<'_> {
    /// `GetTownRoadType`: the road type that towns should build at this current
    /// moment. They may have built a different type in the past.
    fn get_town_road_type(&self) -> RoadType {
        const ASSUME_MAX_SPEED: u16 = 50;
        let mut best_rt = RoadType::ROADTYPE_ROAD;
        let mut best_speed = None;
        let mut mask = crate::road::get_mask_for_road_tram_type(crate::road_type::RTT_ROAD);
        while mask != 0 {
            let rt = RoadType(mask.trailing_zeros() as u8);
            mask &= mask - 1;
            let rti = (self.leaves.road_info)(rt.0);
            /* Can town build this road. */
            if !rti.town_build {
                continue;
            }
            /* Not yet introduced at this date. */
            let date = rti.introduction_date;
            if (0..MAX_DATE).contains(&date) && date > crate::timer_game_common::calendar_date().0 {
                continue;
            }
            let speed = if rti.max_speed == 0 {
                ASSUME_MAX_SPEED
            } else {
                rti.max_speed
            };
            if best_speed.is_some_and(|best| speed < best) {
                continue;
            }
            best_rt = rt;
            best_speed = Some(speed);
        }
        best_rt
    }
    /// `IsNeighbourRoadTile`: check for parallel road inside a given distance.
    /// Assuming a road from (tile - `TileOffsByDiagDir(dir)`) to tile, is there a
    /// parallel road left or right of it within distance `dist_multi`?
    fn is_neighbour_road_tile(&self, tile: TileIndex, dir: DiagDirection, dist_multi: u32) -> bool {
        if !self.is_valid_tile(tile) {
            return false;
        }
        /* Lookup table for the used diff values */
        let tid_lt = [
            self.map().tile_offs_by_diag_dir(
                change_diag_dir(dir, DiagDirDiff::DIAGDIRDIFF_90RIGHT, self.check()),
                self.check(),
            ),
            self.map().tile_offs_by_diag_dir(
                change_diag_dir(dir, DiagDirDiff::DIAGDIRDIFF_90LEFT, self.check()),
                self.check(),
            ),
            self.map()
                .tile_offs_by_diag_dir(reverse_diag_dir(dir, self.check()), self.check()),
        ];
        let dist_multi = (dist_multi + 1) * 4;
        for pos in 4..dist_multi {
            /* Go (pos / 4) tiles to the left or the right */
            let mut cur = tid_lt[usize::from(pos & 1 == 0)].wrapping_mul((pos / 4).cast_signed());
            /* Use the current tile as origin, or go one tile backwards */
            if pos & 2 != 0 {
                cur = cur.wrapping_add(tid_lt[2]);
            }
            /* Test for roadbit parallel to dir and facing towards the middle axis */
            let test = TileIndex(tile.0.wrapping_add_signed(cur));
            if self.is_valid_tile(test)
                && self.get_town_road_bits(test)
                    & diag_dir_to_road_bits(
                        if pos & 2 != 0 {
                            dir
                        } else {
                            reverse_diag_dir(dir, self.check())
                        },
                        self.check(),
                    )
                    != RoadBits::ROAD_NONE
            {
                return true;
            }
        }
        false
    }
    /// `IsRoadAllowedHere`: check if a road is allowed on a given tile, extending
    /// the town in `dir`.
    fn is_road_allowed_here(&self, t: TownID, tile: TileIndex, dir: DiagDirection) -> bool {
        if self.map().distance_from_edge(tile, self.size_y()) == 0 {
            return false;
        }
        /* Prevent towns from building roads under bridges along the bridge. Looks silly. */
        if self.is_bridge_above(tile)
            && Axis((self.services.bridge_axis)(tile.0)) == diag_dir_to_axis(dir, self.check())
        {
            return false;
        }
        /* Check if there already is a road at this point? */
        if self.get_town_road_bits(tile) == RoadBits::ROAD_NONE {
            /* No, try if we are able to build a road piece there.
             * If that fails clear the land, and if that fails exit.
             * This is to make sure that we can build a road here later. */
            let rt = self.get_town_road_type();
            let rcmd = if dir == DiagDirection::DIAGDIR_NW || dir == DiagDirection::DIAGDIR_SE {
                RoadBits::ROAD_Y
            } else {
                RoadBits::ROAD_X
            };
            if !self.cmd_build_road(DC_AUTO_NO_WATER, tile, rcmd, rt, t)
                && !self.cmd_landscape_clear(DC_AUTO_NO_WATER, tile).success
            {
                return false;
            }
        }
        let cur_slope = if (self.services.build_on_slopes)() {
            crate::movement::foundation_slope(self.services, tile.0).slope
        } else {
            self.get_tile_slope(tile)
        };
        let ret = !self.is_neighbour_road_tile(
            tile,
            dir,
            if self.layout() == TownLayout::TL_ORIGINAL {
                1
            } else {
                2
            },
        );
        if cur_slope == Slope::SLOPE_FLAT {
            return ret;
        }
        /* If the tile is not a slope in the right direction, then
         * maybe terraform some. */
        let desired_slope = if dir == DiagDirection::DIAGDIR_NW || dir == DiagDirection::DIAGDIR_SE
        {
            Slope::SLOPE_NW
        } else {
            Slope::SLOPE_NE
        };
        if desired_slope != cur_slope && complement_slope(desired_slope, self.check()) != cur_slope
        {
            if chance16(self.services, 1, 8) {
                let mut res = false;
                if !self.leaves.generating_world() && chance16(self.services, 1, 10) {
                    /* Note: Do not replace "^ SLOPE_ELEVATED" with ComplementSlope(). The slope might be steep. */
                    let edges = if chance16(self.services, 1, 16) {
                        cur_slope
                    } else {
                        cur_slope ^ Slope::SLOPE_ELEVATED
                    };
                    res = self
                        .cmd_terraform_land(DC_EXECUTE_AUTO_NO_WATER, tile, edges, false)
                        .success;
                }
                if !res && chance16(self.services, 1, 3) {
                    /* We can consider building on the slope, though. */
                    return ret;
                }
            }
            return false;
        }
        ret
    }
    /// `TerraformTownTile`.
    fn terraform_town_tile(&self, tile: TileIndex, edges: Slope, dir: bool) -> bool {
        self.assert_tile_in_map(tile);

        let r = self.cmd_terraform_land(DC_AUTO_NO_WATER, tile, edges, dir);
        if !r.success || r.cost >= ((self.leaves.terraform_price)() + 2) * 8 {
            return false;
        }
        self.cmd_terraform_land(DC_EXECUTE_AUTO_NO_WATER, tile, edges, dir);
        true
    }
    /// `LevelTownLand`.
    fn level_town_land(&self, tile: TileIndex) {
        self.assert_tile_in_map(tile);

        /* Don't terraform if land is plain or if there's a house there. */
        if self.is_tile_type(tile, TileType::MP_HOUSE) {
            return;
        }
        let tileh = self.get_tile_slope(tile);
        if tileh == Slope::SLOPE_FLAT {
            return;
        }
        /* First try up, then down */
        if !self.terraform_town_tile(tile, !tileh & Slope::SLOPE_ELEVATED, true) {
            self.terraform_town_tile(tile, tileh & Slope::SLOPE_ELEVATED, false);
        }
    }
    /// `GetTownRoadGridElement`: the `RoadBits` of a grid tile, growing towards
    /// `dir`, regarding the town's grid layout.
    fn get_town_road_grid_element(&self, tile: TileIndex, dir: DiagDirection) -> RoadBits {
        /* align the grid to the downtown */
        let grid_pos = self.map().tile_index_to_tile_index_diff_c(self.xy(), tile); // Vector from downtown to the tile
        let mut rcmd = RoadBits::ROAD_NONE;
        match self.layout() {
            TownLayout::TL_2X2_GRID => {
                if grid_pos.x % 3 == 0 {
                    rcmd |= RoadBits::ROAD_Y;
                }
                if grid_pos.y % 3 == 0 {
                    rcmd |= RoadBits::ROAD_X;
                }
            }
            TownLayout::TL_3X3_GRID => {
                if grid_pos.x % 4 == 0 {
                    rcmd |= RoadBits::ROAD_Y;
                }
                if grid_pos.y % 4 == 0 {
                    rcmd |= RoadBits::ROAD_X;
                }
            }
            _ => self.not_reached(),
        }
        /* Optimise only X-junctions */
        if rcmd != RoadBits::ROAD_ALL {
            return rcmd;
        }
        let rb_template = match self.get_tile_slope(tile) {
            Slope::SLOPE_W => RoadBits::ROAD_NW | RoadBits::ROAD_SW,
            Slope::SLOPE_SW => RoadBits::ROAD_Y | RoadBits::ROAD_SW,
            Slope::SLOPE_S => RoadBits::ROAD_SW | RoadBits::ROAD_SE,
            Slope::SLOPE_SE => RoadBits::ROAD_X | RoadBits::ROAD_SE,
            Slope::SLOPE_E => RoadBits::ROAD_SE | RoadBits::ROAD_NE,
            Slope::SLOPE_NE => RoadBits::ROAD_Y | RoadBits::ROAD_NE,
            Slope::SLOPE_N => RoadBits::ROAD_NE | RoadBits::ROAD_NW,
            Slope::SLOPE_NW => RoadBits::ROAD_X | RoadBits::ROAD_NW,
            Slope::SLOPE_STEEP_W
            | Slope::SLOPE_STEEP_S
            | Slope::SLOPE_STEEP_E
            | Slope::SLOPE_STEEP_N => RoadBits::ROAD_NONE,
            _ => RoadBits::ROAD_ALL,
        };
        /* Stop if the template is compatible to the growth dir */
        if diag_dir_to_road_bits(reverse_diag_dir(dir, self.check()), self.check()) & rb_template
            != RoadBits::ROAD_NONE
        {
            return rb_template;
        }
        /* If not generate a straight road in the direction of the growth */
        diag_dir_to_road_bits(dir, self.check())
            | diag_dir_to_road_bits(reverse_diag_dir(dir, self.check()), self.check())
    }
    /// `GrowTownWithExtraHouse`: check if there are enough neighbour house tiles
    /// next to the current tile. If there are enough add another house.
    fn grow_town_with_extra_house(
        &self,
        t: TownID,
        tile: TileIndex,
        modes: TownExpandModes,
    ) -> bool {
        /* We can't look further than that. */
        if self.map().distance_from_edge(tile, self.size_y()) == 0 {
            return false;
        }
        let mut counter = 0; // counts the house neighbour tiles

        /* Check the tiles E,N,W and S of the current tile for houses */
        for dir in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
            /* Count both void and house tiles for checking whether there
             * are enough houses in the area. This to make it likely that
             * houses get build up to the edge of the map. */
            if matches!(
                self.get_tile_type(self.map().tile_add_by_diag_dir(
                    tile,
                    DiagDirection(dir),
                    self.check()
                )),
                TileType::MP_HOUSE | TileType::MP_VOID
            ) {
                counter += 1;
            }
            /* If there are enough neighbours stop here */
            if counter >= 3 {
                return self.try_build_town_house(t, tile, modes);
            }
        }
        false
    }
    /// `GrowTownWithRoad`: grow the town with a road piece.
    fn grow_town_with_road(&self, t: TownID, tile: TileIndex, rcmd: RoadBits) -> bool {
        let rt = self.get_town_road_type();
        self.cmd_build_road(DC_EXECUTE_AUTO_NO_WATER, tile, rcmd, rt, t)
    }
    /// `CanRoadContinueIntoNextTile`: check if a town road can be continued into
    /// the next tile. Road vehicle stations, bridges, and tunnels are fine, as
    /// long as they are facing the right direction.
    fn can_road_continue_into_next_tile(
        &self,
        t: TownID,
        tile: TileIndex,
        road_dir: DiagDirection,
    ) -> bool {
        let delta = self.map().tile_offs_by_diag_dir(road_dir, self.check()); // +1 tile in the direction of the road
        let next_tile = TileIndex(tile.0.wrapping_add_signed(delta)); // The tile beyond which must be connectable to the target tile
        let rt = self.get_town_road_type();
        /* Before we try anything, make sure the tile is on the map and not the void. */
        if !self.is_valid_tile(next_tile) {
            return false;
        }
        match self.get_tile_type(next_tile) {
            /* If the next tile is a bridge or tunnel, allow if it's continuing in the same direction. */
            TileType::MP_TUNNELBRIDGE => {
                return self.get_tunnel_bridge_transport_type(next_tile)
                    == TransportType::TRANSPORT_ROAD
                    && self.get_tunnel_bridge_direction(next_tile) == road_dir;
            }
            /* If the next tile is a station, allow if it's a road station facing the proper direction. Otherwise return false. */
            TileType::MP_STATION => {
                /* If the next tile is a road station, allow if it can be entered by the new tunnel/bridge, otherwise disallow. */
                if (self.services.is_drive_through_stop)(next_tile.0) {
                    return Axis((self.services.drive_through_axis)(next_tile.0))
                        == diag_dir_to_axis(road_dir, self.check());
                }
                if (self.services.is_bay_stop)(next_tile.0) {
                    return DiagDirection((self.services.bay_stop_direction)(next_tile.0))
                        == reverse_diag_dir(road_dir, self.check());
                }
                return false;
            }
            /* If the next tile is a road depot, allow if it's facing the right way. */
            TileType::MP_ROAD => {
                return (self.services.is_road_depot)(next_tile.0)
                    && DiagDirection((self.services.road_depot_direction)(next_tile.0))
                        == reverse_diag_dir(road_dir, self.check());
            }
            /* If the next tile is a railroad track, check if towns are allowed to build level crossings.
             * If level crossing are not allowed, reject the construction. Else allow DoCommand to determine if the rail track is buildable. */
            TileType::MP_RAILWAY => {
                if !self.allow_town_level_crossings() {
                    return false;
                }
            }
            _ => {}
        }
        /* If a road tile can be built, the construction is allowed. */
        let rcmd = diag_dir_to_road_bits(reverse_diag_dir(road_dir, self.check()), self.check());
        self.cmd_build_road(DC_AUTO_NO_WATER, next_tile, rcmd, rt, t)
    }
    /// `GrowTownWithBridge`: at first we check if a bridge is reasonable. If so we
    /// check if we are able to build it.
    fn grow_town_with_bridge(&self, t: TownID, tile: TileIndex, bridge_dir: DiagDirection) -> bool {
        const TOWN_BRIDGE_LENGTH_CAP: u32 = 11;
        let slope = self.get_tile_slope(tile);
        /* Make sure the direction is compatible with the slope.
         * Well we check if the slope has an up bit set in the
         * reverse direction. */
        if slope != Slope::SLOPE_FLAT
            && slope & inclined_slope(bridge_dir, || self.not_reached()) != Slope::SLOPE_FLAT
        {
            return false;
        }
        /* Assure that the bridge is connectable to the start side */
        if self.get_town_road_bits(self.map().tile_add_by_diag_dir(
            tile,
            reverse_diag_dir(bridge_dir, self.check()),
            self.check(),
        )) & diag_dir_to_road_bits(bridge_dir, self.check())
            == RoadBits::ROAD_NONE
        {
            return false;
        }
        /* We are in the right direction */
        let mut bridge_length = 0_u32; // This value stores the length of the possible bridge
        let mut bridge_tile = tile; // Used to store the other waterside

        /* To prevent really small towns from building disproportionately
         * long bridges, make the max a function of its population. */
        let base_bridge_length = 5;
        let max_bridge_length =
            (self.population() / 1000 + base_bridge_length).min(TOWN_BRIDGE_LENGTH_CAP);
        let world = || crate::map_func::World::new(self.services);
        let one_way_road = |tile: TileIndex| {
            world().is_normal_road_tile(tile.0)
                && DisallowedRoadDirections(world().disallowed_road_directions(tile.0))
                    != DisallowedRoadDirections::DRD_NONE
        };
        if slope == Slope::SLOPE_FLAT {
            /* Bridges starting on flat tiles are only allowed when crossing rivers, rails or one-way roads. */
            loop {
                let length = bridge_length;
                bridge_length += 1;
                if length >= base_bridge_length {
                    /* Allow to cross rivers, not big lakes, nor large amounts of rails or one-way roads. */
                    return false;
                }
                bridge_tile =
                    self.map()
                        .tile_add_by_diag_dir(bridge_tile, bridge_dir, self.check());
                if !(self.is_valid_tile(bridge_tile)
                    && (((self.services.is_water_tile)(bridge_tile.0)
                        && !world().is_sea(bridge_tile.0))
                        || (self.services.is_plain_rail)(bridge_tile.0)
                        || one_way_road(bridge_tile)))
                {
                    break;
                }
            }
        } else {
            loop {
                let length = bridge_length;
                bridge_length += 1;
                if length >= max_bridge_length {
                    /* Ensure the bridge is not longer than the max allowed length. */
                    return false;
                }
                bridge_tile =
                    self.map()
                        .tile_add_by_diag_dir(bridge_tile, bridge_dir, self.check());
                if !(self.is_valid_tile(bridge_tile)
                    && ((self.services.is_water_tile)(bridge_tile.0)
                        || (self.services.is_plain_rail)(bridge_tile.0)
                        || one_way_road(bridge_tile)))
                {
                    break;
                }
            }
        }
        /* Don't allow a bridge where the start and end tiles are adjacent with no span between. */
        if bridge_length == 1 {
            return false;
        }
        /* Make sure the road can be continued past the bridge. At this point, bridge_tile holds the end tile of the bridge. */
        if !self.can_road_continue_into_next_tile(t, bridge_tile, bridge_dir) {
            return false;
        }
        /* If another parallel bridge exists nearby, this one would be redundant and shouldn't be built. We don't care about flat bridges. */
        if slope != Slope::SLOPE_FLAT {
            let map = self.map();
            let mut search = crate::spiral::hole(
                map.tile_x(tile),
                map.tile_y(tile),
                bridge_length,
                0,
                0,
                map.size_x(),
                self.size_y(),
            );
            while !search.is_end() {
                let at = map.tile_xy(search.x, search.y);
                /* Only consider bridge head tiles. */
                /* Only consider road bridges. */
                /* If the bridge is facing the same direction as the proposed bridge, we've found a redundant bridge. */
                if (self.services.is_bridge)(at.0)
                    && self.get_tunnel_bridge_transport_type(at) == TransportType::TRANSPORT_ROAD
                    && self.get_tile_slope(at)
                        & inclined_slope(reverse_diag_dir(bridge_dir, self.check()), || {
                            self.not_reached()
                        })
                        != Slope::SLOPE_FLAT
                {
                    return false;
                }
                search = search.advance(map.size_x(), self.size_y());
            }
        }
        for _times in 0..=22_u8 {
            let bridge_type = self.services.random_range(MAX_BRIDGES - 1);
            /* Can we actually build the bridge? */
            let rt = self.get_town_road_type();
            if (self.leaves.build_bridge)(tile.0, bridge_tile.0, bridge_type, u32::from(rt.0), 0) {
                (self.leaves.build_bridge)(tile.0, bridge_tile.0, bridge_type, u32::from(rt.0), 1);
                return true;
            }
        }
        /* Quit if it selecting an appropriate bridge type fails a large number of times. */
        false
    }
    /// `GrowTownWithTunnel`: first we check if a tunnel is reasonable. If so we
    /// check if we are able to build it.
    fn grow_town_with_tunnel(&self, t: TownID, tile: TileIndex, tunnel_dir: DiagDirection) -> bool {
        /* Only consider building a tunnel if the starting tile is sloped properly. */
        if self.get_tile_slope(tile) != inclined_slope(tunnel_dir, || self.not_reached()) {
            return false;
        }
        /* Assure that the tunnel is connectable to the start side */
        if self.get_town_road_bits(self.map().tile_add_by_diag_dir(
            tile,
            reverse_diag_dir(tunnel_dir, self.check()),
            self.check(),
        )) & diag_dir_to_road_bits(tunnel_dir, self.check())
            == RoadBits::ROAD_NONE
        {
            return false;
        }
        /* There are two conditions for building tunnels: Under a mountain and under an obstruction. */
        let max_tunnel_length = if self.can_road_continue_into_next_tile(t, tile, tunnel_dir) {
            /* Only tunnel under a mountain if the slope is continuous for at least 4 tiles. We want tunneling to be a last resort for large hills. */
            let mut slope_tile = tile;
            for _tiles in 0..4_u8 {
                if !self.is_valid_tile(slope_tile) {
                    return false;
                }
                let slope = self.get_tile_slope(slope_tile);
                if slope != inclined_slope(tunnel_dir, || self.not_reached())
                    && !is_steep_slope(slope)
                    && !is_slope_with_one_corner_raised(slope)
                {
                    return false;
                }
                slope_tile = self
                    .map()
                    .tile_add_by_diag_dir(slope_tile, tunnel_dir, self.check());
            }
            /* More population means longer tunnels, but make sure we can at least cover the smallest mountain which necessitates tunneling. */
            self.population() / 1000 + 7
        } else {
            /* When tunneling under an obstruction, the length limit is 5, enough to tunnel under a four-track railway. */
            5
        };
        let mut tunnel_length = 0_u8;
        let mut tunnel_tile = tile; // Iterator to store the other end tile of the tunnel.

        /* Find the end tile of the tunnel for length and continuation checks. */
        loop {
            let length = tunnel_length;
            tunnel_length = tunnel_length.wrapping_add(1);
            if u32::from(length) >= max_tunnel_length {
                return false;
            }
            tunnel_tile = self
                .map()
                .tile_add_by_diag_dir(tunnel_tile, tunnel_dir, self.check());
            /* The tunnel ends when start and end tiles are the same height. */
            if !self.is_valid_tile(tunnel_tile)
                || self.get_tile_z(tile) == self.get_tile_z(tunnel_tile)
            {
                break;
            }
        }
        /* Don't allow a tunnel where the start and end tiles are adjacent. */
        if tunnel_length == 1 {
            return false;
        }
        /* Make sure the road can be continued past the tunnel. At this point, tunnel_tile holds the end tile of the tunnel. */
        if !self.can_road_continue_into_next_tile(t, tunnel_tile, tunnel_dir) {
            return false;
        }
        /* Attempt to build the tunnel. Return false if it fails to let the town build a road instead. */
        let rt = self.get_town_road_type();
        if (self.leaves.build_tunnel)(tile.0, u32::from(rt.0), 0) {
            (self.leaves.build_tunnel)(tile.0, u32::from(rt.0), 1);
            return true;
        }
        false
    }
    /// `RoadTypesAllowHouseHere`: whether at least one surrounding road type
    /// allows building a house here.
    fn road_types_allow_house_here(&self, t: TileIndex) -> bool {
        const TILES: [(i32, i32); 8] = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];
        let mut allow = false;
        for (x, y) in TILES {
            let cur_tile = self.map().tile_add_xy(t, x, y);
            if !self.is_valid_tile(cur_tile) {
                continue;
            }
            if !(self.is_tile_type(cur_tile, TileType::MP_ROAD)
                || crate::map_func::World::new(self.services).any_road_stop_tile(cur_tile.0))
            {
                continue;
            }
            allow = true;
            let road_rt = self.get_road_type_road(cur_tile);
            if road_rt != RoadType::INVALID_ROADTYPE
                && !self.road_type_flags(road_rt).test(RoadTypeFlag::NoHouses)
            {
                return true;
            }
        }
        /* If no road was found surrounding the tile we can allow building the house since there is
         * nothing which forbids it, if a road was found but the execution reached this point, then
         * all the found roads don't allow houses to be built */
        !allow
    }
    /// `TownCanGrowRoad`: whether the tile's road type doesn't prevent extending the road.
    fn town_can_grow_road(&self, tile: TileIndex) -> bool {
        if !self.is_tile_type(tile, TileType::MP_ROAD) {
            return true;
        }
        /* Allow extending on roadtypes which can be built by town, or if the road type matches the type the town will build. */
        let rt = self.get_road_type_road(tile);
        self.road_type_flags(rt).test(RoadTypeFlag::TownBuild) || self.get_town_road_type() == rt
    }
    /// `CanFollowRoad`: whether a road can be followed or is a dead end that can
    /// not be extended to the next tile. This only checks trivial but often cases.
    fn can_follow_road(&self, tile: TileIndex, dir: DiagDirection, modes: TownExpandModes) -> bool {
        let target_tile = self.map().tile_add_by_diag_dir(tile, dir, self.check());
        if !self.is_valid_tile(target_tile) || self.has_tile_water_ground(target_tile) {
            return false;
        }
        let target_rb = self.get_town_road_bits(target_tile);
        if town_allowed_to_build_roads(modes) {
            /* Check whether a road connection exists or can be build. */
            match self.get_tile_type(target_tile) {
                TileType::MP_ROAD => target_rb != RoadBits::ROAD_NONE,
                TileType::MP_STATION => (self.services.is_drive_through_stop)(target_tile.0),
                TileType::MP_TUNNELBRIDGE => {
                    self.get_tunnel_bridge_transport_type(target_tile)
                        == TransportType::TRANSPORT_ROAD
                }
                TileType::MP_HOUSE | TileType::MP_INDUSTRY | TileType::MP_OBJECT => false,
                /* Checked for void and water earlier */
                _ => true,
            }
        } else {
            /* Check whether a road connection already exists,
             * and it leads somewhere else. */
            let back_rb = diag_dir_to_road_bits(reverse_diag_dir(dir, self.check()), self.check());
            target_rb & back_rb != RoadBits::ROAD_NONE
                && target_rb & !back_rb != RoadBits::ROAD_NONE
        }
    }
}
/// `TownAllowedToBuildRoads`.
fn town_allowed_to_build_roads(modes: TownExpandModes) -> bool {
    modes.test(TownExpandMode::Roads)
}
/// Town growth (`town_cmd.cpp`).
impl Engine<'_> {
    /// `GrowTownInTile`: grow the given town. There are at the moment 3 possible
    /// ways for the town expansion:
    /// - Generate a random tile and check if there is a road allowed
    ///   (`TL_ORIGINAL`, `TL_BETTER_ROADS`)
    /// - Check if the town geometry allows a road and which one
    ///   (`TL_2X2_GRID`, `TL_3X3_GRID`)
    /// - Forbid roads, only build houses
    fn grow_town_in_tile(
        &self,
        t1: TownID,
        tile_ptr: &mut TileIndex,
        cur_rb: RoadBits,
        mut target_dir: DiagDirection,
        modes: TownExpandModes,
    ) -> TownGrowthResult {
        let mut rcmd; // RoadBits for the road construction command
        let tile = *tile_ptr; // The main tile on which we base our growth

        self.assert_tile_in_map(tile);

        let layout = self.layout();
        if cur_rb == RoadBits::ROAD_NONE {
            /* Tile has no road.
             * We will return TownGrowthResult::SearchStopped to say that this is the last iteration. */
            if !town_allowed_to_build_roads(modes) {
                return TownGrowthResult::SearchStopped;
            }
            if !self.allow_town_level_crossings() && self.is_tile_type(tile, TileType::MP_RAILWAY) {
                return TownGrowthResult::SearchStopped;
            }
            /* Remove hills etc */
            if !(self.services.build_on_slopes)() || chance16(self.services, 1, 6) {
                self.level_town_land(tile);
            }
            /* Is a road allowed here? */
            match layout {
                TownLayout::TL_3X3_GRID | TownLayout::TL_2X2_GRID => {
                    rcmd = self.get_town_road_grid_element(tile, target_dir);
                    if rcmd == RoadBits::ROAD_NONE {
                        return TownGrowthResult::SearchStopped;
                    }
                }
                TownLayout::TL_BETTER_ROADS | TownLayout::TL_ORIGINAL => {
                    if !self.is_road_allowed_here(t1, tile, target_dir) {
                        return TownGrowthResult::SearchStopped;
                    }
                    let source_dir = reverse_diag_dir(target_dir, self.check());
                    if chance16(self.services, 1, 4) {
                        /* Randomize a new target dir */
                        target_dir = self.random_diag_dir();
                        while target_dir == source_dir {
                            target_dir = self.random_diag_dir();
                        }
                    }
                    let next_tile = self
                        .map()
                        .tile_add_by_diag_dir(tile, target_dir, self.check());
                    if !self.is_road_allowed_here(t1, next_tile, target_dir) {
                        /* A road is not allowed to continue the randomized road,
                         *  return if the road we're trying to build is curved. */
                        if target_dir != reverse_diag_dir(source_dir, self.check()) {
                            return TownGrowthResult::SearchStopped;
                        }
                        /* Return if neither side of the new road is a house */
                        let right = change_diag_dir(
                            target_dir,
                            DiagDirDiff::DIAGDIRDIFF_90RIGHT,
                            self.check(),
                        );
                        let left = change_diag_dir(
                            target_dir,
                            DiagDirDiff::DIAGDIRDIFF_90LEFT,
                            self.check(),
                        );
                        if !self.is_tile_type(
                            self.map().tile_add_by_diag_dir(tile, right, self.check()),
                            TileType::MP_HOUSE,
                        ) && !self.is_tile_type(
                            self.map().tile_add_by_diag_dir(tile, left, self.check()),
                            TileType::MP_HOUSE,
                        ) {
                            return TownGrowthResult::SearchStopped;
                        }
                        /* That means that the road is only allowed if there is a house
                         *  at any side of the new road. */
                    }
                    rcmd = diag_dir_to_road_bits(target_dir, self.check())
                        | diag_dir_to_road_bits(source_dir, self.check());
                }
                _ => self.not_reached(),
            }
        } else if target_dir.0 < DiagDirection::DIAGDIR_END.0
            && cur_rb
                & diag_dir_to_road_bits(reverse_diag_dir(target_dir, self.check()), self.check())
                == RoadBits::ROAD_NONE
        {
            if !self.town_can_grow_road(tile) {
                return TownGrowthResult::Continue;
            }
            if !town_allowed_to_build_roads(modes) {
                return TownGrowthResult::SearchStopped;
            }
            /* Continue building on a partial road.
             * Should be always OK, so we only generate
             * the fitting RoadBits */
            match layout {
                TownLayout::TL_3X3_GRID | TownLayout::TL_2X2_GRID => {
                    rcmd = self.get_town_road_grid_element(tile, target_dir);
                }
                TownLayout::TL_BETTER_ROADS | TownLayout::TL_ORIGINAL => {
                    rcmd = diag_dir_to_road_bits(
                        reverse_diag_dir(target_dir, self.check()),
                        self.check(),
                    );
                }
                _ => self.not_reached(),
            }
        } else {
            let mut allow_house = true; // Value which decides if we want to construct a house

            /* Reached a tunnel/bridge? Then continue at the other side of it, unless
             * it is the starting tile. Half the time, we stay on this side then.*/
            if self.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) {
                if self.get_tunnel_bridge_transport_type(tile) == TransportType::TRANSPORT_ROAD
                    && (target_dir != DiagDirection::DIAGDIR_END || chance16(self.services, 1, 2))
                {
                    *tile_ptr = TileIndex((self.services.other_tunnel_end)(tile.0));
                }
                return TownGrowthResult::Continue;
            }
            /* Possibly extend the road in a direction.
             * Randomize a direction and if it has a road, bail out. */
            target_dir = self.random_diag_dir();
            let target_rb = diag_dir_to_road_bits(target_dir, self.check());
            // position of a possible house
            let house_tile = if cur_rb & target_rb == RoadBits::ROAD_NONE {
                self.map()
                    .tile_add_by_diag_dir(tile, target_dir, self.check())
            } else {
                /* If it's a road turn possibly build a house in a corner.
                 * Use intersection with straight road as an indicator
                 * that we randomised corner house position.
                 * A turn (and we check for that later) always has only
                 * one common bit with a straight road so it has the same
                 * chance to be chosen as the house on the side of a road.
                 */
                if cur_rb & RoadBits::ROAD_X != target_rb {
                    return TownGrowthResult::Continue;
                }
                /* Check whether it is a turn and if so determine
                 * position of the corner tile */
                let Some(house_tile) = self.road_turn_corner_tile(tile, cur_rb) else {
                    return TownGrowthResult::Continue; // not a turn
                };
                target_dir = DiagDirection::DIAGDIR_END;
                house_tile
            };
            /* Don't walk into water. */
            if self.has_tile_water_ground(house_tile) || !self.is_valid_tile(house_tile) {
                return TownGrowthResult::Continue;
            }
            let mut result = TownGrowthResult::Continue;
            rcmd = RoadBits::ROAD_NONE;
            if target_dir != DiagDirection::DIAGDIR_END && town_allowed_to_build_roads(modes) {
                match layout {
                    TownLayout::TL_3X3_GRID | TownLayout::TL_2X2_GRID => {
                        // TL_3X3_GRID: Use 2x2 grid afterwards!
                        if layout == TownLayout::TL_3X3_GRID
                            && self.grow_town_with_extra_house(
                                t1,
                                self.map().tile_add_by_diag_dir(
                                    house_tile,
                                    target_dir,
                                    self.check(),
                                ),
                                modes,
                            )
                        {
                            result = TownGrowthResult::Succeed;
                        }
                        rcmd = self.get_town_road_grid_element(tile, target_dir);
                        allow_house = rcmd & target_rb == RoadBits::ROAD_NONE;
                    }
                    TownLayout::TL_BETTER_ROADS | TownLayout::TL_ORIGINAL => {
                        // TL_BETTER_ROADS: Use original afterwards!
                        if layout == TownLayout::TL_BETTER_ROADS
                            && self.grow_town_with_extra_house(
                                t1,
                                self.map().tile_add_by_diag_dir(
                                    house_tile,
                                    target_dir,
                                    self.check(),
                                ),
                                modes,
                            )
                        {
                            result = TownGrowthResult::Succeed;
                        }
                        /* Allow a house at the edge. 60% chance or
                         * always ok if no road allowed. */
                        rcmd = target_rb;
                        allow_house = !self.is_road_allowed_here(t1, house_tile, target_dir)
                            || chance16(self.services, 6, 10);
                    }
                    _ => self.not_reached(),
                }
            }
            allow_house &= self.road_types_allow_house_here(house_tile);
            if allow_house {
                /* Build a house, but not if there already is a house there. */
                if !self.is_tile_type(house_tile, TileType::MP_HOUSE) {
                    /* Level the land if possible */
                    if chance16(self.services, 1, 6) {
                        self.level_town_land(house_tile);
                    }
                    /* And build a house.
                     * Set result to -1 if we managed to build it. */
                    if self.try_build_town_house(t1, house_tile, modes) {
                        result = TownGrowthResult::Succeed;
                    }
                }
                return result;
            }
            if !self.town_can_grow_road(tile) {
                return result;
            }
        }
        /* Return if a water tile */
        if self.has_tile_water_ground(tile) {
            return TownGrowthResult::SearchStopped;
        }
        /* Make the roads look nicer */
        rcmd = crate::road_cleanup::clean_up_road_bits(self.services, tile.0, rcmd);
        if rcmd == RoadBits::ROAD_NONE {
            return TownGrowthResult::SearchStopped;
        }
        /* Only use the target direction for bridges and tunnels to ensure they're connected.
         * The target_dir is as computed previously according to town layout, so
         * it will match it perfectly. */
        if self.grow_town_with_bridge(t1, tile, target_dir)
            || self.grow_town_with_tunnel(t1, tile, target_dir)
            || self.grow_town_with_road(t1, tile, rcmd)
        {
            return TownGrowthResult::Succeed;
        }
        TownGrowthResult::SearchStopped
    }
    /// `GrowTownInTile`'s corner tile of a road turn, or `None` if `cur_rb` is not a turn.
    fn road_turn_corner_tile(&self, tile: TileIndex, cur_rb: RoadBits) -> Option<TileIndex> {
        match cur_rb {
            RoadBits::ROAD_N => Some(self.map().tile_add_by_dir(
                tile,
                Direction::DIR_S,
                self.check(),
            )),
            RoadBits::ROAD_S => Some(self.map().tile_add_by_dir(
                tile,
                Direction::DIR_N,
                self.check(),
            )),
            RoadBits::ROAD_E => Some(self.map().tile_add_by_dir(
                tile,
                Direction::DIR_W,
                self.check(),
            )),
            RoadBits::ROAD_W => Some(self.map().tile_add_by_dir(
                tile,
                Direction::DIR_E,
                self.check(),
            )),
            _ => None,
        }
    }
    /// `GrowTownAtRoad`: try to grow a town at a given road tile.
    fn grow_town_at_road(&self, t: TownID, mut tile: TileIndex, modes: TownExpandModes) -> bool {
        /* Special case.
         * @see GrowTownInTile Check the else if
         */
        let mut target_dir = DiagDirection::DIAGDIR_END; // The direction in which we want to extend the town

        self.assert_tile_in_map(tile);

        /* Number of times to search.
         * Better roads, 2X2 and 3X3 grid grow quite fast so we give
         * them a little handicap. */
        let mut iterations = match self.layout() {
            TownLayout::TL_BETTER_ROADS => (10 + self.num_houses().wrapping_mul(2) / 9) as i32,
            TownLayout::TL_3X3_GRID | TownLayout::TL_2X2_GRID => {
                (10 + self.num_houses().wrapping_mul(1) / 9) as i32
            }
            _ => (10 + self.num_houses().wrapping_mul(4) / 9) as i32,
        };
        loop {
            let mut cur_rb = self.get_town_road_bits(tile); // The RoadBits of the current tile

            /* Try to grow the town from this point */
            match self.grow_town_in_tile(t, &mut tile, cur_rb, target_dir, modes) {
                TownGrowthResult::Succeed => return true,
                TownGrowthResult::SearchStopped => iterations = 0,
                _ => {}
            }
            /* Exclude the source position from the bitmask
             * and return if no more road blocks available */
            if is_valid_diag_direction(target_dir) {
                cur_rb &= !diag_dir_to_road_bits(
                    reverse_diag_dir(target_dir, self.check()),
                    self.check(),
                );
            }
            if cur_rb == RoadBits::ROAD_NONE {
                return false;
            }
            if self.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) {
                /* Only build in the direction away from the tunnel or bridge. */
                target_dir = reverse_diag_dir(self.get_tunnel_bridge_direction(tile), self.check());
            } else {
                /* Select a random bit from the blockmask, walk a step
                 * and continue the search from there. */
                loop {
                    if cur_rb == RoadBits::ROAD_NONE {
                        return false;
                    }
                    let target_bits = loop {
                        target_dir = self.random_diag_dir();
                        let target_bits = diag_dir_to_road_bits(target_dir, self.check());
                        if cur_rb & target_bits != RoadBits::ROAD_NONE {
                            break target_bits;
                        }
                    };
                    cur_rb &= !target_bits;
                    if self.can_follow_road(tile, target_dir, modes) {
                        break;
                    }
                }
            }
            tile = self
                .map()
                .tile_add_by_diag_dir(tile, target_dir, self.check());
            if self.is_tile_type(tile, TileType::MP_ROAD)
                && !(self.services.is_road_depot)(tile.0)
                && (self.services.has_road_type)(tile.0)
            {
                let owner = crate::map_func::World::new(self.services).road_owner(tile.0, false);
                /* Don't allow building over roads of other cities */
                if owner == OWNER_TOWN && TownID((self.services.town_index)(tile.0)) != t {
                    return false;
                }
                if owner == OWNER_NONE && (self.leaves.environment.editor)() {
                    /* If we are in the SE, and this road-piece has no town owner yet, it just found an
                     * owner :) (happy happy happy road now) */
                    (self.leaves.set_road_town)(tile.0, u32::from(t.0));
                }
            }
            /* Max number of times is checked. */
            iterations -= 1;
            if iterations < 0 {
                break;
            }
        }
        false
    }
    /// `GenRandomRoadBits`: a random road block. The probability of a straight
    /// road is somewhat higher than a curved.
    fn gen_random_road_bits(&self) -> RoadBits {
        let r = self.services.random();
        let a = gb(r, 0, 2);
        let mut b = gb(r, 8, 2);
        if a == b {
            b ^= 2;
        }
        RoadBits(
            ((u32::from(RoadBits::ROAD_NW.0) << a) + (u32::from(RoadBits::ROAD_NW.0) << b)) as u8,
        )
    }
    /// `GrowTown`: whether we successfully grew the town with a road or house.
    fn grow_town(&self, t: TownID, modes: TownExpandModes) -> bool {
        const TOWN_COORD_MOD: [TileIndexDiffC; 13] = [
            TileIndexDiffC { x: -1, y: 0 },
            TileIndexDiffC { x: 1, y: 1 },
            TileIndexDiffC { x: 1, y: -1 },
            TileIndexDiffC { x: -1, y: -1 },
            TileIndexDiffC { x: -1, y: 0 },
            TileIndexDiffC { x: 0, y: 2 },
            TileIndexDiffC { x: 2, y: 0 },
            TileIndexDiffC { x: 0, y: -2 },
            TileIndexDiffC { x: -1, y: -1 },
            TileIndexDiffC { x: -2, y: 2 },
            TileIndexDiffC { x: 2, y: 2 },
            TileIndexDiffC { x: 2, y: -2 },
            TileIndexDiffC { x: 0, y: 0 },
        ];
        /* Current "company" is a town */
        // Ordinary shared commands are noexcept; restore the original company on each return.
        // The exchange service carries the saved `CompanyID` in its base integer.
        let cur_company = CompanyID((self.leaves.exchange_company)(u32::from(OWNER_TOWN)) as u8);
        let mut tile = self.xy(); // The tile we are working with ATM

        /* Find a road that we can base the construction on. */
        for ptr in TOWN_COORD_MOD {
            if self.get_town_road_bits(tile) != RoadBits::ROAD_NONE {
                let success = self.grow_town_at_road(t, tile, modes);
                (self.leaves.exchange_company)(u32::from(cur_company.0));
                return success;
            }
            tile = self
                .map()
                .tile_add_xy(tile, i32::from(ptr.x), i32::from(ptr.y));
        }
        /* No road available, try to build a random road block by
         * clearing some land and then building a road there. */
        if town_allowed_to_build_roads(modes) {
            tile = self.xy();
            for ptr in TOWN_COORD_MOD {
                /* Only work with plain land that not already has a house */
                if !self.is_tile_type(tile, TileType::MP_HOUSE)
                    && crate::map_func::World::new(self.services).tile_flat(tile.0)
                    && self.cmd_landscape_clear(DC_AUTO_NO_WATER, tile).success
                {
                    let rt = self.get_town_road_type();
                    self.cmd_build_road(DC_EXECUTE_AUTO, tile, self.gen_random_road_bits(), rt, t);
                    (self.leaves.exchange_company)(u32::from(cur_company.0));
                    return true;
                }
                tile = self
                    .map()
                    .tile_add_xy(tile, i32::from(ptr.x), i32::from(ptr.y));
            }
        }
        (self.leaves.exchange_company)(u32::from(cur_company.0));
        false
    }
}
/// House construction (`town_cmd.cpp`).
impl Engine<'_> {
    /// `TownLayoutAllowsHouseHere`: whether the current town layout allows building here.
    fn town_layout_allows_house_here(&self, tile: TileIndex, modes: TownExpandModes) -> bool {
        if !modes.test(TownExpandMode::Buildings) {
            return false;
        }
        /* Allow towns everywhere when we don't build roads */
        if !town_allowed_to_build_roads(modes) {
            return true;
        }
        let grid_pos = self.map().tile_index_to_tile_index_diff_c(self.xy(), tile);
        match self.layout() {
            TownLayout::TL_2X2_GRID => grid_pos.x % 3 != 0 && grid_pos.y % 3 != 0,
            TownLayout::TL_3X3_GRID => grid_pos.x % 4 != 0 && grid_pos.y % 4 != 0,
            _ => true,
        }
    }
    /// `TownLayoutAllows2x2HouseHere`: whether the current town layout allows a
    /// 2x2 building here.
    fn town_layout_allows_2x2_house_here(&self, tile: TileIndex, modes: TownExpandModes) -> bool {
        if !modes.test(TownExpandMode::Buildings) {
            return false;
        }
        /* Allow towns everywhere when we don't build roads */
        if !town_allowed_to_build_roads(modes) {
            return true;
        }
        /* Compute relative position of tile. (Positive offsets are towards north) */
        let grid_pos = self.map().tile_index_to_tile_index_diff_c(self.xy(), tile);
        match self.layout() {
            TownLayout::TL_2X2_GRID => {
                matches!(grid_pos.x % 3, 2 | -1) && matches!(grid_pos.y % 3, 2 | -1)
            }
            TownLayout::TL_3X3_GRID => grid_pos.x & 3 >= 2 && grid_pos.y & 3 >= 2,
            _ => true,
        }
    }
    /// `CanBuildHouseHere`: whether a house can be built here, based on slope,
    /// whether there's a bridge above, and if we can clear the land.
    fn can_build_house_here(&self, tile: TileIndex, noslope: bool) -> bool {
        /* cannot build on these slopes... */
        let slope = self.get_tile_slope(tile);
        if (noslope && slope != Slope::SLOPE_FLAT) || is_steep_slope(slope) {
            return false;
        }
        /* at least one RoadTypes allow building the house here? */
        if !self.road_types_allow_house_here(tile) {
            return false;
        }
        /* building under a bridge? */
        if self.is_bridge_above(tile) {
            return false;
        }
        /* can we clear the land? */
        self.cmd_landscape_clear(DC_AUTO_NO_WATER, tile).success
    }
    /// `CheckBuildHouseSameZ`: whether a tile where we want to build a multi-tile
    /// house has an appropriate max Z.
    fn check_build_house_same_z(&self, tile: TileIndex, z: i32, noslope: bool) -> bool {
        if !self.can_build_house_here(tile, noslope) {
            return false;
        }
        /* if building on slopes is allowed, there will be flattening foundation (to tile max z) */
        self.get_tile_max_z(tile) == z
    }
    /// `CheckFree2x2Area`: whether a house of size 2x2 can be built at this tile.
    fn check_free_2x2_area(&self, mut tile: TileIndex, z: i32, noslope: bool) -> bool {
        /* we need to check this tile too because we can be at different tile now */
        if !self.check_build_house_same_z(tile, z, noslope) {
            return false;
        }
        for d in DiagDirection::DIAGDIR_SE.0..DiagDirection::DIAGDIR_END.0 {
            tile = self
                .map()
                .tile_add_by_diag_dir(tile, DiagDirection(d), self.check());
            if !self.check_build_house_same_z(tile, z, noslope) {
                return false;
            }
        }
        true
    }
    /// `CheckTownBuild2House`: whether a 1x2 or 2x1 building is allowed here,
    /// accounting for road layout and tile heights. Also, tests both building
    /// positions that occupy this tile.
    fn check_town_build_2_house(
        &self,
        tile: &mut TileIndex,
        maxz: i32,
        noslope: bool,
        second: DiagDirection,
        modes: TownExpandModes,
    ) -> bool {
        /* 'tile' is already checked in BuildTownHouse() - CanBuildHouseHere() and slope test */
        let mut tile2 = self.map().tile_add_by_diag_dir(*tile, second, self.check());
        if self.town_layout_allows_house_here(tile2, modes)
            && self.check_build_house_same_z(tile2, maxz, noslope)
        {
            return true;
        }
        tile2 = self.map().tile_add_by_diag_dir(
            *tile,
            reverse_diag_dir(second, self.check()),
            self.check(),
        );
        if self.town_layout_allows_house_here(tile2, modes)
            && self.check_build_house_same_z(tile2, maxz, noslope)
        {
            *tile = tile2;
            return true;
        }
        false
    }
    /// `CheckTownBuild2x2House`: whether a 2x2 building is allowed here,
    /// accounting for road layout and tile heights. Also, tests all four building
    /// positions that occupy this tile.
    fn check_town_build_2x2_house(
        &self,
        tile: &mut TileIndex,
        maxz: i32,
        noslope: bool,
        modes: TownExpandModes,
    ) -> bool {
        let mut tile2 = *tile;
        // 'd' goes through DIAGDIR_SE, DIAGDIR_SW, DIAGDIR_NW, DIAGDIR_END
        for d in DiagDirection::DIAGDIR_SE.0..=DiagDirection::DIAGDIR_END.0 {
            if self.town_layout_allows_2x2_house_here(tile2, modes)
                && self.check_free_2x2_area(tile2, maxz, noslope)
            {
                *tile = tile2;
                return true;
            }
            if d == DiagDirection::DIAGDIR_END.0 {
                break;
            }
            tile2 = self.map().tile_add_by_diag_dir(
                tile2,
                reverse_diag_dir(DiagDirection(d), self.check()),
                self.check(),
            ); // go clockwise
        }
        false
    }
    /// `ClearMakeHouseTile`: clear the tile and build a house or house part.
    fn clear_make_house_tile(
        &self,
        tile: TileIndex,
        t: TownID,
        house: HouseID,
        c: HouseConstruction,
    ) {
        (self.leaves.assert_clear_house)(
            self.cmd_landscape_clear(DC_EXECUTE_AUTO_NO_WATER, tile)
                .success,
        );
        let class = (self.leaves.house_class)(house);
        // SAFETY: Entry resolved the live town once. Count borrows end here,
        // before map writes or callbacks can reenter and mutate the owner.
        unsafe { increase_building(self.owner, house, class) };
        (self.leaves.make_house)(
            tile.0,
            u32::from(t.0),
            u32::from(house),
            u32::from(c.counter)
                | (u32::from(c.stage) << 8)
                | (u32::from(c.random_bits) << 16)
                | (u32::from(c.is_protected) << 24),
        );
        if house::building_flags(self.leaves, house).test(BuildingFlag::IsAnimated) {
            crate::animated_tile::openttd_rust_animation_add(tile.0, false);
        }
        (self.services.mark_dirty)(tile.0);
    }
    /// `MakeTownHouse`: write house information into the map. For multi-tile
    /// houses, all tiles are marked; `tile` is the northern-most tile.
    fn make_town_house(
        &self,
        tile: TileIndex,
        t: TownID,
        mut house: HouseID,
        c: HouseConstruction,
    ) {
        let size = house::building_flags(self.leaves, house);
        self.clear_make_house_tile(tile, t, house, c);
        if size.any(BuildingFlags::BUILDING_2_TILES_Y) {
            house += 1;
            self.clear_make_house_tile(self.map().tile_add_xy(tile, 0, 1), t, house, c);
        }
        if size.any(BuildingFlags::BUILDING_2_TILES_X) {
            house += 1;
            self.clear_make_house_tile(self.map().tile_add_xy(tile, 1, 0), t, house, c);
        }
        if size.any(BuildingFlags::BUILDING_HAS_4_TILES) {
            house += 1;
            self.clear_make_house_tile(self.map().tile_add_xy(tile, 1, 1), t, house, c);
        }
        let area = crate::tilearea::TileArea {
            tile: tile.0,
            w: if size.any(BuildingFlags::BUILDING_2_TILES_X) {
                2
            } else {
                1
            },
            h: if size.any(BuildingFlags::BUILDING_2_TILES_Y) {
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
    }
    /// `BuildTownHouse`: build a house at this tile.
    fn build_town_house(
        &self,
        t: TownID,
        tile: TileIndex,
        house: HouseID,
        random_bits: u8,
        house_completed: bool,
        is_protected: bool,
    ) {
        /* build the house */
        self.set_num_houses(self.num_houses().wrapping_add(1));
        let mut construction_counter = 0;
        let mut construction_stage = 0;
        if self.leaves.generating_world() || (self.leaves.environment.editor)() || house_completed {
            let construction_random = self.services.random();
            construction_stage = TOWN_HOUSE_COMPLETED;
            if self.leaves.generating_world()
                && !HouseExtraFlags((self.leaves.house_extra_flags)(house))
                    .test(HouseExtraFlag::BuildingIsHistorical)
                && chance16(self.services, 1, 7)
            {
                construction_stage = gb(construction_random, 0, 2) as u8;
            }
            if construction_stage == TOWN_HOUSE_COMPLETED {
                // SAFETY: Owner mutation ends before possible presentation reentry.
                unsafe {
                    change_population(
                        self.leaves,
                        self.owner,
                        t,
                        i32::from((self.leaves.house_population)(house)),
                    );
                }
            } else {
                construction_counter = gb(construction_random, 2, 2) as u8;
            }
        }
        self.make_town_house(
            tile,
            t,
            house,
            HouseConstruction {
                counter: construction_counter,
                stage: construction_stage,
                random_bits,
                is_protected,
            },
        );
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe {
            update_town_radius(self.owner);
        }
        self.update_town_growth_rate(t);
        let size = house::building_flags(self.leaves, house);
        // Resolve only the native shell at the first construction trigger; the
        // entry already resolved its owner. House lifecycle callers also supplied
        // their shell. No owner reference spans any construction callback.
        let shell = *self.house_shell.get_or_init(|| {
            // SAFETY: Static house table referenced by the town leaves.
            (self.services.town_by_id)(u32::from(t.0))
        });
        let town_ref = TownRef {
            shell,
            state: self.owner,
            id: t,
        };
        let stage_changed = |part: TileIndex| {
            house::trigger_house_animation_construction_stage_changed(
                self.leaves,
                self.services,
                town_ref,
                part,
            );
        };
        stage_changed(tile);
        if size.any(BuildingFlags::BUILDING_2_TILES_Y) {
            stage_changed(self.map().tile_add_xy(tile, 0, 1));
        }
        if size.any(BuildingFlags::BUILDING_2_TILES_X) {
            stage_changed(self.map().tile_add_xy(tile, 1, 0));
        }
        if size.any(BuildingFlags::BUILDING_HAS_4_TILES) {
            stage_changed(self.map().tile_add_xy(tile, 1, 1));
        }
    }
}
/// House selection, ticks and the town command bodies (`town_cmd.cpp`).
impl Engine<'_> {
    /// `TryBuildTownHouse`'s zone set: the town zone of the tile and its climate.
    fn town_house_zones(&self, tile: TileIndex, maxz: i32) -> HouseZones {
        // SAFETY: Live bound owner; immediate copied reads.
        let mut zones = HouseZones::from_element(unsafe {
            get_town_radius_group(self.owner, tile, self.map())
        });
        zones = zones.with(HouseZones::from_element(
            match LandscapeType((self.leaves.climate)()) {
                LandscapeType::Temperate => HouseZone::ClimateTemperate,
                LandscapeType::Arctic => {
                    if maxz > i32::from(crate::snow_line::highest(self.services)) {
                        HouseZone::ClimateSubarcticAboveSnow
                    } else {
                        HouseZone::ClimateSubarcticBelowSnow
                    }
                }
                LandscapeType::Tropic => HouseZone::ClimateSubtropic,
                // LandscapeType::Toyland
                _ => HouseZone::ClimateToyland,
            },
        ));
        zones
    }
    /// `TryBuildTownHouse`'s list of all possible houses that can be built, with
    /// their total probability.
    fn collect_town_house_candidates(&self, zones: HouseZones) -> u32 {
        clear_house_candidates();
        let mut probability_max = 0_u32;
        /* Generate a list of all possible houses that can be built. */
        for index in 0..(self.leaves.house_count)() {
            let house = index as HouseID;
            let hs = (self.leaves.house_availability)(house);
            /* Verify that the candidate house spec matches the current tile status */
            if !HouseZones(hs.zones).all(zones) || !hs.enabled || hs.override_id != INVALID_HOUSE_ID
            {
                continue;
            }
            /* Don't let these counters overflow. Global counters are 32bit, there will never be that many houses. */
            // SAFETY: Original valid spec/count indices; immediate read ends
            // before the next spec query or any construction callback.
            let count = unsafe {
                if hs.class_id == HOUSE_NO_CLASS {
                    /* If the house has no class, check id_count instead */
                    (&(*self.owner).id_counts)[usize::from(house)]
                } else {
                    /* id_count is always <= class_count, so it doesn't need to be checked */
                    (&(*self.owner).class_counts)[usize::from(hs.class_id)]
                }
            };
            if count == u16::MAX {
                continue;
            }
            let cur_prob = u32::from(hs.probability);
            probability_max = probability_max.wrapping_add(cur_prob);
            add_house_candidate(house, cur_prob);
        }
        probability_max
    }
    /// `TryBuildTownHouse`: false iff no house can be built on this tile.
    fn try_build_town_house(&self, t: TownID, mut tile: TileIndex, modes: TownExpandModes) -> bool {
        /* forbidden building here by town layout */
        if !self.town_layout_allows_house_here(tile, modes) {
            return false;
        }
        /* no house allowed at all, bail out */
        if !self.can_build_house_here(tile, false) {
            return false;
        }
        let slope = self.get_tile_slope(tile);
        let maxz = self.get_tile_max_z(tile);
        /* Get the town zone type of the current tile, as well as the climate.
         * This will allow to easily compare with the specs of the new house to build */
        let zones = self.town_house_zones(tile, maxz);
        /* bits 0-4 are used
         * bits 11-15 are used
         * bits 5-10 are not used. */
        let mut probability_max = self.collect_town_house_candidates(zones);
        let base_tile = tile;
        while probability_max > 0 {
            /* Building a multitile building can change the location of tile.
             * The building would still be built partially on that tile, but
             * its northern tile would be elsewhere. However, if the callback
             * fails we would be basing further work from the changed tile.
             * So a next 1x1 tile building could be built on the wrong tile. */
            tile = base_tile;
            /* remove tested house from the set */
            let (house, probability) =
                take_house_candidate(self.services.random_range(probability_max));
            probability_max -= probability;
            let hs = (self.leaves.house_placement)(house);
            let extra_flags = || HouseExtraFlags((self.leaves.house_extra_flags)(house));
            if !self.leaves.generating_world()
                && !(self.leaves.environment.editor)()
                && extra_flags().test(HouseExtraFlag::BuildingIsHistorical)
            {
                continue;
            }
            let year = crate::timer_game_common::calendar_year().0;
            if year < hs.min_year || year > hs.max_year {
                continue;
            }
            /* Special houses that there can be only one of. */
            let building_flags = house::building_flags(self.leaves, house);
            let mut oneof = TownFlags::EMPTY;
            if building_flags.test(BuildingFlag::IsChurch) {
                oneof.set(TownFlag::HasChurch);
            } else if building_flags.test(BuildingFlag::IsStadium) {
                oneof.set(TownFlag::HasStadium);
            }
            if self.flags().any(oneof) {
                continue;
            }
            /* Make sure there is no slope? */
            let noslope = building_flags.test(BuildingFlag::NotSloped);
            if noslope && slope != Slope::SLOPE_FLAT {
                continue;
            }
            if building_flags.test(BuildingFlag::Size2x2) {
                if !self.check_town_build_2x2_house(&mut tile, maxz, noslope, modes) {
                    continue;
                }
            } else if building_flags.test(BuildingFlag::Size2x1) {
                if !self.check_town_build_2_house(
                    &mut tile,
                    maxz,
                    noslope,
                    DiagDirection::DIAGDIR_SW,
                    modes,
                ) {
                    continue;
                }
            } else if building_flags.test(BuildingFlag::Size1x2) {
                if !self.check_town_build_2_house(
                    &mut tile,
                    maxz,
                    noslope,
                    DiagDirection::DIAGDIR_SE,
                    modes,
                ) {
                    continue;
                }
            } else {
                /* 1x1 house checks are already done */
            }
            let random_bits = self.services.random() as u8;
            if HouseCallbackMasks((self.leaves.house_callbacks)(house))
                .test(HouseCallbackMask::AllowConstruction)
                && !unsafe {
                    resolver::allow_construction(self.leaves, t, tile, house, random_bits)
                }
            {
                continue;
            }
            /* Special houses that there can be only one of. */
            self.set_flags(self.flags() | oneof);
            self.build_town_house(
                t,
                tile,
                house,
                random_bits,
                false,
                extra_flags().test(HouseExtraFlag::BuildingIsProtected),
            );
            return true;
        }
        false
    }
    /// `TownTickHandler`: handle the town tick for a single town, by growing the
    /// town if desired.
    fn town_tick_handler(&self, t: TownID) {
        if !self.flags().test(TownFlag::IsGrowing) {
            return;
        }
        let mut modes = TownExpandModes::from_element(TownExpandMode::Buildings);
        if unsafe { self.services.settings().allow_town_roads.read() } {
            modes = modes.with(TownExpandModes::from_element(TownExpandMode::Roads));
        }
        let mut i = i32::from(self.grow_counter()) - 1;
        if i < 0 {
            i = i32::from(if self.grow_town(t, modes) {
                self.growth_rate()
            } else {
                /* If growth failed wait a bit before retrying */
                self.growth_rate().min(TOWN_GROWTH_TICKS - 1)
            });
        }
        self.set_grow_counter(i as u16);
    }
    /// `CmdExpandTown`'s execution.
    fn expand_town(&self, t: TownID, grow_amount: u32, modes: TownExpandModes) {
        /* The more houses, the faster we grow */
        if grow_amount == 0 {
            let amount = self
                .services
                .random_range(u32::from(clamp_to_u16(i64::from(self.num_houses() / 10))))
                + 3;
            self.set_num_houses(self.num_houses().wrapping_add(amount));
            // SAFETY: Pure immediate owner mutation, without native callbacks.
            unsafe {
                update_town_radius(self.owner);
            }
            for _ in 0..amount * 10 {
                self.grow_town(t, modes);
            }
            self.set_num_houses(self.num_houses().wrapping_sub(amount));
        } else {
            for _ in 0..grow_amount {
                /* Try several times to grow, as we are really suppose to grow */
                for _ in 0..25 {
                    if self.grow_town(t, modes) {
                        break;
                    }
                }
            }
        }
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe {
            update_town_radius(self.owner);
        }
        policy::update_town_max_pass(self.leaves, self.owner);
    }
    /// `OnTick_Town`: iterate through all towns and call their tick handler.
    fn on_tick_town(mut self) {
        if (self.leaves.environment.editor)() {
            return;
        }
        for id in iterate(|from| self.leaves.next_town(from)) {
            // Each town starts with an empty map and shell cache.
            self.map.take();
            self.house_shell.take();
            self.owner = (self.leaves.resolve_owner)(u32::from(id.0));
            self.town_tick_handler(id);
        }
    }
    /// `CmdTownGrowthRate`'s execution.
    fn town_growth_rate(&self, t: TownID, growth_rate: u16) {
        if growth_rate == 0 {
            /* Just clear the flag, UpdateTownGrowth will determine a proper growth rate */
            self.flags_reset(TownFlag::CustomGrowth);
        } else {
            let old_rate = u32::from(self.growth_rate());
            let grow_counter = u32::from(self.grow_counter());
            self.set_grow_counter(if grow_counter >= old_rate {
                /* This also catches old_rate == 0 */
                growth_rate
            } else {
                /* Scale grow_counter, so half finished houses stay half finished */
                (grow_counter * u32::from(growth_rate) / old_rate) as u16
            });
            self.set_growth_rate(growth_rate);
            self.flags_set(TownFlag::CustomGrowth);
        }
        self.update_town_growth(t);
    }
    /// `TownActionFundBuildings`'s execution.
    fn town_action_fund_buildings(&self, t: TownID) {
        /* And grow for 3 months */
        self.set_fund_buildings_months(3);
        /* Enable growth (also checking GameScript's opinion) */
        self.update_town_growth(t);
        /* Build a new house, but add a small delay to make sure
         * that spamming funding doesn't let town grow any faster
         * than 1 house per 2 * TOWN_GROWTH_TICKS ticks.
         * Also emulate original behaviour when town was only growing in
         * TOWN_GROWTH_TICKS intervals, to make sure that it's not too
         * tick-perfect and gives player some time window where they can
         * spam funding with the exact same efficiency.
         */
        let grow_counter = i32::from(self.grow_counter());
        let growth_rate = i32::from(self.growth_rate());
        let ticks = i32::from(TOWN_GROWTH_TICKS);
        self.set_grow_counter(
            (grow_counter.min(2 * ticks - (growth_rate - grow_counter) % ticks)) as u16,
        );
    }
}
pub(crate) unsafe fn tick_all(
    leaves: *const Leaves,
    services: *const Services,
    road: *const crate::road_cmd::CommandContext,
) {
    let mut engine = engine(leaves, services);
    engine.road = road;
    engine.on_tick_town();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_road_type(
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    u32::from(engine(leaves, services).get_town_road_type().0)
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
        production: clamp_to_u32(production / history.len() as u64),
        transported: clamp_to_u32(transported / history.len() as u64),
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
/// The monthly town loop (`town_cmd.cpp`'s economy monthly timer).
impl Engine<'_> {
    /// The monthly timer's counters and statistics for one town.
    fn monthly_statistics(&self, month: u8) {
        // SAFETY: No callback or service spans this borrow of canonical histories.
        let state = unsafe { &mut *self.owner };
        /* Check for active town actions and decrement their counters. */
        if state.growth.road != 0 {
            state.growth.road -= 1;
        }
        if state.growth.funding != 0 {
            state.growth.funding -= 1;
        }
        if state.simulation.exclusive_counter != 0 {
            state.simulation.exclusive_counter -= 1;
            if state.simulation.exclusive_counter == 0 {
                state.simulation.exclusivity = CompanyID::INVALID.0;
            }
        }
        /* Check for active failed bribe cooloff periods and decrement them. */
        // Company traversal is external, so this borrow ends before it.
        let mut company = (self.services.next_company)(0);
        while company != CompanyID::INVALID.0 {
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
        state.simulation.valid_history = crate::history::update_valid_history_year(
            state.simulation.valid_history,
            u32::from(month),
        );
        /* Update cargo statistics. */
        for supplied in &mut state.supplied {
            crate::history::rotate_history_year(
                &mut supplied.history,
                state.simulation.valid_history,
                u32::from(month),
            );
        }
        for received in &mut state.simulation.received {
            received.old_max = received.new_max;
            received.new_max = 0;
            received.old_act = received.new_act;
            received.new_act = 0;
        }
    }
    /// `UpdateTownRating`: monthly update of town and station ratings.
    fn update_town_rating(&self, t: TownID) {
        /* Increase company ratings if they're low */
        let mut company = (self.services.next_company)(0);
        while company != CompanyID::INVALID.0 {
            // SAFETY: Immediate field update; borrow ends before pool traversal.
            unsafe {
                let rating = &mut (*self.owner).simulation.ratings[usize::from(company)];
                if i32::from(*rating) < RATING_GROWTH_MAXIMUM {
                    *rating = RATING_GROWTH_MAXIMUM.min(i32::from(*rating) + RATING_GROWTH_UP_STEP)
                        as i16;
                }
            }
            company = (self.services.next_company)(company + 1);
        }
        self.for_all_stations_near_town(|c, st| {
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
                    let new_rating = i32::from(*rating) + RATING_STATION_UP_STEP;
                    new_rating.min(i32::from(i16::MAX)) as i16 // do not let it overflow
                } else {
                    let new_rating = i32::from(*rating) + RATING_STATION_DOWN_STEP;
                    new_rating.max(i32::from(i16::MIN)) as i16
                };
            }
        });
        /* clamp all ratings to valid values */
        // SAFETY: All station visitors returned; no callback spans clamping.
        unsafe {
            for rating in &mut (*self.owner).simulation.ratings {
                *rating = i32::from(*rating).clamp(RATING_MINIMUM, RATING_MAXIMUM) as i16;
            }
        }
        (self.leaves.dirty_authority)(u32::from(t.0));
    }
}
pub(crate) unsafe fn openttd_rust_town_monthly_all(
    leaves: *const Leaves,
    services: *const Services,
    month: u8,
) {
    let engine = engine(leaves, services);
    for t in iterate(|from| engine.leaves.next_town(from)) {
        let owner = engine.clone().bind(t);
        owner.monthly_statistics(month);
        owner.update_town_growth(t);
        owner.update_town_rating(t);
        (owner.leaves.dirty_town)(u32::from(t.0));
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
    let t = TownID(town as u16);
    let e = engine(leaves, services).bind(t);
    let count = e.count_active_stations();
    e.update_town_rating(t);
    count
}
/// `_town_rating_test` and `_town_test_ratings`; the town rating is in test-mode
/// while `depth` is not zero.
#[derive(Default)]
struct RatingTest {
    /// Number of times test-mode is switched on.
    depth: i32,
    /// Map of towns to modified ratings, while in town rating test-mode.
    ratings: std::collections::BTreeMap<usize, i32>,
}
static RATING_TEST: GameState<RatingTest> = GameState::new(RatingTest {
    depth: 0,
    ratings: std::collections::BTreeMap::new(),
});
/// `SetTownRatingTestMode`: switch the town rating to test-mode, to allow commands
/// to be tested without affecting current ratings. The function is safe to use
/// in nested calls.
pub(crate) fn set_town_rating_test_mode(shared: &Services, mode: bool) {
    // SAFETY: Game-state-mutex serialized scratch; no callback.
    let test = unsafe { &mut *RATING_TEST.get() };
    if mode {
        if test.depth == 0 {
            test.ratings.clear();
        }
        test.depth = test.depth.wrapping_add(1);
    } else {
        // assert(ref_count > 0); the assertion policy is read only when it fails.
        if test.depth <= 0 {
            let world = crate::map_func::World::new(shared);
            world.assert_with(|| test.depth > 0);
        }
        test.depth = test.depth.wrapping_sub(1);
    }
}
/// # Safety
/// `shared` is the live shared services table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_rating_test(shared: *const Services, mode: bool) {
    set_town_rating_test_mode(unsafe { &*shared }, mode);
}
/// `GetRating`: the rating of a town for the current company. `shell` keys the
/// test-mode ratings, as the original's `Town *` does.
/// # Safety
/// `state` is the live owner of `shell` and `company` is valid.
pub(crate) unsafe fn get_rating(
    state: *const State,
    shell: *const crate::services::TownHandle,
    company: CompanyID,
) -> i32 {
    // SAFETY: Serialized immediate accesses; native shell address preserves source map keys and reuse.
    let test = unsafe { &*RATING_TEST.get() };
    if test.depth != 0 {
        if let Some(rating) = test.ratings.get(&(shell as usize)) {
            return *rating;
        }
    }
    // SAFETY: Original source invokes this only with live owner and valid company.
    unsafe { i32::from((*state).simulation.ratings[usize::from(company.0)]) }
}
/// The town of a `ChangeTownRating` call: its owner (null for no town), its index and
/// the shell address that keys the test-mode ratings.
#[derive(Clone, Copy)]
pub(crate) struct RatedTown {
    pub state: *mut State,
    pub id: TownId,
    pub shell: *const crate::services::TownHandle,
}
/// The arguments of one `ChangeTownRating` call; `company` is `_current_company` and
/// `bulldozer` the magic bulldozer cheat, both observed by the caller.
#[derive(Clone, Copy)]
pub(crate) struct RatingChange {
    pub company: CompanyID,
    pub add: i32,
    pub maximum: i32,
    pub flags: DoCommandFlags,
    pub bulldozer: bool,
}
/// `ChangeTownRating`: change the town rating of `change.company` by `change.add`,
/// without passing `change.maximum`.
/// # Safety
/// `t.state` is null or a live owner; the tables are live for the call.
pub(crate) unsafe fn change_town_rating(
    leaves: &Leaves,
    services: &Services,
    t: RatedTown,
    change: RatingChange,
) {
    let RatingChange {
        company,
        add,
        maximum,
        flags,
        bulldozer,
    } = change;
    let (state, shell) = (t.state, t.shell as usize);
    /* if magic_bulldozer cheat is active, town doesn't penalize for removing stuff */
    if state.is_null()
        || flags.test(DoCommandFlag::NoModifyTownRating)
        || !(services.company_valid)(company.0)
        || (bulldozer && add < 0)
    {
        return;
    }
    let company = company.0;
    // SAFETY: Valid company, immediate read through original scratch precedence.
    let mut rating = unsafe { get_rating(state, t.shell, CompanyID(company)) };
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
            let mut have_ratings = CompanyMask((*state).simulation.have_ratings);
            have_ratings.set(CompanyID(company));
            (*state).simulation.have_ratings = have_ratings.0;
            (*state).simulation.ratings[usize::from(company)] = rating as i16;
        }
        (leaves.dirty_authority)(u32::from(t.id.0));
    }
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
    let flags = if no_modify {
        DoCommandFlags::from_element(DoCommandFlag::NoModifyTownRating)
    } else {
        DoCommandFlags(0)
    };
    unsafe {
        change_town_rating(
            &*leaves,
            &*services,
            RatedTown {
                state,
                id: TownId(town as u16),
                shell: shell as *const crate::services::TownHandle,
            },
            RatingChange {
                company: CompanyID(company),
                add,
                maximum,
                flags,
                bulldozer,
            },
        );
    }
}

/// `Town::GetPercentTransported`: the last-month transported percentage of a cargo.
/// SAFETY: Serialized live owner; no callback or owner borrow survives this call.
pub(crate) unsafe fn get_percent_transported(state: *const State, cargo_type: CargoType) -> u8 {
    if !is_valid_cargo_type(cargo_type.0) {
        return 0;
    }
    let rows = unsafe { &(*state).supplied };
    let at = unsafe { find_supplied(state, cargo_type) };
    if at == rows.len() {
        return 0;
    }
    rows[at].history[usize::from(LAST_MONTH)].percent_transported()
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownSuppliedEntry" ["probe"] => SuppliedEntry {
        entry,
        created,
    };
    "OpenTTDTownAuthorityServices" ["probe"] => AuthorityServices {
        bulldozer,
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
        shared,
        map_log_x,

        environment,
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
        build_bridge,
        build_tunnel,
        assert_clear_house,
        houses,
    };
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    extern "C" fn fixture_random(_: *mut std::ffi::c_void) -> u32 {
        0
    }
    extern "C" fn fixture_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    /// Shared services over the static fixture map, for the rating test mode.
    fn fixture_shared() -> Services {
        crate::services::fixture(std::ptr::null_mut(), fixture_random, fixture_industry)
    }

    /// Serializes the tests that touch process-wide game state: the town owners and index,
    /// the road and tram masks, the price table and the rating-test flag.
    pub(crate) static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn gui_and_game_threads_share_original_globals() {
        let _guard = GLOBAL_LOCK.lock().unwrap();
        // Spawn/join serializes each access like the native game-state mutex.
        // Size a NewGRF-range house ID on the command thread before ottd:game.
        // SAFETY: Serialized by the lock; no view of the counts is live.
        unsafe {
            let counts = &mut *BUILDING_COUNTS.get();
            counts.ids.resize(4096, 0);
            counts.classes.resize(16, 0);
        }
        let ids = openttd_rust_town_global_id_counts();
        // SAFETY: Serialized by the lock; sized above and not resized until the end.
        let classes = unsafe { (*BUILDING_COUNTS.get()).classes.as_mut_ptr() };
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
        set_town_rating_test_mode(&fixture_shared(), true);
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
            let rating =
                unsafe { get_rating(owner, std::ptr::without_provenance(42), CompanyID(0)) };
            (
                openttd_rust_town_global_id_counts().data as usize,
                // SAFETY: The parent does not touch the counts until this join.
                unsafe { (*BUILDING_COUNTS.get()).classes.as_ptr() } as usize,
                rating,
            )
        })
        .join()
        .unwrap();
        assert_eq!(ids.data as usize, id_pointer);
        assert_eq!(classes as usize, class_pointer);
        assert_eq!(rating, -99);
        assert_eq!(take_house_candidate(0), (4095, 1));
        assert_eq!(openttd_rust_town_generating_exchange_abi(false), 0);
        // SAFETY: Child has joined; original pointers remain valid without resize.
        unsafe {
            assert_eq!(*ids.data.cast::<u32>().add(110), 1);
            assert_eq!(*classes.add(15), 1);
            assert_eq!(
                get_rating(
                    &raw const owner,
                    std::ptr::without_provenance(42),
                    CompanyID(0)
                ),
                -99
            );
            decrease_building(&raw mut owner, 110, 15);
            assert_eq!(*ids.data.cast::<u32>().add(110), 0);
            assert_eq!(*classes.add(15), 0);
        }
        assert_eq!(owner.id_counts[110], 0);
        set_town_rating_test_mode(&fixture_shared(), false);
        // SAFETY: Test scratch is disabled; the live owner's rating is restored.
        assert_eq!(
            unsafe {
                get_rating(
                    &raw const owner,
                    std::ptr::without_provenance(42),
                    CompanyID(0),
                )
            },
            77
        );
        // SAFETY: Serialized by the lock; the views above are no longer used.
        unsafe {
            let counts = &mut *BUILDING_COUNTS.get();
            counts.ids.clear();
            counts.classes.clear();
        }
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
    extern "C" fn bulldozer() -> bool {
        CHEAT.get()
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
        // The authority cases' tolerance cell and the default 20-tile authority distance.
        services.settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            town_council_tolerance: COUNCIL.with(Cell::as_ptr),
            dist_local_authority: Box::leak(Box::new(20_u8)),
            ..crate::settings_type::SettingAddresses::NULL
        }));
        services
    }
    extern "C" fn unexpected_map_log_x() -> u32 {
        unreachable!("unexpected Leaves::map_log_x")
    }
    extern "C" fn unexpected_generating_world() -> bool {
        unreachable!("unexpected Leaves::generating_world")
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
            shared: std::ptr::null(),
            map_log_x: unexpected_map_log_x,
            environment: crate::services::EnvironmentServices {
                editor: unexpected_generating_world,
                freeform_edges: unexpected_generating_world,
            },
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
            build_bridge: unexpected_build_bridge,
            build_tunnel: unexpected_build_tunnel,
            assert_clear_house: unexpected_assert_clear_house,
            houses: ptr::null(),
        }
    }

    /// Compare regenerated `"<kind> <inputs> <outputs>"` rows with the kind's oracle section,
    /// which stores only the outputs; the input field counts are the header's.
    fn check_rows(kind: char, actual: &[String], count: usize) {
        let inputs = match kind {
            'H' => 4,
            'R' => 8,
            'Z' => 1,
            'A' => 9,
            'P' => 2,
            _ => unreachable!("unknown policy oracle kind {kind}"),
        };
        let oracle = crate::oracle::parse(include_str!("town/policy-oracle.txt"));
        assert_eq!(actual.len(), count);
        let rows = actual.iter().map(|row| {
            let fields: Vec<&str> = row.split(' ').collect();
            (fields[..=inputs].join(" "), fields[inputs + 1..].join(" "))
        });
        assert_eq!(
            crate::oracle::check(&oracle, &kind.to_string(), rows),
            count
        );
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
        // Exact 4 x 5 x 5 x 4 input order from CompareHistories.
        for case in 0..400 {
            let count = [1, 3, 4, 61][case / 100];
            let production = [0_u32, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX][(case / 20) % 5];
            let transported = [0_u32, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX][(case / 4) % 5];
            let stride = [0_u32, 17, 0x4000_0000, 0x8000_0000][case % 4];
            let history = supplied_history(count, production, transported, stride);
            let result = sum_history(&history);
            rows.push(format!(
                "H {count} {production} {transported} {stride} {} {}",
                result.production, result.transported
            ));
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
                update_town_radius(&raw mut town);
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
            set_town_rating_test_mode(&self.shared, enabled);
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
                unsafe {
                    get_rating(
                        &raw const *self.town,
                        std::ptr::without_provenance(42),
                        CompanyID(company),
                    )
                }
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
            bulldozer,
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
            set_town_rating_test_mode(&services, true);
            set_town_rating_test_mode(&services, true);
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
            set_town_rating_test_mode(&services, false);
            set_town_rating_test_mode(&services, false);
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

/// `GrowTownWithTunnel` and `CanRoadContinueIntoNextTile` over the retired
/// `town-reference-gaps.cpp` fixture's controlled map (#736 W), frozen from the unchanged
/// original only (`town/tunnel-oracle.txt`).
#[cfg(test)]
mod tunnel_tests {
    use super::*;
    use crate::oracle;
    use crate::test_tables::{COMMAND_COSTS, RAIL_COMMANDS, ROAD_COMMANDS, SHARED, TOWN_LEAVES};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fmt::Write as _;
    use std::ptr;

    const ORACLE: &str = include_str!("town/tunnel-oracle.txt");
    const TOWN: u32 = 17;
    const START: u32 = 512 * 1024 + 512;
    const MP_ROAD: u32 = 2;
    const MP_RAILWAY: u32 = 1;
    const MP_STATION: u32 = 5;
    const MP_TUNNELBRIDGE: u32 = 9;
    const TRANSPORT_ROAD: u32 = 1;

    /// One fixture tile; absent tiles read as the default.
    #[derive(Clone, Copy)]
    struct Tile {
        kind: u32,
        slope: u32,
        z: u32,
        roads: u32,
        stop: u32,
        facing: u32,
        transport: u32,
        valid: bool,
    }
    const DEFAULT_TILE: Tile = Tile {
        kind: 0,
        slope: 0,
        z: 4,
        roads: 0,
        stop: 0,
        facing: 0,
        transport: TRANSPORT_ROAD,
        valid: true,
    };

    /// The fixture's world: map, command outcomes, setting and the observed trace.
    #[derive(Clone)]
    struct World {
        direction: u32,
        population: u32,
        tiles: HashMap<u32, Tile>,
        /// The fixture's failing command: 0 road test, 1 tunnel test, 2 tunnel execute.
        failure: Option<u32>,
        allow_crossings: bool,
        trace: Vec<[u32; 7]>,
        effects: u32,
        draws: u32,
    }

    thread_local! {
        static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
    }
    fn with<R>(f: impl FnOnce(&mut World) -> R) -> R {
        WORLD.with(|world| f(world.borrow_mut().as_mut().expect("fixture world")))
    }
    fn at(tile: u32) -> Tile {
        if tile >= 1024 * 1024 || tile % 1024 == 1023 || tile / 1024 == 1023 {
            return Tile {
                valid: false,
                ..DEFAULT_TILE
            };
        }
        with(|w| w.tiles.get(&tile).copied().unwrap_or(DEFAULT_TILE))
    }
    fn offset(direction: u32) -> i32 {
        [-1, 1024, 1, -1024][direction as usize]
    }
    fn along(w: &World, distance: i32) -> u32 {
        START.wrapping_add_signed(distance * offset(w.direction))
    }
    fn tile(w: &mut World, distance: i32) -> &mut Tile {
        let index = along(w, distance);
        w.tiles.entry(index).or_insert(DEFAULT_TILE)
    }
    fn trace(entry: [u32; 7]) {
        with(|w| w.trace.push(entry));
    }

    extern "C" fn map_log_x() -> u32 {
        10
    }
    extern "C" fn road_info(id: u8) -> RoadChoice {
        trace([24, u32::from(id), 0, 0, 0, 0, 0]);
        RoadChoice {
            introduction_date: 0,
            max_speed: 50,
            town_build: true,
        }
    }
    thread_local! {
        static OWNER: RefCell<State> = RefCell::new(State::default());
    }
    extern "C" fn resolve_owner(id: u32) -> *mut State {
        assert_eq!(id, TOWN);
        OWNER.with(RefCell::as_ptr)
    }
    extern "C" fn build_tunnel(tile: u32, road_type: u32, execute: u32) -> bool {
        trace([5, 0, tile, road_type, execute, 0, 0]);
        with(|w| {
            /* A failing execute can leave effects; the original ignores its return. */
            if execute != 0 {
                w.effects += 1;
            }
            w.failure != Some(if execute != 0 { 2 } else { 1 })
        })
    }
    extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
        with(|w| w.draws += 1);
        0
    }
    extern "C" fn town_by_id(id: u32) -> *mut crate::services::TownHandle {
        assert_eq!(id, TOWN);
        ptr::NonNull::dangling().as_ptr()
    }
    extern "C" fn tile_type(id: u32) -> u8 {
        at(id).kind as u8
    }
    extern "C" fn tile_valid(id: u32) -> bool {
        at(id).valid
    }
    extern "C" fn tile_slope(id: u32) -> u8 {
        at(id).slope as u8
    }
    extern "C" fn tile_z(id: u32) -> i32 {
        at(id).z as i32
    }
    extern "C" fn road_bits(id: u32, road_tram: u8, straight: bool) -> u8 {
        assert!(road_tram == 0 && straight);
        at(id).roads as u8
    }
    extern "C" fn tunnel_transport(id: u32) -> u8 {
        at(id).transport as u8
    }
    extern "C" fn facing(id: u32) -> u8 {
        at(id).facing as u8
    }
    extern "C" fn drive_through(id: u32) -> bool {
        at(id).stop == 1
    }
    extern "C" fn bay(id: u32) -> bool {
        at(id).stop == 2
    }
    extern "C" fn depot(id: u32) -> bool {
        at(id).stop == 3
    }
    unsafe extern "C" fn do_build(
        _: *const crate::road_cmd::RoadCommandServices,
        _: *mut crate::road_cmd::CommandScratch,
        out: *mut crate::services::CommandCost,
        _: *mut crate::services::CommandCost,
        flags: u16,
        tile: u32,
        bits: u8,
        road_type: u8,
        _: u8,
        town: u16,
    ) {
        assert_eq!(flags, 10);
        trace([
            1,
            u32::from(town),
            tile,
            u32::from(bits),
            u32::from(road_type),
            0,
            0,
        ]);
        // SAFETY: The context's cost slot is this test's bool.
        unsafe { *out.cast::<bool>() = with(|w| w.failure != Some(0)) };
    }
    extern "C" fn success(out: *const crate::services::CommandCost) -> bool {
        // SAFETY: The context's cost slot is this test's bool.
        unsafe { *out.cast::<bool>() }
    }

    /// Run `GrowTownWithTunnel` on one fixture; output `result effects draws | trace...`.
    fn run(world: World) -> String {
        let population = world.population;
        let direction = world.direction;
        WORLD.with(|slot| *slot.borrow_mut() = Some(world));
        OWNER.with(|owner| {
            *owner.borrow_mut() = State::default();
            owner.borrow_mut().simulation.population = population;
        });
        let mut leaves = TOWN_LEAVES;
        leaves.map_log_x = map_log_x;
        leaves.road_info = road_info;
        leaves.resolve_owner = resolve_owner;
        leaves.build_tunnel = build_tunnel;
        let mut services = SHARED;
        services.map = crate::map_func::fixture::statics();
        services.settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            allow_town_level_crossings: Box::leak(Box::new(with(|w| w.allow_crossings))),
            ..crate::settings_type::SettingAddresses::NULL
        }));
        services.random = random;
        services.town_by_id = town_by_id;
        services.tile_type = tile_type;
        services.tile_valid = tile_valid;
        services.tile_slope = tile_slope;
        services.tile_z = tile_z;
        services.road_bits = road_bits;
        services.tunnel_transport = tunnel_transport;
        services.tunnel_bridge_direction = facing;
        services.is_drive_through_stop = drive_through;
        services.drive_through_axis = facing;
        services.is_bay_stop = bay;
        services.bay_stop_direction = facing;
        services.is_road_depot = depot;
        services.road_depot_direction = facing;
        let mut costs = COMMAND_COSTS;
        costs.success = success;
        let mut rail = RAIL_COMMANDS;
        rail.cost = &raw const costs;
        let mut roads = ROAD_COMMANDS;
        roads.do_build = do_build;
        roads.rail_commands = &raw const rail;
        let mut road_ok = false;
        let context = crate::road_cmd::CommandContext {
            services: &raw const roads,
            scratch: ptr::null_mut(),
            cost: (&raw mut road_ok).cast(),
            ret: ptr::null_mut(),
        };
        let mut engine = engine(&raw const leaves, &raw const services);
        engine.road = &raw const context;
        let t = TownID(TOWN as u16);
        let result = engine.bind(t).grow_town_with_tunnel(
            t,
            TileIndex(START),
            DiagDirection(direction as u8),
        );
        let world = WORLD.with(|slot| slot.borrow_mut().take().expect("fixture world"));
        let mut text = format!("{} {} {}", u8::from(result), world.effects, world.draws);
        for entry in &world.trace {
            text += " |";
            for value in entry {
                let _ = write!(text, " {value}");
            }
        }
        text
    }

    /// The fixture's `Fixture(dir, length, mountain)`.
    fn fixture(direction: u32, length: u32, mountain: bool) -> World {
        let mut w = World {
            direction,
            population: 0,
            tiles: HashMap::new(),
            failure: None,
            allow_crossings: true,
            trace: Vec::new(),
            effects: 0,
            draws: 0,
        };
        tile(&mut w, -1).roads = 1 << (3 - direction);
        for i in 0..=length {
            let t = tile(&mut w, i as i32);
            t.z = if i == 0 || i == length { 4 } else { 5 };
            t.slope = [12, 6, 3, 9][direction as usize];
        }
        if !mountain {
            tile(&mut w, 1).kind = MP_STATION;
        }
        w
    }

    type Case = (String, World);

    /// The obstruction and mountain length matrix.
    fn length_cases(dir: u32) -> Vec<Case> {
        let lengths = [1, 2, 4, 5, 6, 7, 8, 9, 255, 256, 257];
        let populations = [0, 999, 1000, 249_000, 250_000];
        let mut cases = Vec::new();
        for (mountain, length) in [false, true]
            .into_iter()
            .flat_map(|m| lengths.into_iter().map(move |l| (m, l)))
        {
            for population in populations {
                let mut w = fixture(dir, length, mountain);
                w.population = population;
                let label = format!("length={length} population={population} mountain={mountain}");
                cases.push((label, w));
            }
        }
        cases
    }

    /// Entrance and mountain slopes, interleaved per slope.
    fn slope_cases(dir: u32) -> Vec<Case> {
        let mut cases = Vec::new();
        for slope in 0..32 {
            for (place, kind) in [(0, "entrance"), (2, "mountain")] {
                let mut w = fixture(dir, 4, true);
                tile(&mut w, place).slope = slope;
                cases.push((format!("{kind}-slope={slope}"), w));
            }
        }
        cases
    }

    /// Continuation tiles at one end: every type, stop and facing, rail tunnels, then invalid.
    fn continuation_cases(dir: u32, place: i32) -> Vec<Case> {
        let kinds = [0, MP_ROAD, MP_RAILWAY, MP_STATION, MP_TUNNELBRIDGE];
        let mut cases = Vec::new();
        for (kind, stop, facing) in kinds.into_iter().flat_map(|kind| {
            (0..4).flat_map(move |stop| (0..4).map(move |facing| (kind, stop, facing)))
        }) {
            let mut w = fixture(dir, 4, true);
            let t = tile(&mut w, place);
            t.kind = kind;
            t.stop = stop;
            t.facing = facing;
            let label =
                format!("continuation place={place} type={kind} stop={stop} facing={facing}");
            cases.push((label.clone(), w.clone()));
            if kind == MP_TUNNELBRIDGE {
                tile(&mut w, place).transport = 0;
                cases.push((format!("rail-tunnel {label}"), w));
            }
        }
        let mut w = fixture(dir, 4, true);
        tile(&mut w, place).valid = false;
        cases.push((format!("invalid-continuation place={place}"), w));
        cases
    }

    /// One direction of the fixture's `Run()`, in order.
    fn direction_cases(dir: u32) -> Vec<Case> {
        let mut cases = length_cases(dir);
        cases.extend(slope_cases(dir));
        let mut w = fixture(dir, 4, false);
        tile(&mut w, -1).roads = 0;
        cases.push(("disconnected".to_owned(), w));
        cases.extend(continuation_cases(dir, 1));
        cases.extend(continuation_cases(dir, 5));
        for place in 1..=4 {
            let mut w = fixture(dir, 4, true);
            tile(&mut w, place).valid = false;
            cases.push((format!("invalid-mountain-end place={place}"), w));
        }
        let mut w = fixture(dir, 4, true);
        tile(&mut w, 5).kind = MP_RAILWAY;
        w.allow_crossings = false;
        cases.push(("crossing-disabled".to_owned(), w));
        for failure in 0..3 {
            let mut w = fixture(dir, 4, true);
            w.failure = Some(failure);
            cases.push((format!("command-failure={failure}"), w));
        }
        cases
    }

    /// Every case of the fixture's `Run()`, in order, with its inputs.
    fn cases() -> Vec<Case> {
        (0..4)
            .flat_map(|dir| {
                direction_cases(dir)
                    .into_iter()
                    .map(move |(label, w)| (format!("{label} dir={dir}"), w))
            })
            .collect()
    }

    #[test]
    fn tunnel_growth_matches_original_fixture_matrix() {
        let _guard = super::tests::GLOBAL_LOCK.lock().unwrap();
        // The fixture's road-type registry and clock: one road type, 1950-01-01 at date 0.
        let mask = crate::type_registry::openttd_rust_road_mask();
        // SAFETY: The registry mask is a process-wide scalar serialized by the lock.
        let saved_mask = unsafe { mask.replace(1) };
        let saved_date = crate::timer_game_common::calendar_date();
        let saved_year = crate::timer_game_common::calendar_year();
        crate::timer_game_common::write_calendar_date(crate::services::CalendarDate(0));
        crate::timer_game_common::write_calendar_year(crate::timer_game_calendar::CalendarYear(
            1950,
        ));
        let oracle = oracle::parse(ORACLE);
        let actual: Vec<_> = cases()
            .into_iter()
            .map(|(inputs, world)| (inputs, run(world)))
            .collect();
        // SAFETY: Same serialized registry scalar.
        unsafe { mask.write(saved_mask) };
        crate::timer_game_common::write_calendar_date(saved_date);
        crate::timer_game_common::write_calendar_year(saved_year);
        assert_eq!(oracle::check(&oracle, "tunnel", actual), 1508);
    }
}

/// The retired `src/tests/town_protocol.hpp` fixture (#736 W, #819 D1): `CmdExpandTown`'s
/// execution, the house tile loop's rebuild, `CmdPlaceHouse`, the skipped lift tick and bound
/// growth, replayed in the fixture's order over its services. Each check is one of the fixture's
/// own; every input state comes from the fixture's earlier steps.
#[cfg(test)]
mod protocol_tests {
    use super::house::{HouseEntry, HouseLoopSpec, HouseLoopTile};
    use super::*;
    use crate::services::{CommandCost, CommandCostStorage, TownHandle};
    use crate::test_tables::{
        ANIMATION, CATCHMENT, CLEAR_GROUND, COMMAND_COSTS, COMMANDS, HOUSE_ANIMATION, HOUSES,
        LANDSCAPE, LANDSCAPE_TILES, MOVEMENT, RAIL_COMMANDS, ROAD_COMMANDS, SHARED, TILE_OWNERS,
        TOWN_LEAVES, WATER_REGION_LEAVES, unreachable0,
    };
    use std::cell::{Cell, RefCell};
    use std::ptr;

    /// The fixture's `TownProtocolWorld`: observable counters and settings.
    #[derive(Default)]
    struct World {
        population: u32,
        draws: u32,
        company: u32,
        owner_reads: u32,
        map_reads: u32,
        square_clears: u32,
        house_callback: bool,
        clear_fails: bool,
        placing: bool,
        house_class: u16,
        roads: u32,
        clears: u32,
        animations: u32,
        landscape_clears: u32,
        house_count: u32,
        writes: Vec<[u32; 4]>,
    }
    thread_local! {
        static WORLD: RefCell<World> = RefCell::new(World::default());
        static OWNER: Cell<*mut State> = const { Cell::new(ptr::null_mut()) };
    }
    fn with<R>(f: impl FnOnce(&mut World) -> R) -> R {
        WORLD.with(|world| f(&mut world.borrow_mut()))
    }
    fn owner() -> *mut State {
        OWNER.get()
    }

    extern "C" fn map_log_x() -> u32 {
        with(|w| w.map_reads += 1);
        6
    }
    extern "C" fn no() -> bool {
        false
    }
    extern "C" fn zero_byte() -> u8 {
        0
    }
    extern "C" fn zero_flags(_: u8) -> u8 {
        0
    }
    extern "C" fn house_availability(_: u16) -> HouseAvailability {
        HouseAvailability {
            zones: 0xFFFF,
            override_id: u16::MAX,
            class_id: with(|w| w.house_class),
            probability: 1,
            enabled: true,
        }
    }
    extern "C" fn house_placement(_: u16) -> HousePlacement {
        HousePlacement {
            min_year: 0,
            max_year: 5000,
        }
    }
    extern "C" fn house_population(house: u16) -> u8 {
        if house == 2 { 0 } else { 10 }
    }
    extern "C" fn house_flags(house: u16) -> u8 {
        if house == 0 || house == 2 { 1 } else { 16 }
    }
    extern "C" fn house_extra_flags(_: u16) -> u8 {
        0
    }
    extern "C" fn house_callbacks(_: u16) -> u16 {
        (1 << 3) | u16::from(with(|w| w.house_callback))
    }
    extern "C" fn road_info(_: u8) -> RoadChoice {
        RoadChoice {
            introduction_date: 0,
            max_speed: 0,
            town_build: false,
        }
    }
    extern "C" fn resolve_owner(_: u32) -> *mut State {
        with(|w| w.owner_reads += 1);
        owner()
    }
    extern "C" fn dirty(_: u32) {}
    extern "C" fn terraform_price() -> i64 {
        0
    }
    extern "C" fn set_road_town(_: u32, _: u32) {}
    extern "C" fn exchange_company(company: u32) -> u32 {
        with(|w| std::mem::replace(&mut w.company, company))
    }
    extern "C" fn house_class(id: u16) -> u16 {
        if id == 2 { 0 } else { with(|w| w.house_class) }
    }
    extern "C" fn make_house(tile: u32, town: u32, house: u32, construction: u32) {
        with(|w| w.writes.push([tile, town, house, construction]));
    }
    extern "C" fn population_changed(_: u32) {
        // SAFETY: The fixture's live owner; no other borrow spans this leaf.
        let population = unsafe { (*owner()).simulation.population };
        with(|w| w.population = population);
    }
    extern "C" fn house_count() -> u32 {
        with(|w| w.house_count)
    }
    extern "C" fn next_town(_: u32) -> u32 {
        u32::MAX
    }
    extern "C" fn no_bridge(_: u32, _: u32, _: u32, _: u32, _: u32) -> bool {
        false
    }
    extern "C" fn no_tunnel(_: u32, _: u32, _: u32) -> bool {
        false
    }
    extern "C" fn assert_clear_house(_: bool) {}
    extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
        with(|w| w.draws += 1);
        0xABCD_EF01
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    /// The fixture's road command: count it; a test succeeds and an execution fails.
    unsafe extern "C" fn do_build(
        _: *const crate::road_cmd::RoadCommandServices,
        _: *mut crate::road_cmd::CommandScratch,
        out: *mut CommandCost,
        _: *mut CommandCost,
        flags: u16,
        _: u32,
        _: u8,
        _: u8,
        _: u8,
        _: u16,
    ) {
        with(|w| w.roads += 1);
        // SAFETY: The context's cost slot is this test's bool.
        unsafe { *out.cast::<bool>() = flags & 1 == 0 };
    }
    extern "C" fn success(out: *const CommandCost) -> bool {
        // SAFETY: Every fixture cost is one bool.
        unsafe { *out.cast::<bool>() }
    }

    /// The fixture's `TownProtocolCost`: one success flag, value-initialised to false.
    unsafe extern "C" fn construct(storage: *mut CommandCostStorage) -> *mut CommandCost {
        let cost = storage.cast::<bool>();
        // SAFETY: The storage is the command's own aligned stack capacity.
        unsafe { cost.write(false) };
        cost.cast()
    }
    unsafe extern "C" fn destroy(_: *mut CommandCost) {}
    fn flag(cost: *const CommandCost) -> *mut bool {
        cost.cast::<bool>().cast_mut()
    }
    extern "C" fn expense_money(cost: *mut CommandCost, _: u8, _: i64) {
        // SAFETY: A live fixture cost.
        unsafe { *flag(cost) = true };
    }
    extern "C" fn add(cost: *mut CommandCost, other: *mut CommandCost) {
        // SAFETY: Two live fixture costs.
        unsafe { *flag(cost) &= *flag(other) };
    }
    extern "C" fn money(_: *const CommandCost) -> i64 {
        0
    }
    extern "C" fn zero(cost: *mut CommandCost) {
        // SAFETY: A live fixture cost.
        unsafe { *flag(cost) = true };
    }
    extern "C" fn error(cost: *mut CommandCost, _: u32) {
        // SAFETY: A live fixture cost.
        unsafe { *flag(cost) = false };
    }
    /// The landscape tile procedure: count each clear (a placement's tests apart) and apply the
    /// fixture's outcome.
    extern "C" fn clear(cost: *mut CommandCost, _: u32, flags: u16) {
        let ok = with(|w| {
            if w.placing && flags & 1 == 0 {
                w.landscape_clears += 1;
            } else {
                w.clears += 1;
            }
            !w.clear_fails
        });
        // SAFETY: A live fixture cost.
        unsafe { *flag(cost) = ok };
    }
    extern "C" fn town_index(_: *const TownHandle) -> u16 {
        0
    }

    /// House 2 is an unpopulated, completed old 1x1 house; its rebuild reaches
    /// `TryBuildTownHouse`.
    extern "C" fn house_entry(_: u32) -> HouseEntry {
        HouseEntry {
            town: house_town(0),
            house: 2,
        }
    }
    extern "C" fn house_town(_: u32) -> TownRef {
        with(|w| w.owner_reads += 1);
        TownRef {
            shell: ptr::null_mut(),
            state: owner(),
            id: TownId(0),
        }
    }
    extern "C" fn loop_tile(_: u32) -> HouseLoopTile {
        HouseLoopTile {
            age: 255,
            completed: true,
            lift_has_destination: false,
        }
    }
    extern "C" fn house_type(_: u32) -> u16 {
        2
    }
    extern "C" fn completed(_: u32) -> bool {
        true
    }
    extern "C" fn loop_spec(id: u16) -> HouseLoopSpec {
        HouseLoopSpec {
            callbacks: house_callbacks(id),
            flags: house_flags(id),
            population: house_population(id),
            mail: 0,
            minimum_life: 0,
        }
    }
    extern "C" fn delete_inspect(_: u32) {}
    extern "C" fn cargogen_mode() -> u8 {
        0
    }
    extern "C" fn place_houses() -> u8 {
        2
    }

    /// The callback reenters the town owner through the direct resolver entry; the original
    /// fixture's mutation and success are preserved.
    extern "C" fn evaluate(_: *const resolver::CallbackArgs, _: *mut i32, _: usize) -> u16 {
        // SAFETY: The fixture's live owner; no other borrow spans this callback.
        unsafe { (*owner()).growth.flags = 0x88 };
        u16::MAX
    }
    extern "C" fn animation_callback(
        _: u16,
        _: u32,
        _: u32,
        _: *const crate::animated_tile::HouseSpec,
        _: *mut TownHandle,
        _: u32,
        _: u64,
    ) -> u16 {
        with(|w| w.animations += 1);
        u16::MAX
    }
    extern "C" fn animation_state(_: u32) -> crate::animated_tile_map::AnimatedTileState {
        crate::animated_tile_map::AnimatedTileState::DELETED
    }
    extern "C" fn set_animation_state(_: u32, _: crate::animated_tile_map::AnimatedTileState) {}
    extern "C" fn animation_spec(_: u16) -> *const crate::animated_tile::HouseSpec {
        ptr::null()
    }

    extern "C" fn no_tile(_: u32) -> bool {
        false
    }
    extern "C" fn yes_tile(_: u32) -> bool {
        true
    }
    extern "C" fn human(_: u8) -> bool {
        true
    }
    extern "C" fn no_town_shell(_: u32) -> *mut TownHandle {
        ptr::null_mut()
    }
    extern "C" fn no_road_bits(_: u32, _: u8, _: bool) -> u8 {
        0
    }
    extern "C" fn clear_type(_: u32) -> u8 {
        0
    }
    extern "C" fn flat(_: u32) -> u8 {
        0
    }
    extern "C" fn sea_level(_: u32) -> i32 {
        0
    }
    extern "C" fn untouched(_: u32) {}
    extern "C" fn map_max() -> u32 {
        63
    }
    extern "C" fn no_towns() -> usize {
        0
    }
    extern "C" fn one_town() -> usize {
        1
    }
    extern "C" fn first_town(from: u32) -> u32 {
        if from == 0 { 0 } else { u32::MAX }
    }
    extern "C" fn no_next_town(_: u32) -> u32 {
        u32::MAX
    }
    extern "C" fn town_owner_by_id(_: u32) -> *mut State {
        owner()
    }
    extern "C" fn town_owner(_: *const TownHandle) -> *mut State {
        with(|w| w.owner_reads += 1);
        owner()
    }
    extern "C" fn no_road_type(_: u32) -> u8 {
        63
    }
    extern "C" fn no_stations() -> usize {
        0
    }
    extern "C" fn second_tick() -> u64 {
        1
    }
    extern "C" fn road_at_centre(tile: u32, _: u8, _: bool) -> u8 {
        u8::from(tile == 2080)
    }
    extern "C" fn road_tile_at_centre(tile: u32) -> u8 {
        if tile == 2080 {
            crate::tile_type::MP_ROAD
        } else {
            0
        }
    }
    extern "C" fn road_type_at_centre(tile: u32) -> u8 {
        if tile == 2080 { 0 } else { 63 }
    }

    thread_local! {
        static LANDSCAPE_TABLE: Cell<*const crate::landscape_cmd::LandscapeServices> =
            const { Cell::new(ptr::null()) };
        static SHARED_TABLE: Cell<*const Services> = const { Cell::new(ptr::null()) };
        static CATCHMENT_TABLE: Cell<*const crate::station::CatchmentServices> =
            const { Cell::new(ptr::null()) };
        static WATER_TABLE: Cell<*const crate::water::WaterServices> = const { Cell::new(ptr::null()) };
        static REGIONS: Cell<crate::water_regions::Context> = const {
            Cell::new(crate::water_regions::Context {
                owner: ptr::null_mut(),
                leaves: ptr::null(),
            })
        };
    }
    /// The synthetic map has no docking tiles and no water neighbours.
    extern "C" fn make_clear(_: u32, _: crate::services::ClearGroundType, _: u32) {
        with(|w| w.square_clears += 1);
    }
    unsafe extern "C" fn debug(_: u8, _: i32, _: i32) {}
    extern "C" fn water_services() -> *const crate::water::WaterServices {
        WATER_TABLE.get()
    }
    extern "C" fn region_context() -> crate::water_regions::Context {
        REGIONS.get()
    }
    extern "C" fn station_catchment() -> *const crate::station::CatchmentServices {
        CATCHMENT_TABLE.get()
    }
    extern "C" fn landscape_commands() -> *const crate::landscape_cmd::LandscapeServices {
        LANDSCAPE_TABLE.get()
    }
    /// Rust owns `MP_CLEAR` clearing; the landscape command alone sees this ground as a
    /// kind still dispatched to the native procedure above.
    extern "C" fn landscape_tile_type(tile: u32) -> u8 {
        // SAFETY: The test's shared table outlives every landscape command.
        let kind = unsafe { ((*SHARED_TABLE.get()).tile_type)(tile) };
        if kind == crate::tile_type::MP_CLEAR {
            crate::tile_type::MP_TUNNELBRIDGE
        } else {
            kind
        }
    }

    /// The fixture's tables live for the test process; its tests hold `&'static` chains, as the
    /// native facades do.
    fn leak<T>(table: T) -> &'static T {
        Box::leak(Box::new(table))
    }

    /// The fixture's 64x64 map: flat, empty clear ground, installed for this thread.
    fn fixture_map() -> (
        Vec<crate::map_func::BaseTileLayout>,
        Vec<crate::map_func::ExtendedTileLayout>,
    ) {
        let base: Vec<_> = (0..4096)
            .map(|_| crate::map_func::BaseTileLayout {
                kind: 0,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect();
        let extended: Vec<_> = (0..4096)
            .map(|_| crate::map_func::ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: base.as_ptr(),
                extended: extended.as_ptr(),
                count: 4096,
                assertions: true,
            },
            6,
        );

        (base, extended)
    }

    /// The fixture's town: 5 house IDs, 2 classes, centred on tile 2080 with radii of 100.
    fn fixture_owner() -> *mut State {
        let mut owner = Box::new(State::default());
        owner.id_counts = vec![0; 5];
        owner.class_counts = vec![0; 2];
        owner.simulation.xy = 2080;
        owner.simulation.radius = [100; 5];
        let owner = Box::into_raw(owner);
        OWNER.set(owner);
        WORLD.with(|w| {
            *w.borrow_mut() = World {
                company: 7,
                house_count: 1,
                ..World::default()
            }
        });

        owner
    }

    /// The fixture's water services: no docking or water neighbours; square clears counted.
    fn install_water(regions: *mut crate::water_regions::Regions) {
        let mut region_leaves = WATER_REGION_LEAVES;
        region_leaves.debug = debug;
        REGIONS.set(crate::water_regions::Context {
            owner: regions,
            leaves: leak(region_leaves),
        });
        let mut clear_ground = CLEAR_GROUND;
        clear_ground.make_clear = make_clear;
        let mut water = crate::water::fixture();
        water.clear = leak(clear_ground);
        water.region_context = region_context;
        WATER_TABLE.set(leak(water));
    }

    /// `OpenTTDFixtureSharedServices` with the fixture's overrides: flat, empty ground without
    /// roads, depots, bridges or stations, and no town yet.
    fn fixture_services() -> *mut Services {
        let mut services = crate::services::fixture(ptr::null_mut(), random, industry);
        services.company_human = human;
        // Town roads and level crossings are disabled; growth uses the normal rate.
        services.settings = leak(crate::settings_type::SettingAddresses {
            allow_town_roads: leak(false),
            town_growth_rate: leak(2),
            allow_town_level_crossings: leak(false),
            ..crate::settings_type::SettingAddresses::NULL
        });
        services.town_by_id = no_town_shell;
        services.is_road_depot = no_tile;
        services.is_bay_stop = no_tile;
        services.road_bits = no_road_bits;
        services.tile_type = clear_type;
        services.tile_slope = flat;
        let mut movement_world = SHARED;
        movement_world.tile_type = clear_type;
        movement_world.map = crate::map_func::fixture::statics();
        let mut movement = MOVEMENT;
        movement.shared = leak(movement_world);
        services.movement = leak(movement);
        services.tile_max_z = sea_level;
        services.tile_z = sea_level;
        services.has_water_ground = no_tile;
        services.mark_dirty = untouched;
        services.clear_square_services.docking = no_tile;
        services.clear_square_services.generating_world = no;
        services.water_services = water_services;
        services.station_catchment = station_catchment;
        services.map = crate::map_func::fixture::statics();
        services.map_max_x = map_max;
        services.map_max_y = map_max;
        services.town_count = no_towns;
        services.town_next_id = next_town;
        services.tile_valid = yes_tile;
        services.road_type = no_road_type;
        let services = Box::into_raw(Box::new(services));
        SHARED_TABLE.set(services);
        // No stations exist, so placed houses find no nearby station.
        let mut catchment = CATCHMENT;
        catchment.shared = services;
        catchment.station_count = no_stations;
        CATCHMENT_TABLE.set(leak(catchment));

        services
    }

    /// The fixture's leaves, with the house services the rebuild and `CmdPlaceHouse` reach.
    fn fixture_leaves(services: *const Services) -> &'static Leaves {
        let mut costs = COMMAND_COSTS;
        costs.construct = construct;
        costs.destroy = destroy;
        costs.expense_money = expense_money;
        costs.add = add;
        costs.money = money;
        costs.zero = zero;
        costs.error = error;
        costs.success = success;
        let costs = leak(costs);
        let mut owners = TILE_OWNERS;
        owners.shared = services;
        let mut tiles = LANDSCAPE_TILES;
        tiles.clear = clear;
        tiles.owners = leak(owners);
        let mut commands = COMMANDS;
        commands.landscape_commands = landscape_commands;
        let commands = leak(commands);
        // SAFETY: Read once, before any later override; the copy keeps the setup's services.
        let mut landscape_world = unsafe { services.read() };
        landscape_world.tile_type = landscape_tile_type;
        let mut landscape = LANDSCAPE;
        landscape.shared = leak(landscape_world);
        landscape.costs = costs;
        landscape.commands = commands;
        landscape.tile_procs = leak(tiles);
        LANDSCAPE_TABLE.set(leak(landscape));
        let mut houses = HOUSES;
        houses.cost = costs;
        houses.commands = commands;
        houses.authority = leak(AuthorityServices {
            bulldozer: unreachable0,
            town_index,
            refusal: 0,
        });
        houses.entry = house_entry;
        houses.loop_tile = loop_tile;
        houses.house_type = house_type;
        houses.completed = completed;
        houses.town = house_town;
        houses.loop_spec = loop_spec;
        houses.delete_inspect = delete_inspect;
        houses.cargogen_mode = cargogen_mode;
        houses.place_houses = place_houses;
        let mut leaves = TOWN_LEAVES;
        leaves.shared = services;
        leaves.map_log_x = map_log_x;
        leaves.environment.editor = no;
        leaves.environment.freeform_edges = no;
        leaves.climate = zero_byte;
        leaves.road_flags = zero_flags;
        leaves.house_availability = house_availability;
        leaves.house_placement = house_placement;
        leaves.house_population = house_population;
        leaves.house_flags = house_flags;
        leaves.house_extra_flags = house_extra_flags;
        leaves.house_callbacks = house_callbacks;
        leaves.road_info = road_info;
        leaves.resolve_owner = resolve_owner;
        leaves.dirty_authority = dirty;
        leaves.dirty_town = dirty;
        leaves.terraform_price = terraform_price;
        leaves.set_road_town = set_road_town;
        leaves.exchange_company = exchange_company;
        leaves.house_class = house_class;
        leaves.make_house = make_house;
        leaves.population_changed = population_changed;
        leaves.house_count = house_count;
        leaves.build_bridge = no_bridge;
        leaves.build_tunnel = no_tunnel;
        leaves.assert_clear_house = assert_clear_house;
        leaves.houses = leak(houses);
        leak(leaves)
    }

    /// The fixture's resolver and animation tables, installed like the native scopes; returns the
    /// previous tables.
    fn install_callbacks(
        services: *const Services,
    ) -> (
        *const resolver::ResolverServices,
        *const crate::animated_tile::Services,
    ) {
        let mut callbacks = resolver::TEST_TABLE;
        callbacks.shared = services;
        callbacks.evaluate = evaluate;
        let mut binding = HOUSE_ANIMATION;
        binding.callback = animation_callback;
        let mut animation = ANIMATION;
        animation.shared = services;
        animation.house = leak(binding);
        animation.state = animation_state;
        animation.set_state = set_animation_state;
        animation.house_spec = animation_spec;
        // SAFETY: Both tables are leaked, so they outlive their installation.
        unsafe {
            (
                resolver::openttd_rust_house_resolver_exchange(leak(callbacks)),
                crate::animated_tile::openttd_rust_animation_exchange(leak(animation)),
            )
        }
    }

    /// The fixture's road command context: tests succeed, executions fail.
    fn fixture_road() -> &'static crate::road_cmd::CommandContext {
        let mut costs = COMMAND_COSTS;
        costs.success = success;
        let mut rail = RAIL_COMMANDS;
        rail.cost = leak(costs);
        let mut roads = ROAD_COMMANDS;
        roads.do_build = do_build;
        roads.rail_commands = leak(rail);
        leak(crate::road_cmd::CommandContext {
            services: leak(roads),
            scratch: ptr::null_mut(),
            cost: Box::into_raw(Box::new(false)).cast(),
            ret: ptr::null_mut(),
        })
    }

    /// The fixture's checks after each expansion (`town_protocol.hpp` lines 301, 303 and 308).
    fn check_expansions(owner: *mut State, expand: impl Fn(u8)) {
        let counts = || with(|w| (w.roads, w.draws, w.company));

        // town_protocol.hpp:301: buildings only.
        expand(1);
        assert_eq!(counts(), (0, 0, 7), "buildings: roads, draws, company");
        // town_protocol.hpp:303: roads; the first road's execute failure still succeeds.
        expand(2);
        assert_eq!(counts(), (1, 1, 7), "roads: roads, draws, company");
        // town_protocol.hpp:308: expansion updates radii; restore the isolated setup. A failing
        // clear makes each of the 25 growth attempts clear 13 tiles, with no further draw.
        // SAFETY: The fixture's live owner; no borrow spans this write.
        unsafe { (*owner).simulation.radius = [100; 5] };
        with(|w| {
            w.clears = 0;
            w.clear_fails = true;
        });
        expand(2);
        assert_eq!(
            with(|w| (w.clears, w.company, w.draws)),
            (25 * 13, 7, 1),
            "failing clears: clears, company, draws"
        );
        with(|w| w.clear_fails = false);
    }

    /// A growing town rebuilds the expired house: one draw, clear, then `TryBuildTownHouse`
    /// (`town_protocol.hpp` lines 311 to 327).
    fn check_rebuilds(owner: *mut State, leaves: &Leaves, services: *const Services) {
        let rebuild = move || {
            // SAFETY: The fixture's live owner; no borrow spans these writes.
            unsafe {
                (*owner).simulation.time_until_rebuild = 1;
                (*owner).simulation.num_houses += 1;
                (*owner).growth.flags |= 1;
            }
            with(|w| (w.owner_reads, w.map_reads) = (0, 0));
            // SAFETY: Live fixture tables and owner.
            unsafe { house::tile_loop(leaves, services, TileIndex(2080)) };
            assert_eq!(
                with(|w| (w.owner_reads, w.map_reads)),
                (1, 1),
                "rebuild: reads"
            );
        };
        with(|w| w.house_callback = true);
        // SAFETY: The fixture's live owner; no borrow spans this write.
        unsafe { (&mut (*owner).id_counts)[0] = u16::MAX };
        rebuild();
        assert_eq!(
            with(|w| (w.writes.len(), w.draws, w.company)),
            (0, 2, 7),
            "full house id: writes, draws, company"
        );
        // SAFETY: As above.
        assert_eq!(
            unsafe { (*owner).simulation.time_until_rebuild },
            0xCD + 192
        );
        // SAFETY: As above.
        unsafe {
            (&mut (*owner).id_counts)[0] = 0;
            (&mut (*owner).class_counts)[1] = u16::MAX;
        }
        with(|w| w.house_class = 1);
        rebuild();
        assert_eq!(
            with(|w| (w.writes.len(), w.draws)),
            (0, 3),
            "full class: writes, draws"
        );
        // SAFETY: As above.
        unsafe { (&mut (*owner).class_counts)[1] = 0 };
        with(|w| w.house_class = 0);
        rebuild();
        // SAFETY: As above.
        let (houses, flags) = unsafe { ((*owner).simulation.num_houses, (*owner).growth.flags) };
        assert_eq!(
            (houses, with(|w| (w.writes.len(), w.animations)), flags),
            (1, (1, 1), 0x88),
            "rebuilt: houses, (writes, animations), growth flags"
        );
        assert_eq!(with(|w| w.square_clears), 3, "rebuilds: square clears");
    }

    /// `CmdPlaceHouse` executes a completed, protected 2x2 house through `BuildTownHouse`
    /// (`town_protocol.hpp` lines 328 to 346).
    fn check_placement(owner: *mut State, leaves: &Leaves, services: *mut Services) {
        with(|w| {
            w.placing = true;
            w.writes.clear();
            w.house_callback = false;
            w.clears = 0;
            w.animations = 0;
            w.house_count = 5;
        });
        let mut cost = false;
        // SAFETY: The test's own services; no table reads them during these writes.
        unsafe {
            (*services).town_count = one_town;
            (*services).town_next_id = first_town;
            (*services).town_owner_by_id = town_owner_by_id;
            spatial::openttd_rust_town_rebuild_index(services);
            (*services).town_owner = town_owner;
        }
        with(|w| (w.owner_reads, w.map_reads) = (0, 0));
        // SAFETY: Live fixture tables, owner and cost.
        unsafe {
            house::openttd_rust_house_place(
                leaves,
                services,
                (&raw mut cost).cast(),
                1,
                2080,
                1,
                true,
                false,
            );
        }
        assert_eq!(
            with(|w| (w.owner_reads, w.map_reads)),
            (1, 1),
            "placement: reads"
        );
        assert_eq!(
            (cost, with(|w| (w.landscape_clears, w.clears, w.animations))),
            (true, (4, 4, 4)),
            "placement: success, (landscape clears, clears, animations)"
        );
        let writes = with(|w| w.writes.iter().map(|w| (w[0], w[2])).collect::<Vec<_>>());
        assert_eq!(
            writes,
            [(2080, 1), (2144, 2), (2081, 3), (2145, 4)],
            "placement: (tile, house) writes"
        );
        // SAFETY: The fixture's live owner; no borrow spans these reads.
        let (houses, ids) = unsafe { ((*owner).simulation.num_houses, (*owner).id_counts.clone()) };
        assert_eq!(
            (with(|w| w.population), houses),
            (10, 2),
            "placement: population, houses"
        );
        assert_eq!(ids, [1; 5], "placement: house id counts");
    }

    /// A skipped builtin lift tick observes neither owner nor map (`town_protocol.hpp` lines 347
    /// to 352); the lift observation stays unreachable, so an eager read fails.
    fn check_skipped_lift(leaves: &Leaves, services: *mut Services) {
        // SAFETY: The test's own services; no table reads them during this write.
        unsafe { (*services).tick_counter = second_tick };
        with(|w| (w.owner_reads, w.map_reads) = (0, 0));
        // SAFETY: Live fixture tables and owner.
        unsafe { house::animate(leaves, services, TileIndex(2080)) };
        assert_eq!(
            with(|w| (w.owner_reads, w.map_reads)),
            (0, 0),
            "skipped lift: reads"
        );
    }

    /// D1 observation 4 (`town_protocol.hpp` lines 353 to 361): ordinary growth binds its town
    /// before walking the road; construction triggers reuse that context rather than resolve the
    /// owner again.
    fn check_bound_growth(services: *mut Services, expand: impl Fn(u8)) {
        // SAFETY: The test's own services; no table reads them during these writes.
        unsafe {
            (*services).road_bits = road_at_centre;
            (*services).tile_type = road_tile_at_centre;
            (*services).road_type = road_type_at_centre;
        }
        with(|w| {
            w.writes.clear();
            w.animations = 0;
            w.house_count = 1;
            (w.owner_reads, w.map_reads) = (0, 0);
        });
        expand(1);
        assert_eq!(
            with(|w| (w.writes.len(), w.animations, w.owner_reads, w.map_reads)),
            (1, 1, 1, 1),
            "bound growth: writes, animations, owner reads, map reads"
        );
    }

    #[test]
    fn protocol_matches_fixture_checks() {
        let _guard = super::tests::GLOBAL_LOCK.lock().unwrap();
        // Commands the protocol reaches run nested, without the top-level money steps.
        crate::command::openttd_rust_command_enter();
        let mask = crate::type_registry::openttd_rust_road_mask();
        // SAFETY: Process-wide registry scalar serialized by the lock.
        let saved_mask = unsafe { mask.replace(0) };
        let saved_year = crate::timer_game_common::calendar_year();
        let saved_date = crate::timer_game_common::calendar_date();
        crate::timer_game_common::write_calendar_year(crate::timer_game_calendar::CalendarYear(
            1950,
        ));
        crate::timer_game_common::write_calendar_date(crate::services::CalendarDate(0));
        // Preserve the process-wide game counters around the isolated fixture.
        // SAFETY: Serialized by the lock; no view of the counters is live.
        let saved_counts = unsafe {
            let counts = &mut *BUILDING_COUNTS.get();
            let saved = (counts.ids.clone(), counts.classes.clone());
            counts.ids.resize(saved.0.len().max(5), 0);
            counts.classes.resize(saved.1.len().max(2), 0);
            saved
        };
        // The tile arrays stay allocated until the test ends.
        let _map = fixture_map();
        let owner = fixture_owner();
        let regions = crate::water_regions::openttd_rust_water_new(64, 64);
        install_water(regions);
        let services = fixture_services();
        let leaves = fixture_leaves(services);
        let (resolver, animation) = install_callbacks(services);
        let road = fixture_road();
        let expand = |modes: u8| {
            let mut engine = engine(leaves, services);
            engine.road = road;
            let t = TownID(0);
            engine.bind(t).expand_town(t, 1, TownExpandModes(modes));
        };

        check_expansions(owner, expand);
        check_rebuilds(owner, leaves, services);
        check_placement(owner, leaves, services);
        check_skipped_lift(leaves, services);
        check_bound_growth(services, expand);

        // SAFETY: Serialized by the lock; the fixture's tables are restored or unused hereafter.
        unsafe {
            let counts = &mut *BUILDING_COUNTS.get();
            (counts.ids, counts.classes) = saved_counts;
            (*services).town_count = no_towns;
            (*services).town_next_id = no_next_town;
            spatial::openttd_rust_town_rebuild_index(services);
            resolver::openttd_rust_house_resolver_exchange(resolver);
            crate::animated_tile::openttd_rust_animation_exchange(animation);
            mask.write(saved_mask);
            crate::water_regions::openttd_rust_water_destroy(regions);
        }
        crate::timer_game_common::write_calendar_year(saved_year);
        crate::timer_game_common::write_calendar_date(saved_date);
        crate::command::openttd_rust_command_leave();
        OWNER.set(ptr::null_mut());
        // SAFETY: Created by `fixture_owner`; no table or index refers to it any more.
        drop(unsafe { Box::from_raw(owner) });
    }
}
