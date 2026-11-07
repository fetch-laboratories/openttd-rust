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
use super::{Breakdown, OrderType, VehicleRef, VehicleServices, VehicleType};
use crate::cargo_type::{CargoType, CargoTypes};
use crate::company_type::CompanyID;
use crate::engine_type::{EngineID, EngineMiscFlags, NO_BREAKDOWN_SMOKE};
use crate::landscape_type::LandscapeType;
use crate::newgrf_sound::VSE_BREAKDOWN;
use crate::sound_type::SoundFx;
use crate::timer_game_common::DAYS_IN_LEAP_YEAR;
use crate::timer_game_economy::DAYS_IN_ECONOMY_MONTH;
use crate::vehicle_base::{GroundVehicleSubtypeFlags, VS_CRASHED, VS_STOPPED, VehState, VehStates};
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
    pub company: *const crate::company_identity::Services,
    /// Company pointers must name the live owner for the synchronous call.
    pub renewal: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> Renewal,
    pub reset: extern "C" fn(VehicleRef) -> Reset,
    pub has_engine: extern "C" fn(VehicleRef) -> bool,
    pub interval: extern "C" fn(VehicleRef) -> Interval,
    pub replacement: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany) -> Replacement,
    pub automatic: extern "C" fn(VehicleRef) -> Automatic,
    pub depot_service: extern "C" fn(VehicleRef) -> bool,
    pub check_breakdown: extern "C" fn(VehicleRef) -> CheckBreakdown,
    pub breakdown_guard: extern "C" fn(VehicleRef) -> BreakdownGuard,
    pub breakdown_trigger: extern "C" fn(VehicleRef) -> BreakdownTrigger,
    pub chance_write: extern "C" fn(VehicleRef, u8),
    pub trigger: extern "C" fn(VehicleRef, u8, u8),
    pub breakdown_probe: extern "C" fn(VehicleRef) -> BreakdownProbe,
    pub breakdown_start: extern "C" fn(VehicleRef) -> BreakdownStart,
    pub breakdown_start_write: extern "C" fn(VehicleRef, u8, u8),
    pub breakdown_active: extern "C" fn(VehicleRef) -> BreakdownActive,
    pub breakdown_countdown: extern "C" fn(VehicleRef) -> BreakdownCountdown,
    pub breakdown_delay_write: extern "C" fn(VehicleRef, u8),
    pub smoke_delay: extern "C" fn(VehicleRef) -> u8,
    pub aircraft_broken: extern "C" fn(VehicleRef),
    pub mark_dirty: extern "C" fn(VehicleRef),
    pub view_dirty: extern "C" fn(VehicleRef),
    pub details_dirty: extern "C" fn(VehicleRef),
    pub age: extern "C" fn(VehicleRef) -> Age,
    pub age_write: extern "C" fn(VehicleRef, CalendarDate),
    pub decay_write: extern "C" fn(VehicleRef, u16),
    pub warning: extern "C" fn(VehicleRef) -> Warning,
    pub warning_company: extern "C" fn(VehicleRef) -> WarningCompany,
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

