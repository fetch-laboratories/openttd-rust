/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Payment state, delivery acceptance and the complete production flush.
//! C++ retains canonical world/pool storage and `NewGRF` resolvers. Leaves
//! cannot reenter these owners or invalidate handles. No world references cross
//! calls; C++ save/load and pointer fixing write `Payment::fields` in place
//! through reference aliases between Rust calls.
//! Exported pointer operations require the live allocations, exclusive access
//! and call-scoped buffers documented in `cargo_payment_ffi.h`; no function
//! retains a buffer or reference. Only cleaning destruction accepts a null table.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]
use crate::station::NearbyIndustryCursor;
use std::ffi::c_void;

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
    pub(crate) front: *mut c_void,
    pub(crate) route_profit: i64,
    pub(crate) visual_profit: i64,
    pub(crate) visual_transfer: i64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Services {
    pub(crate) spec: extern "C" fn(u8, *mut Spec),
    pub(crate) callback: extern "C" fn(u8, u32) -> u16,
    pub(crate) station_read: extern "C" fn(u16, u8, u8) -> u32,
    pub(crate) industry_read: extern "C" fn(*mut c_void, u8, u32) -> u32,
    pub(crate) industry_write: extern "C" fn(*mut c_void, u8, u32, u32),
    pub(crate) refuses: extern "C" fn(*mut c_void, u8) -> u8,
    pub(crate) accept: extern "C" fn(*mut c_void, u32, u32),
    pub(crate) statistics: extern "C" fn(u16, u8, u8, u32, u8),
    pub(crate) subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    pub(crate) industry_effect: extern "C" fn(*mut c_void, u8),
    pub(crate) vehicle_read: extern "C" fn(*mut c_void, u8) -> u32,
    pub(crate) settle: extern "C" fn(*mut c_void, u8, i64, i64) -> u32,
    pub(crate) feeder: extern "C" fn(*const c_void, u32) -> i64,
    pub(crate) setting: extern "C" fn(u8) -> u32,
}
/// Rust-allocated payment with the `OpenTTDCargoPayment` C layout (ABI layout 60).
#[repr(C)]
pub struct Payment {
    pub(crate) fields: Fields,
    pub(crate) station: u16,
}
#[derive(Default)]
pub struct Delivery {
    destinations: Vec<*mut c_void>,
}

