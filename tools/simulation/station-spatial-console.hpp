/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station-spatial-console.hpp Unsorted station observations and actual query consumers. */
#ifdef WITH_RUST
#include "tests/station_spatial_fixture.hpp"
#else
#include "station602-town-original.hpp"
enum TriggerArea : uint8_t;
TileArea GetRailTileArea(const BaseStation *, TileIndex, TriggerArea);
#endif
#include "vehicle-record-fixture.hpp"
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
	/* Build/sell establishes and balances owner, group counts and unit IDs. */
	TileIndex depot = tile + TileDiffXY(0, 3);
	StationNeed(Command<CMD_BUILD_ROAD_DEPOT>::Do(exec, depot, ROADTYPE_ROAD, DIAGDIR_NE).Succeeded(), __LINE__);
	EngineID engine = EngineID::Invalid();
	for (const Engine *e : Engine::IterateType(VEH_ROAD)) {
		if (engine == EngineID::Invalid() && IsEngineBuildable(e->index, VEH_ROAD, _current_company) && e->VehInfo<RoadVehicleInfo>().roadtype == ROADTYPE_ROAD && !e->info.callback_mask.Test(VehicleCallbackMask::ArticEngine)) engine = e->index;
	}
	StationNeed(engine != EngineID::Invalid(), __LINE__);
	auto build = [&] {
		auto [cost, id, refit, capacity, cargo] = Command<CMD_BUILD_VEHICLE>::Do(exec, depot, engine, false, INVALID_CARGO, INVALID_CLIENT_ID);
		StationNeed(cost.Succeeded() && RoadVehicle::Get(id)->Next() == nullptr, __LINE__); return RoadVehicle::Get(id);
	};
	RoadVehicle *v = build(), *part = build();
	auto cargo_before = v->cargo_type; auto compatible_before = v->compatible_roadtypes;
	auto part_subtype = FIXTURE_FIELD(part, subtype);
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
	FIXTURE_FIELD(part, subtype) = 0; part->SetArticulatedPart(); v->SetNext(part);
	RoadStop *drive = Station602Primary(st, v);
	StationNeed(drive != nullptr && IsDriveThroughStopTile(drive->xy) && Station602Next(drive, v) == nullptr, __LINE__);
	fmt::print(stderr, "STCMD spatial602-stop occupied-articulated {} {}\n", bay->index.base(), drive->index.base());
	v->SetNext(nullptr); FIXTURE_FIELD(part, subtype) = part_subtype; bay->status = status;
	v->cargo_type = cargo_before; v->compatible_roadtypes = compatible_before;
	for (RoadVehicle *vehicle : {part, v}) StationNeed(Command<CMD_SELL_VEHICLE>::Do(exec, vehicle->index, false, false, INVALID_CLIENT_ID).Succeeded(), __LINE__);
	StationNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(exec, depot).Succeeded(), __LINE__);
}
/* Real Action2 callbacks publish each draw into its receiving tile. Expected
 * tile/platform/whole coverage is enumerated independently from GetRailTileArea. */
