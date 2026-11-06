/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Base-vehicle maintenance policy over canonical records. Values captured by the
//! original stay local; later fields reload after callbacks, effects and dirty work.
#![allow(
    unsafe_code,
    clippy::struct_excessive_bools,
    clippy::struct_field_names
)]
use super::{Breakdown, OT_GOTO_DEPOT, OT_LOADING, VehicleRef, VehicleServices, VehicleType};
use crate::newgrf_sound::VSE_BREAKDOWN;
use crate::raw_field::read_field;
use crate::timer_game_common::DAYS_IN_LEAP_YEAR;
use crate::vehicle_base::{VS_CRASHED, VS_HIDDEN, VS_STOPPED};
use crate::{company::OpenTTDCompany, fleet, services};

use crate::services::{CalendarDate, EconomyDate};

#[repr(C)]
pub struct Replacement {
    pub engine: u16,
    pub when_old: bool,
}

/// Each callback is a named shared service, observation or consecutive scalar
/// store. Tables are immutable and outlive the synchronous native/Rust entry.
#[repr(C)]
pub struct Services {
    pub shared: *const services::Services,
    pub transaction: *const fleet::transactions::Services,
    pub group: *const fleet::GroupServices,
    pub company: *const crate::company_identity::Services,
    pub replacement: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany, u16) -> Replacement,
    pub mark_dirty: extern "C" fn(VehicleRef),
    pub view_dirty: extern "C" fn(VehicleRef),
    pub details_dirty: extern "C" fn(VehicleRef),
    pub replacement_active: unsafe extern "C" fn(VehicleRef, *const OpenTTDCompany, u16) -> bool,
    pub old_news: extern "C" fn(VehicleRef, u32),
    pub reached_min_age: extern "C" fn(VehicleRef),
    pub max_calendar_date: CalendarDate,
    pub max_economy_date: EconomyDate,
    pub profit_min_age: EconomyDate,
    pub anniversaries: [CalendarDate; 5],
    pub before_zero: CalendarDate,
    pub news_getting_old: u32,
    pub news_very_old: u32,
    pub news_very_old_and: u32,
}

