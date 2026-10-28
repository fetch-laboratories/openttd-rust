/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file timer_game_calendar.h Definition of the game-calendar-timer */

#ifndef TIMER_GAME_CALENDAR_H
#define TIMER_GAME_CALENDAR_H

#include "timer_game_common.h"
#ifdef WITH_RUST
#include "../rust/clock_state.h"
#endif

/**
 * Timer that is increased every 27ms, and counts towards ticks / days / months / years.
 *
 * The amount of days in a month depends on the month and year (leap-years).
 * There are always 74 ticks in a day (and with 27ms, this makes 1 day 1.998 seconds).
 *
 * Calendar time is used for technology and time-of-year changes, including:
 * - Vehicle, airport, station, object introduction and obsolescence
 * - NewGRF variables for visual styles or behavior based on year or time of year (e.g. variable snow line)
 * - Inflation, since it is tied to original game years. One interpretation of inflation is that it compensates for faster and higher capacity vehicles,
 *   another is that it compensates for more established companies. Each of these point to a different choice of calendar versus economy time, but we have to pick one
 *   so we follow a previous decision to tie inflation to original TTD game years.
 */
class TimerGameCalendar : public TimerGame<struct Calendar> {
public:
#ifdef WITH_RUST
	inline static Year &year = GetRustClockState().calendar_year; ///< Current year, starting at 0.
#else
	static Year year; ///< Current year, starting at 0.
#endif
#ifdef WITH_RUST
	inline static Month &month = GetRustClockState().calendar_month; ///< Current month (0..11).
#else
	static Month month; ///< Current month (0..11).
#endif
#ifdef WITH_RUST
	inline static Date &date = GetRustClockState().calendar_date; ///< Current date in days (day counter).
#else
	static Date date; ///< Current date in days (day counter).
#endif
#ifdef WITH_RUST
	inline static DateFract &date_fract = GetRustClockState().calendar_fraction; ///< Fractional part of the day.
#else
	static DateFract date_fract; ///< Fractional part of the day.
#endif
#ifdef WITH_RUST
	inline static uint16_t &sub_date_fract = GetRustClockState().calendar_subfraction; ///< Subpart of date_fract that we use when calendar days are slower than economy days.
#else
	static uint16_t sub_date_fract; ///< Subpart of date_fract that we use when calendar days are slower than economy days.
#endif

	static YearMonthDay ConvertDateToYMD(Date date);
	static Date ConvertYMDToDate(Year year, Month month, Day day);
	static void SetDate(Date date, DateFract fract);
};

/**
 * Storage class for Calendar time constants.
 */
class CalendarTime : public TimerGameConst<struct Calendar> {
public:
	static constexpr int DEF_MINUTES_PER_YEAR = 12;
	static constexpr int FROZEN_MINUTES_PER_YEAR = 0;
	static constexpr int MAX_MINUTES_PER_YEAR = 10080; // One week of real time. The actual max that doesn't overflow TimerGameCalendar::sub_date_fract is 10627, but this is neater.
};

#endif /* TIMER_GAME_CALENDAR_H */
