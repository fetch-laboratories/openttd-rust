/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station-resolver-console.hpp Controlled actual Action2 programs, run by both executables. */
#include "newgrf_station.h"
#include "newgrf.h"
#include "newgrf_spritegroup.h"
static void StationResolverFixture(TileIndex origin, DoCommandFlags exec, bool reloaded = false)
{
	Station *st = nullptr; Waypoint *wp = nullptr;
	if (reloaded) {
		for (Station *candidate : Station::Iterate()) if (candidate->name == "Resolver station") st = candidate;
		for (Waypoint *candidate : Waypoint::Iterate()) if (candidate->name == "Resolver waypoint") wp = candidate;
		StationNeed(st != nullptr && wp != nullptr, __LINE__);
	} else {
		TileIndex tile = origin + TileDiffXY(2, 2), waytile = origin + TileDiffXY(6, 6);
		StationNeed(Command<CMD_BUILD_RAIL_STATION>::Do(exec, tile, RAILTYPE_RAIL, AXIS_X, 2, 3, STAT_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
		st = Station::GetByTile(tile); st->name = "Resolver station";
		_settings_game.station.station_spread = 32;
		StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, origin + TileDiffXY(8, 2), 1, 1, RoadStopType::Truck, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, st->index, true).Succeeded(), __LINE__);
		StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, origin + TileDiffXY(10, 2), 1, 1, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, st->index, true).Succeeded(), __LINE__);
		StationNeed(Command<CMD_BUILD_AIRPORT>::Do(exec, origin + TileDiffXY(14, 2), AT_SMALL, 0, st->index, true).Succeeded(), __LINE__);

		StationNeed(Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, waytile, RAILTYPE_RAIL, TRACK_X, false).Succeeded(), __LINE__);
		StationNeed(Command<CMD_BUILD_RAIL_WAYPOINT>::Do(exec, waytile, AXIS_X, 1, 1, STAT_CLASS_WAYP, 0, StationID::Invalid(), false).Succeeded(), __LINE__);
		wp = Waypoint::GetByTile(waytile); wp->name = "Resolver waypoint";
		StationNeed(CargoPacket::CanAllocateItem(2), __LINE__);
		st->goods[0].GetOrCreateData().cargo.Append(new CargoPacket(4133, 5, st->index, tile, 0), StationID::Invalid());
		st->goods[1].GetOrCreateData().cargo.Append(new CargoPacket(9, 13, st->index, tile, 0), StationID::Invalid());
	}
	TileIndex tile = st->train_station.tile, waytile = wp->train_station.tile;
	st->truck_stops->status.Reset(RoadStop::RoadStopStatusFlag::Bay0Free);
	st->airport.blocks.Set({AirportBlock::Term1, AirportBlock::RunwayIn, AirportBlock::AirportClosed});

	fmt::print(stderr, "STCMD resolver-reconstruction {} {} {} {}\n", reloaded, st->index.base(), st->goods[0].TotalCount(), st->goods[0].GetData().cargo.PeriodsInTransit());
	GRFFile file; file.grfid = 0x52533939; file.grf_version = 8;
	StationSpec spec; spec.grf_prop.grffile = &file; spec.grf_prop.grfid = file.grfid; spec.grf_prop.local_id = 19;
	StationNeed(SpriteGroup::CanAllocateItem(6), __LINE__);
	auto *result = new CallbackResultSpriteGroup(0x400);
	auto *det = new DeterministicSpriteGroup(); det->var_scope = VSG_SCOPE_SELF; det->size = DSG_SIZE_DWORD;
	det->default_result = {false, result};
	spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det);
	spec.grf_prop.SetSpriteGroup(0, det);
	StationResolverObject object(&spec, st, tile);
	StationNeed(object.station_scope.cargo_type == 0, __LINE__);
	for (uint8_t var : {0x40, 0x41, 0x45, 0x46, 0x47, 0x49}) {
		det->adjusts = {{.operation = DSGA_OP_RST, .variable = var, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
		std::array<int32_t, 1> regs{};
		StationNeed(object.ResolveCallback(regs) == 0x400, __LINE__); auto first = regs[0];
		/* A persistent object retains all six caches across independent DoResolve calls. */
		auto gfx = GetStationGfx(tile);
		SetStationGfx(tile, gfx ^ 2);
		if (var == 0x45) StationNeed(Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, tile + TileDiffXY(-1, 0), RAILTYPE_RAIL, TRACK_X, false).Succeeded(), __LINE__);
		object.ResolveCallback(regs); StationNeed(regs[0] == first, __LINE__);
		{ StationResolverObject fresh(&spec, st, tile); std::array<int32_t, 1> other{}; StationNeed(fresh.ResolveCallback(other) == 0x400, __LINE__); StationNeed(other[0] != first, __LINE__); }
		if (var == 0x45) StationNeed(Command<CMD_REMOVE_SINGLE_RAIL>::Do(exec, tile + TileDiffXY(-1, 0), TRACK_X).Succeeded(), __LINE__);
		SetStationGfx(tile, gfx);
		fmt::print(stderr, "STCMD resolver-cache {:x} {} {}\n", var, first, regs[0]);
	}
	/* Every variable is observed natively for availability and through actual
	 * deterministic Action2 for callback/register results, including deprecated
	 * cargo encodings and unavailable branches. Associations are transient. */
	st->speclist.resize(2); st->speclist[1] = {&spec, file.grfid, 19};
	wp->speclist.resize(2); wp->speclist[1] = {&spec, file.grfid, 19};
	for (TileIndex t : st->train_station) if (st->TileBelongsToRailStation(t)) SetCustomStationSpecIndex(t, 1);
	SetCustomStationSpecIndex(waytile, 1);
	file.cargo_list = {CargoSpec::Get(0)->label, CargoSpec::Get(1)->label};
	auto variables = [&](const char *kind, BaseStation *base, TileIndex t, Axis axis) {
		StationResolverObject query(&spec, base, t); query.station_scope.axis = axis;
		for (uint var = 0x40; var <= 0xFA; ++var) {
			for (uint8_t parameter : {0, 1, 2, 0x10, 0xFF}) {
				bool available = true; auto value = query.station_scope.GetVariable(var, parameter, available);
				det->adjusts = {{.operation = DSGA_OP_RST, .variable = static_cast<uint8_t>(var), .parameter = parameter, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
				std::array<int32_t, 1> registers{}; auto cb = query.ResolveCallback(registers);
				if (available) StationNeed(cb == 0x400 && static_cast<uint32_t>(registers[0]) == value, __LINE__);
				fmt::print(stderr, "STCMD resolver-variable {} {} {:x} {} {} {} {} {}\n", kind, file.grf_version, var, parameter, available, value, cb, registers[0]);
			}
		}
	};
	for (uint8_t version : {6, 7, 8}) for (uint8_t status : {0, 1, 2, 0x3C, 0x3F}) {
		file.grf_version = version;
		auto &ge = st->goods[0]; ge.status = GoodsEntry::States(status); ge.time_since_pickup = 27; ge.rating = 123;
		ge.last_speed = status == 0 ? 0 : 77; ge.last_age = 11;
		variables("station", st, tile, INVALID_AXIS);
		fmt::print(stderr, "STCMD resolver-state {} {}\n", status, ge.ConvertState());
	}
	variables("waypoint", wp, waytile, INVALID_AXIS);
	variables("purchase", nullptr, INVALID_TILE, INVALID_AXIS);
	variables("slope-x", nullptr, tile, AXIS_X); variables("slope-y", nullptr, tile, AXIS_Y);
	/* Lazy town selection remains stable over repeated resolutions; a parent
	 * Action2 program exercises native persistent storage reentry. */
	StationNeed(object.GetTown() != nullptr, __LINE__);
	auto *parent = object.GetTown(); auto *original_town = st->town; st->town = nullptr;
#ifdef WITH_RUST
	StationNeed(object.GetTown() == parent && parent->context.town == original_town, __LINE__);
#else
	StationNeed(object.GetTown() == parent && parent->t == original_town, __LINE__);
#endif
	st->town = original_town;
	det->var_scope = VSG_SCOPE_PARENT;
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}, {.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0x399}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 3}, {.operation = DSGA_OP_RST, .variable = 0x7C, .parameter = 3, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101}};
	std::array<int32_t, 2> parent_regs{}; StationNeed(object.ResolveCallback(parent_regs) == 0x400 && parent_regs[1] == 0x399, __LINE__);
	StationResolverObject prototype(&spec, nullptr, tile);
