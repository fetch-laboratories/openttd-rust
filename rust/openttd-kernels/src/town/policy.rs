/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Remaining town policies over canonical owners and shared native operations.
use super::{Leaves, State};
use crate::cargo_type::{CargoType, INVALID_CARGO};
use crate::cargotype::TownProductionEffect;
use crate::road_type::{RoadTramType, RoadType, RoadTypes};
use crate::timer_game_calendar::CalendarDate;
use crate::{cargo_registry, services::Services};

/// `UpdateTownMaxPass`: update maximum monthly passengers and mail from population.
pub(crate) fn update_town_max_pass(leaves: &Leaves, owner: *mut State) {
    for (effect, shift) in [
        (TownProductionEffect::TPE_PASSENGERS, 3),
        (TownProductionEffect::TPE_MAIL, 4),
    ] {
        let mut i = 0;
        while i < cargo_registry::town_count(effect.0) {
            let population = unsafe { super::population(owner) };
            let production = (unsafe { &*leaves.houses }.scale_cargo)(population >> shift);
            if production != 0 {
                let cargo = CargoType(cargo_registry::town_cargo(effect.0, i));
                // GetOrCreateCargoSupplied can relocate the vector; no borrow spans scaling.
                let entry = unsafe { super::openttd_rust_town_supplied_entry(owner, cargo.0) };
                unsafe {
                    (*entry.entry).history[usize::from(crate::history_type::LAST_MONTH)]
                        .production = production;
                }
            }
            i += 1;
        }
    }
}
/// `CheckTownRoadTypes`: whether towns can currently build a road type.
pub(crate) fn check_town_road_types(leaves: &Leaves) -> bool {
    // GetTownRoadTypeFirstIntroductionDate: earliest town-buildable road type,
    // or INT32_MAX if none is available.
    let mut mask = RoadTypes(crate::road::get_mask_for_road_tram_type(
        RoadTramType::RTT_ROAD,
    ));
    let mut first = CalendarDate(i32::MAX);
    while mask.0 != 0 {
        let road = RoadType(mask.0.trailing_zeros() as u8);
        mask.0 &= mask.0 - 1;
        let info = (leaves.road_info)(road.0);
        // Town can't build a road type without the TownBuild flag.
        if info.town_build && CalendarDate(info.introduction_date) < first {
            first = CalendarDate(info.introduction_date);
        }
    }
    if first <= crate::timer_game_common::calendar_date() {
        return true;
    }
    (leaves.road_type_error)(first.0);
    false
}
/// `HouseSpec::GetRemovalCost`: cost of removing this house, adjusted for inflation.
pub(crate) fn get_removal_cost(factor: u8) -> i64 {
    crate::company::price(crate::economy_type::PR_CLEAR_HOUSE).saturating_mul(i64::from(factor))
        >> 8
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_update_max_pass(
    leaves: *const Leaves,
    owner: *mut State,
) {
    update_town_max_pass(unsafe { &*leaves }, owner);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_check_road_types_abi(leaves: *const Leaves) -> u8 {
    u8::from(check_town_road_types(unsafe { &*leaves }))
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
