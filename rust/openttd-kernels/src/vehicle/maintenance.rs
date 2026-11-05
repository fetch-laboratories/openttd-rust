/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Base-vehicle maintenance policy over shared records and unmoved native fields.
//! No world borrow survives callbacks, effects or news.
#![allow(
    unsafe_code,
    clippy::struct_excessive_bools,
    clippy::struct_field_names
)]
use super::{
    Breakdown, OT_GOTO_DEPOT, OT_LOADING, OrderType, VehicleRef, VehicleServices, VehicleType,
};
use crate::newgrf_sound::VSE_BREAKDOWN;
use crate::timer_game_common::DAYS_IN_LEAP_YEAR;
use crate::vehicle_base::{VS_CRASHED, VS_HIDDEN, VS_STOPPED};
use crate::{company::OpenTTDCompany, fleet, services};

use crate::services::{CalendarDate, EconomyDate};

#[repr(C)]
pub struct Renewal {
    pub age: CalendarDate,
    pub max_age: CalendarDate,
    pub months: i16,
    pub enabled: bool,
}
#[repr(C)]
pub struct Reset {
    pub economy_date: EconomyDate,
    pub calendar_date: CalendarDate,
    pub reliability: u16,
    pub chance: u8,
    pub breakdowns: u8,
}
#[repr(C)]
pub struct Interval {
    pub company: *const OpenTTDCompany,
    pub last_service: EconomyDate,
    pub now: EconomyDate,
    pub interval: u16,
    pub reliability: u16,
    pub percent: bool,
    pub wallclock: bool,
    pub no_service: bool,
    pub breakdowns: u8,
}
#[repr(C)]
pub struct Replacement {
    pub engine: u16,
    pub when_old: bool,
}

#[repr(C)]
pub struct Automatic {
    pub has_depot_order: bool,
    pub order_type: OrderType,
}
#[repr(C)]
pub struct CheckBreakdown {
    pub reliability: u16,
    pub decay: u16,
    pub no_service: bool,
    pub breakdowns: u8,
}
#[repr(C)]
pub struct BreakdownGuard {
    pub breakdowns: u8,
    pub menu: bool,
}
#[repr(C)]
pub struct BreakdownTrigger {
    pub reliability: u16,
    pub chance: u8,
    pub breakdowns: u8,
}
#[repr(C)]
pub struct Age {
    pub age: CalendarDate,
    pub max_age: CalendarDate,
    pub decay: u16,
}
#[repr(C)]
pub struct Warning {
    pub enabled: bool,
    pub local_company: u8,
}
#[repr(C)]
pub struct WarningCompany {
    pub company: *const OpenTTDCompany,
    pub renewal: bool,
}