#ifdef WITH_RUST
	StationNeed(prototype.GetTown() != nullptr && prototype.GetTown()->context.readonly, __LINE__);
#else
	StationNeed(prototype.GetTown() != nullptr && prototype.GetTown()->readonly, __LINE__);
#endif
	StationResolverObject no_town(&spec, nullptr, INVALID_TILE); StationNeed(no_town.GetTown() == nullptr, __LINE__);
	StationNeed(no_town.ResolveCallback({}) == CALLBACK_FAILED, __LINE__);
	det->var_scope = VSG_SCOPE_RELATIVE;
	StationNeed(object.ResolveCallback({}) == CALLBACK_FAILED, __LINE__); det->var_scope = VSG_SCOPE_PARENT;
	/* A readonly purchase parent cannot change the shared town storage. */
	det->adjusts[2].and_mask = 0x400; StationNeed(prototype.ResolveCallback(parent_regs) == 0x400 && parent_regs[1] == 0x399, __LINE__); det->adjusts[2].and_mask = 0x399;
	st->town = nullptr; StationResolverObject retry(&spec, st, tile);
	StationNeed(retry.ResolveCallback({}) == CALLBACK_FAILED, __LINE__); st->town = original_town;
	StationNeed(retry.ResolveCallback(parent_regs) == 0x400 && parent_regs[1] == 0x399, __LINE__);
	fmt::print(stderr, "STCMD resolver-scope-fallback parent relative readonly retry\n");

