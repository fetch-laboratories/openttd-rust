/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Payment state, delivery acceptance and the production flush. C++ keeps world and pool
//! storage and the `NewGRF` resolvers; save/load and pointer fixing write `Payment::fields`
//! in place between Rust calls. Pointer contracts are in `cargo_payment_ffi.h`.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]
use crate::cargo_type::CargoType;
use crate::company_type::{CompanyID, Owner};
use crate::industry_type::{Industry as ShellHandle, IndustryID};
use crate::newgrf_callbacks::{CALLBACK_FAILED, CBID_CARGO_PROFIT_CALC};
use crate::source_type::Source;
use crate::station_type::Station;
use crate::station_type::StationID;
use crate::vehicle_type::OpenTTDVehicle;

/// Opaque live native `CargoPayment`; never dereferenced by Rust.
#[repr(C)]
pub struct NativePayment {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Default)]
pub struct Spec {
    pub(crate) payment: i64,
    pub(crate) valid: u8,
    pub(crate) callback: u8,
    pub(crate) periods1: u8,
    pub(crate) periods2: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fields {
    pub(crate) front: *mut OpenTTDVehicle,
    pub(crate) route_profit: i64,
    pub(crate) visual_profit: i64,
    pub(crate) visual_transfer: i64,
}
#[repr(C)]
pub struct Services {
    pub(crate) spec: extern "C" fn(u8, *mut Spec),
    pub(crate) registry: *const crate::cargo_registry::Services,
    pub(crate) station_company: extern "C" fn(*const Station) -> CompanyID,
    pub(crate) station_town: extern "C" fn(*const Station) -> *const crate::town_type::TownHandle,
    pub(crate) resolver: *const crate::industry::resolver::ResolverServices,
    pub(crate) subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    pub(crate) industry_delivery: *const crate::industry::DeliveryServices,
    pub(crate) detach_payment: extern "C" fn(*mut OpenTTDVehicle),
    pub(crate) finance: *const crate::company::FinanceServices,
    pub(crate) vehicles: *const crate::vehicle::VehicleServices,
    pub(crate) local_company: extern "C" fn() -> bool,
    pub(crate) expense: extern "C" fn(*mut OpenTTDVehicle, bool) -> crate::services::ExpenseType,
    pub(crate) profit: extern "C" fn(*mut OpenTTDVehicle) -> i64,
    pub(crate) shared: *const crate::services::Services,
    pub(crate) feeder_animation: extern "C" fn(i32, i32, i32, i64, i64),
    pub(crate) subsidy_multiplier: extern "C" fn() -> u8,
    pub(crate) feeder_payment_share: extern "C" fn() -> u8,
}
/// Rust-allocated payment with the `OpenTTDCargoPayment` C layout (ABI layout 60).
#[repr(C)]
pub struct Payment {
    pub(crate) fields: Fields,
    pub(crate) station: StationID,
}
#[derive(Default)]
pub struct Delivery {
    /// The industries to which cargo has currently been brought.
    destinations: Vec<*mut ShellHandle>,
}

/// Original `economy.cpp::GetTransportedGoodsIncome`.
fn get_transported_goods_income(
    s: &Services,
    num_pieces: u32,
    dist: u32,
    transit_periods: u16,
    cargo_type: CargoType,
) -> i64 {
    const MIN_TIME_FACTOR: i32 = 31;
    const MAX_TIME_FACTOR: i32 = 255;
    const TIME_FACTOR_FRAC_BITS: i32 = 4;
    const TIME_FACTOR_FRAC: i32 = 1 << TIME_FACTOR_FRAC_BITS;

    let mut cs = Spec::default();
    (s.spec)(cargo_type.0, &raw mut cs);
    if cs.valid == 0 {
        // The user changed NewGRFs, but a vehicle still carries cargo that is no longer available.
        return 0;
    }
    // Use the callback to calculate cargo profit, if available.
    if cs.callback != 0 {
        let var18 = dist.min(65535)
            | (num_pieces.min(255) << 16)
            | (u32::from(transit_periods).min(255) << 24);
        let callback = unsafe {
            crate::cargo_registry::get_cargo_callback(
                &*s.registry,
                cargo_type.0,
                CBID_CARGO_PROFIT_CALC.0,
                0,
                var18,
            )
        };
        if callback != CALLBACK_FAILED {
            let mut result = i32::from(callback & 0x3fff);
            // Simulate a 15-bit signed value.
            if callback & 0x4000 != 0 {
                result -= 0x4000;
            }
            // Multiply the signed multiplier by the amount of cargo moved and the price
            // factor, then divide by 8192.
            // result * uint first promotes to uint and wraps; the uint * Money
            // overload passes through int64 then operator*=(int), narrowing!
            let factor = (result as u32).wrapping_mul(num_pieces) as i32;
            return cs.payment.saturating_mul(i64::from(factor)) / 8192;
        }
    }
    let periods1 = i32::from(cs.periods1);
    let periods2 = i32::from(cs.periods2);
    let transit = i32::from(transit_periods);
    let periods_over_periods1 = (transit - periods1).max(0);
    let periods_over_periods2 = (periods_over_periods1 - periods2).max(0);
    let mut periods_over_max = MIN_TIME_FACTOR - MAX_TIME_FACTOR;
    if periods2 > -periods_over_max {
        periods_over_max += transit - periods1;
    } else {
        periods_over_max += 2 * (transit - periods1) - periods2;
    }
    // The time factor compares the transit periods with two cargo-dependent values:
    // constant for fast transits, decreasing with slope -1 for medium transports,
    // then with slope -2 for slow transports. After MIN_TIME_FACTOR it decreases
    // asymptotically towards 1 using a scaled 1/(x+1) function.
    let (time_factor, shift) = if periods_over_max > 0 {
        // MIN_TIME_FACTOR / (x/(2 * TIME_FACTOR_FRAC) + 1) + 1,
        // expressed as fixed point with TIME_FACTOR_FRAC_BITS.
        (
            (2 * MIN_TIME_FACTOR * TIME_FACTOR_FRAC * TIME_FACTOR_FRAC
                / (periods_over_max + 2 * TIME_FACTOR_FRAC))
                .max(1),
            21 + TIME_FACTOR_FRAC_BITS,
        )
    } else {
        (
            (MAX_TIME_FACTOR - periods_over_periods1 - periods_over_periods2).max(MIN_TIME_FACTOR),
            21,
        )
    };
    // BigMulS takes two int32 parameters and returns int32 after signed shift.
    let native = dist
        .wrapping_mul(time_factor as u32)
        .wrapping_mul(num_pieces) as i32;
    i64::from(((i64::from(native) * i64::from(cs.payment as i32)) >> shift) as i32)
}

/// Deliver goods to industries/towns and calculate payment. Cargo is added to
/// industry stockpiles; the caller must trigger production after unloading.
fn deliver_goods(
    d: *mut Delivery,
    s: &Services,
    num_pieces: u32,
    cargo_type: CargoType,
    dest: StationID,
    distance: u32,
    periods_in_transit: u16,
    company: CompanyID,
    src: Source,
    monitors: &crate::cargomonitor::Services,
) -> i64 {
    let industry_source = if src.kind == Source::INDUSTRY {
        IndustryID(src.id)
    } else {
        IndustryID::INVALID
    };
    // Give the goods to the industry. Find the nearest industry tile to the station
    // sign inside its catchment area whose industry accepts the cargo. This can fail
    // when houses provide acceptance, nearby industries temporarily refuse cargo
    // before the daily station loop updates acceptance, or CBID_INDUSTRY_REFUSE_CARGO
    // and CBID_INDTILE_CARGO_ACCEPTANCE give inconsistent results (documented behaviour).
    let mut remaining = num_pieces;
    let mut accepted_ind = 0_u32;
    let subsidies = unsafe { &*(s.subsidies)() };
    let st = (subsidies.station)(dest.0);
    let catchment = crate::station::Ctx::from_shared(subsidies.shared());
    let station_owner = (catchment.c.station_owner)(dest);
    let visit = |near: crate::station::IndustryListEntry| {
        if remaining == 0 {
            return false;
        }
        let ind = near.industry;
        if near.id == industry_source {
            return true;
        }
        let owner = (subsidies.industry_owner)(ind.cast());
        // Check if matching cargo has been found.
        let Some(slot) = (unsafe { crate::industry::get_cargo_accepted(owner, cargo_type.0) })
        else {
            return true;
        };
        // Check if the industry temporarily refuses acceptance.
        if unsafe {
            crate::industry::resolver::industry_temporarily_refuses_cargo(
                ind.cast(),
                owner,
                cargo_type,
                &*s.resolver,
            )
        } {
            return true;
        }
        let supplier = unsafe { crate::industry::exclusive_supplier(owner) };
        if supplier != Owner::INVALID && supplier != (s.station_company)(st) {
            return true;
        }
        // Insert the industry into the delivery destinations if not yet contained.
        // No owner borrow is alive during a world leaf; include preserves order.
        let destinations = unsafe { &mut (*d).destinations };
        if !destinations.contains(&ind) {
            destinations.push(ind);
        }
        let amount = unsafe {
            crate::industry::accept_delivery(owner, slot, remaining, || {
                crate::timer_game_common::economy_date().0
            })
        };
        remaining -= amount;
        accepted_ind += amount;
        // Update the cargo monitor.
        crate::cargomonitor::add_delivery(
            monitors,
            cargo_type.0,
            company.0,
            amount,
            Source::industry(industry_source.0),
            st,
            station_owner,
            near.id,
        );
        true
    };
    // SAFETY: The live station owner; each entry is copied before the services in `visit`.
    unsafe { crate::station::for_each_industry_near(station_owner, visit) };
    // If this cargo type is always accepted, accept all.
    let accepted_total =
        if unsafe { crate::station_service::always_accepts(station_owner, cargo_type.0) } {
            num_pieces
        } else {
            accepted_ind
        };
    // Update station statistics.
    if accepted_total > 0 {
        unsafe { crate::station_service::record_delivery(station_owner, cargo_type.0) };
    }
    // Update company statistics.
    let finance = unsafe { &*s.finance };
    let finances = (finance.company_finances)(company.0);
    unsafe { crate::company::record_delivery(finances, cargo_type.0, accepted_total) };
    // Increase the town counter for town effects.
    let effect = crate::cargo_registry::town_acceptance(cargo_type.0);
    let town = (s.station_town)(st);
    let town = (subsidies.shared().town_owner)(town);
    unsafe { crate::town::record_delivery(town, effect, accepted_total) };
    // Determine profit.
    let mut profit =
        get_transported_goods_income(s, accepted_total, distance, periods_in_transit, cargo_type);
    // Update the cargo monitor.
    crate::cargomonitor::add_delivery(
        monitors,
        cargo_type.0,
        company.0,
        accepted_total - accepted_ind,
        src,
        st,
        station_owner,
        IndustryID::INVALID,
    );
    // Modify profit if a subsidy is in effect.
    if subsidies.check_subsidised(cargo_type, company, src, dest) {
        profit = match (s.subsidy_multiplier)() {
            0 => profit.saturating_add(profit >> 1),
            1 => profit.saturating_mul(2),
            2 => profit.saturating_mul(3),
            _ => profit.saturating_mul(4),
        };
    }
    profit
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_payment_new(
    front: *mut OpenTTDVehicle,
    station: StationID,
) -> *mut Payment {
    Box::into_raw(Box::new(Payment {
        fields: Fields {
            front,
            ..Fields::default()
        },
        station,
    }))
}
/// The payment's live front record.
fn record(front: *mut OpenTTDVehicle) -> crate::vehicle::VehicleRef {
    crate::vehicle::VehicleRef(std::ptr::NonNull::new(front).unwrap())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_front(
    p: *const Payment,
) -> *mut OpenTTDVehicle {
    unsafe { (*p).fields.front }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_afterload(p: *mut Payment) {
    let front = unsafe { (*p).fields.front };
    let station = StationID(record(front).last_station());
    unsafe {
        (*p).station = station;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_destroy(
    p: *mut Payment,
    s: *const Services,
    cleaning: u8,
) {
    let fields = unsafe { (*p).fields };
    if cleaning == 0 {
        let s = unsafe { &*s };
        (s.detach_payment)(fields.front);
        if fields.visual_profit != 0 || fields.visual_transfer != 0 {
            let finance = unsafe { &*s.finance };
            let vehicles = unsafe { &*s.vehicles };
            let old_company = CompanyID((finance.current_company)());
            let company = CompanyID(record(fields.front).owner());
            (finance.set_current_company)(company.0);
            let expense = (s.expense)(fields.front, true);
            let finances = (finance.company_finances)(company.0);
            // SAFETY: The front's owner is the live company selected above.
            if !finances.is_null() {
                unsafe {
                    crate::company::subtract_money(
                        finance,
                        finances,
                        company.0,
                        fields.route_profit.saturating_neg(),
                        expense.0,
                    );
                }
            }
            // SAFETY: Settlement retains its live native front until all callbacks return.
            let front = crate::vehicle::VehicleRef(unsafe {
                std::ptr::NonNull::new_unchecked(fields.front)
            });
            let profit = (s.profit)(fields.front).saturating_add(
                fields
                    .visual_profit
                    .saturating_add(fields.visual_transfer)
                    .wrapping_shl(8),
            );
            (vehicles.write_profit)(front, profit);
            if fields.route_profit != 0
                && (s.local_company)()
                && !(vehicles.play_vehicle_sound)(front, crate::newgrf_sound::VSE_LOAD_UNLOAD)
            {
                (vehicles.play_sound)(front, crate::sound_type::SND_14_CASHTILL);
            }
            let front = record(fields.front);
            let (x_pos, y_pos, z_pos) = (front.x(), front.y(), front.z());
            if fields.visual_transfer != 0 {
                (s.feeder_animation)(
                    x_pos,
                    y_pos,
                    z_pos,
                    fields.visual_transfer,
                    fields.visual_profit.saturating_neg(),
                );
            } else {
                // SAFETY: the caller supplies the immutable shared table for settlement.
                unsafe {
                    ((*s.shared).cost_animation)(
                        x_pos,
                        y_pos,
                        z_pos,
                        fields.visual_profit.saturating_neg(),
                    );
                }
            }
            (finance.set_current_company)(old_company.0);
        }
    }
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_transfer(
    p: *mut Payment,
    s: *const Services,
    cargo: u8,
    packet: *const crate::cargo_storage::Packet,
    count: u32,
    distance: u32,
    periods: u16,
) -> i64 {
    let s = unsafe { &*s };
    let cargo = CargoType(cargo);
    // Pay the transfer vehicle the journey's payment from the source to this point,
    // less the sum of previous transfer payments.
    let feeder = crate::cargo_storage::share(unsafe { packet.read() }, count).saturating_neg();
    let profit = feeder
        .saturating_add(get_transported_goods_income(
            s, count, distance, periods, cargo,
        ))
        .saturating_mul(i64::from((s.feeder_payment_share)()))
        / 100;
    unsafe {
        // Accumulate transfer profits for the whole vehicle.
        (*p).fields.visual_transfer = (*p).fields.visual_transfer.saturating_add(profit);
    }
    profit // Account for the virtual profit already made for this cargo packet.
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_final(
    p: *mut Payment,
    d: *mut Delivery,
    s: *const Services,
    cargo: u8,
    packet: *const crate::cargo_storage::Packet,
    count: u32,
    distance: u32,
    periods: u16,
    source: Source,
    monitors: *const crate::cargomonitor::Services,
) {
    let s = unsafe { &*s };
    let front = unsafe { (*p).fields.front };
    let station = unsafe { (*p).station };
    let company = CompanyID(record(front).owner());
    // Handle end-of-route payment.
    let profit = deliver_goods(
        d,
        s,
        count,
        CargoType(cargo),
        station,
        distance,
        periods,
        company,
        source,
        unsafe { &*monitors },
    );
    unsafe {
        (*p).fields.route_profit = (*p).fields.route_profit.saturating_add(profit);
    }
    // The vehicle's profit is the route profit minus feeder shares.
    let visual =
        profit.saturating_sub(crate::cargo_storage::share(unsafe { packet.read() }, count));
    unsafe {
        (*p).fields.visual_profit = (*p).fields.visual_profit.saturating_add(visual);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_income(
    s: *const Services,
    pieces: u32,
    distance: u32,
    periods: u16,
    cargo: u8,
) -> i64 {
    get_transported_goods_income(unsafe { &*s }, pieces, distance, periods, CargoType(cargo))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_delivery_new() -> *mut Delivery {
    Box::into_raw(Box::new(Delivery::default()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_delivery_destroy(d: *mut Delivery) {
    unsafe {
        drop(Box::from_raw(d));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_delivery_flush(d: *mut Delivery, s: *const Services) {
    let s = unsafe { &*s };
    // Call the production machinery of the industries that received cargo.
    let mut index = 0;
    while index < unsafe { (*d).destinations.len() } {
        let ind = unsafe { (&(*d).destinations)[index] };
        let subsidies = unsafe { &*(s.subsidies)() };
        let owner = (subsidies.industry_owner)(ind.cast());
        unsafe {
            crate::industry::trigger_delivery(
                ind,
                owner,
                IndustryID((subsidies.industry_id)(ind.cast())),
                &*s.resolver,
                &*subsidies.industry,
                &*s.industry_delivery,
            );
        };
        index += 1;
    }
    unsafe {
        (*d).destinations.clear();
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoSpec" ["probe"] => Spec {
        payment,
        valid,
        callback,
        periods1,
        periods2,
    };
    "OpenTTDCargoPaymentFields" ["probe"] => Fields {
        front,
        route_profit,
        visual_profit,
        visual_transfer,
    };
    "OpenTTDCargoPayment" ["probe"] => Payment {
        fields,
        station,
    };
    "OpenTTDCargoServices" ["probe"] => Services {
        spec,
        registry,
        station_company,
        station_town,
        resolver,
        subsidies,
        industry_delivery,
        detach_payment,
        finance,
        vehicles,
        local_company,
        expense,
        profit,
        shared,
        feeder_animation,
        subsidy_multiplier,
        feeder_payment_share,
    };
}
