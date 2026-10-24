/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared time constants from `timer_game_common.h`.
/// `CalendarTime::MONTHS_IN_YEAR`.
pub const MONTHS_IN_YEAR: u32 = 12;
/// `TimerGameConst::DAYS_IN_YEAR`.
pub const DAYS_IN_YEAR: i32 = 365;
/// `TimerGameConst::DAYS_IN_LEAP_YEAR`.
pub const DAYS_IN_LEAP_YEAR: i32 = 366;

/// `EconomyTime::SECONDS_PER_DAY`.
pub const SECONDS_PER_DAY: i32 = 2;
/// `TimerGameConst::INVALID_DATE`, inherited by `EconomyTime`/`CalendarTime`.
pub const INVALID_DATE: i32 = -1;

/// `EconomyTime::ORIGINAL_BASE_YEAR` / `CalendarTime::ORIGINAL_BASE_YEAR`.
pub const ORIGINAL_BASE_YEAR: i32 = 1920;
pub const ORIGINAL_MAX_YEAR: i32 = 2090;
/// `TimerGameConst::DAYS_TILL_ORIGINAL_BASE_YEAR` (Gregorian date at 1920).
pub const DAYS_TILL_ORIGINAL_BASE_YEAR: i32 = 701_265;

/// `TimerGameConst::IsLeapYear` (the same signed remainder rules).
pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