#ifdef WITH_RUST
	fmt::print(stderr, "STCMD resolver-parent {} {} {}\n", parent->context.town->index.base(), parent_regs[1], prototype.GetTown()->context.town->index.base());
#else
	fmt::print(stderr, "STCMD resolver-parent {} {} {}\n", parent->t->index.base(), parent_regs[1], prototype.GetTown()->t->index.base());
#endif
	det->var_scope = VSG_SCOPE_SELF;
	/* Real-group thresholds, division, fallbacks and cargo-map order use the
	 * same actual groups in both games. New owners snapshot root choice only. */
	auto *other = new CallbackResultSpriteGroup(0x401);
	auto *real = new RealSpriteGroup(); real->loading = {result, other}; real->loaded = {other, result};
	for (auto cargo : {CargoType{1}, CargoType{0}, CargoGRFFileProps::SG_DEFAULT, CargoGRFFileProps::SG_DEFAULT_NA, CargoGRFFileProps::SG_PURCHASE}) {
		spec.grf_prop.spritegroups.clear(); spec.grf_prop.SetSpriteGroup(cargo, real);
		if (cargo == 1) spec.grf_prop.SetSpriteGroup(0, other);
		for (bool divide : {false, true}) for (uint16_t threshold : {0, 8, 9, 36, 37, 4095, 65535}) {
			spec.cargo_threshold = threshold; spec.flags.Set(StationSpecFlag::DivByStationSize, divide);
			StationResolverObject choice(&spec, st, tile);
			fmt::print(stderr, "STCMD resolver-root {} {} {} {} {}\n", cargo, divide, threshold, choice.station_scope.cargo_type, choice.ResolveCallback({}));
		}
	}
	for (BaseStation *base : {static_cast<BaseStation *>(wp), static_cast<BaseStation *>(nullptr)}) {
		StationResolverObject choice(&spec, base, base == nullptr ? INVALID_TILE : waytile);
		fmt::print(stderr, "STCMD resolver-root-null {} {} {}\n", base == nullptr, choice.station_scope.cargo_type, choice.ResolveCallback({}));
	}
	for (uint8_t empty : {0, 1, 2, 3}) {
		real->loading = (empty & 1) ? std::vector<const SpriteGroup *>{} : std::vector<const SpriteGroup *>{result, other};
		real->loaded = (empty & 2) ? std::vector<const SpriteGroup *>{} : std::vector<const SpriteGroup *>{other, result};
		spec.grf_prop.spritegroups.clear(); spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, real); spec.flags.Reset(StationSpecFlag::DivByStationSize);
		for (uint16_t threshold : {36, 4095}) { spec.cargo_threshold = threshold; StationResolverObject choice(&spec, st, tile); fmt::print(stderr, "STCMD resolver-real-empty {} {} {}\n", empty, threshold, choice.ResolveCallback({})); }
	}
	spec.grf_prop.spritegroups.clear(); spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det); spec.grf_prop.SetSpriteGroup(0, det);
	det->adjusts.clear();
	StationNeed(GetStationCallback(CBID_STATION_LAND_SLOPE_CHECK, 1, 2, &spec, st, tile) == 0x400, __LINE__);
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x10, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
	for (uint8_t version : {6, 7, 8}) for (Axis axis : {AXIS_X, AXIS_Y}) for (uint16_t value : std::array<uint16_t, 3>{0x400, 0x401, CALLBACK_FAILED}) {
		file.grf_version = version; result->result = value;
		StationCost("resolver-slope", PerformStationTileSlopeCheck(tile, tile, &spec, axis, 3, 2));
	}
	/* Actual randomized Action2 requests lower-only reseeding: every member draws
	 * once for its tile even though the upper reseed mask is zero, then the
	 * station draws once. The state oracle advances a saved native RNG directly. */
	/* Scratch corner mutations expose slope nibble rotation and the distinct
	 * live/null nearby-axis rules; restore canonical terrain before simulation. */
	result->result = 0x400;
	for (TileIndex corner : {tile + TileDiffXY(1, 0), tile + TileDiffXY(0, 1)}) {
		auto height = TileHeight(corner); SetTileHeight(corner, height + 1);
		for (uint8_t version : {6, 7, 8}) for (Axis axis : {AXIS_X, AXIS_Y}) {
			file.grf_version = version; det->default_result = {false, result}; variables("terrain-live", st, tile, axis);
			variables("terrain-null", nullptr, tile, axis); det->default_result = {true, nullptr};
			for (uint8_t var : {0x10, 0x18, 0x67}) for (uint8_t shift : {0, 1, 3, 4, 7, 8}) {
				det->adjusts = {{.operation = DSGA_OP_RST, .variable = var, .shift_num = shift, .and_mask = 1}, {.operation = DSGA_OP_OR, .variable = 0x1A, .and_mask = 0x400}};
				StationCost("resolver-slope-policy", PerformStationTileSlopeCheck(tile - TileDiffXY(1, 2), tile, &spec, axis, 3, 2));
			}
		}
		SetTileHeight(corner, height);
	}
	det->default_result = {false, result};
	file.grf_version = 8;
	auto *random = new RandomizedSpriteGroup(); random->var_scope = VSG_SCOPE_SELF;
	random->triggers = 1; random->lowest_randbit = 0; random->groups = {result, result};
	spec.grf_prop.spritegroups.clear(); spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, random); spec.cargo_triggers = 1;
	st->speclist.resize(2); st->speclist[1] = {&spec, file.grfid, 19};
	uint count = 0;
	for (TileIndex t : st->train_station) if (st->TileBelongsToRailStation(t)) { SetCustomStationSpecIndex(t, 1); ++count; }
	st->cached_cargo_triggers = 1;
	auto expected = _random; for (uint i = 0; i < count + 1; ++i) expected.Next();
	TriggerStationRandomisation(st, tile, StationRandomTrigger::NewCargo, 0);
	StationNeed(_random.state[0] == expected.state[0] && _random.state[1] == expected.state[1], __LINE__);
	fmt::print(stderr, "STCMD resolver-rng lower {} {} {} {}\n", count, st->waiting_random_triggers.base(), _random.state[0], _random.state[1]);
	/* Upper-only, combined, trigger mismatch, cached-mask exits and CargoTaken
	 * all-empty gating. Ordered per-tile bits and final consumed-trigger removal
	 * are observable, including publication before each common resolution. */
	for (uint8_t bit : {0, 16, 17}) for (auto trigger : {StationRandomTrigger::NewCargo, StationRandomTrigger::CargoTaken, StationRandomTrigger::VehicleArrives, StationRandomTrigger::VehicleDeparts, StationRandomTrigger::VehicleLoads, StationRandomTrigger::PathReservation}) {
		random->lowest_randbit = bit; random->triggers = 1U << to_underlying(trigger);
		for (CargoTypes mask : {CargoTypes{0}, CargoTypes{1}, CargoTypes{2}, CargoTypes{1} << 63}) {
			spec.cargo_triggers = mask; st->cached_cargo_triggers = mask;
			TriggerStationRandomisation(st, tile, trigger, 0);
			fmt::print(stderr, "STCMD resolver-rng cases {} {} {} {} {} {} {}", bit, to_underlying(trigger), mask, st->waiting_random_triggers.base(), st->random_bits, _random.state[0], _random.state[1]);
			for (TileIndex t : st->train_station) if (st->TileBelongsToRailStation(t)) fmt::print(stderr, " {}:{}", t.base(), GetStationTileRandomBits(t));
			fmt::print(stderr, "\n");
		}
	}
	/* ALL consumes a previously waiting CargoTaken together with NewCargo. */
	random->cmp_mode = RSG_CMP_ALL; random->triggers = 3; random->lowest_randbit = 0; spec.cargo_triggers = 1; st->cached_cargo_triggers = 1;
	TriggerStationRandomisation(st, tile, StationRandomTrigger::NewCargo, 0);
	StationNeed(st->waiting_random_triggers.None(), __LINE__);
	fmt::print(stderr, "STCMD resolver-rng all {} {} {}\n", st->random_bits, _random.state[0], _random.state[1]); random->cmp_mode = RSG_CMP_ANY;
	/* Nested lower/upper groups combine masks in one resolution. */
	auto *upper = new RandomizedSpriteGroup(); upper->var_scope = VSG_SCOPE_SELF; upper->triggers = 1; upper->lowest_randbit = 16; upper->groups = {result, result};
	random->lowest_randbit = 0; random->triggers = 1; random->groups = {upper, upper}; spec.cargo_triggers = 1; st->cached_cargo_triggers = 1;
	TriggerStationRandomisation(st, tile, StationRandomTrigger::NewCargo, INVALID_CARGO);
	fmt::print(stderr, "STCMD resolver-rng combined {} {} {} {}\n", st->random_bits, GetStationTileRandomBits(tile), _random.state[0], _random.state[1]);
	random->groups = {result, result}; random->lowest_randbit = 17;
	/* Waypoints use the same runtime without station cargo ownership. */
	random->triggers = 4; wp->cached_cargo_triggers = 1;
	TriggerStationRandomisation(wp, waytile, StationRandomTrigger::VehicleArrives, INVALID_CARGO);
	fmt::print(stderr, "STCMD resolver-rng waypoint {} {} {} {} {}\n", wp->waiting_random_triggers.base(), wp->random_bits, GetStationTileRandomBits(waytile), _random.state[0], _random.state[1]);
	/* A cargo absent from the station admits CargoTaken and its whole-area draw. */
	random->triggers = 2; spec.cargo_triggers = CargoTypes{1} << 63; st->cached_cargo_triggers = spec.cargo_triggers;
	TriggerStationRandomisation(st, tile, StationRandomTrigger::CargoTaken, 63);
	fmt::print(stderr, "STCMD resolver-rng empty {} {} {} {}\n", st->waiting_random_triggers.base(), st->random_bits, _random.state[0], _random.state[1]);
	for (TileIndex t : st->train_station) if (st->TileBelongsToRailStation(t)) SetCustomStationSpecIndex(t, 0);
	SetCustomStationSpecIndex(waytile, 0); wp->speclist.clear(); StationUpdateCachedTriggers(wp);
	st->speclist.clear(); StationUpdateCachedTriggers(st);
	/* Saved random/waiting/cargo fields persist; local Action2 programs and spec
	 * associations are removed before save and reconstructed by resolver-inspect. */
	delete real; delete other; delete upper; delete random; delete det; delete result;
}

