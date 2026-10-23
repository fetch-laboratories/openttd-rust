/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Deterministic `ScriptList` storage, ordering, mutation accounting and live cursor.
//! C++ owns all VM/world/serialization callbacks. Scalar borrows end at each return.

#[path = "script_list_control.rs"]
pub(crate) mod control;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};

#[allow(clippy::struct_excessive_bools)] // Independent original policy/initialized/delayed-end flags.
pub(crate) struct List {
    items: BTreeMap<i64, i64>,
    values: BTreeSet<(i64, i64)>,
    sort_by_item: bool,
    ascending: bool,
    initialized: bool,
    ended: bool,
    pending: Option<i64>,
    modifications: i32,
}

impl Default for List {
    fn default() -> Self {
        Self {
            items: BTreeMap::new(),
            values: BTreeSet::new(),
            sort_by_item: false,
            ascending: false,
            initialized: false,
            ended: true,
            pending: None,
            modifications: 0,
        }
    }
}

impl List {
    pub(crate) fn cargo_merge(&mut self, key: i64, amount: i64) {
        if let Some(value) = self.items.get(&key).copied() {
            // Fresh public collector: <2^24 u16 packets or <=2^16 u32 origin totals fit i64.
            self.set(
                key,
                value.checked_add(amount).expect("defined cargo list value"),
            );
        } else {
            self.add(key, amount);
        }
    }

