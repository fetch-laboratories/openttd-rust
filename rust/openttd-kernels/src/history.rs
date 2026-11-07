/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Direct history operations on canonical town/industry records and immutable native ranges.

/// Scalar range fields shared by monthly mask updates and immutable query arithmetic.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Descriptor {
    /// Previous subdivision present when nonzero.
    pub child: usize,
    /// Number of reportable periods.
    pub periods: u8,
    /// Number of stored records.
    pub records: u8,
    /// First stored index.
    pub first: u8,
    /// Exclusive last stored index.
    pub last: u8,
    /// Number of child records in one period.
    pub division: u8,
    /// Number of initial periods in one period.
    pub total_division: u8,
    /// Child period count, zero when absent.
    pub child_periods: u8,
    /// Child division, zero when absent.
    pub child_division: u8,
}

fn has_bit(mask: u64, slot: u32) -> bool {
    // HasBit's C++ uint8 parameter narrows the unsigned index before shifting.
    let slot = slot.to_le_bytes()[0];
    mask & (1_u64 << slot) != 0
}

fn aggregate(d: Descriptor, age: u32) -> bool {
    // C++ first subtracts promoted signed ints, then converts to native uint.
    let threshold = (i32::from(d.child_periods) - i32::from(d.division)).to_ne_bytes();
    d.child != 0 && age.wrapping_mul(u32::from(d.division)) < u32::from_ne_bytes(threshold)
}

fn start(d: Descriptor, age: u32, month: u32) -> u32 {
    age.wrapping_mul(u32::from(d.division))
        .wrapping_add((month / u32::from(d.child_division)) % u32::from(d.division))
}

fn slot(d: Descriptor, age: u32) -> u32 {
    let offset = if d.child == 0 {
        0
    } else {
        (u32::from(d.child_periods) / u32::from(d.division)).wrapping_sub(1)
    };
    u32::from(d.first).wrapping_add(age).wrapping_sub(offset)
}

pub(crate) fn update(mask: u64, d: Descriptor, month: u32) -> u64 {
    if has_bit(mask, u32::from(d.last).wrapping_sub(1))
        || !month.is_multiple_of(u32::from(d.total_division))
        || (d.division != 1
            && !has_bit(mask, u32::from(d.first).wrapping_sub(u32::from(d.division))))
    {
        return mask;
    }
    let window = (1_u64 << d.records) - 1;
    // GB returns uint (32 bits) even when its input is uint64; retain truncation.
    let extracted = (mask >> d.first) & window;
    let low = extracted.to_le_bytes();
    let extracted = u32::from_le_bytes([low[0], low[1], low[2], low[3]]);
    let shifted = (u64::from(extracted) << 1) | 1;
    // SB clears the window, then ORs the supplied bits without a second mask.
    (mask & !(window << d.first)) | (shifted << d.first)
}

crate::abi::layouts! {
    fn abi_layout;
    "HistoryRange" ["probe"] => HistoryRange {
        hr, periods, records, first, last, division, total_division,
    };
    "OpenTTDRustHistoryQuery" ["probe"] => HistoryQuery { is_valid, in_range };

}

/// Native immutable `HistoryRange` chain from `misc/history_type.hpp`.
#[repr(C)]
pub struct HistoryRange {
    /// Previous subdivision, or null for the initial range.
    pub hr: *const Self,
    /// Number of periods for this range.
    pub periods: u8,
    /// Number of records needed for this range.
    pub records: u8,
    /// Index of first element in history data.
    pub first: u8,
    /// Index past the last element in history data.
    pub last: u8,
    /// Number of divisions of the previous history range.
    pub division: u8,
    /// Number of divisions of the initial history range.
    pub total_division: u8,
}

/// Native fatal handling and data validity are separate query outcomes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HistoryQuery {
    /// Whether any queried child has valid data.
    pub is_valid: bool,
    /// Whether the original query returns rather than reaching `NOT_REACHED`.
    pub in_range: bool,
}

impl From<Option<bool>> for HistoryQuery {
    fn from(value: Option<bool>) -> Self {
        Self {
            is_valid: value.unwrap_or(false),
            in_range: value.is_some(),
        }
    }
}

// Maximum subdivision among the original month, quarter and year ranges.
const HISTORY_MAX_DIVISION: usize = 4;

trait HistoryElement: Copy + Default {
    fn sum(history: &[Self]) -> Self;
}

impl HistoryElement for crate::industry::Pair {
    fn sum(history: &[Self]) -> Self {
        crate::industry::policy::sum_history(history)
    }
}