#include "newgrf_config.h"
#include "newgrf_badge.h"
#include "newgrf_roadstop.h"
#include "newgrf/newgrf_internal.h"
#include "strings_func.h"
static uint32_t StationDefinitionID()
{
	return uint32_t('S') | uint32_t('4') << 8 | uint32_t('8') << 16 | uint32_t('4') << 24;
}
/* Full loading frees failed specs. These production-entry observations retain the
 * original ByteReader frame to distinguish partial mutations before that cleanup. */
static void StationDefinitionPartialReads()
{
	GRFFile file; file.grf_version = 8;
	AutoRestoreBackup current(_cur_gps.grffile, &file);
	AutoRestoreBackup skip(_cur_gps.skip_sprites, 0);
	auto read = [&](GrfSpecFeature feature, uint32_t first, uint32_t last, int property, std::initializer_list<uint8_t> data) {
		ByteReader reader(data.begin(), data.size());
		int result = -1; bool bounds = false;
		try {
			result = feature == GSF_STATIONS ? GrfChangeInfoHandler<GSF_STATIONS>::Activation(first, last, property, reader) : GrfChangeInfoHandler<GSF_ROADSTOPS>::Activation(first, last, property, reader);
		} catch (const OTTDByteReaderSignal &) { bounds = true; }
		fmt::print(stderr, "STCMD definitions-partial {} {} {} {} {} {}\n", to_underlying(feature), property, first, result, bounds, reader.Remaining());
		return bounds;
	};
	StationNeed(read(GSF_STATIONS, 0, 1, 8, {'D', 'F'}) && file.stations[0] != nullptr, __LINE__);
	StationNeed(!read(GSF_STATIONS, 0, 1, 8, {'D', 'F', 'L', 'T'}), __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 0x16, {7}) && file.stations[0]->animation.frames == 7, __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 0x20, {3, 4, 5}), __LINE__);
	StationNeed(file.stations[0]->bridgeable_info.size() == 3 && file.stations[0]->bridgeable_info[1].height == 5 && file.stations[0]->bridgeable_info[2].height == 0, __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 9, {2, 1, 0}) && file.stations[0]->renderdata.size() == 1, __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 0x1A, {2, 1, 1, 0}) && file.stations[0]->renderdata.size() == 1, __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 0x0E, {2, 2, 0, 1, 2}) && file.stations[0]->layouts.empty(), __LINE__);
	StationNeed(!read(GSF_STATIONS, 1, 2, 0x16, {4, 0}), __LINE__);
	StationNeed(!read(GSF_ROADSTOPS, 1, 2, 0x0E, {4, 0}), __LINE__);
	StationNeed(read(GSF_ROADSTOPS, 0, 1, 8, {'D', 'F'}) && file.roadstops[0] != nullptr, __LINE__);
	StationNeed(!read(GSF_ROADSTOPS, 0, 1, 8, {'D', 'F', 'L', 'T'}), __LINE__);
	StationNeed(read(GSF_ROADSTOPS, 0, 1, 0x15, {23}) && file.roadstops[0]->build_cost_multiplier == 23 && file.roadstops[0]->clear_cost_multiplier == 16, __LINE__);
	StationNeed(read(GSF_ROADSTOPS, 0, 1, 0x13, {7, 4, 5, 6}) && file.roadstops[0]->bridgeable_info[2].height == 6, __LINE__);
	StationNeed(!read(GSF_ROADSTOPS, 0, 2, 0xFE, {99}), __LINE__);
	BadgeID badge = GetOrCreateBadge("owner484/partial").index;
	BadgeID skipped = GetOrCreateBadge("owner484/skipped").index;
	file.badge_list = {badge, skipped};
	StationNeed(!read(GSF_ROADSTOPS, 1, 2, 0x16, {1, 0, 1, 0}) && !GetBadge(skipped)->features.Test(GSF_ROADSTOPS), __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 0x1F, {2, 0, 0, 0}) && file.stations[0]->badges.empty() && GetBadge(badge)->features.Test(GSF_STATIONS), __LINE__);
	StationNeed(read(GSF_ROADSTOPS, 0, 1, 0x16, {2, 0, 0, 0}) && file.roadstops[0]->badges.empty() && GetBadge(badge)->features.Test(GSF_ROADSTOPS), __LINE__);
	StationNeed(read(GSF_STATIONS, 0, 1, 9, {2, 1, 0, 0, 0, 5, 6}) && file.stations[0]->renderdata[0].ground.sprite == 1 && file.stations[0]->renderdata[0].seq.empty(), __LINE__);
	fmt::print(stderr, "STCMD definitions-partial-state {} {} {} {}\n", file.stations[0]->animation.frames, file.stations[0]->renderdata[0].seq.size(), file.stations[0]->bridgeable_info[1].height, file.roadstops[0]->bridgeable_info[2].height);
}
static void StationDefinitionMissing()
{
	uint count = 0;
	for (BaseStation *st : BaseStation::Iterate()) {
		if (!st->name.starts_with("Definitions ")) continue;
		for (const auto &slot : st->speclist) if (slot.grfid != 0) {
			StationNeed(slot.spec == nullptr && slot.grfid == StationDefinitionID(), __LINE__);
			fmt::print(stderr, "STCMD definitions-identity missing-rail {} {} {}\n", st->index.base(), slot.grfid, slot.localidx);
		}
		for (const auto &slot : st->roadstop_speclist) if (slot.grfid != 0) {
			StationNeed(slot.spec == nullptr && slot.grfid == StationDefinitionID(), __LINE__);
			fmt::print(stderr, "STCMD definitions-identity missing-road {} {} {}\n", st->index.base(), slot.grfid, slot.localidx);
		}
		StationNeed(st->cached_anim_triggers.None() && st->cached_cargo_triggers == 0 && st->cached_roadstop_anim_triggers.None() && st->cached_roadstop_cargo_triggers == 0, __LINE__);
		if (st->name == "Definitions station") {
			StationSpec spec{};
			auto next = AllocateSpecToStation(&spec, st);
			StationNeed(next.has_value() && *next == 3, __LINE__);
		}
		++count;
	}
	StationNeed(count == 4, __LINE__);
	fmt::print(stderr, "STCMD definitions-missing {} preserved no-reuse\n", count);
}
static void StationDefinitionLoad(std::string_view folder)
{
	StationDefinitionPartialReads();
	SetRandomSeed(0x48400001);
	_grfconfig.clear();
	for (const char *name : {"stations", "broken-station", "broken-road", "invalid-layout"}) {
		auto config = std::make_unique<GRFConfig>(std::string(folder) + "/" + name + ".grf");
		StationNeed(FillGRFDetails(*config, false, NO_DIRECTORY), __LINE__);
		config->flags.Reset(GRFConfigFlag::InitOnly);
		_grfconfig.push_back(std::move(config));
	}
	ReloadNewGRFData();
}

