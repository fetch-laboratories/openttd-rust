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
        let Some(slot) = (unsafe { crate::industry::get_cargo_accepted(owner, cargo_type) }) else {
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
                crate::timer_game_common::economy_date()
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
        if unsafe { crate::station_service::always_accepts(station_owner, cargo_type) } {
            num_pieces
        } else {
            accepted_ind
        };
    // Update station statistics.
    if accepted_total > 0 {
        unsafe { crate::station_service::record_delivery(station_owner, cargo_type) };
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
            crate::industry::trigger_industry_production(
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

/// `GetTransportedGoodsIncome` and `CargoPayment::PayTransfer` over the retired
/// `tools/migration/cargo-income-gap.cpp` domain (#736 F), against its original-only capture
/// in `cargo_payment/income-oracle.txt`.
#[cfg(test)]
mod income_tests {
    use super::*;
    use crate::cargo_registry::Invocation;
    use crate::oracle;
    use crate::test_tables::CARGO;
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::ptr;

    const ORACLE: &str = include_str!("cargo_payment/income-oracle.txt");
    const PAYMENTS: [i64; 7] = [
        i64::MIN,
        i32::MIN as i64,
        -1,
        0,
        6000,
        i32::MAX as i64,
        i64::MAX,
    ];
    const PERIODS1: [u8; 3] = [0, 20, 255];
    const PERIODS2: [u8; 5] = [0, 20, 224, 225, 255];
    const PIECES: [u32; 7] = [0, 1, 255, 256, 65535, 0x8000_0000, u32::MAX];
    const DISTANCES: [u32; 6] = [0, 1, 65535, 65536, 0x8000_0000, u32::MAX];
    const PERIODS: [u16; 7] = [0, 20, 244, 245, 255, 256, 65535];
    const CALLBACKS: [u16; 6] = [0, 0x3FFF, 0x4000, 0x7FFF, 0x8000, 0xFFFF];
    const EXTREMES: [i64; 4] = [i64::MIN, -1, 0, i64::MAX];
    const SHARES: [u8; 4] = [0, 75, 100, 255];

    thread_local! {
        /// The fixture's spec: payment, valid, transit periods; its profit callback is enabled.
        static SPEC: Cell<(i64, u8, u8, u8)> = const { Cell::new((0, 0, 0, 0)) };
        static CALLBACK: Cell<u16> = const { Cell::new(0) };
        static CALLS: Cell<u32> = const { Cell::new(0) };
        /// The last callback's `var18`; like the fixture's global, never reset.
        static ARGUMENT: Cell<u32> = const { Cell::new(0) };
        static SHARE: Cell<u8> = const { Cell::new(0) };
    }
    extern "C" fn spec(_: u8, out: *mut Spec) {
        let (payment, valid, periods1, periods2) = SPEC.get();
        // SAFETY: The income routine's own spec slot.
        unsafe {
            out.write(Spec {
                payment,
                valid,
                callback: 1,
                periods1,
                periods2,
            });
        }
    }
    extern "C" fn registry_spec(_: u8) -> *const c_void {
        ptr::null()
    }
    extern "C" fn invocation(_: *const c_void, _: *mut Invocation) {}
    extern "C" fn callback_frame(invocation: *const Invocation, _: *mut i32, _: usize) -> u16 {
        CALLS.set(CALLS.get() + 1);
        // SAFETY: The callback's live invocation.
        ARGUMENT.set(unsafe { (*invocation).param2 });
        CALLBACK.get()
    }
    extern "C" fn feeder_payment_share() -> u8 {
        SHARE.get()
    }

    /// Split a case index of nested loops into its digits, innermost loop first.
    fn digits<const N: usize>(mut index: usize, radices: [usize; N]) -> [usize; N] {
        radices.map(|radix| {
            let digit = index % radix;
            index /= radix;
            digit
        })
    }

    /// Income for each case of the fixture's nested domain: (inputs, "income calls argument").
    fn income_cases(s: &Services) -> Vec<(String, String)> {
        let radices = [
            CALLBACKS.len(),
            PERIODS.len(),
            DISTANCES.len(),
            PIECES.len(),
            PERIODS2.len(),
            PERIODS1.len(),
            PAYMENTS.len(),
        ];
        (0..radices.iter().product())
            .map(|index| {
                let [callback, periods, distance, pieces, p2, p1, payment] = digits(index, radices);
                let (callback, periods, distance, pieces) = (
                    CALLBACKS[callback],
                    PERIODS[periods],
                    DISTANCES[distance],
                    PIECES[pieces],
                );
                let (p1, p2, payment) = (PERIODS1[p1], PERIODS2[p2], PAYMENTS[payment]);
                CALLBACK.set(callback);
                CALLS.set(0);
                SPEC.set((payment, 1, p1, p2));
                // SAFETY: The test's live services.
                let income = unsafe { openttd_rust_cargo_income(s, pieces, distance, periods, 0) };
                (
                    format!(
                        "payment {payment} periods {p1}/{p2} pieces {pieces} distance {distance} transit {periods} callback {callback:#x}"
                    ),
                    format!("{income} {} {}", CALLS.get(), ARGUMENT.get()),
                )
            })
            .collect()
    }

    /// Original-order case indices of the income domain enumerated in `order`, outermost
    /// first, over dimensions 0 payment, 1 `transit_periods[0]`, 2 `transit_periods[1]`,
    /// 3 pieces, 4 distance, 5 periods and 6 callback.
    fn in_order(order: [usize; 7]) -> Vec<usize> {
        let lengths = [
            PAYMENTS.len(),
            PERIODS1.len(),
            PERIODS2.len(),
            PIECES.len(),
            DISTANCES.len(),
            PERIODS.len(),
            CALLBACKS.len(),
        ];
        let mut strides = [1_usize; 7];
        for d in (0..6).rev() {
            strides[d] = strides[d + 1] * lengths[d + 1];
        }
        (0..lengths.iter().product())
            .map(|mut rest: usize| {
                let mut index = 0;
                for &d in order.iter().rev() {
                    index += (rest % lengths[d]) * strides[d];
                    rest /= lengths[d];
                }
                index
            })
            .collect()
    }

    /// One column of `cases`: the space-separated output fields `fields`, in `order`.
    fn column(
        cases: &[(String, String)],
        order: Option<[usize; 7]>,
        fields: std::ops::Range<usize>,
    ) -> Vec<(String, String)> {
        let indices = order.map_or_else(|| (0..cases.len()).collect(), in_order);
        indices
            .into_iter()
            .map(|index| {
                let (inputs, output) = &cases[index];
                let output: Vec<&str> = output.split(' ').collect();
                (inputs.clone(), output[fields.clone()].join(" "))
            })
            .collect()
    }

    /// `PayTransfer` of 65535 of a 65535-piece packet, distance 250 and 45 periods, with the
    /// income loop's last spec: (inputs, "profit `visual_transfer` calls argument").
    fn transfer_cases(s: &Services) -> Vec<(String, String)> {
        let radices = [SHARES.len(), EXTREMES.len(), EXTREMES.len()];
        (0..radices.iter().product())
            .map(|index| {
                let [share, feeder, initial] = digits(index, radices);
                transfer_case(s, EXTREMES[initial], EXTREMES[feeder], SHARES[share])
            })
            .collect()
    }
    fn transfer_case(s: &Services, initial: i64, feeder: i64, share: u8) -> (String, String) {
        SHARE.set(share);
        CALLS.set(0);
        let payment = openttd_rust_cargo_payment_new(ptr::null_mut(), StationID(0xFFFF));
        let packet = crate::cargo_storage::Packet {
            feeder_share: feeder,
            count: 65535,
            ..crate::cargo_storage::Packet::default()
        };
        // SAFETY: Allocated above and released below; the packet is a local.
        let (profit, fields) = unsafe {
            (*payment).fields = Fields {
                front: ptr::null_mut(),
                route_profit: i64::MIN,
                visual_profit: i64::MAX,
                visual_transfer: initial,
            };
            let profit = openttd_rust_cargo_payment_transfer(
                payment,
                s,
                0,
                &raw const packet,
                65535,
                250,
                45,
            );
            let fields = (*payment).fields;
            openttd_rust_cargo_payment_destroy(payment, ptr::null(), 1);
            (profit, fields)
        };
        let inputs = format!("visual_transfer {initial} feeder_share {feeder} share {share}");
        // The fixture's own check: a transfer leaves the route and visual profits alone.
        assert_eq!(
            (fields.route_profit, fields.visual_profit),
            (i64::MIN, i64::MAX),
            "{inputs}: route and visual profit"
        );
        let output = format!(
            "{profit} {} {} {}",
            fields.visual_transfer,
            CALLS.get(),
            ARGUMENT.get()
        );
        (inputs, output)
    }

    #[test]
    fn income_and_transfer_match_original() {
        let mut registry = crate::cargo_registry::TEST_TABLE;
        registry.spec = registry_spec;
        registry.invocation = invocation;
        registry.callback_frame = callback_frame;
        let mut s = CARGO;
        s.spec = spec;
        s.registry = &raw const registry;
        s.feeder_payment_share = feeder_payment_share;
        let sections = oracle::parse(ORACLE);

        // The cases run in the fixture's order: its callback argument and spec carry over.
        // Each column is then compared in its own stored order.
        let income = income_cases(&s);
        let transfer = transfer_cases(&s);
        let income_order = Some([3, 6, 0, 1, 2, 4, 5]);
        let var18_order = Some([5, 4, 3, 6, 1, 2, 0]);
        for (name, cases, order, fields) in [
            ("income", &income, income_order, 0..2),
            ("var18", &income, var18_order, 2..3),
            ("transfer", &transfer, None, 0..2),
            ("transfer-callback", &transfer, None, 2..4),
        ] {
            assert_eq!(
                oracle::check(&sections, name, column(cases, order, fields)),
                cases.len(),
                "{name}"
            );
        }
        assert_eq!(income.len(), 185_220);
        assert_eq!(transfer.len(), 64);
        let (payment, _, periods1, periods2) = SPEC.get();
        SPEC.set((payment, 0, periods1, periods2));
        CALLS.set(0);
        // SAFETY: The test's live services.
        let income =
            unsafe { openttd_rust_cargo_income(&raw const s, u32::MAX, u32::MAX, 65535, 0) };
        let invalid = [(
            "invalid spec, pieces and distance UINT32_MAX, transit 65535".to_owned(),
            format!("{income} {}", CALLS.get()),
        )];
        assert_eq!(oracle::check(&sections, "invalid", invalid), 1);
    }
}

/// `CargoPayment::~CargoPayment` settlement from the retired `src/tests/cargo_payment.cpp`
/// (#736 F): the fixture's operation order, `Money` boundaries and cleanup, with its own
/// expected values.
#[cfg(test)]
mod settlement_tests {
    use super::*;
    use crate::economy_type::ExpensesType;
    use crate::test_tables::{CARGO, FINANCE, SHARED, VEHICLE};
    use crate::vehicle::VehicleRef;
    use std::cell::{Cell, RefCell};
    use std::ptr;

    /// The fixture's settlement log: (operation, first value, second value). Operations:
    /// 0 detach, 1 current company, 2 select the owner, 3 invalidate its windows, 4 write the
    /// profit, 5 local company, 6 vehicle sound, 7 cash-till sound, 8 feeder animation,
    /// 9 cost animation, 10 restore the company.
    type Settlement = Vec<(u8, i64, i64)>;
    thread_local! {
        static SETTLEMENT: RefCell<Settlement> = const { RefCell::new(Vec::new()) };
        static FINANCES: Cell<*mut crate::company::Finances> = const { Cell::new(ptr::null_mut()) };
        static LOCAL: Cell<bool> = const { Cell::new(false) };
        static CUSTOM_SOUND: Cell<bool> = const { Cell::new(false) };
        static VALID_COMPANY: Cell<bool> = const { Cell::new(false) };
        static CURRENT: Cell<u8> = const { Cell::new(0) };
    }
    fn log(operation: u8, first: i64, second: i64) {
        SETTLEMENT.with(|s| s.borrow_mut().push((operation, first, second)));
    }
    extern "C" fn current_company() -> u8 {
        log(1, 0, 0);
        CURRENT.get()
    }
    extern "C" fn set_current_company(id: u8) {
        log(if id == 3 { 2 } else { 10 }, i64::from(id), 0);
        CURRENT.set(id);
    }
    extern "C" fn company_finances(id: u8) -> *mut crate::company::Finances {
        assert_eq!(id, 3);
        if VALID_COMPANY.get() {
            FINANCES.get()
        } else {
            ptr::null_mut()
        }
    }
    extern "C" fn invalidate_company_windows(id: u8) {
        log(3, i64::from(id), 0);
    }
    extern "C" fn write_profit(_: VehicleRef, value: i64) {
        log(4, value, 0);
    }
    extern "C" fn play_vehicle_sound(
        _: VehicleRef,
        event: crate::newgrf_sound::VehicleSoundEvent,
    ) -> bool {
        assert_eq!(event.0, crate::newgrf_sound::VSE_LOAD_UNLOAD.0);
        log(6, 0, 0);
        CUSTOM_SOUND.get()
    }
    extern "C" fn play_sound(_: VehicleRef, sound: u16) {
        assert_eq!(sound, crate::sound_type::SND_14_CASHTILL);
        log(7, 0, 0);
    }
    extern "C" fn detach_payment(_: *mut OpenTTDVehicle) {
        log(0, 0, 0);
    }
    extern "C" fn local_company() -> bool {
        log(5, 0, 0);
        LOCAL.get()
    }
    extern "C" fn expense(_: *mut OpenTTDVehicle, income: bool) -> ExpensesType {
        assert!(income);
        ExpensesType::EXPENSES_TRAIN_REVENUE
    }
    extern "C" fn profit(_: *mut OpenTTDVehicle) -> i64 {
        1000
    }
    extern "C" fn cost_animation(x: i32, y: i32, z: i32, income: i64) {
        assert_eq!((x, y, z), (11, 22, 33));
        log(9, 0, income);
    }
    extern "C" fn feeder_animation(x: i32, y: i32, z: i32, transfer: i64, income: i64) {
        assert_eq!((x, y, z), (11, 22, 33));
        log(8, transfer, income);
    }

    /// One fixture case: cleanup or settlement, a maximal or no transfer, a local company,
    /// a custom vehicle sound and a valid company.
    /// The fixture's nested loop index: bit 4 cleaning, then transfer, local, custom and bit 0
    /// valid, so counting up follows its loop order.
    #[derive(Clone, Copy)]
    struct Case(u8);
    impl Case {
        fn bit(self, n: u8) -> bool {
            self.0 & (16 >> n) != 0
        }
        fn cleaning(self) -> bool {
            self.bit(0)
        }
        fn transfer(self) -> bool {
            self.bit(1)
        }
        fn local(self) -> bool {
            self.bit(2)
        }
        fn custom(self) -> bool {
            self.bit(3)
        }
        fn valid(self) -> bool {
            self.bit(4)
        }
    }

    /// The fixture's expected operation order for one settled payment.
    fn expected_order(case: Case) -> Vec<u8> {
        let mut order = vec![0, 1, 2];
        if case.valid() {
            order.push(3);
        }
        order.extend([4, 5]);
        if case.local() {
            order.push(6);
            if !case.custom() {
                order.push(7);
            }
        }
        order.push(if case.transfer() { 8 } else { 9 });
        order.push(10);
        order
    }

    /// One fixture case: settle (or clean up) a payment whose route and visual profits are
    /// `INT64_MIN`, with a maximal or no transfer.
    fn settle(s: &Services, case: Case) {
        let (cleaning, transfer, local, custom, valid) = (
            case.cleaning(),
            case.transfer(),
            case.local(),
            case.custom(),
            case.valid(),
        );
        let inputs = format!(
            "cleaning {cleaning} transfer {transfer} local {local} custom {custom} valid {valid}"
        );
        SETTLEMENT.with(|s| s.borrow_mut().clear());
        // SAFETY: The case's zeroed finance record, owned by this thread.
        unsafe { FINANCES.get().write(std::mem::zeroed()) };
        CURRENT.set(17);
        LOCAL.set(local);
        CUSTOM_SOUND.set(custom);
        VALID_COMPANY.set(valid);
        let front = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::TRAIN,
        );
        // SAFETY: The record allocated above, released at the end of the case.
        unsafe {
            (*front).owner = crate::company_type::CompanyID(3);
            ((*front).x_pos, (*front).y_pos, (*front).z_pos) = (11, 22, 33);
        }
        let payment = openttd_rust_cargo_payment_new(front, StationID(42));
        // SAFETY: The payment allocated above; destroy releases it.
        unsafe {
            (*payment).fields = Fields {
                front,
                route_profit: i64::MIN,
                visual_profit: i64::MIN,
                visual_transfer: if transfer { i64::MAX } else { 0 },
            };
            assert_eq!(
                openttd_rust_cargo_payment_front(payment),
                front,
                "{inputs}: front"
            );
            openttd_rust_cargo_payment_destroy(payment, s, u8::from(cleaning));
        }
        let settlement = SETTLEMENT.with(|s| s.borrow().clone());
        // SAFETY: The settlement has ended; nothing else holds the record or the finances.
        let (money, revenue) = unsafe {
            crate::vehicle_base::openttd_rust_vehicle_record_delete(front);
            let finances = FINANCES.get();
            (
                (*finances).money,
                (*finances).expenses[0][usize::from(ExpensesType::EXPENSES_TRAIN_REVENUE.0)],
            )
        };
        if cleaning {
            assert!(settlement.is_empty(), "{inputs}: cleanup settles nothing");
            return;
        }
        let order: Vec<u8> = settlement
            .iter()
            .map(|&(operation, ..)| operation)
            .collect();
        assert_eq!(order, expected_order(case), "{inputs}: order");
        let expected = if valid { (-i64::MAX, i64::MAX) } else { (0, 0) };
        assert_eq!((money, revenue), expected, "{inputs}: money, revenue");
        assert_eq!(
            settlement[if valid { 4 } else { 3 }].1,
            if transfer { 744 } else { 1000 },
            "{inputs}: written profit"
        );
        assert_eq!(
            settlement[settlement.len() - 2].2,
            i64::MAX,
            "{inputs}: animated income"
        );
        assert_eq!(CURRENT.get(), 17, "{inputs}: restored company");
    }

    #[test]
    fn settlement_order_money_boundaries_and_cleanup() {
        let mut finance = FINANCE;
        finance.current_company = current_company;
        finance.set_current_company = set_current_company;
        finance.company_finances = company_finances;
        finance.invalidate_company_windows = invalidate_company_windows;
        let mut vehicles = VEHICLE;
        vehicles.write_profit = write_profit;
        vehicles.play_vehicle_sound = play_vehicle_sound;
        vehicles.play_sound = play_sound;
        let mut shared = SHARED;
        shared.cost_animation = cost_animation;
        let mut s = CARGO;
        s.finance = &raw const finance;
        s.vehicles = &raw const vehicles;
        s.detach_payment = detach_payment;
        s.local_company = local_company;
        s.expense = expense;
        s.profit = profit;
        s.shared = &raw const shared;
        s.feeder_animation = feeder_animation;
        // SAFETY: Plain integer record; every case rewrites it before use.
        let mut finances: Box<crate::company::Finances> = Box::new(unsafe { std::mem::zeroed() });
        FINANCES.set(&raw mut *finances);

        // The fixture's nested loops, outermost first.
        for case in 0..32 {
            settle(&s, Case(case));
        }
        // A payment with neither visual profit nor transfer only detaches.
        SETTLEMENT.with(|s| s.borrow_mut().clear());
        let payment = openttd_rust_cargo_payment_new(ptr::null_mut(), StationID(0xFFFF));
        // SAFETY: The payment allocated above; destroy releases it.
        unsafe {
            (*payment).fields = Fields {
                front: ptr::null_mut(),
                route_profit: i64::MAX,
                visual_profit: 0,
                visual_transfer: 0,
            };
            openttd_rust_cargo_payment_destroy(payment, &raw const s, 0);
        }
        assert_eq!(SETTLEMENT.with(|s| s.borrow().clone()), [(0, 0, 0)]);
        FINANCES.set(ptr::null_mut());
    }
}
