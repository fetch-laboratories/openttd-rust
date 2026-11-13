/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Rail definition association policy. Native station shells and saved slot records remain
//! canonical; typed container operations preserve their identity through resize and reload.
//! Entries run under the game-state mutex. No native element borrow crosses a callback.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::cargo_type::CargoTypes;
use crate::newgrf_class::Identity;
use crate::newgrf_station::NUM_STATIONSSPECS_PER_STATION;
use crate::station_cmd::{RailSpec, SpecIndex};
use crate::station_type::BaseStation;
use crate::station_type::StationAnimationTriggers;
use std::ptr;
#[repr(C)]
pub struct Vacancy {
    spec: *const RailSpec,
    grfid: u32,
}
#[repr(C)]
pub struct Services {
    /// `st->cached_anim_triggers.Set(spec->animation.triggers)`.
    pub(crate) rail_triggers: extern "C" fn(*mut BaseStation, *const RailSpec),
    /// The rail resolver services, for the cached and spec cargo triggers.
    resolver: *const crate::newgrf_station::Services,
    shared: *const crate::services::Services,
    roadstop: *const crate::newgrf_roadstop::Services,
    count: unsafe extern "C" fn(*const BaseStation) -> usize,
    vacancy: unsafe extern "C" fn(*const BaseStation, usize) -> Vacancy,
    spec: unsafe extern "C" fn(*const BaseStation, usize) -> *const RailSpec,
    key: unsafe extern "C" fn(*const BaseStation, usize) -> Identity,
    resize: unsafe extern "C" fn(*mut BaseStation, usize),
    clear: unsafe extern "C" fn(*mut BaseStation),
    set_slot: unsafe extern "C" fn(*mut BaseStation, u8, *const RailSpec, u32, u16),
    rebind: unsafe extern "C" fn(*mut BaseStation, usize, *const RailSpec),
    set_cached: unsafe extern "C" fn(*mut BaseStation, u16, u64),
    set_cached_cargo: unsafe extern "C" fn(*mut BaseStation, u64),
}
/// `AllocateSpecToStation`: allocate a `StationSpec` to a station. `valid ==
/// false` is `std::nullopt`, no free slot; the index is in `value` otherwise.
pub(crate) fn allocate(
    s: *const Services,
    spec: *const RailSpec,
    st: *mut BaseStation,
) -> SpecIndex {
    if spec.is_null() {
        return SpecIndex {
            value: 0,
            valid: true,
        };
    }
    // If the station does not exist yet, its first custom slot is available.
    if st.is_null() {
        return SpecIndex {
            value: 1,
            valid: true,
        };
    }
    let s = unsafe { &*s };
    let count = unsafe { (s.count)(st) };
    // `i` is the original `uint i` reused by both loops.
    let mut i = 1;
    while i < count && i < NUM_STATIONSSPECS_PER_STATION {
        let slot = unsafe { (s.vacancy)(st, i) };
        if slot.spec.is_null() && slot.grfid == 0 {
            break;
        }
        i += 1;
    }
    if i == NUM_STATIONSSPECS_PER_STATION {
        // As final effort when the spec list is already full...
        // try to find the same spec and return that one. This might
        // result in slightly "wrong" (as per specs) looking stations,
        // but it's fairly unlikely that one reaches the limit anyways.
        for i in 1..count.min(NUM_STATIONSSPECS_PER_STATION) {
            if unsafe { (s.spec)(st, i) } == spec {
                return SpecIndex {
                    value: i as u8,
                    valid: true,
                };
            }
        }
        return SpecIndex {
            value: 0,
            valid: false,
        };
    }
    SpecIndex {
        value: i as u8,
        valid: true,
    }
}
/// `StationUpdateCachedTriggers`: update the cached animation trigger bitmask
/// for a station.
pub(crate) fn cache(native: *const Services, st: *mut BaseStation) {
    let s = unsafe { &*native };
    // `cached_anim_triggers = {}` and `cached_cargo_triggers = 0`.
    unsafe {
        (s.set_cached)(st, StationAnimationTriggers::EMPTY.0, CargoTypes(0).0);
    }
    /* Combine animation trigger bitmask for all station specs
     * of this station. */
    for i in 0..unsafe { (s.count)(st) } {
        let spec = unsafe { (s.spec)(st, i) };
        if spec.is_null() {
            continue;
        }
        (s.rail_triggers)(st, spec);
        // `cached_cargo_triggers |= spec->cargo_triggers`: the right operand
        // is read before the cached field.
        let resolver = unsafe { &*s.resolver };
        let cargo = CargoTypes((resolver.spec_cargo)(spec));
        let cached = CargoTypes((resolver.cached_cargo)(st));
        unsafe {
            (s.set_cached_cargo)(st, cached.0 | cargo.0);
        }
    }
}
/// `AssignSpecToStation`: assign a previously allocated `StationSpec` spec
/// index to a station.
pub(crate) fn assign(
    native: *const Services,
    spec: *const RailSpec,
    st: *mut BaseStation,
    index: u8,
) {
    if index == 0 {
        return;
    }
    let s = unsafe { &*native };
    if usize::from(index) >= unsafe { (s.count)(st) } {
        unsafe {
            (s.resize)(st, usize::from(index) + 1);
        }
    }
    let identity = unsafe { crate::newgrf_class::station_identity(spec) };
    unsafe {
        (s.set_slot)(st, index, spec, identity.grfid, identity.local_id);
    }
    cache(native, st);
}
/// `DeallocateSpecFromStation`: deallocate a `StationSpec` from a station.
/// Called when removing a single station tile.
pub(crate) fn deallocate(native: *const Services, st: *mut BaseStation, index: u8) {
    // specindex of 0 (default) is never freeable
    if index == 0 {
        return;
    }
    let s = unsafe { &*native };
    let animation = crate::animated_tile::services();
    let area = crate::newgrf_station::rail_tile_area(
        unsafe { &*s.shared },
        st,
        crate::tile_type::INVALID_TILE,
        crate::newgrf_station::TA_WHOLE,
    );
    let shared = unsafe { &*s.shared };
    let command = unsafe { &*(shared.station_commands)() };
    let map = shared.map();
    // Check all tiles over the station to check if the specindex is still in use
    for tile in area.tiles(map) {
        if (animation.rail_belongs)(st, tile.0)
            && (command.custom_rail_index)(tile.0) == u32::from(index)
        {
            return;
        }
    }
    // This specindex is no longer in use, so deallocate it
    unsafe {
        (s.set_slot)(st, index, ptr::null(), 0, 0);
    }
    let count = unsafe { (s.count)(st) };
    // If this was the highest spec index, reallocate
    if usize::from(index) == count - 1 {
        let mut num = count - 1;
        while num > 0 {
            if unsafe { (s.key)(st, num) }.grfid != 0 {
                break;
            }
            num -= 1;
        }
        if num > 0 {
            unsafe {
                (s.resize)(st, num + 1);
            }
        } else {
            unsafe {
                (s.clear)(st);
                (s.set_cached)(st, StationAnimationTriggers::EMPTY.0, CargoTypes(0).0);
            }
            return;
        }
    }
    cache(native, st);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_spec_allocate(
    s: *const Services,
    spec: *const RailSpec,
    st: *mut BaseStation,
) -> SpecIndex {
    allocate(s, spec, st)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_spec_assign(
    s: *const Services,
    spec: *const RailSpec,
    st: *mut BaseStation,
    index: u8,
) {
    assign(s, spec, st, index);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_spec_cache(s: *const Services, st: *mut BaseStation) {
    cache(s, st);
}
/// The speclist part of `AfterLoadStations`, for one station.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_spec_rebind(
    s: *const Services,
    st: *mut BaseStation,
) {
    let s = unsafe { &*s };
    // Update the speclists of all stations to point to the currently loaded custom stations.
    for i in 0..unsafe { (s.count)(st) } {
        let key = unsafe { (s.key)(st, i) };
        if key.grfid == 0 {
            continue;
        }
        let spec = unsafe { crate::newgrf_class::station::by_grf(key.grfid, key.local_id) };
        unsafe {
            (s.rebind)(st, i, spec);
        }
    }
    let road = unsafe { &*s.roadstop };
    for i in 0..unsafe { (road.spec_count)(st) } {
        let key = unsafe { (road.slot_key)(st, i) };
        if key.grfid == 0 {
            continue;
        }
        let spec = unsafe { crate::newgrf_class::roadstop::by_grf(key.grfid, key.local_id) };
        unsafe {
            (road.rebind)(st, i, spec);
        }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationSpecVacancy" ["station_service"] => Vacancy { spec, grfid };
    "OpenTTDStationAssociationServices" ["station_service"] => Services { rail_triggers, resolver, shared, roadstop, count, vacancy, spec, key, resize, clear, set_slot, rebind, set_cached, set_cached_cargo };
}
