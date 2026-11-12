/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical registry inputs for paired rail/road availability. Tables and shell
//! handles are call-scoped, serialized by the game-state mutex. No lifecycle
//! borrow survives a callback; registry observations copy only consumed fields.
#![allow(unsafe_code)]
use crate::engine::EngineRef;
use crate::road::RoadTypeFlags;
use crate::road_type::{RoadType, RoadTypeLabel, RoadTypes};
use crate::services::Services as Shared;
use crate::timer_game_calendar::CalendarDate;
#[repr(C)]
pub struct Introduction {
    pub label: u32,
    pub date: i32,
    pub required: u64,
    pub introduces: u64,
}
/// Native `RoadTypeInfo`, opaque: Rust holds the `GetRoadTypeInfo` result only through the
/// call that resolved it and reads its fields through the accessors below.
#[repr(C)]
pub struct RoadTypeInfo {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Services {
    pub assertions: bool,
    pub company_railtypes: extern "C" fn(u8) -> u64,
    pub company_roadtypes: extern "C" fn(u8) -> u64,
    pub editor: extern "C" fn() -> bool,
    pub next: unsafe extern "C" fn(u32) -> EngineRef,
    pub rail_introduces: extern "C" fn(u8) -> u64,
    pub rail_introduction: extern "C" fn(u8) -> Introduction,
    /// `RoadTypeInfo::flags`.
    pub road_flags: extern "C" fn(*const RoadTypeInfo) -> RoadTypeFlags,
    /// `GetRoadTypeInfo`, retaining its assertion.
    pub road_info: extern "C" fn(RoadType) -> *const RoadTypeInfo,
    /// `RoadTypeInfo::introduces_roadtypes`.
    pub road_introduces: extern "C" fn(*const RoadTypeInfo) -> RoadTypes,
    /// `RoadTypeInfo::introduction_date`.
    pub road_introduction_date: extern "C" fn(*const RoadTypeInfo) -> CalendarDate,
    /// `RoadTypeInfo::introduction_required_roadtypes`.
    pub road_introduction_required: extern "C" fn(*const RoadTypeInfo) -> RoadTypes,
    /// `RoadTypeInfo::label`.
    pub road_label: extern "C" fn(*const RoadTypeInfo) -> RoadTypeLabel,
    pub set_company_railtypes: extern "C" fn(u8, u64),
    pub set_company_roadtypes: extern "C" fn(u8, u64),
    pub shared: *const Shared,
}
/// Handles returned here remain live only in this synchronous no-reset interval.
pub fn services(shared: &Shared) -> &Services {
    // SAFETY: The native entry supplies its immutable live table. No owner is borrowed.
    unsafe { &*(shared.type_availability)() }
}
/// Read canonical lifecycle scalars without borrowing the owner across reentry.
pub unsafe fn company_available(e: EngineRef, company: u8) -> bool {
    // SAFETY: next resolved one live engine shell/owner; reads end before leaves.
    let available = unsafe { std::ptr::addr_of!((*e.state).company_avail).read() };
    u64::from(available) & 1u64.wrapping_shl(u32::from(company)) != 0
        || crate::timer_game_common::calendar_date().0
            >= unsafe { std::ptr::addr_of!((*e.state).intro_date).read() }
                .wrapping_add(crate::timer_game_common::DAYS_IN_YEAR)
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTypeIntroduction" ["engine"] => Introduction { label, date, required, introduces };
    "OpenTTDTypeAvailabilityServices" ["engine"] => Services {
        assertions,
        company_railtypes,
        company_roadtypes,
        editor,
        next,
        rail_introduces,
        rail_introduction,
        road_flags,
        road_info,
        road_introduces,
        road_introduction_date,
        road_introduction_required,
        road_label,
        set_company_railtypes,
        set_company_roadtypes,
        shared,
    };
}
