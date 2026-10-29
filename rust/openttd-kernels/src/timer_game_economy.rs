/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical economy state, wallclock conversion and daily dispatch.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::wildcard_imports,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use crate::services::{CalendarYmd, EconomyDate};
use crate::timer_game_common::{self as common, *};
use crate::timer_game_tick::DAY_TICKS;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EconomyYear(pub i32);
pub const DAYS_IN_ECONOMY_YEAR: i32 = 360;
pub const DAYS_IN_ECONOMY_MONTH: i32 = 30;

pub fn date_to_ymd(date: i32, wallclock: bool) -> CalendarYmd {
    if !wallclock {
        return calendar_date_to_ymd(date);
    }
    CalendarYmd {
        year: date / DAYS_IN_ECONOMY_YEAR,
        month: ((date % DAYS_IN_ECONOMY_YEAR) / DAYS_IN_ECONOMY_MONTH) as u8,
        day: ((date % DAYS_IN_ECONOMY_MONTH) + 1) as u8,
    }
}
pub fn ymd_to_date(year: i32, month: u8, day: u8, wallclock: bool) -> i32 {
    if !wallclock {
        return calendar_ymd_to_date(year, month, day);
    }
    year.wrapping_mul(MONTHS_IN_YEAR as i32)
        .wrapping_add(i32::from(month))
        .wrapping_mul(DAYS_IN_ECONOMY_MONTH)
        .wrapping_add(i32::from(day))
        .wrapping_sub(1)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_date_to_ymd(date: i32, wallclock: bool) -> CalendarYmd {
    date_to_ymd(date, wallclock)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_ymd_to_date(
    year: i32,
    month: u8,
    day: u8,
    wallclock: bool,
) -> i32 {
    ymd_to_date(year, month, day, wallclock)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_set_date(date: i32, fraction: u16, wallclock: bool) {
    write_economy_date(EconomyDate(date));
    write_economy_fraction(fraction);
    let ymd = date_to_ymd(date, wallclock);
    write_economy_year(EconomyYear(ymd.year));
    write_economy_month(ymd.month);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_economy_elapsed(
    services: &ClockServices,
    console_exception: *mut ConsoleException,
) -> bool {
    if crate::session::mode() == crate::openttd::GameMode::MENU {
        return false;
    }
    let fraction = economy_fraction().wrapping_add(1);
    write_economy_fraction(fraction);
    if fraction < DAY_TICKS as u16 {
        return true;
    }
    write_economy_fraction(0);
    write_economy_date(EconomyDate(economy_date().0.wrapping_add(1)));
    write_days_since_last_month(days_since_last_month().wrapping_add(1));
    let ymd = date_to_ymd(economy_date().0, (services.using_wallclock_units)(false));
    let new_month = ymd.month != economy_month();
    let new_year = ymd.year != economy_year().0;
    write_economy_month(ymd.month);
    write_economy_year(EconomyYear(ymd.year));
    let timers = common::economy_snapshot();
    if !unsafe { dispatch(&timers, DAY, console_exception) } {
        return false;
    }
    if economy_date().0 % 7 == 3 && !unsafe { dispatch(&timers, WEEK, console_exception) } {
        return false;
    }
    if new_month {
        if !unsafe { dispatch(&timers, MONTH, console_exception) } {
            return false;
        }
        if economy_month().is_multiple_of(3)
            && !unsafe { dispatch(&timers, QUARTER, console_exception) }
        {
            return false;
        }
    }
    if new_year && !unsafe { dispatch(&timers, YEAR, console_exception) } {
        return false;
    }
    if new_month {
        write_days_since_last_month(0);
    }
    if economy_year().0 == MAX_YEAR + 1 {
        let year = economy_year().0.wrapping_sub(1);
        write_economy_year(EconomyYear(year));
        // timer_game_economy.cpp uses Gregorian days even in wallclock mode.
        let days = if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        };
        write_economy_date(EconomyDate(economy_date().0.wrapping_sub(days)));
        let shared = unsafe { &*services.shared };
        let mut from = 0;
        while let Some(vehicle) = (shared.vehicle_next)(from) {
            from = (services.vehicle_id)(vehicle.0.as_ptr().cast()).wrapping_add(1);
            (services.vehicle_shift_dates)(vehicle, -days);
        }
        let mut from = 0;
        loop {
            let graph = (services.next_graph)(&raw mut from);
            if graph.is_null() {
                break;
            }
            unsafe {
                crate::linkgraph::live::openttd_rust_live_graph_shift(
                    (services.graph_owner)(graph),
                    -days,
                );
            }
        }
    }
    true
}
