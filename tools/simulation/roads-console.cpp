/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file roads-console.cpp Road-command inputs, included by the shared signal fixture. */
#include "town.h"
#include "road.h"
#include "newgrf_roadstop.h"
extern RoadTypeInfo _roadtypes[ROADTYPE_END];
extern RoadTypes _roadtypes_road;
#include "road_func.h"
#include "landscape.h"
#include "tile_type.h"
#include "depot_func.h"
#include "effectvehicle_func.h"
#include "effectvehicle_base.h"
extern const TileTypeProcs _tile_type_road_procs;
static void RoadPrint(std::string_view name, const CommandCost &cost, Money extra = 0)
{
	fmt::print(stderr, "ROADCMD {} {} {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), static_cast<int>(cost.GetExpensesType()), extra);
}
template <Commands Tcmd, typename... Args>
static void RoadRun(std::string_view name, Args... args)
{
	for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Execute}}) {
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		DoCommandFlags command_flags = flags;
		if constexpr (Tcmd == CMD_BUILD_ROAD || Tcmd == CMD_BUILD_LONG_ROAD || Tcmd == CMD_BUILD_ROAD_DEPOT || Tcmd == CMD_BUILD_SINGLE_RAIL) command_flags.Set(DoCommandFlag::Auto);
		auto result = Command<Tcmd>::Do(command_flags, args...);
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
	RoadRun<CMD_BUILD_ROAD>("zero", t(2, 2), ROAD_NONE, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_LONG_ROAD>("drag", t(12, 2), t(2, 2), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, false);
	RoadRun<CMD_BUILD_LONG_ROAD>("reverse", t(2, 2), t(12, 2), ROADTYPE_ROAD, AXIS_X, DRD_SOUTHBOUND, true, false, true);
	RoadRun<CMD_BUILD_ROAD>("oneway", t(5, 2), ROAD_X, ROADTYPE_ROAD, DRD_NORTHBOUND, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("junction", t(5, 2), ROAD_Y, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("tram", t(6, 2), ROAD_X, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("half-tram", t(15, 2), ROAD_NE, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD_DEPOT>("depot", t(8, 5), ROADTYPE_ROAD, DIAGDIR_NE);
	RoadRun<CMD_BUILD_ROAD_DEPOT>("depot-same", t(8, 5), ROADTYPE_ROAD, DIAGDIR_NE);
	RoadRun<CMD_BUILD_ROAD_DEPOT>("rotate", t(8, 5), ROADTYPE_ROAD, DIAGDIR_SE);
	RoadRun<CMD_BUILD_LONG_ROAD>("obstacle", t(12, 5), t(2, 5), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, false);
	RoadRun<CMD_BUILD_LONG_ROAD>("ai-obstacle", t(12, 5), t(2, 5), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, true);
	RoadRun<CMD_BUILD_LONG_ROAD>("half-single", t(15, 5), t(15, 5), ROADTYPE_ROAD, AXIS_Y, DRD_NONE, true, true, true);
	RoadRun<CMD_BUILD_LONG_ROAD>("invalid-drag", t(12, 6), t(2, 5), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, false);
	RoadRun<CMD_CONVERT_ROAD>("convert", t(12, 2), t(2, 2), alternate, false);
	RoadRun<CMD_CONVERT_ROAD>("convert-back", t(12, 2), t(2, 2), ROADTYPE_ROAD, true);
	RoadRun<CMD_REMOVE_LONG_ROAD>("remove-halves", t(12, 2), t(2, 2), ROADTYPE_ROAD, AXIS_X, true, false);
	RoadRun<CMD_REMOVE_LONG_ROAD>("remove-tram", t(6, 2), t(6, 2), ROADTYPE_TRAM, AXIS_X, false, true);
	/* Crossing construction over a reserved track and tram-first clearing. */
	RoadRun<CMD_BUILD_SINGLE_RAIL>("rail", t(3, 9), RAILTYPE_RAIL, TRACK_Y, false);
	SetTrackReservation(t(3, 9), TRACK_BIT_Y);
	RoadRun<CMD_BUILD_ROAD>("crossing", t(3, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_BUILD_ROAD>("crossing-tram", t(3, 9), ROAD_X, ROADTYPE_TRAM, DRD_NONE, TownID::Invalid());
	RoadRun<CMD_REMOVE_LONG_ROAD>("crossing-remove-tram", t(3, 9), t(3, 9), ROADTYPE_TRAM, AXIS_X, false, true);
	RoadRun<CMD_REMOVE_LONG_ROAD>("crossing-remove", t(3, 9), t(3, 9), ROADTYPE_ROAD, AXIS_X, false, true);
	RoadRun<CMD_BUILD_ROAD>("crossing-again", t(3, 9), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
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
	RoadPrint("build-above", _tile_type_road_procs.check_build_above_proc(t(8, 5), {}, AXIS_X, 64));
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
	/* Low-funds removal returns the full remaining estimate, retaining prior effects. */
	RoadRun<CMD_BUILD_LONG_ROAD>("money-road", t(12, 20), t(2, 20), ROADTYPE_ROAD, AXIS_X, DRD_NONE, false, true, true);
#ifdef WITH_RUST
	Company::Get(owner)->Finances().money = Money(700);
#else
	Company::Get(owner)->money = Money(700);
#endif
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
	RoadRun<CMD_REMOVE_LONG_ROAD>("town-inner", t(16, 20), t(16, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	RoadRun<CMD_REMOVE_LONG_ROAD>("town-edge", t(15, 20), t(15, 20), ROADTYPE_ROAD, AXIS_X, false, true);
	dynamite.Restore();
	RoadRun<CMD_BUILD_ROAD>("transfer-road", t(20, 20), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid());
	_tile_type_road_procs.change_tile_owner_proc(t(20, 20), owner, other);
	RoadRun<CMD_BUILD_ROAD>("foreign-oneway", t(20, 20), ROAD_X, ROADTYPE_ROAD, DRD_SOUTHBOUND, TownID::Invalid());
	_tile_type_road_procs.change_tile_owner_proc(t(20, 20), other, INVALID_OWNER);
	UpdateNearestTownForRoadTiles(false);
	SaveRandomSeeds(&after);
	bool same = before.random.state[0] == after.random.state[0] && before.random.state[1] == after.random.state[1] && before.interactive_random.state[0] == after.interactive_random.state[0] && before.interactive_random.state[1] == after.interactive_random.state[1];
	fmt::print(stderr, "ROADCMD rng {}\n", same);
	company.Restore();
	return true;
}
