/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station-resolver-console.hpp Controlled actual Action2 programs, run by both executables. */
#include "newgrf_station.h"
#include "newgrf.h"
#include "newgrf_spritegroup.h"
static void StationResolverFixture(TileIndex origin, DoCommandFlags exec)
{
	TileIndex tile = origin + TileDiffXY(2, 2);
	StationNeed(Command<CMD_BUILD_RAIL_STATION>::Do(exec, tile, RAILTYPE_RAIL, AXIS_X, 2, 3, STAT_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
	auto *st = Station::GetByTile(tile);
	GRFFile file; file.grfid = 0x52533939; file.grf_version = 8;
	StationSpec spec; spec.grf_prop.grffile = &file; spec.grf_prop.grfid = file.grfid; spec.grf_prop.local_id = 19;
	StationNeed(SpriteGroup::CanAllocateItem(3), __LINE__);
	auto *result = new CallbackResultSpriteGroup(0x400);
	auto *det = new DeterministicSpriteGroup(); det->var_scope = VSG_SCOPE_SELF; det->size = DSG_SIZE_DWORD;
	det->default_result = {false, result};
	spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det);
	StationNeed(CargoPacket::CanAllocateItem(), __LINE__);
	st->goods[0].GetOrCreateData().cargo.Append(new CargoPacket(37, 5, st->index, tile, 0), StationID::Invalid());
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
		object.ResolveCallback(regs); StationNeed(regs[0] == first, __LINE__);
		if (var != 0x45) { StationResolverObject fresh(&spec, st, tile); std::array<int32_t, 1> other{}; StationNeed(fresh.ResolveCallback(other) == 0x400, __LINE__); StationNeed(other[0] != first, __LINE__); }
		SetStationGfx(tile, gfx);
		fmt::print(stderr, "STCMD resolver-cache {:x} {} {}\n", var, first, regs[0]);
	}
	StationNeed(GetStationCallback(CBID_STATION_LAND_SLOPE_CHECK, 1, 2, &spec, st, tile) == 0x400, __LINE__);
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x10, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
	for (uint16_t value : std::array<uint16_t, 3>{0x400, 0x401, CALLBACK_FAILED}) {
		result->result = value;
		StationCost("resolver-slope", PerformStationTileSlopeCheck(tile, tile, &spec, AXIS_X, 3, 2));
	}
	/* Actual randomized Action2 requests lower-only reseeding: every member draws
	 * once for its tile even though the upper reseed mask is zero, then the
	 * station draws once. The state oracle advances a saved native RNG directly. */
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
	for (TileIndex t : st->train_station) if (st->TileBelongsToRailStation(t)) SetCustomStationSpecIndex(t, 0);
	st->speclist.clear(); StationUpdateCachedTriggers(st);
	delete random; delete det; delete result;
}
