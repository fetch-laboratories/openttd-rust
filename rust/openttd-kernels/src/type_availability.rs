/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The rail and road type availability table: the C++ the availability bodies of `rail.cpp`
//! and `road.cpp` read (`rail.rs`, `road/availability.rs`) and the engine and company code
//! that keeps a company's available types (`engine.rs`, `engine_loading.rs`), with the
//! engine test both `GetCompanyRailTypes` and `GetCompanyRoadTypes` write.
//!
//! The table is C++'s process-lifetime `GetRustTypeAvailabilityServices()`. The engines `next`
//! returns live in C++'s engine pool, and the `RoadTypeInfo` records `road_info` returns in
//! `_roadtypes`; both are read only by the call that resolved them, under the game-state
//! mutex, and no read of either lasts across a call.
use crate::company_type::{CompanyID, CompanyMask};
use crate::engine::EngineRef;
use crate::raw_field::read_field;
use crate::road::{RoadTypeFlags, RoadTypeInfo};
use crate::road_type::{RoadType, RoadTypeLabel, RoadTypes};
use crate::services::Services as Shared;
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_common::{DAYS_IN_YEAR, calendar_date};

/// The availability table.
#[repr(C)]
pub struct Services {
    /// `WITH_ASSERT`, for the two hand-written assertions of `engine.rs`.
    pub assertions: bool,
    /// `Company::Get(company)->avail_railtypes`.
    pub avail_railtypes: extern "C" fn(u8) -> u64,
    /// `Company::Get(company)->avail_roadtypes`.
    pub avail_roadtypes: extern "C" fn(u8) -> u64,
    /// `_game_mode == GM_EDITOR`.
    pub editor: extern "C" fn() -> bool,
    /// The engine at or after the given ID, in ascending ID order (`Engine::Iterate`), or a
    /// null shell past the end.
    pub next: unsafe extern "C" fn(u32) -> EngineRef,
    /// `RoadTypeInfo::flags`.
    pub road_flags: extern "C" fn(*const RoadTypeInfo) -> RoadTypeFlags,
    /// `GetRoadTypeInfo`, with its assertion.
    pub road_info: extern "C" fn(RoadType) -> *const RoadTypeInfo,
    /// `RoadTypeInfo::introduces_roadtypes`.
    pub road_introduces_roadtypes: extern "C" fn(*const RoadTypeInfo) -> RoadTypes,
    /// `RoadTypeInfo::introduction_date`.
    pub road_introduction_date: extern "C" fn(*const RoadTypeInfo) -> CalendarDate,
    /// `RoadTypeInfo::introduction_required_roadtypes`.
    pub road_introduction_required_roadtypes: extern "C" fn(*const RoadTypeInfo) -> RoadTypes,
    /// `RoadTypeInfo::label`.
    pub road_label: extern "C" fn(*const RoadTypeInfo) -> RoadTypeLabel,
    /// Store `Company::Get(company)->avail_railtypes`.
    pub set_company_railtypes: extern "C" fn(u8, u64),
    /// Store `Company::Get(company)->avail_roadtypes`.
    pub set_company_roadtypes: extern "C" fn(u8, u64),
    /// The shared table.
    pub shared: *const Shared,
}

/// The availability table the shared table names.
#[allow(
    unsafe_code,
    reason = "the process-lifetime table the shared table names"
)]
pub fn services(shared: &Shared) -> &Services {
    // SAFETY: The shared table's `type_availability` returns the process-lifetime table.
    unsafe { &*(shared.type_availability)() }
}

/// `e->company_avail.Test(company) || TimerGameCalendar::date >= e->intro_date +
/// CalendarTime::DAYS_IN_YEAR`, the engine test of `GetCompanyRailTypes` and
/// `GetCompanyRoadTypes`, on an engine `next` returned.
#[allow(
    unsafe_code,
    reason = "field-sized reads of the engine lifecycle `next` resolved"
)]
pub(crate) fn company_available(e: EngineRef, company: CompanyID) -> bool {
    CompanyMask(read_field!(e.state, company_avail)).test(company)
        || calendar_date().0 >= read_field!(e.state, intro_date).wrapping_add(DAYS_IN_YEAR)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTypeAvailabilityServices" ["engine"] => Services {
        assertions,
        avail_railtypes,
        avail_roadtypes,
        editor,
        next,
        road_flags,
        road_info,
        road_introduces_roadtypes,
        road_introduction_date,
        road_introduction_required_roadtypes,
        road_label,
        set_company_railtypes,
        set_company_roadtypes,
        shared,
    };
}
