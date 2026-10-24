/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical registry inputs for paired rail/road availability. Tables and shell
//! handles are call-scoped, serialized by the game-state mutex. No lifecycle
//! borrow survives a callback; registry observations copy only consumed fields.
#![allow(unsafe_code)]
use crate::engine::EngineRef;
use crate::services::{RoadTramType, Services as Shared};
use crate::vehicle::{OpenTTDEngine, RailVehicleTypes};
#[repr(C)]
pub struct Introduction {
    pub label: u32,
    pub date: i32,
    pub required: u64,
    pub introduces: u64,
}
#[repr(C)]
pub struct RoadConstruction {
    pub label: u32,
    pub date: i32,
    pub hidden: bool,
    pub town_build: bool,
}
#[repr(C)]
pub struct Services {
    pub next: unsafe extern "C" fn(u32) -> EngineRef,
    pub climate: unsafe extern "C" fn(*const OpenTTDEngine) -> bool,
    pub rail_kind: unsafe extern "C" fn(*const OpenTTDEngine) -> RailVehicleTypes,
    pub engine_railtypes: unsafe extern "C" fn(*const OpenTTDEngine) -> u64,
    pub engine_roadtype: unsafe extern "C" fn(*const OpenTTDEngine) -> u8,
    pub rail_introduction: extern "C" fn(u8) -> Introduction,
    pub road_introduction: extern "C" fn(u8) -> Introduction,
    pub rail_introduces: extern "C" fn(u8) -> u64,
    pub road_introduces: extern "C" fn(u8) -> u64,
    pub road_construction: extern "C" fn(u8) -> RoadConstruction,
    pub rail_hidden: extern "C" fn() -> u64,
    pub road_hidden: extern "C" fn() -> u64,
    pub road_tram_mask: extern "C" fn(RoadTramType) -> u64,
    pub company_railtypes: extern "C" fn(u8) -> u64,
    pub company_roadtypes: extern "C" fn(u8) -> u64,
    pub set_company_railtypes: extern "C" fn(u8, u64),
    pub set_company_roadtypes: extern "C" fn(u8, u64),
    pub editor: extern "C" fn() -> bool,
    pub assertions: bool,
}
/// Handles returned here remain live only in this synchronous no-reset interval.
pub fn services(shared: &Shared) -> &Services {
    // SAFETY: The native entry supplies its immutable live table. No owner is borrowed.
    unsafe { &*(shared.type_availability)() }
}
/// Read canonical lifecycle scalars without borrowing the owner across reentry.
pub unsafe fn company_available(e: EngineRef, company: u8, shared: &Shared) -> bool {
    // SAFETY: next resolved one live engine shell/owner; reads end before leaves.
    let available = unsafe { std::ptr::addr_of!((*e.state).company_avail).read() };
    u64::from(available) & 1u64.wrapping_shl(u32::from(company)) != 0
        || (shared.calendar_date)()
            >= unsafe { std::ptr::addr_of!((*e.state).intro_date).read() }
                .wrapping_add(crate::timer_game_common::DAYS_IN_YEAR)
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTypeIntroduction" ["engine"] => Introduction { label, date, required, introduces };
    "OpenTTDRoadTypeConstruction" ["engine"] => RoadConstruction { label, date, hidden, town_build };
    "OpenTTDTypeAvailabilityServices" ["engine"] => Services {
        next, climate, rail_kind, engine_railtypes, engine_roadtype,
        rail_introduction, road_introduction, rail_introduces, road_introduces,
        road_construction, rail_hidden, road_hidden, road_tram_mask,
        company_railtypes, company_roadtypes, set_company_railtypes,
        set_company_roadtypes, editor, assertions
    };
}
