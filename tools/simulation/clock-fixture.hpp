/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#ifndef SIMULATION_CLOCK_FIXTURE_HPP
#define SIMULATION_CLOCK_FIXTURE_HPP
#ifdef WITH_RUST
#include "rust/clock_native.hpp"
#include "session-fixture.hpp"
struct ClockFixture : SessionFixture {
	std::exception_ptr exception;
	uint checks = 0;
	bool advanced = false;
	static ClockFixture &Current() { return *static_cast<ClockFixture *>(active); }
};
#endif
/* Stop the production session immediately after its clocks. Its vehicle-table
 * observation exposes the calendar's return without a fixture-only export. */
template <typename Clock> static bool FixtureClockElapsed()
{
#ifdef WITH_RUST
	ClockFixture fixture;
	fixture.clock = GetRustClockServices();
	fixture.skip_animation = true;
	fixture.services.console_exception = [](OpenTTDSessionFrame *) noexcept {
		return reinterpret_cast<OpenTTDConsoleException *>(&ClockFixture::Current().exception);
	};
	fixture.services.failed = [](OpenTTDSessionFrame *) noexcept {
		auto &current = ClockFixture::Current();
		if (current.exception) return true;
		if (current.checks++ != 0) return true;
		// Calendar-only probes freeze the following economy step at its native
		// menu gate; all calendar callbacks have already returned here.
		if constexpr (std::is_same_v<Clock, TimerGameCalendar>) _game_mode = GM_MENU;
		return false;
	};
	fixture.services.vehicle_tables = [](OpenTTDSessionFrame *, bool) noexcept {
		auto &current = ClockFixture::Current();
		current.advanced = true;
		return current.tables;
	};
	if constexpr (std::is_same_v<Clock, TimerGameEconomy>) {
		fixture.clock.minutes_per_calendar_year = []() noexcept -> uint16_t { return 0; };
	}
	const auto saved_mode = _game_mode;
	const auto saved_pause = _pause_mode;
	if (_game_mode == GM_EDITOR) _game_mode = GM_NORMAL;
	_pause_mode = {};
	openttd_rust_session_state_loop(&fixture.services, nullptr);
	// The intentional phase stop leaves storage entered, just like a real
	// exception. Restore the fixture scope before relaying an exception.
	openttd_rust_storage_switch_mode(PSM_LEAVE_GAMELOOP, true, fixture.storage.assert_condition, fixture.shared.storage_discard);
	_game_mode = saved_mode;
	_pause_mode = saved_pause;
	if (fixture.exception) std::rethrow_exception(fixture.exception);
	if constexpr (std::is_same_v<Clock, TimerGameCalendar>) return fixture.advanced;
	else return saved_mode != GM_MENU;
#else
	return TimerManager<Clock>::Elapsed(1);
#endif
}
#endif
