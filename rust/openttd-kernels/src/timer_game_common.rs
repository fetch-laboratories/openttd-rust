/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared Gregorian conversion, canonical clock storage and subscription dispatch.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
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

/// `CalendarTime::MAX_DATE` (last day of year 5,000,000).
pub const MAX_DATE: i32 = 365 * 5_000_001 + 5_000_000 / 4 - 5_000_000 / 100 + 5_000_000 / 400;

/// Calendar year limits from `TimerGameConst`.
pub const MIN_YEAR: i32 = 0;
pub const MAX_YEAR: i32 = 5_000_000;

// Native StrongType subobjects are placement-constructed once before publishing aliases.
// Storage lives through native static timer destruction. Access is serialized by the
// game-state mutex; raw field accesses end before any callback can reenter either owner.
#[repr(C)]
pub struct ClockState {
    pub calendar_year: crate::timer_game_calendar::CalendarYear,
    pub calendar_date: crate::services::CalendarDate,
    pub economy_year: crate::timer_game_economy::EconomyYear,
    pub economy_date: crate::services::EconomyDate,
    pub days_since_last_month: u32,
    pub calendar_fraction: u16,
    pub calendar_subfraction: u16,
    pub economy_fraction: u16,
    pub calendar_month: u8,
    pub economy_month: u8,
}
static CLOCK: crate::game_state::GameState<std::mem::MaybeUninit<ClockState>> =
    crate::game_state::GameState::new(std::mem::MaybeUninit::zeroed());
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_clock_storage() -> *mut ClockState {
    CLOCK.get().cast()
}
macro_rules! clock_field {
    ($read:ident, $write:ident, $field:ident, $ty:ty) => {
        pub fn $read() -> $ty {
            unsafe { std::ptr::addr_of!((*CLOCK.get().cast::<ClockState>()).$field).read() }
        }
        pub fn $write(value: $ty) {
            unsafe {
                std::ptr::addr_of_mut!((*CLOCK.get().cast::<ClockState>()).$field).write(value);
            }
        }
    };
}
clock_field!(
    calendar_year,
    write_calendar_year,
    calendar_year,
    crate::timer_game_calendar::CalendarYear
);
clock_field!(
    calendar_date,
    write_calendar_date,
    calendar_date,
    crate::services::CalendarDate
);
clock_field!(
    economy_year,
    write_economy_year,
    economy_year,
    crate::timer_game_economy::EconomyYear
);
clock_field!(
    economy_date,
    write_economy_date,
    economy_date,
    crate::services::EconomyDate
);
clock_field!(
    days_since_last_month,
    write_days_since_last_month,
    days_since_last_month,
    u32
);
clock_field!(
    calendar_fraction,
    write_calendar_fraction,
    calendar_fraction,
    u16
);
clock_field!(
    calendar_subfraction,
    write_calendar_subfraction,
    calendar_subfraction,
    u16
);
clock_field!(
    economy_fraction,
    write_economy_fraction,
    economy_fraction,
    u16
);
clock_field!(calendar_month, write_calendar_month, calendar_month, u8);
clock_field!(economy_month, write_economy_month, economy_month, u8);

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_clock_calendar_date() -> i32 {
    calendar_date().0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_clock_calendar_year() -> i32 {
    calendar_year().0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_clock_economy_date() -> i32 {
    economy_date().0
}
const ACCUM_DAYS: [i32; 12] = [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335];
const MONTH_DATE: [u16; 366] = {
    let mut table = [0; 366];
    let mut index = 0;
    let mut month = 0;
    while month < 12 {
        let end = if month == 11 {
            366
        } else {
            ACCUM_DAYS[month + 1]
        };
        let mut day = 1;
        while index < end as usize {
            table[index] = ((month as u16) << 5) | day;
            day += 1;
            index += 1;
        }
        month += 1;
    }
    table
};
/// `TimerGame::DateAtStartOfYear`, including signed division and integer width.
pub const fn date_at_start_of_year(year: i32) -> i32 {
    let prev = year.wrapping_sub(1);
    let leap_years = if year == 0 {
        0
    } else {
        (prev / 4)
            .wrapping_sub(prev / 100)
            .wrapping_add(prev / 400)
            .wrapping_add(1)
    };
    year.wrapping_mul(DAYS_IN_YEAR).wrapping_add(leap_years)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_calendar_date_to_ymd(date: i32) -> crate::services::CalendarYmd {
    calendar_date_to_ymd(date)
}
pub fn calendar_date_to_ymd(date: i32) -> crate::services::CalendarYmd {
    let mut year = 400 * (date / (DAYS_IN_YEAR * 400 + 97));
    let mut rem = date % (DAYS_IN_YEAR * 400 + 97);
    if rem >= DAYS_IN_YEAR * 100 + 25 {
        year += 100;
        rem -= DAYS_IN_YEAR * 100 + 25;
        year += 100 * (rem / (DAYS_IN_YEAR * 100 + 24));
        rem %= DAYS_IN_YEAR * 100 + 24;
    }
    if !is_leap_year(year) && rem >= DAYS_IN_YEAR * 4 {
        year += 4;
        rem -= DAYS_IN_YEAR * 4;
    }
    year += 4 * (rem / (DAYS_IN_YEAR * 4 + 1));
    rem %= DAYS_IN_YEAR * 4 + 1;
    while rem
        >= if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        }
    {
        rem -= if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        };
        year += 1;
    }
    if !is_leap_year(year) && rem >= ACCUM_DAYS[2] - 1 {
        rem += 1;
    }
    // timer_game_common.cpp requires a Gregorian date whose year-day indexes its table.
    let x = unsafe { *MONTH_DATE.get_unchecked(rem as usize) };
    crate::services::CalendarYmd {
        year,
        month: (x >> 5) as u8,
        day: (x & 0x1f) as u8,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_calendar_ymd_to_date(year: i32, month: u8, day: u8) -> i32 {
    calendar_ymd_to_date(year, month, day)
}
pub fn calendar_ymd_to_date(year: i32, month: u8, day: u8) -> i32 {
    // Native callers supply a month indexing _accum_days_for_month.
    let mut days = unsafe { *ACCUM_DAYS.get_unchecked(usize::from(month)) } + i32::from(day) - 1;
    if !is_leap_year(year) && days >= ACCUM_DAYS[2] {
        days -= 1;
    }
    date_at_start_of_year(year).wrapping_add(days)
}

use crate::game_state::GameState;
use std::collections::BTreeMap;
use std::ffi::c_void;
#[repr(C)]
pub struct NativeClockTimer {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct ConsoleException {
    _opaque: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Period {
    pub trigger: u8,
    pub priority: u8,
}
pub const DAY: u8 = 0;
pub const WEEK: u8 = 1;
pub const MONTH: u8 = 2;
pub const QUARTER: u8 = 3;
pub const YEAR: u8 = 4;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Callback {
    pub invoke: unsafe extern "C" fn(*const c_void, *mut ConsoleException) -> bool,
    pub context: *const c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Binding {
    pub callback: Callback,
    pub fired: *mut bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Subscription {
    period: *const Period,
    binding: *const Binding,
}
type Registry = BTreeMap<(u8, u8, usize), Subscription>;
static CALENDAR: GameState<Registry> = GameState::new(BTreeMap::new());
static ECONOMY: GameState<Registry> = GameState::new(BTreeMap::new());
fn key(timer: *const NativeClockTimer, period: Period) -> (u8, u8, usize) {
    (period.trigger, period.priority, timer as usize)
}
fn valid(registry: &GameState<Registry>, period: Period) -> bool {
    period.priority == 0
        || unsafe {
            !(&*registry.get()).values().any(|t| {
                let live = t.period.read();
                live.trigger == period.trigger && live.priority == period.priority
            })
        }
}
unsafe fn register(
    registry: &GameState<Registry>,
    timer: *const NativeClockTimer,
    period: *const Period,
    binding: *const Binding,
) {
    unsafe {
        (&mut *registry.get())
            .entry(key(timer, period.read()))
            .or_insert(Subscription { period, binding });
    }
}
unsafe fn unregister(
    registry: &GameState<Registry>,
    timer: *const NativeClockTimer,
    period: Period,
) {
    unsafe {
        (&mut *registry.get()).remove(&key(timer, period));
    }
}
unsafe fn change(
    registry: &GameState<Registry>,
    timer: *const NativeClockTimer,
    period: *mut Period,
    binding: *const Binding,
    new_period: Period,
    assert_valid: extern "C" fn(bool),
) {
    unsafe {
        unregister(registry, timer, period.read());
        period.write(new_period);
    }
    assert_valid(valid(registry, new_period));
    unsafe {
        register(registry, timer, period, binding);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_calendar_change(
    timer: *const NativeClockTimer,
    period: *mut Period,
    binding: *const Binding,
    new_period: Period,
    assert_valid: extern "C" fn(bool),
) {
    unsafe {
        change(&CALENDAR, timer, period, binding, new_period, assert_valid);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_economy_change(
    timer: *const NativeClockTimer,
    period: *mut Period,
    binding: *const Binding,
    new_period: Period,
    assert_valid: extern "C" fn(bool),
) {
    unsafe {
        change(&ECONOMY, timer, period, binding, new_period, assert_valid);
    }
}
macro_rules! registry_entries {
    ($registry:ident, $valid:ident, $register:ident, $unregister:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $valid(period: Period) -> bool {
            valid(&$registry, period)
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $register(
            timer: *const NativeClockTimer,
            period: *const Period,
            binding: *const Binding,
        ) {
            unsafe {
                register(&$registry, timer, period, binding);
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $unregister(timer: *const NativeClockTimer, period: Period) {
            unsafe {
                unregister(&$registry, timer, period);
            }
        }
    };
}
registry_entries!(
    CALENDAR,
    openttd_rust_calendar_valid,
    openttd_rust_calendar_register,
    openttd_rust_calendar_unregister
);
registry_entries!(
    ECONOMY,
    openttd_rust_economy_valid,
    openttd_rust_economy_register,
    openttd_rust_economy_unregister
);
pub(crate) fn calendar_snapshot() -> Vec<Subscription> {
    unsafe { (&*CALENDAR.get()).values().copied().collect() }
}
pub(crate) fn economy_snapshot() -> Vec<Subscription> {
    unsafe { (&*ECONOMY.get()).values().copied().collect() }
}
pub(crate) unsafe fn dispatch(
    timers: &[Subscription],
    trigger: u8,
    exception: *mut ConsoleException,
) -> bool {
    for timer in timers {
        let fired = unsafe { std::ptr::addr_of!((*timer.binding).fired).read() };
        if !fired.is_null() && unsafe { fired.read() } {
            continue;
        }
        if unsafe { timer.period.read().trigger } != trigger {
            continue;
        }
        // BaseTimer registers before derived construction binds the opaque closure.
        let callback = unsafe { std::ptr::addr_of!((*timer.binding).callback).read() };
        if !unsafe { (callback.invoke)(callback.context, exception) } {
            return false;
        }
        if !fired.is_null() {
            unsafe {
                fired.write(true);
            }
        }
    }
    true
}
#[repr(C)]
pub struct ClockServices {
    pub shared: *const crate::services::Services,
    pub game_mode: extern "C" fn() -> crate::openttd::GameMode,
    pub minutes_per_calendar_year: extern "C" fn() -> u16,
    pub using_wallclock_units: extern "C" fn(bool) -> bool,
    pub vehicle_id: extern "C" fn(*const c_void) -> u32,
    pub vehicle_shift_dates: extern "C" fn(crate::vehicle::VehicleRef, i32),
    pub next_graph: extern "C" fn(*mut u32) -> *mut crate::linkgraph::NativeGraph,
    pub graph_owner:
        extern "C" fn(*mut crate::linkgraph::NativeGraph) -> *mut crate::linkgraph::live::Graph,
}
#[repr(C)]
pub struct ClockIndustry {
    pub builder: *mut crate::industry::Builder,
    pub services: *const crate::industry::Services,
}
#[repr(C)]
pub struct ClockTown {
    pub leaves: *const crate::town::Leaves,
    pub shared: *const crate::services::Services,
}
#[repr(C)]
pub struct ClockCompany {
    pub services: *const crate::company::Services,
    pub settings: extern "C" fn() -> crate::company::EconomySettings,
    pub month: extern "C" fn() -> crate::company::Month,
    pub year: extern "C" fn() -> crate::company::Year,
}
macro_rules! owner_callback {
    ($name:ident, $ty:ty, $s:ident, $body:block) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(context: *const c_void, _: *mut ConsoleException) -> bool {
            let $s = unsafe { &*context.cast::<$ty>() };
            unsafe { $body }
            true
        }
    };
}
owner_callback!(openttd_rust_clock_industry_daily, ClockIndustry, c, {
    crate::industry::openttd_rust_industry_daily(c.builder, c.services);
});
owner_callback!(openttd_rust_clock_industry_monthly, ClockIndustry, c, {
    crate::industry::openttd_rust_industry_monthly(c.builder, c.services);
});
owner_callback!(
    openttd_rust_clock_engine_daily,
    crate::engine::Services,
    s,
    {
        crate::engine::openttd_rust_engine_daily(s);
    }
);
owner_callback!(
    openttd_rust_clock_engine_monthly,
    crate::engine::Services,
    s,
    {
        crate::engine::openttd_rust_engine_monthly(s);
    }
);
owner_callback!(
    openttd_rust_clock_station_monthly,
    crate::station_service::World,
    s,
    {
        crate::station_service::openttd_rust_station_monthly(s);
    }
);
owner_callback!(
    openttd_rust_clock_subsidy_monthly,
    crate::subsidy::Services,
    s,
    {
        let mut cargo = [0; crate::cargo_type::NUM_CARGO as usize];
        crate::subsidy::openttd_rust_subsidy_monthly(
            s,
            crate::subsidy::Cargo {
                data: cargo.as_mut_ptr(),
            },
        );
    }
);
owner_callback!(openttd_rust_clock_town_monthly, ClockTown, c, {
    crate::town::openttd_rust_town_monthly_all(c.leaves, c.shared, economy_month());
});
owner_callback!(openttd_rust_clock_town_yearly, ClockTown, c, {
    crate::town::house::openttd_rust_house_age_all(c.leaves, c.shared);
});
owner_callback!(
    openttd_rust_clock_disaster_daily,
    crate::disaster::Services,
    s,
    {
        crate::disaster::openttd_rust_disaster_daily(s);
    }
);
owner_callback!(openttd_rust_clock_economy_month, ClockCompany, c, {
    crate::company::openttd_rust_economy_month(&*c.services, &(c.month)());
});
owner_callback!(
    openttd_rust_clock_economy_calendar_month,
    ClockCompany,
    c,
    {
        crate::company::openttd_rust_economy_calendar_month(&*c.services, &(c.settings)());
    }
);
owner_callback!(openttd_rust_clock_company_yearly, ClockCompany, c, {
    crate::company::openttd_rust_company_yearly(&*c.services, &(c.year)());
});
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDClockIndustry" ["clocks"] => ClockIndustry { builder, services };
    "OpenTTDClockTown" ["clocks"] => ClockTown { leaves, shared };
    "OpenTTDClockCompany" ["clocks"] => ClockCompany { services, settings, month, year };
    "OpenTTDClockState" ["clocks"] => ClockState { calendar_year, calendar_date, economy_year, economy_date, days_since_last_month, calendar_fraction, calendar_subfraction, economy_fraction, calendar_month, economy_month };
    "OpenTTDClockPeriod" ["clocks"] => Period { trigger, priority };
    "OpenTTDEconomyClockPeriod" ["clocks"] => Period { trigger, priority };
    "RustClockCallback" ["clocks"] => Callback { invoke, context };
    "RustClockBinding" ["clocks"] => Binding { callback, fired };
    "OpenTTDClockServices" ["clocks"] => ClockServices { shared, game_mode, minutes_per_calendar_year, using_wallclock_units, vehicle_id, vehicle_shift_dates, next_graph, graph_owner };
}
