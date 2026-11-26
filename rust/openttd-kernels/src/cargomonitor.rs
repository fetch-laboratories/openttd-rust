/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `cargomonitor.h` and `cargomonitor.cpp`: the cargo transport monitoring.
//!
//! The two maps are process-wide statics, serialised by the game-state mutex. No reference into
//! either map is alive across a leaf or the walk over a station's nearby industries. The `next`
//! exports, which the save code calls with uninitialised storage, write only their two output
//! scalars.
use crate::bitmath_func::{gb, has_bit, sb, set_bit};
use crate::cargo_type::CargoType;
use crate::company_type::{CompanyID, MAX_COMPANIES, Owner};
use crate::game_state::GameState;
use crate::industry_type::IndustryID;
use crate::overflowsafe_type::add_u32;
use crate::source_type::{Source, SourceType};
use crate::station_service::LoadingStation;
use crate::town_type::TownID;
use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Unbounded};

/// Unique number for a company / cargo type / (town or industry).
/// Encoding is as follows:
/// - bits 0-15 town or industry number
/// - bit 16 is set if it is an industry number (else it is a town number).
/// - bits 19-23 Cargo type.
/// - bits 24-31 %Company number.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoMonitorID(pub u32);

/// Map type for storing and updating active cargo monitor numbers and their amounts.
type CargoMonitorMap = BTreeMap<CargoMonitorID, i32>;

/// Map of monitored pick-ups to the amount since last query/activation.
static CARGO_PICKUPS: GameState<CargoMonitorMap> = GameState::new(BTreeMap::new());
/// Map of monitored deliveries to the amount since last query/activation.
static CARGO_DELIVERIES: GameState<CargoMonitorMap> = GameState::new(BTreeMap::new());

/// The map `map` holds, for one operation that ends before any leaf.
#[allow(
    unsafe_code,
    reason = "borrows a process-wide map under the module's contract"
)]
fn monitor_map(map: &GameState<CargoMonitorMap>) -> &mut CargoMonitorMap {
    // SAFETY: The game-state mutex serialises every access, and each caller ends the borrow
    // before a leaf or the industry walk.
    unsafe { &mut *map.get() }
}

/* Constants for encoding and extracting cargo monitors. */
/// Start bit of the town or industry number.
pub const CCB_TOWN_IND_NUMBER_START: u8 = 0;
/// Number of bits of the town or industry number.
pub const CCB_TOWN_IND_NUMBER_LENGTH: u8 = 16;
/// Bit indicating the town/industry number is an industry.
pub const CCB_IS_INDUSTRY_BIT: u8 = 16;
/// Start bit of the cargo type field.
pub const CCB_CARGO_TYPE_START: u8 = 19;
/// Number of bits of the cargo type field.
pub const CCB_CARGO_TYPE_LENGTH: u8 = 6;
/// Start bit of the company field.
pub const CCB_COMPANY_START: u8 = 25;
/// Number of bits of the company field.
pub const CCB_COMPANY_LENGTH: u8 = 4;

const _: () = assert!(CargoType::NUM_CARGO.0 as u32 <= (1 << CCB_CARGO_TYPE_LENGTH));
const _: () = assert!(MAX_COMPANIES as u32 <= (1 << CCB_COMPANY_LENGTH));

/// `EncodeCargoIndustryMonitor`: encode a cargo monitor for pickup or delivery at an industry.
/// `company` performs the transport, `ctype` is the cargo type being transported and `ind` the
/// industry providing or accepting the cargo. Returns the encoded cargo/company/industry number.
pub fn encode_cargo_industry_monitor(
    company: CompanyID,
    ctype: CargoType,
    ind: IndustryID,
) -> CargoMonitorID {
    crate::services_assert::assert_with(|| u32::from(ctype.0) < (1 << CCB_CARGO_TYPE_LENGTH));
    crate::services_assert::assert_with(|| u32::from(company.0) < (1 << CCB_COMPANY_LENGTH));

    let mut ret = 0_u32;
    sb(
        &mut ret,
        CCB_TOWN_IND_NUMBER_START,
        CCB_TOWN_IND_NUMBER_LENGTH,
        u32::from(ind.0),
    );
    set_bit(&mut ret, CCB_IS_INDUSTRY_BIT);
    sb(
        &mut ret,
        CCB_CARGO_TYPE_START,
        CCB_CARGO_TYPE_LENGTH,
        u32::from(ctype.0),
    );
    sb(
        &mut ret,
        CCB_COMPANY_START,
        CCB_COMPANY_LENGTH,
        u32::from(company.0),
    );
    CargoMonitorID(ret)
}

