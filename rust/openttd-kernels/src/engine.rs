/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Engine lifecycle state. Native shells own this allocation through ordinary and indexed
//! construction, temporary staging and vector relocation; native scalar aliases carry their
//! real widths.
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
use crate::raw_field::{read_field, write_field};
use crate::services::ByteSpan;
use crate::timer_game_common::{DAYS_IN_LEAP_YEAR, DAYS_IN_YEAR};
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Lifecycle {
    pub company_avail: u16,
    pub company_hidden: u16,
    pub preview_asked: u16,
    pub intro_date: i32,
    pub age: i32,
    pub reliability: u16,
    pub reliability_spd_dec: u16,
    pub reliability_start: u16,
    pub reliability_max: u16,
    pub reliability_final: u16,
    pub duration_phase_1: u16,
    pub duration_phase_2: u16,
    pub duration_phase_3: u16,
    pub flags: u8,
    pub preview_company: u8,
    pub preview_wait: u8,
}
#[repr(C)]
struct State {
    lifecycle: Lifecycle,
    name: Vec<u8>,
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_new() -> *mut Lifecycle {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    Box::into_raw(Box::new(State {
        lifecycle: Lifecycle {
            preview_company: 255,
            ..Lifecycle::default()
        },
        name: Vec::new(),
    }))
    .cast()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_destroy(state: *mut Lifecycle) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    // SAFETY: The shell transfers its sole live allocation at destruction.
    drop(unsafe { Box::from_raw(state.cast::<State>()) });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_name(state: *const Lifecycle) -> ByteSpan {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    // SAFETY: Caller keeps the live owner stable until the borrowed bytes expire.
    let name = unsafe { &(*state.cast::<State>()).name };
    ByteSpan {
        data: name.as_ptr(),
        size: name.len(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_set_name(state: *mut Lifecycle, text: ByteSpan) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    // SAFETY: Owner is live and serialized; source may alias this owner's name,
    // so copy before replacing. Empty spans may have a null pointer.
    let name = if text.size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(text.data, text.size) }.to_vec()
    };
    unsafe {
        (*state.cast::<State>()).name = name;
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
#[repr(C)]
pub struct SyncVariant {
    pub parent: u16,
    pub sync: bool,
}
#[repr(C)]
pub struct DisplayVariant {
    pub parent: u16,
    pub display: u16,
}
#[repr(C)]
pub struct PreviewVariant {
    pub parent: u16,
    pub join: bool,
}
#[repr(C)]
pub struct Reliability {
    pub base_life: i32,
    pub retire_early: i8,
}
#[repr(C)]
pub struct Startup {
    pub base_intro: i32,
    pub grfid: u32,
    pub base_life: i32,
    pub decay_speed: u8,
    pub climate: bool,
}
#[repr(C)]
pub struct SeedConfig {
    pub base_intro: i32,
    pub lifelength: i32,
    pub decay_speed: u8,
    pub retire_early: i8,
}
#[repr(C)]
pub struct AgingYear {
    pub base_intro: i32,
    pub lifelength: i32,
}
pub use crate::services::CalendarYmd as Ymd;
#[repr(C)]
pub struct StartSettings {
    pub generation_seed: u32,
    pub starting_year: i32,
    pub date: i32,
}
#[repr(C)]
pub struct Company {
    pub shell: *const OpenTTDCompany,
    pub id: u8,
}
#[repr(C)]
pub struct CompanyPreview {
    pub blocked: u8,
    pub performance: i32,
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
    pub rng: *const RandomServices,
    pub next: unsafe extern "C" fn(u32) -> EngineRef,
    pub resolve: unsafe extern "C" fn(u16) -> EngineRef,
    pub sync_variant: unsafe extern "C" fn(*const OpenTTDEngine) -> SyncVariant,
    pub display_variant: unsafe extern "C" fn(*const OpenTTDEngine) -> DisplayVariant,
    pub preview_variant: unsafe extern "C" fn(*const OpenTTDEngine) -> PreviewVariant,
    pub extra_flags: unsafe extern "C" fn(*const OpenTTDEngine) -> u8,
    pub engine_id: unsafe extern "C" fn(*const OpenTTDEngine) -> u16,
    pub reliability: unsafe extern "C" fn(*const OpenTTDEngine) -> Reliability,
    pub startup: unsafe extern "C" fn(*const OpenTTDEngine) -> Startup,
    pub seed_config: unsafe extern "C" fn(*const OpenTTDEngine) -> SeedConfig,
    pub base_intro: unsafe extern "C" fn(*const OpenTTDEngine) -> i32,
    pub aging_year: unsafe extern "C" fn(*const OpenTTDEngine) -> AgingYear,
    pub calendar_year: unsafe extern "C" fn() -> i32,
    pub never_expire: unsafe extern "C" fn() -> bool,
    pub start_settings: unsafe extern "C" fn() -> StartSettings,
    pub train_limit: unsafe extern "C" fn() -> u16,
    pub road_limit: unsafe extern "C" fn() -> u16,
    pub ship_limit: unsafe extern "C" fn() -> u16,
    pub aircraft_limit: unsafe extern "C" fn() -> u16,
    pub ai_train_disabled: unsafe extern "C" fn() -> bool,
    pub ai_road_disabled: unsafe extern "C" fn() -> bool,
    pub ai_ship_disabled: unsafe extern "C" fn() -> bool,
    pub ai_aircraft_disabled: unsafe extern "C" fn() -> bool,
    pub date_to_ymd: unsafe extern "C" fn(i32) -> Ymd,
    pub ymd_to_date: unsafe extern "C" fn(i32, u8, u8) -> i32,
    pub enabled: unsafe extern "C" fn(*const OpenTTDEngine) -> bool,
    pub climate: unsafe extern "C" fn(*const OpenTTDEngine) -> bool,
    pub rail_kind: unsafe extern "C" fn(*const OpenTTDEngine) -> RailVehicleTypes,
    pub clear_last_variant: unsafe extern "C" fn(*const OpenTTDEngine),
    pub refresh_type: unsafe extern "C" fn(VehicleType),
    pub refresh_lists: unsafe extern "C" fn(bool),
    pub company_next: unsafe extern "C" fn(u32) -> Company,
    pub company_preview: unsafe extern "C" fn(*const OpenTTDCompany) -> CompanyPreview,
    pub vehicle_next: unsafe extern "C" fn(u32) -> Vehicle,
    pub vehicle_cargo: unsafe extern "C" fn(*mut OpenTTDVehicle) -> VehicleCargo,
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    pub update_railtypes: unsafe extern "C" fn(u8),
    pub update_roadtypes: unsafe extern "C" fn(u8),
    pub date_company_types: unsafe extern "C" fn(u8, i32),
    pub company_valid: unsafe extern "C" fn(u8) -> bool,
    pub compatible_railtypes: unsafe extern "C" fn(*const OpenTTDEngine) -> u64,
    pub powered_roadtypes: unsafe extern "C" fn(*const OpenTTDEngine) -> u64,
    pub company_railtypes: unsafe extern "C" fn(u8) -> u64,
    pub company_roadtypes: unsafe extern "C" fn(u8) -> u64,
    pub company_engine_count: unsafe extern "C" fn(u8, u16) -> c_uint,
    pub block_preview: unsafe extern "C" fn(u8),
    pub introduces_railtypes: unsafe extern "C" fn(*const OpenTTDEngine) -> u64,
    pub introduces_roadtypes: unsafe extern "C" fn(*const OpenTTDEngine) -> u64,
    pub set_railtypes: unsafe extern "C" fn(u8, u64, i32),
    pub set_roadtypes: unsafe extern "C" fn(u8, u64, i32),
    pub current_company: unsafe extern "C" fn() -> u8,
    pub local_company: unsafe extern "C" fn() -> u8,
    pub toolbar_enable: unsafe extern "C" fn(VehicleType),
    pub toolbar_release: unsafe extern "C" fn(VehicleType),
    pub close_preview: unsafe extern "C" fn(u16),
    pub invalidate_preview: unsafe extern "C" fn(u16),
    pub offer_preview: unsafe extern "C" fn(u16, u8, bool),
    pub available_event: unsafe extern "C" fn(u16),
    pub available_news: unsafe extern "C" fn(u16),
    pub no_engines: unsafe extern "C" fn(i32),
    pub screen_dirty: unsafe extern "C" fn(),
    pub utf8_length: unsafe extern "C" fn(*const u8, usize) -> usize,
}

const AVAILABLE: u8 = 1;
const EXCLUSIVE: u8 = 2;
use crate::company_type::OWNER_DEITY;
use crate::engine_type::INVALID_ENGINE as INVALID;
// Original static year is process-wide; all engine entries run serialized by
// the game-state mutex, including main-thread GUI commands and game timers.
static mut AGING_STOPS: i32 = 0;
macro_rules! call {
    ($s:expr, $field:ident $(, $arg:expr)* $(,)?) => {
        crate::witness::counted!(ENGINE_SERVICES, ENGINE_BYTES, ($s.$field)($($arg),*))
    };
}
unsafe fn parent(s: &Services, mut e: EngineRef) -> EngineRef {
    loop {
        let v = unsafe { call!(s, sync_variant, e.shell) };
        if v.parent == INVALID || !v.sync {
            return e;
        }
        crate::witness::hit(crate::witness::ENGINE_SYNC);
        e = unsafe { call!(s, resolve, v.parent) };
    }
}
unsafe fn clear_last_variant(s: &Services, e: EngineRef) {
    unsafe {
        call!(s, clear_last_variant, e.shell);
    }
}
unsafe fn never_expire(s: &Services) -> bool {
    let enabled = unsafe { call!(s, never_expire) };
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
    let cfg = unsafe { call!(s, reliability, e.shell) };
    let phase1 = u32::from(read_field!(e.state, duration_phase_1));
    let phase2 = u32::from(read_field!(e.state, duration_phase_2));
    let phase3 = u32::from(read_field!(e.state, duration_phase_3));
    if read_field!(e.state, company_avail) != 0
        && !unsafe { never_expire(s) }
        && cfg.base_life != 255
    {
        let early = i32::from(cfg.retire_early);
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
        if age < phase2 || unsafe { never_expire(s) } || cfg.base_life == 255 {
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
unsafe fn start_one(s: &Services, e: EngineRef, aging: Ymd, seed: u32) {
    let cfg = unsafe { call!(s, startup, e.shell) };
    let settings = unsafe { call!(s, start_settings) };
    write_field!(e.state, age, 0);
    write_field!(e.state, flags, 0);
    write_field!(e.state, company_avail, 0);
    write_field!(e.state, company_hidden, 0);
    let rng = unsafe { &*s.rng };
    let shared = unsafe { &*s.shared };
    let saved = unsafe { call!(rng, save) };
    unsafe {
        call!(
            rng,
            set_seed,
            settings.generation_seed
                ^ seed
                ^ cfg.base_intro as u32
                ^ u32::from(e.kind.0)
                ^ cfg.grfid,
        );
    }
    let r = call!(shared, random, shared.context);
    let cutoff = unsafe { call!(s, ymd_to_date, settings.starting_year.wrapping_add(2), 0, 1) };
    let intro = if cfg.base_intro <= cutoff {
        cfg.base_intro
    } else {
        cfg.base_intro.wrapping_add((r & 511) as i32)
    };
    write_field!(e.state, intro_date, intro);
    if intro <= settings.date {
        let ymd = unsafe { call!(s, date_to_ymd, intro) };
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
    let re_cfg = unsafe { call!(s, seed_config, re.shell) };
    unsafe {
        call!(
            rng,
            set_seed,
            settings.generation_seed
                ^ seed
                ^ (u32::from(call!(s, engine_id, re.shell)) << 16)
                ^ (re_cfg.base_intro as u32).wrapping_shl(12)
                ^ (u32::from(re_cfg.decay_speed) << 8)
                ^ (re_cfg.lifelength as u32).wrapping_shl(4)
                ^ (i32::from(re_cfg.retire_early) as u32)
                ^ u32::from(e.kind.0)
                ^ cfg.grfid,
        );
    }
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
        cfg.base_life
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
        u16::from(cfg.decay_speed) << 2
    );
    if !cfg.climate {
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
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let mut year = 2050;
    let mut from = 0;
    loop {
        let e = unsafe { call!(s, next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if !unsafe { call!(s, climate, e.shell) } || unsafe { wagon(s, e) } {
            continue;
        }
        let cfg = unsafe { call!(s, aging_year, e.shell) };
        let date = cfg
            .base_intro
            .wrapping_add(cfg.lifelength.wrapping_mul(DAYS_IN_LEAP_YEAR) / 2);
        year = year.max(unsafe { call!(s, date_to_ymd, date) }.year);
    }
    unsafe {
        AGING_STOPS = year;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_start_all(s: *const Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let date = unsafe { call!(&*s.shared, calendar_date) }
        .min(unsafe { call!(s, ymd_to_date, AGING_STOPS, 0, 1) });
    let aging = unsafe { call!(s, date_to_ymd, date) };
    let shared = unsafe { &*s.shared };
    let seed = call!(shared, random, shared.context);
    let mut from = 0;
    loop {
        let e = unsafe { call!(s, next, from) };
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
        let e = unsafe { call!(s, next, from) };
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
            call!(s, update_railtypes, c.id);
            call!(s, update_roadtypes, c.id);
        }
    }
    unsafe {
        call!(s, refresh_lists, true);
    }
}
unsafe fn wagon(s: &Services, e: EngineRef) -> bool {
    e.kind == VehicleType::TRAIN
        && unsafe { call!(s, rail_kind, e.shell) } == RailVehicleTypes::WAGON
}
unsafe fn disabled(s: &Services, kind: VehicleType, ai: bool) -> bool {
    match kind {
        VehicleType::TRAIN => unsafe {
            call!(s, train_limit) == 0 || (ai && call!(s, ai_train_disabled))
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
            call!(s, update_railtypes, company);
        } else if e.kind == VehicleType::ROAD {
            call!(s, update_roadtypes, company);
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
            call!(s, update_railtypes, company);
        } else if e.kind == VehicleType::ROAD {
            call!(s, update_roadtypes, company);
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
        let ve = unsafe { call!(s, next, from) };
        if ve.shell.is_null() {
            break;
        }
        from = u32::from(ve.id) + 1;
        if ve.kind != e.kind {
            continue;
        }
        let v = unsafe { call!(s, preview_variant, ve.shell) };
        if ve.id != e.id && v.parent == e.id && v.join {
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
        let preview = unsafe { call!(s, company_preview, c.shell) };
        if preview.blocked != 0
            || read_field!(e.state, preview_asked) & (1 << c.id) != 0
            || preview.performance <= performance
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
                crate::engine_capabilities::openttd_rust_engine_can_carry(
                    s.capabilities,
                    cargo.engine,
                )
            } || mask & 1u64.wrapping_shl(u32::from(cargo.cargo)) == 0
            {
                continue;
            }
            performance = preview.performance;
            best = c.id;
            break;
        }
    }
    best
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_daily(s: *const Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let date = unsafe { call!(&*s.shared, calendar_date) };
    let mut from = 0;
    loop {
        let c = unsafe { call!(s, company_next, from) };
        if c.id == 255 {
            break;
        }
        from = u32::from(c.id) + 1;
        unsafe {
            call!(s, date_company_types, c.id, date);
        }
    }
    if unsafe { call!(s, calendar_year) } >= unsafe { AGING_STOPS } {
        crate::witness::hit(crate::witness::ENGINE_CUTOFF);
        return;
    }
    from = 0;
    loop {
        let e = unsafe { call!(s, next, from) };
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
                unsafe {
                    crate::witness::hit(crate::witness::ENGINE_PENALTY);
                    call!(s, block_preview, c.id);
                }
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
    if unsafe { wagon(s, e) } {
        return;
    }
    if e.kind == VehicleType::TRAIN {
        let introduced = unsafe { call!(s, introduces_railtypes, e.shell) };
        let mut from = 0;
        loop {
            let c = unsafe { call!(s, company_next, from) };
            if c.id == 255 {
                break;
            }
            from = u32::from(c.id) + 1;
            let types = unsafe { call!(s, company_railtypes, c.id) } | introduced;
            unsafe {
                call!(s, set_railtypes, c.id, types, date);
            }
        }
    } else if e.kind == VehicleType::ROAD {
        let introduced = unsafe { call!(s, introduces_roadtypes, e.shell) };
        let mut from = 0;
        loop {
            let c = unsafe { call!(s, company_next, from) };
            if c.id == 255 {
                break;
            }
            from = u32::from(c.id) + 1;
            let types = unsafe { call!(s, company_roadtypes, c.id) } | introduced;
            unsafe {
                call!(s, set_roadtypes, c.id, types, date);
            }
        }
    }
    if !unsafe { disabled(s, e.kind, true) } {
        unsafe {
            call!(s, available_event, e.id);
        }
    }
    if !unsafe { disabled(s, e.kind, false) } && unsafe { call!(s, extra_flags, e.shell) } & 1 == 0
    {
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
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    if unsafe { call!(s, calendar_year) } >= unsafe { AGING_STOPS } {
        crate::witness::hit(crate::witness::ENGINE_CUTOFF);
        return;
    }
    let date = unsafe { call!(&*s.shared, calendar_date) };
    let mut refresh = false;
    let mut from = 0;
    loop {
        let e = unsafe { call!(s, next, from) };
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
        if !unsafe { call!(s, enabled, e.shell) } {
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
                || unsafe { wagon(s, e) }
                || unsafe { call!(s, extra_flags, e.shell) } & 2 != 0
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
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let mut from = 0;
    loop {
        let e = unsafe { call!(s, next, from) };
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
    if !unsafe { call!(s, enabled, e.shell) } {
        return false;
    }
    if company != OWNER_DEITY {
        if kind == VehicleType::TRAIN
            && unsafe {
                call!(s, compatible_railtypes, e.shell) & call!(s, company_railtypes, company)
            } == 0
        {
            return false;
        }
        if kind == VehicleType::ROAD
            && unsafe {
                call!(s, powered_roadtypes, e.shell) & call!(s, company_roadtypes, company)
            } == 0
        {
            return false;
        }
    }
    true
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_buildable(
    s: *const Services,
    e: EngineRef,
    kind: VehicleType,
    company: u8,
) -> bool {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    unsafe { buildable(&*s, e, kind, company) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_variant_hidden(
    s: *const Services,
    e: EngineRef,
    company: u8,
) -> bool {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    if company >= 15 {
        return false;
    }
    if read_field!(e.state, company_hidden) & (1 << company) != 0 {
        return true;
    }
    let s = unsafe { &*s };
    let mut re = e;
    loop {
        let variant = unsafe { call!(s, display_variant, re.shell) };
        let ve = if variant.display == re.id || variant.display == INVALID {
            re
        } else {
            unsafe { call!(s, resolve, variant.display) }
        };
        let hidden = read_field!(ve.state, company_hidden) & (1 << company) != 0;
        if hidden || variant.parent == INVALID {
            return hidden;
        }
        re = unsafe { call!(s, resolve, variant.parent) };
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
    let company = unsafe { call!(s, current_company) };
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
    let company = unsafe { call!(s, current_company) };
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_control(
    s: *const Services,
    e: EngineRef,
    company: u8,
    allow: bool,
    execute: bool,
) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    if unsafe { call!(s, current_company) } != OWNER_DEITY
        || e.shell.is_null()
        || !unsafe { call!(s, company_valid, company) }
    {
        return 1;
    }
    if execute {
        crate::witness::hit(crate::witness::ENGINE_CONTROL);
        if allow {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_rename(
    s: *const Services,
    e: EngineRef,
    name: ByteSpan,
    execute: bool,
) -> u8 {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    if e.shell.is_null() {
        return 1;
    }
    let s = unsafe { &*s };
    if name.size != 0 {
        if unsafe { call!(s, utf8_length, name.data, name.size) } >= 32 {
            return 1;
        }
        let bytes = unsafe { std::slice::from_raw_parts(name.data, name.size) };
        let mut from = 0;
        loop {
            let other = unsafe { call!(s, next, from) };
            if other.shell.is_null() {
                break;
            }
            from = u32::from(other.id) + 1;
            let old = unsafe { openttd_rust_engine_name(other.state) };
            if old.size != 0 && unsafe { std::slice::from_raw_parts(old.data, old.size) } == bytes {
                return 2;
            }
        }
    }
    if execute {
        crate::witness::hit(crate::witness::ENGINE_RENAME);
        unsafe {
            openttd_rust_engine_set_name(e.state, name);
            call!(s, screen_dirty);
        }
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_check(s: *const Services) {
    crate::witness::hit(crate::witness::ENGINE_ENTRIES);
    let s = unsafe { &*s };
    let mut min = i32::MAX;
    let mut from = 0;
    loop {
        let e = unsafe { call!(s, next, from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if !unsafe { call!(s, enabled, e.shell) } || unsafe { wagon(s, e) } {
            continue;
        }
        if read_field!(e.state, flags) & AVAILABLE != 0 && read_field!(e.state, company_avail) != 0
        {
            return;
        }
        min = min.min(unsafe { call!(s, base_intro, e.shell) });
    }
    unsafe {
        call!(s, no_engines, min);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEngineLifecycle" ["engine"] => Lifecycle {
        company_avail,
        company_hidden,
        preview_asked,
        intro_date,
        age,
        reliability,
        reliability_spd_dec,
        reliability_start,
        reliability_max,
        reliability_final,
        duration_phase_1,
        duration_phase_2,
        duration_phase_3,
        flags,
        preview_company,
        preview_wait,
    };
    "OpenTTDEngineRef" ["engine"] => EngineRef {
        shell,
        state,
        id,
        kind => type,
    };
    "OpenTTDEngineSyncVariant" ["engine"] => SyncVariant {
        parent,
        sync,
    };
    "OpenTTDEngineDisplayVariant" ["engine"] => DisplayVariant {
        parent,
        display,
    };
    "OpenTTDEnginePreviewVariant" ["engine"] => PreviewVariant {
        parent,
        join,
    };
    "OpenTTDEngineReliability" ["engine"] => Reliability {
        base_life,
        retire_early,
    };
    "OpenTTDEngineStartup" ["engine"] => Startup {
        base_intro,
        grfid,
        decay_speed,
        base_life,
        climate,
    };
    "OpenTTDEngineSeed" ["engine"] => SeedConfig {
        base_intro,
        lifelength,
        decay_speed,
        retire_early,
    };
    "OpenTTDEngineYear" ["engine"] => AgingYear {
        base_intro,
        lifelength,
    };
    "OpenTTDEngineYMD" ["engine"] => Ymd {
        year,
        month,
        day,
    };
    "OpenTTDEngineStartSettings" ["engine"] => StartSettings {
        generation_seed,
        starting_year,
        date,
    };
    "OpenTTDEngineCompany" ["engine"] => Company {
        shell,
        id,
    };
    "OpenTTDEngineCompanyPreview" ["engine"] => CompanyPreview {
        blocked,
        performance,
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
    "OpenTTDEngineCarrying" ["engine"] => Carrying { kind, ground_capacity, default_cargo };
    "OpenTTDEngineCapacityConfig" ["engine"] => CapacityConfig { id, callbacks, misc };
    "OpenTTDEngineServices" ["engine"] => Services {
        shared,
        rng,
        next,
        resolve,
        sync_variant,
        display_variant,
        preview_variant,
        extra_flags,
        engine_id,
        reliability,
        startup,
        seed_config,
        base_intro,
        aging_year,
        calendar_year,
        never_expire,
        start_settings,
        train_limit,
        road_limit,
        ship_limit,
        aircraft_limit,
        ai_train_disabled,
        ai_road_disabled,
        ai_ship_disabled,
        ai_aircraft_disabled,
        date_to_ymd,
        ymd_to_date,
        enabled,
        climate,
        rail_kind,
        clear_last_variant,
        refresh_type,
        refresh_lists,
        company_next,
        company_preview,
        vehicle_next,
        vehicle_cargo,
        capabilities,
        update_railtypes,
        update_roadtypes,
        date_company_types,
        company_valid,
        compatible_railtypes,
        powered_roadtypes,
        company_railtypes,
        company_roadtypes,
        company_engine_count,
        block_preview,
        introduces_railtypes,
        introduces_roadtypes,
        set_railtypes,
        set_roadtypes,
        current_company,
        local_company,
        toolbar_enable,
        toolbar_release,
        close_preview,
        invalidate_preview,
        offer_preview,
        available_event,
        available_news,
        no_engines,
        screen_dirty,
        utf8_length,
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
