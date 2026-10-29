/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Save files cannot observe subscription identity, live mutation or native exception cleanup. */
#include "stdafx.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"
#include "settings_type.h"
#include "openttd.h"
#include "script/script_fatalerror.hpp"
#include <thread>
#include <cstdio>

// Clock probes use no script configuration; native settings own empty config pointers.
class AIConfig {};
class GameConfig {};
ScriptConfigSettings::ScriptConfigSettings() = default;
ScriptConfigSettings::~ScriptConfigSettings() = default;
#ifndef WITH_RUST
GameMode _game_mode = GM_NORMAL;
#endif
GameSettings _settings_game{}, _settings_newgame{};
#ifdef WITH_RUST
#include "rust/clock_native.hpp"
static const OpenTTDSharedServices shared{.vehicle_next = [](uint32_t) noexcept -> OpenTTDVehicle * { return nullptr; }};
const OpenTTDClockServices &GetRustClockServices() noexcept
{
	static const OpenTTDClockServices services{
		.shared = &shared,
		.minutes_per_calendar_year = []() noexcept { return _settings_game.economy.minutes_per_calendar_year; },
		.using_wallclock_units = [](bool newgame) noexcept { return (newgame ? _settings_newgame : _settings_game).economy.timekeeping_units == TKU_WALLCLOCK; },
		.next_graph = [](uint32_t *) noexcept -> LinkGraph * { return nullptr; },
	};
	return services;
}
#else
struct Vehicle {
	static std::array<Vehicle *, 0> Iterate() { return {}; }
	void ShiftDates(TimerGameEconomy::Date) {}
};
struct LinkGraph {
	static std::array<LinkGraph *, 0> Iterate() { return {}; }
	void ShiftDates(TimerGameEconomy::Date) {}
};
#endif
#include "clock-bodies.inc"



