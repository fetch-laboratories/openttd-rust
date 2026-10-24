/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete process-wide cargo-monitor maps, accounting and lifecycle.
//! The native game-state mutex serializes every entry, including script queries,
//! saving and loading. No owner borrow survives a native callback. Native entry
//! frames own independent cursors for nested payment and monitor scans. Handles
//! are never dereferenced or retained; no delivery allocates or inserts a key.
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
    company_type::INVALID_OWNER,
    overflowsafe_type::add_u32,
    source_type::Source,
    station::{NearbyIndustryCursor, Station},
};
use std::{
    collections::BTreeMap,
    ffi::c_void,
    ops::Bound::{Excluded, Unbounded},
};

pub const CCB_TOWN_IND_NUMBER_START: u8 = 0;
pub const CCB_TOWN_IND_NUMBER_LENGTH: u8 = 16;
pub const CCB_IS_INDUSTRY_BIT: u8 = 16;
pub const CCB_CARGO_TYPE_START: u8 = 19;
pub const CCB_CARGO_TYPE_LENGTH: u8 = 6;
pub const CCB_COMPANY_START: u8 = 25;
pub const CCB_COMPANY_LENGTH: u8 = 4;

type MonitorMap = BTreeMap<u32, i32>;
struct Owner(GameState<MonitorMap>);
static PICKUPS: Owner = Owner(GameState::new(BTreeMap::new()));
static DELIVERIES: Owner = Owner(GameState::new(BTreeMap::new()));
impl Owner {
    fn clear(&self, company: u8) {
        // SAFETY: Serialized entry; no callback or reentry while borrowed.
        let map = unsafe { &mut *self.0.get() };
        if company == INVALID_OWNER {
            map.clear();
        } else {
            map.retain(|&key, _| openttd_rust_monitor_company(key) != company);
        }
    }
    fn amount(&self, key: u32, keep: bool) -> i32 {
        let map = unsafe { &mut *self.0.get() };
        if keep {
            std::mem::replace(map.entry(key).or_insert(0), 0)
        } else {
            map.remove(&key).unwrap_or(0)
        }
    }
    fn add(&self, key: u32, amount: u32) {
        if let Some(value) = unsafe { &mut *self.0.get() }.get_mut(&key) {
            add_u32(value, amount);
        }
    }
    fn next(&self, after: Option<u32>) -> Option<(u32, u32)> {
        let map = unsafe { &*self.0.get() };
        let pair = after.map_or_else(
            || map.first_key_value(),
            |key| map.range((Excluded(key), Unbounded)).next(),
        );
        pair.map(|(&key, &amount)| (key, amount as u32))
    }
    fn insert(&self, key: u32, amount: u32) {
        unsafe { &mut *self.0.get() }
            .entry(key)
            .or_insert(amount as i32);
    }
}
fn encode(company: u8, cargo: u8, id: u16, check: extern "C" fn(bool)) -> u32 {
    check(u32::from(cargo) < (1 << CCB_CARGO_TYPE_LENGTH));
    check(u32::from(company) < (1 << CCB_COMPANY_LENGTH));
    let mut key = 0;
    sb(
        &mut key,
        CCB_TOWN_IND_NUMBER_START,
        CCB_TOWN_IND_NUMBER_LENGTH,
        u32::from(id),
    );
    sb(
        &mut key,
        CCB_CARGO_TYPE_START,
        CCB_CARGO_TYPE_LENGTH,
        u32::from(cargo),
    );
    sb(
        &mut key,
        CCB_COMPANY_START,
        CCB_COMPANY_LENGTH,
        u32::from(company),
    );
    key
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_encode_industry(
    company: u8,
    cargo: u8,
    id: u16,
    check: extern "C" fn(bool),
) -> u32 {
    set_bit(&mut encode(company, cargo, id, check), CCB_IS_INDUSTRY_BIT)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_encode_town(
    company: u8,
    cargo: u8,
    id: u16,
    check: extern "C" fn(bool),
) -> u32 {
    encode(company, cargo, id, check)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_company(key: u32) -> u8 {
    gb(key, CCB_COMPANY_START, CCB_COMPANY_LENGTH) as u8
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_cargo(key: u32) -> u8 {
    gb(key, CCB_CARGO_TYPE_START, CCB_CARGO_TYPE_LENGTH) as u8
}
fn monitor_monitors_industry(key: u32) -> bool {
    has_bit(key, CCB_IS_INDUSTRY_BIT)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_industry(key: u32) -> u16 {
    if monitor_monitors_industry(key) {
        gb(key, CCB_TOWN_IND_NUMBER_START, CCB_TOWN_IND_NUMBER_LENGTH) as u16
    } else {
        u16::MAX
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_town(key: u32) -> u16 {
    if monitor_monitors_industry(key) {
        u16::MAX
    } else {
        gb(key, CCB_TOWN_IND_NUMBER_START, CCB_TOWN_IND_NUMBER_LENGTH) as u16
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_clear_pickups(company: u8) {
    PICKUPS.clear(company);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_clear_deliveries(company: u8) {
    DELIVERIES.clear(company);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_pickup(key: u32, keep: bool) -> i32 {
    PICKUPS.amount(key, keep)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_delivery(key: u32, keep: bool) -> i32 {
    DELIVERIES.amount(key, keep)
}
// Native TempStorage is uninitialized on the first save call. Read only a
// previously returned key, and initialize output scalars through raw writes.
unsafe fn next(owner: &Owner, first: bool, key: *mut u32, amount: *mut u32) -> bool {
    let after = if first {
        None
    } else {
        Some(unsafe { key.read() })
    };
    if let Some((number, value)) = owner.next(after) {
        unsafe {
            key.write(number);
            amount.write(value);
        }
        true
    } else {
        false
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_monitor_next_pickup(
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> bool {
    unsafe { next(&PICKUPS, first, key, amount) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_monitor_next_delivery(
    first: bool,
    key: *mut u32,
    amount: *mut u32,
) -> bool {
    unsafe { next(&DELIVERIES, first, key, amount) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_insert_pickup(key: u32, amount: u32) {
    PICKUPS.insert(key, amount);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_monitor_insert_delivery(key: u32, amount: u32) {
    DELIVERIES.insert(key, amount);
}

#[repr(C)]
pub struct Services {
    pub station_town: extern "C" fn(*const Station) -> u16,
    pub begin_nearby: extern "C" fn(*mut NearbyIndustryCursor, *const Station),
    pub next_nearby: extern "C" fn(*mut NearbyIndustryCursor) -> *mut crate::industry::ShellHandle,
    pub industry_id: extern "C" fn(*mut c_void) -> u16,
    pub check: extern "C" fn(bool),
}
/// `AddCargoDelivery`. `station` is resolved at the caller's original source point.
/// No reference into either map spans a world observation or traversal operation.
pub fn add_delivery(
    s: &Services,
    cargo: u8,
    company: u8,
    amount: u32,
    source: Source,
    station: *const Station,
    destination: u16,
    cursor: *mut NearbyIndustryCursor,
) {
    if amount == 0 {
        return;
    }
    if source.is_valid() {
        let key = match source.kind {
            Source::INDUSTRY => Some(openttd_rust_monitor_encode_industry(
                company, cargo, source.id, s.check,
            )),
            Source::TOWN => Some(openttd_rust_monitor_encode_town(
                company, cargo, source.id, s.check,
            )),
            _ => None,
        };
        if let Some(key) = key {
            PICKUPS.add(key, amount);
        }
    }
    let town = (s.station_town)(station);
    DELIVERIES.add(
        openttd_rust_monitor_encode_town(company, cargo, town, s.check),
        amount,
    );
    (s.begin_nearby)(cursor, station);
    loop {
        let industry = (s.next_nearby)(cursor);
        if industry.is_null() {
            break;
        }
        let id = (s.industry_id)(industry.cast());
        if id != destination {
            continue;
        }
        DELIVERIES.add(
            openttd_rust_monitor_encode_industry(company, cargo, id, s.check),
            amount,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_monitor_add(
    s: *const Services,
    cargo: u8,
    company: u8,
    amount: u32,
    source: u16,
    kind: u8,
    station: *const Station,
    destination: u16,
    cursor: *mut NearbyIndustryCursor,
) {
    add_delivery(
        unsafe { &*s },
        cargo,
        company,
        amount,
        Source { id: source, kind },
        station,
        destination,
        cursor,
    );
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoMonitorServices" ["probe"] => Services {
        station_town, begin_nearby, next_nearby, industry_id, check,
    };
}
