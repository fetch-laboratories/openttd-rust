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
extern RoadTypes _roadtypes_road, _roadtypes_tram;
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
static TileIndex TypeFlatArea()
{
	for (uint y = 2; y + 10 < Map::SizeY(); ++y) for (uint x = 2; x + 10 < Map::SizeX(); ++x) {
		bool clear = true;
		for (uint dy = 0; dy < 10; ++dy) for (uint dx = 0; dx < 10; ++dx) {
			auto type = GetTileType(TileXY(x + dx, y + dy)); if (type != MP_CLEAR && type != MP_TREES) clear = false;
		}
		if (!clear) continue;
		for (uint dy = 0; dy < 10; ++dy) for (uint dx = 0; dx < 10; ++dx) {
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
		auto result = kind == 0 ? Command<CMD_BUILD_SINGLE_RAIL>::Do(flags, tile, static_cast<RailType>(type), TRACK_X, false) : Command<CMD_BUILD_ROAD>::Do(flags, tile, ROAD_X, static_cast<RoadType>(type), DRD_NONE, TownID{0});
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
static bool TypesScenario(std::span<std::string_view> args)
{
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