#include "clock-fixture.hpp"
static void Check(const char *name, bool valid)
{
	std::printf("CLOCK %s %s\n", name, valid ? "true" : "false");
	if (!valid) std::abort();
}
static void CalendarEnd(int year = 1999)
{
	TimerGameCalendar::SetDate(TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year}, 11, 31), 73);
}
static void CalendarDay()
{
	TimerGameCalendar::date_fract = 73;
	FixtureClockElapsed<TimerGameCalendar>();
}
static int effects;
static void ScheduledFailure() { effects = effects * 10 + 2; throw Script_FatalError("clock probe"); }
static void Subscriptions()
{
	using C = TimerGameCalendar;
	using Interval = IntervalTimer<C>;
	std::vector<int> calls;
	std::array<std::optional<Interval>, 3> ordered;
	for (int i : {2, 1, 0}) ordered[i].emplace(C::TPeriod{C::MONTH, C::NONE}, [&, i](auto) { calls.push_back(i); if (i == 2) ordered[i]->SetInterval({C::YEAR, C::NONE}); });
	std::optional<Interval> added;
	bool first = true;
	Interval daily({C::DAY, C::NONE}, [&](auto) {
		if (!first) return;
		first = false;
		calls.push_back(9);
		TimerManager<C>::UnregisterTimer(*ordered[2]);
		ordered[1]->SetInterval({C::YEAR, C::NONE});
		added.emplace(C::TPeriod{C::DAY, C::NONE}, [&](auto) { calls.push_back(8); });
	});
	CalendarEnd(); FixtureClockElapsed<C>();
	Check("snapshot-live-order", calls == std::vector<int>{9, 0, 2, 1, 2});
	TimerManager<C>::RegisterTimer(*added);
	CalendarDay();
	Check("added-next-snapshot", calls == std::vector<int>{9, 0, 2, 1, 2, 8});
}
static void Timeouts()
{
	using C = TimerGameCalendar;
	TimeoutTimer<C> *self = nullptr;
	int count = 0;
	TimeoutTimer<C> timer({C::DAY, C::NONE}, [&] { count++; self->Reset(); }, true);
	self = &timer;
	CalendarEnd(); FixtureClockElapsed<C>(); CalendarDay();
	Check("timeout-post-reset", count == 1 && timer.HasFired());
	timer.Reset({C::MONTH, C::NONE}); CalendarEnd(); FixtureClockElapsed<C>();
	Check("timeout-period-reset", count == 2 && timer.HasFired());
	timer.Reset(); timer.Abort(); CalendarEnd(); FixtureClockElapsed<C>();
	Check("timeout-abort", count == 2 && timer.HasFired());
}
static void LiveEconomy()
{
	using E = TimerGameEconomy;
	int week = 0, quarter = 0;
	IntervalTimer<E> daily({E::DAY, E::NONE}, [](auto) { E::date = E::Date{3}; });
	IntervalTimer<E> weekly({E::WEEK, E::NONE}, [&](auto) { week++; });
	IntervalTimer<E> monthly({E::MONTH, E::NONE}, [](auto) { E::month = 3; });
	IntervalTimer<E> quarterly({E::QUARTER, E::NONE}, [&](auto) { quarter++; });
	int year = 2000;
	while ((E::ConvertYMDToDate(E::Year{year}, 2, 1).base() % 7) == 3) year++;
	E::SetDate(E::ConvertYMDToDate(E::Year{year}, 2, 1) - 1, 73);
	FixtureClockElapsed<E>();
	Check("live-week-quarter", week == 1 && quarter == 1 && E::days_since_last_month == 0);
}
static void ThreadsAndAliases()
{
	using C = TimerGameCalendar;
	int count = 0;
	std::unique_ptr<IntervalTimer<C>> timer;
	auto *year = &C::year; auto *date = &C::date;
	C::sub_date_fract = 19;
	std::thread first([&] {
		timer = std::make_unique<IntervalTimer<C>>(C::TPeriod{C::YEAR, C::NONE}, [&](auto) { count++; });
		CalendarEnd();
		Check("thread-aliases", year == &C::year && date == &C::date && C::sub_date_fract == 19);
	}); first.join();
	FixtureClockElapsed<C>();
	Check("thread-registration", count == 1 && C::year == 2000);
	std::thread second([] { FixtureClockElapsed<C>(); }); second.join();
	Check("thread-clock", C::date_fract == 1);
	TimerGameEconomy::days_since_last_month = 123;
	TimerGameEconomy::SetDate(TimerGameEconomy::Date{3}, 71);
	Check("setdate-retains-fields", TimerGameEconomy::days_since_last_month == 123);
#ifdef WITH_RUST
	auto &state = GetRustClockState();
	Check("native-canonical-addresses", &state.calendar_year == year && &state.calendar_date == date && &state.economy_fraction == &TimerGameEconomy::date_fract);
#else
	Check("native-canonical-addresses", true);
#endif
}
static void Exceptions()
{
	using C = TimerGameCalendar;
	IntervalTimer<C> before({C::DAY, C::NONE}, [](auto) { effects = effects * 10 + 1; });
#ifdef WITH_RUST
	IntervalTimer<C> throwing({C::MONTH, C::NONE}, RustClockCallback{[](const void *, void *exception) noexcept { return RustClockRelayScheduledConsole(ScheduledFailure, exception); }, nullptr});
#else
	IntervalTimer<C> throwing({C::MONTH, C::NONE}, [](auto) { ScheduledFailure(); });
#endif
	IntervalTimer<C> after({C::YEAR, C::NONE}, [](auto) { effects = effects * 10 + 3; });
	CalendarEnd(CalendarTime::MAX_YEAR.base()); effects = 0;
	bool caught = false;
	try { FixtureClockElapsed<C>(); } catch (const Script_FatalError &) { caught = true; }
	Check("console-partial-exception", caught && effects == 12 && C::year == CalendarTime::MAX_YEAR + 1 && C::date_fract == 0);
	throwing.SetInterval({C::WEEK, C::NONE});
	CalendarEnd(); FixtureClockElapsed<C>();
	Check("exception-snapshot-cleanup", C::year == 2000 && effects == 1213);
}
int main(int argc, char **)
{
	_game_mode = GM_NORMAL;
	_settings_game.economy.minutes_per_calendar_year = CalendarTime::DEF_MINUTES_PER_YEAR;
	_settings_game.economy.timekeeping_units = TKU_CALENDAR;
	if (argc > 1) {
		IntervalTimer<TimerGameCalendar> a({TimerGameCalendar::DAY, TimerGameCalendar::ENGINE}, [](auto) {});
		IntervalTimer<TimerGameCalendar> b({TimerGameCalendar::DAY, TimerGameCalendar::ENGINE}, [](auto) {});
		return 0;
	}
	Subscriptions(); Timeouts(); LiveEconomy(); ThreadsAndAliases(); Exceptions();
}
