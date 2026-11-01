/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file roads-console.cpp Road-command inputs, included by the shared signal fixture. */
#include "town.h"
#include "tunnelbridge.h"
#include "water_map.h"
#include "void_map.h"
#include "pathfinder/water_regions.h"
#include "pathfinder/pathfinder_func.h"
#include "road.h"
#include "newgrf_roadstop.h"
#include "tree_map.h"
#include "tunnel_map.h"
#include "road_internal.h"
#ifdef WITH_RUST
#include "rust/services_ffi.h"
extern "C" uint32_t openttd_test_clean_up_road_bits(const OpenTTDSharedServices *, uint32_t, uint32_t);
#endif
extern RoadTypeInfo _roadtypes[ROADTYPE_END];
#ifdef WITH_RUST
extern RoadTypes &_roadtypes_road;
#else
extern RoadTypes _roadtypes_road;
#endif
#include "road_func.h"
#include "landscape.h"
#include "tile_type.h"
#include "depot_func.h"
#include "effectvehicle_func.h"
#include "effectvehicle_base.h"
#include "engine_base.h"
#include "engine_func.h"
#include "roadveh.h"
#include "vehicle_cmd.h"
#include "water_cmd.h"
extern const TileTypeProcs _tile_type_road_procs;
/* The pinned original bodies are the oracle in both builds. */
namespace RoadCleanupProbe {
#include "road-cleanup-original.hpp"
static uint32_t Candidate(TileIndex tile, uint32_t bits)
{
#ifdef WITH_RUST
	return openttd_test_clean_up_road_bits(&GetRustSharedServices(), tile.base(), bits);
#else
	return ::CleanUpRoadBits(tile, static_cast<RoadBits>(bits));
#endif
}
static void Run(TileIndex center)
{
	AutoRestoreBackup<Randomizer> interactive(_interactive_random, _interactive_random);
	_interactive_random.SetSeed(619);
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	/* Restore all fixture fields before returning to simulation; no pool entry is
	 * allocated or dereferenced by the tile-only connectivity helpers. */
	auto capture = [](Tile t) { return std::array<uint16_t, 10>{t.type(), t.height(), t.m1(), t.m2(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8()}; };
	auto restore = [](Tile t, const std::array<uint16_t, 10> &v) { t.type() = v[0]; t.height() = v[1]; t.m1() = v[2]; t.m2() = v[3]; t.m3() = v[4]; t.m4() = v[5]; t.m5() = v[6]; t.m6() = v[7]; t.m7() = v[8]; t.m8() = v[9]; };
	std::vector<std::tuple<TileIndex, std::array<uint16_t, 10>>> saved;
	for (int y = -2; y <= 3; ++y) for (int x = -2; x <= 3; ++x) {
		TileIndex tile = center + TileDiffXY(x, y);
		saved.emplace_back(tile, capture(tile));
	}
	uint count = 0;
	auto compare = [&](TileIndex tile, uint32_t plan) {
		const uint32_t expected = RoadCleanupProbe::CleanUpRoadBits(tile, static_cast<RoadBits>(plan));
		const uint32_t actual = Candidate(tile, plan);
		if (actual != expected) {
			fmt::print(stderr, "ROADCMD cleanup-failure {} {} {} {}\n", tile.base(), plan, expected, actual);
			SignalFixtureAbort(__LINE__);
		}
		++count;
	};
	for (DiagDirection dir = DIAGDIR_BEGIN; dir < DIAGDIR_END; ++dir) {
		TileIndex at = ::TileAddByDiagDir(center, dir);
		for (uint kind = 0; kind < 34; ++kind) for (uint orientation = 0; orientation < 4; ++orientation) {
			for (int y = -1; y <= 2; ++y) for (int x = -1; x <= 2; ++x) { TileIndex t = center + TileDiffXY(x, y); MakeClear(t, CLEAR_GRASS, 3); SetTileHeight(t, 2); }
			auto d = static_cast<DiagDirection>(orientation);
			auto axis = DiagDirToAxis(d);
			switch (kind) {
				case 0: break;
				case 1: MakeTree(at, TREE_TEMPERATE, 0, TreeGrowthStage::Grown, TREE_GROUND_GRASS, 3); break;
				case 2: MakeRailNormal(at, OWNER_NONE, TRACK_BIT_UPPER, RAILTYPE_RAIL); break;
				case 3: MakeSea(at); break;
				case 4: MakeShore(at); break;
				case 5: MakeRiver(at, 0); break;
				case 6: MakeCanal(at, OWNER_NONE, 0); break;
				case 7: MakeLockTile(at, OWNER_NONE, LockPart::Middle, d, WaterClass::Canal); break;
				case 8: MakeRoadNormal(at, DiagDirToRoadBits(d), ROADTYPE_ROAD, INVALID_ROADTYPE, TownID::Invalid(), OWNER_NONE, OWNER_NONE); break;
				case 9: MakeRoadNormal(at, ROAD_NONE, INVALID_ROADTYPE, ROADTYPE_TRAM, TownID::Invalid(), OWNER_NONE, OWNER_NONE); break;
				case 10: MakeRoadDepot(at, OWNER_NONE, DepotID(0), d, ROADTYPE_ROAD); break;
				case 11: MakeRoadDepot(at, OWNER_NONE, DepotID(0), d, ROADTYPE_TRAM); break;
				case 12: MakeRoadCrossing(at, OWNER_NONE, OWNER_NONE, OWNER_NONE, axis, RAILTYPE_RAIL, ROADTYPE_ROAD, ROADTYPE_TRAM, TownID::Invalid()); break;
				case 13: MakeRoadStop(at, OWNER_NONE, StationID(0), RoadStopType::Bus, ROADTYPE_ROAD, INVALID_ROADTYPE, d); break;
				case 14: MakeRoadStop(at, OWNER_NONE, StationID(0), RoadStopType::Truck, INVALID_ROADTYPE, ROADTYPE_TRAM, d); break;
				case 15: MakeDriveThroughRoadStop(at, OWNER_NONE, OWNER_NONE, OWNER_NONE, StationID(0), StationType::Bus, ROADTYPE_ROAD, ROADTYPE_TRAM, axis); break;
				case 16: MakeRailStation(at, OWNER_NONE, StationID(0), axis, 0, RAILTYPE_RAIL); break;
				case 17: MakeRoadTunnel(at, OWNER_NONE, d, ROADTYPE_ROAD, ROADTYPE_TRAM); break;
				case 18: MakeRoadBridgeRamp(at, OWNER_NONE, OWNER_NONE, OWNER_NONE, 0, d, INVALID_ROADTYPE, ROADTYPE_TRAM); break;
				case 19: MakeRailTunnel(at, OWNER_NONE, d, RAILTYPE_RAIL); break;
				case 20: MakeRailNormal(at, OWNER_NONE, AxisToTrackBits(axis), RAILTYPE_RAIL); break;
				case 21: MakeRailNormal(at, OWNER_NONE, TRACK_BIT_X | TRACK_BIT_Y, RAILTYPE_RAIL); break;
				case 22: MakeRailNormal(at, OWNER_NONE, AxisToTrackBits(axis), RAILTYPE_RAIL); SetHasSignals(at, true); break;
				case 23: MakeRailDepot(at, OWNER_NONE, DepotID(0), d, RAILTYPE_RAIL); break;
				case 28: SetTileType(at, MP_HOUSE); break;
				case 29: SetTileType(at, MP_INDUSTRY); break;
				case 30: SetTileType(at, MP_OBJECT); break;
				case 31: MakeRoadDepot(at, OWNER_NONE, DepotID(0), d, ROADTYPE_ROAD); SetRoadType(at, RTT_ROAD, INVALID_ROADTYPE); break;
				case 32: MakeRoadStop(at, OWNER_NONE, StationID(0), RoadStopType::Bus, INVALID_ROADTYPE, INVALID_ROADTYPE, d); break;
				case 33: MakeRoadTunnel(at, OWNER_NONE, d, INVALID_ROADTYPE, INVALID_ROADTYPE); break;
				default: MakeRailNormal(at, OWNER_NONE, AxisToTrackBits(axis), RAILTYPE_RAIL); break;
			}
			if (kind >= 24 && kind < 28) {
				Slope slope = static_cast<Slope>(std::array<uint, 4>{SLOPE_N, SLOPE_NE, SLOPE_NWS, SLOPE_STEEP_N}[kind - 24]);
				SetTileHeight(at, 2 + ((slope & SLOPE_N) != 0) + (slope == SLOPE_STEEP_N));
				SetTileHeight(at + TileDiffXY(1, 0), 2 + ((slope & SLOPE_W) != 0));
				SetTileHeight(at + TileDiffXY(0, 1), 2 + ((slope & SLOPE_E) != 0));
				SetTileHeight(at + TileDiffXY(1, 1), 2 + ((slope & SLOPE_S) != 0));
			}
			for (uint plan = 0; plan <= UINT8_MAX; ++plan) compare(center, plan);
			for (uint32_t plan : {256U, 0x8000000FU, UINT32_MAX}) compare(center, plan);
		}
		fmt::print(stderr, "ROADCMD cleanup-matrix {} {}\n", static_cast<uint>(dir), count);
	}
	for (TileIndex tile : {INVALID_TILE, TileIndex{Map::Size()}, TileXY(Map::MaxX(), 2), TileXY(2, Map::MaxY()), TileXY(1, 1), TileXY(Map::MaxX() - 1, Map::MaxY() - 1)}) for (uint plan = 0; plan <= UINT8_MAX; ++plan) compare(tile, plan);
	for (auto [tile, state] : saved) restore(tile, state);
	SaveRandomSeeds(&after);
	const bool same = before.random.state[0] == after.random.state[0] && before.random.state[1] == after.random.state[1] && before.interactive_random.state[0] == after.interactive_random.state[0] && before.interactive_random.state[1] == after.interactive_random.state[1];
	fmt::print(stderr, "ROADCMD cleanup-done {} {} {} {} {} {}\n", count, same, after.random.state[0], after.random.state[1], after.interactive_random.state[0], after.interactive_random.state[1]);
	if (!same) SignalFixtureAbort(__LINE__);
}
} // namespace RoadCleanupProbe

static void RoadPrint(std::string_view name, const CommandCost &cost, Money extra = 0)
{
	fmt::print(stderr, "ROADCMD {} {} {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), static_cast<int>(cost.GetExpensesType()), extra);
}
#ifdef WITH_RUST
#include "rust/road_command_services.h"
static CommandCost RoadBuild(DoCommandFlags flags, TileIndex tile, RoadBits bits, RoadType rt, DisallowedRoadDirections drd, TownID town)
{
	RustRoadCommandContext c; openttd_rust_road_do_build(c.view.services, &c.scratch, c.view.cost, c.view.ret, flags.base(), tile.base(), bits, rt, drd, town.base()); return std::move(c.cost);
}
#else
static constexpr auto RoadBuild = Command<CMD_BUILD_ROAD>::Do;
#endif
template <Commands Tcmd, typename... Args>
static void RoadRun(std::string_view name, Args... args)
{
	for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Execute}}) {
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		DoCommandFlags command_flags = flags;
		if constexpr (Tcmd == CMD_BUILD_ROAD || Tcmd == CMD_BUILD_LONG_ROAD || Tcmd == CMD_BUILD_ROAD_DEPOT || Tcmd == CMD_BUILD_SINGLE_RAIL) command_flags.Set(DoCommandFlag::Auto);
		auto result = [&]() {
			if constexpr (Tcmd == CMD_BUILD_ROAD) return RoadBuild(command_flags, args...);
			else return Command<Tcmd>::Do(command_flags, args...);
		}();
		SaveRandomSeeds(&after);
		if constexpr (Tcmd == CMD_BUILD_ROAD || Tcmd == CMD_BUILD_LONG_ROAD || Tcmd == CMD_REMOVE_LONG_ROAD || Tcmd == CMD_BUILD_ROAD_DEPOT || Tcmd == CMD_CONVERT_ROAD) {
			bool same = before.random.state[0] == after.random.state[0] && before.random.state[1] == after.random.state[1] && before.interactive_random.state[0] == after.interactive_random.state[0] && before.interactive_random.state[1] == after.interactive_random.state[1];
			if (!same) SignalFixtureAbort(__LINE__);
		}
		auto label = fmt::format("{}-{}", name, flags.Test(DoCommandFlag::Execute) ? "exec" : "test");
		if constexpr (std::is_same_v<decltype(result), CommandCost>) RoadPrint(label, result);
		else RoadPrint(label, std::get<0>(result), std::get<1>(result));
		UpdateSignalsInBuffer();
	}
}
extern const TileTypeProcs _tile_type_tunnelbridge_procs;
static void TunnelBridgeCommands(CompanyID owner, CompanyID other, RoadVehicle *rv)
{
	TileIndex origin = SignalFlatArea(24);
	auto t = [origin](uint x, uint y) { return origin + TileDiffXY(x, y); };
	auto ramp = [](TileIndex tile, Slope slope) {
		SetTileHeight(tile, 1 + ((slope & SLOPE_N) != 0));
		SetTileHeight(tile + TileDiffXY(1, 0), 1 + ((slope & SLOPE_W) != 0));
		SetTileHeight(tile + TileDiffXY(0, 1), 1 + ((slope & SLOPE_E) != 0));
		SetTileHeight(tile + TileDiffXY(1, 1), 1 + ((slope & SLOPE_S) != 0));
	};
	Backup<bool> slopes(_settings_game.construction.build_on_slopes, true);
	for (Axis axis : {AXIS_X, AXIS_Y}) for (TransportType transport : {TRANSPORT_RAIL, TRANSPORT_ROAD, TRANSPORT_WATER}) {
		uint lane = static_cast<uint>(transport);
		TileIndex start = axis == AXIS_X ? t(2, 3 + lane * 4) : t(14 + lane * 3, 2);
		TileIndex end = axis == AXIS_X ? t(8, 3 + lane * 4) : t(14 + lane * 3, 8);
		if (transport == TRANSPORT_WATER) {
			ramp(start, axis == AXIS_X ? SLOPE_NE : SLOPE_NW);
			ramp(end, axis == AXIS_X ? SLOPE_SW : SLOPE_SE);
		}
		auto name = fmt::format("tb-bridge-{}-{}", static_cast<uint>(axis), static_cast<uint>(transport));
		RoadPrint(name + "-query", Command<CMD_BUILD_BRIDGE>::Do(DoCommandFlag::QueryCost, end, start, transport, 0, 0));
		{
			Backup<bool> ai(Company::Get(owner)->is_ai, true);
			Backup<CompanyID> local(_local_company, other);
			RoadPrint(name + "-ai-query", Command<CMD_BUILD_BRIDGE>::Do(DoCommandFlag::QueryCost, end, start, transport, 0, 0));
			local.Restore(); ai.Restore();
		}
		RoadRun<CMD_BUILD_BRIDGE>(name, end, start, transport, 0, 0);
		if (!IsBridgeTile(start)) SignalFixtureAbort(__LINE__);
		RoadPrint(name + "-above", _tile_type_tunnelbridge_procs.check_build_above_proc(start, {}, OtherAxis(axis), GetBridgeHeight(start) + 1));
		RoadPrint(name + "-auto", _tile_type_tunnelbridge_procs.clear_tile_proc(start, DoCommandFlag::Auto));
		if (transport != TRANSPORT_WATER) {
			RoadRun<CMD_BUILD_BRIDGE>(name + "-replace", start, end, transport, 3, 0);
			RoadPrint(name + "-autoslope", _tile_type_tunnelbridge_procs.terraform_tile_proc(start, {}, GetTileZ(start), GetTileSlope(start)));
		}
		if (transport == TRANSPORT_ROAD) {
			RoadRun<CMD_BUILD_ROAD>(name + "-tram", start, AxisToRoadBits(axis), ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
			TileIndex old = rv->tile; rv->tile = start; rv->UpdatePosition();
			RoadRun<CMD_LANDSCAPE_CLEAR>(name + "-occupied", start);
			rv->tile = old; rv->UpdatePosition();
		}
		for (TileIndex tile : {start, end}) ChangeTileOwner(tile, owner, other);
		RoadPrint(name + "-foreign", _tile_type_tunnelbridge_procs.clear_tile_proc(start, {}));
		Backup<CompanyID> bankrupt(_current_company, other);
		for (TileIndex tile : {start, end}) if (IsBridgeTile(tile)) ChangeTileOwner(tile, other, INVALID_OWNER);
		bankrupt.Restore();
		if (IsBridgeTile(start)) RoadRun<CMD_LANDSCAPE_CLEAR>(name + "-clear", end);
		if (transport == TRANSPORT_WATER) { ramp(start, SLOPE_FLAT); ramp(end, SLOPE_FLAT); }
	}
	for (TransportType transport : {TRANSPORT_RAIL, TRANSPORT_ROAD}) {
		uint row = 17 + static_cast<uint>(transport) * 3;
		for (uint x = 3; x <= 7; ++x) for (uint y = row; y <= row + 1; ++y) SetTileHeight(t(x, y), 2);
		TileIndex start = t(7, row), end = t(2, row);
		if (transport == TRANSPORT_ROAD) SetTileHeight(end, 2); // Non-complementary exit needs recursive terraform.
		auto name = fmt::format("tb-tunnel-{}", static_cast<uint>(transport));
		Backup<uint16_t> limit(_settings_game.construction.max_tunnel_length, 1);
		RoadPrint(name + "-length", Command<CMD_BUILD_TUNNEL>::Do({}, start, transport, 0));
#ifdef WITH_RUST
		RoadPrint(name + "-highlight", CommandCost(EXPENSES_CONSTRUCTION, Money(GetBuildTunnelEndTile().base() - origin.base())));
#else
		RoadPrint(name + "-highlight", CommandCost(EXPENSES_CONSTRUCTION, Money(_build_tunnel_endtile.base() - origin.base())));
#endif
		limit.Restore();
		RoadRun<CMD_BUILD_TUNNEL>(name, start, transport, 0);
		if (!IsTunnelTile(start)) SignalFixtureAbort(__LINE__);
		RoadPrint(name + "-above", _tile_type_tunnelbridge_procs.check_build_above_proc(start, {}, AXIS_X, 0));
		TileIndex old = rv->tile; rv->tile = end; rv->UpdatePosition();
		RoadRun<CMD_LANDSCAPE_CLEAR>(name + "-occupied", start);
		rv->tile = old; rv->UpdatePosition();
		RoadRun<CMD_LANDSCAPE_CLEAR>(name + "-clear", start);
	}
	for (BridgeType bridge : {BridgeType{255}, BridgeType{256}, std::numeric_limits<BridgeType>::max()}) {
		RoadPrint(fmt::format("tb-invalid-type-{}", bridge), Command<CMD_BUILD_BRIDGE>::Do({}, t(8, 14), t(2, 14), TRANSPORT_RAIL, bridge, RAILTYPE_RAIL));
	}
	/* Positive Y-direction tunnel exercises the source's other Map::MaxX branch. */
	for (uint y = 3; y <= 7; ++y) for (uint x = 17; x <= 18; ++x) SetTileHeight(t(x, y), 2);
	RoadRun<CMD_BUILD_TUNNEL>("tb-tunnel-y", t(17, 2), TRANSPORT_ROAD, 0);
	if (!IsTunnelTile(t(17, 2))) SignalFixtureAbort(__LINE__);
	RoadRun<CMD_LANDSCAPE_CLEAR>("tb-tunnel-y-clear", t(17, 2));
	/* Town/deity policy, town downgrade refusal, rating refusal, then allowed removal. */
	Town *town = *Town::Iterate().begin();
	Backup<bool> dynamite(_settings_game.construction.extra_dynamite, true);
	{
		Backup<CompanyID> company(_current_company, OWNER_TOWN);
		RoadRun<CMD_BUILD_BRIDGE>("tb-town", t(8, 15), t(2, 15), TRANSPORT_ROAD, 3, ROADTYPE_ROAD);
		company.Restore();
	}
	RoadRun<CMD_BUILD_BRIDGE>("tb-town-downgrade", t(8, 15), t(2, 15), TRANSPORT_ROAD, 0, ROADTYPE_ROAD);
	Town *nearest = ClosestTownFromTile(t(2, 15), UINT_MAX);
	if (nearest == nullptr) SignalFixtureAbort(__LINE__);
	nearest->ratings[owner] = RATING_MINIMUM;
	RoadRun<CMD_LANDSCAPE_CLEAR>("tb-town-refuse", t(2, 15));
	nearest->ratings[owner] = RATING_MAXIMUM;
	RoadRun<CMD_LANDSCAPE_CLEAR>("tb-town-clear", t(2, 15));
	{
		Backup<CompanyID> deity(_current_company, OWNER_DEITY);
		RoadRun<CMD_BUILD_BRIDGE>("tb-deity", t(12, 14), t(12, 10), TRANSPORT_ROAD, 0, ROADTYPE_ROAD);
		deity.Restore();
	}
	/* Shared pricing helper thresholds run by default against each unchanged executable. */
	for (int length : {0, 1, 2, 3, 4, 5, 25, 26, 50, 51, 100, 200, 1000}) {
		fmt::print(stderr, "ROADCMD tb-price {} {}\n", length, CalcBridgeLenCostFactor(length));
	}
	/* Normal CmdBuildTunnel prices around each coefficient change, on a declared clear strip. */
	TileIndex strip = INVALID_TILE;
	for (uint y = 2; y + 5 < Map::SizeY() && strip == INVALID_TILE; ++y) for (uint x = 2; x + 108 < Map::SizeX(); ++x) {
		bool clear = true;
		for (uint dy = 0; dy < 4 && clear; ++dy) for (uint dx = 0; dx < 108; ++dx) {
			TileType type = GetTileType(TileXY(x + dx, y + dy));
			if (type != MP_CLEAR && type != MP_TREES && !(type == MP_WATER && IsWaterTile(TileXY(x + dx, y + dy)) && GetWaterClass(TileXY(x + dx, y + dy)) == WaterClass::Sea)) { clear = false; break; }
		}
		if (clear) { strip = TileXY(x, y); break; }
	}
	if (strip == INVALID_TILE) SignalFixtureAbort(__LINE__);
	for (uint dy = 0; dy < 4; ++dy) for (uint dx = 0; dx < 108; ++dx) { TileIndex tile = strip + TileDiffXY(dx, dy); MakeClear(tile, CLEAR_GRASS, 3); InvalidateWaterRegion(tile); }
	Backup<uint16_t> tunnel_limit(_settings_game.construction.max_tunnel_length, 65535);
	for (uint length : {24, 25, 26, 49, 50, 51, 99, 100, 101}) {
		for (uint dy = 0; dy < 4; ++dy) for (uint dx = 0; dx < 108; ++dx) SetTileHeight(strip + TileDiffXY(dx, dy), 1);
		for (uint y = 1; y <= 2; ++y) for (uint x = 3; x <= length + 3; ++x) SetTileHeight(strip + TileDiffXY(x, y), 2);
		RoadPrint(fmt::format("tb-tunnel-price-{}", length), Command<CMD_BUILD_TUNNEL>::Do({}, strip + TileDiffXY(2, 1), TRANSPORT_RAIL, RAILTYPE_RAIL));
	}
	tunnel_limit.Restore();
	fmt::print(stderr, "ROADCMD tb-done {}\n", town != nullptr);
	dynamite.Restore(); slopes.Restore();
}

static bool RoadCommandsScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	CompanyID owner = (*Company::Iterate().begin())->index;
	Backup<CompanyID> company(_current_company, owner);
	if (args[1] == "inspect") {
		uint roads = 0, depots = 0;
		for (TileIndex tile : Map::Iterate()) if (IsTileType(tile, MP_ROAD)) { ++roads; depots += IsRoadDepot(tile); }
		fmt::print(stderr, "ROADCMD reload {} {}\n", roads, depots);
		company.Restore(); return true;
	}
	TileIndex origin = SignalFlatArea(24);
	SignalRichCompany(owner);
	auto t = [origin](uint x, uint y) { return origin + TileDiffXY(x, y); };
	/* Declared builtin-like type inputs, identical on both executables. No resolver is replaced. */
	RoadType alternate = static_cast<RoadType>(2);
	_roadtypes[alternate] = _roadtypes[ROADTYPE_ROAD];
	_roadtypes[alternate].powered_roadtypes.Set(alternate);
	_roadtypes_road.Set(alternate);
	Company::Get(owner)->avail_roadtypes.Set(alternate);
	Company::Get(owner)->avail_roadtypes.Set(ROADTYPE_TRAM);
	if (args[1] == "tram") {
		/* Declared engine input (#156): the first buildable stock road engine no vehicle
		 * uses becomes a tram for this run. No reload follows, so it stays configured. */
		Engine *tram = nullptr;
		for (Engine *e : Engine::IterateType(VEH_ROAD)) {
			if (!IsEngineBuildable(e->index, VEH_ROAD, owner)) continue;
			bool used = false;
			for (const RoadVehicle *v : RoadVehicle::Iterate()) used |= v->engine_type == e->index;
			if (!used) { tram = e; break; }
		}
		if (tram == nullptr) SignalFixtureAbort(__LINE__);
		tram->VehInfo<RoadVehicleInfo>().roadtype = ROADTYPE_TRAM;
		fmt::print(stderr, "ROADCMD tram-engine {}\n", tram->index.base());
		/* Dead ends at x=6: clear land accepts the turn's build query, so that tram waits;
		 * an own canal refuses it under NoWater, so that tram turns on its own tile. */
		RoadRun<CMD_BUILD_CANAL>("tram-canal", t(7, 8), t(7, 8), WaterClass::Canal, false);
		for (uint y : {4, 8}) {
			RoadRun<CMD_BUILD_ROAD_DEPOT>("tram-depot", t(2, y), ROADTYPE_TRAM, DIAGDIR_SW);
			RoadRun<CMD_BUILD_LONG_ROAD>("tram-line", t(6, y), t(3, y), ROADTYPE_TRAM, AXIS_X, DRD_NONE, false, true, true);
			auto [cost, id, refit, part, cargo] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, t(2, y), tram->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
			RoadPrint("tram-vehicle", cost, id.base());
			RoadPrint("tram-start", Command<CMD_START_STOP_VEHICLE>::Do(DoCommandFlag::Execute, id, false));
			fmt::print(stderr, "ROADCMD tram-end {} {} {}\n", y, id.base(), t(6, y).base());
		}
		company.Restore();
		return true;
	}
	RoadRun<CMD_BUILD_BRIDGE>("non-road-bridge", t(14, 22), t(20, 22), TRANSPORT_RAIL, 0, RAILTYPE_RAIL);
	/* MayHaveRoad excludes rail stations and non-road tunnel/bridge heads. */
	std::array<bool, 2> non_road_seen{};
	for (TileIndex tile : Map::Iterate()) {
		int kind = IsTileType(tile, MP_STATION) && !IsAnyRoadStop(tile) ? 0 : IsTileType(tile, MP_TUNNELBRIDGE) && GetTunnelBridgeTransportType(tile) != TRANSPORT_ROAD ? 1 : -1;
		if (kind < 0 || non_road_seen[kind]) continue;
		RoadRun<CMD_REMOVE_LONG_ROAD>(fmt::format("non-road-remove-{}", kind), tile, tile, ROADTYPE_ROAD, AXIS_X, false, true);
		RoadRun<CMD_CONVERT_ROAD>(fmt::format("non-road-convert-{}", kind), tile, tile, alternate, false);
		non_road_seen[kind] = true;
		if (non_road_seen[0] && non_road_seen[1]) break;
	}
	if (!non_road_seen[0] || !non_road_seen[1]) SignalFixtureAbort(__LINE__);
	CompanyID other = RailCompany();
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	RoadCleanupProbe::Run(t(18, 15));
	RoadRun<CMD_BUILD_ROAD>("zero", t(2, 2), ROAD_NONE, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadPrint("protocol-outside", RoadBuild({}, TileIndex{Map::Size()}, ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	RoadPrint("protocol-void", RoadBuild({}, TileXY(Map::MaxX(), 2), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	RoadPrint("protocol-alltiles", RoadBuild(DoCommandFlag::AllTiles, TileXY(Map::MaxX(), 2), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	RoadRun<CMD_BUILD_LONG_ROAD>("drag", t(12, 2), t(2, 2), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, false);
	RoadRun<CMD_BUILD_LONG_ROAD>("reverse", t(2, 2), t(12, 2), ROADTYPE_ROAD, AXIS_X, DRD_SOUTHBOUND, true, false, true);
	RoadRun<CMD_BUILD_ROAD>("oneway", t(5, 2), ROAD_X, ROADTYPE_ROAD, DRD_NORTHBOUND, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("junction", t(5, 2), ROAD_Y, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("tram", t(6, 2), ROAD_X, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("half-tram", t(15, 2), ROAD_NE, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	for (RoadBits bits : {ROAD_NW, ROAD_SW, ROAD_SE, ROAD_NE}) {
		RoadRun<CMD_LANDSCAPE_CLEAR>("half-tram-clear", t(15, 2));
		RoadRun<CMD_BUILD_ROAD>("half-tram-direction", t(15, 2), bits, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
		fmt::print(stderr, "ROADCMD movement-half-tram-{} {}\n", static_cast<uint>(bits), GetTrackdirBitsForRoad(t(15, 2), RTT_TRAM));
	}
	RoadRun<CMD_BUILD_ROAD_DEPOT>("depot", t(8, 5), ROADTYPE_ROAD, DIAGDIR_NE);
	RoadRun<CMD_BUILD_ROAD_DEPOT>("depot-same", t(8, 5), ROADTYPE_ROAD, DIAGDIR_NE);
	RoadRun<CMD_BUILD_ROAD_DEPOT>("rotate", t(8, 5), ROADTYPE_ROAD, DIAGDIR_SE);
	RoadRun<CMD_BUILD_LONG_ROAD>("obstacle", t(12, 5), t(2, 5), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, false);
	RoadRun<CMD_BUILD_LONG_ROAD>("ai-obstacle", t(12, 5), t(2, 5), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, true);
	RoadRun<CMD_BUILD_LONG_ROAD>("half-single", t(15, 5), t(15, 5), ROADTYPE_ROAD, AXIS_Y, DRD_NONE, true, true, true);
	RoadRun<CMD_BUILD_LONG_ROAD>("invalid-drag", t(12, 6), t(2, 5), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, false);
	RoadRun<CMD_CONVERT_ROAD>("convert", t(12, 2), t(2, 2), alternate, false);
	RoadRun<CMD_CONVERT_ROAD>("convert-back", t(12, 2), t(2, 2), ROADTYPE_ROAD, true);
	/* Force build's nested conversion path with declared asymmetric power masks. */
	Backup<RoadTypes> old_power(_roadtypes[ROADTYPE_ROAD].powered_roadtypes);
	Backup<RoadTypes> new_power(_roadtypes[alternate].powered_roadtypes);
	_roadtypes[ROADTYPE_ROAD].powered_roadtypes.Set(alternate);
	_roadtypes[alternate].powered_roadtypes.Reset(ROADTYPE_ROAD);
	if (GetRoadType(t(7, 5), RTT_ROAD) != ROADTYPE_ROAD) SignalFixtureAbort(__LINE__);
	RoadRun<CMD_BUILD_ROAD>("automatic-convert", t(7, 5), ROAD_Y, alternate, DRD_NONE, TownID::Invalid());
	if (GetRoadType(t(7, 5), RTT_ROAD) != alternate) SignalFixtureAbort(__LINE__);
	old_power.Restore(); new_power.Restore();
	RoadRun<CMD_REMOVE_LONG_ROAD>("remove-halves", t(12, 2), t(2, 2), ROADTYPE_ROAD, AXIS_X, true, false);
	RoadRun<CMD_REMOVE_LONG_ROAD>("remove-tram", t(6, 2), t(6, 2), ROADTYPE_TRAM, AXIS_X, false, true);
	/* Crossing construction over a reserved track and tram-first clearing. */
	RoadRun<CMD_BUILD_SINGLE_RAIL>("rail", t(3, 9), RAILTYPE_RAIL, TRACK_Y, false);
	SetTrackReservation(t(3, 9), TRACK_BIT_Y);
	RoadRun<CMD_BUILD_ROAD>("crossing", t(3, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("crossing-tram", t(3, 9), ROAD_X, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	/* Actual owner queries on adjacent barred crossings: both escape directions. */
	RoadRun<CMD_BUILD_SINGLE_RAIL>("adjacent-rail", t(4, 9), RAILTYPE_RAIL, TRACK_Y, false);
	RoadRun<CMD_BUILD_ROAD>("adjacent-crossing", t(4, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	BarCrossing(t(3, 9)); BarCrossing(t(4, 9));
	for (TileIndex crossing : {t(3, 9), t(4, 9)}) {
		fmt::print(stderr, "ROADCMD movement-crossing {} {}\n", crossing.base(), GetTileTrackStatus(crossing, TRANSPORT_ROAD, RTT_ROAD));
	}
	UnbarCrossing(t(3, 9)); UnbarCrossing(t(4, 9));
	RoadRun<CMD_LANDSCAPE_CLEAR>("adjacent-clear", t(4, 9));
	/* The map-edge mask remains observable with freeform edges enabled. */
	for (TileIndex edge : {TileXY(0, 10), TileXY(10, 0), TileXY(0, 0)}) {
		if (!IsTileType(edge, MP_VOID)) SignalFixtureAbort(__LINE__);
		Backup<bool> freeform(_settings_game.construction.freeform_edges, false);
		MakeSea(edge);
		fmt::print(stderr, "ROADCMD movement-water {} {}\n", edge.base(), GetTileTrackStatus(edge, TRANSPORT_WATER, 0));
		freeform.Restore();
		MakeVoid(edge);
	}
	/* Terraform heights are tile levels, including non-flat crossing surfaces. */
	RoadPrint("crossing-autoslope", _tile_type_road_procs.terraform_tile_proc(t(3, 9), {}, GetTileMaxZ(t(3, 9)) - 1, SLOPE_WSE));
	RoadRun<CMD_REMOVE_LONG_ROAD>("crossing-remove-tram", t(3, 9), t(3, 9), ROADTYPE_TRAM, AXIS_X, false, true);
	RoadRun<CMD_REMOVE_LONG_ROAD>("crossing-remove", t(3, 9), t(3, 9), ROADTYPE_ROAD, AXIS_X, false, true);
	RoadRun<CMD_BUILD_ROAD>("crossing-again", t(3, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	Backup<RoadTypeFlags> no_crossing(_roadtypes[alternate].flags);
	_roadtypes[alternate].flags.Set(RoadTypeFlag::NoLevelCrossing);
	RoadRun<CMD_CONVERT_ROAD>("crossing-convert", t(3, 9), t(3, 9), alternate, false);
	RoadRun<CMD_BUILD_ROAD>("crossing-disallowed", t(3, 9), ROAD_X, alternate, DRD_NONE, TownID::Invalid());
	no_crossing.Restore();
	RoadRun<CMD_LANDSCAPE_CLEAR>("clear-crossing", t(3, 9));
	/* Source-order terraform and roadwork failures on declared normal tiles. */
	RoadRun<CMD_BUILD_ROAD>("slope-road", t(6, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadPrint("autoslope", _tile_type_road_procs.terraform_tile_proc(t(6, 9), {}, GetTileZ(t(6, 9)), SLOPE_FLAT));
	RoadPrint("terraform-clear-test", _tile_type_road_procs.terraform_tile_proc(t(6, 9), {}, GetTileZ(t(6, 9)) + TILE_HEIGHT, SLOPE_W));
	StartRoadWorks(t(6, 9));
	RoadRun<CMD_BUILD_ROAD>("works", t(6, 9), ROAD_Y, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	CreateEffectVehicleAbove(TileX(t(6, 9)) * TILE_SIZE + 7, TileY(t(6, 9)) * TILE_SIZE + 7, 2, EV_BULLDOZER);
	Backup<CompanyID> water_company(_current_company, OWNER_WATER);
	RoadPrint("works-flood-test", _tile_type_road_procs.clear_tile_proc(t(6, 9), {}));
	RoadPrint("works-flood-exec", _tile_type_road_procs.clear_tile_proc(t(6, 9), DoCommandFlag::Execute));
	water_company.Restore();
	RoadRun<CMD_TERRAFORM_LAND>("raise", t(10, 9), SLOPE_N, true);
	RoadRun<CMD_BUILD_ROAD>("slope", t(10, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	Backup<bool> no_slopes(_settings_game.construction.build_on_slopes, false);
	RoadRun<CMD_REMOVE_LONG_ROAD>("slope-remove-half", t(10, 9), t(10, 9), ROADTYPE_ROAD, AXIS_X, false, false);
	no_slopes.Restore();
	RoadPrint("build-above", _tile_type_road_procs.check_build_above_proc(t(8, 5), {}, AXIS_X, 64));
	RoadPrint("depot-autoslope", _tile_type_road_procs.terraform_tile_proc(t(8, 5), {}, GetTileZ(t(8, 5)) - 1, SLOPE_SE));
	RoadRun<CMD_LANDSCAPE_CLEAR>("depot-remove", t(8, 5));
	/* Station coexistence and tunnel/bridge charging/removal. */
	RoadRun<CMD_BUILD_ROAD_STOP>("stop", t(14, 12), 1, 1, RoadStopType::Bus, true, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, StationID::Invalid(), false);
	RoadRun<CMD_BUILD_ROAD>("stop-tram", t(14, 12), ROAD_X, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_REMOVE_LONG_ROAD>("stop-remove-tram", t(14, 12), t(14, 12), ROADTYPE_TRAM, AXIS_X, false, true);
	RoadRun<CMD_BUILD_BRIDGE>("bridge", t(3, 16), t(9, 16), TRANSPORT_ROAD, 0, ROADTYPE_ROAD);
	RoadRun<CMD_BUILD_ROAD>("bridge-tram", t(3, 16), ROAD_NE, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_CONVERT_ROAD>("bridge-convert", t(9, 16), t(3, 16), alternate, false);
	RoadRun<CMD_CONVERT_ROAD>("bridge-back", t(9, 16), t(3, 16), ROADTYPE_ROAD, false);
	RoadRun<CMD_REMOVE_LONG_ROAD>("bridge-remove-tram", t(9, 16), t(3, 16), ROADTYPE_TRAM, AXIS_X, false, true);
	RoadRun<CMD_REMOVE_LONG_ROAD>("stop-remove-last", t(14, 12), t(14, 12), ROADTYPE_ROAD, AXIS_X, false, true);
	RoadRun<CMD_TERRAFORM_LAND>("tunnel-hill", t(18, 18), SLOPE_ELEVATED, true);
	RoadRun<CMD_BUILD_TUNNEL>("tunnel", t(17, 18), TRANSPORT_ROAD, ROADTYPE_ROAD);
	RoadRun<CMD_CONVERT_ROAD>("tunnel-convert", t(19, 18), t(17, 18), alternate, false);
	RoadRun<CMD_CONVERT_ROAD>("tunnel-back", t(17, 18), t(19, 18), ROADTYPE_ROAD, false);
	/* A stopped road vehicle in its depot, then moved in the tile hash onto road and wormhole heads. */
	RoadRun<CMD_BUILD_ROAD_DEPOT>("rv-depot", t(2, 13), ROADTYPE_ROAD, DIAGDIR_SW);
	RoadRun<CMD_BUILD_LONG_ROAD>("rv-road", t(6, 13), t(3, 13), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, false, false);
	RoadVehicle *rv = nullptr;
	for (const Engine *e : Engine::IterateType(VEH_ROAD)) {
		auto [cost, id, capacity, mail, cargo] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, t(2, 13), e->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
		if (cost.Succeeded()) { rv = RoadVehicle::Get(id); break; }
	}
	if (rv == nullptr) SignalFixtureAbort(__LINE__);
	RoadRun<CMD_BUILD_ROAD_DEPOT>("rv-rotate", t(2, 13), ROADTYPE_ROAD, DIAGDIR_NE);
	RoadRun<CMD_LANDSCAPE_CLEAR>("rv-depot-remove", t(2, 13));
	RoadRun<CMD_CONVERT_ROAD>("rv-depot-convert", t(2, 13), t(2, 13), alternate, false);
	Backup<RoadTypes> rv_power(_roadtypes[ROADTYPE_ROAD].powered_roadtypes);
	_roadtypes[ROADTYPE_ROAD].powered_roadtypes.Set(alternate);
	RoadRun<CMD_CONVERT_ROAD>("rv-depot-power", t(2, 13), t(2, 13), alternate, false);
	rv_power.Restore();
	RoadRun<CMD_CONVERT_ROAD>("rv-depot-back", t(2, 13), t(2, 13), ROADTYPE_ROAD, false);
	auto move = [rv](TileIndex tile) { rv->tile = tile; rv->UpdatePosition(); };
	move(t(4, 13));
	RoadRun<CMD_BUILD_ROAD>("rv-oneway", t(4, 13), ROAD_X, ROADTYPE_ROAD, DRD_NORTHBOUND, TownID::Invalid());
	RoadRun<CMD_REMOVE_LONG_ROAD>("rv-remove", t(4, 13), t(4, 13), ROADTYPE_ROAD, AXIS_X, false, true);
	move(t(3, 16));
	RoadRun<CMD_REMOVE_LONG_ROAD>("rv-bridge", t(3, 16), t(3, 16), ROADTYPE_ROAD, AXIS_X, false, true);
	move(t(17, 18));
	RoadRun<CMD_CONVERT_ROAD>("rv-tunnel", t(17, 18), t(17, 18), alternate, false);
	move(t(2, 13));
	/* Low-funds removal returns the full remaining estimate, retaining prior effects. */
	RoadRun<CMD_BUILD_LONG_ROAD>("money-road", t(12, 20), t(2, 20), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, true);
#ifdef WITH_RUST
	Company::Get(owner)->Finances().money = Money(700);
#else
	Company::Get(owner)->money = Money(700);
#endif
#ifdef WITH_RUST
	Backup<Money> protocol_money(Company::Get(owner)->Finances().money, Money(0));
#else
	Backup<Money> protocol_money(Company::Get(owner)->money, Money(0));
#endif
	RoadPrint("protocol-money", RoadBuild({}, t(21, 2), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	RoadPrint("protocol-query", RoadBuild(DoCommandFlag::QueryCost, t(21, 2), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	RoadPrint("protocol-bankrupt", RoadBuild(DoCommandFlag::Bankrupt, t(21, 2), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	protocol_money.Restore();
	RoadRun<CMD_REMOVE_LONG_ROAD>("money-remove", t(12, 20), t(2, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Execute}}) {
		auto [cost, extra] = CmdRemoveLongRoad(flags, t(12, 20), t(2, 20), ROADTYPE_ROAD, AXIS_X, false, true);
		RoadPrint(flags.Test(DoCommandFlag::Execute) ? "money-body-exec" : "money-body-test", cost, extra);
	}
	SignalRichCompany(owner);
	/* Town inner/edge removal in the real rating test mode; no RNG is consumed. */
	Town *town = *Town::Iterate().begin();
	town->ratings[owner] = 800;
	Backup<bool> dynamite(_settings_game.construction.extra_dynamite, true);
	for (uint x = 15; x <= 17; ++x) {
		Backup<CompanyID> town_company(_current_company, OWNER_TOWN);
		RoadRun<CMD_BUILD_ROAD>("town-road", t(x, 20), ROAD_X, ROADTYPE_ROAD, DRD_NONE, town->index);
		town_company.Restore();
	}
	town->ratings[owner] = RATING_MINIMUM;
	RoadRun<CMD_REMOVE_LONG_ROAD>("town-rating", t(15, 20), t(15, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	RoadRun<CMD_CONVERT_ROAD>("town-convert-rating", t(17, 20), t(15, 20), alternate, false);
	town->ratings[owner] = 800;
	RoadRun<CMD_CONVERT_ROAD>("town-convert", t(17, 20), t(15, 20), alternate, false);
	Backup<RoadTypes> town_power(_roadtypes[ROADTYPE_ROAD].powered_roadtypes);
	_roadtypes[ROADTYPE_ROAD].powered_roadtypes.Set(alternate);
	RoadRun<CMD_CONVERT_ROAD>("town-convert-power", t(17, 20), t(15, 20), alternate, false);
	town_power.Restore();
	RoadRun<CMD_CONVERT_ROAD>("town-convert-back", t(17, 20), t(15, 20), ROADTYPE_ROAD, false);
	_settings_game.construction.extra_dynamite = false;
	RoadRun<CMD_REMOVE_LONG_ROAD>("town-refuse", t(16, 20), t(16, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	_settings_game.construction.extra_dynamite = true;
	RoadRun<CMD_REMOVE_LONG_ROAD>("town-inner", t(16, 20), t(16, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	RoadRun<CMD_REMOVE_LONG_ROAD>("town-edge", t(15, 20), t(15, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	dynamite.Restore();
	RoadRun<CMD_BUILD_ROAD>("transfer-road", t(20, 20), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	ChangeTileOwner(t(20, 20), owner, other);
	RoadRun<CMD_BUILD_ROAD>("foreign-oneway", t(20, 20), ROAD_X, ROADTYPE_ROAD, DRD_SOUTHBOUND, TownID::Invalid());
	ChangeTileOwner(t(20, 20), other, INVALID_OWNER);
	RoadRun<CMD_BUILD_ROAD_DEPOT>("transfer-depot", t(21, 20), ROADTYPE_ROAD, DIAGDIR_NE);
	ChangeTileOwner(t(21, 20), owner, other);
	/* Bankruptcy clears the owner fields, so observe both road/tram owners first. */
	bool merged = IsRoadDepotTile(t(21, 20)) && GetTileOwner(t(21, 20)) == other && GetRoadOwner(t(21, 20), RTT_ROAD) == other && GetRoadOwner(t(21, 20), RTT_TRAM) == other;
	Backup<CompanyID> bankrupt(_current_company, other);
	ChangeTileOwner(t(21, 20), other, INVALID_OWNER);
	bankrupt.Restore();
	fmt::print(stderr, "ROADCMD transfer-depot {} {}\n", merged, IsTileType(t(21, 20), MP_CLEAR));
	TunnelBridgeCommands(owner, other, rv);
	UpdateNearestTownForRoadTiles(false);
	SaveRandomSeeds(&after);
	bool same = before.random.state[0] == after.random.state[0] && before.random.state[1] == after.random.state[1] && before.interactive_random.state[0] == after.interactive_random.state[0] && before.interactive_random.state[1] == after.interactive_random.state[1];
	fmt::print(stderr, "ROADCMD rng {}\n", same);
	company.Restore();
	return true;
}