    fn touch(&mut self) {
        // Match the compiled original's signed int wrap; tokens are compared for equality.
        self.modifications = self.modifications.wrapping_add(1);
    }
    fn first(&self, ascending: bool) -> Option<i64> {
        if self.sort_by_item {
            if ascending {
                self.items.first_key_value()
            } else {
                self.items.last_key_value()
            }
            .map(|(k, _)| *k)
        } else {
            if ascending {
                self.values.first()
            } else {
                self.values.last()
            }
            .map(|(_, k)| *k)
        }
    }
    fn successor(&self, item: i64) -> Option<i64> {
        if self.sort_by_item {
            if self.ascending {
                self.items.range((Excluded(item), Unbounded)).next()
            } else {
                self.items.range((Unbounded, Excluded(item))).next_back()
            }
            .map(|(k, _)| *k)
        } else {
            let pair = (self.items[&item], item);
            if self.ascending {
                self.values.range((Excluded(pair), Unbounded)).next()
            } else {
                self.values.range((Unbounded, Excluded(pair))).next_back()
            }
            .map(|(_, k)| *k)
        }
    }
    fn advance(&mut self) {
        self.pending = if let Some(item) = self.pending {
            self.successor(item)
        } else {
            self.ended = true;
            None
        };
    }
    fn remove_pending(&mut self, item: i64) {
        if !self.items.is_empty() && !self.ended && self.pending == Some(item) {
            self.advance();
        }
    }
    fn begin(&mut self) -> i64 {
        self.initialized = true;
        self.pending = self.first(self.ascending);
        if self.pending.is_none() {
            return 0;
        }
        self.ended = false;
        let current = self.pending.unwrap();
        self.advance();
        current
    }
    fn next(&mut self) -> i64 {
        if !self.initialized || self.items.is_empty() || self.ended {
            return 0;
        }
        let current = self.pending.unwrap_or(0);
        self.advance();
        current
    }
    fn clear(&mut self) {
        self.touch();
        self.items.clear();
        self.values.clear();
        self.pending = None;
        self.ended = true;
    }
    fn add(&mut self, item: i64, value: i64) {
        self.touch();
        if self.items.contains_key(&item) {
            return;
        }
        self.items.insert(item, value);
        self.values.insert((value, item));
    }
    fn remove(&mut self, item: i64) {
        self.touch();
        let Some(value) = self.items.get(&item).copied() else {
            return;
        };
        self.remove_pending(item);
        assert!(self.values.remove(&(value, item)));
        self.items.remove(&item);
    }
    fn set(&mut self, item: i64, value: i64) -> bool {
        self.touch();
        let Some(old) = self.items.get(&item).copied() else {
            return false;
        };
        if old == value {
            return true;
        }
        // This skips the pending item even in item-sort mode, as the original does.
        self.remove_pending(item);
        assert!(self.values.remove(&(old, item)));
        self.items.insert(item, value);
        self.values.insert((value, item));
        true
    }
    fn sort(&mut self, kind: i32, ascending: bool) {
        self.touch();
        if !(0..=1).contains(&kind) {
            return;
        }
        let by_item = kind == 1;
        if by_item == self.sort_by_item && ascending == self.ascending {
            return;
        }
        self.sort_by_item = by_item;
        self.ascending = ascending;
        self.initialized = false;
        self.pending = None;
        self.ended = true;
    }
    fn filter(&mut self, operation: u8, start: i64, end: i64) {
        self.touch();
        let mut previous = None;
        loop {
            // Ascending item order, independent of public sorter. No iterator survives mutation.
            let entry = if let Some(key) = previous {
                self.items.range((Excluded(key), Unbounded)).next()
            } else {
                self.items.first_key_value()
            }
            .map(|(key, value)| (*key, *value));
            let Some((key, value)) = entry else {
                return;
            };
            previous = Some(key);
            let remove = match operation {
                0 => value > start,
                1 => value < start,
                2 => value > start && value < end,
                3 => value == start,
                4 => value <= start,
                5 => value >= start,
                6 => value <= start || value >= end,
                7 => value != start,
                _ => std::process::abort(),
            };
            if remove {
                self.remove(key);
            }
        }
    }
    fn remove_rank(&mut self, top: bool, mut count: i64) {
        self.touch();
        if !self.ascending {
            self.sort(i32::from(self.sort_by_item), true);
            self.remove_rank(!top, count);
            self.sort(i32::from(self.sort_by_item), false);
            return;
        }
        while let Some(key) = self.first(top) {
            // --count runs only for a present element and wraps in the compiled original.
            count = count.wrapping_sub(1);
            if count < 0 {
                return;
            }
            self.remove(key);
        }
    }
    fn keep_rank(&mut self, top: bool, count: i64) {
        self.touch();
        // Allocated map nodes bound the entry count below i64::MAX.
        let size = i64::try_from(self.items.len()).expect("representable list count");
        // Count() - count wraps in the compiled original, including count == i64::MIN.
        self.remove_rank(!top, size.wrapping_sub(count));
    }
    fn copy_contents(&mut self, source: &Self) {
        self.sort(i32::from(source.sort_by_item), source.ascending);
        self.items.clone_from(&source.items);
        self.values.clone_from(&source.values);
    }
    fn add_list(&mut self, source: &Self) {
        if self.items.is_empty() {
            self.items.clone_from(&source.items);
            self.values.clone_from(&source.values);
            self.touch();
        } else {
            for (&key, &value) in &source.items {
                self.add(key, 0);
                self.set(key, value);
            }
        }
    }
    fn remove_list(&mut self, source: &Self) {
        self.touch();
        for &key in source.items.keys() {
            self.remove(key);
        }
    }
    fn keep_list(&mut self, source: &Self) {
        // Original: outer increment, then RemoveList(tmp) increment; tmp contains the difference.
        self.touch();
        self.touch();
        let mut previous = None;
        loop {
            let key = if let Some(key) = previous {
                self.items.range((Excluded(key), Unbounded)).next()
            } else {
                self.items.first_key_value()
            }
            .map(|(key, _)| *key);
            let Some(key) = key else {
                return;
            };
            previous = Some(key);
            if !source.items.contains_key(&key) {
                self.remove(key);
            }
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDListControlInput" ["probe"] => control::Input {
        a,
        b,
        flag,
        kind,
    };
    "OpenTTDListControlAction" ["probe"] => control::Action {
        a,
        kind,
    };
}

#[cfg(test)]
mod tests {
    use super::List;