/// Function to tell if a vehicle needs to be autorenewed.
/// `company` is the vehicle owner; `use_renew_setting` considers its renew setting.
/// Returns true if the vehicle is old enough for replacement.
unsafe fn needs_autorenewing<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    company: *const OpenTTDCompany,
    use_renew_setting: bool,
) -> bool {
    let s = unsafe { &*base.maintenance };
    // We can always generate the Company pointer when we have the vehicle.
    // However this takes time and since the Company pointer is often present
    // when this function is called then it's faster to pass the pointer as an
    // argument rather than finding it again.
    // SAFETY: the native entry supplies the live owner and asserts its identity.
    let r = unsafe { svc!(C, s.renewal, v, company) };
    branch!(C, VEHICLE_MAINT_RENEW);
    if use_renew_setting && !r.enabled {
        return false;
    }
    if r.age.0.wrapping_sub(r.max_age.0) < i32::from(r.months) * 30 {
        return false;
    }
    // Only engines need renewing.
    r.kind != VehicleType::VEH_TRAIN
        || r.subtype & (1 << GroundVehicleSubtypeFlags::GVSF_ENGINE.0) != 0
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
        svc!(C, base.write_reliability, v, (base.engine)(v).reliability());
        svc!(C, s.chance_write, v, chance);
        branch!(C, VEHICLE_MAINT_RESET);
        let Some(next) = v.next() else {
            break;
        };
        if !svc!(C, s.has_engine, next) {
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
/// Check if the vehicle needs to go to a depot in near future (if a opportunity
/// presents itself) for service or replacement. See `needs_automatic_servicing`.
unsafe fn needs_servicing_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    let s = unsafe { &*base.maintenance };
    // Stopped or crashed vehicles will not move, as such making unmovable
    // vehicles to go for service is lame.
    if VehStates(v.status()).0
        & VehStates::from_element(VehState::Stopped)
            .with(VehStates::from_element(VehState::Crashed))
            .0
        != 0
    {
        return false;
    }
    // Are we ready for the next service cycle?
    let r = svc!(C, s.interval, v);
    branch!(C, VEHICLE_MAINT_SERVICE);
    // Service intervals can be measured in different units, which we handle individually.
    if r.percent {
        // Service interval is in percents.
        branch!(C, VEHICLE_MAINT_PERCENT);
        let reliability = (base.engine)(v).reliability();
        if i32::from(r.reliability)
            >= i32::from(reliability).wrapping_mul(100 - i32::from(r.interval)) / 100
        {
            return false;
        }
    } else {
        let interval = i32::from(r.interval)
            * if r.wallclock {
                // Service interval is in minutes.
                branch!(C, VEHICLE_MAINT_MINUTES);
                DAYS_IN_ECONOMY_MONTH
            } else {
                // Service interval is in days.
                branch!(C, VEHICLE_MAINT_DAYS);
                1
            };
        if r.last_service.0.wrapping_add(interval) >= r.now.0 {
            return false;
        }
    }
    // If we're servicing anyway, because we have not disabled servicing when
    // there are no breakdowns or we are playing with breakdowns, bail out.
    if !r.no_service || r.breakdowns != 0 {
        return true;
    }
    // Test whether there is some pending autoreplace.
    // Note: We do this after the service-interval test.
    // There are a lot more reasons for autoreplace to fail than we can test here reasonably.
    // SAFETY: interval resolved the live owner; no callback can destroy it here.
    let companies = unsafe { &*s.company };
    let mut needed_money = i64::from((companies.renew_reserve)(r.company));
    if needed_money > unsafe { available_money(s, r.company) } {
        return false;
    }
    let mut pending_replace = false;
    let mut vehicle = Some(v);
    while let Some(v) = vehicle {
        if unsafe {
            replacement_needed::<C>(v, base, r.company, &mut needed_money, &mut pending_replace)
        } {
            return false;
        }
        vehicle = if v.vehicle_type() == VehicleType::VEH_TRAIN.0 {
            v.next_unit()
        } else {
            None
        };
    }
    pending_replace
}
unsafe fn replacement_needed<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    company: *const OpenTTDCompany,
    needed_money: &mut i64,
    pending_replace: &mut bool,
) -> bool {
    let s = unsafe { &*base.maintenance };
    // SAFETY: needs_servicing_impl retains the live native owner through this decision.
    let replacement = unsafe { svc!(C, s.replacement, v, company) };
    let new_engine = EngineID(replacement.engine);
    // Check engine availability.
    if new_engine == EngineID::INVALID
        || crate::vehicle::EngineRef::of((unsafe { &*s.transaction }.capabilities.engine)(
            new_engine.0,
        ))
        .company_availability()
            & (1 << CompanyID(v.owner()).0)
            == 0
    {
        return false;
    }
    // Is the vehicle old if we are not always replacing?
    if replacement.when_old && !unsafe { needs_autorenewing::<C>(v, base, company, false) } {
        return false;
    }
    // Check refittability.
    let t = unsafe { &*s.transaction };
    let (mut union_mask, mut available_cargo_types) = (CargoTypes(0), CargoTypes(0));
    unsafe {
        crate::engine_capabilities::openttd_rust_articulated_refit_masks(
            t.capabilities,
            new_engine.0,
            true,
            &raw mut union_mask.0,
            &raw mut available_cargo_types.0,
        );
    };
    if C {
        crate::witness::add(
            crate::witness::VEHICLE_MAINT_BYTES,
            (std::mem::size_of_val(&union_mask.0) + std::mem::size_of_val(&available_cargo_types.0))
                as u64,
        );
    }
    // Is there anything to refit?
    if union_mask.0 != 0 {
        let mut cargo_type = CargoType(0);
        let cargo_mask = CargoTypes(unsafe {
            crate::engine_capabilities::openttd_rust_articulated_live_cargoes(
                t.capabilities,
                v.0.as_ptr(),
                &raw mut cargo_type.0,
            )
        });
        if C {
            crate::witness::add(
                crate::witness::VEHICLE_MAINT_BYTES,
                std::mem::size_of_val(&cargo_type.0) as u64,
            );
        }
        if cargo_mask.0 & cargo_mask.0.wrapping_sub(1) != 0 {
            let new_engine_default_cargoes = CargoTypes(unsafe {
                crate::engine_capabilities::openttd_rust_articulated_cargoes(
                    t.capabilities,
                    new_engine.0,
                )
            });
            if cargo_mask.0 & new_engine_default_cargoes.0 != cargo_mask.0 {
                // We cannot refit to mixed cargoes in an automated way.
                return false;
            }
            // engine_type is already a mixed cargo type which matches the incoming
            // vehicle by default, no refit required.
        } else if cargo_type.0 < CargoType::NUM_CARGO.0 // Did the old vehicle carry anything?
            && available_cargo_types.0 & (1_u64 << cargo_type.0) == 0
        {
            // We can't refit the vehicle to carry the cargo we want.
            return false;
        }
    }
    // Check money.
    // We want 2*(the price of the new vehicle) without looking at the value of
    // the vehicle we are going to sell.
    *pending_replace = true;
    // SAFETY: the same live owner remains valid after the bounded refit queries.
    let cap = t.capabilities;
    let cost = unsafe {
        crate::engine_capabilities::openttd_rust_engine_cost(cap, (cap.engine)(new_engine.0))
    };
    *needed_money = needed_money.saturating_add(cost.saturating_mul(2));
    let available_money = unsafe { available_money(s, company) };
    branch!(C, VEHICLE_MAINT_REPLACE);
    *needed_money > available_money
}
/// Checks if the current order should be interrupted for a service-in-depot
/// order. See `needs_servicing`.
unsafe fn needs_automatic_servicing_impl<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
) -> bool {
    let s = unsafe { &*base.maintenance };
    let r = svc!(C, s.automatic, v);
    if r.has_depot_order
        || r.order_type == OrderType::OT_LOADING
        || (r.order_type == OrderType::OT_GOTO_DEPOT && !svc!(C, s.depot_service, v))
    {
        return false;
    }
    unsafe { needs_servicing_impl::<C>(v, base) }
}

