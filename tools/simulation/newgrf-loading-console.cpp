/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf-loading-console.cpp Production loading from deterministic on-disk GRFs. */
#include "stdafx.h"
#include "console_internal.h"
#include "newgrf/newgrf_act2.cpp"
#include "newgrf_station.h"
#include "station_cmd.h"
#include "landscape.h"
#include "tile_map.h"
#include "vehicle_base.h"
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
static void ResetAction2Callbacks(bool final)
{
#ifdef WITH_RUST
	openttd_rust_action2_reset(final);
#else
	ResetCallbacks(final);
#endif
}
static void Action2PartialRead()
{
	auto groups = _cur_gps.spritegroups;
	AutoRestoreBackup file(_cur_gps.grffile, GetFileByGRFID(ID("R471")));
	AutoRestoreBackup line(_cur_gps.nfo_line, 485U);
	ResetAction2Callbacks(false);
	auto parse = [](std::initializer_list<uint8_t> bytes) {
		ByteReader reader(bytes.begin(), bytes.size());
		try { NewSpriteGroup(reader); return std::pair{false, reader.Remaining()}; }
		catch (const OTTDByteReaderSignal &) { return std::pair{true, reader.Remaining()}; }
	};
	Require(!parse({1, 200, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127}).first, "action2-first-publication");
	const auto *published = _cur_gps.spritegroups[200];
	const auto *first = dynamic_cast<const DeterministicSpriteGroup *>(published);
	Require(first != nullptr && first->ranges.size() == 1, "action2-first-range");
	size_t before = SpriteGroup::GetNumItems();
	auto failed = parse({1, 200, 0x81, 0x0C, 0, 255, 1, 4, 128});
	Require(failed.first && failed.second == 0 && _cur_gps.spritegroups[200] == published && SpriteGroup::GetNumItems() == before + 2, "action2-partial-read");
	before = SpriteGroup::GetNumItems();
	Require(!parse({1, 201, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127}).first, "action2-repeat-parse");
	const auto *repeat = dynamic_cast<const DeterministicSpriteGroup *>(_cur_gps.spritegroups[201]);
	Require(repeat->ranges[0].result.group == first->ranges[0].result.group && SpriteGroup::GetNumItems() == before + 1, "action2-interned");
	ResetAction2Callbacks(true);
	before = SpriteGroup::GetNumItems();
	Require(!parse({1, 202, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127}).first && SpriteGroup::GetNumItems() == before + 2, "action2-reset-cache");
	Require(!parse({10, 200, 3}).first && _cur_gps.spritegroups[200] == nullptr, "action2-explicit-null");
	ResetAction2Callbacks(true);
	_cur_gps.spritegroups = groups;
}
static void Inspect(bool reload)
{
	Require(_cur_gps.file == nullptr && _cur_gps.grffile == nullptr && _cur_gps.grfconfig == nullptr && _cur_gps.nfo_line == 0 && _cur_gps.skip_sprites == 0 && std::ranges::all_of(_cur_gps.spritegroups, [](const auto *group) { return group == nullptr; }), "execution-restored");
	GRFConfig *main = GetGRFConfig(ID("R471"));
	GRFFile *file = GetFileByGRFID(ID("R471"));
	Require(main != nullptr && file != nullptr && main->status == GCS_ACTIVATED, "active");
	Require(file->GetParam(0) == 77 && file->GetParam(1) == 123 && file->GetParam(2) == 2, "d6-jumps");
	Require(file->GetParam(31) == 66 && file->GetParam(32) == 55 && file->GetParam(35) == 0x201, "patch-flags-stages");
	Require(file->GetParam(40) == 1 && file->GetParam(41) == 0xC0000000 && file->GetParam(42) == 0xFFFFFFFC && file->GetParam(43) == 17, "arithmetic");
	fmt::print(stderr, "LOADING patch-map {}\n", file->GetParam(30));
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
	GRFConfig *graph_broken = GetGRFConfig(ID("B485"));
	Require(graph_broken != nullptr && graph_broken->status == GCS_DISABLED && graph_broken->errors.size() == 1 && graph_broken->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "action2-file-truncated");
	const auto *graph = dynamic_cast<const DeterministicSpriteGroup *>(Engine::Get(id)->grf_prop.GetSpriteGroup(CargoGRFFileProps::SG_DEFAULT));
	Require(graph != nullptr && graph->ranges.size() == 3 && graph->ranges[0].low == 0 && graph->ranges[0].high == 20 && graph->ranges[1].low == 21 && graph->ranges[1].high == 32, "action2-file-graph");
	Require(Engine::Get(id)->DetermineCapacity(nullptr) == 55, "action2-file-capacity");
	Action2PartialRead();
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
	for (const char *name : {"main", "graph-broken", "broken", "victim", "fatal", "before", "after"}) {
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
		Require(capacity == 55 && Vehicle::Get(vehicle)->cargo_cap == 55, "action2-build-capacity");
		auto [refit, refit_capacity, refit_mail, refit_cargos] = Command<CMD_REFIT_VEHICLE>::Do(DoCommandFlag::Execute, vehicle, Vehicle::Get(vehicle)->cargo_type, 1, false, true, 1);
		Require(refit.Succeeded() && refit_capacity == 55, "action2-refit-capacity");
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
