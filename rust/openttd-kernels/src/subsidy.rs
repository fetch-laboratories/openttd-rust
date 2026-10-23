/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical subsidy owners, route searches, expiry and delivery awards.
//! Native pool shells retain identity and allocation order. The game-state mutex
//! serializes process-wide registration; no owner reference spans any callback.
//! Native leaves are noexcept and script events enqueue without entering a VM.
//! Panics abort. Source arithmetic explicitly retains narrowing and wrapping.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::comparison_chain,
    clippy::if_not_else,
    clippy::too_many_lines
)]
pub use crate::source_type::Source;
use crate::subsidy_type::PartOfSubsidy;
use crate::{
    cargo_type::{INVALID_CARGO, NUM_CARGO, is_valid_cargo_type},
    subsidy_base::{
        SUBSIDY_CARGO_MIN_POPULATION, SUBSIDY_MAX_DISTANCE, SUBSIDY_MAX_PCT_TRANSPORTED,
        SUBSIDY_OFFER_MONTHS, SUBSIDY_PAX_MIN_POPULATION, SUBSIDY_TOWN_CARGO_RADIUS,
    },
    timer_game_common::MONTHS_IN_YEAR,
};
use crate::{industry, services::Services as Shared, town};
use std::{cell::UnsafeCell, ffi::c_void, ptr};
#[repr(C)]
pub struct State {
    pub cargo_type: u8,
    pub remaining: u16,
    pub awarded: u8,
    pub src: Source,
    pub dst: Source,
}
impl State {
    fn is_awarded(&self) -> bool {
        self.awarded != u8::MAX
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cargo {
    pub object: *mut c_void,
    pub data: *mut u32,
}
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub town: *const crate::town::Leaves,
    pub industry: *const crate::industry::Services,
    pub random_town: extern "C" fn() -> crate::town::TownRef,
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
    pub passenger_count: extern "C" fn() -> u32,
    pub passenger_at: extern "C" fn(u32) -> u8,
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
    pub station_empty: extern "C" fn(*const Station) -> bool,
    pub catchment: extern "C" fn(*const Station, *mut c_void, extern "C" fn(*mut c_void, u32)),
    pub nearby_industry: extern "C" fn(*const Station, u32) -> *mut industry::ShellHandle,
    pub assert_destination: extern "C" fn(Source),
}
struct RegistryState {
    slots: [*mut State; 256],
    limit: u16,
}
struct Registry(UnsafeCell<RegistryState>);
// SAFETY: Native game-state mutex serializes registration, iteration and aliases.
unsafe impl Sync for Registry {}
static OWNERS: Registry = Registry(UnsafeCell::new(RegistryState {
    slots: [ptr::null_mut(); 256],
    limit: 0,
}));
fn owner(id: u16) -> *mut State {
    // SAFETY: Serialized native pool index; no registry borrow crosses callbacks.
    unsafe { (*OWNERS.0.get()).slots[usize::from(id)] }
}
fn each(mut visit: impl FnMut(u16, *mut State)) {
    let mut id = 0;
    while id < unsafe { (*OWNERS.0.get()).limit } {
        let state = owner(id);
        if !state.is_null() {
            visit(id, state);
        }
        id += 1;
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_subsidy_new(id: u16) -> *mut State {
    let state = Box::into_raw(Box::new(State {
        cargo_type: INVALID_CARGO,
        remaining: 0,
        awarded: 255,
        src: Source::default(),
        dst: Source::default(),
    }));
    // SAFETY: The constructing native pool shell owns this stable allocation.
    unsafe {
        (*OWNERS.0.get()).slots[usize::from(id)] = state;
        (*OWNERS.0.get()).limit = (*OWNERS.0.get()).limit.max(id + 1);
    }
    state
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_destroy(id: u16, state: *mut State) {
    // SAFETY: Native pool destroys exactly once, after every scalar alias ends.
    unsafe {
        (*OWNERS.0.get()).slots[usize::from(id)] = ptr::null_mut();
        while (*OWNERS.0.get()).limit > 0
            && (*OWNERS.0.get()).slots[usize::from((*OWNERS.0.get()).limit - 1)].is_null()
        {
            (*OWNERS.0.get()).limit -= 1;
        }
        drop(Box::from_raw(state));
    }
}
impl Services {
    fn shared(&self) -> &Shared {
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
    fn flag(&self, source: Source, flag: PartOfSubsidy) {
        (self.set_source_flag)(source, flag);
    }
    fn distance_tiles(&self, a: u32, b: u32) -> bool {
        let map = crate::map_func::Map {
            log_x: (self.shared().map_log_x)() as u8,
        };
        map.distance_manhattan(a, b) <= SUBSIDY_MAX_DISTANCE
    }
    fn distance(&self, a: Source, b: Source) -> bool {
        self.distance_tiles(self.position(a), self.position(b))
    }
    fn rebuild(&self) {
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
            self.flag(src, PartOfSubsidy::Source);
            self.flag(dst, PartOfSubsidy::Destination);
        });
    }
    fn duplicate(cargo: u8, src: Source, dst: Source) -> bool {
        let mut found = false;
        each(|_, state| {
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
        let state = owner(id);
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
        if !(self.can_allocate)() {
            return false;
        }
        let n = self.shared().random_range((self.passenger_count)());
        let cargo = (self.passenger_at)(n);
        let src = (self.random_town)();
        if unsafe {
            town::population(src.state) < SUBSIDY_PAX_MIN_POPULATION
                || town::subsidy_percent_transported(src.state, cargo) > SUBSIDY_MAX_PCT_TRANSPORTED
        } {
            return false;
        }
        let dst = (self.random_town)();
        if unsafe { town::population(dst.state) < SUBSIDY_PAX_MIN_POPULATION }
            || src.shell == dst.shell
        {
            return false;
        }
        let a = Source::town(src.id.0);
        let b = Source::town(dst.id.0);
        let from = unsafe { town::position(src.state) };
        let to = unsafe { town::position(dst.state) };
        if !self.distance_tiles(from, to) || Self::duplicate(cargo, a, b) {
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
        let map = crate::map_func::Map {
            log_x: (shared.map_log_x)() as u8,
        };
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
                    town::house::openttd_rust_house_accepted(
                        self.town,
                        self.shared,
                        tile,
                        entry.house,
                        entry.town.shell,
                        scratch.object,
                        &raw mut always,
                    );
                } else {
                    town::house::openttd_rust_house_produced(
                        self.town,
                        self.shared,
                        tile,
                        scratch.object,
                    );
                }
            }
        }
    }

    fn town_route(&self, scratch: Cargo) -> bool {
        if !(self.can_allocate)() {
            return false;
        }
        let src = (self.random_town)();
        if unsafe { town::population(src.state) < SUBSIDY_CARGO_MIN_POPULATION } {
            return false;
        }
        self.scan(scratch, src.state, false);
        for index in 0..(self.passenger_count)() {
            let cargo = (self.passenger_at)(index);
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
        if !(self.can_allocate)() {
            return false;
        }
        let shell = unsafe { (self.industry().random_industry)() };
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
        let dst = if crate::services::chance16_i(1, 2, self.shared().random()) {
            let town = (self.random_town)();
            self.scan(scratch, town.state, true);
            if unsafe { scratch.data.add(usize::from(cargo)).read() < 8 } {
                return false;
            }
            Source::town(town.id.0)
        } else {
            let shell = unsafe { (self.industry().random_industry)() };
            if shell.is_null() {
                return false;
            }
            if !unsafe { industry::subsidy_accepts((self.industry_owner)(shell), cargo) } {
                return false;
            }
            Source::industry((self.industry_id)(shell))
        };
        if src == dst || !self.distance(src, dst) || Self::duplicate(cargo, src, dst) {
            return false;
        }
        self.create(cargo, src, dst);
        true
    }
}

impl Services {
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
        each(|id, state| {
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
        each(|id, state| {
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
        if !src.is_valid()
            || src.kind > Source::TOWN
            || !(self.has_source_flag)(src, PartOfSubsidy::Source)
        {
            return false;
        }
        // The original lazily creates a vector only after the first applicable
        // town destination. A fixed membership set preserves first-discovery
        // semantics without allocating on the cargo-delivery hot path.
        let station = (self.station)(station);
        let mut towns = TownsNear {
            services: self,
            bits: [0; 1024],
        };
        if !(self.station_empty)(station) {
            let mut needed = false;
            each(|_, state| {
                needed |= unsafe {
                    (*state).dst.kind == Source::TOWN
                        && (*state).cargo_type == cargo
                        && (*state).src == src
                        && (!(*state).is_awarded() || (*state).awarded == company)
                };
            });
            if needed {
                (self.catchment)(station, (&raw mut towns).cast(), catchment_tile);
            }
        }
        let mut subsidised = false;
        each(|id, state| {
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
                let mut index = 0;
                loop {
                    let industry = (self.nearby_industry)(station, index);
                    if industry.is_null() {
                        break;
                    }
                    if (self.industry_id)(industry.cast()) == dst.id {
                        (self.assert_destination)(dst);
                        subsidised = true;
                        if unsafe { !(*state).is_awarded() } {
                            self.award(id, company);
                        }
                    }
                    index += 1;
                }
            } else if towns.bits[usize::from(dst.id) / 64] & (1 << (dst.id % 64)) != 0 {
                (self.assert_destination)(dst);
                subsidised = true;
                if unsafe { !(*state).is_awarded() } {
                    self.award(id, company);
                }
            }
        });
        subsidised
    }
}
#[repr(C)]
pub struct Station {
    _private: [u8; 0],
}
struct TownsNear<'a> {
    services: &'a Services,
    bits: [u64; 1024],
}
extern "C" fn catchment_tile(context: *mut c_void, tile: u32) {
    // SAFETY: Synchronous visitor receives the live stack context; scalar map
    // and cache reads finish before any callback. No native state is borrowed.
    let cache = unsafe { &mut *context.cast::<TownsNear<'_>>() };
    let shared = cache.services.shared();
    if (shared.tile_type)(tile) != crate::tile_type::MP_HOUSE {
        return;
    }
    let town = (shared.town_index)(tile);
    if (cache.services.has_source_flag)(Source::town(town), PartOfSubsidy::Destination) {
        cache.bits[usize::from(town) / 64] |= 1 << (town % 64);
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
pub unsafe extern "C" fn openttd_rust_subsidy_command(
    s: *const Services,
    flags: u16,
    cargo: u8,
    src: Source,
    dst: Source,
) -> bool {
    let s = unsafe { &*s };
    if !(s.can_allocate)()
        || (s.shared().current_company)() != crate::company_type::OWNER_DEITY
        || cargo >= NUM_CARGO
        || !(s.cargo_valid)(cargo)
        || !(s.valid_source)(src)
        || !(s.valid_source)(dst)
    {
        return false;
    }
    if flags & crate::command_type::DoCommandFlag::EXECUTE != 0 {
        s.create(cargo, src, dst);
    }
    true
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_monthly(s: *const Services, scratch: Cargo) {
    unsafe { &*s }.monthly(scratch);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_subsidy_check(
    s: *const Services,
    cargo: u8,
    company: u8,
    src: Source,
    station: u16,
) -> bool {
    unsafe { &*s }.check(cargo, company, src, station)
}

crate::abi::layouts! {
    fn abi_layout;
    "Source" ["probe"] => crate::subsidy::Source { id, kind => type, };
    "OpenTTDSubsidyState" ["probe"] => State { cargo_type, remaining, awarded, src, dst, };
    "OpenTTDSubsidyCargo" ["probe"] => Cargo { object, data, };
    "OpenTTDSubsidyServices" ["probe"] => Services {
        shared,
        town,
        industry,
        random_town,
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
        passenger_count,
        passenger_at,
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
        station_empty,
        catchment,
        nearby_industry,
        assert_destination,
    };
}
