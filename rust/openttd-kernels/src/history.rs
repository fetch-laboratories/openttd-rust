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

/// `HasBit(mask, slot)`, whose C++ `uint8_t` parameter narrows the unsigned index.
fn has_bit(mask: u64, slot: u32) -> bool {
    crate::bitmath_func::has_bit64(mask, slot.to_le_bytes()[0])
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
    "HistoryRange" ["probe"] => crate::history_type::HistoryRange {
        hr, periods, records, first, last, division, total_division,
    };
    "OpenTTDRustHistoryQuery" ["probe"] => HistoryQuery { is_valid, in_range };

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

use crate::history_type::{
    HISTORY_MAX_DIVISION, HISTORY_MONTH, HISTORY_QUARTER, HISTORY_RECORDS, HISTORY_YEAR,
    HistoryRange,
};

pub(crate) trait HistoryElement: Copy + Default {
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

/// The scalar fields of `HISTORY_MONTH`, `HISTORY_QUARTER` and `HISTORY_YEAR`, in the
/// subdivision-first order of the original recursion over `HISTORY_YEAR`. Only the
/// mask update and rotation fields are used; the query-only child fields stay zero.
const HISTORY_YEAR_RANGES: [Descriptor; 3] = {
    const fn range(hr: &HistoryRange) -> Descriptor {
        Descriptor {
            child: 0,
            periods: hr.periods,
            records: hr.records,
            first: hr.first,
            last: hr.last,
            division: hr.division,
            total_division: hr.total_division,
            child_periods: 0,
            child_division: 0,
        }
    }
    [
        range(&HISTORY_MONTH),
        range(&HISTORY_QUARTER),
        range(&HISTORY_YEAR),
    ]
};

/// `UpdateValidHistory(valid_history, HISTORY_YEAR, cur_month)`: subdivisions update
/// first, as the original recursion.
pub(crate) fn update_valid_history_year(valid_history: u64, cur_month: u32) -> u64 {
    HISTORY_YEAR_RANGES
        .into_iter()
        .fold(valid_history, |mask, hr| update(mask, hr, cur_month))
}

/// `GetAndResetAccumulatedAverage<uint16_t>`: get an average value for the previous month,
/// as reset for the next month. `total` is the accrued total to average and is reset to zero;
/// `days_since_last_month` is `TimerGameEconomy::days_since_last_month`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ClampTo<uint16_t> narrows the clamped average"
)]
pub(crate) fn get_and_reset_accumulated_average(
    total: &mut u32,
    days_since_last_month: u32,
) -> u16 {
    let result = (*total / days_since_last_month.max(1)).min(u32::from(u16::MAX)) as u16;
    *total = 0;
    result
}

/// `RotateHistory(history, valid_history, HISTORY_YEAR, cur_month)`: subdivisions
/// rotate first, as the original recursion.
pub(crate) fn rotate_history_year<T: HistoryElement>(
    history: &mut [T; HISTORY_RECORDS],
    valid_history: u64,
    cur_month: u32,
) {
    for hr in HISTORY_YEAR_RANGES {
        if !cur_month.is_multiple_of(u32::from(hr.total_division)) {
            continue;
        }
        let (first, last, division) = (
            usize::from(hr.first),
            usize::from(hr.last),
            usize::from(hr.division),
        );
        history.copy_within(first..last - 1, first + 1);
        if hr.total_division == 1 {
            history[first] = history[first - 1];
            history[0] = T::default();
        } else if has_bit(valid_history, u32::from(hr.first - hr.division)) {
            history[first] = T::sum(&history[first - division..first]);
        }
    }
}

