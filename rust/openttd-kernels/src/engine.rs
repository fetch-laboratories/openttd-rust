/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Engine lifecycle state. Native shells own this allocation through ordinary and indexed
//! construction, temporary staging and vector relocation. Both languages read the generated
//! record in place, and views end before mutation or callbacks.
#![allow(
    unsafe_code,
    clippy::cast_ptr_alignment,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::struct_field_names,
    clippy::struct_excessive_bools,
    clippy::fn_params_excessive_bools
)]
mod access;
pub(crate) mod bindings;
pub(crate) mod records;
use crate::industrytype::NativeVec;
use crate::raw_field::{read_field, write_field};
use crate::services::ByteSpan;
use crate::timer_game_common::{DAYS_IN_LEAP_YEAR, DAYS_IN_YEAR};
crate::abi::shared_records! {
    fn abi_record;
    /// Canonical lifecycle and name, stable through temporary shell relocation.
    "OpenTTDEngine" ["engine"] => pub struct Lifecycle {
        pub shell: *const crate::engine_type::NativeEngine => "const Engine *",
        pub company_avail: u16 => "CompanyMask",
        pub company_hidden: u16 => "CompanyMask",
        pub preview_asked: u16 => "CompanyMask",
        pub intro_date: i32 => "TimerGameCalendar::Date",
        pub age: i32,
        pub reliability: u16,
        pub reliability_spd_dec: u16,
        pub reliability_start: u16,
        pub reliability_max: u16,
        pub reliability_final: u16,
        pub duration_phase_1: u16,
        pub duration_phase_2: u16,
        pub duration_phase_3: u16,
        pub flags: u8 => "EngineFlags",
        pub preview_company: u8 => "CompanyID",
        pub preview_wait: u8,
        pub name: NativeVec<u8> => "RustVector<uint8_t>",
        pub info: records::EngineInfo,
        pub vehicle_info: records::VehicleInfo,
        pub vehicle_info_type: VehicleType => "VehicleType",
        pub original_image_index: u8,
        pub r#type: VehicleType => "VehicleType",
        pub list_position: u16,
        pub bindings: bindings::Bindings,
    }
}
impl Drop for Lifecycle {
    fn drop(&mut self) {
        // Native binding members are destroyed before the lifecycle/name owner.
        drop(std::mem::take(&mut self.bindings));
    }
}
impl Lifecycle {
    /// Active payload pointer for a live serialized record; never retained across mutation/drop.
    pub(crate) unsafe fn rail(record: *const Self) -> *mut records::RailVehicleInfo {
        assert_eq!(
            read_field!(record, vehicle_info_type).0,
            VehicleType::TRAIN.0
        );
        unsafe { std::ptr::addr_of_mut!((*record.cast_mut()).vehicle_info.rail) }
    }
    /// Active payload pointer for a live serialized record; never retained across mutation/drop.
    pub(crate) unsafe fn road(record: *const Self) -> *mut records::RoadVehicleInfo {
        assert_eq!(
            read_field!(record, vehicle_info_type).0,
            VehicleType::ROAD.0
        );
        unsafe { std::ptr::addr_of_mut!((*record.cast_mut()).vehicle_info.road) }
    }
    /// Active payload pointer for a live serialized record; never retained across mutation/drop.
    pub(crate) unsafe fn ship(record: *const Self) -> *mut records::ShipVehicleInfo {
        assert_eq!(
            read_field!(record, vehicle_info_type).0,
            VehicleType::SHIP.0
        );
        unsafe { std::ptr::addr_of_mut!((*record.cast_mut()).vehicle_info.ship) }
    }
    /// Active payload pointer for a live serialized record; never retained across mutation/drop.
    pub(crate) unsafe fn aircraft(record: *const Self) -> *mut records::AircraftVehicleInfo {
        assert_eq!(
            read_field!(record, vehicle_info_type).0,
            VehicleType::AIRCRAFT.0
        );
        unsafe { std::ptr::addr_of_mut!((*record.cast_mut()).vehicle_info.aircraft) }
    }
}
impl Default for Lifecycle {
    fn default() -> Self {
        Self {
            shell: std::ptr::null(),
            company_avail: 0,
            company_hidden: 0,
            preview_asked: 0,
            intro_date: 0,
            age: 0,
            reliability: 0,
            reliability_spd_dec: 0,
            reliability_start: 0,
            reliability_max: 0,
            reliability_final: 0,
            duration_phase_1: 0,
            duration_phase_2: 0,
            duration_phase_3: 0,
            flags: 0,
            preview_company: 255,
            preview_wait: 0,
            name: NativeVec::new(),
            info: records::EngineInfo::default(),
            vehicle_info: records::VehicleInfo::default(),
            vehicle_info_type: VehicleType(crate::vehicle_type::VEH_INVALID),
            original_image_index: 0,
            r#type: VehicleType(crate::vehicle_type::VEH_INVALID),
            list_position: 0,
            bindings: bindings::Bindings::default(),
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_new() -> *mut Lifecycle {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    Box::into_raw(Box::new(Lifecycle::default()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_destroy(state: *mut Lifecycle) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    // SAFETY: The shell transfers its sole live allocation at destruction.
    drop(unsafe { Box::from_raw(state) });
}
unsafe fn set_name(state: *mut Lifecycle, text: ByteSpan) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    // SAFETY: The live owner's byte vector is Rust-owned and serialized. The
    // shared assignment copies aliased input before replacing that allocation.
    unsafe {
        crate::owned_bytes::openttd_rust_owned_bytes_assign(
            std::ptr::addr_of_mut!((*state).name),
            text,
        );
    }
}

use crate::company::OpenTTDCompany;
use crate::services::{RandomServices, Services as SharedServices};
use crate::vehicle::{OpenTTDEngine, OpenTTDVehicle, RailVehicleTypes, VehicleType};
use std::ffi::c_uint;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EngineRef {
    pub shell: *const OpenTTDEngine,
    pub state: *mut Lifecycle,
    pub id: u16,
    pub kind: VehicleType,
}
pub use crate::services::CalendarYmd as Ymd;
#[repr(C)]
pub struct StartSettings {
    pub starting_year: i32,
    pub date: i32,
}
#[repr(C)]
pub struct Company {
    pub shell: *const OpenTTDCompany,
    pub id: u8,
}
#[repr(C)]
pub struct Vehicle {
    pub shell: *mut OpenTTDVehicle,
    pub id: u32,
    pub owner: u8,
    pub kind: VehicleType,
}
#[repr(C)]
pub struct VehicleCargo {
    pub engine: *const OpenTTDEngine,
    pub cargo: u8,
}
#[repr(C)]
pub struct Services {
    pub shared: *const SharedServices,
    /// Canonical company finance records (`Company::Finances()`).
    pub finance: *const crate::company::FinanceServices,
    pub rng: *const RandomServices,
    pub types: *const crate::type_availability::Services,
    pub resolve: unsafe extern "C" fn(u16) -> EngineRef,
    pub display_variant: unsafe extern "C" fn(*const OpenTTDEngine) -> u16,
    pub engine_id: unsafe extern "C" fn(*const OpenTTDEngine) -> u16,
    pub start_settings: unsafe extern "C" fn() -> StartSettings,
    pub road_limit: unsafe extern "C" fn() -> u16,
    pub ship_limit: unsafe extern "C" fn() -> u16,
    pub aircraft_limit: unsafe extern "C" fn() -> u16,
    pub ai_train_disabled: unsafe extern "C" fn() -> bool,
    pub ai_road_disabled: unsafe extern "C" fn() -> bool,
    pub ai_ship_disabled: unsafe extern "C" fn() -> bool,
    pub ai_aircraft_disabled: unsafe extern "C" fn() -> bool,
    pub clear_last_variant: unsafe extern "C" fn(*const OpenTTDEngine),
    pub refresh_type: unsafe extern "C" fn(VehicleType),
    pub refresh_lists: unsafe extern "C" fn(bool),
    pub company_next: unsafe extern "C" fn(u32) -> Company,
    pub vehicle_next: unsafe extern "C" fn(u32) -> Vehicle,
    pub vehicle_cargo: unsafe extern "C" fn(*mut OpenTTDVehicle) -> VehicleCargo,
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    pub company_engine_count: unsafe extern "C" fn(u8, u16) -> c_uint,
    pub local_company: unsafe extern "C" fn() -> u8,
    pub toolbar_enable: unsafe extern "C" fn(VehicleType),
    pub toolbar_release: unsafe extern "C" fn(VehicleType),
    pub close_preview: unsafe extern "C" fn(u16),
    pub invalidate_preview: unsafe extern "C" fn(u16),
    pub offer_preview: unsafe extern "C" fn(u16, u8, bool),
    pub available_event: unsafe extern "C" fn(u16),
    pub available_news: unsafe extern "C" fn(u16),
    pub no_engines: unsafe extern "C" fn(i32),
}

const AVAILABLE: u8 = 1;
const EXCLUSIVE: u8 = 2;
use crate::company_type::{CompanyID, OWNER_DEITY};
use crate::engine_type::INVALID_ENGINE as INVALID;
// Original static year is process-wide; all engine entries run serialized by
// the game-state mutex, including main-thread GUI commands and game timers.
static mut AGING_STOPS: i32 = 0;
macro_rules! call {
    ($s:expr, $field:ident $(, $arg:expr)* $(,)?) => {
        crate::witness::counted!(ENGINE_SERVICES, ENGINE_BYTES, ($s.$field)($($arg),*))
    };
}
/// `Company::Get(company)->Finances()`, the canonical record of a live company.
/// # Safety
/// `company` names a live company; the immutable finance table outlives the call.
unsafe fn finances(s: &Services, company: u8) -> *mut crate::company::Finances {
    unsafe { ((*s.finance).company_finances)(company) }
}
unsafe fn parent(s: &Services, mut e: EngineRef) -> EngineRef {
    loop {
        let parent = read_field!(e.state, info.variant_id).0;
        let sync = read_field!(e.state, info.extra_flags) & (1 << 3) != 0;
        if parent == INVALID || !sync {
            return e;
        }
        crate::witness::hit(crate::witness::ENGINE_SYNC);
        e = unsafe { call!(s, resolve, parent) };
    }
}
unsafe fn clear_last_variant(s: &Services, e: EngineRef) {
    unsafe {
        call!(s, clear_last_variant, e.shell);
    }
}
unsafe fn never_expire(s: &Services) -> bool {
    let enabled = unsafe { (*s.shared).settings().never_expire_vehicles.read() };
    if enabled {
        crate::witness::hit(crate::witness::ENGINE_NEVER_EXPIRE);
    }
    enabled
}
unsafe fn reliability(s: &Services, e: EngineRef, month: bool) {
    let re = unsafe { parent(s, e) };
    let mut age = read_field!(re.state, age) as u32;
    if month
        && unsafe { call!(s, engine_id, re.shell) > call!(s, engine_id, e.shell) }
        && age != i32::MAX as u32
    {
        age = age.wrapping_add(1);
    }
    let phase1 = u32::from(read_field!(e.state, duration_phase_1));
    let phase2 = u32::from(read_field!(e.state, duration_phase_2));
    let phase3 = u32::from(read_field!(e.state, duration_phase_3));
    if read_field!(e.state, company_avail) != 0
        && !unsafe { never_expire(s) }
        && read_field!(e.state, info.base_life) != 255
    {
        let early = i32::from(read_field!(e.state, info.retire_early));
        let max_age = ((phase1 + phase2) as i32 - early * 12).max(0) as u32;
        if early != 0 && age >= max_age {
            write_field!(e.state, company_avail, 0);
            unsafe {
                clear_last_variant(s, e);
                call!(s, refresh_type, e.kind);
            }
        }
    }
    let value;
    if age < phase1 {
        let start = u32::from(read_field!(e.state, reliability_start));
        value = age
            .wrapping_mul(u32::from(read_field!(e.state, reliability_max)).wrapping_sub(start))
            / phase1
            + start;
    } else {
        age = age.wrapping_sub(phase1);
        if age < phase2 || unsafe { never_expire(s) } || read_field!(e.state, info.base_life) == 255
        {
            value = u32::from(read_field!(e.state, reliability_max));
        } else {
            age = age.wrapping_sub(phase2);
            if age < phase3 {
                let max = u32::from(read_field!(e.state, reliability_max));
                let delta =
                    u32::from(read_field!(e.state, reliability_final)).wrapping_sub(max) as i32;
                value = ((age as i32).wrapping_mul(delta) / phase3 as i32) as u32;
                write_field!(e.state, reliability, value.wrapping_add(max) as u16);
                return;
            }
            crate::witness::hit(crate::witness::ENGINE_RETIRE);
            write_field!(e.state, company_avail, 0);
            write_field!(
                e.state,
                reliability,
                read_field!(e.state, reliability_final)
            );
            unsafe {
                clear_last_variant(s, e);
                call!(s, refresh_type, e.kind);
            }
            return;
        }
    }
    write_field!(e.state, reliability, value as u16);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_reliability(
    s: *const Services,
    e: EngineRef,
    month: bool,
) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    // SAFETY: Entry borrows its immutable native table; state accesses are raw,
    // field-sized and end before every possibly reentrant callback.
    unsafe {
        reliability(&*s, e, month);
    }
}
#[allow(clippy::too_many_lines)]
unsafe fn start_one(s: &Services, e: EngineRef, aging: Ymd, seed: u32) {
    let climate = unsafe { climate(s, e.shell) };
    let settings = unsafe { call!(s, start_settings) };
    write_field!(e.state, age, 0);
    write_field!(e.state, flags, 0);
    write_field!(e.state, company_avail, 0);
    write_field!(e.state, company_hidden, 0);
    let rng = unsafe { &*s.rng };
    let shared = unsafe { &*s.shared };
    let saved = unsafe { call!(rng, save) };
    let intro_seed = unsafe { shared.settings().generation_seed.read() }
        ^ seed
        ^ read_field!(e.state, info.base_intro) as u32
        ^ u32::from(e.kind.0)
        ^ unsafe { crate::engine_capabilities::openttd_rust_engine_grfid(s.capabilities, e.shell) };
    unsafe { call!(rng, set_seed, intro_seed) };
    let r = call!(shared, random, shared.context);
    let cutoff = crate::timer_game_common::calendar_ymd_to_date(
        settings.starting_year.wrapping_add(2),
        0,
        1,
    );
    let base_intro = read_field!(e.state, info.base_intro);
    let intro = if base_intro <= cutoff {
        base_intro
    } else {
        base_intro.wrapping_add((r & 511) as i32)
    };
    write_field!(e.state, intro_date, intro);
    if intro <= settings.date {
        let ymd = crate::timer_game_common::calendar_date_to_ymd(intro);
        let aging_months = aging
            .year
            .wrapping_mul(12)
            .wrapping_add(i32::from(aging.month));
        let intro_months = ymd
            .year
            .wrapping_mul(12)
            .wrapping_add(i32::from(ymd.month))
            .wrapping_add(i32::from(ymd.day > 1));
        write_field!(e.state, age, aging_months.wrapping_sub(intro_months));
        write_field!(e.state, company_avail, u16::MAX);
        write_field!(e.state, flags, read_field!(e.state, flags) | AVAILABLE);
    }
    let re = unsafe { parent(s, e) };
    let reliability_seed = unsafe { shared.settings().generation_seed.read() }
        ^ seed
        ^ (u32::from(unsafe { call!(s, engine_id, re.shell) }) << 16)
        ^ (read_field!(re.state, info.base_intro) as u32).wrapping_shl(12)
        ^ (u32::from(read_field!(re.state, info.decay_speed)) << 8)
        ^ (read_field!(re.state, info.lifelength) as u32).wrapping_shl(4)
        ^ (i32::from(read_field!(re.state, info.retire_early)) as u32)
        ^ u32::from(e.kind.0)
        ^ unsafe { crate::engine_capabilities::openttd_rust_engine_grfid(s.capabilities, e.shell) };
    unsafe { call!(rng, set_seed, reliability_seed) };
    let r = call!(shared, random, shared.context);
    write_field!(
        e.state,
        reliability_start,
        ((r >> 16) & 16383) as u16 + 0x7ae0
    );
    write_field!(e.state, reliability_max, (r & 16383) as u16 + 0xbfff);
    let r = call!(shared, random, shared.context);
    write_field!(
        e.state,
        reliability_final,
        ((r >> 16) & 16383) as u16 + 0x3fff
    );
    write_field!(e.state, duration_phase_1, (r & 31) as u16 + 7);
    write_field!(
        e.state,
        duration_phase_2,
        read_field!(e.state, info.base_life)
            .wrapping_mul(12)
            .wrapping_add(((r >> 5) & 15) as i32)
            .wrapping_sub(96)
            .max(0) as u16
    );
    write_field!(e.state, duration_phase_3, ((r >> 9) & 127) as u16 + 120);
    unsafe {
        call!(rng, restore, saved);
    }
    write_field!(
        e.state,
        reliability_spd_dec,
        u16::from(read_field!(e.state, info.decay_speed)) << 2
    );
    if !climate {
        write_field!(e.state, flags, read_field!(e.state, flags) | AVAILABLE);
        write_field!(e.state, company_avail, 0);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_start_one(
    s: *const Services,
    e: EngineRef,
    aging: Ymd,
    seed: u32,
) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    unsafe {
        start_one(&*s, e, aging, seed);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_aging_stops(s: *const Services) {
    set_year_engine_aging_stops(unsafe { &*s });
}
/// `SetYearEngineAgingStops`: compute the year in which engine aging stops.
fn set_year_engine_aging_stops(s: &Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let mut year = 2050;
    let mut from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if !unsafe { climate(s, e.shell) } || unsafe { wagon(e) } {
            continue;
        }
        let date = read_field!(e.state, info.base_intro).wrapping_add(
            read_field!(e.state, info.lifelength).wrapping_mul(DAYS_IN_LEAP_YEAR) / 2,
        );
        year = year.max(crate::timer_game_common::calendar_date_to_ymd(date).year);
    }
    unsafe {
        AGING_STOPS = year;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_start_all(s: *const Services) {
    unsafe { startup_engines(&*s) };
}
/// `StartupEngines`: start/initialise all our engines.
/// # Safety
/// The immutable tables and the engine records they reach stay live for the call.
pub(crate) unsafe fn startup_engines(s: &Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let date = crate::timer_game_common::calendar_date().0.min(
        crate::timer_game_common::calendar_ymd_to_date(unsafe { AGING_STOPS }, 0, 1),
    );
    let aging = crate::timer_game_common::calendar_date_to_ymd(date);
    let shared = unsafe { &*s.shared };
    let seed = call!(shared, random, shared.context);
    let mut from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        unsafe {
            start_one(s, e, aging, seed);
        }
    }
    from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        unsafe {
            reliability(s, e, false);
        }
    }
    from = 0;
    loop {
        let c = unsafe { call!(s, company_next, from) };
        if c.id == 255 {
            break;
        }
        from = u32::from(c.id) + 1;
        unsafe {
            ((&*s.types).set_company_railtypes)(
                c.id,
                crate::rail::openttd_rust_company_rail_types(&*s.types, c.id, true),
            );
            ((&*s.types).set_company_roadtypes)(
                c.id,
                crate::road::openttd_rust_company_road_types(&*s.types, c.id, true),
            );
        }
    }
    unsafe {
        call!(s, refresh_lists, true);
    }
}
unsafe fn wagon(e: EngineRef) -> bool {
    e.kind == VehicleType::TRAIN
        && read_field!(Lifecycle::rail(e.shell), railveh_type) == RailVehicleTypes::WAGON.0
}
unsafe fn disabled(s: &Services, kind: VehicleType, ai: bool) -> bool {
    match kind {
        VehicleType::TRAIN => unsafe {
            (*s.shared).settings().max_trains.read() == 0 || (ai && call!(s, ai_train_disabled))
        },
        VehicleType::ROAD => unsafe {
            call!(s, road_limit) == 0 || (ai && call!(s, ai_road_disabled))
        },
        VehicleType::SHIP => unsafe {
            call!(s, ship_limit) == 0 || (ai && call!(s, ai_ship_disabled))
        },
        VehicleType::AIRCRAFT => unsafe {
            call!(s, aircraft_limit) == 0 || (ai && call!(s, ai_aircraft_disabled))
        },
        _ => unreachable!(),
    }
}
unsafe fn enable(s: &Services, e: EngineRef, company: u8) {
    write_field!(
        e.state,
        company_avail,
        read_field!(e.state, company_avail) | (1u16 << company)
    );
    unsafe {
        if e.kind == VehicleType::TRAIN {
            ((&*s.types).set_company_railtypes)(
                company,
                crate::rail::openttd_rust_company_rail_types(&*s.types, company, true),
            );
        } else if e.kind == VehicleType::ROAD {
            ((&*s.types).set_company_roadtypes)(
                company,
                crate::road::openttd_rust_company_road_types(&*s.types, company, true),
            );
        }
    }
    if company == unsafe { call!(s, local_company) } {
        unsafe {
            call!(s, refresh_type, e.kind);
            call!(s, toolbar_enable, e.kind);
        }
    }
}
unsafe fn disable(s: &Services, e: EngineRef, company: u8) {
    write_field!(
        e.state,
        company_avail,
        read_field!(e.state, company_avail) & !(1u16 << company)
    );
    unsafe {
        if e.kind == VehicleType::TRAIN {
            ((&*s.types).set_company_railtypes)(
                company,
                crate::rail::openttd_rust_company_rail_types(&*s.types, company, true),
            );
        } else if e.kind == VehicleType::ROAD {
            ((&*s.types).set_company_roadtypes)(
                company,
                crate::road::openttd_rust_company_road_types(&*s.types, company, true),
            );
        }
    }
    if company == unsafe { call!(s, local_company) } {
        unsafe {
            clear_last_variant(s, e);
            call!(s, refresh_type, e.kind);
        }
    }
}
unsafe fn accept(s: &Services, e: EngineRef, company: u8, depth: i32) {
    crate::witness::hit(if depth == 0 {
        crate::witness::ENGINE_ACCEPT
    } else {
        crate::witness::ENGINE_JOIN
    });
    write_field!(e.state, preview_company, 255);
    write_field!(e.state, preview_asked, u16::MAX);
    unsafe {
        enable(s, e, company);
        call!(s, invalidate_preview, e.id);
    }
    if depth >= 10 {
        return;
    }
    let mut from = 0;
    loop {
        let ve = unsafe { call!((*s.types), next, from) };
        if ve.shell.is_null() {
            break;
        }
        from = u32::from(ve.id) + 1;
        if ve.kind != e.kind {
            continue;
        }
        let parent = read_field!(ve.state, info.variant_id).0;
        let join = read_field!(ve.state, info.extra_flags) & (1 << 2) != 0;
        if ve.id != e.id && parent == e.id && join {
            unsafe {
                accept(s, ve, company, depth + 1);
            }
        }
    }
}
unsafe fn preview_company(s: &Services, e: EngineRef) -> u8 {
    let mask = if e.kind == VehicleType::TRAIN {
        u64::MAX
    } else {
        unsafe {
            crate::engine_capabilities::openttd_rust_articulated_refit_union(
                s.capabilities,
                (s.engine_id)(e.shell),
                true,
            )
        }
    };
    let mut best = 255;
    let mut performance = -1;
    let mut from = 0;
    loop {
        let c = unsafe { call!(s, company_next, from) };
        if c.id == 255 {
            break;
        }
        from = u32::from(c.id) + 1;
        let finances = unsafe { finances(s, c.id) };
        if read_field!(finances, preview) != 0
            || read_field!(e.state, preview_asked) & (1 << c.id) != 0
            || read_field!(finances, old[0].performance) <= performance
        {
            continue;
        }
        let mut veh_from = 0;
        loop {
            let v = unsafe { call!(s, vehicle_next, veh_from) };
            if v.shell.is_null() {
                break;
            }
            veh_from = v.id + 1;
            if v.owner != c.id || v.kind != e.kind {
                continue;
            }
            let cargo = unsafe { call!(s, vehicle_cargo, v.shell) };
            if !unsafe {
                crate::engine_capabilities::openttd_rust_engine_can_carry_abi(
                    s.capabilities,
                    cargo.engine,
                ) != 0
            } || mask & 1u64.wrapping_shl(u32::from(cargo.cargo)) == 0
            {
                continue;
            }
            performance = read_field!(finances, old[0].performance);
            best = c.id;
            break;
        }
    }
    best
}
pub(crate) unsafe fn openttd_rust_engine_daily(s: *const Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let date = crate::timer_game_common::calendar_date().0;
    let mut from = 0;
    loop {
        let c = unsafe { call!(s, company_next, from) };
        if c.id == 255 {
            break;
        }
        from = u32::from(c.id) + 1;
        unsafe {
            let types = &*s.types;
            (types.set_company_railtypes)(
                c.id,
                crate::rail::openttd_rust_add_date_rail_types(
                    types,
                    call!((*s.types), company_railtypes, c.id),
                    date,
                ),
            );
            (types.set_company_roadtypes)(
                c.id,
                crate::road::openttd_rust_add_date_road_types(
                    types,
                    call!((*s.types), company_roadtypes, c.id),
                    date,
                ),
            );
        }
    }
    if crate::timer_game_common::calendar_year().0 >= unsafe { AGING_STOPS } {
        crate::witness::hit(crate::witness::ENGINE_CUTOFF);
        return;
    }
    from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if read_field!(e.state, flags) & EXCLUSIVE == 0 {
            continue;
        }
        if read_field!(e.state, preview_company) != 255 {
            let wait = read_field!(e.state, preview_wait).wrapping_sub(1);
            write_field!(e.state, preview_wait, wait);
            if wait == 0 {
                unsafe {
                    call!(s, close_preview, e.id);
                }
                write_field!(e.state, preview_company, 255);
            }
        } else if read_field!(e.state, preview_asked).count_ones() < 15 {
            let company = unsafe { preview_company(s, e) };
            write_field!(e.state, preview_company, company);
            if company == 255 {
                write_field!(e.state, preview_asked, u16::MAX);
                continue;
            }
            write_field!(
                e.state,
                preview_asked,
                read_field!(e.state, preview_asked) | (1 << company)
            );
            write_field!(e.state, preview_wait, 20);
            let ai = !unsafe { disabled(s, e.kind, true) };
            unsafe {
                crate::witness::hit(crate::witness::ENGINE_OFFER);
                call!(s, offer_preview, e.id, company, ai);
            }
        }
    }
}
unsafe fn available(s: &Services, e: EngineRef, date: i32) {
    if read_field!(e.state, flags) & EXCLUSIVE != 0 {
        let mut from = 0;
        loop {
            let c = unsafe { call!(s, company_next, from) };
            if c.id == 255 {
                break;
            }
            from = u32::from(c.id) + 1;
            if read_field!(e.state, company_avail) & (1 << c.id) == 0 {
                continue;
            }
            if unsafe { call!(s, company_engine_count, c.id, e.id) } == 0 {
                crate::witness::hit(crate::witness::ENGINE_PENALTY);
                let finances = unsafe { finances(s, c.id) };
                write_field!(finances, preview, 20);
            }
        }
    }
    write_field!(
        e.state,
        flags,
        (read_field!(e.state, flags) & !EXCLUSIVE) | AVAILABLE
    );
    unsafe {
        call!(s, refresh_type, e.kind);
    }
    write_field!(e.state, company_avail, u16::MAX);
    if unsafe { wagon(e) } {
        return;
    }
    if e.kind == VehicleType::TRAIN {
        let types = unsafe { &*s.types };
        let rails = crate::vehicle::EngineRef::of(e.shell).railtypes();
        if types.assertions {
            assert_ne!(rails, 0);
        }
        let introduced = crate::rail::get_all_introduces_rail_types(types, rails);
        let mut from = 0;
        loop {
            let c = unsafe { call!(s, company_next, from) };
            if c.id == 255 {
                break;
            }
            from = u32::from(c.id) + 1;
            let types = unsafe { call!((*s.types), company_railtypes, c.id) } | introduced;
            unsafe {
                ((&*s.types).set_company_railtypes)(
                    c.id,
                    crate::rail::openttd_rust_add_date_rail_types(&*s.types, types, date),
                );
            }
        }
    } else if e.kind == VehicleType::ROAD {
        let types = unsafe { &*s.types };
        let rt = crate::vehicle::EngineRef::of(e.shell).roadtype();
        if types.assertions {
            assert!(rt < crate::road_type::ROADTYPE_END);
        }
        let introduced = (types.road_introduces)(rt);
        let mut from = 0;
        loop {
            let c = unsafe { call!(s, company_next, from) };
            if c.id == 255 {
                break;
            }
            from = u32::from(c.id) + 1;
            let types = unsafe { call!((*s.types), company_roadtypes, c.id) } | introduced;
            unsafe {
                ((&*s.types).set_company_roadtypes)(
                    c.id,
                    crate::road::openttd_rust_add_date_road_types(&*s.types, types, date),
                );
            }
        }
    }
    if !unsafe { disabled(s, e.kind, true) } {
        unsafe {
            call!(s, available_event, e.id);
        }
    }
    if !unsafe { disabled(s, e.kind, false) } && read_field!(e.state, info.extra_flags) & 1 == 0 {
        unsafe {
            call!(s, available_news, e.id);
        }
    }
    unsafe {
        call!(s, toolbar_release, e.kind);
        call!(s, close_preview, e.id);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_monthly(s: *const Services) {
    unsafe { calendar_engines_monthly_loop(&*s) };
}
/// `CalendarEnginesMonthlyLoop`: monthly update of the availability, reliability and
/// preview of engines.
/// # Safety
/// The immutable tables and the engine records they reach stay live for the call.
pub(crate) unsafe fn calendar_engines_monthly_loop(s: &Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    if crate::timer_game_common::calendar_year().0 >= unsafe { AGING_STOPS } {
        crate::witness::hit(crate::witness::ENGINE_CUTOFF);
        return;
    }
    let date = crate::timer_game_common::calendar_date().0;
    let mut refresh = false;
    let mut from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if read_field!(e.state, flags) & AVAILABLE != 0 && read_field!(e.state, age) != i32::MAX {
            write_field!(e.state, age, read_field!(e.state, age).wrapping_add(1));
            unsafe {
                reliability(s, e, true);
            }
            refresh = true;
        }
        if !unsafe { openttd_rust_engine_enabled_abi(s, e.shell) != 0 } {
            continue;
        }
        if read_field!(e.state, flags) & AVAILABLE == 0
            && date >= read_field!(e.state, intro_date).wrapping_add(DAYS_IN_YEAR)
        {
            unsafe {
                available(s, e, date);
            }
        } else if read_field!(e.state, flags) & (AVAILABLE | EXCLUSIVE) == 0
            && date >= read_field!(e.state, intro_date)
        {
            if unsafe { disabled(s, e.kind, false) }
                || unsafe { wagon(e) }
                || read_field!(e.state, info.extra_flags) & 2 != 0
            {
                continue;
            }
            write_field!(e.state, flags, read_field!(e.state, flags) | EXCLUSIVE);
            write_field!(e.state, preview_company, 255);
            write_field!(e.state, preview_asked, 0);
        }
    }
    unsafe {
        call!(s, refresh_lists, refresh);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_clear_hidden(s: *const Services, company: u8) {
    clear_engines_hidden_flag_of_company(unsafe { &*s }, CompanyID(company));
}
/// `ClearEnginesHiddenFlagOfCompany`: clear the 'hidden' flag for all engines of a new
/// company.
fn clear_engines_hidden_flag_of_company(s: &Services, cid: CompanyID) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let company = cid.0;
    let mut from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        write_field!(
            e.state,
            company_hidden,
            read_field!(e.state, company_hidden) & !company_mask(company)
        );
    }
}
fn company_mask(company: u8) -> u16 {
    if company < 16 { 1 << company } else { 0 }
}
unsafe fn buildable(s: &Services, e: EngineRef, kind: VehicleType, company: u8) -> bool {
    if e.shell.is_null() || e.kind != kind {
        return false;
    }
    if company == OWNER_DEITY {
        if read_field!(e.state, flags) & AVAILABLE == 0 || read_field!(e.state, company_avail) == 0
        {
            return false;
        }
    } else if read_field!(e.state, company_avail) & company_mask(company) == 0 {
        return false;
    }
    if !unsafe { openttd_rust_engine_enabled_abi(s, e.shell) != 0 } {
        return false;
    }
    if company != OWNER_DEITY {
        if kind == VehicleType::TRAIN
            && unsafe {
                crate::vehicle::EngineRef::of(e.shell).compatible_railtypes(&*s.shared)
                    & call!((*s.types), company_railtypes, company)
            } == 0
        {
            return false;
        }
        if kind == VehicleType::ROAD
            && unsafe {
                crate::vehicle::EngineRef::of(e.shell).powered_roadtypes(&*s.shared)
                    & call!((*s.types), company_roadtypes, company)
            } == 0
        {
            return false;
        }
    }
    true
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_buildable_abi(
    s: *const Services,
    e: EngineRef,
    kind: VehicleType,
    company: u8,
) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    u8::from(unsafe { buildable(&*s, e, kind, company) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_variant_hidden_abi(
    s: *const Services,
    e: EngineRef,
    company: u8,
) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    if company >= 15 {
        return u8::from(false);
    }
    if read_field!(e.state, company_hidden) & (1 << company) != 0 {
        return u8::from(true);
    }
    let s = unsafe { &*s };
    let mut re = e;
    loop {
        let parent = read_field!(re.state, info.variant_id).0;
        let display = unsafe { call!(s, display_variant, re.shell) };
        let ve = if display == re.id || display == INVALID {
            re
        } else {
            unsafe { call!(s, resolve, display) }
        };
        let hidden = read_field!(ve.state, company_hidden) & (1 << company) != 0;
        if hidden || parent == INVALID {
            return u8::from(hidden);
        }
        re = unsafe { call!(s, resolve, parent) };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_visibility(
    s: *const Services,
    e: EngineRef,
    hide: bool,
    execute: bool,
) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let company = unsafe { call!(&*s.shared, current_company) };
    if e.shell.is_null() || company >= 15 || !unsafe { buildable(s, e, e.kind, company) } {
        return 1;
    }
    if execute {
        crate::witness::hit(crate::witness::ENGINE_VISIBILITY);
        let mask = 1 << company;
        write_field!(
            e.state,
            company_hidden,
            if hide {
                read_field!(e.state, company_hidden) | mask
            } else {
                read_field!(e.state, company_hidden) & !mask
            }
        );
        unsafe {
            call!(s, refresh_type, e.kind);
        }
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_want_preview(
    s: *const Services,
    e: EngineRef,
    execute: bool,
) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let company = unsafe { call!(&*s.shared, current_company) };
    if e.shell.is_null()
        || read_field!(e.state, flags) & EXCLUSIVE == 0
        || read_field!(e.state, preview_company) != company
    {
        return 1;
    }
    if execute {
        unsafe {
            accept(s, e, company, 0);
        }
    }
    0
}
/// `CmdEngineCtrl`'s arguments.
#[derive(Clone, Copy)]
struct CmdEngineCtrl {
    /// Engine id, resolved by the caller (a null shell for an invalid id).
    engine_id: EngineRef,
    /// Company to allow/forbid the use of an engine.
    company_id: CompanyID,
    /// false to forbid, true to allow.
    allow: bool,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_control(
    s: *const Services,
    e: EngineRef,
    company: u8,
    allow: bool,
    execute: bool,
) -> u8 {
    cmd_engine_ctrl(
        unsafe { &*s },
        execute,
        CmdEngineCtrl {
            engine_id: e,
            company_id: CompanyID(company),
            allow,
        },
    )
}
/// `CmdEngineCtrl`: allow or forbid a specific company to use an engine. Returns 0 on
/// success and 1 for the command error.
fn cmd_engine_ctrl(s: &Services, execute: bool, args: CmdEngineCtrl) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let (e, company) = (args.engine_id, args.company_id.0);
    if unsafe { call!(&*s.shared, current_company) } != OWNER_DEITY
        || e.shell.is_null()
        || !unsafe { call!((*s.shared), company_valid, company) }
    {
        return 1;
    }
    if execute {
        crate::witness::hit(crate::witness::ENGINE_CONTROL);
        if args.allow {
            unsafe {
                enable(s, e, company);
            }
        } else {
            unsafe {
                disable(s, e, company);
            }
        }
    }
    0
}
/// `CmdRenameEngine`'s arguments.
#[derive(Clone, Copy)]
struct CmdRenameEngine {
    /// Engine ID to rename, resolved by the caller (a null shell for an invalid id).
    engine_id: EngineRef,
    /// The new name or an empty string when resetting to the default.
    text: ByteSpan,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_rename(
    s: *const Services,
    e: EngineRef,
    name: ByteSpan,
    execute: bool,
) -> u8 {
    cmd_rename_engine(
        unsafe { &*s },
        execute,
        CmdRenameEngine {
            engine_id: e,
            text: name,
        },
    )
}
/// `CmdRenameEngine`: rename an engine. Returns 0 on success, 1 for an invalid engine or
/// too long a name and 2 for a name that is not unique.
fn cmd_rename_engine(s: &Services, execute: bool, args: CmdRenameEngine) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let (e, name) = (args.engine_id, args.text);
    if e.shell.is_null() {
        return 1;
    }
    if name.size != 0 {
        if unsafe { call!((*s.shared), utf8_length, name.data, name.size) }
            >= crate::engine_type::MAX_LENGTH_ENGINE_NAME_CHARS as usize
        {
            return 1;
        }
        let bytes = unsafe { std::slice::from_raw_parts(name.data, name.size) };
        let mut from = 0;
        loop {
            let other = unsafe { call!((*s.types), next, from) };
            if other.shell.is_null() {
                break;
            }
            from = u32::from(other.id) + 1;
            let old = unsafe { &(*other.state).name };
            if !old.is_empty() && old.as_slice() == bytes {
                return 2;
            }
        }
    }
    if execute {
        crate::witness::hit(crate::witness::ENGINE_RENAME);
        unsafe {
            set_name(e.state, name);
            call!(&*s.shared, screen_dirty);
        }
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_check(s: *const Services) {
    check_engines(unsafe { &*s });
}
/// `CheckEngines`: check for engines that have an appropriate availability.
fn check_engines(s: &Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let mut min = i32::MAX;
    let mut from = 0;
    loop {
        let e = unsafe { call!((*s.types), next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if !unsafe { openttd_rust_engine_enabled_abi(s, e.shell) != 0 } || unsafe { wagon(e) } {
            continue;
        }
        if read_field!(e.state, flags) & AVAILABLE != 0 && read_field!(e.state, company_avail) != 0
        {
            return;
        }
        min = min.min(read_field!(e.state, info.base_intro));
    }
    unsafe {
        call!(s, no_engines, min);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEngineRef" ["engine"] => EngineRef {
        shell,
        state,
        id,
        kind => type,
    };
    "OpenTTDEngineYMD" ["engine"] => Ymd {
        year,
        month,
        day,
    };
    "OpenTTDEngineStartSettings" ["engine"] => StartSettings {
        starting_year,
        date,
    };
    "OpenTTDEngineCompany" ["engine"] => Company {
        shell,
        id,
    };
    "OpenTTDEngineVehicle" ["engine"] => Vehicle {
        shell,
        id,
        owner,
        kind => type,
    };
    "OpenTTDEngineVehicleCargo" ["engine"] => VehicleCargo {
        engine,
        cargo,
    };
    "OpenTTDEngineServices" ["engine"] => Services {
        shared,
        finance,
        rng,
        types,
        resolve,
        display_variant,
        engine_id,
        start_settings,
        road_limit,
        ship_limit,
        aircraft_limit,
        ai_train_disabled,
        ai_road_disabled,
        ai_ship_disabled,
        ai_aircraft_disabled,
        clear_last_variant,
        refresh_type,
        refresh_lists,
        company_next,
        vehicle_next,
        vehicle_cargo,
        capabilities,
        company_engine_count,
        local_company,
        toolbar_enable,
        toolbar_release,
        close_preview,
        invalidate_preview,
        offer_preview,
        available_event,
        available_news,
        no_engines,
    };
}

/// Fields observed together by `Engine::CanCarryCargo` before any callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Carrying {
    pub kind: VehicleType,
    pub ground_capacity: u8,
    pub default_cargo: u8,
}
/// Capacity configuration is immutable through `GetVehicleCallback`/`GetEngineProperty`:
/// `SpriteGroup::Resolve` reads parsed specs/variables and writes registers or PSA;
/// it cannot load GRFs, reset engines, execute commands/scripts or yield the mutex.
#[repr(C)]
pub(crate) struct CapacityConfig {
    pub id: u16,
    pub callbacks: u16,
    pub misc: u8,
}

unsafe fn climate(s: &Services, e: *const OpenTTDEngine) -> bool {
    let climate = unsafe { (*s.shared).settings().climate.read() };
    read_field!(e, info.climates) & (1 << climate) != 0
}

/// Original invalid-string short circuit precedes the climate observation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_enabled_abi(
    s: &Services,
    e: *const OpenTTDEngine,
) -> u8 {
    u8::from(unsafe {
        crate::vehicle::EngineRef::of(e).valid_string(s.capabilities.invalid_string_id)
            && climate(s, e)
    })
}
