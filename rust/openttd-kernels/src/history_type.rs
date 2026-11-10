/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared history indices from `misc/history_type.hpp`.

/// Native immutable `HistoryRange` chain from `misc/history_type.hpp`.
#[repr(C)]
pub struct HistoryRange {
    /// Previous subdivision, or null for the initial range.
    pub hr: Option<&'static Self>,
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
impl HistoryRange {
    /// `HistoryRange(periods)`: the initial range.
    #[must_use]
    pub const fn new(periods: u8) -> Self {
        Self {
            hr: None,
            periods,
            records: periods,
            first: 1,
            last: 1_u8.wrapping_add(periods),
            division: 1,
            total_division: 1,
        }
    }
    /// `HistoryRange(hr, division, periods)`: a range subdividing `hr`, including the
    /// original's `uint8_t` narrowing. The range keeps `&hr`, so `hr` must outlive it.
    #[must_use]
    pub const fn subdividing(hr: &'static Self, division: u8, periods: u8) -> Self {
        let records = periods.wrapping_sub((hr.periods / division).wrapping_sub(1));
        Self {
            hr: Some(hr),
            periods,
            records,
            first: hr.last,
            last: hr.last.wrapping_add(records),
            division,
            total_division: division.wrapping_mul(hr.total_division),
        }
    }
}

/// Native `HISTORY_PERIODS`: the number of periods each history range reports.
pub const HISTORY_PERIODS: u8 = 24;
/// Native `HISTORY_MONTH`.
pub static HISTORY_MONTH: HistoryRange = HistoryRange::new(HISTORY_PERIODS);
/// Native `HISTORY_QUARTER`.
pub static HISTORY_QUARTER: HistoryRange =
    HistoryRange::subdividing(&HISTORY_MONTH, 3, HISTORY_PERIODS);
/// Native `HISTORY_YEAR`.
pub static HISTORY_YEAR: HistoryRange =
    HistoryRange::subdividing(&HISTORY_QUARTER, 4, HISTORY_PERIODS);
/// Native `HISTORY_MAX_DIVISION`: maximum number of divisions from previous history range.
pub const HISTORY_MAX_DIVISION: u8 = {
    let a = HISTORY_MONTH.division;
    let b = HISTORY_QUARTER.division;
    let c = HISTORY_YEAR.division;
    let ab = if a > b { a } else { b };
    if ab > c { ab } else { c }
};
/// Native `HISTORY_RECORDS`: total number of records require for all history data.
pub const HISTORY_RECORDS: usize = HISTORY_YEAR.last as usize;

/// Native `THIS_MONTH`: the history record being accumulated.
pub const THIS_MONTH: u8 = 0;
/// Native `LAST_MONTH`: the most recently completed month.
pub const LAST_MONTH: u8 = 1;
