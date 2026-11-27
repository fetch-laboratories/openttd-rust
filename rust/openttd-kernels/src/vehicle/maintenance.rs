/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `vehicle.cpp`'s base-vehicle maintenance: `Vehicle::NeedsAutorenewing`,
//! `VehicleServiceInDepot`, `Vehicle::NeedsServicing`, `Vehicle::NeedsAutomaticServicing`,
//! `DecreaseVehicleValue`, `CheckVehicleBreakdown`, `Vehicle::HandleBreakdown`,
//! `EconomyAgeVehicle` and `AgeVehicle`.
//!
//! Each body reads a member the vehicle record holds in place, and a member still in the C++
//! shell through its `VehicleRef` accessor, where the original reads it. The tables are
//! immutable and live for the process; the vehicle is a live shell of the serialized game state
//! for the whole entry, and no value is carried across a call that can reenter Rust (sound,
//! effect creation, dirtying, windows and news).

use super::{VehicleRef, VehicleServices, VehicleType};
use crate::autoreplace_func::{engine_has_replacement_for_company, engine_replacement_for_company};
use crate::bitmath_func::{gb, has_at_most_one_bit, has_bit64};
use crate::cargo_type::{CargoType, is_valid_cargo_type};
use crate::company::{OpenTTDCompany, get_available_money};
use crate::company_type::{CompanyID, CompanyMask};
use crate::effectvehicle_func::EffectVehicleType;
use crate::engine_type::{EngineID, EngineMiscFlag, EngineMiscFlags};
use crate::landscape_type::LandscapeType;
use crate::math::{clamp_to_u8, clamp_to_u16};
use crate::newgrf_sound::VehicleSoundEvent as SoundEvent;
use crate::news_func::add_vehicle_advice_news_item;
use crate::news_type::AdviceType;
use crate::openttd::GameMode;
use crate::order_type::{OrderDepotTypeFlag, OrderType};
use crate::random_func::chance16_i;
use crate::sound_type::SoundFx;
use crate::strings_func::get_encoded_string;
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::{
    STR_NEWS_VEHICLE_IS_GETTING_OLD, STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD,
    STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD_AND,
};
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_common::{
    DAYS_IN_LEAP_YEAR, MAX_DATE, calendar_date, date_at_start_of_year, economy_date,
};
use crate::timer_game_economy::{DAYS_IN_ECONOMY_MONTH, EconomyDate};
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_func::VEHICLE_PROFIT_MIN_AGE;
use crate::window_func::set_window_dirty;
use crate::window_type::{WindowClass, WindowNumber};
use crate::{fleet, newgrf_vehicle, services};

/// The maintenance services: the tables the bodies reach, `Vehicle::MarkDirty`, and the
/// accessor slots of members still stored in the C++ shell. Each slot passes the member's base
/// integer; its `VehicleRef` accessor converts.
#[repr(C)]
pub struct Services {
    pub clock: *const crate::timer_game_common::ClockServices,
    pub company: *const crate::company_identity::Services,
    pub economy_age: extern "C" fn(VehicleRef) -> i32,
    pub group: *const fleet::GroupServices,
    pub mark_dirty: extern "C" fn(VehicleRef),
    pub set_age: extern "C" fn(VehicleRef, i32),
    pub set_breakdown_chance: extern "C" fn(VehicleRef, u8),
    pub set_breakdown_delay: extern "C" fn(VehicleRef, u8),
    pub set_economy_age: extern "C" fn(VehicleRef, i32),
    pub set_reliability_spd_dec: extern "C" fn(VehicleRef, u16),
    pub set_value: extern "C" fn(VehicleRef, i64),
    pub shared: *const services::Services,
    pub transaction: *const fleet::transactions::Services,
}

/// One C call of this file: a `crossings` hit and its result's size in `bytes`.
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

#[allow(
    unsafe_code,
    reason = "the table pointers name immutable process-lifetime tables, and the group \
              table's virtual takes the live shell"
)]
impl Services {
    /// The vehicle table's maintenance services.
    pub(crate) fn of(base: &VehicleServices) -> &Self {
        // SAFETY: The module's table contract.
        unsafe { &*base.maintenance }
    }
    fn clock(&self) -> &crate::timer_game_common::ClockServices {
        // SAFETY: The module's table contract.
        unsafe { &*self.clock }
    }
    fn company(&self) -> &crate::company_identity::Services {
        // SAFETY: The module's table contract.
        unsafe { &*self.company }
    }
    fn group(&self) -> &fleet::GroupServices {
        // SAFETY: The module's table contract.
        unsafe { &*self.group }
    }
    fn shared(&self) -> &services::Services {
        // SAFETY: The module's table contract.
        unsafe { &*self.shared }
    }
    fn transaction(&self) -> &fleet::transactions::Services {
        // SAFETY: The module's table contract.
        unsafe { &*self.transaction }
    }
    fn commands(&self) -> &services::CommandServices {
        // SAFETY: The module's table contract.
        unsafe { &*self.transaction().commands }
    }
}

/// Count one getter call of this file: the table pointer it returns.
fn getter<const C: bool>() {
    if C {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_CROSSINGS);
        crate::witness::add(
            crate::witness::VEHICLE_MAINT_BYTES,
            std::mem::size_of::<*const ()>() as u64,
        );
    }
}

/// The vehicle's resolver table, one C call.
fn resolver<const C: bool>(base: &VehicleServices) -> &'static newgrf_vehicle::Services {
    getter::<C>();
    base.newgrf()
}

/// `SetWindowDirty(cls, v->index)`.
fn set_vehicle_window_dirty<const C: bool>(
    cls: WindowClass,
    v: VehicleRef,
    r: &newgrf_vehicle::Services,
) {
    set_window_dirty(cls, WindowNumber::from(svc!(C, VehicleRef::index, v, r)));
}

/// `Vehicle::NeedsAutorenewing`: function to tell if a vehicle needs to be autorenewed. `c` is the
/// vehicle owner; `use_renew_setting` says whether the company renew setting should be considered.
/// Returns true if the vehicle is old enough for replacement.
fn needs_autorenewing<const C: bool>(
    v: VehicleRef,
    r: &newgrf_vehicle::Services,
    companies: &crate::company_identity::Services,
    c: *const OpenTTDCompany,
    use_renew_setting: bool,
) -> bool {
    // We can always generate the Company pointer when we have the vehicle.
    // However this takes time and since the Company pointer is often present
    // when this function is called then it's faster to pass the pointer as an
    // argument rather than finding it again.
    crate::services_assert::assert_with(|| std::ptr::eq(c, svc!(C, companies.company, v.owner())));
    branch!(C, VEHICLE_MAINT_RENEW);

    if use_renew_setting && !svc!(C, companies.engine_renew, c) {
        return false;
    }
    if svc!(C, VehicleRef::age, v, r)
        .0
        .wrapping_sub(svc!(C, VehicleRef::max_age, v, r).0)
        < i32::from(svc!(C, companies.engine_renew_months, c)) * 30
    {
        return false;
    }

    // Only engines need renewing.
    if v.vehicle_type() == VehicleType::VEH_TRAIN && !crate::ground_vehicle::is_engine(v.subtype())
    {
        return false;
    }

    true
}

/// `VehicleServiceInDepot`: service a vehicle and all subsequent vehicles in the consist; `v` is
/// the vehicle or vehicle chain being serviced.
fn vehicle_service_in_depot_impl<const C: bool>(mut v: VehicleRef, base: &VehicleServices) {
    // The original's `assert(v != nullptr)` holds for every `VehicleRef`.
    let m = Services::of(base);
    let r = resolver::<C>(base);
    // Ensure that last service date and reliability are updated.
    set_vehicle_window_dirty::<C>(WindowClass::WC_VEHICLE_DETAILS, v, r);

    loop {
        svc!(
            C,
            VehicleRef::set_date_of_last_service,
            v,
            base,
            economy_date()
        );
        svc!(
            C,
            VehicleRef::set_date_of_last_service_newgrf,
            v,
            base,
            calendar_date()
        );
        svc!(C, VehicleRef::set_breakdowns_since_last_service, v, base, 0);
        let engine = svc!(C, base.engine, v);
        svc!(
            C,
            VehicleRef::set_reliability,
            v,
            base,
            engine.reliability()
        );
        // Prevent vehicles from breaking down directly after exiting the depot.
        let chance = svc!(C, VehicleRef::breakdown_chance, v, r) / 4;
        svc!(C, VehicleRef::set_breakdown_chance, v, m, chance);
        if m.shared().settings().vehicle_breakdowns() == 1 {
            // On reduced breakdown.
            svc!(C, VehicleRef::set_breakdown_chance, v, m, 0);
            branch!(C, VEHICLE_MAINT_RESET_REDUCED);
        }
        branch!(C, VEHICLE_MAINT_RESET);
        match v.next() {
            Some(next) if next.has_engine_type() => v = next,
            _ => break,
        }
    }
}

