/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Stack-owned cargo collector. Typed world iteration stays outside Rust.

use crate::script_list::List;
use crate::script_stationlist::{CargoMode, CargoSelector};
use crate::station_type::StationID;

/// `CargoCollector` scalar state pinned by `station_cargo_ffi.h` on 32/64-bit
/// targets; the record keeps base integers.
#[repr(C)]
pub struct Collector {
    pub amount: u32,
    pub previous: u32,
    pub last_key: u16,
    pub other: u16,
    pub origin: u16,
    pub selector: u8,
    pub finalized: u8,
}

impl Collector {
    /// `CargoCollector::CargoCollector`, with the template selector as state.
    pub(crate) fn new(selector: CargoSelector, other: StationID) -> Self {
        assert!(selector.0 <= CargoSelector::CS_FROM_BY_VIA.0);
        Self {
            amount: 0,
            previous: 0,
            last_key: StationID::INVALID.0,
            other: other.0,
            origin: StationID::INVALID.0,
            selector: selector.0,
            finalized: 0,
        }
    }

    /// `CargoCollector::SetValue`.
    fn flush(&self, list: &mut List) {
        if self.amount != 0 {
            list.cargo_merge(i64::from(self.last_key), i64::from(self.amount));
        }
    }

    /// `CargoCollector::Update<Tselector>`.
    pub(crate) fn packet(&mut self, list: &mut List, from: StationID, via: StationID, amount: u32) {
        assert_eq!(self.finalized, 0);
        let other = StationID(self.other);
        // Filtering precedes even observing/changing the pending run key.
        let key = match CargoSelector(self.selector) {
            CargoSelector::CS_VIA_BY_FROM if via != other => return,
            CargoSelector::CS_BY_FROM | CargoSelector::CS_VIA_BY_FROM => from,
            CargoSelector::CS_FROM_BY_VIA if from != other => return,
            CargoSelector::CS_BY_VIA | CargoSelector::CS_FROM_BY_VIA => via,
            _ => unreachable!("valid selector"),
        };
        if key.0 == self.last_key {
            self.amount = self.amount.wrapping_add(amount);
        } else {
            self.flush(list);
            self.amount = amount;
            self.last_key = key.0;
        }
    }

    /// Begin one `FlowStatMap` entry of `ScriptStationList_CargoPlanned::Add`:
    /// `uint prev = 0`.
    pub(crate) fn origin(&mut self, origin: StationID) {
        assert_eq!(self.finalized, 0);
        self.origin = origin.0;
        self.previous = 0;
    }

    /// One share of `ScriptStationList_CargoPlanned::Add`:
    /// `Update(iter->first, flow_iter->second, flow_iter->first - prev)`.
    pub(crate) fn share(&mut self, list: &mut List, via: StationID, cumulative: u32) {
        let amount = cumulative.wrapping_sub(self.previous);
        self.packet(list, StationID(self.origin), via, amount);
        // Every visited share advances this, including filtered/restricted shares.
        self.previous = cumulative;
    }

    /// `CargoCollector::~CargoCollector`, once.
    pub(crate) fn finish(&mut self, list: &mut List) {
        if self.finalized == 0 {
            self.flush(list);
            self.finalized = 1;
        }
    }
}

/// Select the typed traversal that the native `ScriptStationList_Cargo`
/// constructor performs. The result is the fork's traversal code: 0 all packets
/// (`CargoWaiting::Add`), 1 the `equal_range(via)` packets of
/// `CargoWaitingViaByFrom`, 2 all flows (`CargoPlanned::Add`), 3 the
/// `flows.find(from)` entry of `CargoPlannedFromByVia`; `u8::MAX` for an
/// invalid mode or selector.
pub fn plan(mode: CargoMode, selector: CargoSelector) -> u8 {
    match (mode, selector) {
        (CargoMode::CM_WAITING, CargoSelector::CS_VIA_BY_FROM) => 1,
        (
            CargoMode::CM_WAITING,
            CargoSelector::CS_BY_FROM | CargoSelector::CS_BY_VIA | CargoSelector::CS_FROM_BY_VIA,
        ) => 0,
        (CargoMode::CM_PLANNED, CargoSelector::CS_FROM_BY_VIA) => 3,
        (
            CargoMode::CM_PLANNED,
            CargoSelector::CS_BY_FROM | CargoSelector::CS_VIA_BY_FROM | CargoSelector::CS_BY_VIA,
        ) => 2,
        _ => u8::MAX,
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoCollector" ["probe"] => Collector {
        amount,
        previous,
        last_key,
        other,
        origin,
        selector,
        finalized,
    };
}