static void StationDefinitionFixture(TileIndex origin, DoCommandFlags test, DoCommandFlags exec, bool reload)
{
	const auto id = StationDefinitionID();
	const GRFFile *file = GetFileByGRFID(id);
	StationNeed(file != nullptr && GetGRFConfig(id)->status == GCS_ACTIVATED, __LINE__);
	const StationSpec *rail = StationClass::GetByGrf(id, 0), *extra = StationClass::GetByGrf(id, 1), *way = StationClass::GetByGrf(id, 2);
	const RoadStopSpec *road = RoadStopClass::GetByGrf(id, 0), *roadway = RoadStopClass::GetByGrf(id, 1);
	StationNeed(rail != nullptr && extra != nullptr && way != nullptr && road != nullptr && roadway != nullptr, __LINE__);
	StationNeed(StationClass::GetClassCount() == 5 && StationClass::GetUIClassCount() == 4 && RoadStopClass::GetClassCount() == 5, __LINE__);
	StationNeed(StationClass::Get(rail->class_index)->GetSpecCount() == 2 && rail->index == (file->GetParam(0) != 0 ? 1 : 0) && extra->index == (file->GetParam(0) != 0 ? 0 : 1) && IsWaypointClass(*StationClass::Get(way->class_index)) && IsWaypointClass(*RoadStopClass::Get(roadway->class_index)), __LINE__);
	StationNeed(GetString(rail->name) == "Station 484" && GetString(StationClass::Get(rail->class_index)->name) == "Class 484" && GetString(road->name) == "Road stop 484" && GetString(RoadStopClass::Get(road->class_index)->name) == "Road class 484", __LINE__);
	for (uint32_t broken : {uint32_t('B') | uint32_t('4') << 8 | uint32_t('8') << 16 | uint32_t('4') << 24, uint32_t('R') | uint32_t('4') << 8 | uint32_t('8') << 16 | uint32_t('4') << 24}) {
		const GRFConfig *config = GetGRFConfig(broken);
		StationNeed(config != nullptr && config->status == GCS_DISABLED && config->errors.size() == 1 && config->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, __LINE__);
	}
	const GRFConfig *invalid = GetGRFConfig(uint32_t('L') | uint32_t('4') << 8 | uint32_t('8') << 16 | uint32_t('4') << 24);
	StationNeed(invalid != nullptr && invalid->status == GCS_DISABLED && invalid->errors.size() == 1 && invalid->errors.front().message == STR_NEWGRF_ERROR_INVALID_SPRITE_LAYOUT, __LINE__);
	fmt::print(stderr, "STCMD definitions-malformed real-file read-bounds\n");
	StationNeed(StationClass::Get(static_cast<StationClassID>(2))->name == STR_UNDEFINED && RoadStopClass::Get(static_cast<RoadStopClassID>(2))->name == STR_UNDEFINED, __LINE__);
	fmt::print(stderr, "STCMD definitions-registry {} {} {} {} {}\n", rail->class_index, way->class_index, road->class_index, roadway->class_index, StationClass::Get(STAT_CLASS_DFLT)->GetSpecCount());
	StationNeed(rail->renderdata.size() == 2 && extra->renderdata.empty() && rail->layouts.at(GetStationLayoutKey(1, 2)) == std::vector<uint8_t>{0, 2} && extra->layouts == rail->layouts, __LINE__);
	StationNeed(rail->renderdata[0].ground.sprite == 1 && rail->renderdata[0].seq.size() == 1, __LINE__);
	const auto &sequence = rail->renderdata[0].seq[0];
	StationNeed(sequence.origin.x == 1 && sequence.origin.y == 2 && sequence.origin.z == 3 && sequence.extent.x == 4 && sequence.extent.y == 5 && sequence.extent.z == 6 && sequence.image.sprite == 2, __LINE__);
	StationNeed(way->renderdata.size() == 2 && way->renderdata[0].ground.sprite == 1 && way->renderdata[1].ground.sprite == 2, __LINE__);
	StationNeed(rail->badges.size() == 1 && road->badges == rail->badges && GetBadge(rail->badges[0])->features.Test(GSF_STATIONS) && GetBadge(rail->badges[0])->features.Test(GSF_ROADSTOPS), __LINE__);
	StationNeed(rail->bridgeable_info.size() == 3 && rail->bridgeable_info[2].height == 6 && rail->bridgeable_info[1].disallowed_pillars.base() == 2 && road->bridgeable_info[5].height == 6, __LINE__);
	fmt::print(stderr, "STCMD definitions-layout {} {} {} {} {}\n", rail->renderdata.size(), extra->renderdata.size(), rail->tileflags[0].base(), rail->tileflags[1].base(), rail->bridgeable_info[2].height);
	Station *station = nullptr, *stop = nullptr;
	Waypoint *waypoint = nullptr, *roadpoint = nullptr;
	if (!reload) {
		auto at = [origin](uint x, uint y) { return origin + TileDiffXY(x, y); };
		auto checked = [](const char *label, CommandCost cost) { StationCost(label, cost); StationNeed(cost.Succeeded(), __LINE__); return cost.GetCost(); };
		auto build = [&](DoCommandFlags flags, TileIndex tile, const StationSpec *spec, StationID join) { return Command<CMD_BUILD_RAIL_STATION>::Do(flags, tile, RAILTYPE_RAIL, AXIS_X, 1, 2, spec->class_index, spec->index, join, true); };
		auto price = checked("definition-rail-test", build(test, at(2, 2), rail, NEW_STATION));
		StationNeed(price == checked("definition-rail-exec", build(exec, at(2, 2), rail, NEW_STATION)), __LINE__);
		station = Station::GetByTile(at(2, 2)); station->name = "Definitions station";
		checked("definition-second", build(exec, at(2, 3), extra, station->index));
		checked("definition-same-new-slot", build(exec, at(2, 4), rail, station->index));
		StationNeed(GetCustomStationSpecIndex(at(2, 4)) == 3, __LINE__);
		checked("definition-remove-middle", Command<CMD_REMOVE_FROM_RAIL_STATION>::Do(exec, at(2, 3), at(3, 3), true));
		checked("definition-reuse", build(exec, at(2, 5), extra, station->index));
		StationNeed(GetCustomStationSpecIndex(at(2, 5)) == 2, __LINE__);
		checked("definition-remove-high", Command<CMD_REMOVE_FROM_RAIL_STATION>::Do(exec, at(2, 4), at(3, 4), true));
		StationNeed(station->speclist.size() == 3, __LINE__);
		checked("definition-way-track", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, at(8, 2), RAILTYPE_RAIL, TRACK_X, false));
		auto build_way = [&](DoCommandFlags flags) { return Command<CMD_BUILD_RAIL_WAYPOINT>::Do(flags, at(8, 2), AXIS_X, 1, 1, way->class_index, way->index, NEW_STATION, true); };
		price = checked("definition-way-test", build_way(test)); StationNeed(price == checked("definition-way-exec", build_way(exec)), __LINE__);
		waypoint = Waypoint::GetByTile(at(8, 2)); waypoint->name = "Definitions waypoint";
		auto build_stop = [&](DoCommandFlags flags) { return Command<CMD_BUILD_ROAD_STOP>::Do(flags, at(12, 2), 1, 1, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, road->class_index, road->index, NEW_STATION, true); };
		price = checked("definition-road-test", build_stop(test)); StationNeed(price == checked("definition-road-exec", build_stop(exec)), __LINE__);
		stop = Station::GetByTile(at(12, 2)); stop->name = "Definitions stop";
		checked("definition-road-track", Command<CMD_BUILD_ROAD>::Do(exec, at(16, 2), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
		auto build_roadway = [&](DoCommandFlags flags) { return Command<CMD_BUILD_ROAD_WAYPOINT>::Do(flags, at(16, 2), AXIS_X, 1, 1, roadway->class_index, roadway->index, NEW_STATION, true); };
		price = checked("definition-roadway-test", build_roadway(test)); StationNeed(price == checked("definition-roadway-exec", build_roadway(exec)), __LINE__);
		roadpoint = Waypoint::GetByTile(at(16, 2)); roadpoint->name = "Definitions roadpoint";
	} else {
		for (Station *st : Station::Iterate()) {
			if (st->name == "Definitions station") station = st;
			if (st->name == "Definitions stop") stop = st;
		}
		for (Waypoint *wp : Waypoint::Iterate()) {
			if (wp->name == "Definitions waypoint") waypoint = wp;
			if (wp->name == "Definitions roadpoint") roadpoint = wp;
		}
	}
	StationNeed(station != nullptr && stop != nullptr && waypoint != nullptr && roadpoint != nullptr, __LINE__);
	for (BaseStation *st : {static_cast<BaseStation *>(station), static_cast<BaseStation *>(waypoint)}) {
		for (const auto &slot : st->speclist) if (slot.grfid != 0) {
			StationNeed(slot.spec == StationClass::GetByGrf(slot.grfid, slot.localidx), __LINE__);
			fmt::print(stderr, "STCMD definitions-identity rail {} {} {} {} {} {}\n", st->index.base(), slot.grfid, slot.localidx, slot.spec->index, st->cached_anim_triggers.base(), st->cached_cargo_triggers);
		}
	}
	for (BaseStation *st : {static_cast<BaseStation *>(stop), static_cast<BaseStation *>(roadpoint)}) {
		for (const auto &slot : st->roadstop_speclist) if (slot.grfid != 0) {
			StationNeed(slot.spec == RoadStopClass::GetByGrf(slot.grfid, slot.localidx), __LINE__);
			fmt::print(stderr, "STCMD definitions-identity road {} {} {} {} {} {}\n", st->index.base(), slot.grfid, slot.localidx, slot.spec->index, st->cached_roadstop_anim_triggers.base(), st->cached_roadstop_cargo_triggers);
		}
	}
	uint occupied = 0;
	for (TileIndex tile : Map::Iterate()) {
		if (!IsTileType(tile, MP_STATION)) continue;
		if (GetStationIndex(tile) != station->index && GetStationIndex(tile) != waypoint->index && GetStationIndex(tile) != stop->index && GetStationIndex(tile) != roadpoint->index) continue;
		++occupied;
		fmt::print(stderr, "STCMD definitions-occupied {} {} {}\n", tile.base(), to_underlying(GetStationType(tile)), HasStationTileRail(tile) ? GetCustomStationSpecIndex(tile) : GetCustomRoadStopSpecIndex(tile));
	}
	StationNeed(occupied == 7, __LINE__);
	StationResolverObject rail_query(rail, station, station->train_station.tile);
	RoadStopResolverObject road_query(road, stop, stop->bus_stops->xy, ROADTYPE_ROAD, StationType::Bus, 0);
	for (CargoType cargo : {CargoGRFFileProps::SG_DEFAULT, CargoType{0}}) {
		rail_query.root_spritegroup = rail->grf_prop.GetSpriteGroup(cargo);
		road_query.root_spritegroup = road->grf_prop.GetSpriteGroup(cargo);
		auto r = rail_query.ResolveCallback({}), b = road_query.ResolveCallback({});
		StationNeed(r == (cargo == 0 ? 0x402 : 0x400) && r == b, __LINE__);
		fmt::print(stderr, "STCMD definitions-callbacks {} {} {}\n", cargo, r, b);
	}
	TriggerStationRandomisation(station, station->train_station.tile, StationRandomTrigger::NewCargo, 0);
	TriggerRoadStopRandomisation(stop, stop->bus_stops->xy, StationRandomTrigger::NewCargo, 0);
	fmt::print(stderr, "STCMD definitions-rng {} {} {} {} {}\n", reload, _random.state[0], _random.state[1], station->waiting_random_triggers.base(), stop->waiting_random_triggers.base());
}