/// `EncodeCargoTownMonitor`: encode a cargo monitoring number for pickup or delivery at a town.
/// `company` performs the transport, `ctype` is the cargo type being transported and `town` the
/// town providing or accepting the cargo. Returns the encoded cargo/company/town number.
pub fn encode_cargo_town_monitor(
    company: CompanyID,
    ctype: CargoType,
    town: TownID,
) -> CargoMonitorID {
    crate::services_assert::assert_with(|| u32::from(ctype.0) < (1 << CCB_CARGO_TYPE_LENGTH));
    crate::services_assert::assert_with(|| u32::from(company.0) < (1 << CCB_COMPANY_LENGTH));

    let mut ret = 0_u32;
    sb(
        &mut ret,
        CCB_TOWN_IND_NUMBER_START,
        CCB_TOWN_IND_NUMBER_LENGTH,
        u32::from(town.0),
    );
    sb(
        &mut ret,
        CCB_CARGO_TYPE_START,
        CCB_CARGO_TYPE_LENGTH,
        u32::from(ctype.0),
    );
    sb(
        &mut ret,
        CCB_COMPANY_START,
        CCB_COMPANY_LENGTH,
        u32::from(company.0),
    );
    CargoMonitorID(ret)
}

/// `DecodeMonitorCompany`: extract the company from the cargo monitor `num`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`static_cast<CompanyID>(GB(num, CCB_COMPANY_START, CCB_COMPANY_LENGTH))`"
)]
pub fn decode_monitor_company(num: CargoMonitorID) -> CompanyID {
    CompanyID(gb(num.0, CCB_COMPANY_START, CCB_COMPANY_LENGTH) as u8)
}

/// `DecodeMonitorCargoType`: extract the cargo type from the cargo monitor `num`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`GB(num, CCB_CARGO_TYPE_START, CCB_CARGO_TYPE_LENGTH)` returned as `CargoType`"
)]
pub fn decode_monitor_cargo_type(num: CargoMonitorID) -> CargoType {
    CargoType(gb(num.0, CCB_CARGO_TYPE_START, CCB_CARGO_TYPE_LENGTH) as u8)
}

/// `MonitorMonitorsIndustry`: does the cargo number `num` monitor an industry or a town? True
/// if monitoring an industry, false if monitoring a town.
pub fn monitor_monitors_industry(num: CargoMonitorID) -> bool {
    has_bit(num.0, CCB_IS_INDUSTRY_BIT)
}

/// `DecodeMonitorIndustry`: extract the industry number from the cargo monitor `num`, or
/// `IndustryID::Invalid()` if the number does not monitor an industry.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`static_cast<IndustryID>(GB(num, CCB_TOWN_IND_NUMBER_START, CCB_TOWN_IND_NUMBER_LENGTH))`"
)]
pub fn decode_monitor_industry(num: CargoMonitorID) -> IndustryID {
    if !monitor_monitors_industry(num) {
        return IndustryID::INVALID;
    }
    IndustryID(gb(num.0, CCB_TOWN_IND_NUMBER_START, CCB_TOWN_IND_NUMBER_LENGTH) as u16)
}

/// `DecodeMonitorTown`: extract the town number from the cargo monitor `num`, or
/// `TownID::Invalid()` if the number does not monitor a town.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`static_cast<TownID>(GB(num, CCB_TOWN_IND_NUMBER_START, CCB_TOWN_IND_NUMBER_LENGTH))`"
)]
pub fn decode_monitor_town(num: CargoMonitorID) -> TownID {
    if monitor_monitors_industry(num) {
        return TownID::INVALID;
    }
    TownID(gb(num.0, CCB_TOWN_IND_NUMBER_START, CCB_TOWN_IND_NUMBER_LENGTH) as u16)
}

/// `ClearCargoMonitoring`: helper method for `ClearCargoPickupMonitoring` and
/// `ClearCargoDeliveryMonitoring`. Clears all monitors of `cargo_monitor_map` that belong to the
/// specified company or all if `INVALID_OWNER` is specified as company.
fn clear_cargo_monitoring(cargo_monitor_map: &GameState<CargoMonitorMap>, company: CompanyID) {
    let cargo_monitor_map = monitor_map(cargo_monitor_map);
    if company == Owner::INVALID {
        cargo_monitor_map.clear();
        return;
    }

    cargo_monitor_map.retain(|&key, _| decode_monitor_company(key) != company);
}

