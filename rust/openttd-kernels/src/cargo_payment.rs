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
use crate::company_type::CompanyID;
use crate::industry_type::{Industry as ShellHandle, IndustryID};
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
    pub(crate) town_acceptance_effect: extern "C" fn(u8) -> u8,
    pub(crate) station_town: extern "C" fn(*const Station) -> *const crate::town_type::TownHandle,
    pub(crate) resolver: *const crate::industry::resolver::ResolverServices,
    pub(crate) subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    pub(crate) industry_delivery: *const crate::industry::DeliveryServices,
    pub(crate) last_station: extern "C" fn(*mut OpenTTDVehicle) -> StationID,
    pub(crate) vehicle_owner: extern "C" fn(*mut OpenTTDVehicle) -> CompanyID,
    pub(crate) detach_payment: extern "C" fn(*mut OpenTTDVehicle),
    pub(crate) finance: *const crate::company::FinanceServices,
    pub(crate) vehicles: *const crate::vehicle::VehicleServices,
    pub(crate) local_company: extern "C" fn() -> bool,
    pub(crate) expense: extern "C" fn(*mut OpenTTDVehicle, bool) -> crate::services::ExpenseType,
    pub(crate) profit: extern "C" fn(*mut OpenTTDVehicle) -> i64,
    pub(crate) x: extern "C" fn(*mut OpenTTDVehicle) -> i32,
    pub(crate) y: extern "C" fn(*mut OpenTTDVehicle) -> i32,
    pub(crate) z: extern "C" fn(*mut OpenTTDVehicle) -> i32,
    pub(crate) cost_animation: extern "C" fn(i32, i32, i32, i64),
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
    destinations: Vec<*mut ShellHandle>,
}

fn income(s: &Services, pieces: u32, distance: u32, periods: u16, cargo: u8) -> i64 {
    let mut cs = Spec::default();
    (s.spec)(cargo, &raw mut cs);
    if cs.valid == 0 {
        return 0;
    }
    if cs.callback != 0 {
        let var18 =
            distance.min(65535) | (pieces.min(255) << 16) | (u32::from(periods).min(255) << 24);
        let callback = unsafe {
            crate::cargo_registry::callback(
                &*s.registry,
                cargo,
                crate::newgrf_callbacks::CBID_CARGO_PROFIT_CALC.0,
                0,
                var18,
            )
        };
        if callback != 0xffff {
            let mut result = i32::from(callback & 0x3fff);
            if callback & 0x4000 != 0 {
                result -= 0x4000;
            }
            // result * uint first promotes to uint and wraps; the uint * Money
            // overload passes through int64 then operator*=(int), narrowing!
            let factor = (result as u32).wrapping_mul(pieces) as i32;
            return cs.payment.saturating_mul(i64::from(factor)) / 8192;
        }
    }
    let p1 = i32::from(cs.periods1);
    let p2 = i32::from(cs.periods2);
    let transit = i32::from(periods);
    let over1 = (transit - p1).max(0);
    let over2 = (over1 - p2).max(0);
    let mut over_max = 31 - 255;
    if p2 > -over_max {
        over_max += transit - p1;
    } else {
        over_max += 2 * (transit - p1) - p2;
    }
    let (factor, shift) = if over_max > 0 {
        ((2 * 31 * 16 * 16 / (over_max + 2 * 16)).max(1), 25)
    } else {
        ((255 - over1 - over2).max(31), 21)
    };
    // BigMulS takes two int32 parameters and returns int32 after signed shift.
    let native = distance.wrapping_mul(factor as u32).wrapping_mul(pieces) as i32;
    i64::from(((i64::from(native) * i64::from(cs.payment as i32)) >> shift) as i32)
}