// All raw accesses are bounded by the original history/range preconditions. The range
// chain is immutable; history/result may alias, so no history reference spans a store.
#[allow(unsafe_code)]
impl HistoryRange {
    unsafe fn descriptor(&self) -> Descriptor {
        let child = self.hr;
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
            // SAFETY: Aggregation requires the previous subdivision.
            let child = unsafe { self.hr.unwrap_unchecked() };
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
            let mut tmp_result = [T::default(); HISTORY_MAX_DIVISION as usize];
            let month = u32::from(crate::timer_game_common::economy_month());
            let start = start(descriptor, age, month);
            // SAFETY: Aggregation requires the previous subdivision.
            let child = unsafe { self.hr.unwrap_unchecked() };
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

#[cfg(test)]
mod comparison_tests {
    //! Original-only `history-comparison.cpp` observations (#736 U); see the oracle header.
    //! `IsValidHistory` and the industry/town `GetHistory` queries call the production Rust
    //! bodies; the uint16 queries reuse the same generic Rust `get` with the probe's reducer.
    use super::{Descriptor, HistoryElement, HistoryRange, update};
    use crate::industry::Pair;
    use crate::oracle;
    use crate::town::SuppliedHistory;

    const ORACLE: &str = include_str!("history/comparison-oracle.txt");
    const RECORDS: usize = crate::history_type::HISTORY_RECORDS;
    type Sections<'a> = std::collections::HashMap<&'a str, oracle::Section<'a>>;

    /// The probe's `SumHistory<uint16_t>`: `ClampTo<uint16_t>` of the plain total.
    impl HistoryElement for u16 {
        fn sum(history: &[Self]) -> Self {
            let total: u32 = history.iter().map(|&v| u32::from(v)).sum();
            u16::try_from(total).unwrap_or(u16::MAX)
        }
    }

    /// The production `HistoryRange` constructors, chained to their parent range.
    /// Each range lives for the rest of the test process, as the original's static ranges do.
    fn initial(periods: u8) -> &'static HistoryRange {
        Box::leak(Box::new(HistoryRange::new(periods)))
    }

    fn sub(hr: &'static HistoryRange, division: u8, periods: u8) -> &'static HistoryRange {
        Box::leak(Box::new(HistoryRange::subdividing(hr, division, periods)))
    }

    /// The test's only unsafe calls: the production chain queries, over range chains that
    /// each test keeps live for its whole run.
    struct Live;

    #[allow(unsafe_code)]
    impl Live {
        fn child(hr: &HistoryRange) -> Option<&HistoryRange> {
            hr.hr
        }

        fn descriptor(hr: &HistoryRange) -> Descriptor {
            // SAFETY: Live chain for the descriptor read.
            unsafe { hr.descriptor() }
        }

        fn is_valid(hr: &HistoryRange, mask: u64, age: u32) -> bool {
            // SAFETY: Every chain built here stays live for the test.
            unsafe { hr.is_valid(mask, age) }
        }

        fn get<T: HistoryElement>(
            history: &[T; RECORDS],
            mask: u64,
            hr: &HistoryRange,
            age: u32,
            result: &mut T,
        ) -> Option<bool> {
            // SAFETY: Live chain, initialized array and a distinct writable result.
            unsafe { hr.get(history.as_ptr(), mask, age, result) }
        }
    }

    /// Subdivisions update first, as the original `UpdateValidHistory` recursion.
    fn update_chain(mask: u64, hr: &HistoryRange, month: u32) -> u64 {
        let mask = Live::child(hr).map_or(mask, |child| update_chain(mask, child, month));
        update(mask, Live::descriptor(hr), month)
    }

    /// The original `RotateHistory` template (unchanged C++), with the production reducers.
    fn rotate<T: HistoryElement>(
        history: &mut [T; RECORDS],
        mask: u64,
        hr: &HistoryRange,
        month: u32,
    ) {
        if let Some(child) = Live::child(hr) {
            rotate(history, mask, child, month);
        }
        if !month.is_multiple_of(u32::from(hr.total_division)) {
            return;
        }
        let (first, last, division) = (
            usize::from(hr.first),
            usize::from(hr.last),
            usize::from(hr.division),
        );
        history.copy_within(first..last - 1, first + 1);
        if hr.total_division == 1 {
            history[first] = history[first - 1];
            history[0] = T::default();
        } else if super::has_bit(mask, u32::from(hr.first - hr.division)) {
            history[first] = T::sum(&history[first - division..first]);
        }
    }

    fn set_month(month: u32) {
        crate::timer_game_common::write_economy_month(u8::try_from(month).unwrap());
    }

    /// One test: every part sets the process-wide economy month its queries read.
    #[test]
    fn history_queries_match_original() {
        let o = oracle::parse(ORACLE);
        let month_range = initial(24);
        let quarter = sub(month_range, 3, 24);
        let year = sub(quarter, 4, 24);
        scalars(&o);
        production(&o, year);
        fillers(&o, quarter);
    }

    /// The probe's seven ages: `IsValidHistory`, then `GetHistory` or its fatal.
    fn query_ages(
        history: &[u16; RECORDS],
        mask: u64,
        hr: &HistoryRange,
        inputs: &str,
    ) -> Vec<(String, String)> {
        let periods = u32::from(hr.periods);
        let ages = [
            0,
            periods / 2,
            periods - 1,
            periods,
            periods + 1,
            u32::MAX,
            0x4000_0000,
        ];
        ages.into_iter()
            .map(|age| {
                let first = u8::from(Live::is_valid(hr, mask, age));
                let mut result = 999_u16;
                let output = match Live::get(history, mask, hr, age, &mut result) {
                    Some(valid) => format!("{first} {} {result}", u8::from(valid)),
                    None => format!("{first} fatal {result}"),
                };
                (format!("{inputs} age {age}"), output)
            })
            .collect()
    }

    fn scalars(o: &Sections<'_>) {
        let month_range = initial(24);
        let quarter = sub(month_range, 3, 24);
        let year = sub(quarter, 4, 24);
        let wide = initial(40);
        let small = initial(2);
        let middle = sub(small, 2, 4);
        let upper = sub(middle, 2, 5);
        let fourth = sub(upper, 1, 5);
        let one = initial(4);
        let two = sub(one, 1, 4);
        let three = sub(two, 1, 4);
        let four = sub(three, 1, 4);
        let five = sub(four, 1, 4);
        let ranges: [&HistoryRange; 9] = [
            month_range,
            quarter,
            year,
            wide,
            small,
            middle,
            fourth,
            five,
            upper,
        ];
        let masks = [
            0,
            1,
            2,
            4,
            8,
            0x00AA_AAAA,
            1 << 24,
            1 << 41,
            (1 << 35) | (1 << 63),
            u64::MAX,
        ];

        let mut history = [0_u16; RECORDS];
        for (i, value) in history.iter_mut().enumerate() {
            *value = u16::try_from(i * 13 + 7).unwrap();
        }
        let (mut updates, mut queries) = (Vec::new(), Vec::new());
        // Probe order: range, then month, then mask.
        for case in 0..ranges.len() * 12 * masks.len() {
            let (index, mask_index) = (case / (12 * masks.len()), case % masks.len());
            let month = u32::try_from(case / masks.len() % 12).unwrap();
            let (hr, mask) = (ranges[index], masks[mask_index]);
            set_month((month + 5) % 12);
            let inputs = format!("range {index} month {month} mask {mask_index}");
            updates.push((inputs.clone(), update_chain(mask, hr, month).to_string()));
            queries.extend(query_ages(&history, mask, hr, &inputs));
        }
        oracle::check(o, "update", updates);
        oracle::check(o, "query", queries);

        // Multiplication wraps before the age guard, yielding a defined in-range query.
        let mut plain = [0_u16; RECORDS];
        for (i, value) in plain.iter_mut().enumerate() {
            *value = u16::try_from(i).unwrap();
        }
        let wrapped = (0..12).map(|month| {
            set_month(month);
            let mut result = 999_u16;
            let valid = Live::get(&plain, u64::MAX, quarter, 0x5555_5556, &mut result);
            let output = valid.map_or_else(
                || "fatal".to_owned(),
                |v| format!("{} {result}", u8::from(v)),
            );
            (format!("month {month}"), output)
        });
        oracle::check(o, "wrapped-quarter", wrapped.collect::<Vec<_>>());
    }

    fn production(o: &Sections<'_>, year: &HistoryRange) {
        let mut produced = [Pair::default(); RECORDS];
        let mut accepted = [Pair::default(); RECORDS];
        let mut supplied = [SuppliedHistory::default(); RECORDS];
        for i in 0..RECORDS {
            let n = u32::try_from(i).unwrap();
            produced[i] = Pair {
                first: u16::try_from(n * 37).unwrap(),
                second: u16::try_from(n * 13).unwrap(),
            };
            accepted[i] = Pair {
                first: u16::try_from(n * 11).unwrap(),
                second: u16::try_from(n * 17).unwrap(),
            };
            supplied[i] = SuppliedHistory {
                production: u32::MAX - n * 97,
                transported: 0x8000_0000 + n * 13,
            };
        }
        let (mut production, mut rotated) = (Vec::new(), Vec::new());
        for month in 0..12 {
            set_month(month);
            for age in [0, 1, 4, 5, 23] {
                let (mut p, mut a, mut t) =
                    (Pair::default(), Pair::default(), SuppliedHistory::default());
                let vp = u8::from(Live::get(&produced, 4, year, age, &mut p).expect("in range"));
                let va = u8::from(Live::get(&accepted, 4, year, age, &mut a).expect("in range"));
                let vt = u8::from(Live::get(&supplied, 4, year, age, &mut t).expect("in range"));
                production.push((
                    format!("month {month} age {age}"),
                    format!(
                        "{vp} {} {} {va} {} {} {vt} {} {}",
                        p.first, p.second, a.first, a.second, t.production, t.transported
                    ),
                ));
            }
            let (mut p, mut a, mut t) = (produced, accepted, supplied);
            rotate(&mut p, u64::MAX, year, month);
            rotate(&mut a, u64::MAX, year, month);
            rotate(&mut t, u64::MAX, year, month);
            let fields: Vec<String> = [0, 1, 24, 25, 41, 42, 60]
                .iter()
                .map(|&i| {
                    let (p, a, t) = (p[i], a[i], t[i]);
                    format!(
                        "{} {} {} {} {} {}",
                        p.first, p.second, a.first, a.second, t.production, t.transported
                    )
                })
                .collect();
            rotated.push((format!("month {month}"), fields.join(" ")));
        }
        oracle::check(o, "production", production);
        oracle::check(o, "production-rotate", rotated);

        // Nested quarter rounding versus one flattened yearly reduction.
        let mut grouped = [Pair::default(); RECORDS];
        for (i, value) in [0, 0, 2, 0, 0, 2, 0, 0, 2, 2, 2, 2].into_iter().enumerate() {
            grouped[i + 1].first = value;
        }
        set_month(0);
        let mut nested = Pair::default();
        Live::get(&grouped, u64::MAX, year, 0, &mut nested).expect("in range");
        let flattened = crate::industry::policy::sum_history(&grouped[1..13]);
        let grouping = format!("{} {}", nested.first, flattened.first);
        oracle::check(o, "grouping", [("nested year 0".to_owned(), grouping)]);
    }

    /// One `FillFromHistory<6>` call: both fillers see each age's event in turn.
    fn fill_events(
        filled: &[u16; RECORDS],
        mask: u64,
        quarter: &HistoryRange,
        present: bool,
    ) -> String {
        let mut result = 0_u16;
        let mut events = Vec::new();
        for i in 0..6_u32 {
            let age = 5 - i;
            let event = match (
                present,
                present && Live::get(filled, mask, quarter, age, &mut result).expect("in range"),
            ) {
                (true, true) => format!("F:{result}"),
                (false, _) if Live::is_valid(quarter, mask, age) => "Z".to_owned(),
                _ => "I".to_owned(),
            };
            events.push(format!("1:{i}:{event}"));
            events.push(format!("2:{i}:{event}"));
        }
        events.join(" ")
    }

    /// `FillFromHistory<6>` over `HISTORY_QUARTER` with two fillers, present or absent.
    fn fillers(o: &Sections<'_>, quarter: &HistoryRange) {
        let mut filled = [0_u16; RECORDS];
        for (i, value) in filled.iter_mut().enumerate() {
            *value = u16::try_from(i * 7).unwrap();
        }
        let mut fillers = Vec::new();
        // Probe order: month, then mask, then absent before present.
        for case in 0..72_u32 {
            let (month, present) = (case / 6, case % 2 == 1);
            let mask = [0, 4, u64::MAX][usize::try_from(case / 2 % 3).unwrap()];
            set_month(month);
            let inputs = format!("month {month} mask {mask} present {present}");
            fillers.push((inputs, fill_events(&filled, mask, quarter, present)));
        }
        oracle::check(o, "fillers", fillers);
    }
}
