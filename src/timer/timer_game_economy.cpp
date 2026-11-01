/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file timer_game_economy.cpp
 * This file implements the timer logic for the game-economy-timer.
 */

/**
 * Economy time is used for the regular pace of the game, including:
 * - Industry and house production/consumption
 * - Industry production changes, closure, and spawning
 * - Town growth
 * - Company age and financial statistics
 * - Vehicle financial statistics
 * - Vehicle aging, depreciation, reliability, and renewal
 * - Payment intervals for running and maintenance costs, loan interest, etc.
 * - Cargo payment "time" calculation
 * - Local authority and station ratings change intervals
 */

#include "../stdafx.h"
#include "../openttd.h"
#include "timer.h"
#include "timer_game_economy.h"
#include "timer_game_tick.h"
#include "../vehicle_base.h"
#include "../linkgraph/linkgraph.h"

#include "../rust/clock_native.hpp"

#include "../safeguards.h"


TimerGameEconomy::YearMonthDay TimerGameEconomy::ConvertDateToYMD(Date date)
{
	auto ymd = openttd_rust_economy_date_to_ymd(date.base(), UsingWallclockUnits());
	return {Year{ymd.year}, ymd.month, ymd.day};
}
TimerGameEconomy::Date TimerGameEconomy::ConvertYMDToDate(Year year, Month month, Day day)
{
	return Date{openttd_rust_economy_ymd_to_date(year.base(), month, day, UsingWallclockUnits())};
}
void TimerGameEconomy::SetDate(Date date, DateFract fract)
{
	assert(fract < Ticks::DAY_TICKS);
	openttd_rust_economy_set_date(date.base(), fract, UsingWallclockUnits());
}
bool TimerGameEconomy::UsingWallclockUnits(bool newgame)
{
	return GetRustClockServices().using_wallclock_units(newgame);
}
template <>
void IntervalTimer<TimerGameEconomy>::Elapsed([[maybe_unused]] TimerGameEconomy::TElapsed trigger)
{
	this->callback(1);
}
template <>
void TimeoutTimer<TimerGameEconomy>::Elapsed([[maybe_unused]] TimerGameEconomy::TElapsed trigger)
{
	this->callback();
}
