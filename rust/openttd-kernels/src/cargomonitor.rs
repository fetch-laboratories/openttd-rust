/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Process-wide cargo-monitor maps, accounting and lifecycle. Native entry frames own
//! independent cursors for nested payment and monitor scans. Handles are never dereferenced
//! or retained, and no delivery allocates or inserts a key.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments
)]
use crate::game_state::GameState;
use crate::{
    bitmath_func::{gb, has_bit, sb, set_bit},
    cargo_type::CargoType,
    company_type::{CompanyID, Owner as CompanyOwner},
    industry_type::IndustryID,
    overflowsafe_type::add_u32,
    source_type::{Source, SourceID, SourceType},
    station::Station,
    town_type::TownID,
};
use std::{
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
};

/// Unique number for a company, cargo type and town or industry, from `cargomonitor.h`.
/// Bits 0–15 hold the town/industry number and bit 16 identifies an industry.
/// The cargo and company fields use the `CCB_*` start/length constants below.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoMonitorID(pub u32);

/// Start bit of the town or industry number.
pub const CCB_TOWN_IND_NUMBER_START: u8 = 0;
/// Number of bits of the town or industry number.
pub const CCB_TOWN_IND_NUMBER_LENGTH: u8 = 16;
/// Bit indicating that the town/industry number is an industry.
pub const CCB_IS_INDUSTRY_BIT: u8 = 16;
/// Start bit of the cargo type field.
pub const CCB_CARGO_TYPE_START: u8 = 19;
/// Number of bits of the cargo type field.
pub const CCB_CARGO_TYPE_LENGTH: u8 = 6;
/// Start bit of the company field.
pub const CCB_COMPANY_START: u8 = 25;
/// Number of bits of the company field.
pub const CCB_COMPANY_LENGTH: u8 = 4;