#include "animated_tile_func.h"
static void Station602Triggers(Station *st, Axis axis)
{
	GRFFile file; file.grfid = 0x602; file.grf_version = 8;
	StationSpec spec; spec.grf_prop.grffile = &file; spec.cargo_triggers = 1;
	StationNeed(SpriteGroup::CanAllocateItem(3), __LINE__);
	auto *result = new CallbackResultSpriteGroup(0);
	auto *animation = new DeterministicSpriteGroup(); animation->var_scope = VSG_SCOPE_SELF; animation->size = DSG_SIZE_DWORD;
	animation->default_result = {true, nullptr};
	/* TriggerStationAnimation passes random bits in var10; var18 is the trigger. */
	animation->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x10, .and_mask = 0x7F}};
	auto *random = new RandomizedSpriteGroup(); random->var_scope = VSG_SCOPE_SELF; random->lowest_randbit = 16; random->groups = {result, result};
	st->speclist.resize(2); st->speclist[1] = {&spec, file.grfid, 0};
	TileIndex start = st->train_station.tile, middle = start + TileOffsByAxis(axis), end = start + 2 * TileOffsByAxis(axis);
	for (TileIndex t : st->train_station) SetCustomStationSpecIndex(t, 1);
	for (bool blocked : {false, true}) for (uint8_t mode = 0; mode < 3; ++mode) {
		SetStationTileBlocked(end, blocked);
		auto covered = [&](TileIndex t) {
			if (mode == 0) return t == middle;
			if (mode == 2) return true;
			return (axis == AXIS_X ? TileY(t) == TileY(start) : TileX(t) == TileX(start)) && (!blocked || t != end);
		};
		auto trigger = mode == 0 ? StationAnimationTrigger::Built : mode == 1 ? StationAnimationTrigger::VehicleArrives : StationAnimationTrigger::NewCargo;
		spec.animation.triggers = {trigger}; st->cached_anim_triggers = {trigger};
		spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, animation);
		StationNeed(GetStationSpec(middle) == &spec, __LINE__);
		StationResolverObject program(&spec, st, middle, CBID_STATION_ANIMATION_TRIGGER, 0x12345678, 7);
		StationNeed(program.root_spritegroup == animation && program.station_scope.cargo_type == CargoGRFFileProps::SG_DEFAULT && program.ResolveCallback({}) == 0x78, __LINE__);
		for (TileIndex t : st->train_station) SetAnimationFrame(t, 0x80);
		auto expected = _random; expected.Next();
		TriggerStationAnimation(st, middle, trigger);
		fmt::print(stderr, "STCMD spatial602-animation {} {} {}", axis, blocked, mode);
		for (TileIndex t : st->train_station) {
			uint8_t frame = covered(t) ? expected.Next() & 0x7F : 0x80;
			if (GetAnimationFrame(t) != frame) fmt::print(stderr, " STCMD spatial602-frame-mismatch {} {} {} {} {} {}\n", axis, blocked, mode, t.base(), GetAnimationFrame(t), frame);
			StationNeed(GetAnimationFrame(t) == frame, __LINE__);
			fmt::print(stderr, " {}:{}", t.base(), GetAnimationFrame(t)); DeleteAnimatedTile(t);
		}
		StationNeed(std::ranges::equal(_random.state, expected.state), __LINE__);
		fmt::print(stderr, " rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
		/* Random triggers have platform and whole coverage, without a tile case. */
		if (mode == 0) continue;
		auto rt = mode == 1 ? StationRandomTrigger::VehicleArrives : StationRandomTrigger::NewCargo;
		random->triggers = 1U << to_underlying(rt); st->cached_cargo_triggers = 1;
		spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, random);
		StationResolverObject reseed(&spec, st, middle, CBID_RANDOM_TRIGGER);
		StationNeed(reseed.root_spritegroup == random && reseed.station_scope.cargo_type == CargoGRFFileProps::SG_DEFAULT, __LINE__);
		reseed.SetWaitingRandomTriggers(StationRandomTriggers{rt}); reseed.ResolveRerandomisation();
		StationNeed(reseed.GetReseedSum() == (1U << 16) && reseed.GetUsedRandomTriggers().base() == random->triggers, __LINE__);
		for (TileIndex t : st->train_station) SetStationTileRandomBits(t, 0);
		expected = _random; TriggerStationRandomisation(st, middle, rt);
		fmt::print(stderr, "STCMD spatial602-random {} {} {}", axis, blocked, mode);
		for (TileIndex t : st->train_station) {
			uint8_t bits = covered(t) ? expected.Next() & 1 : 0;
			StationNeed(GetStationTileRandomBits(t) == bits, __LINE__);
			fmt::print(stderr, " {}:{}", t.base(), GetStationTileRandomBits(t));
		}
		StationNeed(std::ranges::equal(_random.state, expected.state), __LINE__);
		fmt::print(stderr, " rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	}
	SetStationTileBlocked(end, false);
	for (TileIndex t : st->train_station) SetCustomStationSpecIndex(t, 0);
	st->speclist.clear(); StationUpdateCachedTriggers(st);
	delete random; delete animation; delete result;
}
static void Station602Rail(TileIndex tile, DoCommandFlags exec)
{
	for (Axis axis : {AXIS_X, AXIS_Y}) {
		TileIndex start = tile + TileDiffXY(8 * axis, 0);
		StationNeed(Command<CMD_BUILD_RAIL_STATION>::Do(exec, start, RAILTYPE_RAIL, axis, 2, 3, STAT_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
		Station *st = Station::GetByTile(start); st->name = fmt::format("Spatial602 rail {}", axis);
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
		Station602Triggers(st, axis);
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
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "script/api/script_event_types.hpp"
#include "news_func.h"
#include "sound_func.h"
#include "train.h"
static Station *_station602_crash = nullptr;
static uint _station602_crash_step = 0, _station602_crash_victims = 0, _station602_other_events = 0;
static Randomizer _station602_crash_rng;
static void Station602CrashStep(uint step, const char *operation)
{
	if (_station602_crash_step != step || _station602_crash->goods[0].rating != (step == 3 ? 40 : 200)) fmt::print(stderr, "STCMD spatial602-crash-step-mismatch {} {} {}\n", step, _station602_crash_step, _station602_crash->goods[0].rating);
	StationNeed(_station602_crash_step++ == step && _station602_crash->goods[0].rating == (step == 3 ? 40 : 200), __LINE__);
	fmt::print(stderr, "STCMD spatial602-crash {} {} {}\n", step, operation, _station602_crash->goods[0].rating);
}
static void Station602CrashEvent(uint step, const char *operation, ScriptEvent *event)
{
	auto *crash = ScriptEventVehicleCrashed::Convert(event);
	int type = event == nullptr ? -1 : static_cast<int>(event->GetEventType());
	if (crash != nullptr) fmt::print(stderr, "STCMD spatial602-crash-event {} type {} id {} site {} reason {} victims {} expected {} rng {} {}\n", operation, type, crash->GetVehicleID().base(), crash->GetCrashSite().base(), static_cast<int>(crash->GetCrashReason()), crash->GetVictims(), _station602_crash_victims, _random.state[0], _random.state[1]);
	if (type != ScriptEvent::ET_VEHICLE_CRASHED) {
		if (_station602_other_events++ < 8) fmt::print(stderr, "STCMD spatial602-crash-other-event {} {} {}\n", operation, type, crash != nullptr);
		StationNeed(event != nullptr && crash == nullptr, __LINE__); return;
	}
	if (crash == nullptr) fmt::print(stderr, "STCMD spatial602-crash-event {} type {} null-cast\n", operation, type);
	StationNeed(crash != nullptr && crash->GetVehicleID() == VehicleID(21) && crash->GetCrashSite() == TileIndex(10005) && crash->GetCrashReason() == ScriptEventVehicleCrashed::CRASH_RV_LEVEL_CROSSING && crash->GetVictims() == _station602_crash_victims, __LINE__);
	StationNeed(std::ranges::equal(_random.state, _station602_crash_rng.state), __LINE__);
	Station602CrashStep(step, operation);
}
extern "C" void __real__ZN2AI8NewEventE6PoolIDIh12CompanyIDTagLh15ELh255EEP11ScriptEvent(CompanyID, ScriptEvent *);
extern "C" void __wrap__ZN2AI8NewEventE6PoolIDIh12CompanyIDTagLh15ELh255EEP11ScriptEvent(CompanyID company, ScriptEvent *event)
{
	if (_station602_crash != nullptr) Station602CrashEvent(0, "ai-event", event);
	__real__ZN2AI8NewEventE6PoolIDIh12CompanyIDTagLh15ELh255EEP11ScriptEvent(company, event);
}
extern "C" void __real__ZN4Game8NewEventEP11ScriptEvent(ScriptEvent *);
extern "C" void __wrap__ZN4Game8NewEventEP11ScriptEvent(ScriptEvent *event)
{
	if (_station602_crash != nullptr) Station602CrashEvent(1, "game-event", event);
	__real__ZN4Game8NewEventEP11ScriptEvent(event);
}
extern "C" void Station602RealNews(EncodedString &&, NewsType, NewsStyle, NewsFlags, NewsReference, NewsReference, std::unique_ptr<NewsAllocatedData> &&, AdviceType) asm("__real_STATION602_NEWS_SYMBOL");
extern "C" void Station602WrapNews(EncodedString &&headline, NewsType type, NewsStyle style, NewsFlags flags, NewsReference ref1, NewsReference ref2, std::unique_ptr<NewsAllocatedData> &&data, AdviceType advice) asm("__wrap_STATION602_NEWS_SYMBOL");
extern "C" void Station602WrapNews(EncodedString &&headline, NewsType type, NewsStyle style, NewsFlags flags, NewsReference ref1, NewsReference ref2, std::unique_ptr<NewsAllocatedData> &&data, AdviceType advice)
{
	if (_station602_crash != nullptr) Station602CrashStep(2, "news");
	Station602RealNews(std::move(headline), type, style, flags, ref1, ref2, std::move(data), advice);
}
extern "C" void __real__Z16SndPlayVehicleFxtPK7Vehicle(SoundID, const Vehicle *);
extern "C" void __wrap__Z16SndPlayVehicleFxtPK7Vehicle(SoundID sound, const Vehicle *vehicle)
{
	if (_station602_crash != nullptr) {
		if (sound == SND_12_EXPLOSION) {
			if (vehicle->index != VehicleID(21)) fmt::print(stderr, "STCMD spatial602-crash-sound-mismatch {} {}\n", sound, vehicle->index.base());
			StationNeed(vehicle->index == VehicleID(21), __LINE__); Station602CrashStep(3, "sound");
		} else if (_station602_other_events++ < 8) fmt::print(stderr, "STCMD spatial602-crash-other-sound {} {}\n", sound, vehicle->index.base());
	}
	__real__Z16SndPlayVehicleFxtPK7Vehicle(sound, vehicle);
}
static void Station602Crash(bool reload)
{
	if (reload) {
		Station *st = nullptr; for (Station *s : Station::Iterate()) if (s->name == "Spatial602 crash") st = s;
		fmt::print(stderr, "STCMD spatial602-crash-reload-state {} {} {}\n", st != nullptr, st == nullptr ? -1 : st->goods[0].rating, FIXTURE_GET(RoadVehicle::Get(VehicleID(21)), VehStatus, vehstatus).Test(VehState::Crashed));
		StationNeed(st != nullptr && st->goods[0].rating == 40 && FIXTURE_GET(RoadVehicle::Get(VehicleID(21)), VehStatus, vehstatus).Test(VehState::Crashed), __LINE__);
		fmt::print(stderr, "STCMD spatial602-crash-reload {}\n", st->goods[0].rating); return;
	}
	/* Same command-built crossing and live vehicles as roads-level-crossing. */
	auto *train = Train::Get(VehicleID(17)); auto *bus = RoadVehicle::Get(VehicleID(21));
	StationNeed(IsLevelCrossingTile(TileIndex(10005)) && !FIXTURE_GET(bus, VehStatus, vehstatus).Test(VehState::Crashed), __LINE__);
	for (Vehicle *v : {static_cast<Vehicle *>(train), static_cast<Vehicle *>(bus)}) {
		FIXTURE_SET(v, Tile, tile, TileIndex(10005)); FIXTURE_FIELD(v, x_pos) = 344; FIXTURE_FIELD(v, y_pos) = 632; v->UpdatePosition();
	}
	FIXTURE_SET(train, VehStatus, vehstatus, VehStates(10)); FIXTURE_SET(bus, Direction, direction, DIR_SE);
#ifdef WITH_RUST
	train->SetTrack(TRACK_BIT_X); bus->SetState(1);
#else
	train->track = TRACK_BIT_X; bus->state = 1;
#endif
	Station *st = Station562New(FIXTURE_GET(bus, Tile, tile), "Spatial602 crash"); st->owner = FIXTURE_GET(bus, Owner, owner);
	st->goods[0].status.Set(GoodsEntry::State::Acceptance); st->goods[0].rating = 200;
	Backup<bool> sound(_settings_client.sound.disaster, true);
	uint passengers = 0;
	for (RoadVehicle *part = bus; part != nullptr; part = part->Next()) if (IsCargoInClass(part->cargo_type, CargoClass::Passengers)) passengers += part->cargo.TotalCount();
	StationNeed(bus->IsFrontEngine() && bus->Next() == nullptr && passengers == 20, __LINE__);
	/* Original Vehicle::Crash samples all passengers once; RoadVehicle adds its driver. */
	_station602_crash_rng = _random; _station602_crash_victims = _station602_crash_rng.Next(passengers + 1) + 1;
	fmt::print(stderr, "STCMD spatial602-crash-input {} {} {} {} {} {}\n", passengers, _station602_crash_victims, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	_station602_crash_step = 0; _station602_other_events = 0; _station602_crash = st; StationNeed(bus->Tick(), __LINE__); _station602_crash = nullptr;
	StationNeed(_station602_crash_step == 4 && st->goods[0].rating == 40 && FIXTURE_GET(bus, VehStatus, vehstatus).Test(VehState::Crashed), __LINE__);
	sound.Restore(); Station562Rng("spatial602-crash");
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
		StationNeed(Waypoint::CanAllocateItem(), __LINE__);
		Waypoint *wp = new Waypoint(old); wp->owner = _current_company; wp->town = moving->town;
		wp->rect.left = moving->rect.left; wp->rect.top = moving->rect.top; wp->rect.right = moving->rect.right; wp->rect.bottom = moving->rect.bottom;
		auto waypoint_collision = std::get<0>(CmdMoveWaypointName({}, wp->index, seeds[0]->xy));
		StationNeed(collision.Failed() && waypoint_collision.Failed(), __LINE__);
		fmt::print(stderr, "STCMD spatial602-collisions {} {}\n", collision.Failed(), waypoint_collision.Failed()); delete wp;
		for (auto [phase, to] : {std::pair{"move-unchanged", old}, std::pair{"move-away", old + TileDiffXY(2, 0)}, std::pair{"move-back", old}}) {
#ifdef WITH_RUST
			openttd_test_station_move_sign(&GetRustSharedServices(), moving, to.base());
#else
			moving->MoveSign(to);
#endif
			Station602Visit(phase, origin, 8);
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
	if (reload) for (Axis axis : {AXIS_X, AXIS_Y}) {
		Station *found = nullptr;
		for (Station *st : Station::Iterate()) if (st->name == fmt::format("Spatial602 rail {}", axis)) found = st;
		StationNeed(found != nullptr, __LINE__); Station602Triggers(found, axis);
	}
	Station602Crash(reload);
	Station562Rng("spatial602");
}