/// `Vehicle::NeedsServicing`: check if the vehicle needs to go to a depot in near future (if a
/// opportunity presents itself) for service or replacement. See `needs_automatic_servicing`.
/// Returns true if the vehicle should go to a depot if a opportunity presents itself.
#[allow(
    clippy::too_many_lines,
    reason = "the original body, with its comments"
)]
fn needs_servicing_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    // Stopped or crashed vehicles will not move, as such making unmovable
    // vehicles to go for service is lame.
    if v.vehstatus().any(
        VehStates::from_element(VehState::Stopped).with(VehStates::from_element(VehState::Crashed)),
    ) {
        return false;
    }

    // Are we ready for the next service cycle?
    let m = Services::of(base);
    let companies = m.company();
    let c = svc!(C, companies.company, v.owner()).cast_const();
    branch!(C, VEHICLE_MAINT_SERVICE);

    let r = resolver::<C>(base);
    // Service intervals can be measured in different units, which we handle individually.
    if v.service_interval_is_percent() {
        // Service interval is in percents.
        branch!(C, VEHICLE_MAINT_PERCENT);
        let reliability = i32::from(svc!(C, VehicleRef::reliability, v, r));
        let engine = svc!(C, base.engine, v);
        let interval = i32::from(svc!(C, VehicleRef::get_service_interval, v, r));
        if reliability >= i32::from(engine.reliability()).wrapping_mul(100 - interval) / 100 {
            return false;
        }
    } else if svc!(C, m.clock().using_wallclock_units, false) {
        // Service interval is in minutes.
        branch!(C, VEHICLE_MAINT_MINUTES);
        let interval = i32::from(svc!(C, VehicleRef::get_service_interval, v, r));
        if svc!(C, VehicleRef::date_of_last_service, v, r)
            .0
            .wrapping_add(interval * DAYS_IN_ECONOMY_MONTH)
            >= economy_date().0
        {
            return false;
        }
    } else {
        // Service interval is in days.
        branch!(C, VEHICLE_MAINT_DAYS);
        let interval = i32::from(svc!(C, VehicleRef::get_service_interval, v, r));
        if svc!(C, VehicleRef::date_of_last_service, v, r)
            .0
            .wrapping_add(interval)
            >= economy_date().0
        {
            return false;
        }
    }

    // If we're servicing anyway, because we have not disabled servicing when
    // there are no breakdowns or we are playing with breakdowns, bail out.
    let settings = m.shared().settings();
    if !settings.no_servicing_if_no_breakdowns() || settings.vehicle_breakdowns() != 0 {
        return true;
    }

    // Test whether there is some pending autoreplace.
    // Note: We do this after the service-interval test.
    // There are a lot more reasons for autoreplace to fail than we can test here reasonably.
    let mut pending_replace = false;
    let mut needed_money = i64::from(svc!(C, companies.engine_renew_money, c));
    if needed_money
        > get_available_money(
            m.commands().shared,
            m.commands().finance,
            svc!(C, companies.index, c.cast_mut()),
        )
    {
        return false;
    }

    let capabilities = m.transaction().capabilities;
    let mut next = Some(v);
    while let Some(v) = next {
        'check: {
            let mut replace_when_old = false;
            let group_id = svc!(C, VehicleRef::group_id, v, m.group());
            let new_engine = engine_replacement_for_company(
                companies,
                c,
                v.engine_type(),
                group_id,
                Some(&mut replace_when_old),
            );

            // Check engine availability.
            if new_engine == EngineID::INVALID {
                break 'check;
            }
            let engine = svc!(C, capabilities.engine, new_engine.0);
            if !CompanyMask(crate::vehicle::EngineRef::of(engine).company_availability())
                .test(v.owner())
            {
                break 'check;
            }
            // Is the vehicle old if we are not always replacing?
            if replace_when_old && !needs_autorenewing::<C>(v, r, companies, c, false) {
                break 'check;
            }

            // Check refittability.
            let (union_mask, available_cargo_types) =
                crate::engine_capabilities::get_articulated_refit_masks(
                    capabilities,
                    new_engine,
                    true,
                );
            // Is there anything to refit?
            if union_mask.0 != 0 {
                let mut cargo_type = CargoType::INVALID_CARGO;
                let cargo_mask = crate::engine_capabilities::get_cargo_types_of_articulated_vehicle(
                    capabilities,
                    v,
                    Some(&mut cargo_type),
                );
                if has_at_most_one_bit(cargo_mask.0) {
                    // Did the old vehicle carry anything?
                    if is_valid_cargo_type(cargo_type.0) {
                        // We can't refit the vehicle to carry the cargo we want.
                        if !has_bit64(available_cargo_types.0, cargo_type.0) {
                            break 'check;
                        }
                    }
                } else {
                    let new_engine_default_cargoes =
                        crate::engine_capabilities::get_cargo_types_of_articulated_parts(
                            capabilities,
                            new_engine,
                        );
                    if cargo_mask.0 & new_engine_default_cargoes.0 != cargo_mask.0 {
                        // We cannot refit to mixed cargoes in an automated way.
                        break 'check;
                    }
                    // engine_type is already a mixed cargo type which matches the incoming
                    // vehicle by default, no refit required.
                }
            }

            // Check money.
            // We want 2*(the price of the new vehicle) without looking at the value of the
            // vehicle we are going to sell.
            pending_replace = true;
            let engine = crate::vehicle::EngineRef::of(svc!(C, capabilities.engine, new_engine.0));
            let cost = crate::engine_capabilities::get_cost(capabilities, engine);
            crate::overflowsafe_type::add_i64(&mut needed_money, cost.saturating_mul(2));
            branch!(C, VEHICLE_MAINT_REPLACE);
            if needed_money
                > get_available_money(
                    m.commands().shared,
                    m.commands().finance,
                    svc!(C, companies.index, c.cast_mut()),
                )
            {
                return false;
            }
        }
        next = if v.vehicle_type() == VehicleType::VEH_TRAIN {
            v.next_unit()
        } else {
            None
        };
    }

    pending_replace
}

/// `Vehicle::NeedsAutomaticServicing`: checks if the current order should be interrupted for a
/// service-in-depot order. See
/// `needs_servicing`. Returns true if the current order should be interrupted.
fn needs_automatic_servicing_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    getter::<C>();
    let orders = base.orders();
    if crate::orders::has_depot_order(v, orders) {
        return false;
    }
    let current_order = v.current_order();
    if current_order.is_type(OrderType::OT_LOADING) {
        return false;
    }
    if current_order.is_type(OrderType::OT_GOTO_DEPOT)
        && !current_order.depot_type().test(OrderDepotTypeFlag::Service)
    {
        return false;
    }
    needs_servicing_impl::<C>(v, base)
}

/// `DecreaseVehicleValue`: decrease the value of a vehicle; `v` is the vehicle to devaluate.
fn decrease_vehicle_value_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let m = Services::of(base);
    let r = resolver::<C>(base);
    let value = svc!(C, VehicleRef::value, v, r);
    svc!(
        C,
        VehicleRef::set_value,
        v,
        m,
        value.saturating_sub(value >> 8)
    );
    set_vehicle_window_dirty::<C>(WindowClass::WC_VEHICLE_DETAILS, v, r);
    branch!(C, VEHICLE_MAINT_VALUE);
}

const BREAKDOWN_CHANCE: [u8; 64] = [
    3, 3, 3, 3, 3, 3, 3, 3, //
    4, 4, 5, 5, 6, 6, 7, 7, //
    8, 8, 9, 9, 10, 10, 11, 11, //
    12, 13, 13, 13, 13, 14, 15, 16, //
    17, 19, 21, 25, 28, 31, 34, 37, //
    40, 44, 48, 52, 56, 60, 64, 68, //
    72, 80, 90, 100, 110, 120, 130, 140, //
    150, 170, 190, 210, 230, 250, 250, 250, //
];

