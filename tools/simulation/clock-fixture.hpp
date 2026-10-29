/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#ifndef SIMULATION_CLOCK_FIXTURE_HPP
#define SIMULATION_CLOCK_FIXTURE_HPP
#ifdef WITH_RUST
#include "rust/clock_native.hpp"
#endif
/* Exercise the real clock entry and its ordinary native exception relay. */
template <typename Clock> static bool FixtureClockElapsed()
{
#ifdef WITH_RUST
	if constexpr (std::is_same_v<Clock, TimerGameCalendar>) return RustClockElapsed<openttd_rust_calendar_elapsed>(GetRustClockServices(), 1);
	else return RustClockElapsed<openttd_rust_economy_elapsed>(GetRustClockServices(), 1);
#else
	return TimerManager<Clock>::Elapsed(1);
#endif
}
#endif
