/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station-spatial-console.hpp Unsorted station observations and actual query consumers. */
#ifdef WITH_RUST
#include "tests/station_spatial_fixture.hpp"
#else
#include "station602-town-original.hpp"
enum TriggerArea : uint8_t;
TileArea GetRailTileArea(const BaseStation *, TileIndex, TriggerArea);
#endif
static void Station602Rebuild()
{
#ifdef WITH_RUST
	openttd_rust_station_rebuild_index(&GetRustSharedServices());
#else
	RebuildStationKdtree();
#endif
}
static std::vector<Station *> Station602Visit(const char *phase, TileIndex tile, uint radius)
{
	std::vector<Station *> visits;
#ifdef WITH_RUST
	openttd_test_station_radius(&GetRustSharedServices(), tile.base(), radius, &visits, [](void *context, BaseStation *st) { static_cast<std::vector<Station *> *>(context)->push_back(Station::From(st)); });
#else
	ForAllStationsRadius(tile, radius, [&](Station *st) { visits.push_back(st); });
#endif
	int16_t rating = INT16_MAX;
	fmt::print(stderr, "STCMD spatial602-visit {} {} {}", phase, tile.base(), radius);
	for (const Station *st : visits) {
		bool active = st->time_since_load <= 20 || st->time_since_unload <= 20;
		rating = active ? std::min<int>(rating + 12, INT16_MAX) : std::max<int>(rating - 15, INT16_MIN);
		fmt::print(stderr, " {}:{}:{}", st->index.base(), active, rating);
	}
	fmt::print(stderr, "\n");
	return visits;
}
static void Station602Rating(TileIndex tile, uint radius, int amount)
{
#ifdef WITH_RUST
	openttd_test_station_rating_around(&GetRustSharedServices(), tile.base(), _current_company.base(), amount, radius);
#else
	ModifyStationRatingAround(tile, _current_company, amount, radius);
#endif
}
static RoadStop *Station602Primary(Station *st, RoadVehicle *v)
{
#ifdef WITH_RUST
	return openttd_test_station_primary_stop(&GetRustSharedServices(), st, RustVehicle(v));
#else
	return st->GetPrimaryRoadStop(v);
#endif
}
static RoadStop *Station602Next(RoadStop *stop, RoadVehicle *v)
{
#ifdef WITH_RUST
	return openttd_test_station_next_stop(&GetRustSharedServices(), stop, RustVehicle(v));
#else
	return stop->GetNextRoadStop(v);
#endif
}
static void Station602Stops(TileIndex tile, DoCommandFlags exec)
{
	StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, tile, 1, 1, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
	Station *st = Station::GetByTile(tile);
	StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, tile + TileDiffXY(3, 0), 1, 1, RoadStopType::Bus, true, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, st->index, true).Succeeded(), __LINE__);
	StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, tile + TileDiffXY(6, 0), 1, 1, RoadStopType::Truck, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, st->index, true).Succeeded(), __LINE__);
	auto *v = new RoadVehicle(); v->SetFrontEngine(); v->owner = _current_company; v->compatible_roadtypes = RoadTypes{ROADTYPE_ROAD};
	for (CargoType cargo : {CargoType{0}, CargoType{1}}) {
		v->cargo_type = cargo;
		for (bool incompatible : {false, true}) {
			v->compatible_roadtypes = incompatible ? RoadTypes{} : RoadTypes{ROADTYPE_ROAD};
			RoadStop *first = Station602Primary(st, v), *next = first == nullptr ? nullptr : Station602Next(first, v);
			fmt::print(stderr, "STCMD spatial602-stop {} {} {} {}\n", v->IsBus(), incompatible, first == nullptr ? UINT32_MAX : first->index.base(), next == nullptr ? UINT32_MAX : next->index.base());
			StationNeed((first == nullptr) == incompatible, __LINE__);
		}
	}
	v->cargo_type = 0; v->compatible_roadtypes = RoadTypes{ROADTYPE_ROAD};
	RoadStop *bay = st->bus_stops; auto status = bay->status; bay->status.Reset(RoadStop::RoadStopStatusFlag::Bay0Free); bay->status.Reset(RoadStop::RoadStopStatusFlag::Bay1Free); bay->status.Set(RoadStop::RoadStopStatusFlag::EntryBusy);
	StationNeed(Station602Primary(st, v) == bay, __LINE__);
	auto *part = new RoadVehicle(); part->SetArticulatedPart(); v->SetNext(part);
	RoadStop *drive = Station602Primary(st, v);
	StationNeed(drive != nullptr && IsDriveThroughStopTile(drive->xy) && Station602Next(drive, v) == nullptr, __LINE__);
	fmt::print(stderr, "STCMD spatial602-stop occupied-articulated {} {}\n", bay->index.base(), drive->index.base());
	v->SetNext(nullptr); delete part; delete v; bay->status = status;
}
static void Station602Rail(TileIndex tile, DoCommandFlags exec)
{
	for (Axis axis : {AXIS_X, AXIS_Y}) {
		TileIndex start = tile + TileDiffXY(8 * axis, 0);
		StationNeed(Command<CMD_BUILD_RAIL_STATION>::Do(exec, start, RAILTYPE_RAIL, axis, 2, 3, STAT_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
		Station *st = Station::GetByTile(start);
		TileIndex middle = start + TileOffsByAxis(axis);
		for (bool incompatible : {false, true}) {
			SetStationTileBlocked(start + 2 * TileOffsByAxis(axis), incompatible);
			for (uint8_t mode = 0; mode < 3; ++mode) {
#ifdef WITH_RUST
				auto area = openttd_test_station_rail_area(&GetRustSharedServices(), st, middle.base(), mode);
				fmt::print(stderr, "STCMD spatial602-rail {} {} {} {} {} {}\n", axis, incompatible, mode, area.tile, area.w, area.h);
#else
				auto area = GetRailTileArea(st, middle, static_cast<TriggerArea>(mode));
				fmt::print(stderr, "STCMD spatial602-rail {} {} {} {} {} {}\n", axis, incompatible, mode, area.tile.base(), area.w, area.h);
#endif
			}
		}
		SetStationTileBlocked(start + 2 * TileOffsByAxis(axis), false);
	}
	TileIndex way = tile + TileDiffXY(18, 0);
	StationNeed(Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, way, RAILTYPE_RAIL, TRACK_X, false).Succeeded(), __LINE__);
	StationNeed(Command<CMD_BUILD_RAIL_WAYPOINT>::Do(exec, way, AXIS_X, 1, 1, STAT_CLASS_WAYP, 0, NEW_STATION, true).Succeeded(), __LINE__);
	Waypoint *wp = Waypoint::GetByTile(way);
#ifdef WITH_RUST
	auto area = openttd_test_station_rail_area(&GetRustSharedServices(), wp, INVALID_TILE.base(), 2);
	fmt::print(stderr, "STCMD spatial602-waypoint {} {} {}\n", area.tile, area.w, area.h);
#else
	auto area = GetRailTileArea(wp, INVALID_TILE, static_cast<TriggerArea>(2));
	fmt::print(stderr, "STCMD spatial602-waypoint {} {} {}\n", area.tile.base(), area.w, area.h);
#endif
}
static void Station602Fixture(TileIndex origin, DoCommandFlags exec, bool reload)
{
	_interactive_random.SetSeed(0x602);
	Station562Rng("spatial602-before");
	std::vector<Station *> seeds;
	if (!reload) {
		for (uint i = 0; i < 20; ++i) {
			Station *st = Station562New(origin + TileDiffXY(i % 3, i % 5), "Spatial602 seed");
			st->time_since_load = i % 2 == 0 ? 20 : 21; st->time_since_unload = 255;
			st->goods[0].status.Set(GoodsEntry::State::Acceptance); st->goods[0].rating = i;
			if (i == 19) st->owner = OWNER_NONE;
			seeds.push_back(st);
		}
	} else {
		for (Station *st : Station::Iterate()) if (st->name == "Spatial602 seed") seeds.push_back(st);
		StationNeed(!seeds.empty(), __LINE__); origin = seeds.front()->xy;
	}
	Station602Rebuild();
	auto visits = Station602Visit(reload ? "reload" : "build", origin, 8);
	StationNeed(!visits.empty(), __LINE__);
	Station602Visit("zero", origin, 0); Station602Visit("edge", TileXY(0, 0), 1);
	for (uint radius : {0U, 1U, 2U, 8U}) for (int amount : {-160, -1, 0, 1, 255}) {
		Station602Rating(origin, radius, amount);
		fmt::print(stderr, "STCMD spatial602-rating {} {}", radius, amount);
		for (Station *st : seeds) fmt::print(stderr, " {}:{}", st->index.base(), st->goods[0].rating);
		fmt::print(stderr, "\n");
	}
	Town *town = seeds.front()->town;
	Backup<TileIndex> location(town->xy, origin);
	Backup<std::array<uint32_t, NUM_HOUSE_ZONES>> radii(town->cache.squared_town_zone_radius); town->cache.squared_town_zone_radius.fill(1000);
	for (int16_t initial : {INT16_MIN, -1000, 999, INT16_MAX}) {
		town->ratings[_current_company] = initial;
#ifdef WITH_RUST
		auto active = openttd_test_town_stations(&GetRustTownServices(), &GetRustSharedServices(), town->index.base());
#else
		auto active = CountActiveStations(town); UpdateTownRating(town);
#endif
		fmt::print(stderr, "STCMD spatial602-town {} {} {}\n", initial, active, town->ratings[_current_company]);
	}
	location.Restore(); radii.Restore();
	if (!reload) {
		Station *moving = seeds[1]; TileIndex old = moving->xy;
		moving->rect.left = TileX(origin); moving->rect.top = TileY(origin); moving->rect.right = TileX(origin) + 4; moving->rect.bottom = TileY(origin) + 4;
		auto collision = std::get<0>(CmdMoveStationName({}, moving->index, seeds[0]->xy));
		Waypoint *wp = new Waypoint(old); wp->owner = _current_company; wp->town = moving->town;
		wp->rect.left = moving->rect.left; wp->rect.top = moving->rect.top; wp->rect.right = moving->rect.right; wp->rect.bottom = moving->rect.bottom;
		auto waypoint_collision = std::get<0>(CmdMoveWaypointName({}, wp->index, seeds[0]->xy));
		StationNeed(collision.Failed() && waypoint_collision.Failed(), __LINE__);
		fmt::print(stderr, "STCMD spatial602-collisions {} {}\n", collision.Failed(), waypoint_collision.Failed()); delete wp;
		for (TileIndex to : {old, old + TileDiffXY(2, 0), old}) {
#ifdef WITH_RUST
			openttd_test_station_move_sign(&GetRustSharedServices(), moving, to.base());
#else
			moving->MoveSign(to);
#endif
			Station602Visit("move", origin, 8);
		}
		delete seeds[3]; delete seeds[0]; Station602Visit("remove", origin, 8);
		Station562New(origin, "Spatial602 seed"); Station602Visit("reuse", origin, 8);
		Station *tie_a = Station562New(origin + TileDiffXY(20, 5), "Spatial602 tie");
		Station *tie_b = Station562New(tie_a->xy, "Spatial602 tie");
		StationID lower = std::min(tie_a->index, tie_b->index);
		TileIndex reuse_tile = origin + TileDiffXY(20, 6);
		StationNeed(Command<CMD_BUILD_RAIL_STATION>::Do(exec, reuse_tile, RAILTYPE_RAIL, AXIS_X, 1, 1, STAT_CLASS_DFLT, 0, StationID::Invalid(), true).Succeeded(), __LINE__);
		StationNeed(GetStationIndex(reuse_tile) == lower, __LINE__);
		fmt::print(stderr, "STCMD spatial602-deleted-tie {} {}\n", lower.base(), GetStationIndex(reuse_tile).base());
		Station602Stops(origin + TileDiffXY(2, 10), exec); Station602Rail(origin + TileDiffXY(2, 18), exec);
	}
	Station562Rng("spatial602");
}