/// `ClearCargoPickupMonitoring`: clear all pick-up cargo monitors for `company`, or if
/// `INVALID_OWNER` is passed, all pick-up monitors regardless of company.
pub(crate) fn clear_cargo_pickup_monitoring(company: CompanyID) {
    clear_cargo_monitoring(&CARGO_PICKUPS, company);
}

/// `ClearCargoDeliveryMonitoring`: clear all delivery cargo monitors for `company`, or if
/// `INVALID_OWNER` is passed, all delivery monitors regardless of company.
pub(crate) fn clear_cargo_delivery_monitoring(company: CompanyID) {
    clear_cargo_monitoring(&CARGO_DELIVERIES, company);
}

/// `GetAmount`: get and reset the amount associated with a cargo monitor. `monitor_map` is the
/// monitoring map to search (and reset for the queried entry), `monitor` the cargo monitor to
/// query/reset; with `keep_monitoring` monitoring continues after the call. Returns the amount
/// collected since last query/activation for the monitored combination.
fn get_amount(
    monitor_map: &GameState<CargoMonitorMap>,
    monitor: CargoMonitorID,
    keep_monitoring: bool,
) -> i32 {
    let monitor_map = self::monitor_map(monitor_map);
    if let Some(iter) = monitor_map.get_mut(&monitor) {
        let result = *iter;
        *iter = 0;
        if !keep_monitoring {
            monitor_map.remove(&monitor);
        }
        result
    } else {
        if keep_monitoring {
            monitor_map.insert(monitor, 0);
        }
        0
    }
}

/// `GetDeliveryAmount`: get the amount of cargo delivered for the given cargo monitor since
/// activation or last query; with `keep_monitoring` monitoring continues after the call.
pub(crate) fn get_delivery_amount(monitor: CargoMonitorID, keep_monitoring: bool) -> i32 {
    get_amount(&CARGO_DELIVERIES, monitor, keep_monitoring)
}

/// `GetPickupAmount`: get the amount of cargo picked up for the given cargo monitor since
/// activation or last query; with `keep_monitoring` monitoring continues after the call.
/// Cargo pick up is counted on final delivery, to prevent users getting credit for picking up
/// cargo without delivering it.
pub(crate) fn get_pickup_amount(monitor: CargoMonitorID, keep_monitoring: bool) -> i32 {
    get_amount(&CARGO_PICKUPS, monitor, keep_monitoring)
}

/// `iter->second += amount` on the entry of `num` in `map`, if it is monitored.
fn add_amount(map: &GameState<CargoMonitorMap>, num: CargoMonitorID, amount: u32) {
    if let Some(iter) = monitor_map(map).get_mut(&num) {
        add_u32(iter, amount);
    }
}

/// `AddCargoDelivery`: cargo was delivered to its final destination, update the pickup and
/// delivery maps. `cargo_type` is the type of cargo, `company` the company delivering it,
/// `amount` the amount of cargo delivered, `src` its source, `st` the station where the cargo
/// is delivered to and `dest` the industry index where the cargo is delivered to. `s` reads the
/// station's town.
#[allow(
    unsafe_code,
    reason = "walks the live station's nearby industries and reads its town"
)]
pub(crate) fn add_cargo_delivery(
    s: &crate::cargo_payment::Services,
    cargo_type: CargoType,
    company: CompanyID,
    amount: u32,
    src: Source,
    st: LoadingStation,
    dest: IndustryID,
) {
    if amount == 0 {
        return;
    }

    if src.is_valid() {
        /* Handle pickup update. */
        match SourceType(src.kind) {
            SourceType::Industry => {
                let num = encode_cargo_industry_monitor(company, cargo_type, src.to_industry_id());
                add_amount(&CARGO_PICKUPS, num, amount);
            }
            SourceType::Town => {
                let num = encode_cargo_town_monitor(company, cargo_type, src.to_town_id());
                add_amount(&CARGO_PICKUPS, num, amount);
            }
            _ => {}
        }
    }

    /* Handle delivery.
     * Note that delivery in the right area is sufficient to prevent trouble with neighbouring industries or houses. */

    /* Town delivery. */
    // SAFETY: The payment table names the immutable process-lifetime resolver table.
    let town = crate::industry::resolver::town_index((s.town)(st.native), unsafe { &*s.resolver });
    let num = encode_cargo_town_monitor(company, cargo_type, town);
    add_amount(&CARGO_DELIVERIES, num, amount);

    /* Industry delivery. */
    // SAFETY: The caller's live station owner; each entry is copied before its visit.
    unsafe {
        crate::station::for_each_industry_near(st.state, |i| {
            if i.id != dest {
                return true;
            }
            let num = encode_cargo_industry_monitor(company, cargo_type, i.id);
            add_amount(&CARGO_DELIVERIES, num, amount);
            true
        });
    }
}