/// Active cargo monitors and their amounts since the last query or activation.
type CargoMonitorMap = BTreeMap<CargoMonitorID, i32>;
struct Owner(GameState<CargoMonitorMap>);
static PICKUPS: Owner = Owner(GameState::new(BTreeMap::new()));
static DELIVERIES: Owner = Owner(GameState::new(BTreeMap::new()));
impl Owner {
    /// Clear this company’s monitors, or every monitor for `INVALID_OWNER`.
    fn clear(&self, company: CompanyID) {
        // SAFETY: Serialized entry; no callback or reentry while borrowed.
        let map = unsafe { &mut *self.0.get() };
        if company == CompanyOwner::INVALID {
            map.clear();
        } else {
            map.retain(|&key, _| key.company() != company);
        }
    }
    /// Get and reset the amount collected since the last query or activation.
    /// Keep an active zero entry when monitoring continues; otherwise remove it.
    fn amount(&self, key: CargoMonitorID, keep: bool) -> i32 {
        let map = unsafe { &mut *self.0.get() };
        if keep {
            std::mem::replace(map.entry(key).or_insert(0), 0)
        } else {
            map.remove(&key).unwrap_or(0)
        }
    }
    fn add(&self, key: CargoMonitorID, amount: u32) {
        if let Some(value) = unsafe { &mut *self.0.get() }.get_mut(&key) {
            add_u32(value, amount);
        }
    }
    fn next(&self, after: Option<CargoMonitorID>) -> Option<(CargoMonitorID, u32)> {
        let map = unsafe { &*self.0.get() };
        let pair = after.map_or_else(
            || map.first_key_value(),
            |key| map.range((Excluded(key), Unbounded)).next(),
        );
        pair.map(|(&key, &amount)| (key, amount as u32))
    }
    fn insert(&self, key: CargoMonitorID, amount: u32) {
        unsafe { &mut *self.0.get() }
            .entry(key)
            .or_insert(amount as i32);
    }
}
fn encode(
    company: CompanyID,
    cargo: CargoType,
    id: SourceID,
    check: extern "C" fn(bool),
) -> CargoMonitorID {
    check(u32::from(cargo.0) < (1 << CCB_CARGO_TYPE_LENGTH));
    check(u32::from(company.0) < (1 << CCB_COMPANY_LENGTH));
    let mut key = 0;
    sb(
        &mut key,
        CCB_TOWN_IND_NUMBER_START,
        CCB_TOWN_IND_NUMBER_LENGTH,
        u32::from(id.0),
    );
    sb(
        &mut key,
        CCB_CARGO_TYPE_START,
        CCB_CARGO_TYPE_LENGTH,
        u32::from(cargo.0),
    );
    sb(
        &mut key,
        CCB_COMPANY_START,
        CCB_COMPANY_LENGTH,
        u32::from(company.0),
    );
    CargoMonitorID(key)
}
/// Encode a cargo monitor for pickup or delivery at an industry.
fn encode_industry(
    company: CompanyID,
    cargo: CargoType,
    id: IndustryID,
    check: extern "C" fn(bool),
) -> CargoMonitorID {
    CargoMonitorID(set_bit(
        &mut encode(company, cargo, SourceID(id.0), check).0,
        CCB_IS_INDUSTRY_BIT,
    ))
}
/// Encode a cargo monitor for pickup or delivery at a town.
fn encode_town(
    company: CompanyID,
    cargo: CargoType,
    id: TownID,
    check: extern "C" fn(bool),
) -> CargoMonitorID {
    encode(company, cargo, SourceID(id.0), check)
}
impl CargoMonitorID {
    /// Extract the company from the cargo monitor.
    fn company(self) -> CompanyID {
        CompanyID(gb(self.0, CCB_COMPANY_START, CCB_COMPANY_LENGTH) as u8)
    }
    /// Extract the cargo type from the cargo monitor.
    fn cargo(self) -> CargoType {
        CargoType(gb(self.0, CCB_CARGO_TYPE_START, CCB_CARGO_TYPE_LENGTH) as u8)
    }
    /// Does the number monitor an industry rather than a town?
    fn monitors_industry(self) -> bool {
        has_bit(self.0, CCB_IS_INDUSTRY_BIT)
    }
    /// Extract the industry, or return invalid when monitoring a town.
    fn industry(self) -> IndustryID {
        if self.monitors_industry() {
            IndustryID(gb(
                self.0,
                CCB_TOWN_IND_NUMBER_START,
                CCB_TOWN_IND_NUMBER_LENGTH,
            ) as u16)
        } else {
            IndustryID::INVALID
        }
    }
    /// Extract the town, or return invalid when monitoring an industry.
    fn town(self) -> TownID {
        if self.monitors_industry() {
            TownID::INVALID
        } else {
            TownID(gb(
                self.0,
                CCB_TOWN_IND_NUMBER_START,
                CCB_TOWN_IND_NUMBER_LENGTH,
            ) as u16)
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_encode_industry(
    company: u8,
    cargo: u8,
    id: u16,
    check: extern "C" fn(bool),
) -> u32 {
    encode_industry(CompanyID(company), CargoType(cargo), IndustryID(id), check).0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_encode_town(
    company: u8,
    cargo: u8,
    id: u16,
    check: extern "C" fn(bool),
) -> u32 {
    encode_town(CompanyID(company), CargoType(cargo), TownID(id), check).0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_company(key: u32) -> u8 {
    CargoMonitorID(key).company().0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_cargo(key: u32) -> u8 {
    CargoMonitorID(key).cargo().0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_industry(key: u32) -> u16 {
    CargoMonitorID(key).industry().0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_town(key: u32) -> u16 {
    CargoMonitorID(key).town().0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_clear_pickups(company: u8) {
    PICKUPS.clear(CompanyID(company));
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_clear_deliveries(company: u8) {
    DELIVERIES.clear(CompanyID(company));
}
#[unsafe(no_mangle)]
/// Pickup is counted on final delivery, so pickup without delivery earns no credit.
pub extern "C" fn openttd_rust_monitor_pickup(key: u32, keep: bool) -> i32 {
    PICKUPS.amount(CargoMonitorID(key), keep)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_delivery(key: u32, keep: bool) -> i32 {
    DELIVERIES.amount(CargoMonitorID(key), keep)
}
// Native TempStorage is uninitialized on the first save call. Read only a
// previously returned key, and initialize output scalars through raw writes.
unsafe fn next(owner: &Owner, first: bool, key: *mut u32, amount: *mut u32) -> bool {
    let after = if first {
        None
    } else {
        Some(CargoMonitorID(unsafe { key.read() }))
    };
    if let Some((number, value)) = owner.next(after) {
        unsafe {
            key.write(number.0);
            amount.write(value);
        }
        true
    } else {
        false
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_monitor_next_pickup_abi(
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> u8 {
    u8::from(unsafe { next(&PICKUPS, first, key, amount) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_monitor_next_delivery_abi(
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> u8 {
    u8::from(unsafe { next(&DELIVERIES, first, key, amount) })
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_insert_pickup(key: u32, amount: u32) {
    PICKUPS.insert(CargoMonitorID(key), amount);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_insert_delivery(key: u32, amount: u32) {
    DELIVERIES.insert(CargoMonitorID(key), amount);
}

#[repr(C)]
pub struct Services {
    pub station_town: extern "C" fn(*const Station) -> u16,
    pub check: extern "C" fn(bool),
}
/// `AddCargoDelivery`. `station` and its owner `state` are resolved at the
/// caller's original source point. No reference into either map spans a world
/// observation or traversal operation.
pub fn add_delivery(
    s: &Services,
    cargo: u8,
    company: u8,
    amount: u32,
    source: Source,
    station: *const Station,
    state: *mut crate::station_service::Service,
    destination: IndustryID,
) {
    if amount == 0 {
        return;
    }
    let cargo = CargoType(cargo);
    let company = CompanyID(company);
    if source.is_valid() {
        // Handle pickup update.
        let key = match SourceType(source.kind) {
            SourceType::Industry => Some(encode_industry(
                company,
                cargo,
                IndustryID(source.id),
                s.check,
            )),
            SourceType::Town => Some(encode_town(company, cargo, TownID(source.id), s.check)),
            _ => None,
        };
        if let Some(key) = key {
            PICKUPS.add(key, amount);
        }
    }
    // Handle delivery. Delivery in the right area is sufficient to prevent
    // trouble with neighbouring industries or houses.

    // Town delivery.
    let town = TownID((s.station_town)(station));
    DELIVERIES.add(encode_town(company, cargo, town, s.check), amount);
    // Industry delivery.
    // SAFETY: The caller's live station owner; each entry is copied before use.
    unsafe {
        crate::station::for_each_industry_near(state, |near| {
            if near.id == destination {
                DELIVERIES.add(encode_industry(company, cargo, near.id, s.check), amount);
            }
            true
        });
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoMonitorServices" ["probe"] => Services {
        station_town, check,
    };
}