/// Each callback is a named shared service, observation or consecutive scalar
/// store. Tables are immutable and outlive the synchronous native/Rust entry.
#[repr(C)]
pub struct Services {
    pub shared: *const services::Services,
    pub transaction: *const fleet::transactions::Services,
    pub group: *const fleet::GroupServices,
    pub company: *const crate::company_identity::Services,
    /// Company pointers must name the live owner for the synchronous call.
    pub renewal: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> Renewal,
    pub reset: extern "C" fn(VehicleRef) -> Reset,
    pub interval: extern "C" fn(VehicleRef) -> Interval,
    pub engine_reliability: extern "C" fn(VehicleRef) -> u16,
    pub replacement: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> Replacement,
    pub replacement_available: extern "C" fn(VehicleRef, u16) -> bool,
    pub automatic: extern "C" fn(VehicleRef) -> Automatic,
    pub depot_service: extern "C" fn(VehicleRef) -> bool,
    pub check_breakdown: extern "C" fn(VehicleRef) -> CheckBreakdown,
    pub breakdown_guard: extern "C" fn(VehicleRef) -> BreakdownGuard,
    pub breakdown_trigger: extern "C" fn(VehicleRef) -> BreakdownTrigger,
    pub chance_write: extern "C" fn(VehicleRef, u8),
    pub breakdown_delay_write: extern "C" fn(VehicleRef, u8),
    pub smoke_disabled: extern "C" fn(VehicleRef) -> bool,
    pub smoke_delay: extern "C" fn(VehicleRef) -> u8,
    pub mark_dirty: extern "C" fn(VehicleRef),
    pub view_dirty: extern "C" fn(VehicleRef),
    pub details_dirty: extern "C" fn(VehicleRef),
    pub age: extern "C" fn(VehicleRef) -> Age,
    pub age_write: extern "C" fn(VehicleRef, CalendarDate),
    pub decay_write: extern "C" fn(VehicleRef, u16),
    pub warning: extern "C" fn(VehicleRef) -> Warning,
    pub warning_company: extern "C" fn(VehicleRef) -> WarningCompany,
    pub engine_available_any: extern "C" fn(VehicleRef) -> bool,
    pub replacement_active: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> bool,
    pub old_news: extern "C" fn(VehicleRef, u32),
    pub economy_age: extern "C" fn(VehicleRef) -> EconomyDate,
    pub economy_age_write: extern "C" fn(VehicleRef, EconomyDate),
    pub reached_min_age: extern "C" fn(VehicleRef),
    pub value: extern "C" fn(VehicleRef) -> i64,
    pub value_write: extern "C" fn(VehicleRef, i64),
    pub max_calendar_date: CalendarDate,
    pub max_economy_date: EconomyDate,
    pub profit_min_age: EconomyDate,
    pub anniversaries: [CalendarDate; 5],
    pub before_zero: CalendarDate,
    pub news_getting_old: u32,
    pub news_very_old: u32,
    pub news_very_old_and: u32,
}

macro_rules! svc {
    ($counted:ident, $function:expr $(, $arg:expr)*) => {
        crate::witness::counted!(
            VEHICLE_MAINT_CROSSINGS, VEHICLE_MAINT_BYTES if $counted, ($function)($($arg),*)
        )
    };
}
macro_rules! branch {
    ($counted:ident, $name:ident) => {
        if $counted {
            crate::witness::hit(crate::witness::$name);
        }
    };
}

