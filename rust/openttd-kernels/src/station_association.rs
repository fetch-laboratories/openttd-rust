/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Rail definition association policy over station-owned Rust reference lists.
//! Definition pointers remain borrowed; saved identities survive resize and reload.
//! Entries run under the game-state mutex. No native element borrow crosses a callback.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::station_cmd::{RailSpec, SpecIndex};
use crate::station_record::record_of;
use crate::station_storage::{rail, road};
use crate::station_type::BaseStation;
use crate::tile_type::TileIndex;
use std::ptr;
const NUM_STATIONSSPECS_PER_STATION: usize = 255;
#[repr(C)]
pub struct Triggers {
    animation: u16,
    cargo: u64,
}
#[repr(C)]
pub struct Services {
    triggers: unsafe extern "C" fn(*const RailSpec) -> Triggers,
    shared: *const crate::services::Services,
}
pub(crate) fn allocate(spec: *const RailSpec, st: *mut BaseStation) -> SpecIndex {
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
    let count = rail::count(st);
    let mut i = 1;
    while i < count && i < NUM_STATIONSSPECS_PER_STATION {
        let slot = rail::get(st, i);
        if slot.spec.is_null() && slot.grfid == 0 {
            break;
        }
        i += 1;
    }
    if i == NUM_STATIONSSPECS_PER_STATION {
        for i in 1..count.min(NUM_STATIONSSPECS_PER_STATION) {
            if rail::get(st, i).spec == spec {
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
    let mut anim = 0;
    let mut cargo = 0;
    for i in 0..rail::count(st) {
        let spec = rail::get(st, i).spec;
        if spec.is_null() {
            continue;
        }
        let triggers = unsafe { (s.triggers)(spec) };
        anim |= triggers.animation;
        cargo |= triggers.cargo;
    }
    crate::station_record::set_cached_triggers(st, anim, cargo);
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
    if usize::from(index) >= rail::count(st) {
        unsafe {
            rail::resize(record_of(st), usize::from(index) + 1);
        }
    }
    let identity = unsafe { crate::newgrf_class::station_identity(spec) };
    rail::set(st, index, spec, identity.grfid, identity.local_id);
    cache(native, st);
}
pub(crate) fn deallocate(native: *const Services, st: *mut BaseStation, index: u8) {
    if index == 0 {
        return;
    }
    let s = unsafe { &*native };
    let area = crate::newgrf_station::rail_tile_area(
        unsafe { &*s.shared },
        st,
        crate::tile_type::INVALID_TILE,
        crate::newgrf_station::TA_WHOLE,
    );
    let shared = unsafe { &*s.shared };
    let command = unsafe { &*(shared.station_commands)() };
    let map = shared.map();
    for y in 0..area.h {
        for x in 0..area.w {
            let tile = TileIndex(area.tile.wrapping_add(map.diff(i32::from(x), i32::from(y))));
            if crate::station_record::rail_member(st, tile.0, shared)
                && (command.custom_rail_index)(tile.0) == u32::from(index)
            {
                return;
            }
        }
    }
    rail::set(st, index, ptr::null(), 0, 0);
    let count = rail::count(st);
    if usize::from(index) == count - 1 {
        let mut num = count - 1;
        while num > 0 {
            if rail::get(st, num).grfid != 0 {
                break;
            }
            num -= 1;
        }
        if num > 0 {
            unsafe {
                rail::resize(record_of(st), num + 1);
            }
        } else {
            unsafe {
                rail::resize(record_of(st), 0);
                crate::station_record::set_cached_triggers(st, 0, 0);
            }
            return;
        }
    }
    cache(native, st);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_spec_allocate(
    spec: *const RailSpec,
    st: *mut BaseStation,
) -> SpecIndex {
    allocate(spec, st)
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
pub unsafe extern "C" fn openttd_rust_station_spec_rebind(st: *mut BaseStation) {
    for i in 0..rail::count(st) {
        let key = rail::get(st, i);
        if key.grfid == 0 {
            continue;
        }
        let spec = unsafe { crate::newgrf_class::station::by_grf(key.grfid, key.localidx) };
        rail::rebind(st, i, spec);
    }
    for i in 0..road::count(st) {
        let key = road::get(st, i);
        if key.grfid == 0 {
            continue;
        }
        let spec = unsafe { crate::newgrf_class::roadstop::by_grf(key.grfid, key.localidx) };
        road::rebind(st, i, spec);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationSpecTriggers" ["station_service"] => Triggers { animation, cargo };
    "OpenTTDStationAssociationServices" ["station_service"] => Services { triggers, shared };
}