impl HistoryElement for crate::town::SuppliedHistory {
    fn sum(history: &[Self]) -> Self {
        crate::town::sum_history(history)
    }
}

// All raw accesses are bounded by the original history/range preconditions. The range
// chain is immutable; history/result may alias, so no history reference spans a store.
#[allow(unsafe_code)]
impl HistoryRange {
    unsafe fn descriptor(&self) -> Descriptor {
        // SAFETY: The native caller keeps the immutable range chain live.
        let child = unsafe { self.hr.as_ref() };
        Descriptor {
            child: usize::from(child.is_some()),
            periods: self.periods,
            records: self.records,
            first: self.first,
            last: self.last,
            division: self.division,
            total_division: self.total_division,
            child_periods: child.map_or(0, |hr| hr.periods),
            child_division: child.map_or(0, |hr| hr.division),
        }
    }

    unsafe fn is_valid(&self, mask: u64, age: u32) -> bool {
        // SAFETY: The immutable native range chain is live.
        let descriptor = unsafe { self.descriptor() };
        if aggregate(descriptor, age) {
            // SAFETY: Aggregation requires the live previous subdivision.
            let child = unsafe { &*self.hr };
            let month = u32::from(crate::timer_game_common::economy_month());
            // SAFETY: The child retains the enclosing chain's lifetime/preconditions.
            return unsafe { child.is_valid(mask, start(descriptor, age, month)) };
        }
        age < u32::from(self.periods) && has_bit(mask, slot(descriptor, age))
    }

    unsafe fn get<T: HistoryElement>(
        &self,
        history: *const T,
        mask: u64,
        age: u32,
        result: *mut T,
    ) -> Option<bool> {
        // SAFETY: The immutable native range chain is live.
        let descriptor = unsafe { self.descriptor() };
        if aggregate(descriptor, age) {
            let mut is_valid = false;
            // No need to clear as we fill every element we use. These concrete types
            // default-construct all four elements without callbacks or side effects.
            let mut tmp_result = [T::default(); HISTORY_MAX_DIVISION];
            let month = u32::from(crate::timer_game_common::economy_month());
            let start = start(descriptor, age, month);
            // SAFETY: Aggregation requires the live previous subdivision.
            let child = unsafe { &*self.hr };
            for i in 0..self.division {
                // SAFETY: Each scratch target is distinct; original child ages and slots apply.
                is_valid |= unsafe {
                    child.get(
                        history,
                        mask,
                        start.wrapping_add(u32::from(i)),
                        &raw mut tmp_result[usize::from(i)],
                    )
                }?;
            }
            let sum = T::sum(&tmp_result[..usize::from(self.division)]);
            // SAFETY: Scratch reads have ended; result can alias any live history element.
            unsafe { result.write(sum) };
            return Some(is_valid);
        }
        if age < u32::from(self.periods) {
            let index = slot(descriptor, age) as usize;
            // SAFETY: The original range/age select a live initialized history slot.
            let source = unsafe { history.add(index) };
            // SAFETY: The selected source is an initialized history element.
            let source = unsafe { source.read() };
            // SAFETY: Read completes before the potentially aliasing result write.
            unsafe { result.write(source) };
            return Some(has_bit(mask, slot(descriptor, age)));
        }
        None
    }
}

/// Test first-child validity without extracting data.
///
/// # Safety
/// `hr` meets the immutable range-chain and original index/divisor/shift contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_is_valid(
    mask: u64,
    hr: *const HistoryRange,
    age: u32,
) -> u8 {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the original range and bit-index preconditions.
    u8::from(unsafe { hr.is_valid(mask, age) })
}

/// Query industry produced/accepted history, preserving nested reduction rounding.
///
/// # Safety
/// `history` is the original initialized array, `result` a writable canonical pair
/// (possibly an array element); `hr` meets the immutable range-chain contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_get_industry(
    history: *const crate::industry::Pair,
    mask: u64,
    hr: *const HistoryRange,
    age: u32,
    result: *mut crate::industry::Pair,
) -> HistoryQuery {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the initialized array/result and original range preconditions.
    unsafe { hr.get(history, mask, age, result) }.into()
}

/// Query town supplied history, preserving nested reduction and aliasing order.
///
/// # Safety
/// `history` is the original initialized array, `result` a writable canonical element
/// (possibly an array element); `hr` meets the immutable range-chain contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_get_town(
    history: *const crate::town::SuppliedHistory,
    mask: u64,
    hr: *const HistoryRange,
    age: u32,
    result: *mut crate::town::SuppliedHistory,
) -> HistoryQuery {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the initialized array/result and original range preconditions.
    unsafe { hr.get(history, mask, age, result) }.into()
}