/// `EncodeCargoIndustryMonitor`, for the header.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_encode_industry(company: u8, cargo: u8, id: u16) -> u32 {
    encode_cargo_industry_monitor(CompanyID(company), CargoType(cargo), IndustryID(id)).0
}
/// `EncodeCargoTownMonitor`, for the header.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_encode_town(company: u8, cargo: u8, id: u16) -> u32 {
    encode_cargo_town_monitor(CompanyID(company), CargoType(cargo), TownID(id)).0
}
/// `DecodeMonitorCompany`, for the header.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_company(key: u32) -> u8 {
    decode_monitor_company(CargoMonitorID(key)).0
}
/// `DecodeMonitorCargoType`, for the header.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_cargo(key: u32) -> u8 {
    decode_monitor_cargo_type(CargoMonitorID(key)).0
}
/// `DecodeMonitorIndustry`, for the header.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_industry(key: u32) -> u16 {
    decode_monitor_industry(CargoMonitorID(key)).0
}
/// `DecodeMonitorTown`, for the header.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_town(key: u32) -> u16 {
    decode_monitor_town(CargoMonitorID(key)).0
}
/// `ClearCargoPickupMonitoring`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_clear_pickups(company: u8) {
    clear_cargo_pickup_monitoring(CompanyID(company));
}
/// `ClearCargoDeliveryMonitoring`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_clear_deliveries(company: u8) {
    clear_cargo_delivery_monitoring(CompanyID(company));
}
/// `GetPickupAmount`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_pickup(key: u32, keep: bool) -> i32 {
    get_pickup_amount(CargoMonitorID(key), keep)
}
/// `GetDeliveryAmount`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_delivery(key: u32, keep: bool) -> i32 {
    get_delivery_amount(CargoMonitorID(key), keep)
}

/// The save code's `for (const auto &[number, amount] : map)` over `map`: the first entry for
/// `first`, else the entry after the key `*key` the previous call stored, into `*key` and
/// `*amount`. Returns whether an entry was stored.
///
/// # Safety
/// `key` and `amount` address writable scalars; `*key` holds the previous call's key unless
/// `first`.
#[allow(
    unsafe_code,
    reason = "reads and writes the save code's output scalars"
)]
#[allow(
    clippy::cast_sign_loss,
    reason = "`OverflowSafeInt32` saved as the chunk's `uint32_t` amount"
)]
unsafe fn next(
    map: &GameState<CargoMonitorMap>,
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> bool {
    let map = monitor_map(map);
    let entry = if first {
        map.first_key_value()
    } else {
        // SAFETY: The caller's previous key; the save code's storage is uninitialised only on
        // the first call.
        let after = CargoMonitorID(unsafe { key.read() });
        map.range((Excluded(after), Unbounded)).next()
    };
    let Some((&number, &value)) = entry else {
        return false;
    };
    // SAFETY: The caller's writable output scalars, written without a reference.
    unsafe {
        key.write(number.0);
        amount.write(value as u32);
    }
    true
}
/// # Safety
/// As `next`.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the save code's scalars"
)]
pub unsafe extern "C" fn openttd_rust_monitor_next_pickup_abi(
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> u8 {
    // SAFETY: The caller's contract.
    u8::from(unsafe { next(&CARGO_PICKUPS, first, key, amount) })
}
/// # Safety
/// As `next`.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the save code's scalars"
)]
pub unsafe extern "C" fn openttd_rust_monitor_next_delivery_abi(
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> u8 {
    // SAFETY: The caller's contract.
    u8::from(unsafe { next(&CARGO_DELIVERIES, first, key, amount) })
}

/// The load code's `_cargo_pickups.insert(storage)`: a duplicate key keeps the first amount.
#[allow(
    clippy::cast_possible_wrap,
    reason = "the chunk's `uint32_t` amount stored as `OverflowSafeInt32`"
)]
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_insert_pickup(key: u32, amount: u32) {
    monitor_map(&CARGO_PICKUPS)
        .entry(CargoMonitorID(key))
        .or_insert(amount as i32);
}
/// The load code's `_cargo_deliveries.insert(storage)`: a duplicate key keeps the first amount.
#[allow(
    clippy::cast_possible_wrap,
    reason = "the chunk's `uint32_t` amount stored as `OverflowSafeInt32`"
)]
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_monitor_insert_delivery(key: u32, amount: u32) {
    monitor_map(&CARGO_DELIVERIES)
        .entry(CargoMonitorID(key))
        .or_insert(amount as i32);
}