    #[test]
    fn modification_token_wraps() {
        let mut list = List {
            modifications: i32::MAX,
            ..List::default()
        };
        list.add(1, 0);
        assert_eq!(list.modifications, i32::MIN);
        assert!(list.set(1, 1));
        assert_eq!(list.modifications, i32::MIN + 1);
    }
}

// Opaque exclusive owners, no foreign allocation/VM/layout crosses these calls.
// Pointers must be live, aligned, initialized; operations serialized; output
// scalars valid writable/disjoint. All borrows end at return. Panics abort.
#[allow(unsafe_code)]
mod ffi {
    use super::List;
    use std::ptr;

    #[unsafe(no_mangle)]
    pub(crate) extern "C" fn openttd_rust_list_new() -> *mut List {
        Box::into_raw(Box::new(List::default()))
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_destroy(owner: *mut List) {
        unsafe {
            drop(Box::from_raw(owner));
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_token(owner: *const List) -> i32 {
        unsafe { (*owner).modifications }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_count(owner: *const List) -> i64 {
        // Allocated map nodes bound the entry count below i64::MAX.
        unsafe { i64::try_from((*owner).items.len()).expect("representable list count") }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_get(
        owner: *const List,
        key: i64,
        value: *mut i64,
    ) -> u8 {
        unsafe {
            if let Some(found) = (*owner).items.get(&key) {
                value.write(*found);
                1
            } else {
                0
            }
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_add(owner: *mut List, key: i64, value: i64) {
        unsafe {
            (*owner).add(key, value);
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_remove(owner: *mut List, key: i64) {
        unsafe {
            (*owner).remove(key);
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_set(
        owner: *mut List,
        key: i64,
        value: i64,
    ) -> u8 {
        unsafe { u8::from((*owner).set(key, value)) }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_clear(owner: *mut List) {
        unsafe {
            (*owner).clear();
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_sort(
        owner: *mut List,
        kind: i32,
        ascending: u8,
    ) {
        unsafe {
            (*owner).sort(kind, ascending != 0);
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_iter(
        owner: *mut List,
        operation: u8,
        value: *mut i64,
    ) -> u8 {
        unsafe {
            let list = &mut *owner;
            if operation == 0 {
                value.write(list.begin());
                return 1;
            }
            if !list.initialized {
                value.write(0);
                return 0;
            }
            match operation {
                1 => value.write(list.next()),
                2 => value.write(i64::from(list.items.is_empty() || list.ended)),
                _ => std::process::abort(),
            }
            1
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_filter(
        owner: *mut List,
        operation: u8,
        start: i64,
        end: i64,
    ) {
        unsafe {
            (*owner).filter(operation, start, end);
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_rank(
        owner: *mut List,
        operation: u8,
        count: i64,
    ) {
        unsafe {
            match operation {
                0 => (*owner).remove_rank(true, count),
                1 => (*owner).remove_rank(false, count),
                2 => (*owner).keep_rank(true, count),
                3 => (*owner).keep_rank(false, count),
                _ => std::process::abort(),
            }
        }
    }
    #[unsafe(no_mangle)]
    pub(crate) unsafe extern "C" fn openttd_rust_list_combine(
        owner: *mut List,
        source: *mut List,
        operation: u8,
    ) {
        unsafe {
            if ptr::eq(owner, source) {
                match operation {
                    2 => {
                        (*owner).touch();
                        (*owner).clear();
                    }
                    4 => {
                        (*owner).touch();
                    }
                    0 | 1 | 3 => (),
                    _ => std::process::abort(),
                }
                return;
            }
            let destination = &mut *owner;
            let source = &mut *source;
            match operation {
                0 => destination.add_list(source),
                1 => std::mem::swap(destination, source),
                2 => destination.remove_list(source),
                3 => destination.keep_list(source),
                4 => destination.copy_contents(source),
                _ => std::process::abort(),
            }
        }
    }
}