fn income(s: Services, pieces: u32, distance: u32, periods: u16, cargo: u8) -> i64 {
    let mut cs = Spec::default();
    (s.spec)(cargo, &raw mut cs);
    if cs.valid == 0 {
        return 0;
    }
    if cs.callback != 0 {
        let var18 =
            distance.min(65535) | (pieces.min(255) << 16) | (u32::from(periods).min(255) << 24);
        let callback = (s.callback)(cargo, var18);
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
    s: Services,
    pieces: u32,
    cargo: u8,
    station: u16,
    distance: u32,
    periods: u16,
    company: u8,
    source: u32,
    cursor: *mut NearbyIndustryCursor,
    monitors: &crate::cargomonitor::Services,
    monitor_cursor: *mut NearbyIndustryCursor,
) -> i64 {
    let industry_source = if source >> 16 == 0 {
        source as u16
    } else {
        0xffff
    };
    let mut remaining = pieces;
    let mut accepted_ind = 0_u32;
    let subsidies = unsafe { &*(s.subsidies)() };
    (subsidies.begin_nearby)(cursor, (subsidies.station)(station));
    loop {
        let ind = (subsidies.next_nearby)(cursor).cast::<c_void>();
        if ind.is_null() || remaining == 0 {
            break;
        }
        if (subsidies.industry_id)(ind) == industry_source {
            continue;
        }
        let slot = (s.industry_read)(ind, 1, u32::from(cargo));
        if slot == u32::MAX {
            continue;
        }
        if (s.refuses)(ind, cargo) != 0 {
            continue;
        }
        let supplier = (s.industry_read)(ind, 2, 0);
        if supplier != 0xff && supplier != (s.station_read)(station, 0, 0) {
            continue;
        }
        // No owner borrow is alive during a world leaf; include preserves order.
        let destinations = unsafe { &mut (*d).destinations };
        if !destinations.contains(&ind) {
            destinations.push(ind);
        }
        let amount = remaining.min(65535 - (s.industry_read)(ind, 3, slot));
        (s.accept)(ind, slot, amount);
        remaining -= amount;
        accepted_ind += amount;
        crate::cargomonitor::add_delivery(
            monitors,
            cargo,
            company,
            amount,
            crate::source_type::Source::industry(industry_source),
            (subsidies.station)(station),
            (subsidies.industry_id)(ind),
            monitor_cursor,
        );
    }
    let accepted = if (s.station_read)(station, 1, cargo) != 0 {
        pieces
    } else {
        accepted_ind
    };
    if accepted > 0 {
        (s.statistics)(station, cargo, company, accepted, 0);
    }
    (s.statistics)(station, cargo, company, accepted, 1);
    (s.statistics)(station, cargo, company, accepted, 2);
    let mut profit = income(s, accepted, distance, periods, cargo);
    crate::cargomonitor::add_delivery(
        monitors,
        cargo,
        company,
        accepted - accepted_ind,
        crate::source_type::Source {
            id: source as u16,
            kind: (source >> 16) as u8,
        },
        (subsidies.station)(station),
        0xffff,
        monitor_cursor,
    );
    if subsidies.check(
        cargo,
        company,
        crate::subsidy::Source {
            id: source as u16,
            kind: (source >> 16) as u8,
        },
        station,
        cursor,
    ) {
        profit = match (s.setting)(0) {
            0 => profit.saturating_add(profit >> 1),
            1 => profit.saturating_mul(2),
            2 => profit.saturating_mul(3),
            _ => profit.saturating_mul(4),
        };
    }
    profit
}

fn trigger(ind: *mut c_void, s: Services) {
    let mask = (s.industry_read)(ind, 4, 0);
    (s.industry_effect)(ind, 0); // was_cargo_delivered
    if mask & 3 != 0 {
        (s.industry_effect)(ind, if mask & 1 != 0 { 1 } else { 2 });
    } else {
        for input in 0..(s.industry_read)(ind, 5, 0) {
            let waiting = (s.industry_read)(ind, 3, input);
            if waiting == 0 || (s.industry_read)(ind, 7, input) == 255 {
                continue;
            }
            for output in 0..(s.industry_read)(ind, 6, 0) {
                if (s.industry_read)(ind, 8, output) == 255 {
                    continue;
                }
                let old = (s.industry_read)(ind, 9, output);
                let multiplier = (s.industry_read)(ind, 10, (input << 16) | output);
                // uint16 operands promote to int; valid multipliers' product is
                // within int32 in the defined source domain. Match native bits.
                let product = (waiting as i32).wrapping_mul(multiplier as i32);
                let result = (old as i32).wrapping_add(product / 256).clamp(0, 65535);
                (s.industry_write)(ind, 0, output, result as u32);
            }
            (s.industry_write)(ind, 1, input, 0);
        }
    }
    (s.industry_effect)(ind, 3);
    (s.industry_effect)(ind, 4);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_payment_new(front: *mut c_void, station: u16) -> *mut Payment {
    Box::into_raw(Box::new(Payment {
        fields: Fields {
            front,
            ..Fields::default()
        },
        station,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_front(p: *const Payment) -> *mut c_void {
    unsafe { (*p).fields.front }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_afterload(p: *mut Payment, s: *const Services) {
    let s = unsafe { *s };
    let front = unsafe { (*p).fields.front };
    let station = (s.vehicle_read)(front, 0) as u16;
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
        let s = unsafe { *s };
        (s.settle)(fields.front, 0, 0, 0); // detach before visual zero test
        if fields.visual_profit != 0 || fields.visual_transfer != 0 {
            let old_company = (s.settle)(fields.front, 1, 0, 0);
            (s.settle)(fields.front, 2, fields.route_profit.saturating_neg(), 0);
            (s.settle)(
                fields.front,
                3,
                fields
                    .visual_profit
                    .saturating_add(fields.visual_transfer)
                    .wrapping_shl(8),
                0,
            );
            if fields.route_profit != 0
                && (s.settle)(fields.front, 4, 0, 0) != 0
                && (s.settle)(fields.front, 8, 0, 0) == 0
            {
                (s.settle)(fields.front, 9, 0, 0);
            }
            (s.settle)(
                fields.front,
                if fields.visual_transfer != 0 { 5 } else { 6 },
                fields.visual_transfer,
                fields.visual_profit.saturating_neg(),
            );
            (s.settle)(fields.front, 7, i64::from(old_company), 0);
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
    packet: *const c_void,
    count: u32,
    distance: u32,
    periods: u16,
) -> i64 {
    let s = unsafe { *s };
    let feeder = (s.feeder)(packet, count).saturating_neg();
    let profit = feeder
        .saturating_add(income(s, count, distance, periods, cargo))
        .saturating_mul(i64::from((s.setting)(1) as i32))
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
    packet: *const c_void,
    count: u32,
    distance: u32,
    periods: u16,
    source: u32,
    cursor: *mut NearbyIndustryCursor,
    monitors: *const crate::cargomonitor::Services,
    monitor_cursor: *mut NearbyIndustryCursor,
) {
    let s = unsafe { *s };
    let front = unsafe { (*p).fields.front };
    let station = unsafe { (*p).station };
    let company = (s.vehicle_read)(front, 1) as u8;
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
        cursor,
        unsafe { &*monitors },
        monitor_cursor,
    );
    unsafe {
        (*p).fields.route_profit = (*p).fields.route_profit.saturating_add(profit);
    }
    let visual = profit.saturating_sub((s.feeder)(packet, count));
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
    income(unsafe { *s }, pieces, distance, periods, cargo)
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
    let s = unsafe { *s };
    let mut index = 0;
    while index < unsafe { (*d).destinations.len() } {
        let ind = unsafe { (&(*d).destinations)[index] };
        trigger(ind, s);
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
        callback,
        station_read,
        industry_read,
        industry_write,
        refuses,
        accept,
        statistics,
        subsidies,
        industry_effect,
        vehicle_read,
        settle,
        feeder,
        setting,
    };
}
