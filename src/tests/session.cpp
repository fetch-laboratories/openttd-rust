/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file session.cpp Canonical native aliases and pause command reentry. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../rust/session_ffi.h"
#include "../core/backup_type.hpp"
#include "../rust/session_native.hpp"
#include <thread>
static bool session_networking;
static uint session_queries, session_changes, session_dirty;
static uint8_t session_previous;
TEST_CASE("Session pause - all command inputs and unsaved native aliases")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, _game_mode);
	AutoRestoreBackup<SwitchMode> switching(_switch_mode, _switch_mode);
	AutoRestoreBackup<PauseModes> paused(_pause_mode, _pause_mode);
	const auto *mode_address = &_game_mode;
	const auto *switch_address = &_switch_mode;
	const auto *pause_address = &_pause_mode;
	std::thread first([] { _game_mode = GM_EDITOR; _switch_mode = SM_RELOADGAME; _pause_mode = PauseMode::Normal; }); first.join();
	CHECK(_game_mode == GM_EDITOR); CHECK(_switch_mode == SM_RELOADGAME); CHECK(_pause_mode == PauseMode::Normal);
	std::thread second([&] {
		CHECK(mode_address == &_game_mode); CHECK(switch_address == &_switch_mode); CHECK(pause_address == &_pause_mode);
		CHECK(openttd_rust_session_storage() == &GetRustSessionState());
		CHECK(_game_mode == GM_EDITOR); CHECK(_switch_mode == SM_RELOADGAME); CHECK(_pause_mode == PauseMode::Normal);
	}); second.join();
	auto services = GetRustSessionServices();
	services.networking = []() noexcept { return session_networking; };
	services.pause_query = []() noexcept { session_queries++; };
	services.pause_change = [](uint8_t previous, PauseMode) noexcept { session_changes++; session_previous = previous; };
	services.pause_dirty = []() noexcept { session_dirty++; };
	for (bool network : {false, true}) for (uint value = 0; value < 256; value++) for (bool execute : {false, true}) {
		session_networking = network; session_queries = session_changes = session_dirty = 0;
		_pause_mode = PauseMode::CommandDuringPause;
		const bool valid = value < 7 && (network || (value != 2 && value != 4));
		const auto selected = static_cast<PauseMode>(value);
		CHECK(openttd_rust_session_pause(&services, selected, true, execute) == valid);
		CHECK(_pause_mode.base() == (valid && execute ? 0x80 | (1U << value) : 0x80));
		CHECK(session_changes == uint(valid && execute)); CHECK(session_dirty == uint(valid && execute)); CHECK(session_queries == 0);
		if (valid && execute) {
			CHECK(session_previous == 0x80);
			CHECK(openttd_rust_session_pause(&services, selected, false, true));
			CHECK(_pause_mode.None());
		}
	}
	_pause_mode = PauseMode::Normal;
	std::thread command([&] { CHECK(openttd_rust_session_pause(&services, PauseMode::GameScript, true, true)); }); command.join();
	CHECK(_pause_mode.base() == 0x21);
	_pause_mode = PauseMode::Error;
	CHECK(openttd_rust_session_pause(&services, PauseMode::Normal, false, true));
	CHECK(_pause_mode == PauseMode::Error); CHECK(session_queries == 1);
}
static const OpenTTDSessionServices *session_reentry;
static bool session_nested;
TEST_CASE("Session pause - native callback sees live state and can reenter")
{
	AutoRestoreBackup<PauseModes> paused(_pause_mode, {});
	auto services = GetRustSessionServices(); session_reentry = &services; session_nested = false;
	services.pause_dirty = []() noexcept {};
	services.pause_change = [](uint8_t previous, PauseMode mode) noexcept {
		if (session_nested) { CHECK(previous == 1); CHECK(mode == PauseMode::GameScript); return; }
		CHECK(previous == 0); CHECK(_pause_mode == PauseMode::Normal);
		session_nested = true;
		CHECK(openttd_rust_session_pause(session_reentry, PauseMode::GameScript, true, true));
	};
	CHECK(openttd_rust_session_pause(&services, PauseMode::Normal, true, true));
	CHECK(_pause_mode.base() == 0x21); CHECK(session_nested);
	session_reentry = nullptr;
}
static SaveOrLoadResult session_load_result;
static bool session_dedicated;
TEST_CASE("Session load - original restoration and callback partial effects")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_NORMAL);
	AutoRestoreBackup<SwitchMode> switching(_switch_mode, SM_NONE);
	AutoRestoreBackup<PauseModes> paused(_pause_mode, {});
	struct RestoreExit { bool value; ~RestoreExit() { _exit_game = this->value; } } exit{_exit_game.load()};
	_exit_game = false;
	auto services = GetRustSessionServices();
	services.dedicated = []() noexcept { return session_dedicated; };
	services.load_failed_debug = [](bool) noexcept {};
	services.load = [](OpenTTDSessionFrame *, OpenTTDSessionLoad *) noexcept {
		CHECK(_game_mode == GM_EDITOR);
		_game_mode = GM_BOOTSTRAP; _switch_mode = SM_EDITOR; _pause_mode = PauseMode::LinkGraph;
		return session_load_result;
	};
	OpenTTDSessionFrame frame;
	session_dedicated = false; session_load_result = SL_ERROR;
	CHECK_FALSE(openttd_rust_session_safe_load(&services, &frame, nullptr, GM_EDITOR));
	CHECK(_game_mode == GM_NORMAL); CHECK(_switch_mode == SM_EDITOR); CHECK(_pause_mode == PauseMode::LinkGraph);
	session_load_result = SL_OK;
	CHECK(openttd_rust_session_safe_load(&services, &frame, nullptr, GM_EDITOR));
	CHECK(_game_mode == GM_BOOTSTRAP);
	_game_mode = GM_MENU; session_dedicated = true; session_load_result = SL_ERROR;
	CHECK_FALSE(openttd_rust_session_safe_load(&services, &frame, nullptr, GM_EDITOR));
	CHECK(_game_mode == GM_BOOTSTRAP); CHECK(_exit_game.load());
}
