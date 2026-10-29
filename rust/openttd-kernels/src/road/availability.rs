/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `road.h` classification and complete road introduction/construction availability.
#![allow(unsafe_code)]
use crate::company_type::{OWNER_DEITY, OWNER_TOWN};
use crate::road_type::ROADTYPE_END;
use crate::services::{RoadTramType, Services as Shared};
use crate::timer_game_common::MAX_DATE;
use crate::type_availability::{Services, company_available, services};
use crate::vehicle::VehicleType;

/// `GetMaskForRoadTramType`; classification masks belong to the registry owner.
#[must_use]
pub fn get_mask_for_road_tram_type(rtt: RoadTramType) -> u64 {
    crate::type_registry::road_tram_mask(rtt)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_road_type(
    shared: &Shared,
    s: &Services,
    company: u8,
    rt: u8,
) -> u8 {
    u8::from(has_road_type(shared, s, company, rt))
}
/// `HasRoadTypeAvail`, retaining the original construction and company gates.
pub fn has_road_type(shared: &Shared, s: &Services, company: u8, rt: u8) -> bool {
    if company == OWNER_DEITY
        || company == OWNER_TOWN
        || (s.editor)()
        || (shared.clear_square_services.generating_world)()
    {
        let info = (s.road_construction)(rt);
        if info.label == 0 {
            return false;
        }
        if (0..MAX_DATE).contains(&info.date) && info.date > (shared.calendar_date)() {
            return false;
        }
        !info.hidden || info.town_build
    } else {
        // Company::GetIfValid has exactly the shared IsValidID precondition.
        if !(shared.company_valid)(company) {
            return false;
        }
        let available = (s.company_roadtypes)(company) & !crate::type_registry::road_hidden();
        available & 1u64.wrapping_shl(u32::from(rt)) != 0
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_any_road_types(
    s: &Services,
    company: u8,
    rtt: RoadTramType,
) -> bool {
    let available = (s.company_roadtypes)(company) & !crate::type_registry::road_hidden();
    available & get_mask_for_road_tram_type(rtt) != 0
}
pub fn val_param_road_type(shared: &Shared, rt: u8) -> bool {
    rt < ROADTYPE_END && has_road_type(shared, services(shared), (shared.current_company)(), rt)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_val_road_type(shared: &Shared, rt: u8) -> u8 {
    u8::from(val_param_road_type(shared, rt))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_add_date_road_types(
    s: &Services,
    mut current: u64,
    date: i32,
) -> u64 {
    loop {
        let mut types = current;
        for rt in 0..ROADTYPE_END {
            let info = (s.road_introduction)(rt);
            if info.label == 0 || !(0..MAX_DATE).contains(&info.date) || info.date > date {
                continue;
            }
            if types & info.required != info.required {
                continue;
            }
            types |= info.introduces;
        }
        if types == current {
            return types;
        }
        current = types;
    }
}
fn engine_types(shared: &Shared, s: &Services, company: Option<u8>, introduces: bool) -> u64 {
    let mut types = 0;
    let mut from = 0;
    loop {
        // SAFETY: Pool leaves resolve live shell/owner handles. No borrow crosses calls.
        let e = unsafe { (s.next)(from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if e.kind != VehicleType::ROAD {
            continue;
        }
        let info = unsafe { (s.road_availability)(e.shell) };
        if !info.climate {
            continue;
        }
        if let Some(company) = company {
            if !unsafe { company_available(e, company, shared) } {
                continue;
            }
        }
        let rt = info.roadtype;
        if s.assertions {
            assert!(rt < ROADTYPE_END);
        }
        types |= if introduces {
            (s.road_introduces)(rt)
        } else {
            1u64.wrapping_shl(u32::from(rt))
        };
    }
    types
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_road_types(
    shared: &Shared,
    s: &Services,
    company: u8,
    introduces: bool,
) -> u64 {
    let types = engine_types(shared, s, Some(company), introduces);
    if introduces {
        openttd_rust_add_date_road_types(s, types, (shared.calendar_date)())
    } else {
        types
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_types(shared: &Shared, s: &Services, introduces: bool) -> u64 {
    let types = engine_types(shared, s, None, introduces);
    if introduces {
        openttd_rust_add_date_road_types(s, types, MAX_DATE)
    } else {
        types
    }
}

/// `RoadTypeIsRoad` reads the canonical classification mask.
#[must_use]
pub fn road_type_is_road(rt: u8) -> bool {
    crate::type_registry::road_tram_mask(crate::road_type::RTT_ROAD) & (1u64 << rt) != 0
}
/// `RoadTypeIsTram` reads the canonical classification mask.
#[must_use]
pub fn road_type_is_tram(rt: u8) -> bool {
    crate::type_registry::road_tram_mask(crate::road_type::RTT_TRAM) & (1u64 << rt) != 0
}
/// `GetRoadTramType` defaults to road for unclassified types.
#[must_use]
pub fn get_road_tram_type(rt: u8) -> RoadTramType {
    RoadTramType(road_type_is_tram(rt))
}
