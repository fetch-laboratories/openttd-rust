/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station-lifecycle-console.hpp Station naming, waypoint ground and destruction fixtures. */
#include "tree_map.h"
#ifdef WITH_RUST
#include "rust/ground_services.h"
#endif
#include "linkgraph/linkgraphschedule.h"

static void Station562Rng(const char *phase)
{
	fmt::print(stderr, "STCMD owner562-rng {} {} {} {} {}\n", phase, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
}
static Station *Station562New(TileIndex tile, const std::string &label)
{
	StationNeed(Station::CanAllocateItem(), __LINE__);
	Station *st = new Station(tile); _station_kdtree.Insert(st->index);
	st->town = ClosestTownFromTile(tile, UINT_MAX); st->owner = _current_company; st->name = label;
	return st;
}
static void Station562Names(TileIndex origin, DoCommandFlags exec)
{
	std::vector<Station *> seeds;
	/* Reserve only the central names so the neighbourhood cases reach their predicates. */
	for (Town *town : Town::Iterate()) for (StringID name : {STR_SV_STNAME, STR_SV_STNAME_CENTRAL}) {
		Station *st = Station562New(origin, "Owner562 seed"); st->town = town; st->string_id = name; seeds.push_back(st);
	}
	auto at = [origin](int x, int y) { return origin + TileDiffXY(x, y); };
	auto build = [exec](TileIndex tile, const char *label, StringID expected, IndustryType industry = IT_INVALID) {
		StationNeed(Command<CMD_BUILD_RAIL_STATION>::Do(exec, tile, RAILTYPE_RAIL, AXIS_X, 1, 1, STAT_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
		Station *st = Station::GetByTile(tile); st->name = std::string("Owner562 ") + label;
		StationNeed(st->string_id == expected && st->indtype == industry, __LINE__);
		fmt::print(stderr, "STCMD owner562-name {} {} {} {}\n", label, st->index.base(), st->string_id, st->indtype);
		return st;
	};
	StationNeed(DistanceFromEdge(at(2, 2)) < 20, __LINE__);
	for (int dx = -2; dx <= 2; dx++) if (dx != 0) for (int dy : {-1, 1}) MakeCanal(at(2 + dx, 2 + dy), _current_company, 0);
	build(at(2, 2), "water", STR_SV_STNAME_LAKESIDE);
	for (int dx = -2; dx <= 2; dx++) if (dx != 0) for (int dy : {-1, 1}) MakeTree(at(12 + dx, 2 + dy), TREE_TEMPERATE, 1, TreeGrowthStage::Grown, TREE_GROUND_GRASS, 3);
	Station *woods = build(at(12, 2), "woods", STR_SV_STNAME_WOODS);
	_settings_game.game_creation.landscape = LandscapeType::Tropic;
	for (int dx = -2; dx <= 2; dx++) if (dx != 0) for (int dy : {-1, 1}) MakeTree(at(22 + dx, 2 + dy), TREE_RAINFOREST, 1, TreeGrowthStage::Grown, TREE_GROUND_GRASS, 3);
	Backup<StringID> woods_name(woods->string_id, STR_SV_STNAME_FALLBACK);
	build(at(22, 2), "forest", STR_SV_STNAME_FOREST);
	woods_name.Restore(); _settings_game.game_creation.landscape = LandscapeType::Temperate;
	StationNeed(Industry::CanAllocateItem(), __LINE__);
	Industry *ind = new Industry(); ind->location = TileArea(at(12, 12), 2, 1); ind->town = ClosestTownFromTile(at(12, 12), UINT_MAX); ind->type = 0;
	ind->produced.resize(1); ind->produced[0].cargo = GetCargoTypeByLabel(CT_COAL);
	MakeIndustry(at(12, 12), ind->index, 0, 0, WaterClass::Invalid); MakeIndustry(at(13, 12), ind->index, 0, 0, WaterClass::Invalid);
	auto *spec = const_cast<IndustrySpec *>(GetIndustrySpec(0));
	Backup<StringID> station_name(spec->station_name, STR_UNDEFINED);
	Backup<IndustryLifeTypes> life(spec->life_type, {IndustryLifeType::Extractive});
	build(at(12, 14), "mine", STR_SV_STNAME_MINES);
	spec->station_name = STR_SV_STNAME_TRANSFER;
	build(at(15, 14), "industry", STR_SV_STNAME_FALLBACK, 0);
	/* An industry name already used in the town suppresses mines and oilfield. */
	Station *used = Station562New(at(18, 14), "Owner562 used-industry"); used->indtype = 0; used->town = ClosestTownFromTile(at(17, 14), UINT_MAX); seeds.push_back(used);
	std::vector<Station *> aliases;
	for (Town *town : Town::Iterate()) for (StringID name = STR_SV_STNAME; name < STR_SV_STNAME_FALLBACK; ++name) if (name != STR_SV_STNAME_TRANSFER) {
		Station *st = Station562New(origin, "Owner562 seed"); st->town = town; st->string_id = name; aliases.push_back(st);
	}
	build(at(17, 14), "industry-used", STR_SV_STNAME_TRANSFER);
	for (Station *st : aliases) delete st;
	station_name.Restore(); life.Restore();
	StationNeed(Industry::CanAllocateItem(), __LINE__);
	Industry *rig = new Industry(); rig->location = TileArea(at(24, 24), 1, 1); rig->town = ClosestTownFromTile(at(24, 24), UINT_MAX); rig->type = 5;
	MakeIndustry(rig->location.tile, rig->index, 0, 0, WaterClass::Sea); BuildOilRig(rig->location.tile);
	StationNeed(rig->neutral_station != nullptr && rig->neutral_station->string_id == STR_SV_STNAME_OILFIELD && rig->neutral_station->indtype == IT_INVALID, __LINE__); rig->neutral_station->name = "Owner562 oilrig";
	fmt::print(stderr, "STCMD owner562-name oilrig {} {} {}\n", rig->neutral_station->index.base(), rig->neutral_station->string_id, rig->neutral_station->indtype);
	/* Exhaust every default suffix using real station records, including forest aliasing. */
	for (Town *town : Town::Iterate()) for (StringID name = STR_SV_STNAME; name < STR_SV_STNAME_FALLBACK; ++name) {
		Station *st = Station562New(origin, "Owner562 seed"); st->town = town; st->string_id = name; seeds.push_back(st);
	}
	build(at(26, 12), "exhausted", STR_SV_STNAME_FALLBACK);
	for (Station *st : seeds) delete st;
	Station562Rng("names");
}
static void Station562Waypoints(TileIndex tile, DoCommandFlags exec, bool arctic, bool reload)
{
	if (!reload) {
		StationNeed(Command<CMD_BUILD_ROAD>::Do(exec, tile, ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()).Succeeded(), __LINE__);
		StationNeed(Command<CMD_BUILD_ROAD_WAYPOINT>::Do(exec, tile, AXIS_X, 1, 1, ROADSTOP_CLASS_WAYP, 0, NEW_STATION, true).Succeeded(), __LINE__);
		Waypoint::GetByTile(tile)->name = "Owner562 waypoint";
	} else {
		for (Waypoint *wp : Waypoint::Iterate()) if (wp->name == "Owner562 waypoint") tile = wp->xy;
		StationNeed(tile != INVALID_TILE && IsRoadWaypointTile(tile), __LINE__);
	}
	_settings_game.game_creation.landscape = arctic ? LandscapeType::Arctic : LandscapeType::Tropic;
	_settings_game.game_creation.snow_line_height = 0; ClearSnowLine();
	if (IsRoadWaypointOnSnowOrDesert(tile)) ToggleRoadWaypointOnSnowOrDesert(tile);
	SetTropicZone(tile, TROPICZONE_DESERT); SetRoadWaypointRoadside(tile, Roadside::Paved);
	const Town *town = ClosestTownFromTile(tile, UINT_MAX);
	Backup<std::array<uint32_t, NUM_HOUSE_ZONES>> radii(const_cast<Town *>(town)->cache.squared_town_zone_radius);
	const_cast<Town *>(town)->cache.squared_town_zone_radius.fill(0);
	for (int phase = 0; phase < 6; ++phase) {
		if (phase == 2) { _settings_game.game_creation.snow_line_height = 2; SetTropicZone(tile, TROPICZONE_NORMAL); }
		if (phase == 4) const_cast<Town *>(town)->cache.squared_town_zone_radius.fill(UINT32_MAX);
#ifdef WITH_RUST
		OpenTTDGroundCall call; openttd_rust_station_tile_loop(&GetRustStationLifecycleServices(), tile.base(), &call);
#else
		TileLoop_Station(tile);
#endif
		StationNeed(IsRoadWaypointOnSnowOrDesert(tile) == (!arctic || phase < 2), __LINE__);
		StationNeed(GetRoadWaypointRoadside(tile) == (phase == 0 || phase == 4 ? Roadside::Barren : phase == 5 ? Roadside::Paved : Roadside::Grass), __LINE__);
		fmt::print(stderr, "STCMD owner562-ground {} {} {} {}\n", arctic, phase, IsRoadWaypointOnSnowOrDesert(tile), to_underlying(GetRoadWaypointRoadside(tile)));
	}
	radii.Restore(); Station562Rng("ground");
}
static void Station562Delete(TileIndex origin, bool reload, bool stage)
{
	Station *victim = nullptr, *survivor = nullptr;
	RoadVehicle *vehicle = nullptr;
	if (!reload) {
		victim = Station562New(origin, "Owner562 victim"); survivor = Station562New(origin + TileDiffXY(4, 0), "Owner562 survivor");
		victim->UpdateVirtCoord(); survivor->UpdateVirtCoord();
		for (RoadVehicle *v : RoadVehicle::Iterate()) if (v->IsPrimaryVehicle() && v->cargo_type == 0 && !v->IsInDepot()) { vehicle = v; break; }
		StationNeed(vehicle != nullptr && vehicle->cargo_payment == nullptr && LinkGraph::CanAllocateItem(2) && CargoPacket::CanAllocateItem(2), __LINE__);
		vehicle->current_order.MakeLoading(false); vehicle->current_order.SetLoadType(OLFB_NO_LOAD); vehicle->current_order.SetUnloadType(OUFB_NO_UNLOAD);
		vehicle->last_station_visited = victim->index; vehicle->last_loading_station = victim->index; victim->loading_vehicles.push_back(vehicle);
		for (CargoType cargo : {0, 1}) {
			LinkGraph *graph = new LinkGraph(cargo); victim->goods[cargo].link_graph = graph->index; victim->goods[cargo].node = graph->AddNode(victim);
			if (cargo == 0) {
				survivor->goods[cargo].link_graph = graph->index; survivor->goods[cargo].node = graph->AddNode(survivor);
				(*graph)[1].AddEdge(0, 12, 3, 8, {EdgeUpdateMode::Increase, EdgeUpdateMode::Unrestricted});
				auto &data = survivor->goods[cargo].GetOrCreateData(); data.flows.AddFlow(victim->index, victim->index, 7); data.flows.AddFlow(survivor->index, victim->index, 7); data.flows.AddFlow(survivor->index, survivor->index, 9);
				data.cargo.Append(new CargoPacket(survivor->index, 5, Source{survivor->town->index, SourceType::Town}), victim->index);
			}
			LinkGraphSchedule::instance.Queue(graph);
		}
		victim->goods[0].GetOrCreateData().cargo.Append(new CargoPacket(victim->index, 11, Source{victim->town->index, SourceType::Town}), survivor->index);
	} else {
		for (Station *st : Station::Iterate()) { if (st->name == "Owner562 victim") victim = st; if (st->name == "Owner562 survivor") survivor = st; }
		StationNeed(victim != nullptr && survivor != nullptr && !victim->loading_vehicles.empty(), __LINE__);
		vehicle = RoadVehicle::From(victim->loading_vehicles.front());
	}
	StationNeed(victim->goods[0].AvailableCount() == 11 && survivor->goods[0].AvailableCount() == 5, __LINE__);
	fmt::print(stderr, "STCMD owner562-delete before {} {} {}\n", victim->goods[0].AvailableCount(), survivor->goods[0].AvailableCount(), victim->loading_vehicles.size());
	if (!stage) {
		StationID id = victim->index; LinkGraphID empty = victim->goods[1].link_graph; delete victim;
		StationNeed(!Station::IsValidID(id) && !LinkGraph::IsValidID(empty), __LINE__);
		StationNeed(vehicle->last_station_visited == StationID::Invalid() && vehicle->last_loading_station == StationID::Invalid() && !vehicle->current_order.IsType(OT_LOADING), __LINE__);
		StationNeed(LinkGraph::Get(survivor->goods[0].link_graph)->Size() == 1 && survivor->goods[0].node == 0 && survivor->goods[0].GetData().flows.find(id) == survivor->goods[0].GetData().flows.end(), __LINE__);
		StationNeed(survivor->goods[0].AvailableCount() == 5, __LINE__);
		const auto packets = survivor->goods[0].GetData().cargo.Packets(); StationNeed(packets->find(id) == packets->end(), __LINE__);
		fmt::print(stderr, "STCMD owner562-delete after 5 1 0\n");
	}
	Station562Rng("delete");
}
static void Station562InspectNames()
{
	uint found = 0;
	for (Station *st : Station::Iterate()) if (st->name.starts_with("Owner562 ")) {
		StationNeed(st->name != "Owner562 seed", __LINE__); ++found;
		fmt::print(stderr, "STCMD owner562-name reloaded {} {} {} {}\n", st->name, st->index.base(), st->string_id, st->indtype);
	}
	StationNeed(found == 8, __LINE__); Station562Rng("names-reloaded");
}
