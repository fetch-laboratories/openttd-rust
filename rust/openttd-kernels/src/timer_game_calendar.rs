/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical calendar state and advancement. Native aliases use the same stable storage.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::wildcard_imports,
    clippy::cast_possible_truncation
)]
use crate::timer_game_common::{self as common, *};
use crate::timer_game_tick::DAY_TICKS;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CalendarYear(pub i32);
pub const DEF_MINUTES_PER_YEAR: u32 = 12;
pub const FROZEN_MINUTES_PER_YEAR: u32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_calendar_set_date(date: i32, fraction: u16) {
    write_calendar_date(crate::services::CalendarDate(date));
    write_calendar_fraction(fraction);
    let ymd = calendar_date_to_ymd(date);
    write_calendar_year(CalendarYear(ymd.year));
    write_calendar_month(ymd.month);
}

/// All field accesses end before native services and dispatch, which may change either clock.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_calendar_elapsed(
    services: &ClockServices,
    console_exception: *mut ConsoleException,
) -> bool {
    if (services.game_mode)() == crate::openttd::GameMode::MENU {
        return false;
    }
    let minutes = u32::from((services.minutes_per_calendar_year)());
    if minutes == FROZEN_MINUTES_PER_YEAR {
        return false;
    }
    if minutes != DEF_MINUTES_PER_YEAR {
        let sub = calendar_subfraction().wrapping_add(DAY_TICKS as u16);
        write_calendar_subfraction(sub);
        let threshold = (minutes.wrapping_mul(DAY_TICKS as u32) / DEF_MINUTES_PER_YEAR) as u16;
        if sub < threshold {
            return false;
        }
        write_calendar_subfraction(sub.wrapping_sub(threshold).min(DAY_TICKS as u16 - 1));
    }
    let fraction = calendar_fraction().wrapping_add(1);
    write_calendar_fraction(fraction);
    if fraction < DAY_TICKS as u16 {
        return true;
    }
    write_calendar_fraction(0);
    write_calendar_subfraction(0);
    let date = calendar_date().0.wrapping_add(1);
    write_calendar_date(crate::services::CalendarDate(date));
    let ymd = calendar_date_to_ymd(date);
    let new_month = ymd.month != calendar_month();
    let new_year = ymd.year != calendar_year().0;
    write_calendar_month(ymd.month);
    write_calendar_year(CalendarYear(ymd.year));
    let timers = common::calendar_snapshot();
    for (trigger, enabled) in [(DAY, true), (MONTH, new_month), (YEAR, new_year)] {
        if enabled && !unsafe { dispatch(&timers, trigger, console_exception) } {
            return false;
        }
    }
    if calendar_year().0 == MAX_YEAR + 1 {
        let year = calendar_year().0.wrapping_sub(1);
        write_calendar_year(CalendarYear(year));
        let days = if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        };
        write_calendar_date(crate::services::CalendarDate(
            calendar_date().0.wrapping_sub(days),
        ));
    }
    true
}
