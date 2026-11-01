/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file timer_game_calendar.cpp
 * This file implements the timer logic for the game-calendar-timer.
 */

/**
 * Calendar time is used for technology and time-of-year changes, including:
 * - Vehicle, airport, station, object introduction and obsolescence
 * - Vehicle and engine age
 * - NewGRF variables for visual styles or behavior based on year or time of year (e.g. variable snow line)
 * - Inflation, since it is tied to original game years. One interpretation of inflation is that it compensates for faster and higher capacity vehicles,
 *   another is that it compensates for more established companies. Each of these point to a different choice of calendar versus economy time, but we have to pick one
 *   so we follow a previous decision to tie inflation to original TTD game years.
 */

#include "../stdafx.h"
#include "../openttd.h"
#include "timer.h"
#include "timer_game_calendar.h"
#include "../vehicle_base.h"

#include "../rust/clock_native.hpp"

#include "../safeguards.h"


TimerGameCalendar::YearMonthDay TimerGameCalendar::ConvertDateToYMD(Date date)
{
	auto ymd = openttd_rust_calendar_date_to_ymd(date.base());
	return {Year{ymd.year}, ymd.month, ymd.day};
}
TimerGameCalendar::Date TimerGameCalendar::ConvertYMDToDate(Year year, Month month, Day day)
{
	return Date{openttd_rust_calendar_ymd_to_date(year.base(), month, day)};
}
void TimerGameCalendar::SetDate(Date date, DateFract fract)
{
	assert(fract < Ticks::DAY_TICKS);
	openttd_rust_calendar_set_date(date.base(), fract);
}
template <>
void IntervalTimer<TimerGameCalendar>::Elapsed([[maybe_unused]] TimerGameCalendar::TElapsed trigger)
{
	this->callback(1);
}
template <>
void TimeoutTimer<TimerGameCalendar>::Elapsed([[maybe_unused]] TimerGameCalendar::TElapsed trigger)
{
	this->callback();
}