unsafe fn renew<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    company: *const OpenTTDCompany,
    use_setting: bool,
) -> bool {
    let s = unsafe { &*base.maintenance };
    // SAFETY: the native entry supplies the live vehicle owner.
    let r = unsafe { svc!(C, s.renewal, v, company) };
    branch!(C, VEHICLE_MAINT_RENEW);
    if use_setting && !r.enabled {
        return false;
    }
    if r.age.0.wrapping_sub(r.max_age.0) < i32::from(r.months) * 30 {
        return false;
    }
    v.vehicle_type() != VehicleType::TRAIN.0 || v.subtype() & 8 != 0
}
unsafe fn reset<const C: bool>(mut v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    svc!(C, s.details_dirty, v);
    loop {
        let r = svc!(C, s.reset, v);
        let chance = if r.breakdowns == 1 {
            branch!(C, VEHICLE_MAINT_RESET_REDUCED);
            0
        } else {
            r.chance / 4
        };
        svc!(C, base.write_economy_service, v, r.economy_date.0);
        svc!(C, base.write_calendar_service, v, r.calendar_date.0);
        svc!(C, base.write_breakdowns, v, 0);
        svc!(C, base.write_reliability, v, r.reliability);
        svc!(C, s.chance_write, v, chance);
        branch!(C, VEHICLE_MAINT_RESET);
        let Some(next) = v.next() else {
            break;
        };
        if !next.has_engine_type() {
            break;
        }
        v = next;
    }
}
// The supplied shell stays live; resolve its canonical finance owner afresh after callbacks.
unsafe fn available_money(s: &Services, company: *const OpenTTDCompany) -> i64 {
    let commands = unsafe { &*(*s.transaction).commands };
    let companies = unsafe { &*s.company };
    let id = (companies.company_id)(company.cast_mut());
    let owner = (commands.finance.company_finances)(id.0);
    let infinite = (commands.infinite_money)();
    unsafe { crate::company::openttd_rust_company_available_money(owner, infinite) }
}
unsafe fn needs_service<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    let s = unsafe { &*base.maintenance };
    if (v).status() & (VS_STOPPED | VS_CRASHED) != 0 {
        return false;
    }
    let r = svc!(C, s.interval, v);
    branch!(C, VEHICLE_MAINT_SERVICE);
    if r.percent {
        branch!(C, VEHICLE_MAINT_PERCENT);
        let reliability = svc!(C, s.engine_reliability, v);
        if i32::from(r.reliability)
            >= i32::from(reliability).wrapping_mul(100 - i32::from(r.interval)) / 100
        {
            return false;
        }
    } else {
        let interval = i32::from(r.interval)
            * if r.wallclock {
                branch!(C, VEHICLE_MAINT_MINUTES);
                30
            } else {
                branch!(C, VEHICLE_MAINT_DAYS);
                1
            };
        if r.last_service.0.wrapping_add(interval) >= r.now.0 {
            return false;
        }
    }
    if !r.no_service || r.breakdowns != 0 {
        return true;
    }
    // SAFETY: interval resolved the live owner; no callback can destroy it here.
    let companies = unsafe { &*s.company };
    let mut needed = i64::from((companies.renew_reserve)(r.company));
    if needed > unsafe { available_money(s, r.company) } {
        return false;
    }
    let mut pending = false;
    let mut current = Some(v);
    while let Some(unit) = current {
        if unsafe { replacement_needed::<C>(unit, base, r.company, &mut needed, &mut pending) } {
            return false;
        }
        current = if unit.vehicle_type() == VehicleType::TRAIN.0 {
            unit.next_unit()
        } else {
            None
        };
    }
    pending
}
unsafe fn replacement_needed<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    company: *const OpenTTDCompany,
    needed: &mut i64,
    pending: &mut bool,
) -> bool {
    let s = unsafe { &*base.maintenance };
    // SAFETY: needs_service retains the live native owner through this decision.
    let replacement = unsafe { svc!(C, s.replacement, v, company) };
    if replacement.engine == crate::engine_type::INVALID_ENGINE
        || !svc!(C, s.replacement_available, v, replacement.engine)
    {
        return false;
    }
    if replacement.when_old && !unsafe { renew::<C>(v, base, company, false) } {
        return false;
    }
    let t = unsafe { &*s.transaction };
    let (mut union, mut available) = (0_u64, 0_u64);
    unsafe {
        crate::engine_capabilities::openttd_rust_articulated_refit_masks(
            t.capabilities,
            replacement.engine,
            true,
            &raw mut union,
            &raw mut available,
        );
    };
    if C {
        crate::witness::add(
            crate::witness::VEHICLE_MAINT_BYTES,
            (std::mem::size_of_val(&union) + std::mem::size_of_val(&available)) as u64,
        );
    }
    if union != 0 {
        let mut cargo_type = 0;
        let cargo = unsafe {
            crate::engine_capabilities::openttd_rust_articulated_live_cargoes(
                t.capabilities,
                v.0.as_ptr(),
                &raw mut cargo_type,
            )
        };
        if C {
            crate::witness::add(
                crate::witness::VEHICLE_MAINT_BYTES,
                std::mem::size_of_val(&cargo_type) as u64,
            );
        }
        if cargo & cargo.wrapping_sub(1) != 0 {
            let defaults = unsafe {
                crate::engine_capabilities::openttd_rust_articulated_cargoes(
                    t.capabilities,
                    replacement.engine,
                )
            };
            if cargo & defaults != cargo {
                return false;
            }
        } else if cargo_type < 64 && available & (1_u64 << cargo_type) == 0 {
            return false;
        }
    }
    *pending = true;
    // SAFETY: the same live owner remains valid after the bounded refit queries.
    let cap = t.capabilities;
    let cost = unsafe {
        crate::engine_capabilities::openttd_rust_engine_cost(cap, (cap.engine)(replacement.engine))
    };
    *needed = needed.saturating_add(cost.saturating_mul(2));
    let available = unsafe { available_money(s, company) };
    branch!(C, VEHICLE_MAINT_REPLACE);
    *needed > available
}
unsafe fn automatic<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    let s = unsafe { &*base.maintenance };
    let r = svc!(C, s.automatic, v);
    if r.has_depot_order
        || r.order_type.0 == OT_LOADING
        || (r.order_type.0 == OT_GOTO_DEPOT && !svc!(C, s.depot_service, v))
    {
        return false;
    }
    unsafe { needs_service::<C>(v, base) }
}