/// `CheckVehicleBreakdown`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's int-to-uint8_t and int-to-uint16_t stores of in-range values"
)]
fn check_vehicle_breakdown_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let m = Services::of(base);
    let settings = m.shared().settings();
    let resolved = std::cell::OnceCell::new();
    let r = || *resolved.get_or_init(|| resolver::<C>(base));
    branch!(C, VEHICLE_MAINT_CHECK);

    // Decrease reliability.
    if !settings.no_servicing_if_no_breakdowns() || settings.vehicle_breakdowns() != 0 {
        let rel_old = i32::from(svc!(C, VehicleRef::reliability, v, r()));
        let rel = (rel_old - i32::from(svc!(C, VehicleRef::reliability_spd_dec, v, r()))).max(0);
        svc!(C, VehicleRef::set_reliability, v, base, rel as u16);
        if rel_old >> 8 != rel >> 8 {
            set_vehicle_window_dirty::<C>(WindowClass::WC_VEHICLE_DETAILS, v, r());
        }
    }

    if v.breakdown_ctr() != 0
        || v.vehstatus().test(VehState::Stopped)
        || settings.vehicle_breakdowns() < 1
        || v.cur_speed() < 5
        || crate::session::mode() == GameMode::GM_MENU
    {
        return;
    }

    let shared = m.shared();
    let random = svc!(C, shared.random, shared.context);

    // Increase chance of failure.
    let mut chance = i32::from(svc!(C, VehicleRef::breakdown_chance, v, r())) + 1;
    if chance16_i(1, 25, random) {
        chance += 25;
    }
    let chance = clamp_to_u8(i64::from(chance));
    svc!(C, VehicleRef::set_breakdown_chance, v, m, chance);

    // Calculate reliability value to use in comparison.
    let mut rel = i32::from(svc!(C, VehicleRef::reliability, v, r()));
    if v.vehicle_type() == VehicleType::VEH_SHIP {
        rel += 0x6666;
    }

    // Reduced breakdowns?
    if settings.vehicle_breakdowns() == 1 {
        rel += 0x6666;
    }

    // Check if to break down; `chance` is the `breakdown_chance` just stored.
    if BREAKDOWN_CHANCE[usize::from(clamp_to_u16(i64::from(rel)) >> 10)] <= chance {
        v.set_breakdown_ctr(gb(random, 16, 6) as u8 + 0x3F);
        svc!(
            C,
            VehicleRef::set_breakdown_delay,
            v,
            m,
            gb(random, 24, 7) as u8 + 0x80
        );
        svc!(C, VehicleRef::set_breakdown_chance, v, m, 0);
        branch!(C, VEHICLE_MAINT_TRIGGER);
    }
}

/// `Vehicle::HandleBreakdown`: handle all of the aspects of a vehicle breakdown. This includes
/// adding smoke and sounds, and ending the breakdown when appropriate. Returns true iff the vehicle
/// is stopped because of a breakdown; this function always returns false for aircraft, since these
/// never stop for breakdowns.
fn handle_breakdown_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) -> bool {
    let resolved = std::cell::OnceCell::new();
    let r = || *resolved.get_or_init(|| resolver::<C>(base));
    // Possible states for Vehicle::breakdown_ctr
    // 0  - vehicle is running normally
    // 1  - vehicle is currently broken down
    // 2  - vehicle is going to break down now
    // >2 - vehicle is counting down to the actual breakdown event
    match v.breakdown_ctr() {
        0 => return false,

        2 => {
            v.set_breakdown_ctr(1);

            let breakdowns = svc!(C, VehicleRef::breakdowns_since_last_service, v, r());
            if breakdowns != 255 {
                svc!(
                    C,
                    VehicleRef::set_breakdowns_since_last_service,
                    v,
                    base,
                    breakdowns + 1
                );
            }
            branch!(C, VEHICLE_MAINT_STARTED);

            if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                // Aircraft just need this flag, the rest is handled elsewhere.
                v.set_vehstatus(v.vehstatus().set(VehState::AircraftBroken));
                branch!(C, VEHICLE_MAINT_AIRCRAFT);
            } else {
                v.set_cur_speed(0);

                if !svc!(C, base.play_vehicle_sound, v, SoundEvent::VSE_BREAKDOWN) {
                    let train_or_ship = v.vehicle_type() == VehicleType::VEH_TRAIN
                        || v.vehicle_type() == VehicleType::VEH_SHIP;
                    let landscape = Services::of(base).shared().settings().landscape();
                    let sound = if landscape != LandscapeType::Toyland {
                        if train_or_ship {
                            SoundFx::SND_10_BREAKDOWN_TRAIN_SHIP
                        } else {
                            SoundFx::SND_0F_BREAKDOWN_ROADVEHICLE
                        }
                    } else if train_or_ship {
                        SoundFx::SND_3A_BREAKDOWN_TRAIN_SHIP_TOYLAND
                    } else {
                        SoundFx::SND_35_BREAKDOWN_ROADVEHICLE_TOYLAND
                    };
                    svc!(C, base.play_sound, v, u16::from(sound.0));
                }

                if !v.vehstatus().test(VehState::Hidden)
                    && !EngineMiscFlags(svc!(C, base.engine, v).misc())
                        .test(EngineMiscFlag::NoBreakdownSmoke)
                {
                    let u = crate::effect::create_effect_vehicle_rel(
                        v,
                        4,
                        4,
                        5,
                        EffectVehicleType::EV_BREAKDOWN_SMOKE,
                    );
                    if let Some(u) = u {
                        let delay = svc!(C, VehicleRef::breakdown_delay, v, r());
                        u.set_animation_state(u16::from(delay) * 2);
                        branch!(C, VEHICLE_MAINT_SMOKE);
                    }
                }
            }

            svc!(C, Services::of(base).mark_dirty, v); // Update graphics after speed is zeroed
            set_vehicle_window_dirty::<C>(WindowClass::WC_VEHICLE_VIEW, v, r());
            set_vehicle_window_dirty::<C>(WindowClass::WC_VEHICLE_DETAILS, v, r());
            // Fall through to the broken-down case.
        }

        1 => {}

        counter => {
            if v.current_order().is_type(OrderType::OT_LOADING) {
                branch!(C, VEHICLE_MAINT_LOADING);
            } else {
                v.set_breakdown_ctr(counter - 1);
                branch!(C, VEHICLE_MAINT_COUNTDOWN);
            }
            return false;
        }
    }

    // Aircraft breakdowns end only when arriving at the airport.
    if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
        return false;
    }

    // For trains this function is called twice per tick, so decrease v->breakdown_delay at
    // half the rate.
    if v.tick_counter()
        & if v.vehicle_type() == VehicleType::VEH_TRAIN {
            3
        } else {
            1
        }
        == 0
    {
        let delay = svc!(C, VehicleRef::breakdown_delay, v, r()).wrapping_sub(1);
        svc!(
            C,
            VehicleRef::set_breakdown_delay,
            v,
            Services::of(base),
            delay
        );
        branch!(C, VEHICLE_MAINT_DELAY);
        if delay == 0 {
            v.set_breakdown_ctr(0);
            svc!(C, Services::of(base).mark_dirty, v);
            set_vehicle_window_dirty::<C>(WindowClass::WC_VEHICLE_VIEW, v, r());
            branch!(C, VEHICLE_MAINT_ENDED);
        }
    }
    true
}

/// `EconomyAgeVehicle`: update economy age of a vehicle; `v` is the vehicle to update.
fn economy_age_vehicle_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let m = Services::of(base);
    let economy_age = svc!(C, VehicleRef::economy_age, v, m);
    if economy_age.0 < MAX_DATE {
        let economy_age = EconomyDate(economy_age.0 + 1);
        svc!(C, VehicleRef::set_economy_age, v, m, economy_age);
        if v.is_primary_vehicle() && economy_age.0 == VEHICLE_PROFIT_MIN_AGE.0 + 1 {
            fleet::vehicle_reached_min_age(m.group(), v);
            branch!(C, VEHICLE_MAINT_PROFIT_AGE);
        }
    } else {
        branch!(C, VEHICLE_MAINT_MAX_AGE);
    }
}

