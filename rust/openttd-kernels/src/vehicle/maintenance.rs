/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Base-vehicle maintenance policy. Native shells remain canonical; observations
//! are operation-sized value copies; native services perform scalar stores. No world borrow survives callbacks, effects or news.
#![allow(
    unsafe_code,
    clippy::struct_excessive_bools,
    clippy::struct_field_names
)]
use super::{
    Breakdown, OT_GOTO_DEPOT, OT_LOADING, OrderType, VS_CRASHED, VS_HIDDEN, VS_STOPPED, VehicleRef,
    VehicleServices, VehicleType,
};
use crate::{company::OpenTTDCompany, fleet, services};

use crate::services::{CalendarDate, EconomyDate};

#[repr(C)]
pub struct Renewal {
    pub age: CalendarDate,
    pub max_age: CalendarDate,
    pub months: i16,
    pub enabled: bool,
    pub kind: VehicleType,
    pub subtype: u8,
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
pub struct ResetWrite {
    pub economy_date: EconomyDate,
    pub calendar_date: CalendarDate,
    pub reliability: u16,
    pub chance: u8,
    pub since: u8,
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
pub struct Budget {
    pub needed: i64,
    pub available: i64,
}
#[repr(C)]
pub struct ReplacementCost {
    pub cost: i64,
    pub available: i64,
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
    pub speed: u16,
    pub counter: u8,
    pub status: u8,
    pub breakdowns: u8,
    pub menu: bool,
}
#[repr(C)]
pub struct BreakdownTrigger {
    pub reliability: u16,
    pub chance: u8,
    pub kind: VehicleType,
    pub breakdowns: u8,
}
#[repr(C)]
pub struct BreakdownProbe {
    pub counter: u8,
    pub status: u8,
}
#[repr(C)]
pub struct BreakdownStart {
    pub since: u8,
    pub kind: VehicleType,
}
#[repr(C)]
pub struct BreakdownActive {
    pub kind: VehicleType,
    pub tick: u8,
    pub delay: u8,
}
#[repr(C)]
pub struct BreakdownCountdown {
    pub order_type: OrderType,
    pub counter: u8,
}
#[repr(C)]
pub struct BreakdownSoundState {
    pub kind: VehicleType,
    pub landscape: services::LandscapeType,
}
#[repr(C)]
pub struct BreakdownSmoke {
    pub shell: Option<VehicleRef>,
    pub delay: u8,
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
    pub has_previous: bool,
    pub local: bool,
    pub status: u8,
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
    /// Company pointers must name the live owner for the synchronous call.
    pub renewal: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> Renewal,
    pub reset: extern "C" fn(VehicleRef) -> Reset,
    pub reset_write: extern "C" fn(VehicleRef, ResetWrite),
    pub has_engine: extern "C" fn(VehicleRef) -> bool,
    pub interval: extern "C" fn(VehicleRef) -> Interval,
    pub engine_reliability: extern "C" fn(VehicleRef) -> u16,
    pub budget: unsafe extern "C" fn(*const OpenTTDCompany) -> Budget,
    pub replacement: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> Replacement,
    pub replacement_available: extern "C" fn(VehicleRef, u16) -> bool,
    pub replacement_cost: unsafe extern "C" fn(*const OpenTTDCompany, u16) -> ReplacementCost,
    pub automatic: extern "C" fn(VehicleRef) -> Automatic,
    pub depot_service: extern "C" fn(VehicleRef) -> bool,
    pub check_breakdown: extern "C" fn(VehicleRef) -> CheckBreakdown,
    pub breakdown_guard: extern "C" fn(VehicleRef) -> BreakdownGuard,
    pub breakdown_trigger: extern "C" fn(VehicleRef) -> BreakdownTrigger,
    pub chance_write: extern "C" fn(VehicleRef, u8),
    pub trigger: extern "C" fn(VehicleRef, u8, u8),
    pub breakdown_counter: extern "C" fn(VehicleRef) -> u8,
    pub breakdown_probe: extern "C" fn(VehicleRef) -> BreakdownProbe,
    pub breakdown_start: extern "C" fn(VehicleRef) -> BreakdownStart,
    pub breakdown_start_write: extern "C" fn(VehicleRef, u8, u8),
    pub breakdown_active: extern "C" fn(VehicleRef) -> BreakdownActive,
    pub breakdown_countdown: extern "C" fn(VehicleRef) -> BreakdownCountdown,
    pub breakdown_sound_state: extern "C" fn(VehicleRef) -> BreakdownSoundState,
    pub breakdown_delay_write: extern "C" fn(VehicleRef, u8),
    pub breakdown_sound: extern "C" fn(VehicleRef) -> bool,
    pub smoke_disabled: extern "C" fn(VehicleRef) -> bool,
    pub smoke: extern "C" fn(VehicleRef) -> BreakdownSmoke,
    pub smoke_animation: extern "C" fn(VehicleRef, u16),
    pub aircraft_broken: extern "C" fn(VehicleRef),
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
    ($counted:ident, $function:expr $(, $arg:expr)*) => {{
        let result = ($function)($($arg),*);
        if $counted {
            crate::witness::hit(crate::witness::VEHICLE_MAINT_CROSSINGS);
            crate::witness::add(crate::witness::VEHICLE_MAINT_BYTES, std::mem::size_of_val(&result) as u64);
        }
        result
    }};
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
    r.kind != VehicleType::TRAIN || r.subtype & 8 != 0
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
        svc!(
            C,
            s.reset_write,
            v,
            ResetWrite {
                economy_date: r.economy_date,
                calendar_date: r.calendar_date,
                reliability: r.reliability,
                chance,
                since: 0
            }
        );
        branch!(C, VEHICLE_MAINT_RESET);
        let Some(next) = svc!(C, base.next, v) else {
            break;
        };
        if !svc!(C, s.has_engine, next) {
            break;
        }
        v = next;
    }
}
unsafe fn needs_service<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    let s = unsafe { &*base.maintenance };
    if svc!(C, base.status, v) & (VS_STOPPED | VS_CRASHED) != 0 {
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
    let budget = unsafe { svc!(C, s.budget, r.company) };
    if budget.needed > budget.available {
        return false;
    }
    let mut needed = budget.needed;
    let mut pending = false;
    let mut current = Some(v);
    while let Some(unit) = current {
        if unsafe { replacement_needed::<C>(unit, base, r.company, &mut needed, &mut pending) } {
            return false;
        }
        current = if svc!(C, base.vehicle_type, unit) == VehicleType::TRAIN.0 {
            let t = unsafe { &*s.transaction };
            let next = unsafe { svc!(C, t.next_unit, unit.0.as_ptr().cast()) };
            std::ptr::NonNull::new(next.cast()).map(VehicleRef)
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
    if replacement.engine == super::INVALID_ENGINE
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
        svc!(
            C,
            t.refit_masks,
            replacement.engine,
            &raw mut union,
            &raw mut available
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
        let cargo = unsafe { svc!(C, t.vehicle_cargo, v.0.as_ptr().cast(), &raw mut cargo_type) };
        if C {
            crate::witness::add(
                crate::witness::VEHICLE_MAINT_BYTES,
                std::mem::size_of_val(&cargo_type) as u64,
            );
        }
        if cargo & cargo.wrapping_sub(1) != 0 {
            let defaults = unsafe { svc!(C, t.default_cargo, replacement.engine) };
            if cargo & defaults != cargo {
                return false;
            }
        } else if cargo_type < 64 && available & (1_u64 << cargo_type) == 0 {
            return false;
        }
    }
    *pending = true;
    // SAFETY: the same live owner remains valid after the bounded refit queries.
    let money = unsafe { svc!(C, s.replacement_cost, company, replacement.engine) };
    *needed = needed.saturating_add(money.cost.saturating_mul(2));
    branch!(C, VEHICLE_MAINT_REPLACE);
    *needed > money.available
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
    if guard.counter != 0
        || guard.status & VS_STOPPED != 0
        || guard.breakdowns < 1
        || guard.speed < 5
        || guard.menu
    {
        return;
    }
    let shared = unsafe { &*s.shared };
    let random = svc!(C, shared.random, shared.context);
    let trigger = svc!(C, s.breakdown_trigger, v);
    let mut chance = u16::from(trigger.chance) + 1;
    if services::chance16_i(1, 25, random) {
        chance += 25;
    }
    let chance = chance.min(255) as u8;
    svc!(C, s.chance_write, v, chance);
    let reliability = u32::from(trigger.reliability)
        + if trigger.kind == VehicleType::SHIP {
            0x6666
        } else {
            0
        }
        + if trigger.breakdowns == 1 { 0x6666 } else { 0 };
    if BREAKDOWN_CHANCE[(reliability.min(65535) >> 10) as usize] <= chance {
        svc!(
            C,
            s.trigger,
            v,
            ((random >> 16) & 63) as u8 + 0x3f,
            ((random >> 24) & 127) as u8 + 0x80
        );
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
            let start = svc!(C, s.breakdown_start, v);
            svc!(
                C,
                s.breakdown_start_write,
                v,
                1,
                start.since.saturating_add(1)
            );
            branch!(C, VEHICLE_MAINT_STARTED);
            if start.kind == VehicleType::AIRCRAFT {
                svc!(C, s.aircraft_broken, v);
                branch!(C, VEHICLE_MAINT_AIRCRAFT);
            } else {
                svc!(C, base.write_speed, v, 0);
                if !svc!(C, s.breakdown_sound, v) {
                    let sound = svc!(C, s.breakdown_sound_state, v);
                    let train_ship =
                        sound.kind == VehicleType::TRAIN || sound.kind == VehicleType::SHIP;
                    let toyland = sound.landscape == services::LandscapeType::TOYLAND;
                    let sound = if toyland {
                        if train_ship { 58 } else { 53 }
                    } else if train_ship {
                        14
                    } else {
                        13
                    };
                    svc!(C, base.play_sound, v, sound);
                }
                if svc!(C, base.status, v) & VS_HIDDEN == 0 && !svc!(C, s.smoke_disabled, v) {
                    let smoke = svc!(C, s.smoke, v);
                    if let Some(shell) = smoke.shell {
                        svc!(C, s.smoke_animation, shell, u16::from(smoke.delay) * 2);
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
            let countdown = svc!(C, s.breakdown_countdown, v);
            if countdown.order_type.0 == OT_LOADING {
                branch!(C, VEHICLE_MAINT_LOADING);
            } else {
                svc!(
                    C,
                    base.write_breakdown_counter,
                    v,
                    countdown.counter.wrapping_sub(1)
                );
                branch!(C, VEHICLE_MAINT_COUNTDOWN);
            }
            return false;
        }
    }
    let active = svc!(C, s.breakdown_active, v);
    if active.kind == VehicleType::AIRCRAFT {
        return false;
    }
    if active.tick
        & if active.kind == VehicleType::TRAIN {
            3
        } else {
            1
        }
        == 0
    {
        let delay = active.delay.wrapping_sub(1);
        svc!(C, s.breakdown_delay_write, v, delay);
        branch!(C, VEHICLE_MAINT_DELAY);
        if delay == 0 {
            svc!(C, base.write_breakdown_counter, v, 0);
            svc!(C, s.mark_dirty, v);
            svc!(C, s.view_dirty, v);
            branch!(C, VEHICLE_MAINT_ENDED);
        }
    }
    true
}
unsafe fn breakdown_result<const C: bool>(v: VehicleRef, base: &VehicleServices) -> Breakdown {
    let s = unsafe { &*base.maintenance };
    let probe = svc!(C, s.breakdown_probe, v);
    if probe.counter == 0 {
        return Breakdown {
            broken: false,
            status: probe.status,
        };
    }
    let broken = unsafe { breakdown_state::<C>(v, base, probe.counter) };
    Breakdown {
        broken,
        status: svc!(C, base.status, v),
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
    if !unsafe { svc!(C, g.primary, v.0.as_ptr().cast()) }
        && (svc!(C, base.vehicle_type, v) != VehicleType::TRAIN.0
            || svc!(C, base.subtype, v) & 8 == 0)
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
        || warning.has_previous
        || !warning.local
        || warning.status & (VS_CRASHED | VS_STOPPED) != 0
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
    } else if difference > s.anniversaries[0].0 && difference % 366 == 0 {
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
        if unsafe { svc!(C, g.primary, v.0.as_ptr().cast()) } && age.0 == s.profit_min_age.0 + 1 {
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
    ($name:ident, $body:ident) => {
        /// # Safety
        /// Native facade supplies a live shell and immutable tables; no borrow
        /// into canonical storage survives any synchronous callback/reentry.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(v: VehicleRef, base: *const VehicleServices) {
            let base = unsafe { &*base };
            if crate::witness::enabled() {
                crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
                unsafe { $body::<true>(v, base) }
            } else {
                unsafe { $body::<false>(v, base) }
            }
        }
    };
}
entry!(openttd_rust_vehicle_service_depot, reset);
entry!(openttd_rust_vehicle_age, age);
entry!(openttd_rust_vehicle_economy_age, economy_age);
entry!(openttd_rust_vehicle_decrease_value, decrease_value);
entry!(openttd_rust_vehicle_check_breakdown, check_breakdown);
/// # Safety
/// As the maintenance entries; company is the live native owner at source scope.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_renew(
    v: VehicleRef,
    base: *const VehicleServices,
    company: *const OpenTTDCompany,
    settings: bool,
) -> bool {
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { renew::<true>(v, &*base, company, settings) }
    } else {
        unsafe { renew::<false>(v, &*base, company, settings) }
    }
}
/// # Safety
/// As the maintenance entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_service(
    v: VehicleRef,
    base: *const VehicleServices,
) -> bool {
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { needs_service::<true>(v, &*base) }
    } else {
        unsafe { needs_service::<false>(v, &*base) }
    }
}
/// # Safety
/// As the maintenance entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_automatic_service(
    v: VehicleRef,
    base: *const VehicleServices,
) -> bool {
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { automatic::<true>(v, &*base) }
    } else {
        unsafe { automatic::<false>(v, &*base) }
    }
}
/// # Safety
/// As the maintenance entries; this native API consumes only the stop result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_handle_breakdown(
    v: VehicleRef,
    base: *const VehicleServices,
) -> bool {
    let base = unsafe { &*base };
    let s = unsafe { &*base.maintenance };
    if crate::witness::enabled() {
        const C: bool = true;
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        let counter = svc!(C, s.breakdown_counter, v);
        unsafe { breakdown_state::<true>(v, base, counter) }
    } else {
        const C: bool = false;
        let counter = svc!(C, s.breakdown_counter, v);
        unsafe { breakdown_state::<false>(v, base, counter) }
    }
}
