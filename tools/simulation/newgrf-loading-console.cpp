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
#include "industry_cmd.h"
#include "newgrf_industries.h"
#include "newgrf_industrytiles.h"
#include "industry_map.h"
#include "industrytype.h"
#include "rail_type.h"
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
#include "newgrf_townname.h"
#include "strings_func.h"
#include "town.h"
#include "town_cmd.h"
#include "townname_func.h"
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
static std::string Hex(std::string_view text)
{
	std::string result;
	for (char c : text) result += fmt::format("{:02X}", uint8_t(c));
	return result;
}
/** Generator lists, style lookups and generated name bytes of the ActionF fixtures. */
static void GrfTownNames(const char *phase, bool loaded)
{
	auto names = GetGRFTownNameList();
	fmt::print(stderr, "LOADING townname {} styles {}\n", phase, names.size());
	for (uint gen = 0; gen <= names.size(); gen++) {
		TownNameParams par(static_cast<uint8_t>(BUILTIN_TOWNNAME_GENERATOR_COUNT + gen));
		StringID name = GetGRFTownNameName(gen);
		fmt::print(stderr, "LOADING townname {} style {} {:08X} {} {} {}\n", phase, gen, GetGRFTownNameId(gen), GetGRFTownNameType(gen), gen < names.size() && name == names[gen], Hex(GetString(name)));
		for (uint32_t k = 0; k < 40; k++) {
			uint32_t seed = k < 4 ? std::array<uint32_t, 4>{0, 1, 0x80000000, UINT32_MAX}[k] : k * 0x9E3779B9;
			fmt::print(stderr, "LOADING townname {} name {} {:08X} {}\n", phase, gen, seed, Hex(GetTownName(&par, seed)));
		}
	}
	if (!loaded) return;
	Require(names.size() == 4 && GetGRFTownNameId(0) == ID("TNA5") && GetGRFTownNameId(2) == ID("TNB5") && GetGRFTownNameId(3) == ID("TNS5") && GetGRFTownNameType(3) == 0, "townname-style-order");
	Require(GetGRFTownNameId(4) == 0 && GetGRFTownNameType(4) == SPECSTR_TOWNNAME_START && GetGRFTownNameName(4) == STR_UNDEFINED, "townname-fallback");
	TownNameParams par(static_cast<uint8_t>(0));
	par.grfid = ID("TNA5");
	par.type = 0;
	Require(GetTownName(&par, 0) == "Y Hill" && GetTownName(&par, 0xFFFFFFFF) == "Alton Hill" && GetTownName(&par, 0x00030000) == "Beton Hill", "townname-repeated-definition");
	par.type = 1;
	Require(GetTownName(&par, 0) == "Q" && GetTownName(&par, 0xFFFF) == "Y Hill", "townname-recursive-final");
	par.grfid = ID("TNB5");
	par.type = 0;
	Require(GetTownName(&par, 0) == "Gam\u20ac", "townname-second-grf");
	par.grfid = ID("TNX5");
	Require(GetTownName(&par, 0).empty(), "townname-invalid-reference-deleted");
	GRFConfig *bad = GetGRFConfig(ID("TNX5"));
	Require(bad->status == GCS_DISABLED && bad->errors.size() == 1 && bad->errors.front().message == STR_NEWGRF_ERROR_INVALID_ID, "townname-invalid-reference");
	GRFConfig *truncated = GetGRFConfig(ID("TNS5"));
	par.grfid = ID("TNS5");
	Require(truncated->status == GCS_DISABLED && truncated->errors.size() == 1 && truncated->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS && GetTownName(&par, 0) == "Shortort", "townname-partial-read");
}
/** Found one town per NewGRF style through the production name generator. */
static void GrfTownNameTowns()
{
	AutoRestoreBackup company(_current_company, OWNER_DEITY);
	auto names = GetGRFTownNameList();
	for (uint gen = 0; gen < names.size(); gen++) {
		AutoRestoreBackup style(_settings_game.game_creation.town_name, static_cast<uint8_t>(BUILTIN_TOWNNAME_GENERATOR_COUNT + gen));
		uint32_t parts = 0;
		bool generated = GenerateTownName(_random, &parts);
		TownID id = TownID::Invalid();
		for (uint32_t n = 1 + 997 * gen; n < Map::Size() && id == TownID::Invalid(); n += 31) {
			TileIndex tile(n);
			if (!IsTileType(tile, MP_CLEAR) || GetTileSlope(tile) != SLOPE_FLAT) continue;
			id = std::get<TownID>(Command<CMD_FOUND_TOWN>::Do(DoCommandFlag::Execute, tile, TSZ_SMALL, false, TL_ORIGINAL, false, parts, {}));
		}
		Require(generated && id != TownID::Invalid(), "townname-found");
		const Town *t = Town::Get(id);
		fmt::print(stderr, "LOADING townname town {} {} {:08X} {} {:08X} {}\n", gen, id.base(), t->townnamegrfid, t->townnametype, t->townnameparts, Hex(GetTownName(t)));
	}
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
#ifdef WITH_RUST
		OpenTTDGrfCursor reader{bytes.begin(), bytes.size(), 0};
		bool success = openttd_rust_action2_parse(&GetRustAction2Services(), &GetRustGrfLoadingServices(), &reader);
		return std::pair{!success, reader.len - reader.pos};
#else
		ByteReader reader(bytes.begin(), bytes.size());
		try {
			NewSpriteGroup(reader);
			return std::pair{false, reader.Remaining()};
		}
		catch (const OTTDByteReaderSignal &) { return std::pair{true, reader.Remaining()}; }
#endif
	};
	Require(!parse({1, 200, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127}).first, "action2-first-publication");
	const auto *published = _cur_gps.spritegroups[200];
	const auto *first = dynamic_cast<const DeterministicSpriteGroup *>(published);
	Require(first != nullptr && first->ranges.size() == 1, "action2-first-range");
	size_t before = SpriteGroup::GetNumItems();
	size_t next = SpriteGroup::GetPoolSize();
	auto failed = parse({1, 200, 0x81, 0x0C, 0, 255, 1, 4, 128});
	Require(failed.first && failed.second == 0 && _cur_gps.spritegroups[200] == published && SpriteGroup::GetNumItems() == before + 2, "action2-partial-read");
	const auto *partial = dynamic_cast<const DeterministicSpriteGroup *>(SpriteGroup::Get(SpriteGroupID(next)));
	const auto *cached = dynamic_cast<const CallbackResultSpriteGroup *>(SpriteGroup::Get(SpriteGroupID(next + 1)));
	Require(partial != nullptr && partial->adjusts.size() == 1 && partial->adjusts[0].and_mask == 255 && partial->ranges.empty() && cached != nullptr && cached->result == 4, "action2-partial-allocation-order");
	before = SpriteGroup::GetNumItems();
	Require(!parse({1, 201, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127}).first, "action2-repeat-parse");
	const auto *repeat = dynamic_cast<const DeterministicSpriteGroup *>(_cur_gps.spritegroups[201]);
	Require(repeat->ranges[0].result.group == first->ranges[0].result.group && SpriteGroup::GetNumItems() == before + 1, "action2-interned");
	ResetAction2Callbacks(true);
	before = SpriteGroup::GetNumItems();
	Require(!parse({1, 202, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127}).first && SpriteGroup::GetNumItems() == before + 2, "action2-reset-cache");
	Require(!parse({10, 200, 3}).first && _cur_gps.spritegroups[200] == nullptr, "action2-explicit-null");
	AutoRestoreBackup version(_cur_gps.grffile->grf_version, uint8_t(7));
	Require(!parse({1, 203, 0x81, 0x0C, 0, 255, 1, 3, 255, 0, 255, 255, 127}).first, "action2-legacy-parse");
	const auto *legacy = static_cast<const DeterministicSpriteGroup *>(_cur_gps.spritegroups[203]);
	Require(static_cast<const CallbackResultSpriteGroup *>(legacy->ranges[0].result.group)->result == 3, "action2-legacy-ffxx");
	_cur_gps.grffile->grf_version = 8;
	Require(!parse({1, 204, 0x81, 0x0C, 0, 255, 1, 3, 255, 0, 255, 255, 127}).first, "action2-modern-parse");
	const auto *modern = static_cast<const DeterministicSpriteGroup *>(_cur_gps.spritegroups[204]);
	Require(static_cast<const CallbackResultSpriteGroup *>(modern->ranges[0].result.group)->result == 0x7F03, "action2-modern-ffxx");
	before = SpriteGroup::GetNumItems();
	failed = parse({9, 205, 1, 1, 0, 0});
	const auto *partial_layout = dynamic_cast<const TileLayoutSpriteGroup *>(SpriteGroup::Get(SpriteGroupID(SpriteGroup::GetPoolSize() - 1)));
	Require(failed.first && failed.second == 1 && SpriteGroup::GetNumItems() == before + 1 && partial_layout != nullptr && partial_layout->dts.seq.size() == 1 && partial_layout->dts.ground.sprite == 1 && partial_layout->dts.ground.pal == 0 && _cur_gps.spritegroups[205] == nullptr, "action2-partial-layout");
	GRFConfig errors("action2-parser-probe");
	AutoRestoreBackup config(_cur_gps.grfconfig, &errors);
	AutoRestoreBackup skip(_cur_gps.skip_sprites, 0);
	Require(!parse({10, 206, 2, 17}).first && errors.status == GCS_DISABLED && errors.errors.back().data == "too many inputs (max 16)", "action2-production-count-error");
	_cur_gps.skip_sprites = 0;
	_cur_gps.nfo_line++;
	Require(!parse({10, 207, 2, 2, 0, 9, 0}).first && errors.errors.back().data == "duplicate input cargo" && _cur_gps.spritegroups[207] == nullptr, "action2-production-duplicate-error");
	_cur_gps.skip_sprites = 0;
	Require(!parse({10, 208, 2, 1, 255, 9, 0, 0}).first, "action2-invalid-cargo-parse");
	const auto *invalid = static_cast<const IndustryProductionSpriteGroup *>(_cur_gps.spritegroups[208]);
	Require(invalid->version == 255 && invalid->cargo_input[0] == INVALID_CARGO && invalid->subtract_input[0] == 9, "action2-invalid-cargo-version");
	_cur_gps.nfo_line++;
	Require(!parse({9, 209, 0x41, 1, 0, 0, 0, 16, 0}).first && errors.errors.back().message == STR_NEWGRF_ERROR_INVALID_SPRITE_LAYOUT && _cur_gps.spritegroups[209] == nullptr, "action2-layout-flag-error");

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
	Require(graph != nullptr && std::ranges::any_of(graph->ranges, [](const auto &r) { return r.low == 21 && r.high == 32 && dynamic_cast<const DeterministicSpriteGroup *>(r.result.group) != nullptr; }), "action2-file-graph");
	const auto &range = *std::ranges::find_if(graph->ranges, [](const auto &r) { return r.low == 21; });
	const auto *subroutine = dynamic_cast<const DeterministicSpriteGroup *>(range.result.group);
	const auto *calculated = dynamic_cast<const DeterministicSpriteGroup *>(subroutine->adjusts[0].subroutine);
	const auto *real = dynamic_cast<const RealSpriteGroup *>(subroutine->ranges[0].result.group);
	const auto *random = dynamic_cast<const RandomizedSpriteGroup *>(graph->ranges[1].result.group);
	Require(graph->size == DSG_SIZE_DWORD && subroutine->adjusts[0].divmod_val == 1 && calculated->size == DSG_SIZE_WORD && calculated->ranges.empty() && calculated->default_result.calculated_result && real != nullptr && real->loaded.size() == 2 && real->loading.size() == 1 && calculated->error_group == real->loading[0] && subroutine->default_result.group == real->loading[0] && graph->default_result.group == nullptr && random != nullptr && random->groups.size() == 3 && random->triggers == 2, "action2-normalized-graph");
	Require(Engine::Get(id)->DetermineCapacity(nullptr) == 55, "action2-file-capacity");
	IndustryGfx gfx = _industile_mngr.GetID(0, file->grfid);
	Industry *context = *Industry::Iterate().begin();
	IndustryTileResolverObject tile_resolver(gfx, context->location.tile, context);
	const auto *layout = tile_resolver.Resolve<TileLayoutSpriteGroup>();
	Require(layout != nullptr && layout->dts.seq.size() == 1 && layout->dts.registers.size() == 2, "action2-layout-parsed");
	uint8_t stage = 3;
	auto processed = layout->ProcessRegisters(tile_resolver, &stage);
	Require(stage == 0 && processed.GetLayout().GetSequence().size() == 1 && processed.GetLayout().ground.sprite == 1, "action2-layout-consumer");
	for (uint8_t variant = 0; variant < 3; variant++) {
		const auto *spec = GetIndustrySpec(_industry_mngr.GetID(variant, file->grfid));
		const auto *wrapper = dynamic_cast<const DeterministicSpriteGroup *>(spec->grf_prop.GetSpriteGroup(true));
		const auto *production = dynamic_cast<const IndustryProductionSpriteGroup *>(wrapper->ranges[0].result.group);
		Require(production != nullptr && production->version == variant && (variant != 0 || (production->subtract_input[0] == -2 && production->add_output[0] == 7)), "action2-production-parsed");
		if (reload) {
			bool found = false;
			for (const Industry *industry : Industry::Iterate()) found |= industry->type == _industry_mngr.GetID(variant, file->grfid) && industry->produced[0].waiting >= 7;
			Require(found, "action2-industry-reload");
		}
	}
	Action2PartialRead();
	if (!reload) Require(GetGRFConfig(ID("MISS"))->status == GCS_NOT_FOUND, "missing");
	fmt::print(stderr, "LOADING state {} {} {} {} {}\n", id.base(), partial_id.base(), file->GetParam(3), broken->errors.front().nfo_line, broken->errors.front().param_value[0]);
	fmt::print(stderr, "LOADING {} true\n", reload ? "reload" : "complete");
}
static void Action2Consumers()
{
	const GRFFile *file = GetFileByGRFID(ID("R471"));
	AutoRestoreBackup company(_current_company, CompanyID(0));
	const StationSpec *station = file->stations[0].get();
	Require(GetStationCallback(CBID_STATION_AVAILABILITY, 0, 0, station, nullptr, INVALID_TILE) == 1, "action2-station-availability");
	bool station_built = false;
	for (uint32_t n = 1; n < Map::Size() && !station_built; ++n) {
		TileIndex tile(n);
		if (!IsTileType(tile, MP_CLEAR) || GetTileSlope(tile) != SLOPE_FLAT || TileHeight(tile) == 0) continue;
		auto cost = Command<CMD_BUILD_RAIL_STATION>::Do({DoCommandFlag::Execute, DoCommandFlag::Auto}, tile, RAILTYPE_RAIL, AXIS_X, 1, 1, station->class_index, station->index, NEW_STATION, true);
		station_built = cost.Succeeded();
	}
	Require(station_built, "action2-station-command");
	AutoRestoreBackup founder(_current_company, OWNER_DEITY);
	AutoRestoreBackup multiple(_settings_game.economy.multiple_industry_per_town, true);
	for (uint8_t variant = 0; variant < 3; variant++) {
		IndustryType type = _industry_mngr.GetID(variant, file->grfid);
		bool built = false;
		for (uint32_t n = 1; n < Map::Size() && !built; n += 17) {
			TileIndex tile(n);
			if (!IsTileType(tile, MP_CLEAR) || GetTileSlope(tile) != SLOPE_FLAT || TileHeight(tile) == 0) continue;
			auto cost = Command<CMD_BUILD_INDUSTRY>::Do(DoCommandFlag::Execute, tile, type, 0, true, 485 + variant);
			if (!cost.Succeeded()) continue;
			for (Industry *industry : Industry::Iterate()) {
				if (industry->type != type) continue;
				auto before = industry->produced[0].waiting;
				IndustryProductionCallback(industry, 1);
				Require(industry->produced[0].waiting == before + 7, fmt::format("action2-production-v{}", variant).c_str());
				fmt::print(stderr, "LOADING production {} {} {} {}\n", variant, type, industry->produced[0].waiting, industry->random);
				built = true;
				break;
			}
		}
		Require(built, "action2-industry-command");
	}
}
static bool LoadingScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "inspect") { Inspect(true); GrfTownNames("inspect", false); return true; }
	if (args.size() < 3) return false;
	SetRandomSeed(0x47100001);
	_grfconfig.clear();
	for (const char *name : {"main", "graph-broken", "broken", "victim", "fatal", "before", "after", "names-a", "names-bad", "names-b", "names-short"}) {
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
	GrfTownNames("first", true);
	ReloadNewGRFData();
	Inspect(false);
	GrfTownNames("reload", true);
	GrfTownNameTowns();
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
		TriggerVehicleRandomisation(Vehicle::Get(vehicle), VehicleRandomTrigger::Depot);
		Require(!Vehicle::Get(vehicle)->waiting_random_triggers.Test(VehicleRandomTrigger::Depot), "action2-random-trigger");
		fmt::print(stderr, "LOADING randomized {} {} {} {}\n", Vehicle::Get(vehicle)->random_bits, Vehicle::Get(vehicle)->waiting_random_triggers.base(), _random.state[0], _random.state[1]);
		built = true;
		break;
	}
	Require(built, "usable-definition");
	Action2Consumers();
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
