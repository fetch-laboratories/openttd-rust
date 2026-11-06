/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Subsidy records and canonical pool policy, route searches, expiry and delivery awards.
//! Native shells retain construction/destruction and script events enqueue without
//! entering a VM. Field access ends before effects; source arithmetic keeps its narrowing.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::comparison_chain,
    clippy::if_not_else
)]
use crate::pool::{PoolCore, TypedPool};
pub use crate::source_type::Source;
use crate::station::Station;
use crate::subsidy_type::{PartOfSubsidy, SubsidyHandle, SubsidyId};
use crate::{
    cargo_type::{INVALID_CARGO, NUM_CARGO, is_valid_cargo_type},
    subsidy_base::{
        SUBSIDY_CARGO_MIN_POPULATION, SUBSIDY_MAX_DISTANCE, SUBSIDY_MAX_PCT_TRANSPORTED,
        SUBSIDY_OFFER_MONTHS, SUBSIDY_PAX_MIN_POPULATION, SUBSIDY_TOWN_CARGO_RADIUS,
    },
    timer_game_common::MONTHS_IN_YEAR,
};
use crate::{industry, services::Services as Shared, town};
use std::ffi::c_void;
crate::abi::shared_records! {
    fn abi_record;
    /// The fields of one native `Subsidy` shell, read and written in place by
    /// both languages. Widths and defaults are the original members'.
    "OpenTTDSubsidyRecord" ["subsidy_record"] => pub struct SubsidyRecord {
        pub cargo_type: u8 => "CargoType",
        pub remaining: u16,
        pub awarded: u8 => "CompanyID",
        pub src: Source => "Source",
        pub dst: Source => "Source",
    }
}
/// The subsidy owner is its record.
pub type State = SubsidyRecord;
impl State {
    fn is_awarded(&self) -> bool {
        self.awarded != u8::MAX
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
    /// Address of the native facade's canonical owner pointer, initialized at startup.
    pub pool: *const *mut PoolCore,
    pub industry_owner: extern "C" fn(*mut c_void) -> *mut crate::industry::Industry,
    pub industry_id: extern "C" fn(*mut c_void) -> u16,
    pub next_industry: extern "C" fn(u32) -> u32,
    pub valid_source: extern "C" fn(Source) -> bool,
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
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_subsidy_new() -> *mut State {
    Box::into_raw(Box::new(State {
        cargo_type: INVALID_CARGO,
        remaining: 0,
        awarded: 255,
        src: Source::default(),
        dst: Source::default(),
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_destroy(state: *mut State) {
    // SAFETY: Native owner retires the slot before dropping its unique record.
    unsafe { drop(Box::from_raw(state)) };
}
impl Services {
    fn core(&self) -> *mut PoolCore {
        // SAFETY: The native facade's owner is initialized before game entries.
        unsafe { *self.pool }
    }
    fn pool(&self) -> TypedPool<SubsidyId, State, SubsidyHandle> {
        // SAFETY: The registered pool outlives entries, including allocation and cleanup.
        unsafe { TypedPool::from_descriptor(PoolCore::descriptor(self.core())) }
    }
    fn owner(&self, id: u16) -> *mut State {
        unsafe { self.pool().record(SubsidyId(id)) }
    }
    fn each(&self, mut visit: impl FnMut(u16, *mut State)) {
        let descriptor = unsafe { PoolCore::descriptor(self.core()) };
        let pool = self.pool();
        let mut id = 0;
        // Reread the live extent and slot after effects that may allocate or delete.
        while id < unsafe { (*descriptor).first_unused } {
            let state = unsafe { pool.record(SubsidyId(id as u16)) };
            if !state.is_null() {
                visit(id as u16, state);
            }
            id += 1;
        }
    }
    fn can_allocate(&self) -> bool {
        unsafe { PoolCore::can_allocate(self.core(), 1) }
    }

    pub(crate) fn shared(&self) -> &Shared {
        unsafe { &*self.shared }
    }
    fn industry(&self) -> &industry::Services {
        unsafe { &*self.industry }
    }
    fn position(&self, source: Source) -> u32 {
        unsafe {
            if source.kind == Source::TOWN {
                town::position((self.shared().town_owner_by_id)(u32::from(source.id)))
            } else {
                industry::subsidy_position((self.industry_owner)((self.industry().get)(source.id)))
            }
        }
    }
    /// Live source at the original cache observation point. Resolving the owner
    /// ends before its field is borrowed; callers retain no view across effects.
    fn source_flags(&self, source: Source) -> *mut u8 {
        match source.kind {
            Source::TOWN => {
                let owner = (self.shared().town_owner_by_id)(u32::from(source.id));
                unsafe { &raw mut (*owner).record.part_of_subsidy }
            }
            Source::INDUSTRY => {
                let shell = unsafe { (self.industry().get)(source.id) };
                let owner = (self.industry_owner)(shell);
                unsafe { &raw mut (*owner).record.part_of_subsidy }
            }
            _ => unreachable!("subsidy source type"),
        }
    }
    fn has_flag(&self, source: Source, flag: PartOfSubsidy) -> bool {
        unsafe { *self.source_flags(source) & (1 << flag as u8) != 0 }
    }
    fn flag(&self, source: Source, flag: PartOfSubsidy) {
        unsafe { *self.source_flags(source) |= 1 << flag as u8 };
    }
    fn assert_destination(&self, source: Source) {
        crate::movement::native_assert(
            self.shared(),
            self.has_flag(source, PartOfSubsidy::Destination),
        );
    }
    fn distance_tiles(&self, a: u32, b: u32) -> bool {
        let map = self.shared().map();
        map.distance_manhattan(a, b) <= SUBSIDY_MAX_DISTANCE
    }
    fn distance(&self, a: Source, b: Source) -> bool {
        self.distance_tiles(self.position(a), self.position(b))
    }
    fn rebuild(&self) {
        let mut id = (self.shared().town_next_id)(0);
        while id != u32::MAX {
            unsafe { *self.source_flags(Source::town(id as u16)) = 0 };
            id = (self.shared().town_next_id)(id + 1);
        }
        let mut id = (self.next_industry)(0);
        while id != u32::MAX {
            unsafe { *self.source_flags(Source::industry(id as u16)) = 0 };
            id = (self.next_industry)(id + 1);
        }
        self.each(|_, state| {
            let (src, dst) = unsafe { ((*state).src, (*state).dst) };
            self.flag(src, PartOfSubsidy::Source);
            self.flag(dst, PartOfSubsidy::Destination);
        });
    }
    fn duplicate(&self, cargo: u8, src: Source, dst: Source) -> bool {
        let mut found = false;
        self.each(|_, state| {
            found |= unsafe {
                (*state).cargo_type == cargo && (*state).src == src && (*state).dst == dst
            };
        });
        found
    }
    fn create(&self, cargo: u8, src: Source, dst: Source) {
        let id = (self.allocate)(cargo, src, dst, SUBSIDY_OFFER_MONTHS);
        (self.offered_news)(id);
        self.flag(src, PartOfSubsidy::Source);
        self.flag(dst, PartOfSubsidy::Destination);
        (self.offer_events)(id);
        (self.dirty)();
        crate::witness::hit(crate::witness::SUBSIDY_CREATE);
    }
    fn award(&self, id: u16, company: u8) {
        let state = self.owner(id);
        // This private callee is reached after the original IsAwarded test.
        unsafe {
            (*state).awarded = company;
        }
        let remaining = u32::from((self.duration)()) * MONTHS_IN_YEAR;
        unsafe {
            (*state).remaining = remaining as u16;
        }
        (self.awarded_news)(id);
        (self.award_events)(id);
        (self.dirty)();
        crate::witness::hit(crate::witness::SUBSIDY_AWARD);
    }
    fn passenger(&self) -> bool {
        if !self.can_allocate() {
            return false;
        }
        let n = self
            .shared()
            .random_range(
                crate::cargo_registry::town_count(crate::cargotype::TPE_PASSENGERS) as u32,
            );
        let cargo = crate::cargo_registry::town_cargo(crate::cargotype::TPE_PASSENGERS, n as usize);
        let src = town::spatial::random(unsafe { &*self.shared });
        if unsafe {
            town::population(src.state) < SUBSIDY_PAX_MIN_POPULATION
                || town::subsidy_percent_transported(src.state, cargo) > SUBSIDY_MAX_PCT_TRANSPORTED
        } {
            return false;
        }
        let dst = town::spatial::random(unsafe { &*self.shared });
        if unsafe { town::population(dst.state) < SUBSIDY_PAX_MIN_POPULATION }
            || src.shell == dst.shell
        {
            return false;
        }
        let a = Source::town(src.id.0);
        let b = Source::town(dst.id.0);
        let from = unsafe { town::position(src.state) };
        let to = unsafe { town::position(dst.state) };
        if !self.distance_tiles(from, to) || self.duplicate(cargo, a, b) {
            return false;
        }
        self.create(cargo, a, b);
        true
    }
    fn scan(&self, scratch: Cargo, owner: *const town::State, accepted: bool) {
        // SAFETY: Native outer frame owns one actual CargoArray and exposes its
        // own data() address. Only immediate scalar accesses cross that view.
        for c in 0..64 {
            unsafe {
                scratch.data.add(c).write(0);
            }
        }
        let shared = self.shared();
        let xy = unsafe { town::position(owner) };
        let map = shared.map();
        let mut area = crate::tilearea::TileArea {
            tile: xy,
            w: 1,
            h: 1,
        };
        area.expand(SUBSIDY_TOWN_CARGO_RADIUS, map, (shared.map_max_y)() + 1);
        for tile in area.tiles(map) {
            if (shared.tile_type)(tile) != crate::tile_type::MP_HOUSE {
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

    fn town_route(&self, scratch: Cargo) -> bool {
        if !self.can_allocate() {
            return false;
        }
        let src = town::spatial::random(unsafe { &*self.shared });
        if unsafe { town::population(src.state) < SUBSIDY_CARGO_MIN_POPULATION } {
            return false;
        }
        self.scan(scratch, src.state, false);
        for index in 0..crate::cargo_registry::town_count(crate::cargotype::TPE_PASSENGERS) as u32 {
            let cargo =
                crate::cargo_registry::town_cargo(crate::cargotype::TPE_PASSENGERS, index as usize);
            unsafe {
                scratch.data.add(usize::from(cargo)).write(0);
            }
        }
        let count = unsafe { crate::cargo_type::cargo_array_count(scratch.data) };
        if count == 0 {
            return false;
        }
        let mut number = self.shared().random_range(count);
        let mut cargo = 0_u8;
        while cargo < NUM_CARGO {
            if unsafe { scratch.data.add(usize::from(cargo)).read() > 0 } {
                if number == 0 {
                    break;
                }
                number -= 1;
            }
            cargo += 1;
        }
        if !(self.cargo_valid)(cargo) || !(self.manual)(cargo) {
            return false;
        }
        if unsafe {
            town::subsidy_percent_transported(src.state, cargo) > SUBSIDY_MAX_PCT_TRANSPORTED
        } {
            return false;
        }
        self.destination(scratch, cargo, Source::town(src.id.0))
    }
    fn industry_route(&self, scratch: Cargo) -> bool {
        if !self.can_allocate() {
            return false;
        }
        let shell = industry::random_industry(self.industry());
        if shell.is_null() {
            return false;
        }
        let state = (self.industry_owner)(shell);
        let count = unsafe { industry::subsidy_produced_count(state) };
        if count == 0 {
            return false;
        }
        let number = self.shared().random_range(count);
        let (cargo, total, trans) = unsafe { industry::subsidy_produced(state, number) };
        if total == 0
            || trans > SUBSIDY_MAX_PCT_TRANSPORTED
            || !is_valid_cargo_type(cargo)
            || !(self.manual)(cargo)
        {
            return false;
        }
        self.destination(scratch, cargo, Source::industry((self.industry_id)(shell)))
    }
    fn destination(&self, scratch: Cargo, cargo: u8, src: Source) -> bool {
        let dst = if crate::random_func::chance16_i(1, 2, self.shared().random()) {
            let town = town::spatial::random(unsafe { &*self.shared });
            self.scan(scratch, town.state, true);
            if unsafe { scratch.data.add(usize::from(cargo)).read() < 8 } {
                return false;
            }
            Source::town(town.id.0)
        } else {
            let shell = industry::random_industry(self.industry());
            if shell.is_null() {
                return false;
            }
            if !unsafe { industry::subsidy_accepts((self.industry_owner)(shell), cargo) } {
                return false;
            }
            Source::industry((self.industry_id)(shell))
        };
        if src == dst || !self.distance(src, dst) || self.duplicate(cargo, src, dst) {
            return false;
        }
        self.create(cargo, src, dst);
        true
    }
}

impl Services {
    fn monthly(&self, scratch: Cargo) {
        let mut modified = false;
        self.each(|id, state| {
            let remaining = unsafe { (*state).remaining.wrapping_sub(1) };
            unsafe {
                (*state).remaining = remaining;
            }
            if remaining != 0 {
                return;
            }
            if !unsafe { (*state).is_awarded() } {
                (self.offer_expired_news)(id);
                (self.offer_expired_events)(id);
            } else {
                if unsafe { (*state).awarded } == (self.local_company)() {
                    (self.withdrawn_news)(id);
                }
                (self.expired_events)(id);
            }
            (self.destroy)(id);
            modified = true;
            crate::witness::hit(crate::witness::SUBSIDY_EXPIRE);
        });
        if modified {
            self.rebuild();
        } else if (self.duration)() == 0 || !(self.any_manual)() {
            return;
        }
        let chance = self.shared().random_range(16);
        if (chance < 2 && (self.manual_pax)()) || chance == 2 || chance == 3 {
            for _ in 0..=1000 {
                let found = if chance < 2 {
                    self.passenger()
                } else if chance == 2 {
                    self.town_route(scratch)
                } else {
                    self.industry_route(scratch)
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
    pub(crate) fn delete_with(&self, source: Source) {
        let mut dirty = false;
        self.each(|id, state| {
            if unsafe { (*state).src == source || (*state).dst == source } {
                (self.destroy)(id);
                dirty = true;
            }
        });
        if dirty {
            (self.dirty)();
            self.rebuild();
        }
    }
    pub(crate) fn transfer(&self, old: u8, new: u8) {
        self.each(|id, state| {
            if unsafe { (*state).awarded } != old {
                return;
            }
            if new == crate::company_type::INVALID_OWNER {
                (self.destroy)(id);
            } else {
                unsafe {
                    (*state).awarded = new;
                }
            }
        });
        if new == crate::company_type::INVALID_OWNER {
            self.rebuild();
        }
    }
    pub(crate) fn check(&self, cargo: u8, company: u8, src: Source, station: u16) -> bool {
        if !src.is_valid() || src.kind > Source::TOWN || !self.has_flag(src, PartOfSubsidy::Source)
        {
            return false;
        }
        // The original lazily creates a vector only after the first applicable
        // town destination. A fixed membership set preserves first-discovery
        // semantics without allocating on the cargo-delivery hot path.
        let catchment = crate::station::Ctx::from_shared(self.shared());
        let station_state = (catchment.c.station_owner)(crate::station_type::StationID(station));
        let mut towns = [0_u64; 1024];
        // SAFETY: The live station owner; immediate read.
        if !unsafe { crate::station::rect_is_empty(station_state) } {
            let mut needed = false;
            self.each(|_, state| {
                needed |= unsafe {
                    (*state).dst.kind == Source::TOWN
                        && (*state).cargo_type == cargo
                        && (*state).src == src
                        && (!(*state).is_awarded() || (*state).awarded == company)
                };
            });
            if needed {
                let shared = self.shared();
                // SAFETY: The live station owner; the visits read only the map and caches.
                unsafe {
                    crate::station::for_each_catchment_tile(station_state, &catchment, |tile| {
                        if (shared.tile_type)(tile) != crate::tile_type::MP_HOUSE {
                            return;
                        }
                        let town = (shared.town_index)(tile);
                        if self.has_flag(Source::town(town), PartOfSubsidy::Destination) {
                            towns[usize::from(town) / 64] |= 1 << (town % 64);
                        }
                    });
                }
            }
        }
        let mut subsidised = false;
        self.each(|id, state| {
            let (applies, dst) = unsafe {
                (
                    (*state).cargo_type == cargo
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
                        if near.id == crate::industry_type::IndustryID(dst.id) {
                            self.assert_destination(dst);
                            subsidised = true;
                            if !(*state).is_awarded() {
                                self.award(id, company);
                            }
                        }
                        true
                    });
                }
            } else if towns[usize::from(dst.id) / 64] & (1 << (dst.id % 64)) != 0 {
                self.assert_destination(dst);
                subsidised = true;
                if unsafe { !(*state).is_awarded() } {
                    self.award(id, company);
                }
            }
        });
        subsidised
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_rebuild(s: *const Services) {
    unsafe { &*s }.rebuild();
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
    if !s.can_allocate()
        || (s.shared().current_company)() != crate::company_type::OWNER_DEITY
        || cargo >= NUM_CARGO
        || !(s.cargo_valid)(cargo)
        || !(s.valid_source)(src)
        || !(s.valid_source)(dst)
    {
        return u8::from(false);
    }
    if flags & crate::command_type::DoCommandFlag::EXECUTE != 0 {
        s.create(cargo, src, dst);
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
    u8::from(unsafe { &*s }.check(cargo, company, src, station))
}

crate::abi::layouts! {
    fn abi_layout;
    "Source" ["probe"] => crate::subsidy::Source { id, kind => type, };
    "OpenTTDSubsidyCargo" ["probe"] => Cargo { data, };
    "OpenTTDSubsidyServices" ["probe"] => Services {
        shared,
        town,
        industry,
        pool,
        industry_owner,
        industry_id,
        next_industry,
        valid_source,
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
    };
}
