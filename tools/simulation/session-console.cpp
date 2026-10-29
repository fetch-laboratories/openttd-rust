/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file session-console.cpp Actual session entries, reset/reload and unsaved exception effects. */
#include "stdafx.h"
#include "console_internal.h"
#include "console_func.h"
#include "openttd.h"
#include "company_base.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"
#include "saveload/saveload.h"
#include "newgrf_storage.h"
#include "newgrf_config.h"
#include "misc_cmd.h"
#include "tile_map.h"
#include "network/network.h"
#include "genworld.h"
#include "script/script_fatalerror.hpp"
#ifdef WITH_RUST
#include "rust/clock_native.hpp"
#endif
extern void MakeNewgameSettingsLive();
extern bool SafeLoad(const std::string &, SaveLoadOperation, DetailedFileType, GameMode, Subdirectory, std::shared_ptr<LoadFilter>);
extern bool _in_modal_progress;
static bool session_active = false;
static int session_failure = 0, session_scans = 0;
static PersistentStorageArray<int32_t, 16> *session_storage;
static void SessionCheck(std::string_view name, bool value)
{
	fmt::print(stderr, "SESSION check {} {}\n", name, value);
	if (!value) std::abort();
}
static void SessionObserve(std::string_view label)
{
	uint64_t map = 14695981039346656037ULL, credit = 0;
	for (TileIndex tile : Map::Iterate()) {
		Tile t{tile};
		for (uint32_t v : {uint32_t(t.type()), uint32_t(t.height()), uint32_t(t.m1()), uint32_t(t.m2()), uint32_t(t.m3()), uint32_t(t.m4()), uint32_t(t.m5()), uint32_t(t.m6()), uint32_t(t.m7()), uint32_t(t.m8())}) map = (map ^ v) * 1099511628211ULL;
	}
	for (Company *c : Company::Iterate()) {
#ifdef WITH_RUST
		const auto &f = c->Finances();
#else
		const auto &f = *c;
#endif
		credit += uint64_t(f.clear_limit) + f.terraform_limit + f.tree_limit + f.build_object_limit;
	}
	fmt::print(stderr, "SESSION state {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n", label, _game_mode, _switch_mode, _pause_mode.base(), TimerGameCalendar::date.base(), TimerGameCalendar::date_fract, TimerGameCalendar::sub_date_fract, TimerGameEconomy::date.base(), TimerGameEconomy::date_fract, TimerGameTick::counter, _random.state[0], _random.state[1], credit, map, _current_company.base(), _local_company.base());
}
static void SessionSave(const std::string &folder, std::string_view label)
{
	SessionObserve(label);
	if (SaveOrLoad(folder + "/" + std::string(label) + ".sav", SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
}
static void SessionThrow()
{
	session_storage->StoreValue(0, 11);
	_current_company = OWNER_DEITY;
	throw Script_FatalError("session exception fixture");
}
extern "C" void __real__ZN4Game8GameLoopEv();
extern "C" void __wrap__ZN4Game8GameLoopEv()
{
	if (session_failure == 2) SessionThrow();
	__real__ZN4Game8GameLoopEv();
}
static void SessionExceptions()
{
	using C = TimerGameCalendar;
	using E = TimerGameEconomy;
	for (int phase : {1, 2}) {
		PersistentStorageArray<int32_t, 16> storage;
		session_storage = &storage; session_failure = phase;
		C::SetDate(C::ConvertYMDToDate(C::Year{2000}, 0, 31), phase == 1 ? 73 : 1);
		E::date_fract = 5;
		std::optional<IntervalTimer<C>> throwing;
		if (phase == 1) {
#ifdef WITH_RUST
			throwing.emplace(C::TPeriod{C::MONTH, C::NONE}, RustClockCallback{[](const void *, void *exception) noexcept { return RustClockRelayScheduledConsole(SessionThrow, exception); }, nullptr});
#else
			throwing.emplace(C::TPeriod{C::MONTH, C::NONE}, [](auto) { SessionThrow(); });
#endif
		}
		auto company = _current_company;
		auto tick = TimerGameTick::counter;
		bool caught = false;
		try { StateGameLoop(); } catch (const Script_FatalError &) { caught = true; }
		session_failure = 0;
		storage.StoreValue(1, 22);
		BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP, true);
		SessionCheck(phase == 1 ? "clock-exception" : "script-exception", caught && _current_company == company && E::date_fract == (phase == 1 ? 5 : 6) && TimerGameTick::counter == tick + (phase == 1 ? 0 : 1) && storage.GetValue(0) == (phase == 1 ? 11 : 0) && storage.GetValue(1) == (phase == 1 ? 22 : 0));
		SessionObserve(phase == 1 ? "clock-exception" : "script-exception");
	}
	session_storage = nullptr;
}
struct SessionScan final : NewGRFScanCallback {
	void OnNewGRFsScanned() override {
		++session_scans;
		SessionCheck("scan-reentry", !RequestNewGRFScan(nullptr));
		delete this;
	}
};
static bool SessionScenario(std::span<std::string_view> args)
{
	if (session_active) return true;
	if (args.size() != 3) return false;
	if (args[1] == "inspect") { SessionObserve("reloaded"); SessionCheck("reload-mode", _game_mode == GM_NORMAL); return true; }
	session_active = true;
	const std::string folder(args[2]);
	AutoRestoreBackup<bool> modal(_in_modal_progress, false), networking(_networking, false), dedicated(_network_dedicated, false), available(_network_available, false);
	_pause_mode = {}; _switch_mode = SM_NONE; _game_mode = GM_NORMAL;
	for (Company *c : Company::Iterate()) {
#ifdef WITH_RUST
		auto &f = c->Finances();
#else
		auto &f = *c;
#endif
		f.clear_limit = f.terraform_limit = f.tree_limit = f.build_object_limit = 0;
	}
	SessionObserve("before-normal"); StateGameLoop(); SessionSave(folder, "normal");
	CmdPause(DoCommandFlag::Execute, PauseMode::Normal, true);
	StateGameLoop(); SessionSave(folder, "paused");
	_in_modal_progress = true; StateGameLoop(); SessionObserve("modal-paused");
	_pause_mode = {}; StateGameLoop(); SessionObserve("modal"); _in_modal_progress = false;
	_settings_game.economy.minutes_per_calendar_year = 0;
	StateGameLoop(); SessionSave(folder, "frozen-calendar");
	_settings_game.economy.minutes_per_calendar_year = CalendarTime::DEF_MINUTES_PER_YEAR;
	SessionExceptions();
	_pause_mode = PauseMode::Normal;
	for (int scan = 0; scan < 2; scan++) {
		SessionCheck("scan-request", RequestNewGRFScan(new SessionScan()));
		SessionCheck("scan-busy", !RequestNewGRFScan(nullptr)); GameLoop();
	}
	SessionCheck("scan-replacement", session_scans == 2);
	_game_mode = GM_BOOTSTRAP; GameLoop(); SessionObserve("bootstrap"); _game_mode = GM_NORMAL;
	_settings_newgame = _settings_game;
	_settings_newgame.game_creation.map_x = _settings_newgame.game_creation.map_y = 6;
	_settings_newgame.game_creation.generation_seed = 12345;
	_settings_newgame.game_creation.starting_year = TimerGameCalendar::Year{1950};
	_settings_newgame.difficulty.max_no_competitors = 0;
	_settings_newgame.difficulty.industry_density = ID_FUND_ONLY;
	_settings_client.gui.pause_on_newgame = true;
	MakeNewgameSettingsLive(); SwitchToMode(SM_NEWGAME); SessionCheck("new-settings", TimerGameCalendar::year == 1950 && _settings_game.game_creation.generation_seed == 12345 && _pause_mode.Test(PauseMode::Normal)); SessionSave(folder, "newgame");
	_settings_game.game_creation.generation_seed = 777;
	_settings_game.game_creation.starting_year = TimerGameCalendar::Year{1960};
	_settings_client.gui.pause_on_newgame = false;
	SwitchToMode(SM_RESTARTGAME); SessionCheck("current-settings", TimerGameCalendar::year == 1960 && _settings_game.game_creation.generation_seed == 777); SessionSave(folder, "restart");
	SwitchToMode(SM_RELOADGAME); SessionSave(folder, "reload-random");
	SwitchToMode(SM_EDITOR); SessionCheck("editor-owner", _local_company == OWNER_NONE); _pause_mode = {}; SessionObserve("before-editor"); StateGameLoop(); SessionSave(folder, "editor");
	_file_to_saveload.SetMode(FIOS_TYPE_BMP, SLO_LOAD); _file_to_saveload.name = std::string(std::getenv("HOME")) + "/wide.bmp";
	SwitchToMode(SM_LOAD_HEIGHTMAP); SessionSave(folder, "editor-heightmap");
	SwitchToMode(SM_RELOADGAME); SessionSave(folder, "reload-heightmap");
	SwitchToMode(SM_START_HEIGHTMAP); SessionCheck("heightmap-new-settings", TimerGameCalendar::year == 1950 && _settings_game.game_creation.generation_seed == 12345); SessionSave(folder, "new-heightmap");
	SwitchToMode(SM_RESTART_HEIGHTMAP); SessionSave(folder, "restart-heightmap");
	_file_to_saveload.SetMode(FIOS_TYPE_SCENARIO, SLO_LOAD); _file_to_saveload.name = folder + "/editor.sav";
	SwitchToMode(SM_LOAD_SCENARIO); SessionSave(folder, "load-scenario");
	SwitchToMode(SM_RELOADGAME); SessionSave(folder, "reload-scenario");
	SessionCheck("missing-load", !SafeLoad("session-missing.sav", SLO_LOAD, DFT_GAME_FILE, GM_NORMAL, NO_DIRECTORY, nullptr) && _game_mode == GM_EDITOR);
	SessionSave(folder, "missing-load");
	SessionCheck("partial-load", !SafeLoad(std::string(std::getenv("HOME")) + "/partial.sav", SLO_LOAD, DFT_GAME_FILE, GM_NORMAL, NO_DIRECTORY, nullptr));
	SessionSave(folder, "partial-load");
	_file_to_saveload.SetMode(FIOS_TYPE_SCENARIO, SLO_LOAD); _file_to_saveload.name = folder + "/normal.sav";
	SwitchToMode(SM_LOAD_GAME); SessionSave(folder, "play-scenario");
	_file_to_saveload.SetMode(FIOS_TYPE_FILE, SLO_LOAD);
	SwitchToMode(SM_RELOADGAME); SessionSave(folder, "reload-save");
	SwitchToMode(SM_MENU); SessionSave(folder, "intro");
	_switch_mode = SM_EDITOR; GameLoop(); SessionSave(folder, "outer-editor");
	_switch_mode = SM_NEWGAME; GameLoop(); _pause_mode = {}; SessionSave(folder, "final");
	session_active = false;
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("session_scenario", SessionScenario);
}