fn deliver(
    d: *mut Delivery,
    s: &Services,
    pieces: u32,
    cargo: u8,
    station: StationID,
    distance: u32,
    periods: u16,
    company: CompanyID,
    source: Source,
    monitors: &crate::cargomonitor::Services,
) -> i64 {
    let industry_source = if source.kind == Source::INDUSTRY {
        IndustryID(source.id)
    } else {
        IndustryID::INVALID
    };
    let mut remaining = pieces;
    let mut accepted_ind = 0_u32;
    let subsidies = unsafe { &*(s.subsidies)() };
    let native = (subsidies.station)(station.0);
    let catchment = crate::station::Ctx::from_shared(subsidies.shared());
    let state = (catchment.c.station_owner)(station);
    let visit = |near: crate::station::IndustryListEntry| {
        if remaining == 0 {
            return false;
        }
        let ind = near.industry;
        if near.id == industry_source {
            return true;
        }
        let owner = (subsidies.industry_owner)(ind.cast());
        let Some(slot) = (unsafe { crate::industry::get_cargo_accepted(owner, cargo) }) else {
            return true;
        };
        if unsafe { crate::industry::resolver::refuses(ind.cast(), owner, cargo, &*s.resolver) } {
            return true;
        }
        let supplier = unsafe { crate::industry::exclusive_supplier(owner) };
        if supplier.0 != crate::company_type::INVALID_OWNER
            && supplier != (s.station_company)(native)
        {
            return true;
        }
        // No owner borrow is alive during a world leaf; include preserves order.
        let destinations = unsafe { &mut (*d).destinations };
        if !destinations.contains(&ind) {
            destinations.push(ind);
        }
        let amount = unsafe {
            crate::industry::accept_delivery(owner, slot, remaining, || {
                (subsidies.shared().economy_date)()
            })
        };
        remaining -= amount;
        accepted_ind += amount;
        crate::cargomonitor::add_delivery(
            monitors,
            cargo,
            company.0,
            amount,
            Source::industry(industry_source.0),
            native,
            state,
            near.id,
        );
        true
    };
    // SAFETY: The live station owner; each entry is copied before the services in `visit`.
    unsafe { crate::station::for_each_industry_near(state, visit) };
    let accepted = if unsafe { crate::station_service::always_accepts(state, cargo) } {
        pieces
    } else {
        accepted_ind
    };
    if accepted > 0 {
        unsafe { crate::station_service::record_delivery(state, cargo) };
    }
    let finance = unsafe { &*s.finance };
    let finances = (finance.company_finances)(company.0);
    unsafe { crate::company::record_delivery(finances, cargo, accepted) };
    let effect = (s.town_acceptance_effect)(cargo);
    let town = (s.station_town)(native);
    let town = (subsidies.shared().town_owner)(town);
    unsafe { crate::town::record_delivery(town, effect, accepted) };
    let mut profit = income(s, accepted, distance, periods, cargo);
    crate::cargomonitor::add_delivery(
        monitors,
        cargo,
        company.0,
        accepted - accepted_ind,
        source,
        native,
        state,
        crate::industry_type::IndustryID::INVALID,
    );
    if subsidies.check(cargo, company.0, source, station.0) {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_front(
    p: *const Payment,
) -> *mut OpenTTDVehicle {
    unsafe { (*p).fields.front }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_afterload(p: *mut Payment, s: *const Services) {
    let s = unsafe { &*s };
    let front = unsafe { (*p).fields.front };
    let station = (s.last_station)(front);
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
            let company = (s.vehicle_owner)(fields.front);
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
            let x_pos = (s.x)(fields.front);
            let y_pos = (s.y)(fields.front);
            let z_pos = (s.z)(fields.front);
            if fields.visual_transfer != 0 {
                (s.feeder_animation)(
                    x_pos,
                    y_pos,
                    z_pos,
                    fields.visual_transfer,
                    fields.visual_profit.saturating_neg(),
                );
            } else {
                (s.cost_animation)(x_pos, y_pos, z_pos, fields.visual_profit.saturating_neg());
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
    let feeder = crate::cargo_storage::share(unsafe { packet.read() }, count).saturating_neg();
    let profit = feeder
        .saturating_add(income(s, count, distance, periods, cargo))
        .saturating_mul(i64::from((s.feeder_payment_share)()))
        / 100;
    unsafe {
        (*p).fields.visual_transfer = (*p).fields.visual_transfer.saturating_add(profit);
    }
    profit
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
    let company = (s.vehicle_owner)(front);
    let profit = deliver(
        d,
        s,
        count,
        cargo,
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
    income(unsafe { &*s }, pieces, distance, periods, cargo)
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
        town_acceptance_effect,
        station_town,
        resolver,
        subsidies,
        industry_delivery,
        last_station,
        vehicle_owner,
        detach_payment,
        finance,
        vehicles,
        local_company,
        expense,
        profit,
        x,
        y,
        z,
        cost_animation,
        feeder_animation,
        subsidy_multiplier,
        feeder_payment_share,
    };
}
