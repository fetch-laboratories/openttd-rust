/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file timer_services.h Shared calendar conversion observations. */
#ifndef RUST_TIMER_SERVICES_H
#define RUST_TIMER_SERVICES_H
#include "services_ffi.h"
#include "../timer/timer_game_calendar.h"
inline OpenTTDCalendarYMD RustCalendarDateToYMD(int32_t date) noexcept
{
	auto ymd = TimerGameCalendar::ConvertDateToYMD(TimerGameCalendar::Date{date});
	return {.year = ymd.year.base(), .month = ymd.month, .day = ymd.day};
}
inline int32_t RustCalendarYMDToDate(int32_t year, uint8_t month, uint8_t day) noexcept { return TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year}, month, day).base(); }
inline uint16_t RustCalendarDateFraction() noexcept { return TimerGameCalendar::date_fract; }
#endif