const BREAKDOWN_CHANCE: [u8; 64] = [
    3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 13, 13, 13, 13,
    14, 15, 16, 17, 19, 21, 25, 28, 31, 34, 37, 40, 44, 48, 52, 56, 60, 64, 68, 72, 80, 90, 100,
    110, 120, 130, 140, 150, 170, 190, 210, 230, 250, 250, 250,
];
unsafe fn check_breakdown<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    let r = svc!(C, s.check_breakdown, v);
    branch!(C, VEHICLE_MAINT_CHECK);
    let mut reliability = r.reliability;
    if !r.no_service || r.breakdowns != 0 {
        reliability = reliability.saturating_sub(r.decay);
        svc!(C, base.write_reliability, v, reliability);
        if r.reliability >> 8 != reliability >> 8 {
            svc!(C, s.details_dirty, v);
        }
    }
    let guard = svc!(C, s.breakdown_guard, v);
    if v.breakdown_counter() != 0
        || v.status() & VS_STOPPED != 0
        || guard.breakdowns < 1
        || v.speed() < 5
        || guard.menu
    {
        return;
    }
    let shared = unsafe { &*s.shared };
    let random = svc!(C, shared.random, shared.context);
    let trigger = svc!(C, s.breakdown_trigger, v);
    let mut chance = u16::from(trigger.chance) + 1;
    if crate::random_func::chance16_i(1, 25, random) {
        chance += 25;
    }
    let chance = chance.min(255) as u8;
    svc!(C, s.chance_write, v, chance);
    let reliability = u32::from(trigger.reliability)
        + if v.vehicle_type() == VehicleType::SHIP.0 {
            0x6666
        } else {
            0
        }
        + if trigger.breakdowns == 1 { 0x6666 } else { 0 };
    if BREAKDOWN_CHANCE[(reliability.min(65535) >> 10) as usize] <= chance {
        v.write_breakdown_counter(((random >> 16) & 63) as u8 + 0x3f);
        svc!(
            C,
            s.breakdown_delay_write,
            v,
            ((random >> 24) & 127) as u8 + 0x80
        );
        svc!(C, s.chance_write, v, 0);
        branch!(C, VEHICLE_MAINT_TRIGGER);
    }
}