const BREAKDOWN_CHANCE: [u8; 64] = [
    3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 13, 13, 13, 13,
    14, 15, 16, 17, 19, 21, 25, 28, 31, 34, 37, 40, 44, 48, 52, 56, 60, 64, 68, 72, 80, 90, 100,
    110, 120, 130, 140, 150, 170, 190, 210, 230, 250, 250, 250,
];
/// `CheckVehicleBreakdown` in `vehicle.cpp`.
unsafe fn check_vehicle_breakdown_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    let check = svc!(C, s.check_breakdown, v);
    branch!(C, VEHICLE_MAINT_CHECK);
    // Decrease reliability.
    let mut rel = check.reliability;
    if !check.no_service || check.breakdowns != 0 {
        rel = rel.saturating_sub(check.decay);
        svc!(C, base.write_reliability, v, rel);
        if check.reliability >> 8 != rel >> 8 {
            svc!(C, s.details_dirty, v);
        }
    }
    let guard = svc!(C, s.breakdown_guard, v);
    if guard.counter != 0
        || VehStates(guard.status).0 & VehStates::from_element(VehState::Stopped).0 != 0
        || guard.breakdowns < 1
        || guard.speed < 5
        || guard.menu
    {
        return;
    }
    let shared = unsafe { &*s.shared };
    let r = svc!(C, shared.random, shared.context);
    let trigger = svc!(C, s.breakdown_trigger, v);
    // Increase chance of failure.
    let mut chance = u16::from(trigger.chance) + 1;
    if crate::random_func::chance16_i(1, 25, r) {
        chance += 25;
    }
    let chance = chance.min(255) as u8;
    svc!(C, s.chance_write, v, chance);
    // Calculate reliability value to use in comparison.
    let rel = u32::from(trigger.reliability)
        + if trigger.kind == VehicleType::VEH_SHIP {
            0x6666
        } else {
            0
        }
        // Reduced breakdowns?
        + if trigger.breakdowns == 1 { 0x6666 } else { 0 };
    // Check if to break down.
    if BREAKDOWN_CHANCE[(rel.min(65535) >> 10) as usize] <= chance {
        svc!(
            C,
            s.trigger,
            v,
            ((r >> 16) & 63) as u8 + 0x3f,
            ((r >> 24) & 127) as u8 + 0x80
        );
        branch!(C, VEHICLE_MAINT_TRIGGER);
    }
}

