/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Clock edge cases run through the real owners and retain every semantic save chunk. */
#include "stdafx.h"
#include "console_internal.h"
#include "console_func.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "settings_type.h"
#include "openttd.h"
#include "vehicle_base.h"
#include "linkgraph/linkgraph.h"
#include "saveload/saveload.h"
#include "clock-fixture.hpp"

static void Check(const char *label, bool value)
{
	fmt::print(stderr, "CLOCK {} {}\n", label, value);
	if (!value) std::abort();
}
static void MaxEconomy(bool wallclock)
{
	using E = TimerGameEconomy;
	_settings_game.economy.timekeeping_units = wallclock ? TKU_WALLCLOCK : TKU_CALENDAR;
	E::SetDate(E::ConvertYMDToDate(EconomyTime::MAX_YEAR + 1, 0, 1) - 1, 73);
	std::vector<std::pair<Vehicle *, E::Date>> vehicles;
	std::vector<std::pair<LinkGraph *, E::Date>> graphs;
	for (Vehicle *v : Vehicle::Iterate()) vehicles.emplace_back(v, v->date_of_last_service);
	for (LinkGraph *g : LinkGraph::Iterate()) graphs.emplace_back(g, g->LastCompression());
	FixtureClockElapsed<E>();
	bool shifted = !vehicles.empty() && !graphs.empty();
	for (auto [vehicle, date] : vehicles) shifted &= vehicle->date_of_last_service == std::max(date - 366, E::Date{0});
	for (auto [graph, date] : graphs) shifted &= graph->LastCompression() == date - 366;
	Check(wallclock ? "wallclock-max-shifts" : "calendar-max-shifts", shifted && E::year == EconomyTime::MAX_YEAR && E::date == E::ConvertYMDToDate(EconomyTime::MAX_YEAR + 1, 0, 1) - 366);
}
static bool ClocksScenario(std::span<std::string_view> args)
{
	using C = TimerGameCalendar;
	using E = TimerGameEconomy;
	if (args.size() < 2) return false;
	if (args[1] == "inspect") {
		Check("reloaded-fractions", C::date_fract == 17 && C::sub_date_fract == 113 && E::date_fract == 37 && E::days_since_last_month == 23);
		return true;
	}
	auto calendar_date = C::date;
	auto economy_date = E::date;
	C::date_fract = 17; C::sub_date_fract = 19; E::date_fract = 37; E::days_since_last_month = 23;
	C::SetDate(calendar_date, 17); E::SetDate(economy_date, 37);
	Check("setdate-fractions", C::sub_date_fract == 19 && E::days_since_last_month == 23);
	_game_mode = GM_MENU;
	Check("menu-frozen", !FixtureClockElapsed<C>() && !FixtureClockElapsed<E>() && C::date_fract == 17 && E::date_fract == 37);
	_game_mode = GM_NORMAL;
	_settings_game.economy.minutes_per_calendar_year = 0;
	Check("calendar-frozen", !FixtureClockElapsed<C>() && C::date_fract == 17);
	_settings_game.economy.minutes_per_calendar_year = 24;
	C::sub_date_fract = 73;
	bool slowed = !FixtureClockElapsed<C>() && C::sub_date_fract == 147 && C::date_fract == 17;
	slowed &= FixtureClockElapsed<C>() && C::sub_date_fract == 73 && C::date_fract == 18;
	_settings_game.economy.minutes_per_calendar_year = CalendarTime::MAX_MINUTES_PER_YEAR;
	C::sub_date_fract = 65535;
	slowed &= !FixtureClockElapsed<C>() && C::sub_date_fract == 73 && C::date_fract == 18;
	Check("slowed-widths", slowed);
	_settings_game.economy.minutes_per_calendar_year = CalendarTime::DEF_MINUTES_PER_YEAR;
	for (int year : {0, 1900, 2000}) {
		C::SetDate(C::ConvertYMDToDate(C::Year{year}, 1, 28), 73);
		FixtureClockElapsed<C>();
		auto ymd = C::ConvertDateToYMD(C::date);
		bool leap = year != 1900;
		Check("leap-century", ymd.year == year && ymd.month == (leap ? 1 : 2) && ymd.day == (leap ? 29 : 1));
	}
	C::SetDate(C::ConvertYMDToDate(CalendarTime::MAX_YEAR + 1, 0, 1) - 1, 73);
	FixtureClockElapsed<C>();
	Check("calendar-max", C::year == CalendarTime::MAX_YEAR && C::date == C::ConvertYMDToDate(CalendarTime::MAX_YEAR, 0, 1) && C::sub_date_fract == 0);
	_settings_newgame.economy.timekeeping_units = TKU_WALLCLOCK;
	_settings_game.economy.timekeeping_units = TKU_CALENDAR;
	Check("newgame-units", E::UsingWallclockUnits(true) && !E::UsingWallclockUnits());
	_settings_game.economy.timekeeping_units = TKU_WALLCLOCK;
	E::SetDate(E::ConvertYMDToDate(E::Year{2000}, 0, 30), 73);
	FixtureClockElapsed<E>();
	auto ymd = E::ConvertDateToYMD(E::date);
	Check("wallclock-month", ymd.year == 2000 && ymd.month == 1 && ymd.day == 1);
	MaxEconomy(false); MaxEconomy(true);
	_settings_game.economy.timekeeping_units = TKU_CALENDAR;
	_settings_newgame.economy.timekeeping_units = TKU_CALENDAR;
	_settings_game.economy.minutes_per_calendar_year = 24;
	C::SetDate(calendar_date, 17); C::sub_date_fract = 113;
	E::SetDate(economy_date, 37); E::days_since_last_month = 23;
	if (args.size() != 3 || SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
	Check("saved-fractions", C::sub_date_fract == 113 && E::days_since_last_month == 23);
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("clocks_scenario", ClocksScenario);
}