/// `AgeVehicle`: update age of a vehicle; `v` is the vehicle to update.
fn age_vehicle_impl<const C: bool>(v: VehicleRef, base: &VehicleServices) {
    let m = Services::of(base);
    let r = resolver::<C>(base);
    let mut calendar_age = svc!(C, VehicleRef::age, v, r);
    if calendar_age.0 < MAX_DATE {
        calendar_age = CalendarDate(calendar_age.0 + 1);
        svc!(C, VehicleRef::set_age, v, m, calendar_age);
    } else {
        branch!(C, VEHICLE_MAINT_MAX_AGE);
    }

    if !v.is_primary_vehicle()
        && (v.vehicle_type() != VehicleType::VEH_TRAIN
            || !crate::ground_vehicle::is_engine(v.subtype()))
    {
        return;
    }

    let age = calendar_age
        .0
        .wrapping_sub(svc!(C, VehicleRef::max_age, v, r).0);
    for i in 0..=4 {
        if age == date_at_start_of_year(i) {
            let decay = svc!(C, VehicleRef::reliability_spd_dec, v, r);
            svc!(
                C,
                VehicleRef::set_reliability_spd_dec,
                v,
                m,
                decay.wrapping_shl(1)
            );
            branch!(C, VEHICLE_MAINT_DECAY);
            break;
        }
    }

    let index = svc!(C, VehicleRef::index, v, r);
    set_window_dirty(WindowClass::WC_VEHICLE_DETAILS, WindowNumber::from(index));
    branch!(C, VEHICLE_MAINT_AGE);

    // Don't warn if warnings are disabled.
    let shared = m.shared();
    if !shared.settings().old_vehicle_warn() {
        return;
    }

    // Don't warn about vehicles which are non-primary (e.g., part of an articulated vehicle),
    // don't belong to us, are crashed, or are stopped.
    if v.previous().is_some()
        || v.owner() != CompanyID(svc!(C, shared.local_company))
        || v.vehstatus().any(
            VehStates::from_element(VehState::Crashed)
                .with(VehStates::from_element(VehState::Stopped)),
        )
    {
        return;
    }

    let companies = m.company();
    let c = svc!(C, companies.company, v.owner()).cast_const();
    // Don't warn if a renew is active.
    if svc!(C, companies.engine_renew, c)
        && CompanyMask(svc!(C, base.engine, v).company_availability()).any()
    {
        return;
    }
    // Don't warn if a replacement is active.
    let group_id = svc!(C, VehicleRef::group_id, v, m.group());
    if engine_has_replacement_for_company(companies, c, v.engine_type(), group_id) {
        return;
    }

    let str = if age == date_at_start_of_year(-1) {
        STR_NEWS_VEHICLE_IS_GETTING_OLD
    } else if age == date_at_start_of_year(0) {
        STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD
    } else if age > date_at_start_of_year(0) && age % DAYS_IN_LEAP_YEAR == 0 {
        STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD_AND
    } else {
        return;
    };

    add_vehicle_advice_news_item(
        AdviceType::VehicleOld,
        &get_encoded_string(str, &[Integer(u64::from(index.0))]),
        index,
    );
    branch!(C, VEHICLE_MAINT_OLD_NEWS);
}

/// One entry per original function: it selects the witness once and runs the body.
macro_rules! entry {
    ($(#[$doc:meta])* $direct:ident, $body:ident $(-> $ret:ty)?) => {
        $(#[$doc])*
        pub(crate) fn $direct(v: VehicleRef, base: &VehicleServices) $(-> $ret)? {
            if crate::witness::enabled() {
                crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
                $body::<true>(v, base)
            } else {
                $body::<false>(v, base)
            }
        }
    };
}
entry!(
    /// `AgeVehicle`.
    age_vehicle,
    age_vehicle_impl
);
entry!(
    /// `CheckVehicleBreakdown`.
    check_vehicle_breakdown,
    check_vehicle_breakdown_impl
);
entry!(
    /// `DecreaseVehicleValue`.
    decrease_vehicle_value,
    decrease_vehicle_value_impl
);
entry!(
    /// `EconomyAgeVehicle`.
    economy_age_vehicle,
    economy_age_vehicle_impl
);
entry!(
    /// `Vehicle::NeedsAutomaticServicing`.
    needs_automatic_servicing,
    needs_automatic_servicing_impl -> bool
);
entry!(
    /// `Vehicle::NeedsServicing`.
    needs_servicing,
    needs_servicing_impl -> bool
);
entry!(
    /// `VehicleServiceInDepot`.
    vehicle_service_in_depot,
    vehicle_service_in_depot_impl
);

entry!(
    /// `Vehicle::HandleBreakdown`.
    handle_breakdown,
    handle_breakdown_impl -> bool
);

// The C++ facades' entries. Each takes a live shell's record and the live vehicle table.

/// `AgeVehicle`.
/// # Safety
/// `base` is the live vehicle table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_age(v: VehicleRef, base: *const VehicleServices) {
    age_vehicle(v, unsafe { &*base });
}
/// `DecreaseVehicleValue`.
/// # Safety
/// `base` is the live vehicle table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_decrease_value(
    v: VehicleRef,
    base: *const VehicleServices,
) {
    decrease_vehicle_value(v, unsafe { &*base });
}
/// `EconomyAgeVehicle`.
/// # Safety
/// `base` is the live vehicle table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_economy_age(
    v: VehicleRef,
    base: *const VehicleServices,
) {
    economy_age_vehicle(v, unsafe { &*base });
}
/// `Vehicle::HandleBreakdown`.
/// # Safety
/// `base` is the live vehicle table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_handle_breakdown_abi(
    v: VehicleRef,
    base: *const VehicleServices,
) -> u8 {
    u8::from(handle_breakdown(v, unsafe { &*base }))
}
/// `Vehicle::NeedsAutomaticServicing`.
/// # Safety
/// `base` is the live vehicle table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_automatic_service_abi(
    v: VehicleRef,
    base: *const VehicleServices,
) -> u8 {
    u8::from(needs_automatic_servicing(v, unsafe { &*base }))
}
/// `Vehicle::NeedsAutorenewing`, resolving the vehicle's resolver table.
fn needs_autorenewing_entry<const C: bool>(
    v: VehicleRef,
    base: &VehicleServices,
    c: *const OpenTTDCompany,
    use_renew_setting: bool,
) -> bool {
    let r = resolver::<C>(base);
    needs_autorenewing::<C>(v, r, Services::of(base).company(), c, use_renew_setting)
}
/// `Vehicle::NeedsAutorenewing`; `company` is the vehicle owner the caller passes.
/// # Safety
/// `base` is the live vehicle table and `company` a live company.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_renew_abi(
    v: VehicleRef,
    base: *const VehicleServices,
    company: *const OpenTTDCompany,
    use_renew_setting: bool,
) -> u8 {
    let base = unsafe { &*base };
    u8::from(if crate::witness::enabled() {
        crate::witness::hit(crate::witness::VEHICLE_MAINT_ENTRIES);
        needs_autorenewing_entry::<true>(v, base, company, use_renew_setting)
    } else {
        needs_autorenewing_entry::<false>(v, base, company, use_renew_setting)
    })
}
/// `Vehicle::NeedsServicing`.
/// # Safety
/// `base` is the live vehicle table.
#[allow(unsafe_code, reason = "the C++ facade's entry")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_needs_service_abi(
    v: VehicleRef,
    base: *const VehicleServices,
) -> u8 {
    u8::from(needs_servicing(v, unsafe { &*base }))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleMaintenanceServices" ["maintenance"] => Services {
        clock,
        company,
        economy_age,
        group,
        mark_dirty,
        set_age,
        set_breakdown_chance,
        set_breakdown_delay,
        set_economy_age,
        set_reliability_spd_dec,
        set_value,
        shared,
        transaction,
    };
}

/// The maintenance bodies over recording services: each member slot reads and writes this
/// thread's `Native` members, and the record holds the counter, tick, speed, type, status,
/// consist flags and current order. Expectations are `vehicle.cpp`'s.
#[cfg(test)]
#[allow(
    unsafe_code,
    reason = "the recording services and the fixture's own records"
)]
mod tests {
    use super::*;
    use crate::company_identity::Services as Companies;
    use crate::engine::Lifecycle;
    use crate::group_type::GroupID;
    use crate::news_func::testing::{Effect as News, Param, encoded};
    use crate::orders::{Consist, Order};
    use crate::test_tables::{
        CLOCK, COMMANDS, COMPANY_IDENTITY, FINANCE, FLEET_GROUPS, FLEET_TRANSACTIONS, SHARED,
        VEHICLE,
    };
    use crate::vehicle::EngineRef;
    use crate::vehicle_base::VehicleRecord;
    use crate::vehicle_type::OpenTTDVehicle;
    use crate::window_func::testing::Effect as Window;
    use std::cell::{Cell, RefCell};
    use std::ptr;