/// Handle all of the aspects of a vehicle breakdown, including smoke, sounds
/// and ending the breakdown when appropriate. Returns whether the vehicle is
/// stopped because of a breakdown; aircraft never stop for breakdowns.
///
/// `Vehicle::HandleBreakdown` in `vehicle.cpp`.
/// Only consumed value observations survive callbacks. Each later observation is
/// reacquired at its source point after sound, effect allocation and dirty work.
unsafe fn handle_breakdown_impl<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    counter: u8,
) -> bool {
    // Possible states for Vehicle::breakdown_ctr:
    // 0: running normally; 1: currently broken down; 2: breaking down now;
    // >2: counting down to the actual breakdown event.
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
            if start.kind == VehicleType::VEH_AIRCRAFT {
                // Aircraft just need this flag; the rest is handled elsewhere.
                svc!(C, s.aircraft_broken, v);
                branch!(C, VEHICLE_MAINT_AIRCRAFT);
            } else {
                (v).write_speed(0);
                if !svc!(C, base.play_vehicle_sound, v, VSE_BREAKDOWN) {
                    let train_or_ship =
                        start.kind == VehicleType::VEH_TRAIN || start.kind == VehicleType::VEH_SHIP;
                    let landscape = LandscapeType(unsafe { (*s.shared).settings().climate.read() });
                    let toyland = landscape == LandscapeType::Toyland;
                    let sound = if toyland {
                        if train_or_ship {
                            SoundFx::SND_3A_BREAKDOWN_TRAIN_SHIP_TOYLAND
                        } else {
                            SoundFx::SND_35_BREAKDOWN_ROADVEHICLE_TOYLAND
                        }
                    } else if train_or_ship {
                        SoundFx::SND_10_BREAKDOWN_TRAIN_SHIP
                    } else {
                        SoundFx::SND_0F_BREAKDOWN_ROADVEHICLE
                    };
                    svc!(C, base.play_sound, v, u16::from(sound.0));
                }
                if VehStates((v).status()).0 & VehStates::from_element(VehState::Hidden).0 == 0
                    && EngineMiscFlags((base.engine)(v).misc()).0 & NO_BREAKDOWN_SMOKE == 0
                {
                    let u = crate::effect::openttd_rust_effect_create_rel(
                        v.0.as_ptr(),
                        4,
                        4,
                        5,
                        crate::effectvehicle_func::EV_BREAKDOWN_SMOKE,
                    );
                    let delay = svc!(C, s.smoke_delay, v);
                    if !u.is_null() {
                        crate::effect::set_animation(u, u16::from(delay) * 2);
                        branch!(C, VEHICLE_MAINT_SMOKE);
                    }
                }
            }
            svc!(C, s.mark_dirty, v); // Update graphics after speed is zeroed.
            svc!(C, s.view_dirty, v);
            svc!(C, s.details_dirty, v);
        }
        1 => {}
        _ => {
            let countdown = svc!(C, s.breakdown_countdown, v);
            if countdown.order_type == OrderType::OT_LOADING {
                branch!(C, VEHICLE_MAINT_LOADING);
            } else {
                (v).write_breakdown_counter(countdown.counter.wrapping_sub(1));
                branch!(C, VEHICLE_MAINT_COUNTDOWN);
            }
            return false;
        }
    }
    let active = svc!(C, s.breakdown_active, v);
    // Aircraft breakdowns end only when arriving at the airport.
    if active.kind == VehicleType::VEH_AIRCRAFT {
        return false;
    }
    // For trains this is called twice per tick, so decrease breakdown_delay at half the rate.
    if active.tick
        & if active.kind == VehicleType::VEH_TRAIN {
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
            (v).write_breakdown_counter(0);
            svc!(C, s.mark_dirty, v);
            svc!(C, s.view_dirty, v);
            branch!(C, VEHICLE_MAINT_ENDED);
        }
    }
    true
}
unsafe fn handle_breakdown_result<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
) -> Breakdown {
    let s = unsafe { &*base.maintenance };
    let probe = svc!(C, s.breakdown_probe, v);
    if probe.counter == 0 {
        return Breakdown {
            broken: false,
            status: probe.status,
        };
    }
    let broken = unsafe { handle_breakdown_impl::<C>(v, base, probe.counter) };
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
    unsafe { handle_breakdown_result::<C>(v, base) }
}
/// `HandleBreakdown()`, then the status it leaves, selecting the witness once.
/// # Safety
/// As `handle_breakdown`.
pub(crate) unsafe fn handle_breakdown_status(v: VehicleRef, base: &VehicleServices) -> Breakdown {
    if crate::witness::enabled() {
        unsafe { handle_breakdown_result::<true>(v, base) }
    } else {
        unsafe { handle_breakdown_result::<false>(v, base) }
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
        || warning.has_previous
        || !warning.local
        || warning.status & (VS_CRASHED | VS_STOPPED) != 0
    {
        return;
    }
    let company = svc!(C, s.warning_company, v);
    if company.renewal && (base.engine)(v).company_availability() != 0 {
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
    check_vehicle_breakdown_impl
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
        unsafe { needs_autorenewing::<true>(v, &*base, company, settings) }
    } else {
        unsafe { needs_autorenewing::<false>(v, &*base, company, settings) }
    })
}
/// `Vehicle::NeedsServicing`.
/// # Safety
/// As the direct maintenance calls.
pub(crate) unsafe fn needs_servicing(v: VehicleRef, base: &VehicleServices) -> bool {
    if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        unsafe { needs_servicing_impl::<true>(v, base) }
    } else {
        unsafe { needs_servicing_impl::<false>(v, base) }
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
        unsafe { needs_automatic_servicing_impl::<true>(v, base) }
    } else {
        unsafe { needs_automatic_servicing_impl::<false>(v, base) }
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
        unsafe { handle_breakdown_impl::<true>(v, base, counter) }
    } else {
        unsafe { handle_breakdown_impl::<false>(v, base, counter) }
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
        kind,
        subtype,
    };
    "OpenTTDVehicleMaintenanceReset" ["maintenance"] => Reset {
        economy_date,
        calendar_date,
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
        speed,
        counter,
        status,
        breakdowns,
        menu,
    };
    "OpenTTDVehicleMaintenanceBreakdownTrigger" ["maintenance"] => BreakdownTrigger {
        reliability,
        chance,
        kind,
        breakdowns,
    };
    "OpenTTDVehicleMaintenanceBreakdownProbe" ["maintenance"] => BreakdownProbe {
        counter,
        status,
    };
    "OpenTTDVehicleMaintenanceBreakdownStart" ["maintenance"] => BreakdownStart {
        since,
        kind,
    };
    "OpenTTDVehicleMaintenanceBreakdownActive" ["maintenance"] => BreakdownActive {
        kind,
        tick,
        delay,
    };
    "OpenTTDVehicleMaintenanceBreakdownCountdown" ["maintenance"] => BreakdownCountdown {
        order_type,
        counter,
    };
    "OpenTTDVehicleMaintenanceAge" ["maintenance"] => Age {
        age,
        max_age,
        decay,
    };
    "OpenTTDVehicleMaintenanceWarning" ["maintenance"] => Warning {
        enabled,
        has_previous,
        local,
        status,
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
        has_engine,
        interval,
        replacement,
        automatic,
        depot_service,
        check_breakdown,
        breakdown_guard,
        breakdown_trigger,
        chance_write,
        trigger,
        breakdown_probe,
        breakdown_start,
        breakdown_start_write,
        breakdown_active,
        breakdown_countdown,
        breakdown_delay_write,
        smoke_delay,
        aircraft_broken,
        mark_dirty,
        view_dirty,
        details_dirty,
        age,
        age_write,
        decay_write,
        warning,
        warning_company,
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
