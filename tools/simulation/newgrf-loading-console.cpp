/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf-loading-console.cpp Production loading from deterministic on-disk GRFs. */
#include "stdafx.h"
#include "console_internal.h"
#include "command_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "depot_base.h"
#include "depot_map.h"
#include "engine_base.h"
#include "engine_func.h"
#include "newgrf.h"
#include "newgrf_config.h"
#include "newgrf_engine.h"
#include "newgrf/newgrf_internal.h"
#include "settings_type.h"
#include "saveload/saveload.h"
#include "vehicle_cmd.h"
#include "table/strings.h"

static uint32_t ID(const char *text)
{
	return uint32_t(uint8_t(text[0])) | uint32_t(uint8_t(text[1])) << 8 | uint32_t(uint8_t(text[2])) << 16 | uint32_t(uint8_t(text[3])) << 24;
}
static void Require(bool value, const char *name)
{
	fmt::print(stderr, "LOADING case {} {}\n", name, value);
	if (!value) std::abort();
}
static void Inspect(bool reload)
{
	GRFConfig *main = GetGRFConfig(ID("R471"));
	GRFFile *file = GetFileByGRFID(ID("R471"));
	Require(main != nullptr && file != nullptr && main->status == GCS_ACTIVATED, "active");
	Require(file->GetParam(0) == 77 && file->GetParam(1) == 123 && file->GetParam(2) == 2, "d6-jumps");
	Require(file->GetParam(22) == 0xFFFFFFFE, "action6-carry-reuse");
	Require(file->GetParam(3) != 0 && file->GetParam(4) == UINT32_MAX, "grm");
	GRFConfig *broken = GetGRFConfig(ID("B471"));
	Require(broken != nullptr && broken->status == GCS_DISABLED && broken->errors.size() == 1 && broken->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "bounds");
	const GRFFile *partial = GetFileByGRFID(ID("B471"));
	Require(partial != nullptr && partial->GetParam(0) == 21, "partial-parameter");
	EngineID id = GetNewEngineID(file, VEH_ROAD, 0);
	Require(id != EngineID::Invalid() && Engine::Get(id)->VehInfo<RoadVehicleInfo>().max_speed == 124 && Engine::Get(id)->VehInfo<RoadVehicleInfo>().capacity == 33, "engine");
	EngineID partial_id = GetNewEngineID(partial, VEH_ROAD, 1);
	Require(partial_id != EngineID::Invalid() && Engine::Get(partial_id)->VehInfo<RoadVehicleInfo>().max_speed == 88, "partial-native-parser");
	Require(GetGRFConfig(ID("V471"))->status == GCS_DISABLED, "inhibition");
	GRFConfig *fatal = GetGRFConfig(ID("F471"));
	Require(fatal->status == GCS_DISABLED && fatal->errors.size() == 1 && fatal->errors.front().message == STR_NEWGRF_ERROR_GRM_FAILED, "fatal-grm");
	Require(GetGRFConfig(ID("Z471"))->status == GCS_DISABLED && GetGRFConfig(ID("Z471"))->errors.empty(), "skip-before-action8");
	Require(GetGRFConfig(ID("E471"))->status == GCS_ACTIVATED && GetFileByGRFID(ID("E471"))->param.empty(), "skip-after-action8");
	if (!reload) Require(GetGRFConfig(ID("MISS"))->status == GCS_NOT_FOUND, "missing");
	fmt::print(stderr, "LOADING state {} {} {} {} {}\n", id.base(), partial_id.base(), file->GetParam(3), broken->errors.front().nfo_line, broken->errors.front().param_value[0]);
	fmt::print(stderr, "LOADING {} true\n", reload ? "reload" : "complete");
}
static bool LoadingScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "inspect") { Inspect(true); return true; }
	if (args.size() < 3) return false;
	SetRandomSeed(0x47100001);
	_grfconfig.clear();
	for (const char *name : {"main", "broken", "victim", "fatal", "before", "after"}) {
		auto c = std::make_unique<GRFConfig>(std::string(args[2]) + "/" + name + ".grf");
		Require(FillGRFDetails(*c, false, NO_DIRECTORY), "file-scan");
		c->flags.Reset(GRFConfigFlag::InitOnly);
		_grfconfig.push_back(std::move(c));
	}
	auto missing = std::make_unique<GRFConfig>(std::string(args[2]) + "/absent.grf");
	missing->ident.grfid = ID("MISS");
	_grfconfig.push_back(std::move(missing));
	_settings_game.vehicle.dynamic_engines = false;
	ReloadNewGRFData();
	Inspect(false);
	ReloadNewGRFData();
	Inspect(false);
	EngineID id = GetNewEngineID(GetFileByGRFID(ID("R471")), VEH_ROAD, 0);
	bool built = false;
	for (const Depot *depot : Depot::Iterate()) {
		if (GetDepotVehicleType(depot->xy) != VEH_ROAD) continue;
		AutoRestoreBackup company(_current_company, GetTileOwner(depot->xy));
		auto [cost, vehicle, capacity, mail, cargo] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, depot->xy, id, false, INVALID_CARGO, INVALID_CLIENT_ID);
		if (!cost.Succeeded()) continue;
		fmt::print(stderr, "LOADING build {} {} {}\n", cost.Succeeded(), vehicle.base(), capacity);
		built = true;
		break;
	}
	Require(built, "usable-definition");
	fmt::print(stderr, "LOADING rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	std::erase_if(_grfconfig, [](const auto &c) { return c->status == GCS_NOT_FOUND; });
	if (args.size() > 3) Require(SaveOrLoad(std::string(args[3]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, "save");
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("loading_scenario", LoadingScenario);
}