    /// The `Vehicle` members still stored in the C++ shell.
    #[derive(Clone, Copy)]
    struct Native {
        age: i32,
        max_age: i32,
        economy_age: i32,
        value: i64,
        breakdown_delay: u8,
        breakdowns_since_last_service: u8,
        breakdown_chance: u8,
        reliability: u16,
        reliability_spd_dec: u16,
        service_interval: u16,
    }
    const NATIVE_DEFAULT: Native = Native {
        age: 0,
        max_age: 0,
        economy_age: 0,
        value: 0,
        breakdown_delay: 0,
        breakdowns_since_last_service: 0,
        breakdown_chance: 0,
        reliability: 0,
        reliability_spd_dec: 0,
        service_interval: 0,
    };
    const INDEX: u32 = 41;
    thread_local! {
        static NATIVE: Cell<Native> = const { Cell::new(NATIVE_DEFAULT) };
        /// Whether the dirty callback makes the vehicle an aircraft.
        static REENTER: Cell<bool> = const { Cell::new(false) };
        /// The engine `GetEngine()` and `Engine::Get` return.
        static ENGINE: Cell<*const Lifecycle> = const { Cell::new(ptr::null()) };
        /// The company, money and price reads in the order they happen.
        static READS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }
    fn native() -> Native {
        NATIVE.get()
    }
    fn set(f: impl FnOnce(&mut Native)) {
        let mut n = NATIVE.get();
        f(&mut n);
        NATIVE.set(n);
    }
    fn read(name: &'static str) {
        READS.with(|r| r.borrow_mut().push(name));
    }
    fn reads() -> Vec<&'static str> {
        READS.with(|r| std::mem::take(&mut *r.borrow_mut()))
    }
    fn rec(v: VehicleRef) -> *mut VehicleRecord {
        v.0.as_ptr()
    }

    // The resolver's member slots.
    unsafe extern "C" fn age(_: *mut OpenTTDVehicle) -> i32 {
        native().age
    }
    unsafe extern "C" fn max_age(_: *mut OpenTTDVehicle) -> i32 {
        native().max_age
    }
    unsafe extern "C" fn breakdowns(_: *mut OpenTTDVehicle) -> u8 {
        native().breakdowns_since_last_service
    }
    unsafe extern "C" fn breakdown_delay(_: *mut OpenTTDVehicle) -> u8 {
        native().breakdown_delay
    }
    unsafe extern "C" fn breakdown_chance(_: *mut OpenTTDVehicle) -> u8 {
        native().breakdown_chance
    }
    unsafe extern "C" fn reliability(_: *mut OpenTTDVehicle) -> u16 {
        native().reliability
    }
    unsafe extern "C" fn reliability_decay(_: *mut OpenTTDVehicle) -> u16 {
        native().reliability_spd_dec
    }
    unsafe extern "C" fn value(_: *mut OpenTTDVehicle) -> i64 {
        native().value
    }
    unsafe extern "C" fn id(_: *mut OpenTTDVehicle) -> u32 {
        INDEX
    }
    unsafe extern "C" fn service_interval(_: *mut OpenTTDVehicle) -> u16 {
        native().service_interval
    }
    unsafe extern "C" fn date_of_last_service(_: *mut OpenTTDVehicle) -> i32 {
        0
    }
    unsafe extern "C" fn engine_by_id(id: u16) -> *const Lifecycle {
        assert_eq!(id, NEW_ID, "the new engine's property");
        ENGINE.get()
    }
    /// The new engine's cost factor callback, answering `PRICE.factor` and then running
    /// `PRICE.then`, as a `NewGRF` callback may change any state before the second budget test.
    unsafe extern "C" fn resolve_callback(
        _: *mut crate::newgrf_vehicle::State,
        id: u16,
        callback: u16,
        property: u32,
        _: u32,
        _: *mut i32,
        _: usize,
    ) -> u16 {
        assert_eq!(id, NEW_ID, "the new engine's callback");
        assert_eq!(
            callback,
            crate::newgrf_callbacks::CallbackID::CBID_VEHICLE_MODIFY_PROPERTY.0
        );
        assert_eq!(
            property,
            u32::from(crate::newgrf_properties::PropertyID::PROP_ROADVEH_COST_FACTOR.0)
        );
        read("price");
        let price = PRICE.get();
        (price.then)();
        price.factor
    }
    extern "C" fn grf_price_shift(_: *const std::ffi::c_void, index: u32) -> i32 {
        assert_eq!(index, crate::economy_type::PR_BUILD_VEHICLE_ROAD);
        read("shift");
        0
    }
    #[derive(Clone, Copy)]
    struct Price {
        factor: u16,
        then: fn(),
    }
    thread_local! {
        static PRICE: Cell<Price> = const {
            Cell::new(Price { factor: crate::newgrf_callbacks::CALLBACK_FAILED, then: || {} })
        };
    }

    // The maintenance table's member slots.
    extern "C" fn set_age(_: VehicleRef, value: i32) {
        set(|n| n.age = value);
    }
    extern "C" fn economy_age(_: VehicleRef) -> i32 {
        native().economy_age
    }
    extern "C" fn set_economy_age(_: VehicleRef, value: i32) {
        set(|n| n.economy_age = value);
    }
    extern "C" fn set_value(_: VehicleRef, value: i64) {
        set(|n| n.value = value);
    }
    extern "C" fn set_breakdown_delay(_: VehicleRef, value: u8) {
        set(|n| n.breakdown_delay = value);
    }
    extern "C" fn set_breakdown_chance(_: VehicleRef, value: u8) {
        set(|n| n.breakdown_chance = value);
    }
    extern "C" fn set_reliability_spd_dec(_: VehicleRef, value: u16) {
        set(|n| n.reliability_spd_dec = value);
    }
    /// `Vehicle::MarkDirty`; once `REENTER` is set it sees the hidden flag the breakdown
    /// sound set, then makes the vehicle an aircraft.
    extern "C" fn mark_dirty(v: VehicleRef) {
        if REENTER.get() {
            assert!(v.vehstatus().test(VehState::Hidden));
            // SAFETY: The test's live record.
            unsafe { (*rec(v)).r#type = VehicleType::VEH_AIRCRAFT };
        }
    }

    // The vehicle table's slots.
    extern "C" fn write_breakdowns(_: VehicleRef, value: u8) {
        set(|n| n.breakdowns_since_last_service = value);
    }
    extern "C" fn write_reliability(_: VehicleRef, value: u16) {
        set(|n| n.reliability = value);
    }
    extern "C" fn write_service(_: VehicleRef, _: i32) {}
    /// Service intervals in days.
    extern "C" fn using_wallclock_units(_: bool) -> bool {
        false
    }
    extern "C" fn engine(_: VehicleRef) -> EngineRef {
        EngineRef::of(ENGINE.get())
    }
    /// The breakdown sound sees the zeroed speed, the started counter and the saturated
    /// count, then hides the vehicle.
    extern "C" fn breakdown_sound(
        v: VehicleRef,
        event: crate::newgrf_sound::VehicleSoundEvent,
    ) -> bool {
        assert_eq!(event.0, SoundEvent::VSE_BREAKDOWN.0);
        assert_eq!((v.cur_speed(), v.breakdown_ctr()), (0, 1));
        assert_eq!(native().breakdowns_since_last_service, 255);
        v.set_vehstatus(v.vehstatus().set(VehState::Hidden));
        true
    }
    extern "C" fn newgrf() -> *const newgrf_vehicle::Services {
        thread_local! {
            static TABLE: &'static newgrf_vehicle::Services = {
                let mut t =
                    newgrf_vehicle::test_table_with_callback(engine_by_id, resolve_callback);
                t.age = age;
                t.max_age = max_age;
                t.breakdowns = breakdowns;
                t.breakdown_delay = breakdown_delay;
                t.breakdown_chance = breakdown_chance;
                t.reliability = reliability;
                t.reliability_decay = reliability_decay;
                t.value = value;
                t.id = id;
                t.service_interval = service_interval;
                t.date_of_last_service = date_of_last_service;
                t.vehicle = resolver_vehicles();
                t.capabilities = capabilities();
                Box::leak(Box::new(t))
            };
        }
        TABLE.with(|t| ptr::from_ref(*t))
    }

    // The vehicle's group and the group and renewal pools.
    extern "C" fn vehicle_group(_: VehicleRef) -> GroupID {
        GroupID::DEFAULT_GROUP
    }
    extern "C" fn renew_state(renew: *mut ()) -> *mut crate::fleet::Renew {
        renew.cast()
    }
    extern "C" fn no_group(_: GroupID) -> crate::fleet::GroupRecord {
        crate::fleet::GroupRecord {
            shell: ptr::null_mut(),
            group: ptr::null_mut(),
            statistics: ptr::null_mut(),
            index: 0,
        }
    }

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn capabilities() -> &'static crate::engine_capabilities::Services {
        extern "C" fn engine_of(_: u16) -> *const Lifecycle {
            ENGINE.get()
        }
        extern "C" fn engine_id(_: *const Lifecycle) -> u16 {
            NEW_ID
        }
        let mut c = crate::engine_capabilities::test_table_with_shared(shared());
        c.engine = engine_of;
        c.id = engine_id;
        c.vehicles = resolver_vehicles();
        leak(c)
    }
    /// The vehicle table the property callback resolves through.
    fn resolver_vehicles() -> &'static crate::vehicle::VehicleServices {
        leak(crate::vehicle::VehicleServices { newgrf, ..VEHICLE })
    }
    fn shared() -> &'static services::Services {
        thread_local! {
            static SETTINGS: Settings = const { Settings::new() };
            static TABLE: &'static services::Services = leak(services::Services {
                settings: leak(crate::settings_type::SettingAddresses {
                    climate: SETTINGS.with(|s| s.climate.as_ptr()),
                    no_servicing_if_no_breakdowns: SETTINGS.with(|s| s.no_service.as_ptr()),
                    old_vehicle_warn: SETTINGS.with(|s| s.warn.as_ptr()),
                    vehicle_breakdowns: SETTINGS.with(|s| s.breakdowns.as_ptr()),
                    infinite_money: SETTINGS.with(|s| s.infinite.as_ptr()),
                    ..crate::settings_type::SettingAddresses::NULL
                }),
                grf_price_shift,
                local_company,
                ..SHARED
            });
        }
        TABLE.with(|t| *t)
    }
    /// The settings the bodies read.
    struct Settings {
        climate: Cell<u8>,
        no_service: Cell<bool>,
        warn: Cell<bool>,
        breakdowns: Cell<u8>,
        infinite: Cell<bool>,
    }
    impl Settings {
        const fn new() -> Self {
            Self {
                climate: Cell::new(0),
                no_service: Cell::new(false),
                warn: Cell::new(false),
                breakdowns: Cell::new(0),
                infinite: Cell::new(false),
            }
        }
    }
    fn settings(f: impl FnOnce(&crate::settings_type::SettingAddresses)) {
        f(shared().settings());
    }
    fn store<T>(address: *const T, value: T) {
        // SAFETY: Each address is one of this thread's `Settings` cells.
        unsafe { address.cast_mut().write(value) };
    }
    extern "C" fn local_company() -> u8 {
        0
    }
    fn vehicles() -> crate::vehicle::VehicleServices {
        crate::vehicle::VehicleServices {
            engine,
            maintenance: leak(maintenance()),
            newgrf,
            play_vehicle_sound: breakdown_sound,
            write_breakdowns,
            write_calendar_service: write_service,
            write_economy_service: write_service,
            write_reliability,
            ..VEHICLE
        }
    }
    fn maintenance() -> Services {
        Services {
            clock: leak(crate::timer_game_common::ClockServices {
                using_wallclock_units,
                ..CLOCK
            }),
            company: companies(),
            economy_age,
            group: leak(fleet::GroupServices {
                group_get_if_valid: no_group,
                renew_state,
                group_id: vehicle_group,
                ..FLEET_GROUPS
            }),
            mark_dirty,
            set_age,
            set_breakdown_chance,
            set_breakdown_delay,
            set_economy_age,
            set_reliability_spd_dec,
            set_value,
            shared: shared(),
            transaction: leak(fleet::transactions::Services {
                commands: leak(services::CommandServices {
                    finance: leak(crate::company::FinanceServices {
                        company_finances,
                        ..FINANCE
                    }),
                    shared: shared(),
                    ..COMMANDS
                }),
                capabilities: capabilities(),
                ..FLEET_TRANSACTIONS
            }),
        }
    }

    // The company: its identity, renewal settings and list, and its finances.
    static COMPANY: u8 = 0;
    static OTHER_COMPANY: u8 = 0;
    fn company() -> *const OpenTTDCompany {
        ptr::from_ref(&COMPANY).cast()
    }
    #[derive(Clone, Copy)]
    struct Owner {
        /// The `CompanyID` `c->index` names; `COMPANY_SPECTATOR` is not a valid company.
        id: CompanyID,
        renew: bool,
        months: i16,
        money: i64,
        /// The rule `OLD` to `NEW` in the default group, replacing when old by setting.
        rule: Option<bool>,
    }
    const OLD: EngineID = EngineID(5);
    /// The new engine's pool index, which its property callback is asked for.
    const NEW_ID: u16 = 123;
    const NEW: EngineID = EngineID(9);
    thread_local! {
        static OWNER: Cell<Owner> = const {
            Cell::new(Owner {
                id: CompanyID(0),
                renew: false,
                months: 0,
                money: 0,
                rule: None,
            })
        };
        static RULE: Cell<crate::fleet::Renew> = const {
            Cell::new(crate::fleet::Renew {
                from: OLD.0,
                to: NEW.0,
                next: ptr::null_mut(),
                group_id: GroupID::DEFAULT_GROUP.0,
                replace_when_old: false,
            })
        };
        /// Two zeroed finance records, every member an integer: the owner's, and the one a
        /// price callback makes the owner's.
        static FINANCES: [*mut crate::company::Finances; 2] = [
            Box::into_raw(Box::new(unsafe { std::mem::zeroed() })),
            Box::into_raw(Box::new(unsafe { std::mem::zeroed() })),
        ];
        /// Which of `FINANCES` the company's finances are.
        static ACTIVE: Cell<usize> = const { Cell::new(0) };
    }
    fn owner(f: impl FnOnce(&mut Owner)) {
        let mut o = OWNER.get();
        f(&mut o);
        OWNER.set(o);
    }
    fn companies() -> &'static Companies {
        extern "C" fn company_of(id: CompanyID) -> *mut OpenTTDCompany {
            assert_eq!(id, CompanyID(0), "the vehicle's owner");
            company().cast_mut()
        }
        extern "C" fn company_id(c: *mut OpenTTDCompany) -> CompanyID {
            assert_eq!(c.cast_const(), company());
            read("company_id");
            OWNER.get().id
        }
        extern "C" fn engine_renew(_: *const OpenTTDCompany) -> bool {
            OWNER.get().renew
        }
        extern "C" fn engine_renew_money(c: *const OpenTTDCompany) -> u32 {
            assert_eq!(c, company());
            read("reserve");
            100
        }
        extern "C" fn engine_renew_months(_: *const OpenTTDCompany) -> i16 {
            OWNER.get().months
        }
        thread_local! {
            /// The company's renewal owner: the Rust-owned slot holding its list head.
            static HEAD: Cell<*mut ()> = const { Cell::new(ptr::null_mut()) };
        }
        extern "C" fn renewal_head(c: *const OpenTTDCompany) -> *mut *mut () {
            assert_eq!(c, company(), "the owner's renewal list");
            let list = OWNER.get().rule.map_or(ptr::null_mut(), |when_old| {
                RULE.with(|r| {
                    let mut rule = r.get();
                    rule.replace_when_old = when_old;
                    r.set(rule);
                    r.as_ptr().cast()
                })
            });
            HEAD.with(|h| {
                h.set(list);
                h.as_ptr()
            })
        }
        leak(Companies {
            company: company_of,
            index: company_id,
            engine_renew,
            engine_renew_money,
            engine_renew_months,
            renewal_head,
            groups: leak(fleet::GroupServices {
                group_get_if_valid: no_group,
                renew_state,
                ..FLEET_GROUPS
            }),
            ..COMPANY_IDENTITY
        })
    }
    extern "C" fn company_finances(id: u8) -> *mut crate::company::Finances {
        read("finances");
        assert_eq!(id, OWNER.get().id.0);
        if id == CompanyID::SPECTATOR.0 {
            return ptr::null_mut();
        }
        FINANCES.with(|f| f[ACTIVE.get()])
    }
    /// The price callback's change of the company's finances to the other record, holding
    /// `money`: a later money read that reused the first record would see its balance.
    fn swap(money: i64) {
        // SAFETY: This thread's finances.
        FINANCES.with(|f| unsafe { (*f[1]).money = money });
        ACTIVE.set(1);
    }

    /// A road vehicle of company 0 with engine `OLD`, its consist and order state.
    struct Fixture {
        record: *mut VehicleRecord,
        engine: *mut Lifecycle,
        base: &'static crate::vehicle::VehicleServices,
    }
    impl Fixture {
        fn new() -> Self {
            // Bind the recording window and news services.
            crate::window_func::testing::take();
            crate::news_func::testing::take();
            NATIVE.set(NATIVE_DEFAULT);
            REENTER.set(false);
            let mut engine = Box::new(Lifecycle::default());
            engine.r#type = VehicleType::VEH_ROAD;
            engine.vehicle_info_type = VehicleType::VEH_ROAD;
            // No capacity, so nothing to refit.
            engine.vehicle_info = crate::engine::records::VehicleInfo {
                road: crate::engine::records::RoadVehicleInfo {
                    cost_factor: 53,
                    ..Default::default()
                },
            };
            engine.company_avail = 1;
            let engine = Box::into_raw(engine);
            ENGINE.set(engine);
            let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
                ptr::null_mut(),
                VehicleType::VEH_ROAD,
            );
            // SAFETY: The fresh record this fixture owns.
            unsafe {
                (*record).engine_type = OLD;
                (*record).owner = CompanyID(0);
                (*record).consist = Box::into_raw(Box::new(Consist {
                    current_order_time: 0,
                    lateness_counter: 0,
                    timetable_start: 0,
                    depot_unbunching_last_departure: 0,
                    depot_unbunching_next_departure: 0,
                    round_trip_time: 0,
                    cur_real_order_index: 0,
                    cur_implicit_order_index: 0,
                    vehicle_flags: 0,
                }));
                (*record).orders = crate::orders::openttd_rust_vehicle_orders_new();
            }
            Self {
                record,
                engine,
                base: leak(vehicles()),
            }
        }
        fn set_owner(&self, company: CompanyID) {
            // SAFETY: The fixture's record.
            unsafe { (*self.record).owner = company };
        }
        fn v(&self) -> VehicleRef {
            VehicleRef::of(self.record)
        }
        fn order(&self, kind: OrderType) {
            // SAFETY: The fixture's order state.
            unsafe {
                (*(*self.record).orders).current_order.write(Order {
                    r#type: kind.0,
                    flags: 0,
                    dest: 0,
                    refit_cargo: 0,
                    wait_time: 0,
                    travel_time: 0,
                    max_speed: 0,
                });
            }
        }
        /// Set the type, counter and tick of the record and the native delay, then break down.
        fn breakdown(&self, kind: VehicleType, counter: u8, tick: u8, delay: u8) -> bool {
            // SAFETY: The fixture's record.
            unsafe {
                (*self.record).r#type = kind;
                (*self.record).breakdown_ctr = counter;
                (*self.record).tick_counter = tick;
            }
            set(|n| n.breakdown_delay = delay);
            handle_breakdown(self.v(), self.base)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            ENGINE.set(ptr::null());
            // SAFETY: Allocated by `new`; nothing refers to them any more.
            unsafe {
                drop(Box::from_raw((*self.record).consist));
                crate::vehicle_base::openttd_rust_vehicle_record_delete(self.record);
                drop(Box::from_raw(self.engine));
            }
        }
    }
    fn details() -> Window {
        Window::SetWindowDirty(
            WindowClass::WC_VEHICLE_DETAILS,
            WindowNumber::from(crate::vehicle_type::VehicleID(INDEX)),
        )
    }
    fn view() -> Window {
        Window::SetWindowDirty(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(crate::vehicle_type::VehicleID(INDEX)),
        )
    }

    /// Ages and value: calendar and economy ages saturate at `MAX_DATE`; value loses
    /// `value >> 8`, rounding a negative value's loss downward, and dirties the details.
    #[test]
    fn ages_saturate_and_value_decays() {
        let f = Fixture::new();
        crate::window_func::testing::take();
        set(|n| n.age = MAX_DATE - 1);
        for _ in 0..2 {
            age_vehicle(f.v(), f.base);
            assert_eq!(native().age, MAX_DATE);
        }
        set(|n| n.economy_age = MAX_DATE - 1);
        for _ in 0..2 {
            economy_age_vehicle(f.v(), f.base);
            assert_eq!(native().economy_age, MAX_DATE);
        }
        crate::window_func::testing::take();
        for (before, after) in [(65535, 65280), (-257, -255)] {
            set(|n| n.value = before);
            decrease_vehicle_value(f.v(), f.base);
            assert_eq!(native().value, after, "value {before}");
            assert_eq!(crate::window_func::testing::take(), [details()]);
        }
    }

    /// `HandleBreakdown`'s cadence, its rereads after the reentrant sound and dirtying, and
    /// the loading countdown.
    #[test]
    fn breakdown_cadence_rereads_and_countdown() {
        let f = Fixture::new();
        let v = f.v();
        crate::window_func::testing::take();

        // A broken-down train decrements its delay every fourth tick.
        for tick in [1, 2, 3] {
            assert!(f.breakdown(VehicleType::VEH_TRAIN, 1, tick, 3));
            assert_eq!(native().breakdown_delay, 3, "train tick {tick}");
        }
        assert!(f.breakdown(VehicleType::VEH_TRAIN, 1, 4, 3));
        assert_eq!(native().breakdown_delay, 2);
        // A road vehicle's last delay ends the breakdown.
        assert!(f.breakdown(VehicleType::VEH_ROAD, 1, 2, 1));
        assert_eq!((v.breakdown_ctr(), native().breakdown_delay), (0, 0));
        assert_eq!(crate::window_func::testing::take(), [view()]);
        // Aircraft never stop for breakdowns.
        assert!(!f.breakdown(VehicleType::VEH_AIRCRAFT, 1, 2, 1));
        assert_eq!((v.breakdown_ctr(), native().breakdown_delay), (1, 1));
        // The native uint8_t pre-decrement wraps.
        assert!(f.breakdown(VehicleType::VEH_ROAD, 1, 2, 0));
        assert_eq!(native().breakdown_delay, 255);

        // Starting a breakdown rereads the hidden flag after the sound and the type after
        // the dirty callback, and saturates the count at 255.
        REENTER.set(true);
        v.set_cur_speed(10);
        set(|n| n.breakdowns_since_last_service = 255);
        assert!(!f.breakdown(VehicleType::VEH_ROAD, 2, 2, 5));
        REENTER.set(false);
        assert_eq!(
            (native().breakdown_delay, v.vehicle_type()),
            (5, VehicleType::VEH_AIRCRAFT)
        );
        assert_eq!(crate::window_func::testing::take(), [view(), details()]);
        // A first breakdown since service counts.
        v.set_vehstatus(VehStates(0));
        set(|n| n.breakdowns_since_last_service = 3);
        assert!(!f.breakdown(VehicleType::VEH_AIRCRAFT, 2, 2, 5));
        assert!(v.vehstatus().test(VehState::AircraftBroken));
        assert_eq!(native().breakdowns_since_last_service, 4);

        // A counting-down vehicle waits while loading, then counts.
        f.order(OrderType::OT_LOADING);
        assert!(!f.breakdown(VehicleType::VEH_EFFECT, 3, 2, 0));
        assert_eq!(v.breakdown_ctr(), 3);
        f.order(OrderType::OT_DUMMY);
        assert!(!f.breakdown(VehicleType::VEH_EFFECT, 3, 2, 0));
        assert_eq!(v.breakdown_ctr(), 2);
        // A running vehicle does nothing.
        assert!(!f.breakdown(VehicleType::VEH_EFFECT, 0, 2, 0));
    }

    /// The failed assertions the recording policy saw in this test process.
    static FAILED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    extern "C" fn record_failure(condition: bool) {
        if !condition {
            FAILED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// `NeedsAutorenewing` asserts that it was given the owner's company, looking it up only
    /// for the assertion; under this test's recording policy it then goes on.
    #[test]
    fn needs_autorenewing_asserts_the_owner() {
        if crate::services_assert::isolate_test_check(
            "vehicle::maintenance::tests::needs_autorenewing_asserts_the_owner",
            record_failure,
        ) {
            return;
        }
        let f = Fixture::new();
        set(|n| {
            n.age = 100;
            n.max_age = 100;
        });
        owner(|o| o.months = -4);
        let failed = || FAILED.load(std::sync::atomic::Ordering::SeqCst);
        // The renew setting off refuses when asked to consider it.
        assert!(needs_autorenewing_entry::<false>(
            f.v(),
            f.base,
            company(),
            false
        ));
        assert!(!needs_autorenewing_entry::<false>(
            f.v(),
            f.base,
            company(),
            true
        ));
        assert_eq!(failed(), 0);
        let other = ptr::from_ref(&OTHER_COMPANY).cast::<OpenTTDCompany>();
        let _ = needs_autorenewing_entry::<false>(f.v(), f.base, other, false);
        assert_eq!(failed(), 1);
    }

    /// Due for service under `no_servicing_if_no_breakdowns` without breakdowns: the
    /// pending autoreplace of `OLD` by `NEW` decides.
    fn servicing(f: &Fixture) -> (bool, Vec<&'static str>) {
        reads();
        ACTIVE.set(0);
        // SAFETY: This thread's finances.
        FINANCES.with(|r| unsafe { (*r[0]).money = OWNER.get().money });
        let due = needs_servicing(f.v(), f.base);
        (due, reads())
    }

    /// Every company, money and price read of a funded replacement, in the original's order.
    const FULL: [&str; 7] = [
        "reserve",
        "company_id",
        "finances",
        "price",
        "shift",
        "company_id",
        "finances",
    ];
    /// A road vehicle due for service with servicing off without breakdowns, the price of
    /// `NEW` at 25,600 and the owner company 7 with a million and the rule `OLD` to `NEW`.
    fn due_for_replacement() -> Fixture {
        let f = Fixture::new();
        crate::timer_game_common::write_economy_date(EconomyDate(100));
        set(|n| n.service_interval = 1);
        settings(|s| {
            store(s.no_servicing_if_no_breakdowns, true);
            store(s.vehicle_breakdowns, 0);
        });
        crate::company::testing::set_price(
            crate::economy_type::Price::PR_BUILD_VEHICLE_ROAD,
            25_600,
        );
        owner(|o| {
            *o = Owner {
                id: CompanyID(7),
                money: 1_000_000,
                months: 0,
                renew: false,
                rule: Some(false),
            };
        });
        f
    }

    /// `NeedsServicing`'s budget: the renew reserve is tested before any replacement is
    /// looked up and the new engine's cost is evaluated before the second money read.
    #[test]
    fn needs_servicing_tests_the_reserve_then_twice_the_new_engine_price() {
        if crate::services_assert::isolate_test_check(
            "vehicle::maintenance::tests::needs_servicing_tests_the_reserve_then_twice_the_new_engine_price",
            record_failure,
        ) {
            return;
        }
        let f = due_for_replacement();
        let full = FULL;

        // Twice the price, 25,600 * factor >> 8, after the reserve of 100: 3,500 with the
        // callback's factor 17, 10,700 with the engine's own 53 when the callback fails. The
        // callback leaves the company the money of the case.
        for (factor, needed) in [
            (17, 3_500),
            (crate::newgrf_callbacks::CALLBACK_FAILED, 10_700),
        ] {
            for funded in [false, true] {
                owner(|o| {
                    *o = Owner {
                        id: CompanyID(7),
                        money: 1_000_000,
                        rule: Some(false),
                        ..*o
                    };
                });
                let then: fn() = match (needed, funded) {
                    (3_500, false) => || swap(3_499),
                    (3_500, true) => || swap(3_500),
                    (_, false) => || swap(10_699),
                    (_, true) => || swap(10_700),
                };
                PRICE.set(Price { factor, then });
                assert_eq!(servicing(&f), (funded, full.to_vec()), "{needed} {funded}");
            }
        }

        // The reserve of 100 comes first: 99 is short before any replacement is looked up,
        // and 100 goes on to the replacement, which the callback then funds.
        PRICE.set(Price {
            factor: 17,
            then: || swap(3_500),
        });
        for (money, due, seen) in [(99, false, &full[..3]), (100, true, &full[..])] {
            owner(|o| o.money = money);
            assert_eq!(servicing(&f), (due, seen.to_vec()), "money {money}");
        }

        assert_eq!(FAILED.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    /// `GetAvailableMoney` reads the infinite-money setting first and looks the company up
    /// only without it, and an invalid company is never short; a replacement only when old
    /// waits for the vehicle to be old.
    #[test]
    fn needs_servicing_money_reads_infinite_money_first() {
        if crate::services_assert::isolate_test_check(
            "vehicle::maintenance::tests::needs_servicing_money_reads_infinite_money_first",
            record_failure,
        ) {
            return;
        }
        let f = due_for_replacement();
        let full = FULL;
        // Infinite money set by the price callback, which also spends the money: the second
        // test does not read it.
        owner(|o| o.money = 1_000_000);
        PRICE.set(Price {
            factor: 17,
            then: || {
                swap(-1);
                settings(|s| store(s.infinite_money, true));
            },
        });
        let (due, seen) = servicing(&f);
        assert!(due);
        assert_eq!(seen, full[..6]);
        // Infinite money from the start reads no money at all.
        PRICE.set(Price {
            factor: 17,
            then: || {},
        });
        let (due, seen) = servicing(&f);
        assert!(due);
        assert_eq!(
            seen,
            ["reserve", "company_id", "price", "shift", "company_id"]
        );
        // An invalid company has `INT64_MAX`.
        owner(|o| {
            o.id = CompanyID::SPECTATOR;
        });
        settings(|s| store(s.infinite_money, false));
        let (due, seen) = servicing(&f);
        assert!(due);
        assert_eq!(seen, full);

        // A replacement only when old waits for the vehicle to be old: no pending replace.
        owner(|o| {
            *o = Owner {
                id: CompanyID(7),
                money: 1_000_000,
                months: 0,
                rule: Some(true),
                ..*o
            };
        });
        set(|n| {
            n.age = 0;
            n.max_age = 100;
        });
        assert!(!servicing(&f).0);
        set(|n| n.age = 100);
        assert!(servicing(&f).0);
        assert_eq!(FAILED.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    /// `AgeVehicle`'s old-vehicle advice: an age of minus one year, zero and a later
    /// multiple of `DAYS_IN_LEAP_YEAR` past zero each post the original's string, and no
    /// other age, setting or vehicle does.
    #[test]
    fn age_vehicle_warns_of_old_vehicles() {
        let f = Fixture::new();
        let v = f.v();
        // A primary vehicle: a front road vehicle.
        // SAFETY: The fixture's record.
        unsafe {
            (*f.record).subtype = 1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_FRONT.0;
        }
        settings(|s| store(s.old_vehicle_warn, true));
        owner(|o| {
            *o = Owner {
                renew: false,
                rule: None,
                ..*o
            };
        });
        crate::news_func::testing::take();
        // `age - max_age` after the increment.
        let age = |relative: i32| {
            set(|n| {
                n.max_age = 1_000;
                n.age = 1_000 + relative - 1;
            });
            age_vehicle(v, f.base);
            crate::news_func::testing::take()
                .into_iter()
                .filter_map(|news| match news {
                    News::News {
                        headline,
                        advice_type,
                        ref1,
                        ..
                    } => Some((headline, advice_type, ref1)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let advice = |str: crate::strings_type::StringID| {
            vec![(
                encoded(str.0, &[Param::N(u64::from(INDEX))]),
                AdviceType::VehicleOld,
                crate::news_type::NewsReference::Vehicle(crate::vehicle_type::VehicleID(INDEX)),
            )]
        };
        // `DateAtStartOfYear(-1)` is -364: year 0 is a leap year.
        assert_eq!(age(-364), advice(STR_NEWS_VEHICLE_IS_GETTING_OLD));
        assert_eq!(age(0), advice(STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD));
        assert_eq!(age(366), advice(STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD_AND));
        assert_eq!(
            age(2 * 366),
            advice(STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD_AND)
        );
        for relative in [-365, -363, -1, 1, 365, 731] {
            assert_eq!(age(relative), [], "age {relative}");
        }

        let silent = |change: fn(&Fixture), restore: fn(&Fixture)| {
            change(&f);
            assert_eq!(age(0), []);
            restore(&f);
            assert_eq!(age(0), advice(STR_NEWS_VEHICLE_IS_GETTING_VERY_OLD));
        };
        // Warnings off.
        silent(
            |_| settings(|s| store(s.old_vehicle_warn, false)),
            |_| settings(|s| store(s.old_vehicle_warn, true)),
        );
        // Not the local company's.
        silent(|f| f.set_owner(CompanyID(1)), |f| f.set_owner(CompanyID(0)));
        // Stopped or crashed.
        for state in [VehState::Stopped, VehState::Crashed] {
            v.set_vehstatus(VehStates::from_element(state));
            assert_eq!(age(0), []);
            v.set_vehstatus(VehStates(0));
        }
        // A renew is active while the engine is available to anyone.
        silent(
            |_| owner(|o| o.renew = true),
            |_| owner(|o| o.renew = false),
        );
        // A replacement is active.
        silent(
            |_| owner(|o| o.rule = Some(false)),
            |_| owner(|o| o.rule = None),
        );
        // A previous part.
        let previous = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            VehicleType::VEH_ROAD,
        );
        // SAFETY: Both records live through this check.
        unsafe {
            (*f.record).previous = previous;
            assert_eq!(age(0), []);
            (*f.record).previous = ptr::null_mut();
            crate::vehicle_base::openttd_rust_vehicle_record_delete(previous);
        }
    }

    /// `VehicleServiceInDepot` stores the dates, count and engine reliability and makes
    /// both chance stores, the second only on reduced breakdowns.
    #[test]
    fn service_in_depot_quarters_then_clears_the_chance_on_reduced_breakdowns() {
        let f = Fixture::new();
        crate::window_func::testing::take();
        // SAFETY: The fixture's engine.
        unsafe {
            (*f.engine).reliability = 0x9876;
        };
        for (breakdowns, chance) in [(2, 3), (1, 0)] {
            settings(|s| store(s.vehicle_breakdowns, breakdowns));
            set(|n| {
                n.breakdown_chance = 12;
                n.breakdowns_since_last_service = 9;
                n.reliability = 1;
            });
            vehicle_service_in_depot(f.v(), f.base);
            let n = native();
            assert_eq!(
                (
                    n.breakdown_chance,
                    n.breakdowns_since_last_service,
                    n.reliability
                ),
                (chance, 0, 0x9876)
            );
            assert_eq!(crate::window_func::testing::take(), [details()]);
        }
    }
}
