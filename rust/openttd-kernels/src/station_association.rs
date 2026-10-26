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
use crate::newgrf_class::Identity;
use crate::station::BaseStation;
use crate::station_cmd::{RailSpec, SpecIndex};
use std::ptr;
const NUM_STATIONSSPECS_PER_STATION: usize = 255;
#[repr(C)]
pub struct Vacancy {
    spec: *const RailSpec,
    grfid: u32,
}
#[repr(C)]
pub struct Triggers {
    animation: u16,
    cargo: u64,
}
#[repr(C)]
pub struct Services {
    triggers: unsafe extern "C" fn(*const RailSpec) -> Triggers,
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
    identity: unsafe extern "C" fn(*const RailSpec) -> Identity,
    custom_index: unsafe extern "C" fn(u32) -> u32,
    set_cached: unsafe extern "C" fn(*mut BaseStation, u16, u64),
}
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
    if st.is_null() {
        return SpecIndex {
            value: 1,
            valid: true,
        };
    }
    let s = unsafe { &*s };
    let count = unsafe { (s.count)(st) };
    let mut i = 1;
    while i < count && i < NUM_STATIONSSPECS_PER_STATION {
        let slot = unsafe { (s.vacancy)(st, i) };
        if slot.spec.is_null() && slot.grfid == 0 {
            break;
        }
        i += 1;
    }
    if i == NUM_STATIONSSPECS_PER_STATION {
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
pub(crate) fn cache(native: *const Services, st: *mut BaseStation) {
    let s = unsafe { &*native };
    unsafe {
        (s.set_cached)(st, 0, 0);
    }
    let mut anim = 0;
    let mut cargo = 0;
    for i in 0..unsafe { (s.count)(st) } {
        let spec = unsafe { (s.spec)(st, i) };
        if spec.is_null() {
            continue;
        }
        let triggers = unsafe { (s.triggers)(spec) };
        anim |= triggers.animation;
        cargo |= triggers.cargo;
        unsafe {
            (s.set_cached)(st, anim, cargo);
        }
    }
}
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
    let identity = unsafe { (s.identity)(spec) };
    unsafe {
        (s.set_slot)(st, index, spec, identity.grfid, identity.local_id);
    }
    cache(native, st);
}
pub(crate) fn deallocate(native: *const Services, st: *mut BaseStation, index: u8) {
    if index == 0 {
        return;
    }
    let s = unsafe { &*native };
    let animation = crate::animated_tile::services();
    let area = (animation.rail_area)(
        st,
        crate::tile_type::INVALID_TILE,
        crate::newgrf_station::TA_WHOLE,
    );
    let map = unsafe { &*s.shared }.map();
    for y in 0..area.h {
        for x in 0..area.w {
            let tile = area.tile.wrapping_add(map.diff(i32::from(x), i32::from(y)));
            if (animation.rail_belongs)(st, tile)
                && unsafe { (s.custom_index)(tile) } == u32::from(index)
            {
                return;
            }
        }
    }
    unsafe {
        (s.set_slot)(st, index, ptr::null(), 0, 0);
    }
    let count = unsafe { (s.count)(st) };
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
                (s.set_cached)(st, 0, 0);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_spec_rebind(
    s: *const Services,
    st: *mut BaseStation,
) {
    let s = unsafe { &*s };
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
    "OpenTTDStationSpecTriggers" ["station_service"] => Triggers { animation, cargo };
    "OpenTTDStationSpecVacancy" ["station_service"] => Vacancy { spec, grfid };
    "OpenTTDStationAssociationServices" ["station_service"] => Services { triggers, shared, roadstop, count, vacancy, spec, key, resize, clear, set_slot, rebind, identity, custom_index, set_cached };
}
