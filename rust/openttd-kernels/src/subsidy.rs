/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Subsidy owners, route searches, expiry and delivery awards. Pool shells keep identity
//! and allocation order; script events enqueue without entering a VM. Source arithmetic
//! keeps its narrowing and wrapping.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::comparison_chain,
    clippy::if_not_else
)]
use crate::cargo_type::{CargoType, NUM_CARGO, is_valid_cargo_type};
use crate::cargotype::TownProductionEffect;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, Owner};
use crate::game_state::GameState;
use crate::industry_type::IndustryID;
pub use crate::source_type::Source;
use crate::station::Station;
use crate::station_type::StationID;
use crate::subsidy_type::{PartOfSubsidy, SubsidyID};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
use crate::town_type::TownID;
use crate::{industry, services::Services as Shared, town};
use crate::{
    subsidy_base::{
        SUBSIDY_CARGO_MIN_POPULATION, SUBSIDY_MAX_DISTANCE, SUBSIDY_MAX_PCT_TRANSPORTED,
        SUBSIDY_OFFER_MONTHS, SUBSIDY_PAX_MIN_POPULATION, SUBSIDY_TOWN_CARGO_RADIUS,
    },
    timer_game_common::MONTHS_IN_YEAR,
};
use std::{ffi::c_void, ptr};
/// `Subsidy`'s fields, shared in place with the native pool shell.
#[repr(C)]
pub struct State {
    /// Cargo type involved in this subsidy, `INVALID_CARGO` for invalid subsidy
    pub cargo_type: CargoType,
    /// Remaining months when this subsidy is valid
    pub remaining: u16,
    /// Subsidy is awarded to this company; `CompanyID::Invalid()` if it's not awarded to anyone
    pub awarded: CompanyID,
    /// Source of subsidised path
    pub src: Source,
    /// Destination of subsidised path
    pub dst: Source,
}
impl State {
    /// Tests whether this subsidy has been awarded to someone
    fn is_awarded(&self) -> bool {
        self.awarded != CompanyID::INVALID
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cargo {
    pub data: *mut u32,
}
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub town: *const crate::town::Leaves,
    pub industry: *const crate::industry::Services,
    pub industry_owner: extern "C" fn(*mut c_void) -> *mut crate::industry::Industry,
    pub industry_id: extern "C" fn(*mut c_void) -> u16,
    pub next_industry: extern "C" fn(u32) -> u32,
    pub has_source_flag: extern "C" fn(Source, PartOfSubsidy) -> bool,
    pub set_source_flag: extern "C" fn(Source, PartOfSubsidy),
    pub clear_source_flags: extern "C" fn(Source),
    pub valid_source: extern "C" fn(Source) -> bool,
    pub can_allocate: extern "C" fn() -> bool,
    pub allocate: extern "C" fn(u8, Source, Source, u16) -> u16,
    pub destroy: extern "C" fn(u16),
    pub duration: extern "C" fn() -> u16,
    pub local_company: extern "C" fn() -> u8,
    pub cargo_valid: extern "C" fn(u8) -> bool,
    pub manual: extern "C" fn(u8) -> bool,
    pub manual_pax: extern "C" fn() -> bool,
    pub any_manual: extern "C" fn() -> bool,
    pub offered_news: extern "C" fn(u16),
    pub awarded_news: extern "C" fn(u16),
    pub offer_expired_news: extern "C" fn(u16),
    pub withdrawn_news: extern "C" fn(u16),
    pub offer_events: extern "C" fn(u16),
    pub award_events: extern "C" fn(u16),
    pub offer_expired_events: extern "C" fn(u16),
    pub expired_events: extern "C" fn(u16),
    pub dirty: extern "C" fn(),
    pub station: extern "C" fn(u16) -> *const Station,
    pub assert_destination: extern "C" fn(Source),
}
struct RegistryState {
    slots: [*mut State; SubsidyID::END.0 as usize],
    limit: u16,
}
static OWNERS: GameState<RegistryState> = GameState::new(RegistryState {
    slots: [ptr::null_mut(); SubsidyID::END.0 as usize],
    limit: 0,
});
fn owner(id: SubsidyID) -> *mut State {
    // SAFETY: Serialized native pool index; no registry borrow crosses callbacks.
    unsafe { (*OWNERS.get()).slots[usize::from(id.0)] }
}
fn each(mut visit: impl FnMut(SubsidyID, *mut State)) {
    let mut id = 0;
    while id < unsafe { (*OWNERS.get()).limit } {
        let state = owner(SubsidyID(id));
        if !state.is_null() {
            visit(SubsidyID(id), state);
        }
        id += 1;
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_subsidy_new(id: u16) -> *mut State {
    let state = Box::into_raw(Box::new(State {
        cargo_type: CargoType::INVALID_CARGO,
        remaining: 0,
        awarded: CompanyID::INVALID,
        src: Source::default(),
        dst: Source::default(),
    }));
    // SAFETY: The constructing native pool shell owns this stable allocation.
    unsafe {
        (*OWNERS.get()).slots[usize::from(id)] = state;
        (*OWNERS.get()).limit = (*OWNERS.get()).limit.max(id + 1);
    }
    state
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_destroy(id: u16, state: *mut State) {
    // SAFETY: Native pool destroys exactly once, after every scalar alias ends.
    unsafe {
        (*OWNERS.get()).slots[usize::from(id)] = ptr::null_mut();
        while (*OWNERS.get()).limit > 0
            && (*OWNERS.get()).slots[usize::from((*OWNERS.get()).limit - 1)].is_null()
        {
            (*OWNERS.get()).limit -= 1;
        }
        drop(Box::from_raw(state));
    }
}
impl Services {
    pub(crate) fn shared(&self) -> &Shared {
        unsafe { &*self.shared }
    }
    fn industry(&self) -> &industry::Services {
        unsafe { &*self.industry }
    }
    /// The `Town::xy` or `Industry::location.tile` of a source, as `CheckSubsidyDistance`
    /// reads it.
    fn source_tile(&self, source: Source) -> TileIndex {
        TileIndex(unsafe {
            if source.kind == Source::TOWN {
                town::position_raw((self.shared().town_owner_by_id)(u32::from(source.id)))
            } else {
                industry::subsidy_position((self.industry_owner)((self.industry().get)(source.id)))
            }
        })
    }
    /// Sets a flag indicating that given town/industry is part of subsidised route.
    fn set_part_of_subsidy_flag(&self, source: Source, flag: PartOfSubsidy) {
        (self.set_source_flag)(source, flag);
    }
    /// The `DistanceManhattan(tile_src, tile_dst) <= SUBSIDY_MAX_DISTANCE` test.
    fn within_subsidy_distance(&self, tile_src: TileIndex, tile_dst: TileIndex) -> bool {
        let map = self.shared().map();
        map.distance_manhattan(tile_src.0, tile_dst.0) <= SUBSIDY_MAX_DISTANCE
    }
    /// Checks if the source and destination of a subsidy are inside the distance limit.
    fn check_subsidy_distance(&self, src: Source, dst: Source) -> bool {
        self.within_subsidy_distance(self.source_tile(src), self.source_tile(dst))
    }
    /// Perform a full rebuild of the subsidies cache.
    fn rebuild_subsidised_source_and_destination_cache(&self) {
        let mut id = (self.shared().town_next_id)(0);
        while id != u32::MAX {
            (self.clear_source_flags)(Source::town(id as u16));
            id = (self.shared().town_next_id)(id + 1);
        }
        let mut id = (self.next_industry)(0);
        while id != u32::MAX {
            (self.clear_source_flags)(Source::industry(id as u16));
            id = (self.next_industry)(id + 1);
        }
        each(|_, state| {
            let (src, dst) = unsafe { ((*state).src, (*state).dst) };
            self.set_part_of_subsidy_flag(src, PartOfSubsidy::Source);
            self.set_part_of_subsidy_flag(dst, PartOfSubsidy::Destination);
        });
    }
    /// Check whether a specific subsidy already exists.
    ///
    /// The original returns at the first match; the remaining visits only read.
    fn check_subsidy_duplicate(cargo: CargoType, src: Source, dst: Source) -> bool {
        let mut found = false;
        each(|_, state| {
            found |= unsafe {
                (*state).cargo_type == cargo && (*state).src == src && (*state).dst == dst
            };
        });
        found
    }
    /// Creates a subsidy with the given parameters.
    fn create_subsidy(&self, cargo_type: CargoType, src: Source, dst: Source) {
        let id = SubsidyID((self.allocate)(
            cargo_type.0,
            src,
            dst,
            SUBSIDY_OFFER_MONTHS,
        ));
        (self.offered_news)(id.0);
        self.set_part_of_subsidy_flag(src, PartOfSubsidy::Source);
        self.set_part_of_subsidy_flag(dst, PartOfSubsidy::Destination);
        (self.offer_events)(id.0);
        (self.dirty)();
        crate::witness::hit(crate::witness::SUBSIDY_CREATE);
    }
    /// Marks subsidy as awarded, creates news and AI event
    fn award_to(&self, id: SubsidyID, company: CompanyID) {
        let state = owner(id);
        // This private callee is reached after the original IsAwarded test.
        unsafe {
            (*state).awarded = company;
        }
        let remaining = u32::from((self.duration)()) * MONTHS_IN_YEAR;
        unsafe {
            (*state).remaining = remaining as u16;
        }
        (self.awarded_news)(id.0);
        (self.award_events)(id.0);
        (self.dirty)();
        crate::witness::hit(crate::witness::SUBSIDY_AWARD);
    }
    /// Tries to create a passenger subsidy between two towns.
    fn find_subsidy_passenger_route(&self) -> bool {
        if !(self.can_allocate)() {
            return false;
        }
        // Pick a random TPE_PASSENGER type
        let r = self
            .shared()
            .random_range(
                crate::cargo_registry::town_count(TownProductionEffect::TPE_PASSENGERS.0) as u32,
            );
        let cargo_type = CargoType(crate::cargo_registry::town_cargo(
            TownProductionEffect::TPE_PASSENGERS.0,
            r as usize,
        ));
        let src = town::spatial::get_random_town(unsafe { &*self.shared });
        if unsafe {
            town::population(src.state) < SUBSIDY_PAX_MIN_POPULATION
                || town::subsidy_percent_transported(src.state, cargo_type.0)
                    > SUBSIDY_MAX_PCT_TRANSPORTED
        } {
            return false;
        }
        let dst = town::spatial::get_random_town(unsafe { &*self.shared });
        if unsafe { town::population(dst.state) < SUBSIDY_PAX_MIN_POPULATION }
            || src.shell == dst.shell
        {
            return false;
        }
        let a = Source::town(src.id.0);
        let b = Source::town(dst.id.0);
        let from = TileIndex(unsafe { town::position_raw(src.state) });
        let to = TileIndex(unsafe { town::position_raw(dst.state) });
        if !self.within_subsidy_distance(from, to)
            || Self::check_subsidy_duplicate(cargo_type, a, b)
        {
            return false;
        }
        self.create_subsidy(cargo_type, a, b);
        true
    }
    /// Adds the produced (`AddProducedCargo`) or accepted (`AddAcceptedCargo`) cargo of the
    /// houses in `TileArea(xy, 1, 1).Expand(SUBSIDY_TOWN_CARGO_RADIUS)` to a fresh
    /// `CargoArray`.
    fn scan(&self, scratch: Cargo, owner: *const town::State, accepted: bool) {
        // SAFETY: Native outer frame owns one actual CargoArray and exposes its
        // own data() address. Only immediate scalar accesses cross that view.
        for c in 0..usize::from(NUM_CARGO) {
            unsafe {
                scratch.data.add(c).write(0);
            }
        }
        let shared = self.shared();
        let xy = unsafe { town::position_raw(owner) };
        let map = shared.map();
        let mut area = crate::tilearea::TileArea {
            tile: xy,
            w: 1,
            h: 1,
        };
        area.expand(SUBSIDY_TOWN_CARGO_RADIUS, map, (shared.map_max_y)() + 1);
        for tile in area.tiles(map) {
            if TileType((shared.tile_type)(tile)) != TileType::MP_HOUSE {
                continue;
            }
            unsafe {
                if accepted {
                    let house = &*(*self.town).houses;
                    let entry = (house.entry)(tile);
                    let mut always = 0;
                    town::house::accepted(
                        self.town,
                        self.shared,
                        tile,
                        entry.house,
                        entry.town.shell,
                        scratch.data,
                        &raw mut always,
                    );
                } else {
                    town::house::openttd_rust_house_produced(
                        self.town,
                        self.shared,
                        tile,
                        scratch.data,
                    );
                }
            }
        }
    }

    /// Tries to create a cargo subsidy with a town as source.
    fn find_subsidy_town_cargo_route(&self, scratch: Cargo) -> bool {
        if !(self.can_allocate)() {
            return false;
        }
        // Select a random town.
        let src = town::spatial::get_random_town(unsafe { &*self.shared });
        if unsafe { town::population(src.state) < SUBSIDY_CARGO_MIN_POPULATION } {
            return false;
        }
        // Calculate the produced cargo of houses around town center.
        self.scan(scratch, src.state, false);
        // Passenger subsidies are not handled here.
        for index in 0..crate::cargo_registry::town_count(TownProductionEffect::TPE_PASSENGERS.0) {
            let cargo = CargoType(crate::cargo_registry::town_cargo(
                TownProductionEffect::TPE_PASSENGERS.0,
                index,
            ));
            unsafe {
                scratch.data.add(usize::from(cargo.0)).write(0);
            }
        }
        let count = unsafe { crate::cargo_type::cargo_array_count(scratch.data) };
        // No cargo produced at all?
        if count == 0 {
            return false;
        }
        // Choose a random cargo that is produced in the town.
        let mut cargo_number = self.shared().random_range(count);
        let mut cargo_type = CargoType(0);
        while cargo_type < CargoType::NUM_CARGO {
            if unsafe { scratch.data.add(usize::from(cargo_type.0)).read() > 0 } {
                if cargo_number == 0 {
                    break;
                }
                cargo_number -= 1;
            }
            cargo_type.0 += 1;
        }
        // Avoid using invalid NewGRF cargoes.
        if !(self.cargo_valid)(cargo_type.0) || !(self.manual)(cargo_type.0) {
            return false;
        }
        // Quit if the percentage transported is large enough.
        if unsafe {
            town::subsidy_percent_transported(src.state, cargo_type.0) > SUBSIDY_MAX_PCT_TRANSPORTED
        } {
            return false;
        }
        self.find_subsidy_cargo_destination(scratch, cargo_type, Source::town(src.id.0))
    }
    /// Tries to create a cargo subsidy with an industry as source.
    fn find_subsidy_industry_cargo_route(&self, scratch: Cargo) -> bool {
        if !(self.can_allocate)() {
            return false;
        }
        // Select a random industry.
        let shell = industry::random_industry(self.industry());
        if shell.is_null() {
            return false;
        }
        let state = (self.industry_owner)(shell);
        // Randomize cargo type
        let num_cargos = unsafe { industry::subsidy_produced_count(state) };
        if num_cargos == 0 {
            return false; // industry produces nothing
        }
        let cargo_num = self.shared().random_range(num_cargos);
        let (cargo_type, total, trans) = unsafe { industry::subsidy_produced(state, cargo_num) };
        let cargo_type = CargoType(cargo_type);
        // Quit if no production in this industry
        // or if the pct transported is already large enough
        // or if the cargo is automatically distributed
        if total == 0
            || trans > SUBSIDY_MAX_PCT_TRANSPORTED
            || !is_valid_cargo_type(cargo_type.0)
            || !(self.manual)(cargo_type.0)
        {
            return false;
        }
        self.find_subsidy_cargo_destination(
            scratch,
            cargo_type,
            Source::industry((self.industry_id)(shell)),
        )
    }
    /// Tries to find a suitable destination for the given source and cargo.
    fn find_subsidy_cargo_destination(
        &self,
        scratch: Cargo,
        cargo_type: CargoType,
        src: Source,
    ) -> bool {
        // Choose a random destination.
        let dst = if crate::random_func::chance16_i(1, 2, self.shared().random()) {
            // Select a random town.
            let dst_town = town::spatial::get_random_town(unsafe { &*self.shared });
            // Calculate cargo acceptance of houses around town center.
            self.scan(scratch, dst_town.state, true);
            // Check if the town can accept this cargo.
            if unsafe { scratch.data.add(usize::from(cargo_type.0)).read() < 8 } {
                return false;
            }
            Source::town(dst_town.id.0)
        } else {
            // Select a random industry.
            let dst_ind = industry::random_industry(self.industry());
            if dst_ind.is_null() {
                return false;
            }
            // The industry must accept the cargo
            if !unsafe { industry::subsidy_accepts((self.industry_owner)(dst_ind), cargo_type.0) } {
                return false;
            }
            Source::industry((self.industry_id)(dst_ind))
        };
        // Check that the source and the destination are not the same.
        if src == dst {
            return false;
        }
        // Check distance between source and destination.
        if !self.check_subsidy_distance(src, dst) {
            return false;
        }
        // Avoid duplicate subsidies.
        if Self::check_subsidy_duplicate(cargo_type, src, dst) {
            return false;
        }
        self.create_subsidy(cargo_type, src, dst);
        true
    }
}

impl Services {
    /// Perform the economy monthly update of open subsidies, and try to create a new one.
    fn monthly(&self, scratch: Cargo) {
        let mut modified = false;
        each(|id, state| {
            let remaining = unsafe { (*state).remaining.wrapping_sub(1) };
            unsafe {
                (*state).remaining = remaining;
            }
            if remaining != 0 {
                return;
            }
            if !unsafe { (*state).is_awarded() } {
                (self.offer_expired_news)(id.0);
                (self.offer_expired_events)(id.0);
            } else {
                if unsafe { (*state).awarded } == CompanyID((self.local_company)()) {
                    (self.withdrawn_news)(id.0);
                }
                (self.expired_events)(id.0);
            }
            (self.destroy)(id.0);
            modified = true;
            crate::witness::hit(crate::witness::SUBSIDY_EXPIRE);
        });
        if modified {
            self.rebuild_subsidised_source_and_destination_cache();
        } else if (self.duration)() == 0 {
            // If subsidy duration is set to 0, subsidies are disabled, so bail out.
            return;
        } else if !(self.any_manual)() {
            // Return early if there are no manually distributed cargoes and if we
            // don't need to invalidate the subsidies window.
            return;
        }
        let random_chance = self.shared().random_range(16);
        // The original's `do { ... } while (!found && n--)` with `n = 1000` makes up to
        // 1001 attempts.
        if (random_chance < 2 && (self.manual_pax)()) || random_chance == 2 || random_chance == 3 {
            for _ in 0..=1000 {
                let found = if random_chance < 2 {
                    // There is a 1/8 chance each month of generating a passenger subsidy.
                    self.find_subsidy_passenger_route()
                } else if random_chance == 2 {
                    // Cargo subsidies with a town as a source have a 1/16 chance.
                    self.find_subsidy_town_cargo_route(scratch)
                } else {
                    // Cargo subsidies with an industry as a source have a 1/16 chance.
                    self.find_subsidy_industry_cargo_route(scratch)
                };
                if found {
                    modified = true;
                    break;
                }
            }
        }
        if modified {
            (self.dirty)();
        }
    }
    /// Original `DeleteSubsidyWith`: delete the subsidies associated with a given cargo source
    /// type and id.
    pub(crate) fn delete_with(&self, source: Source) {
        let mut dirty = false;
        each(|id, state| {
            if unsafe { (*state).src == source || (*state).dst == source } {
                (self.destroy)(id.0);
                dirty = true;
            }
        });
        if dirty {
            (self.dirty)();
            self.rebuild_subsidised_source_and_destination_cache();
        }
    }
    /// The subsidy part of `ChangeOwnershipOfCompanyItems` (`economy.cpp`).
    pub(crate) fn transfer(&self, old_owner: Owner, new_owner: Owner) {
        each(|id, state| {
            if unsafe { (*state).awarded } != old_owner {
                return;
            }
            if new_owner == Owner::INVALID {
                (self.destroy)(id.0);
            } else {
                unsafe {
                    (*state).awarded = new_owner;
                }
            }
        });
        if new_owner == Owner::INVALID {
            self.rebuild_subsidised_source_and_destination_cache();
        }
    }
    /// Tests whether given delivery is subsidised and possibly awards the subsidy to delivering
    /// company
    pub(crate) fn check_subsidised(
        &self,
        cargo_type: CargoType,
        company: CompanyID,
        src: Source,
        station: StationID,
    ) -> bool {
        // If the source isn't subsidised, don't continue
        if !src.is_valid()
            || src.kind > Source::TOWN
            || !(self.has_source_flag)(src, PartOfSubsidy::Source)
        {
            return false;
        }
        // The original lazily creates a vector only after the first applicable
        // town destination. A fixed membership set preserves first-discovery
        // semantics without allocating on the cargo-delivery hot path.
        let catchment = crate::station::Ctx::from_shared(self.shared());
        let station_state = (catchment.c.station_owner)(station);
        // One bit per `TownID` value.
        let mut towns_near = [0_u64; (u16::MAX as usize + 1) / u64::BITS as usize];
        let town_word = |town: TownID| usize::from(town.0) / u64::BITS as usize;
        let town_bit = |town: TownID| 1_u64 << (u32::from(town.0) % u64::BITS);
        // Remember all towns near this station (at least one house in its catchment radius)
        // which are destination of subsidised path. Do that only if needed
        // SAFETY: The live station owner; immediate read.
        if !unsafe { crate::station::rect_is_empty(station_state) } {
            // Don't create the cache if there is no applicable subsidy with town as destination
            let mut needed = false;
            each(|_, state| {
                needed |= unsafe {
                    (*state).dst.kind == Source::TOWN
                        && (*state).cargo_type == cargo_type
                        && (*state).src == src
                        && (!(*state).is_awarded() || (*state).awarded == company)
                };
            });
            if needed {
                let shared = self.shared();
                // SAFETY: The live station owner; the visits read only the map and caches.
                unsafe {
                    crate::station::for_each_catchment_tile(station_state, &catchment, |tile| {
                        if TileType((shared.tile_type)(tile)) != TileType::MP_HOUSE {
                            return;
                        }
                        let town = TownID((shared.town_index)(tile));
                        if (self.has_source_flag)(Source::town(town.0), PartOfSubsidy::Destination)
                        {
                            towns_near[town_word(town)] |= town_bit(town);
                        }
                    });
                }
            }
        }
        let mut subsidised = false;
        // Check if there's a (new) subsidy that applies. There can be more subsidies triggered by this delivery!
        // Think about the case that subsidies are A->B and A->C and station has both B and C in its catchment area
        each(|id, state| {
            let (applies, dst) = unsafe {
                (
                    (*state).cargo_type == cargo_type
                        && (*state).src == src
                        && (!(*state).is_awarded() || (*state).awarded == company),
                    (*state).dst,
                )
            };
            if !applies {
                return;
            }
            if dst.kind == Source::INDUSTRY {
                // SAFETY: The live station owner; each entry is copied before the award.
                unsafe {
                    crate::station::for_each_industry_near(station_state, |near| {
                        if near.id == IndustryID(dst.id) {
                            (self.assert_destination)(dst);
                            subsidised = true;
                            if !(*state).is_awarded() {
                                self.award_to(id, company);
                            }
                        }
                        true
                    });
                }
            } else if towns_near[town_word(TownID(dst.id))] & town_bit(TownID(dst.id)) != 0 {
                (self.assert_destination)(dst);
                subsidised = true;
                if unsafe { !(*state).is_awarded() } {
                    self.award_to(id, company);
                }
            }
        });
        subsidised
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_rebuild(s: *const Services) {
    unsafe { &*s }.rebuild_subsidised_source_and_destination_cache();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_delete_with(s: *const Services, source: Source) {
    unsafe { &*s }.delete_with(source);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_command_abi(
    s: *const Services,
    flags: u16,
    cargo: u8,
    src: Source,
    dst: Source,
) -> u8 {
    let s = unsafe { &*s };
    let cargo_type = CargoType(cargo);
    let flags = DoCommandFlags(flags);
    if !(s.can_allocate)()
        || CompanyID((s.shared().current_company)()) != Owner::DEITY
        || cargo_type >= CargoType::NUM_CARGO
        || !(s.cargo_valid)(cargo_type.0)
        || !(s.valid_source)(src)
        || !(s.valid_source)(dst)
    {
        return u8::from(false);
    }
    if flags.test(DoCommandFlag::Execute) {
        s.create_subsidy(cargo_type, src, dst);
    }
    u8::from(true)
}
pub(crate) unsafe fn openttd_rust_subsidy_monthly(s: *const Services, scratch: Cargo) {
    unsafe { &*s }.monthly(scratch);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_check_abi(
    s: *const Services,
    cargo: u8,
    company: u8,
    src: Source,
    station: u16,
) -> u8 {
    u8::from(unsafe { &*s }.check_subsidised(
        CargoType(cargo),
        CompanyID(company),
        src,
        StationID(station),
    ))
}

crate::abi::layouts! {
    fn abi_layout;
    "Source" ["probe"] => crate::subsidy::Source { id, kind => type, };
    "OpenTTDSubsidyState" ["probe"] => State { cargo_type, remaining, awarded, src, dst, };
    "OpenTTDSubsidyCargo" ["probe"] => Cargo { data, };
    "OpenTTDSubsidyServices" ["probe"] => Services {
        shared,
        town,
        industry,
        industry_owner,
        industry_id,
        next_industry,
        has_source_flag,
        set_source_flag,
        clear_source_flags,
        valid_source,
        can_allocate,
        allocate,
        destroy,
        duration,
        local_company,
        cargo_valid,
        manual,
        manual_pax,
        any_manual,
        offered_news,
        awarded_news,
        offer_expired_news,
        withdrawn_news,
        offer_events,
        award_events,
        offer_expired_events,
        expired_events,
        dirty,
        station,
        assert_destination,
    };
}
