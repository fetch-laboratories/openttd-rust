/** @file threaded-console.cpp Fixture commands running under the driver's existing state mutex. */
#include "stdafx.h"
#include "vehicle-record-fixture.hpp"
#include "console_internal.h"
#include "console_func.h"
#include "video/video_driver.hpp"
#include "vehicle_base.h"
#include "vehicle_func.h"
#include "company_base.h"
#include "road_map.h"
#include "landscape_cmd.h"
#include "command_func.h"
#include "core/backup_type.hpp"
#include "gfx_func.h"
#include "openttd.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "table/strings.h"
#include <thread>

static std::thread::id game_thread;

static bool ThreadedStart(std::span<std::string_view>)
{
	game_thread = std::this_thread::get_id();
	if (!_pause_mode.Test(PauseMode::Normal)) std::abort();
	bool credits = true;
	for (const Company *c : Company::Iterate()) {
#ifdef WITH_RUST
		const auto &f = c->Finances();
#else
		const auto &f = *c;
#endif
		const auto &s = _settings_game.construction;
		credits &= f.terraform_limit == uint32_t(s.terraform_frame_burst) << 16;
		credits &= f.clear_limit == uint32_t(s.clear_frame_burst) << 16;
		credits &= f.tree_limit == uint32_t(s.tree_frame_burst) << 16;
		credits &= f.build_object_limit == uint32_t(s.build_object_frame_burst) << 16;
	}
	fmt::print(stderr, "THREAD506 credits {}\n", credits);
	if (!credits) std::abort();
	for (const Vehicle *v : Vehicle::Iterate()) {
		if (FIXTURE_GET(v, Type, type) != VEH_ROAD || !IsTileType(FIXTURE_GET(v, Tile, tile), MP_ROAD) || !IsNormalRoad(FIXTURE_GET(v, Tile, tile))) continue;
		const TileIndex tile = FIXTURE_GET(v, Tile, tile);
		const CompanyID company = FIXTURE_GET(v, Owner, owner);
		const auto occupied = EnsureNoVehicleOnGround(tile);
		fmt::print(stderr, "THREAD506 game tile {} occupied {} error {}\n", tile.base(), occupied.Failed(), occupied.GetErrorMessage());
		if (occupied.Succeeded() || occupied.GetErrorMessage() != STR_ERROR_ROAD_VEHICLE_IN_THE_WAY) std::abort();
		VideoDriver::GetInstance()->QueueOnMainThread([tile, company] {
			Backup<CompanyID> owner(_current_company, company);
			const bool split = game_thread != std::this_thread::get_id();
			const auto occupied = EnsureNoVehicleOnGround(tile);
			const auto clear = Command<CMD_LANDSCAPE_CLEAR>::Do({}, tile);
			fmt::print(stderr, "THREAD506 main split {} tile {} occupied {} error {} clear {} error {}\n", split, tile.base(), occupied.Failed(), occupied.GetErrorMessage(), clear.Failed(), clear.GetErrorMessage());
			if (!split || occupied.Succeeded() || clear.Succeeded() || occupied.GetErrorMessage() != STR_ERROR_ROAD_VEHICLE_IN_THE_WAY || clear.GetErrorMessage() != STR_ERROR_ROAD_VEHICLE_IN_THE_WAY) std::abort();
			owner.Restore();
			ChangeGameSpeed(true);
			IConsoleCmdExec("unpause");
		});
		return true;
	}
	std::abort();
}

static bool ThreadedStop(std::span<std::string_view>)
{
	fmt::print(stderr, "THREAD506 stop game {} calendar {} fract {} economy {} fract {}\n", game_thread == std::this_thread::get_id(), TimerGameCalendar::date.base(), TimerGameCalendar::date_fract, TimerGameEconomy::date.base(), TimerGameEconomy::date_fract);
	return true;
}

extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("threaded_start", ThreadedStart);
	IConsole::CmdRegister("threaded_stop", ThreadedStop);
}
