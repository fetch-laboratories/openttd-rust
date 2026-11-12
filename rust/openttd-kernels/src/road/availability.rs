/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `road.h` classification and the `road.cpp` road type introduction and construction
//! availability.
#![allow(unsafe_code)]
use crate::company_type::{CompanyID, Owner};
use crate::road_type::{RoadTramType, RoadType, RoadTypes};
use crate::services::Services as Shared;
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_common::MAX_DATE;
use crate::type_availability::{Services, company_available, services};
use crate::vehicle::VehicleType;

/// `GetMaskForRoadTramType`; classification masks belong to the registry owner.
#[must_use]
pub fn get_mask_for_road_tram_type(rtt: RoadTramType) -> RoadTypes {
    RoadTypes(crate::type_registry::road_tram_mask(rtt))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_road_type(
    shared: &Shared,
    s: &Services,
    company: u8,
    rt: u8,
) -> u8 {
    u8::from(has_road_type_avail(
        shared,
        s,
        CompanyID(company),
        RoadType(rt),
    ))
}
/// Base-integer form of `has_road_type_avail` for callers in files no pass has finished.
pub fn has_road_type_avail_raw(shared: &Shared, s: &Services, company: u8, rt: u8) -> bool {
    has_road_type_avail(shared, s, CompanyID(company), RoadType(rt))
}
/// `HasRoadTypeAvail`: finds out, whether given company has a given `RoadType` available for
/// construction.
pub fn has_road_type_avail(
    shared: &Shared,
    s: &Services,
    company: CompanyID,
    roadtype: RoadType,
) -> bool {
    if company == Owner::DEITY
        || company == Owner::TOWN
        || (s.editor)()
        || (shared.clear_square_services.generating_world)()
    {
        let rti = (s.road_construction)(roadtype.0);
        if rti.label == 0 {
            return false;
        }

        /* Not yet introduced at this date. */
        if (0..MAX_DATE).contains(&rti.date)
            && rti.date > crate::timer_game_common::calendar_date().0
        {
            return false;
        }

        /*
         * Do not allow building hidden road types, except when a town may build it.
         * The GS under deity mode, as well as anybody in the editor builds roads that are
         * owned by towns. So if a town may build it, it should be buildable by them too.
         */
        !rti.hidden || rti.town_build
    } else {
        // Company::GetIfValid has exactly the shared IsValidID precondition.
        if !(shared.company_valid)(company.0) {
            return false;
        }
        let avail =
            RoadTypes((s.company_roadtypes)(company.0) & !crate::type_registry::road_hidden());
        avail.test(roadtype)
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_any_road_types_abi(
    s: &Services,
    company: u8,
    rtt: RoadTramType,
) -> u8 {
    u8::from(has_any_road_types_avail(s, CompanyID(company), rtt))
}
/// `HasAnyRoadTypesAvail`: test if any buildable `RoadType` is available for a company.
pub fn has_any_road_types_avail(s: &Services, company: CompanyID, rtt: RoadTramType) -> bool {
    let avail = (s.company_roadtypes)(company.0) & !crate::type_registry::road_hidden();
    avail & get_mask_for_road_tram_type(rtt).0 != 0
}
/// Base-integer form of `val_param_road_type` for callers in files no pass has finished.
pub fn val_param_road_type_raw(shared: &Shared, roadtype: u8) -> bool {
    val_param_road_type(shared, RoadType(roadtype))
}
/// `ValParamRoadType`: validate functions for rail building; true if the current company may
/// build the road.
pub fn val_param_road_type(shared: &Shared, roadtype: RoadType) -> bool {
    roadtype < RoadType::ROADTYPE_END
        && has_road_type_avail(
            shared,
            services(shared),
            CompanyID((shared.current_company)()),
            roadtype,
        )
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_val_road_type(shared: &Shared, rt: u8) -> u8 {
    u8::from(val_param_road_type(shared, RoadType(rt)))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_add_date_road_types(s: &Services, current: u64, date: i32) -> u64 {
    add_date_introduced_road_types(s, RoadTypes(current), CalendarDate(date)).0
}
/// `AddDateIntroducedRoadTypes`: add the road types that are to be introduced at the given
/// date to `current`, the currently available roadtypes.
pub fn add_date_introduced_road_types(
    s: &Services,
    mut current: RoadTypes,
    date: CalendarDate,
) -> RoadTypes {
    loop {
        let mut rts = current;
        for rt in RoadType::ROADTYPE_BEGIN.0..RoadType::ROADTYPE_END.0 {
            let rti = (s.road_introduction)(rt);
            /* Unused road type. */
            if rti.label == 0 {
                continue;
            }

            /* Not date introduced. */
            if !(0..MAX_DATE).contains(&rti.date) {
                continue;
            }

            /* Not yet introduced at this date. */
            if rti.date > date.0 {
                continue;
            }

            /* Have we introduced all required roadtypes? */
            let required = rti.required;
            if rts.0 & required != required {
                continue;
            }

            rts = rts.with(RoadTypes(rti.introduces));
        }

        /* When we added roadtypes we need to run this method again; the added
         * roadtypes might enable more rail types to become introduced. */
        if rts == current {
            return rts;
        }
        current = rts;
    }
}
/// The `Engine::IterateType(VEH_ROAD)` loop of `GetCompanyRoadTypes` and `GetRoadTypes`, in
/// ascending engine IDs. With a company it also keeps the engine availability gate of
/// `GetCompanyRoadTypes`.
fn engine_road_types(s: &Services, company: Option<CompanyID>, introduces: bool) -> RoadTypes {
    let mut rts = RoadTypes::EMPTY;
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
        let engine = crate::vehicle::EngineRef::of(e.shell);
        // SAFETY: The availability table points at the process-lifetime shared table.
        let shared = unsafe { &*s.shared };
        let climate = unsafe { shared.settings().climate.read() };
        if !engine.climate(climate) {
            continue;
        }
        if let Some(company) = company
            && !unsafe { company_available(e, company.0) }
        {
            continue;
        }
        let roadtype = RoadType(engine.roadtype());
        crate::movement::native_assert(shared, roadtype < RoadType::ROADTYPE_END);
        rts = rts.with(if introduces {
            RoadTypes((s.road_introduces)(roadtype.0))
        } else {
            RoadTypes::from_element(roadtype)
        });
    }
    rts
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_road_types(
    s: &Services,
    company: u8,
    introduces: bool,
) -> u64 {
    get_company_road_types(s, CompanyID(company), introduces).0
}
/// `GetCompanyRoadTypes`: get the road types the given company can build. If `introduces`,
/// include road types introduced by other road types.
pub fn get_company_road_types(s: &Services, company: CompanyID, introduces: bool) -> RoadTypes {
    let rts = engine_road_types(s, Some(company), introduces);
    if introduces {
        return add_date_introduced_road_types(s, rts, crate::timer_game_common::calendar_date());
    }
    rts
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_types(s: &Services, introduces: bool) -> u64 {
    get_road_types(s, introduces).0
}
/// `GetRoadTypes`: get list of road types, regardless of company availability. If
/// `introduces`, include road types introduced by other road types.
pub fn get_road_types(s: &Services, introduces: bool) -> RoadTypes {
    let rts = engine_road_types(s, None, introduces);
    if introduces {
        return add_date_introduced_road_types(s, rts, CalendarDate(MAX_DATE));
    }
    rts
}

/// Base-integer form of `road_type_is_road` for callers in files no pass has finished.
#[must_use]
pub fn road_type_is_road_raw(roadtype: u8) -> bool {
    road_type_is_road(RoadType(roadtype))
}
/// `RoadTypeIsRoad` reads the canonical classification mask.
#[must_use]
pub fn road_type_is_road(roadtype: RoadType) -> bool {
    get_mask_for_road_tram_type(RoadTramType::RTT_ROAD).test(roadtype)
}
/// Base-integer form of `road_type_is_tram` for callers in files no pass has finished.
#[must_use]
pub fn road_type_is_tram_raw(roadtype: u8) -> bool {
    road_type_is_tram(RoadType(roadtype))
}
/// `RoadTypeIsTram` reads the canonical classification mask.
#[must_use]
pub fn road_type_is_tram(roadtype: RoadType) -> bool {
    get_mask_for_road_tram_type(RoadTramType::RTT_TRAM).test(roadtype)
}
/// Base-integer form of `get_road_tram_type` for callers in files no pass has finished.
#[must_use]
pub fn get_road_tram_type_raw(roadtype: u8) -> RoadTramType {
    get_road_tram_type(RoadType(roadtype))
}
/// `GetRoadTramType`.
#[must_use]
pub fn get_road_tram_type(roadtype: RoadType) -> RoadTramType {
    if road_type_is_tram(roadtype) {
        RoadTramType::RTT_TRAM
    } else {
        RoadTramType::RTT_ROAD
    }
}
