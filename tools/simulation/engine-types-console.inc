/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** Native transient registry inputs; shared by both actual game implementations. */
#include "rail.h"
#include "rail_cmd.h"
#include "road.h"
#include "road_func.h"
#include "road_cmd.h"
#include "clear_map.h"
#include "landscape.h"
#include "town.h"
#include "openttd.h"
#include "genworld.h"
#include "core/random_func.hpp"
extern RailTypeInfo _railtypes[RAILTYPE_END];
extern RoadTypeInfo _roadtypes[ROADTYPE_END];
#ifdef WITH_RUST
extern RoadTypes &_roadtypes_road, &_roadtypes_tram;
#else
extern RoadTypes _roadtypes_road, _roadtypes_tram;
#endif
static std::array<RailTypeInfo, RAILTYPE_END> TypeOriginalRail;
static std::array<RoadTypeInfo, ROADTYPE_END> TypeOriginalRoad;
static RoadTypes TypeOriginalRoadMask, TypeOriginalTramMask;
static bool TypesInstalled = false;
static void TypeRequire(bool value, const char *label)
{
	fmt::print(stderr, "ENGINE types-case {} {}\n", label, value);
	if (!value) FixtureAbort(__LINE__);
}
static void InstallTypeInputs()
{
	std::copy(std::begin(_railtypes), std::end(_railtypes), TypeOriginalRail.begin());
	std::copy(std::begin(_roadtypes), std::end(_roadtypes), TypeOriginalRoad.begin());
	TypeOriginalRoadMask = _roadtypes_road; TypeOriginalTramMask = _roadtypes_tram;
	for (uint i = 4; i < RAILTYPE_END; ++i) if (AllocateRailType(0x54520000 + i) != i) FixtureAbort(__LINE__);
	for (uint i = 2; i < ROADTYPE_END; ++i) if (AllocateRoadType(0x54520000 + i, i == 61 ? RTT_TRAM : RTT_ROAD) != i) FixtureAbort(__LINE__);
	const auto date = TimerGameCalendar::date;
	/* Default type zero otherwise date-introduces itself from an empty mask.
	 * These declared inputs isolate engine publication and explicit mask seeds. */
	_railtypes[RAILTYPE_RAIL].introduction_date = CalendarTime::INVALID_DATE;
	_roadtypes[ROADTYPE_ROAD].introduction_date = CalendarTime::INVALID_DATE;
	for (uint i = 50; i < 64; ++i) {
		RailTypeInfo &r = _railtypes[i];
		r.introduction_date = date; r.introduction_required_railtypes = RailTypes{RAILTYPE_RAIL};
		if (i < 63) { RoadTypeInfo &d = _roadtypes[i]; d.introduction_date = date; d.introduction_required_roadtypes = RoadTypes{ROADTYPE_ROAD}; }
	}
	for (uint i = 60; i < 63; ++i) _railtypes[i].introduction_required_railtypes = RailTypes{static_cast<RailType>(i + 1)};
	for (uint i = 60; i < 62; ++i) _roadtypes[i].introduction_required_roadtypes = RoadTypes{static_cast<RoadType>(i + 1)};
	for (uint i : {50, 51, 52, 53}) {
		auto d = i == 50 ? CalendarTime::INVALID_DATE : i == 51 ? TimerGameCalendar::Date{0} : i == 52 ? CalendarTime::MAX_DATE : date + 1;
		_railtypes[i].introduction_date = d; _roadtypes[i].introduction_date = d;
	}
	_railtypes[54].label = 0; _roadtypes[54].label = 0;
	for (uint i : {55, 56, 57}) {
		_railtypes[i].introduction_required_railtypes = RailTypes{static_cast<RailType>(i == 55 ? 56 : i == 56 ? 55 : 49)};
		_roadtypes[i].introduction_required_roadtypes = RoadTypes{static_cast<RoadType>(i == 55 ? 56 : i == 56 ? 55 : 49)};
	}
	_railtypes[62].flags.Set(RailTypeFlag::Hidden);
	_roadtypes[58].flags.Set(RoadTypeFlag::Hidden);
	_roadtypes[59].flags.Set(RoadTypeFlag::Hidden); _roadtypes[59].flags.Set(RoadTypeFlag::TownBuild);
	InitRailTypes(); InitRoadTypes();
	TypesInstalled = true;
}
extern "C" void __real__Z18CopyTempEngineDatav();
extern "C" void __wrap__Z18CopyTempEngineDatav()
{
	__real__Z18CopyTempEngineDatav();
	if (std::getenv("OPENTTD_ENGINE_TYPES") == nullptr) return;
	/* Actual load phase: sprites/specs and saved lifecycle are available, but
	 * AfterLoadGame has not yet rebuilt the unsaved Company type masks. */
	InstallTypeInputs();
	fmt::print(stderr, "ENGINE types-loadphase true\n");
}
static auto TypeTileState(Tile t)
{
	return std::tuple{t.type(), t.height(), t.m1(), t.m2(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8()};
}
static bool TypeSameRng(const SavedRandomSeeds &a, const SavedRandomSeeds &b)
{
	return a.random.state[0] == b.random.state[0] && a.random.state[1] == b.random.state[1] && a.interactive_random.state[0] == b.interactive_random.state[0] && a.interactive_random.state[1] == b.interactive_random.state[1];
}
static TileIndex TypeFlatArea(uint size = 10)
{
	for (uint y = 2; y + size < Map::SizeY(); ++y) for (uint x = 2; x + size < Map::SizeX(); ++x) {
		bool clear = true;
		for (uint dy = 0; dy < size; ++dy) for (uint dx = 0; dx < size; ++dx) {
			auto type = GetTileType(TileXY(x + dx, y + dy)); if (type != MP_CLEAR && type != MP_TREES) clear = false;
		}
		if (!clear) continue;
		for (uint dy = 0; dy < size; ++dy) for (uint dx = 0; dx < size; ++dx) {
			Tile tile{TileXY(x + dx, y + dy)}; SetTileHeight(tile, 1); MakeClear(tile, CLEAR_GRASS, 3);
		}
		return TileXY(x + 2, y + 2);
	}
	FixtureAbort(__LINE__);
}
static void TypeConstruction(Company *c, TileIndex tile, int kind, uint8_t type, bool success, const char *label)
{
	for (DoCommandFlags flags : {DoCommandFlags{DoCommandFlag::Auto}, DoCommandFlags{DoCommandFlag::Auto, DoCommandFlag::Execute}}) {
		auto before = TypeTileState(tile); auto rail = c->infrastructure.rail; auto road = c->infrastructure.road;
		SavedRandomSeeds rng, after; SaveRandomSeeds(&rng);
		auto result = kind == 0 ? Command<CMD_BUILD_SINGLE_RAIL>::Do(flags, tile, static_cast<RailType>(type), TRACK_X, false) : Command<CMD_BUILD_ROAD>::Do(flags, tile, ROAD_X, static_cast<RoadType>(type), DRD_NONE, _current_company == OWNER_TOWN ? TownID{0} : TownID::Invalid());
		SaveRandomSeeds(&after);
		bool unchanged = TypeTileState(tile) == before && rail == c->infrastructure.rail && road == c->infrastructure.road;
		fmt::print(stderr, "ENGINE types-command {} {} {} {} {} {} {}\n", label, flags.Test(DoCommandFlag::Execute), result.Succeeded(), result.GetCost(), result.GetErrorMessage(), unchanged, TypeSameRng(rng, after));
		if (result.Succeeded() != success || !TypeSameRng(rng, after) || ((!success || !flags.Test(DoCommandFlag::Execute)) && !unchanged)) FixtureAbort(__LINE__);
	}
}
static void TypeEngineInputs(Company *c)
{
	struct Saved { Engine *e; LandscapeTypes climates; CompanyMask avail; TimerGameCalendar::Date intro; EngineFlags flags; };
	std::vector<Saved> saved;
	for (Engine *e : Engine::Iterate()) { saved.push_back({e, e->info.climates, e->company_avail, e->intro_date, e->flags}); e->info.climates = {}; e->flags = {}; }
	Engine *e = nullptr;
	for (auto &s : saved) if (s.e->type == VEH_TRAIN && s.e->VehInfo<RailVehicleInfo>().railveh_type != RAILVEH_WAGON) { e = s.e; break; }
	Engine *r = nullptr; for (auto &s : saved) if (s.e->type == VEH_ROAD) { r = s.e; break; }
	if (e == nullptr || r == nullptr) FixtureAbort(__LINE__);
	auto railinfo = e->VehInfo<RailVehicleInfo>(); auto roadinfo = r->VehInfo<RoadVehicleInfo>(); auto flags = e->flags;
	e->info.climates = _settings_game.game_creation.landscape; r->info.climates = _settings_game.game_creation.landscape;
	e->VehInfo<RailVehicleInfo>().railtypes = RailTypes{static_cast<RailType>(59), static_cast<RailType>(63)};
	r->VehInfo<RoadVehicleInfo>().roadtype = static_cast<RoadType>(62);
	e->company_avail = {}; r->company_avail = {}; e->intro_date = r->intro_date = TimerGameCalendar::date - 364;
	TypeRequire(GetCompanyRailTypes(c->index, false).None() && GetCompanyRoadTypes(c->index, false).None(), "before-release");
	bool special = true; for (uint8_t id : {16, 17, 32, 33, 48, 49, 63}) special &= GetCompanyRailTypes(CompanyID{id}, false).None() && GetCompanyRoadTypes(CompanyID{id}, false).None();
	TypeRequire(special, "company-bits");
	TypeRequire(GetRailTypes(false) == e->VehInfo<RailVehicleInfo>().railtypes && GetRoadTypes(false) == RoadTypes{static_cast<RoadType>(62)}, "all-multirail");
	e->company_avail.Set(c->index); r->company_avail.Set(c->index);
	TypeRequire(GetCompanyRailTypes(c->index, false) == GetRailTypes(false) && GetCompanyRoadTypes(c->index, false) == GetRoadTypes(false) && GetCompanyRailTypes(CompanyID{static_cast<uint8_t>(c->index.base() ^ 1)}, false).None() && GetCompanyRoadTypes(CompanyID{static_cast<uint8_t>(c->index.base() ^ 1)}, false).None(), "preview-owner");
	e->company_avail = {}; r->company_avail = {}; e->intro_date = r->intro_date = TimerGameCalendar::date - CalendarTime::DAYS_IN_YEAR;
	TypeRequire(GetCompanyRailTypes(c->index, false) == GetRailTypes(false) && GetCompanyRoadTypes(c->index, false) == GetRoadTypes(false), "one-year");
	e->VehInfo<RailVehicleInfo>().railveh_type = RAILVEH_WAGON;
	TypeRequire(GetRailTypes(false).None() && GetCompanyRailTypes(c->index, false).None(), "wagon-only");
	e->VehInfo<RailVehicleInfo>().railveh_type = railinfo.railveh_type;
	e->info.climates = {}; r->info.climates = {};
	TypeRequire(GetRailTypes(false).None() && GetRoadTypes(false).None(), "climate");
	e->info.climates = _settings_game.game_creation.landscape; r->info.climates = _settings_game.game_creation.landscape;
	e->intro_date = r->intro_date = TimerGameCalendar::date + 365;
	Backup<CompanyID> actor(_current_company, OWNER_DEITY);
	auto on = Command<CMD_ENGINE_CTRL>::Do(DoCommandFlag::Execute, e->index, c->index, true);
	auto roadon = Command<CMD_ENGINE_CTRL>::Do(DoCommandFlag::Execute, r->index, c->index, true);
	TypeRequire(on.Succeeded() && roadon.Succeeded() && c->avail_railtypes.Test(static_cast<RailType>(60)) && c->avail_roadtypes.Test(static_cast<RoadType>(60)), "lifecycle-enable");
	auto off = Command<CMD_ENGINE_CTRL>::Do(DoCommandFlag::Execute, e->index, c->index, false);
	auto roadoff = Command<CMD_ENGINE_CTRL>::Do(DoCommandFlag::Execute, r->index, c->index, false);
	TypeRequire(off.Succeeded() && roadoff.Succeeded() && !c->avail_railtypes.Test(static_cast<RailType>(60)) && !c->avail_roadtypes.Test(static_cast<RoadType>(60)), "lifecycle-disable");
	actor.Restore();
	/* Actual general-release path introduces each engine's native types before
	 * applying the fixed points; empty masks rule out a date-only seed. */
	e->intro_date = r->intro_date = TimerGameCalendar::date - CalendarTime::DAYS_IN_YEAR;
	c->avail_railtypes = {}; c->avail_roadtypes = {};
	CalendarEnginesMonthlyLoop();
	TypeRequire(e->company_avail.base() == UINT16_MAX && r->company_avail.base() == UINT16_MAX && c->avail_railtypes.Test(static_cast<RailType>(60)) && c->avail_roadtypes.Test(static_cast<RoadType>(60)), "monthly-publication");
	fmt::print(stderr, "ENGINE types-masks monthly {:016x} {:016x}\n", c->avail_railtypes.base(), c->avail_roadtypes.base());
	e->VehInfo<RailVehicleInfo>() = railinfo; r->VehInfo<RoadVehicleInfo>() = roadinfo; e->flags = flags;
	for (auto &s : saved) { s.e->info.climates = s.climates; s.e->company_avail = s.avail; s.e->intro_date = s.intro; s.e->flags = s.flags; }
}

#include "newgrf_config.h"
#include "newgrf_railtype.h"
#include "newgrf_roadtype.h"
#include "vehicle_cmd.h"
#include "station_cmd.h"
#include "tunnelbridge_cmd.h"
#include "landscape_cmd.h"
#include "terraform_cmd.h"
#include "train.h"
#include "roadveh.h"
static void DefinitionLoad(std::initializer_list<const char *> files)
{
	ClearGRFConfigList(_grfconfig);
	for (auto name : files) {
		auto config = std::make_unique<GRFConfig>(name);
		TypeRequire(FillGRFDetails(*config, false), "definitions-file");
		_grfconfig.push_back(std::move(config));
	}
	ReloadNewGRFData();
}
static void DefinitionTrace(const char *phase)
{
	fmt::print(stderr, "ENGINE definitions-masks {} {:016x} {:016x} {:016x} {:016x}\n", phase, _railtypes_hidden_mask.base(), _roadtypes_hidden_mask.base(), _roadtypes_road.base(), _roadtypes_tram.base());
	for (auto rt : _sorted_railtypes) fmt::print(stderr, "ENGINE definitions-rail {} {} {:08x} {}\n", phase, rt, GetRailTypeInfo(rt)->label, GetRailTypeInfo(rt)->sorting_order);
	for (auto rt : _sorted_roadtypes) fmt::print(stderr, "ENGINE definitions-road {} {} {:08x} {}\n", phase, rt, GetRoadTypeInfo(rt)->label, GetRoadTypeInfo(rt)->sorting_order);
	for (const auto &config : _grfconfig) {
		const auto *file = GetFileByGRFID(config->ident.grfid);
		TypeRequire(file != nullptr, "definitions-live-file");
		for (uint8_t index : {0, 1, 2, 3, 63, 255}) {
			auto rail = GetRailTypeTranslation(index, file); auto road = GetRoadTypeTranslation(RTT_ROAD, index, file); auto tram = GetRoadTypeTranslation(RTT_TRAM, index, file);
			fmt::print(stderr, "ENGINE definitions-translation {} {:08x} {} {} {} {}\n", phase, config->ident.grfid, index, rail, road, tram);
		}
	}
}
static void DefinitionProperties()
{
	auto rail = GetRailTypeByLabel(0x52344141); auto second = GetRailTypeByLabel(0x52344242); auto third = GetRailTypeByLabel(0x52344343);
	auto road = GetRoadTypeByLabel(0x44344141); auto tram = GetRoadTypeByLabel(0x54344141);
	TypeRequire(rail != INVALID_RAILTYPE && road != INVALID_ROADTYPE && tram != INVALID_ROADTYPE, "definitions-reserved");
	TypeRequire(GetRailTypeByLabel(0x41344141) != rail && GetRailTypeByLabel(0x5a344141) == rail && GetRailTypeByLabel(0) == INVALID_RAILTYPE, "definitions-exact-alternate-zero");
	const auto *r = GetRailTypeInfo(rail); const auto *d = GetRoadTypeInfo(road); const auto *t = GetRoadTypeInfo(tram);
	TypeRequire(r->powered_railtypes.Test(second) && r->powered_railtypes.Test(third) && GetRailTypeInfo(second)->powered_railtypes.Test(rail), "definitions-cross-file");
	TypeRequire(d->powered_roadtypes.Test(GetRoadTypeByLabel(0x44344242)) && !d->powered_roadtypes.Test(tram) && GetRoadTramType(tram) == RTT_TRAM, "definitions-road-tram");
	TypeRequire(r->introduction_required_railtypes.Test(second) && GetRailTypeInfo(second)->introduces_railtypes.Test(rail) && d->introduction_required_roadtypes.Test(tram) && t->introduces_roadtypes.Test(road), "definitions-introduction-lists");
	TypeRequire(r->curve_speed == 9 && r->fallback_railtype == 2 && r->acceleration_type == VehicleAccelerationModel::Maglev, "definitions-clamps");
	TypeRequire(r->cost_multiplier == 13 && d->cost_multiplier == 13 && t->cost_multiplier == 13 && r->max_speed == 177 && d->max_speed == 177 && t->max_speed == 177 && r->maintenance_multiplier == 19 && d->maintenance_multiplier == 19 && t->maintenance_multiplier == 19, "definitions-properties");
	TypeRequire(_railtypes_hidden_mask.Test(second) && _roadtypes_hidden_mask.Test(GetRoadTypeByLabel(0x44344444)) && r->introduction_date.base() == 1 && d->introduction_date.base() == 1, "definitions-hidden-introduction");
	const auto *file = GetFileByGRFID(0x41373454);
	TypeRequire(file != nullptr && file->railtype_map[0] == rail && file->railtype_map[1] == rail && file->railtype_map[2] == second && GetReverseRailTypeTranslation(rail, file) == 0 && GetReverseRoadTypeTranslation(road, file) == 0, "definitions-maps-first-match");
	TypeRequire(GetRoadTypeTranslation(RTT_ROAD, 2, file) == INVALID_ROADTYPE && GetRoadTypeTranslation(RTT_TRAM, 1, file) == INVALID_ROADTYPE && GetRailTypeTranslation(3, file) == GetRailTypeByLabel(0x41344141), "definitions-forward-subtype");
}
static void DefinitionReadFailures()
{
	auto *file = GetFileByGRFID(0x41373454);
	Backup<GRFFile *> current(_cur_gps.grffile, file);
	auto failed = [](auto parse, std::span<const uint8_t> input) {
		ByteReader reader(input.data(), input.size());
		try { parse(reader); } catch (const OTTDByteReaderSignal &) { return reader.Remaining(); }
		FixtureAbort(__LINE__);
	};
	const std::array<uint8_t, 5> rail_label{'P', '4', 'A', 'A', 0}, road_label{'Q', '4', 'A', 'A', 0};
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_RAILTYPES>::Reserve(60, 62, 8, reader); }, rail_label) == 1 && file->railtype_map[60] == GetRailTypeByLabel(0x50344141), "definitions-partial-rail-label");
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_ROADTYPES>::Reserve(60, 62, 8, reader); }, road_label) == 1 && file->roadtype_map[60] == GetRoadTypeByLabel(0x51344141), "definitions-partial-road-label");
	const std::array<uint8_t, 6> alternate{2, 'J', '4', 'A', 'A', 0}, compatible{2, 'R', '4', 'D', 'D', 0}, powered{2, 'D', '4', 'D', 'D', 0};
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_RAILTYPES>::Reserve(1, 2, 0x1d, reader); }, alternate) == 1 && GetRailTypeByLabel(0x4a344141) == file->railtype_map[1], "definitions-partial-alternate");
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_RAILTYPES>::Activation(1, 2, 0x0f, reader); }, compatible) == 1 && GetRailTypeInfo(file->railtype_map[1])->powered_railtypes.Test(GetRailTypeByLabel(0x52344444)) && GetRailTypeInfo(file->railtype_map[1])->compatible_railtypes.Test(GetRailTypeByLabel(0x52344444)), "definitions-partial-compatible");
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_ROADTYPES>::Activation(0, 1, 0x0f, reader); }, powered) == 1 && GetRoadTypeInfo(file->roadtype_map[0])->powered_roadtypes.Test(GetRoadTypeByLabel(0x44344444)), "definitions-partial-powered");
	const BadgeID old_badge = GetOrCreateBadge("type/old").index, new_badge = GetOrCreateBadge("type/new").index;
	file->badge_list = {new_badge}; _railtypes[file->railtype_map[1]].badges = {old_badge}; _roadtypes[file->roadtype_map[0]].badges = {old_badge};
	const std::array<uint8_t, 5> badges{2, 0, 0, 0, 0};
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_RAILTYPES>::Activation(1, 2, 0x1e, reader); }, badges) == 1 && GetRailTypeInfo(file->railtype_map[1])->badges == std::vector<BadgeID>{old_badge} && GetBadge(new_badge)->features.Test(GSF_RAILTYPES), "definitions-partial-rail-badges");
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_ROADTYPES>::Activation(0, 1, 0x1e, reader); }, badges) == 1 && GetRoadTypeInfo(file->roadtype_map[0])->badges == std::vector<BadgeID>{old_badge} && GetBadge(new_badge)->features.Test(GSF_ROADTYPES), "definitions-partial-road-badges");
	TypeRequire(failed([](ByteReader &reader) { GrfChangeInfoHandler<GSF_GLOBALVAR>::Reserve(0, 2, 0x12, reader); }, rail_label) == 1 && file->railtype_list == std::vector<RailTypeLabel>{0x50344141}, "definitions-partial-table");
	file->railtype_list.assign(257, 0x4d495353); file->railtype_list.back() = 0x52344141;
	file->roadtype_list.assign(257, 0x4d495353); file->roadtype_list.back() = 0x44344141;
	TypeRequire(GetReverseRailTypeTranslation(file->railtype_map[1], file) == 0 && GetReverseRoadTypeTranslation(file->roadtype_map[0], file) == 0, "definitions-reverse-narrowing");
	GRFFile empty;
	TypeRequire(GetRailTypeTranslation(RAILTYPE_RAIL, nullptr) == RAILTYPE_RAIL && GetRailTypeTranslation(RAILTYPE_RAIL, &empty) == RAILTYPE_RAIL && GetRoadTypeTranslation(RTT_ROAD, 0, &empty) == INVALID_ROADTYPE && GetReverseRailTypeTranslation(RAILTYPE_RAIL, &empty) == RAILTYPE_RAIL && GetReverseRoadTypeTranslation(ROADTYPE_TRAM, &empty) == ROADTYPE_TRAM, "definitions-no-table");
	auto *overridden = GetFileByGRFID(0x42373454);
	SetNewGRFOverride(file->grfid, overridden->grfid);
	auto table_copy = [&](int prop, auto &source, auto &target) {
		using Label = typename std::remove_reference_t<decltype(source)>::value_type;
		target = {Label{0x4d495353}};
		TypeRequire(failed([&](ByteReader &reader) { GrfChangeInfoHandler<GSF_GLOBALVAR>::Reserve(0, 2, prop, reader); }, rail_label) == 1 && source == std::vector<Label>{Label{0x50344141}} && target == std::vector<Label>{Label{0x4d495353}}, "definitions-override-partial");
		const std::array<uint8_t, 4> complete{'O', '4', 'V', 'R'}; ByteReader reader(complete.data(), complete.size());
		TypeRequire(GrfChangeInfoHandler<GSF_GLOBALVAR>::Reserve(0, 1, prop, reader) == CIR_SUCCESS && source == std::vector<Label>{Label{0x4f345652}} && target == source, "definitions-override-table");
	};
	table_copy(0x09, file->cargo_list, overridden->cargo_list); table_copy(0x12, file->railtype_list, overridden->railtype_list);
	table_copy(0x16, file->roadtype_list, overridden->roadtype_list); table_copy(0x17, file->tramtype_list, overridden->tramtype_list);
	current.Restore();
}
static bool DefinitionScenario(std::span<std::string_view> args)
{
	Company *company = *Company::Iterate().begin();
	Backup<CompanyID> actor(_current_company, company->index);
	_settings_game.vehicle.never_expire_vehicles = true;
	const DoCommandFlags exec = {DoCommandFlag::Execute, DoCommandFlag::Auto};
	auto require_cost = [](const CommandCost &cost, const char *label = "construction") { if (cost.Failed()) { fmt::print(stderr, "ENGINE definitions-command-error {} {}\n", label, cost.GetErrorMessage()); FixtureAbort(__LINE__); } };
	if (args[1] == "types-definitions-build") {
		DefinitionLoad({"types-a.grf", "types-b.grf"}); DefinitionProperties(); DefinitionTrace("loaded"); DefinitionReadFailures();
		DefinitionLoad({"types-a.grf", "types-b.grf", "types-mismatch.grf"});
		TypeRequire(GetRoadTramType(GetRoadTypeByLabel(0x54344141)) == RTT_TRAM, "definitions-reserve-mismatch");
		DefinitionLoad({"types-a.grf", "types-b.grf", "types-full.grf"});
		TypeRequire(AllocateRailType(0x46554c4c) == INVALID_RAILTYPE && AllocateRoadType(0x46554c4c, RTT_ROAD) == INVALID_ROADTYPE, "definitions-exhaustion");
		DefinitionLoad({"types-a.grf", "types-b.grf"}); DefinitionProperties();
		TypeRequire(GetRailTypeByLabel(0x58303030) == INVALID_RAILTYPE && GetRoadTypeByLabel(0x59303030) == INVALID_ROADTYPE, "definitions-reset");
		TileIndex base = TypeFlatArea(14);
		for (uint kind = 0; kind < 3; ++kind) {
			TileIndex tile = base + TileDiffXY(0, kind * 2);
			RailType rail = GetRailTypeByLabel(0x52344141); RoadType road = GetRoadTypeByLabel(kind == 1 ? 0x44344141 : 0x54344141);
			if (kind == 0) {
				require_cost(Command<CMD_BUILD_TRAIN_DEPOT>::Do(exec, tile, rail, DIAGDIR_SW));
				for (int x = 1; x <= 5; ++x) require_cost(Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, tile + TileDiffXY(x, 0), rail, TRACK_X, false));
			} else {
				require_cost(Command<CMD_BUILD_ROAD_DEPOT>::Do(exec, tile, road, DIAGDIR_SW));
				for (int x = 1; x <= 5; ++x) require_cost(Command<CMD_BUILD_ROAD>::Do(exec, tile + TileDiffXY(x, 0), ROAD_X, road, DRD_NONE, TownID::Invalid()));
			}
			EngineID engine = EngineID::Invalid();
			for (const Engine *e : Engine::IterateType(kind == 0 ? VEH_TRAIN : VEH_ROAD)) {
				if (!IsEngineBuildable(e->index, e->type, company->index)) continue;
				if (kind == 0 ? !e->VehInfo<RailVehicleInfo>().railtypes.Test(rail) : e->VehInfo<RoadVehicleInfo>().roadtype != road) continue;
				if (std::get<0>(Command<CMD_BUILD_VEHICLE>::Do({}, tile, e->index, false, INVALID_CARGO, INVALID_CLIENT_ID)).Failed()) continue;
				engine = e->index; break;
			}
			TypeRequire(engine != EngineID::Invalid(), "definitions-usable-engine");
			fmt::print(stderr, "ENGINE definitions-build {} {}\n", kind, engine.base());
			auto built = Command<CMD_BUILD_VEHICLE>::Do(exec, tile, engine, false, INVALID_CARGO, INVALID_CLIENT_ID);
			require_cost(std::get<0>(built), "vehicle"); auto id = std::get<1>(built);
			require_cost(Command<CMD_RENAME_VEHICLE>::Do(exec, id, fmt::format("Type registry {}", kind)), "rename");
			require_cost(Command<CMD_START_STOP_VEHICLE>::Do(exec, id, false), "start");
		}
		auto rail = GetRailTypeByLabel(0x52344141); auto road = GetRoadTypeByLabel(0x44344141); auto tram = GetRoadTypeByLabel(0x54344141);
		require_cost(Command<CMD_BUILD_ROAD>::Do(exec, base + TileDiffXY(3, 0), ROAD_Y, road, DRD_NONE, TownID::Invalid()));
		require_cost(Command<CMD_BUILD_RAIL_STATION>::Do(exec, base + TileDiffXY(1, 1), rail, AXIS_X, 1, 3, STAT_CLASS_DFLT, 0, StationID::Invalid(), false));
		for (auto [dy, rt] : {std::pair{3, road}, std::pair{5, tram}}) require_cost(Command<CMD_BUILD_ROAD_STOP>::Do(exec, base + TileDiffXY(1, dy), 1, 1, RoadStopType::Bus, true, DIAGDIR_NE, rt, ROADSTOP_CLASS_DFLT, 0, StationID::Invalid(), false));
		require_cost(Command<CMD_BUILD_BRIDGE>::Do(exec, base + TileDiffXY(4, 7), base + TileDiffXY(1, 7), TRANSPORT_RAIL, BridgeType{0}, rail));
		for (auto rt : {road, tram}) require_cost(Command<CMD_BUILD_BRIDGE>::Do(exec, base + TileDiffXY(7, 1), base + TileDiffXY(4, 1), TRANSPORT_ROAD, BridgeType{0}, rt));
		TileIndex tunnel = TypeFlatArea(14);
		for (auto [dy, transport, rt] : {std::tuple{2, TRANSPORT_RAIL, uint8_t(rail)}, std::tuple{4, TRANSPORT_ROAD, uint8_t(road)}}) {
			require_cost(std::get<0>(Command<CMD_TERRAFORM_LAND>::Do(exec, tunnel + TileDiffXY(3, dy), SLOPE_ELEVATED, true)));
			require_cost(Command<CMD_BUILD_TUNNEL>::Do(exec, tunnel + TileDiffXY(2, dy), transport, rt));
		}
		require_cost(Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, base + TileDiffXY(0, 6), GetRailTypeByLabel(0x52344444), TRACK_X, false));
		TypeRequire(true, "definitions-built");
	} else {
		DefinitionProperties(); DefinitionTrace("saved");
		uint moving = 0;
		for (const Vehicle *vehicle : Vehicle::Iterate()) if (vehicle->name.starts_with("Type registry ")) { TypeRequire(!vehicle->IsInDepot(), "definitions-movement"); ++moving; }
		TypeRequire(moving == 3, "definitions-file-loaded-save");
		std::vector<std::pair<TileIndex, RailTypeLabel>> rails;
		std::vector<std::tuple<TileIndex, RoadTypeLabel, RoadTypeLabel>> roads;
		uint rail_categories = 0, road_categories = 0;
		for (auto tile : Map::Iterate()) {
			auto rail = GetTileRailType(tile); if (rail != INVALID_RAILTYPE) { rails.emplace_back(tile, GetRailTypeInfo(rail)->label); if (GetRailTypeInfo(rail)->label == 0x52344141) rail_categories |= 1u << GetTileType(tile); }
			if (MayHaveRoad(tile)) { auto road = GetRoadTypeRoad(tile); auto tram = GetRoadTypeTram(tile); if (road != INVALID_ROADTYPE && GetRoadTypeInfo(road)->label == 0x44344141) road_categories |= 1u << GetTileType(tile); roads.emplace_back(tile, road == INVALID_ROADTYPE ? 0 : GetRoadTypeInfo(road)->label, tram == INVALID_ROADTYPE ? 0 : GetRoadTypeInfo(tram)->label); }
		}
		TypeRequire(rail_categories == ((1u << MP_RAILWAY) | (1u << MP_ROAD) | (1u << MP_STATION) | (1u << MP_TUNNELBRIDGE)) && road_categories == ((1u << MP_ROAD) | (1u << MP_STATION) | (1u << MP_TUNNELBRIDGE)), "definitions-conversion-categories");
		auto old = GetRailTypeByLabel(0x52344141);
		DefinitionLoad({"types-b.grf", "types-a.grf"}); DefinitionProperties(); DefinitionTrace("reordered");
		TypeRequire(old != GetRailTypeByLabel(0x52344141), "definitions-allocation-order");
		bool same_rail = true, same_road = true, same_tram = true;
		for (const auto &[tile, label] : rails) same_rail &= GetRailTypeInfo(GetTileRailType(tile))->label == label;
		for (const auto &[tile, road, tram] : roads) { if (road != 0) same_road &= GetRoadTypeInfo(GetRoadTypeRoad(tile))->label == road; if (tram != 0) same_tram &= GetRoadTypeInfo(GetRoadTypeTram(tile))->label == tram; }
		TypeRequire(same_rail && same_road && same_tram, "definitions-reordered-tiles");
		TypeRequire(true, "definitions-reordered");
		DefinitionLoad({"types-missing.grf"}); DefinitionTrace("missing");
		uint alternate = 0, missing_rail = 0, missing_road = 0, mismatched_tram = 0;
		for (const auto &[tile, label] : rails) { if (label == 0x52344141) { same_rail &= GetRailTypeInfo(GetTileRailType(tile))->label == 0x52344343; ++alternate; } if (label == 0x52344444) { same_rail &= GetTileRailType(tile) == RAILTYPE_RAIL; ++missing_rail; } }
		for (const auto &[tile, road, tram] : roads) { if (road == 0x44344141) { same_road &= GetRoadTypeRoad(tile) == ROADTYPE_ROAD; ++missing_road; } if (tram == 0x54344141) { same_tram &= GetRoadTypeTram(tile) == ROADTYPE_TRAM; ++mismatched_tram; } }
		TypeRequire(same_rail && same_road && same_tram && alternate != 0 && missing_rail != 0 && missing_road != 0 && mismatched_tram != 0, "definitions-missing-fallbacks");
		TypeRequire(true, "definitions-missing");
	}
	actor.Restore(); return true;
}

