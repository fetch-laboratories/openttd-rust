/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Remaining town policies over canonical owners and shared native operations.
use super::{Leaves, State};
use crate::cargo_type::{CargoType, INVALID_CARGO};
use crate::cargotype::TownProductionEffect;
use crate::company_type::CompanyID;
use crate::error::{WarningLevel, show_error_message};
use crate::road_type::RoadTramType;
use crate::strings_func::get_encoded_string;
use crate::strings_type::{EncodedString, StringParameterData};
use crate::table::strings::{
    STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL,
    STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL_EXPLANATION,
    STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET,
    STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET_EXPLANATION,
};
use crate::tile_type::TileIndex;
use crate::timer_game_calendar::CalendarDate;
use crate::{cargo_registry, services::Services};

/// Update the maximum amount of monthly passengers and mail for a town, based on its
/// population (`UpdateTownMaxPass`).
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
                let entry = unsafe { super::supplied_entry(owner, cargo) };
                unsafe {
                    (*entry.entry).history[usize::from(crate::history_type::LAST_MONTH)]
                        .production = production;
                }
            }
            i += 1;
        }
    }
}
/// Get the calendar date of the earliest town-buildable road type
/// (`GetTownRoadTypeFirstIntroductionDate`).
/// Return the introduction date of the earliest road type, or `INT32_MAX` if none is available.
fn get_town_road_type_first_introduction_date(leaves: &Leaves) -> CalendarDate {
    let mut best = CalendarDate(i32::MAX);
    for rt in crate::road::get_mask_for_road_tram_type(RoadTramType::RTT_ROAD).iter() {
        let rti = (leaves.road_info)(rt.0);
        // Town can't build this road type.
        if !rti.town_build {
            continue;
        }
        if CalendarDate(rti.introduction_date) >= best {
            continue;
        }
        best = CalendarDate(rti.introduction_date);
    }
    best
}
/// Check if towns are able to build road (`CheckTownRoadTypes`).
/// Return true iff the towns are currently able to build road.
pub(crate) fn check_town_road_types(leaves: &Leaves) -> bool {
    let min_date = get_town_road_type_first_introduction_date(leaves);
    if min_date <= crate::timer_game_common::calendar_date() {
        return true;
    }
    if min_date < CalendarDate(i32::MAX) {
        show_error_message(
            get_encoded_string(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET, &[]),
            get_encoded_string(
                STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET_EXPLANATION,
                // `StringParameter` stores the date's `int32_t` base as `uint64_t`.
                &[StringParameterData::Integer(
                    i64::from(min_date.0).cast_unsigned(),
                )],
            ),
            WarningLevel::WL_CRITICAL,
            0,
            0,
            EncodedString::default(),
            CompanyID::INVALID,
        );
    } else {
        show_error_message(
            get_encoded_string(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL, &[]),
            get_encoded_string(
                STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL_EXPLANATION,
                &[],
            ),
            WarningLevel::WL_CRITICAL,
            0,
            0,
            EncodedString::default(),
            CompanyID::INVALID,
        );
    }
    false
}
// Compatibility name for the scalar world-generation caller.
pub(crate) use check_town_road_types as road_types;

/// Get the cost for removing this house (`HouseSpec::GetRemovalCost`).
/// Return the cost adjusted for inflation, etc.; `Money` multiplication saturates.
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
pub unsafe extern "C" fn openttd_rust_town_radius_group(
    owner: *const State,
    s: *const Services,
    tile: u32,
) -> u8 {
    unsafe { super::get_town_radius_group(owner, TileIndex(tile), (&*s).map()) }.0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_first_cargo(effect: u8) -> u8 {
    cargo_registry::first_town_acceptance(effect).unwrap_or(INVALID_CARGO)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_climate_mask(landscape: u8) -> u16 {
    crate::house::get_climate_mask_for_landscape(crate::landscape_type::LandscapeType(landscape)).0
}

#[cfg(test)]
mod tests {
    use crate::news_func::testing::{Param, encoded, error_message, take};
    use crate::table::strings::{
        STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL,
        STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL_EXPLANATION,
        STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET,
        STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET_EXPLANATION,
    };
    use crate::town::RoadChoice;
    use std::cell::Cell;

    thread_local! {
        static INTRODUCTION: Cell<i32> = const { Cell::new(0) };
    }
    extern "C" fn road_info(rt: u8) -> RoadChoice {
        assert_eq!(rt, 0);
        RoadChoice {
            introduction_date: INTRODUCTION.get(),
            max_speed: 0,
            town_build: true,
        }
    }

    /// `CheckTownRoadTypes`: a road type introduced later shows "not yet" with the date as the
    /// detail's one parameter; none at all (`INT32_MAX`) shows "not at all" without parameters.
    #[test]
    fn check_town_road_types_shows_the_original_messages() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let mask = crate::type_registry::openttd_rust_road_mask();
        // SAFETY: The registry mask is a process-wide scalar serialized by the lock.
        let saved_mask = unsafe { mask.replace(1) };
        let mut leaves = crate::test_tables::TOWN_LEAVES;
        leaves.road_info = road_info;
        take();
        // Later than any date of the test clock.
        INTRODUCTION.set(i32::MAX - 1);
        let later = super::check_town_road_types(&leaves);
        INTRODUCTION.set(i32::MAX);
        let never = super::check_town_road_types(&leaves);
        // SAFETY: Same serialized registry scalar.
        unsafe { mask.write(saved_mask) };
        assert!(!later && !never);
        assert_eq!(
            take(),
            [
                error_message(
                    encoded(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET.0, &[]),
                    encoded(
                        STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET_EXPLANATION.0,
                        &[Param::N(0x7FFF_FFFE)]
                    ),
                    3
                ),
                error_message(
                    encoded(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL.0, &[]),
                    encoded(
                        STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL_EXPLANATION.0,
                        &[]
                    ),
                    3
                ),
            ]
        );
    }
}
