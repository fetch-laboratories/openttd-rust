/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Stack-owned cargo collector. Typed world iteration stays outside Rust.

use crate::script_list::List;

/// Scalar state pinned by `station_cargo_ffi.h` on 32/64-bit targets.
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
    pub(crate) fn new(selector: u8, other: u16) -> Self {
        assert!(selector < 4);
        Self {
            amount: 0,
            previous: 0,
            last_key: u16::MAX,
            other,
            origin: u16::MAX,
            selector,
            finalized: 0,
        }
    }

    fn flush(&self, list: &mut List) {
        if self.amount != 0 {
            list.cargo_merge(i64::from(self.last_key), i64::from(self.amount));
        }
    }

    pub(crate) fn packet(&mut self, list: &mut List, from: u16, via: u16, amount: u32) {
        assert_eq!(self.finalized, 0);
        // Filtering precedes even observing/changing the pending run key.
        let key = match self.selector {
            0 => from,
            1 if via == self.other => from,
            2 => via,
            3 if from == self.other => via,
            1 | 3 => return,
            _ => unreachable!("valid selector"),
        };
        if key == self.last_key {
            self.amount = self.amount.wrapping_add(amount);
        } else {
            self.flush(list);
            self.amount = amount;
            self.last_key = key;
        }
    }

    pub(crate) fn origin(&mut self, origin: u16) {
        assert_eq!(self.finalized, 0);
        self.origin = origin;
        self.previous = 0;
    }

    pub(crate) fn share(&mut self, list: &mut List, via: u16, cumulative: u32) {
        let amount = cumulative.wrapping_sub(self.previous);
        self.packet(list, self.origin, via, amount);
        // Every visited share advances this, including filtered/restricted shares.
        self.previous = cumulative;
    }

    pub(crate) fn finish(&mut self, list: &mut List) {
        if self.finalized == 0 {
            self.flush(list);
            self.finalized = 1;
        }
    }
}

/// Select actual typed traversal: packets all/range, flows all/find.
pub fn plan(mode: u8, selector: u8) -> u8 {
    match (mode, selector) {
        (0, 1) => 1,
        (0, 0 | 2 | 3) => 0,
        (1, 3) => 3,
        (1, 0..=2) => 2,
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