static bool TypesScenario(std::span<std::string_view> args)
{
	if (args[1].starts_with("types-definitions")) return DefinitionScenario(args);
	if (!TypesInstalled) FixtureAbort(__LINE__);
	Company *c = nullptr; for (Company *other : Company::Iterate()) { c = other; break; }
	if (c == nullptr) FixtureAbort(__LINE__);
	Backup<CompanyID> actor(_current_company, c->index);
	const RailTypes railchain{static_cast<RailType>(60), static_cast<RailType>(61), static_cast<RailType>(62), static_cast<RailType>(63)};
	const RoadTypes roadchain{static_cast<RoadType>(60), static_cast<RoadType>(61), static_cast<RoadType>(62)};
	TypeRequire(c->avail_railtypes.All(railchain) && c->avail_roadtypes.All(roadchain), "afterload-masks");
	fmt::print(stderr, "ENGINE types-masks loaded {:016x} {:016x}\n", c->avail_railtypes.base(), c->avail_roadtypes.base());
	if (args[1] != "types-inspect") {
		auto date = TimerGameCalendar::date;
		RailTypes rs = AddDateIntroducedRailTypes(RailTypes{RAILTYPE_RAIL}, date);
		RoadTypes ds = AddDateIntroducedRoadTypes(RoadTypes{ROADTYPE_ROAD}, date);
		TypeRequire(rs.All(railchain) && ds.All(roadchain), "fixedpoint");
		for (uint i : {50, 52, 53, 54, 55, 56, 57}) TypeRequire(!rs.Test(static_cast<RailType>(i)) && !ds.Test(static_cast<RoadType>(i)), fmt::format("excluded-{}", i).c_str());
		TypeRequire(rs.Test(static_cast<RailType>(51)) && ds.Test(static_cast<RoadType>(51)), "date-zero");
		TypeRequire(AddDateIntroducedRailTypes(RailTypes{static_cast<RailType>(55)}, date).Test(static_cast<RailType>(56)) && AddDateIntroducedRoadTypes(RoadTypes{static_cast<RoadType>(55)}, date).Test(static_cast<RoadType>(56)), "seeded-cycle");
		TypeRequire(AddDateIntroducedRailTypes(RailTypes{RAILTYPE_RAIL}, date - 1).Test(static_cast<RailType>(60)) == false && AddDateIntroducedRoadTypes(RoadTypes{ROADTYPE_ROAD}, date - 1).Test(static_cast<RoadType>(60)) == false, "inclusive-day");
		TypeRequire(GetRailTypes(true).Test(static_cast<RailType>(53)) && GetRoadTypes(true).Test(static_cast<RoadType>(53)), "max-date");
		TypeEngineInputs(c);
		TileIndex tile = TypeFlatArea(); auto infra = c->infrastructure;
		c->avail_railtypes = RailTypes{RAILTYPE_RAIL}; c->avail_roadtypes = RoadTypes{ROADTYPE_ROAD};
		TypeConstruction(c, tile, 0, 60, false, "rail-before"); TypeConstruction(c, tile + TileDiffXY(2, 0), 1, 60, false, "road-before"); TypeConstruction(c, tile + TileDiffXY(4, 0), 1, 61, false, "tram-before");
		/* Native daily timer calls the actual engine lifecycle publication path. */
		TimerGameCalendar::SetDate(date, 73); TimerManager<TimerGameCalendar>::Elapsed(1);
		TypeRequire(c->avail_railtypes.All(railchain) && c->avail_roadtypes.All(roadchain), "daily-publication");
		fmt::print(stderr, "ENGINE types-masks daily {:016x} {:016x}\n", c->avail_railtypes.base(), c->avail_roadtypes.base());
		TypeConstruction(c, tile, 0, 60, true, "rail-after"); TypeConstruction(c, tile + TileDiffXY(2, 0), 1, 60, true, "road-after"); TypeConstruction(c, tile + TileDiffXY(4, 0), 1, 61, true, "tram-after");
		TypeRequire(!HasRailTypeAvail(c->index, static_cast<RailType>(62)) && HasAnyRailTypesAvail(c->index) && HasAnyRoadTypesAvail(c->index, RTT_ROAD) && HasAnyRoadTypesAvail(c->index, RTT_TRAM), "hidden-and-masks");
		{
			Backup<RailTypes> rails(c->avail_railtypes, RailTypes{static_cast<RailType>(62)});
			TypeRequire(!HasAnyRailTypesAvail(c->index), "hidden-only-rail");
			rails.Restore();
			Backup<RoadTypes> roads(c->avail_roadtypes, RoadTypes{static_cast<RoadType>(61)});
			TypeRequire(!HasAnyRoadTypesAvail(c->index, RTT_ROAD) && HasAnyRoadTypesAvail(c->index, RTT_TRAM), "tram-only-mask");
			roads.Change(RoadTypes{static_cast<RoadType>(60)});
			TypeRequire(HasAnyRoadTypesAvail(c->index, RTT_ROAD) && !HasAnyRoadTypesAvail(c->index, RTT_TRAM), "road-only-mask");
			roads.Restore();
		}
		TypeRequire(!HasRoadTypeAvail(CompanyID::Invalid(), static_cast<RoadType>(59)) && !HasRoadTypeAvail(c->index, static_cast<RoadType>(59)), "company-hidden");
		TypeConstruction(c, tile + TileDiffXY(6, 0), 1, 59, false, "hidden-company");
		for (CompanyID owner : {OWNER_TOWN, OWNER_DEITY}) {
			actor.Change(owner);
			/* Move future-date input two days ahead: the actual daily tick passed one day. */
			_roadtypes[53].introduction_date = TimerGameCalendar::date + 1;
			TypeRequire(HasRoadTypeAvail(owner, static_cast<RoadType>(59)) && !HasRoadTypeAvail(owner, static_cast<RoadType>(58)) && !HasRoadTypeAvail(owner, static_cast<RoadType>(53)) && !HasRoadTypeAvail(owner, static_cast<RoadType>(54)) && HasRoadTypeAvail(owner, static_cast<RoadType>(50)) && HasRoadTypeAvail(owner, static_cast<RoadType>(52)), owner == OWNER_TOWN ? "town" : "deity");
		}
		actor.Change(OWNER_TOWN); TypeConstruction(c, tile + TileDiffXY(6, 0), 1, 59, true, "hidden-town");
		actor.Change(c->index);
		{ Backup<GameMode> mode(_game_mode, GM_EDITOR); TypeRequire(HasRoadTypeAvail(c->index, static_cast<RoadType>(59)) && !HasRoadTypeAvail(c->index, static_cast<RoadType>(58)), "editor"); mode.Restore(); }
		{ Backup<bool> generating(_generating_world, true); TypeRequire(HasRoadTypeAvail(c->index, static_cast<RoadType>(59)) && !HasRoadTypeAvail(c->index, static_cast<RoadType>(58)), "generation"); generating.Restore(); }
		for (uint dx : {0, 2, 4, 6}) MakeClear(tile + TileDiffXY(dx, 0), CLEAR_GRASS, 3);
		c->infrastructure = infra;
	}
	std::copy(TypeOriginalRail.begin(), TypeOriginalRail.end(), std::begin(_railtypes)); std::copy(TypeOriginalRoad.begin(), TypeOriginalRoad.end(), std::begin(_roadtypes));
	_roadtypes_road = TypeOriginalRoadMask; _roadtypes_tram = TypeOriginalTramMask; InitRailTypes(); InitRoadTypes();
	for (Company *other : Company::Iterate()) { other->avail_railtypes = GetCompanyRailTypes(other->index); other->avail_roadtypes = GetCompanyRoadTypes(other->index); }
	TypeRequire(_railtypes[60].label == 0 && _roadtypes[60].label == 0, "restoration");
	fmt::print(stderr, "ENGINE types-complete true\n");
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	actor.Restore(); return true;
}