impl Services {
    /// Reuse the command/Company registry's existing settings table route at
    /// game entry, avoiding a second binding or a static-initialization cycle.
    fn settings(&self) -> &'static crate::settings_table::Services {
        let commands = unsafe { &*(*self.transaction).commands };
        let company = unsafe { &*(commands.company)() };
        (company.settings)()
    }
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
    let companies = unsafe { &*s.company };
    if base.assertions {
        let expected = (companies.company)(crate::company_type::CompanyID(v.owner()));
        crate::movement::native_assert(unsafe { &*s.shared }, expected.cast_const() == company);
    }
    let age = v.age();
    let max_age = v.max_age();
    let settings = (companies.settings)(company);
    let months = read_field!(settings, engine_renew_months);
    let enabled = read_field!(settings, engine_renew);
    let kind = VehicleType(v.vehicle_type());
    let subtype = v.subtype();
    branch!(C, VEHICLE_MAINT_RENEW);
    if use_setting && !enabled {
        return false;
    }
    if age.0.wrapping_sub(max_age.0) < i32::from(months) * 30 {
        return false;
    }
    kind != VehicleType::TRAIN || subtype & 8 != 0
}
unsafe fn reset<const C: bool>(mut v: VehicleRef, base: &VehicleServices) {
    let s = unsafe { &*base.maintenance };
    svc!(C, s.details_dirty, v);
    loop {
        let economy_date = crate::timer_game_common::economy_date();
        let calendar_date = crate::timer_game_common::calendar_date();
        let reliability = (base.engine)(v).reliability();
        let old_chance = v.breakdown_chance();
        let breakdowns = unsafe { (*s.shared).settings().vehicle_breakdowns.read() };
        let chance = if breakdowns == 1 {
            branch!(C, VEHICLE_MAINT_RESET_REDUCED);
            0
        } else {
            old_chance / 4
        };
        v.write_date_of_last_service(economy_date);
        v.write_date_of_last_service_newgrf(calendar_date);
        v.write_breakdowns_since_last_service(0);
        v.write_reliability(reliability);
        v.write_breakdown_chance(chance);
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
    let companies = unsafe { &*s.company };
    let company = (companies.company)(crate::company_type::CompanyID(v.owner()));
    let last_service = v.date_of_last_service();
    let now = crate::timer_game_common::economy_date();
    let interval = v.service_interval();
    let reliability = v.reliability();
    let percent = v.vehicle_flags() & (1 << 9) != 0;
    let wallclock = (s.settings().clock.using_wallclock_units)(false);
    let settings = unsafe { (*s.shared).settings() };
    let no_service = unsafe { settings.no_servicing_if_no_breakdowns.read() };
    let breakdowns = unsafe { settings.vehicle_breakdowns.read() };
    branch!(C, VEHICLE_MAINT_SERVICE);
    if percent {
        branch!(C, VEHICLE_MAINT_PERCENT);
        let engine_reliability = (base.engine)(v).reliability();
        if i32::from(reliability)
            >= i32::from(engine_reliability).wrapping_mul(100 - i32::from(interval)) / 100
        {
            return false;
        }
    } else {
        let interval = i32::from(interval)
            * if wallclock {
                branch!(C, VEHICLE_MAINT_MINUTES);
                30
            } else {
                branch!(C, VEHICLE_MAINT_DAYS);
                1
            };
        if last_service.0.wrapping_add(interval) >= now.0 {
            return false;
        }
    }
    if !no_service || breakdowns != 0 {
        return true;
    }
    // The same live Company member remains valid through these bounded queries.
    let mut needed = i64::from(read_field!(
        (companies.settings)(company),
        engine_renew_money
    ));
    if needed > unsafe { available_money(s, company) } {
        return false;
    }
    let mut pending = false;
    let mut current = Some(v);
    while let Some(unit) = current {
        if unsafe { replacement_needed::<C>(unit, base, company, &mut needed, &mut pending) } {
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
    let replacement = unsafe { svc!(C, s.replacement, v, company, v.engine_id()) };
    if replacement.engine == crate::engine_type::INVALID_ENGINE
        || crate::vehicle::EngineRef::of((unsafe { &*s.transaction }.capabilities.engine)(
            replacement.engine,
        ))
        .company_availability()
            & (1 << v.owner())
            == 0
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
    let orders = crate::orders::VisitOrders::new(base.orders());
    let list = crate::orders::VisitOrders::list(v);
    let has_depot_order = !list.is_null()
        && (0..orders.stored_count(list))
            .any(|i| orders.stored_order(list, i).kind() == OT_GOTO_DEPOT);
    let current = v.current_order();
    if has_depot_order
        || current.kind() == OT_LOADING
        || (current.kind() == OT_GOTO_DEPOT && current.depot_type() & 1 == 0)
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
    let shared = unsafe { &*s.shared };
    let settings = shared.settings();
    let before = v.reliability();
    let decay = v.reliability_spd_dec();
    let no_service = unsafe { settings.no_servicing_if_no_breakdowns.read() };
    let breakdowns = unsafe { settings.vehicle_breakdowns.read() };
    branch!(C, VEHICLE_MAINT_CHECK);
    if !no_service || breakdowns != 0 {
        let reliability = before.saturating_sub(decay);
        v.write_reliability(reliability);
        if before >> 8 != reliability >> 8 {
            svc!(C, s.details_dirty, v);
        }
    }
    let speed = v.speed();
    let counter = v.breakdown_counter();
    let status = v.status();
    let breakdowns = unsafe { settings.vehicle_breakdowns.read() };
    let menu = crate::session::mode() == crate::openttd::GameMode::MENU;
    if counter != 0 || status & VS_STOPPED != 0 || breakdowns < 1 || speed < 5 || menu {
        return;
    }
    let random = svc!(C, shared.random, shared.context);
    let reliability = v.reliability();
    let chance = v.breakdown_chance();
    let kind = VehicleType(v.vehicle_type());
    let breakdowns = unsafe { settings.vehicle_breakdowns.read() };
    let mut chance = u16::from(chance) + 1;
    if crate::random_func::chance16_i(1, 25, random) {
        chance += 25;
    }
    let chance = chance.min(255) as u8;
    v.write_breakdown_chance(chance);
    let reliability = u32::from(reliability)
        + if kind == VehicleType::SHIP { 0x6666 } else { 0 }
        + if breakdowns == 1 { 0x6666 } else { 0 };
    if BREAKDOWN_CHANCE[(reliability.min(65535) >> 10) as usize] <= chance {
        v.write_breakdown_counter(((random >> 16) & 63) as u8 + 0x3f);
        v.write_breakdown_delay(((random >> 24) & 127) as u8 + 0x80);
        v.write_breakdown_chance(0);
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
            let since = v.breakdowns_since_last_service();
            let kind = VehicleType(v.vehicle_type());
            v.write_breakdown_counter(1);
            v.write_breakdowns_since_last_service(since.saturating_add(1));
            branch!(C, VEHICLE_MAINT_STARTED);
            if kind == VehicleType::AIRCRAFT {
                v.write_status(v.status() | crate::vehicle_base::VS_AIRCRAFT_BROKEN);
                branch!(C, VEHICLE_MAINT_AIRCRAFT);
            } else {
                (v).write_speed(0);
                if !svc!(C, base.play_vehicle_sound, v, VSE_BREAKDOWN) {
                    let train_ship = kind == VehicleType::TRAIN || kind == VehicleType::SHIP;
                    let climate = unsafe { (*s.shared).settings().climate.read() };
                    let toyland = climate == services::LandscapeType::TOYLAND.0;
                    let sound = if toyland {
                        if train_ship { 58 } else { 53 }
                    } else if train_ship {
                        14
                    } else {
                        13
                    };
                    svc!(C, base.play_sound, v, sound);
                }
                if (v).status() & VS_HIDDEN == 0
                    && crate::vehicle::EngineRef::of((unsafe { &*s.transaction }
                        .capabilities
                        .engine)(v.engine_id()))
                    .misc()
                        & crate::engine_type::NO_BREAKDOWN_SMOKE
                        == 0
                {
                    let smoke = crate::effect::openttd_rust_effect_create_rel(
                        v.0.as_ptr(),
                        4,
                        4,
                        5,
                        crate::effectvehicle_func::EV_BREAKDOWN_SMOKE,
                    );
                    let delay = v.breakdown_delay();
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
            let order_type = v.current_order().kind();
            let counter = v.breakdown_counter();
            if order_type == OT_LOADING {
                branch!(C, VEHICLE_MAINT_LOADING);
            } else {
                (v).write_breakdown_counter(counter.wrapping_sub(1));
                branch!(C, VEHICLE_MAINT_COUNTDOWN);
            }
            return false;
        }
    }
    let kind = VehicleType(v.vehicle_type());
    let tick = v.tick();
    let delay = v.breakdown_delay();
    if kind == VehicleType::AIRCRAFT {
        return false;
    }
    if tick & if kind == VehicleType::TRAIN { 3 } else { 1 } == 0 {
        let delay = delay.wrapping_sub(1);
        v.write_breakdown_delay(delay);
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
    let status = v.status();
    if counter == 0 {
        return Breakdown {
            broken: false,
            status,
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
    let before = v.age();
    let max_age = v.max_age();
    let decay = v.reliability_spd_dec();
    let age = if before.0 < s.max_calendar_date.0 {
        before.0 + 1
    } else {
        branch!(C, VEHICLE_MAINT_MAX_AGE);
        before.0
    };
    if before.0 < s.max_calendar_date.0 {
        v.write_age(CalendarDate(age));
    }
    let g = unsafe { &*s.group };
    if !unsafe { svc!(C, g.primary, v.shell().cast()) }
        && (v.vehicle_type() != VehicleType::TRAIN.0 || v.subtype() & 8 == 0)
    {
        return;
    }
    let difference = age.wrapping_sub(max_age.0);
    for anniversary in s.anniversaries {
        if difference == anniversary.0 {
            v.write_reliability_spd_dec(decay.wrapping_shl(1));
            branch!(C, VEHICLE_MAINT_DECAY);
            break;
        }
    }
    svc!(C, s.details_dirty, v);
    branch!(C, VEHICLE_MAINT_AGE);
    let enabled = unsafe { (*s.shared).settings().old_vehicle_warn.read() };
    let has_previous = v.previous().is_some();
    let local = v.owner() == (s.settings().company.local_company)();
    let status = v.status();
    if !enabled || has_previous || !local || status & (VS_CRASHED | VS_STOPPED) != 0 {
        return;
    }
    let companies = unsafe { &*s.company };
    let company = (companies.company)(crate::company_type::CompanyID(v.owner()));
    let renewal = read_field!((companies.settings)(company), engine_renew);
    if renewal
        && crate::vehicle::EngineRef::of((unsafe { &*s.transaction }.capabilities.engine)(
            v.engine_id(),
        ))
        .company_availability()
            != 0
    {
        return;
    }
    // Engine availability only reads; the Company is still the original live owner.
    if unsafe { svc!(C, s.replacement_active, v, company, v.engine_id()) } {
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
    let r = v.economy_age();
    if r.0 < s.max_economy_date.0 {
        let age = EconomyDate(r.0 + 1);
        v.write_economy_age(age);
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
    let value = v.value();
    v.write_value(value.saturating_sub(value >> 8));
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
    "OpenTTDVehicleMaintenanceReplacement" ["maintenance"] => Replacement {
        engine,
        when_old,
    };

    "OpenTTDVehicleMaintenanceServices" ["maintenance"] => Services {
        shared,
        transaction,
        group,
        company,
        replacement,
        mark_dirty,
        view_dirty,
        details_dirty,
        replacement_active,
        old_news,
        reached_min_age,
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