/// Only consumed value observations survive callbacks. Each later observation is
/// reacquired at its source point after sound, effect allocation and dirty work.
unsafe fn breakdown_state<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    counter: u8,
) -> bool {
    if counter == 0 {
        return false;
    }
    let s = unsafe { &*base.maintenance };
    match counter {
        2 => {
            v.write_breakdown_counter(1);
            let resolver = unsafe { &*(base.newgrf)() };
            let since = unsafe { (resolver.breakdowns)(v.0.as_ptr()) };
            svc!(C, base.write_breakdowns, v, since.saturating_add(1));
            branch!(C, VEHICLE_MAINT_STARTED);
            if v.vehicle_type() == VehicleType::AIRCRAFT.0 {
                v.write_status(v.status() | crate::vehicle_base::VS_AIRCRAFT_BROKEN);
                branch!(C, VEHICLE_MAINT_AIRCRAFT);
            } else {
                (v).write_speed(0);
                if !svc!(C, base.play_vehicle_sound, v, VSE_BREAKDOWN) {
                    let kind = VehicleType(v.vehicle_type());
                    let train_ship = kind == VehicleType::TRAIN || kind == VehicleType::SHIP;
                    let shared = unsafe { &*s.shared };
                    let toyland = unsafe { shared.settings().climate.read() }
                        == services::LandscapeType::TOYLAND.0;
                    let sound = if toyland {
                        if train_ship { 58 } else { 53 }
                    } else if train_ship {
                        14
                    } else {
                        13
                    };
                    svc!(C, base.play_sound, v, sound);
                }
                if (v).status() & VS_HIDDEN == 0 && !svc!(C, s.smoke_disabled, v) {
                    let smoke = crate::effect::openttd_rust_effect_create_rel(
                        v.0.as_ptr(),
                        4,
                        4,
                        5,
                        crate::effectvehicle_func::EV_BREAKDOWN_SMOKE,
                    );
                    let delay = svc!(C, s.smoke_delay, v);
                    if !smoke.is_null() {
                        crate::effect::set_animation(smoke, u16::from(delay) * 2);
                        branch!(C, VEHICLE_MAINT_SMOKE);
                    }
                }
            }
            svc!(C, s.mark_dirty, v);
            svc!(C, s.view_dirty, v);
            svc!(C, s.details_dirty, v);
        }
        1 => {}
        _ => {
            if v.current_order().kind() == OT_LOADING {
                branch!(C, VEHICLE_MAINT_LOADING);
            } else {
                v.write_breakdown_counter(v.breakdown_counter().wrapping_sub(1));
                branch!(C, VEHICLE_MAINT_COUNTDOWN);
            }
            return false;
        }
    }
    let kind = VehicleType(v.vehicle_type());
    if kind == VehicleType::AIRCRAFT {
        return false;
    }
    if v.tick() & if kind == VehicleType::TRAIN { 3 } else { 1 } == 0 {
        let delay = svc!(C, s.smoke_delay, v).wrapping_sub(1);
        svc!(C, s.breakdown_delay_write, v, delay);
        branch!(C, VEHICLE_MAINT_DELAY);
        if delay == 0 {
            (v).write_breakdown_counter(0);
            svc!(C, s.mark_dirty, v);
            svc!(C, s.view_dirty, v);
            branch!(C, VEHICLE_MAINT_ENDED);
        }
    }
    true
}
unsafe fn breakdown_result<const C: bool>(v: VehicleRef, base: &VehicleServices) -> Breakdown {
    let counter = v.breakdown_counter();
    if counter == 0 {
        return Breakdown {
            broken: false,
            status: v.status(),
        };
    }
    let broken = unsafe { breakdown_state::<C>(v, base, counter) };
    Breakdown {
        broken,
        status: (v).status(),
    }
}
/// Direct shared Rust policy for controllers that consume both source results.
/// # Safety
/// A live serialized shell and immutable native tables for the call.
#[inline]
pub(crate) unsafe fn handle_breakdown<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
) -> Breakdown {
    unsafe { breakdown_result::<C>(v, base) }
}
/// `HandleBreakdown()`, then the status it leaves, selecting the witness once.
/// # Safety
/// As `handle_breakdown`.
pub(crate) unsafe fn handle_breakdown_status(v: VehicleRef, base: &VehicleServices) -> Breakdown {
    if crate::witness::enabled() {
        unsafe { breakdown_result::<true>(v, base) }
    } else {
        unsafe { breakdown_result::<false>(v, base) }
    }
}
unsafe fn age<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    let r = svc!(C, s.age, v);
    let age = if r.age.0 < s.max_calendar_date.0 {
        r.age.0 + 1
    } else {
        branch!(C, VEHICLE_MAINT_MAX_AGE);
        r.age.0
    };
    if r.age.0 < s.max_calendar_date.0 {
        svc!(C, s.age_write, v, CalendarDate(age));
    }
    let g = unsafe { &*s.group };
    if !unsafe { svc!(C, g.primary, v.shell().cast()) }
        && (v.vehicle_type() != VehicleType::TRAIN.0 || v.subtype() & 8 == 0)
    {
        return;
    }
    let difference = age.wrapping_sub(r.max_age.0);
    for anniversary in s.anniversaries {
        if difference == anniversary.0 {
            svc!(C, s.decay_write, v, r.decay.wrapping_shl(1));
            branch!(C, VEHICLE_MAINT_DECAY);
            break;
        }
    }
    svc!(C, s.details_dirty, v);
    branch!(C, VEHICLE_MAINT_AGE);
    let warning = svc!(C, s.warning, v);
    if !warning.enabled
        || v.previous().is_some()
        || v.owner() != warning.local_company
        || v.status() & (VS_CRASHED | VS_STOPPED) != 0
    {
        return;
    }
    let company = svc!(C, s.warning_company, v);
    if company.renewal && svc!(C, s.engine_available_any, v) {
        return;
    }
    // SAFETY: warning_company supplies the live owner; availability only reads.
    if unsafe { svc!(C, s.replacement_active, v, company.company) } {
        return;
    }
    let news = if difference == s.before_zero.0 {
        s.news_getting_old
    } else if difference == s.anniversaries[0].0 {
        s.news_very_old
    } else if difference > s.anniversaries[0].0 && difference % DAYS_IN_LEAP_YEAR == 0 {
        s.news_very_old_and
    } else {
        return;
    };
    svc!(C, s.old_news, v, news);
    branch!(C, VEHICLE_MAINT_OLD_NEWS);
}
unsafe fn economy_age<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    let r = svc!(C, s.economy_age, v);
    if r.0 < s.max_economy_date.0 {
        let age = EconomyDate(r.0 + 1);
        svc!(C, s.economy_age_write, v, age);
        let g = unsafe { &*s.group };
        if unsafe { svc!(C, g.primary, v.shell().cast()) } && age.0 == s.profit_min_age.0 + 1 {
            svc!(C, s.reached_min_age, v);
            branch!(C, VEHICLE_MAINT_PROFIT_AGE);
        }
    } else {
        branch!(C, VEHICLE_MAINT_MAX_AGE);
    }
}
unsafe fn decrease_value<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    let value = svc!(C, s.value, v);
    svc!(C, s.value_write, v, value.saturating_sub(value >> 8));
    svc!(C, s.details_dirty, v);
    branch!(C, VEHICLE_MAINT_VALUE);
}
macro_rules! entry {
    ($name:ident, $direct:ident, $body:ident) => {
        /// Direct Rust call for the vehicle ports; the native facade calls the same.
        /// # Safety
        /// A live serialized shell and immutable tables; no borrow into canonical
        /// storage survives any synchronous callback/reentry.
        pub(crate) unsafe fn $direct(v: VehicleRef, base: &VehicleServices) {
            if crate::witness::enabled() {
                crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
                unsafe { $body::<true>(v, base) }
            } else {
                unsafe { $body::<false>(v, base) }
            }
        }
        /// # Safety
        /// As the direct call; `base` is the live immutable native table.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(v: VehicleRef, base: *const VehicleServices) {
            unsafe { $direct(v, &*base) }
        }
    };
}
entry!(openttd_rust_vehicle_service_depot, service_in_depot, reset);
entry!(openttd_rust_vehicle_age, age_vehicle, age);
entry!(
    openttd_rust_vehicle_economy_age,
    economy_age_vehicle,
    economy_age
);
entry!(
    openttd_rust_vehicle_decrease_value,
    decrease_vehicle_value,
    decrease_value
);
entry!(
    openttd_rust_vehicle_check_breakdown,
    check_vehicle_breakdown,
    check_breakdown
);
/// # Safety
/// As the maintenance entries; company is the live native owner at source scope.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_renew_abi(
    v: VehicleRef,
    base: *const VehicleServices,
    company: *const OpenTTDCompany,
    settings: bool,
) -> u8 {
    u8::from(if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { renew::<true>(v, &*base, company, settings) }
    } else {
        unsafe { renew::<false>(v, &*base, company, settings) }
    })
}
/// `Vehicle::NeedsServicing`.
/// # Safety
/// As the direct maintenance calls.
pub(crate) unsafe fn needs_servicing(v: VehicleRef, base: &VehicleServices) -> bool {
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { needs_service::<true>(v, base) }
    } else {
        unsafe { needs_service::<false>(v, base) }
    }
}
/// # Safety
/// As the maintenance entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_service_abi(
    v: VehicleRef,
    base: *const VehicleServices,
) -> u8 {
    u8::from(unsafe { needs_servicing(v, &*base) })
}
/// `Vehicle::NeedsAutomaticServicing`.
/// # Safety
/// As the direct maintenance calls.
pub(crate) unsafe fn needs_automatic_servicing(v: VehicleRef, base: &VehicleServices) -> bool {
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { automatic::<true>(v, base) }
    } else {
        unsafe { automatic::<false>(v, base) }
    }
}
/// # Safety
/// As the maintenance entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_automatic_service_abi(
    v: VehicleRef,
    base: *const VehicleServices,
) -> u8 {
    u8::from(unsafe { needs_automatic_servicing(v, &*base) })
}
/// `Vehicle::HandleBreakdown`, for callers that consume only its result.
/// # Safety
/// As the direct maintenance calls.
pub(crate) unsafe fn vehicle_handle_breakdown(v: VehicleRef, base: &VehicleServices) -> bool {
    let counter = v.breakdown_counter();
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { breakdown_state::<true>(v, base, counter) }
    } else {
        unsafe { breakdown_state::<false>(v, base, counter) }
    }
}
/// # Safety
/// As the maintenance entries; this native API consumes only the stop result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_handle_breakdown_abi(
    v: VehicleRef,
    base: *const VehicleServices,
) -> u8 {
    u8::from(unsafe { vehicle_handle_breakdown(v, &*base) })
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleMaintenanceRenewal" ["maintenance"] => Renewal {
        age,
        max_age,
        months,
        enabled,
    };
    "OpenTTDVehicleMaintenanceReset" ["maintenance"] => Reset {
        economy_date,
        calendar_date,
        reliability,
        chance,
        breakdowns,
    };
    "OpenTTDVehicleMaintenanceInterval" ["maintenance"] => Interval {
        company,
        last_service,
        now,
        interval,
        reliability,
        percent,
        wallclock,
        no_service,
        breakdowns,
    };
    "OpenTTDVehicleMaintenanceReplacement" ["maintenance"] => Replacement {
        engine,
        when_old,
    };

    "OpenTTDVehicleMaintenanceAutomatic" ["maintenance"] => Automatic {
        has_depot_order,
        order_type,
    };
    "OpenTTDVehicleMaintenanceCheckBreakdown" ["maintenance"] => CheckBreakdown {
        reliability,
        decay,
        no_service,
        breakdowns,
    };
    "OpenTTDVehicleMaintenanceBreakdownGuard" ["maintenance"] => BreakdownGuard {
        breakdowns,
        menu,
    };
    "OpenTTDVehicleMaintenanceBreakdownTrigger" ["maintenance"] => BreakdownTrigger {
        reliability,
        chance,
        breakdowns,
    };
    "OpenTTDVehicleMaintenanceAge" ["maintenance"] => Age {
        age,
        max_age,
        decay,
    };
    "OpenTTDVehicleMaintenanceWarning" ["maintenance"] => Warning {
        enabled,
        local_company,
    };
    "OpenTTDVehicleMaintenanceWarningCompany" ["maintenance"] => WarningCompany {
        company,
        renewal,
    };
    "OpenTTDVehicleMaintenanceServices" ["maintenance"] => Services {
        shared,
        transaction,
        group,
        company,
        renewal,
        reset,
        interval,
        engine_reliability,
        replacement,
        replacement_available,
        automatic,
        depot_service,
        check_breakdown,
        breakdown_guard,
        breakdown_trigger,
        chance_write,
        breakdown_delay_write,
        smoke_disabled,
        smoke_delay,
        mark_dirty,
        view_dirty,
        details_dirty,
        age,
        age_write,
        decay_write,
        warning,
        warning_company,
        engine_available_any,
        replacement_active,
        old_news,
        economy_age,
        economy_age_write,
        reached_min_age,
        value,
        value_write,
        max_calendar_date,
        max_economy_date,
        profit_min_age,
        anniversaries,
        before_zero,
        news_getting_old,
        news_very_old,
        news_very_old_and,
    };
}
impl VehicleServices {
    /// Run a vehicle-maintenance entry.
    /// # Safety
    /// The caller must meet all preconditions of `entry(v, self)`, including a live,
    /// serialized shell and ending owner borrows before any reentrant callback.
    #[inline]
    pub(crate) unsafe fn maintain<R>(
        &self,
        v: VehicleRef,
        entry: unsafe fn(VehicleRef, &VehicleServices) -> R,
    ) -> R {
        // SAFETY: The caller satisfies the supplied entry's complete contract.
        unsafe { entry(v, self) }
    }
}
