/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Remaining town policies over canonical owners and shared native operations.
use super::{Leaves, State};
use crate::{cargo_registry, cargo_type::INVALID_CARGO, services::Services};

pub(crate) fn update_max_pass(leaves: &Leaves, owner: *mut State) {
    for (effect, shift) in [
        (crate::cargotype::TPE_PASSENGERS, 3),
        (crate::cargotype::TPE_MAIL, 4),
    ] {
        let mut i = 0;
        while i < cargo_registry::town_count(effect) {
            let population = unsafe { super::population(owner) };
            let production = (unsafe { &*leaves.houses }.scale_cargo)(population >> shift);
            if production != 0 {
                let cargo = cargo_registry::town_cargo(effect, i);
                // GetOrCreateCargoSupplied can relocate the vector; no borrow spans scaling.
                let entry = unsafe { super::openttd_rust_town_supplied_entry(owner, cargo) };
                unsafe {
                    (*entry.entry).history[usize::from(crate::history_type::LAST_MONTH)]
                        .production = production;
                }
            }
            i += 1;
        }
    }
}
pub(crate) fn road_types(leaves: &Leaves) -> bool {
    let mut mask = crate::road::get_mask_for_road_tram_type(crate::road_type::RTT_ROAD);
    let mut first = i32::MAX;
    while mask != 0 {
        let road = mask.trailing_zeros();
        mask &= mask - 1;
        let info = (leaves.road_info)(road as u8);
        if info.town_build && info.introduction_date < first {
            first = info.introduction_date;
        }
    }
    if first <= crate::timer_game_common::calendar_date().0 {
        return true;
    }
    (leaves.road_type_error)(first);
    false
}
pub(crate) fn removal_cost(factor: u8) -> i64 {
    crate::company::price(crate::economy_type::PR_CLEAR_HOUSE).saturating_mul(i64::from(factor))
        >> 8
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_update_max_pass(
    leaves: *const Leaves,
    owner: *mut State,
) {
    update_max_pass(unsafe { &*leaves }, owner);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_check_road_types(leaves: *const Leaves) -> bool {
    road_types(unsafe { &*leaves })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_radius_group(
    owner: *const State,
    s: *const Services,
    tile: u32,
) -> u8 {
    unsafe { super::get_radius_group(owner, tile, (&*s).map()) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_first_cargo(effect: u8) -> u8 {
    cargo_registry::first_town_acceptance(effect).unwrap_or(INVALID_CARGO)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_climate_mask(landscape: u8) -> u16 {
    crate::house::climate_mask(landscape)
}
