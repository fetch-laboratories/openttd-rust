/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
/** @file water-console.cpp Identical real-map water command and restoration inputs. */
#include "stdafx.h"
#include "console_internal.h"
#include "console_func.h"
#include "command_func.h"
#include "water_cmd.h"
#include "vehicle_cmd.h"
#include "vehicle_base.h"
#include "water.h"
#include "water_map.h"
#include "clear_map.h"
#include "company_base.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "landscape_cmd.h"
#include "tile_cmd.h"
#include "landscape.h"
#include "road_map.h"
#include "town.h"
#include "object_cmd.h"
#include "object_type.h"
#ifdef WITH_RUST
#include "rust/command_services.h"
#endif
#include "station_base.h"
#include "waypoint_base.h"
#include "rail_cmd.h"
#include "road_cmd.h"
#include "station_cmd.h"
#include "waypoint_cmd.h"
#include "tunnelbridge_cmd.h"
#include "industry.h"
#include "newgrf_station.h"
#include "newgrf_roadstop.h"
#include "airport.h"
#include "signal_func.h"
#include "openttd.h"
extern const TileTypeProcs _tile_type_water_procs;
static void WaterCost(const char *name, CommandCost cost)
{
	fmt::print(stderr, "WATER-CMD cost {} {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), cost.GetErrorOwner().base());
	if (!cost.GetEncodedMessage().empty()) fmt::print(stderr, "WATER-CMD parameter {} {}\n", name, cost.GetEncodedMessage().GetDecodedString());
}
static void WaterState(const char *name, TileIndex tile)
{
	fmt::print(stderr, "WATER-CMD state {} {} {} {} {} {} {} {} {}\n", name, tile.base(), GetTileType(tile), GetTileOwner(tile).base(), Tile{tile}.m1(), Tile{tile}.m2(), Tile{tile}.m4(), Tile{tile}.m5(), Company::Get(_current_company)->infrastructure.water);
	fmt::print(stderr, "WATER-CMD rng {} {}\n", _random.state[0], _random.state[1]);
}
static void WaterLand(TileIndex tile, uint height = 3)
{
	for (int y = -4; y <= 5; ++y) for (int x = -4; x <= 5; ++x) {
		TileIndex t = tile + TileDiffXY(x, y);
		MakeClear(t, CLEAR_GRASS, 3); SetTileHeight(t, height);
	}
}
static bool WaterScenario([[maybe_unused]] std::span<std::string_view> args)
{
	Backup<CompanyID> company(_current_company, CompanyID(1));
	Backup<CompanyID> local(_local_company, CompanyID(1));
	TileIndex tile = TileXY(40, 40);
	/* Captured-owner checks intentionally differ from the tile's owner. */
	MakeRoadNormal(tile, ROAD_X, ROADTYPE_ROAD, INVALID_ROADTYPE, (*Town::Iterate().begin())->index, OWNER_TOWN, OWNER_NONE);
	for (CompanyID viewer : {CompanyID(0), CompanyID(1)}) {
		_local_company = viewer;
		for (Owner captured : {Owner(0), Owner(1), OWNER_TOWN}) for (TileIndex at : {TileIndex{0}, tile}) {
			if (captured == OWNER_TOWN && at == 0) continue; // original precondition
			CommandCost cost;
#ifdef WITH_RUST
			openttd_rust_company_check_ownership(GetRustCommandServices().company(), RustCommandCost(&cost), captured.base(), at.base());
#else
			cost = CheckOwnership(captured, at);
#endif
			fmt::print(stderr, "WATER-CMD captured {} {} {}\n", viewer.base(), captured.base(), at.base());
			WaterCost("captured-owner", cost);
		}
	}
	_local_company = CompanyID(1);
	/* Restore first, then dispatch only the originally requested tile. */
	const char *restored[] = {"clear", "sea", "elevated-sea-canal", "canal", "river", "sloped-canal-land", "sloped-river-land"};
	for (uint kind = 0; kind < 7; ++kind) for (bool mismatch : {false, true}) for (bool bankrupt : {false, true}) {
		WaterLand(tile, kind == 1 ? 0 : 3);
		if (kind != 0) {
			WaterClass wc = kind == 1 || kind == 2 ? WaterClass::Sea : kind == 4 || kind == 6 ? WaterClass::River : WaterClass::Canal;
			MakeWater(tile, _current_company, wc, 7);
			if (kind >= 5) SetTileHeight(tile, 4);
			MakeWaterKeepingClass(tile, _current_company);
		}
		Owner old = mismatch ? Owner(0) : GetTileOwner(tile);
		ChangeTileOwner(tile, old, bankrupt ? INVALID_OWNER : Owner(0));
		fmt::print(stderr, "WATER-CMD restored-owner {} {} {} {} {} {} {} {} {}\n", restored[kind], mismatch, bankrupt, Company::Get(CompanyID(0))->infrastructure.water, IsWaterTile(tile) && IsDockingTile(tile), Tile{tile}.m3(), Tile{tile}.m6(), Tile{tile}.m7(), Tile{tile}.m8());
		WaterState("restored-owner", tile);
	}

	WaterLand(tile);
	WaterCost("native-object", Command<CMD_BUILD_OBJECT>::Do(DoCommandFlag::Execute, tile, OBJECT_OWNED_LAND, 0));
	ChangeTileOwner(tile, _current_company, Owner(0)); WaterState("native-object-transfer", tile);
	ChangeTileOwner(tile, Owner(0), INVALID_OWNER); WaterState("native-object-bankrupt", tile);
	for (Axis axis : {AXIS_X, AXIS_Y}) {
		TileIndex other = tile + TileOffsByAxis(axis);
		WaterCost("canal-test", Command<CMD_BUILD_CANAL>::Do({}, tile, other, WaterClass::Canal, false));
		WaterCost("canal", Command<CMD_BUILD_CANAL>::Do(DoCommandFlag::Execute, tile, other, WaterClass::Canal, false));
		WaterCost("skip", Command<CMD_BUILD_CANAL>::Do(DoCommandFlag::Execute, tile, other, WaterClass::Canal, false));
		WaterCost("depot-test", Command<CMD_BUILD_SHIP_DEPOT>::Do({}, tile, axis));
		WaterCost("depot", Command<CMD_BUILD_SHIP_DEPOT>::Do(DoCommandFlag::Execute, tile, axis));
		WaterState("depot", other);
		auto built = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, EngineID(206), false, INVALID_CARGO, INVALID_CLIENT_ID);
		WaterCost("ship", std::get<0>(built));
		WaterCost("occupied", Command<CMD_LANDSCAPE_CLEAR>::Do({}, tile));
		WaterCost("bankrupt-occupied", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Bankrupt, tile));
		WaterCost("sell", Command<CMD_SELL_VEHICLE>::Do(DoCommandFlag::Execute, std::get<1>(built), false, false, INVALID_CLIENT_ID));
		WaterCost("auto-depot", _tile_type_water_procs.clear_tile_proc(other, DoCommandFlag::Auto));
		WaterCost("remove-test", _tile_type_water_procs.clear_tile_proc(other, {}));
		WaterCost("remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, other));
		WaterState("restore-depot", tile); WaterState("restore-other", other);
		WaterCost("depot-again", Command<CMD_BUILD_SHIP_DEPOT>::Do(DoCommandFlag::Execute, tile, axis));
		WaterCost("force", Command<CMD_LANDSCAPE_CLEAR>::Do({DoCommandFlag::Execute, DoCommandFlag::ForceClearTile}, tile));
		WaterState("force-first", tile); WaterState("force-other", other);
		WaterCost("no-water", _tile_type_water_procs.clear_tile_proc(other, DoCommandFlag::NoWater));
		WaterCost("clear-canal", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, other));
	}
	/* Buoys (station_cmd/waypoint.rs): water classes, occupied refusal, bankruptcy and deleted-buoy reuse. */
	for (WaterClass wc : {WaterClass::Sea, WaterClass::Canal, WaterClass::River}) {
		tile = TileXY(45, 45); WaterLand(tile, wc == WaterClass::Sea ? 0 : 3);
		MakeWater(tile, wc == WaterClass::Canal ? Owner(_current_company) : OWNER_WATER, wc, 7);
		WaterCost("buoy-test", Command<CMD_BUILD_BUOY>::Do({}, tile));
		WaterCost("buoy-build", Command<CMD_BUILD_BUOY>::Do(DoCommandFlag::Execute, tile));
		if (!IsBuoyTile(tile)) std::abort();
		StationID id = GetStationIndex(tile); WaterState("buoy", tile);
		Vehicle *v = nullptr; for (Vehicle *ship : Vehicle::Iterate()) if (ship->type == VEH_SHIP && !ship->vehstatus.Test(VehState::Hidden)) { v = ship; break; }
		if (v == nullptr) std::abort();
		TileIndex old_tile = v->tile; int old_x = v->x_pos, old_y = v->y_pos, old_z = v->z_pos;
		v->tile = tile; v->x_pos = TileX(tile) * TILE_SIZE + 8; v->y_pos = TileY(tile) * TILE_SIZE + 8; v->z_pos = GetTileZ(tile); v->UpdatePosition();
		WaterCost("buoy-occupied", Command<CMD_LANDSCAPE_CLEAR>::Do({}, tile));
		WaterCost("buoy-bankrupt", Command<CMD_LANDSCAPE_CLEAR>::Do({DoCommandFlag::Execute, DoCommandFlag::Bankrupt}, tile));
		v->tile = old_tile; v->x_pos = old_x; v->y_pos = old_y; v->z_pos = old_z; v->UpdatePosition();
		if (!IsWaterTile(tile) || GetWaterClass(tile) != wc) std::abort();
		WaterState("buoy-restored", tile);
		WaterCost("buoy-reuse", Command<CMD_BUILD_BUOY>::Do(DoCommandFlag::Execute, tile));
		if (GetStationIndex(tile) != id) std::abort();
		WaterCost("buoy-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile));
	}
	WaterCost("invalid-axis", Command<CMD_BUILD_SHIP_DEPOT>::Do({}, tile, INVALID_AXIS));
	WaterCost("invalid-lock", Command<CMD_BUILD_LOCK>::Do({}, tile));
	WaterCost("diagonal", Command<CMD_BUILD_CANAL>::Do(DoCommandFlag::Execute, tile + TileDiffXY(2, 2), tile, WaterClass::Canal, true));
	for (DiagDirection dir = DIAGDIR_BEGIN; dir < DIAGDIR_END; ++dir) {
		for (LockPart part = LockPart::Middle; part < LockPart::End; ++part) {
			tile = TileXY(60 + uint(dir) * 12, 60 + uint(part) * 12);
			WaterLand(tile);
			/* A planar step aligned with the selected uphill direction. */
			for (int y = -3; y <= 4; ++y) for (int x = -3; x <= 4; ++x) {
				bool high = dir == DIAGDIR_NE ? x <= 0 : dir == DIAGDIR_SE ? y >= 1 : dir == DIAGDIR_SW ? x >= 1 : y <= 0;
				SetTileHeight(tile + TileDiffXY(x, y), 3 + high);
			}
			TileIndex lower = tile - TileOffsByDiagDir(dir), upper = tile + TileOffsByDiagDir(dir);
			if (part == LockPart::Upper) for (TileIndex t : {tile, lower, upper}) MakeRiver(t, 7);
			WaterCost("lock-test", Command<CMD_BUILD_LOCK>::Do({}, tile));
			WaterCost("lock", Command<CMD_BUILD_LOCK>::Do(DoCommandFlag::Execute, tile));
			WaterState("lock", tile);
			WaterCost("auto-lock", _tile_type_water_procs.clear_tile_proc(tile, DoCommandFlag::Auto));
			WaterCost("above-lock", _tile_type_water_procs.check_build_above_proc(lower, {}, AXIS_X, GetTileMaxZ(lower)));
			WaterCost("above-lock-int-min", _tile_type_water_procs.check_build_above_proc(lower, {}, AXIS_X, INT32_MIN));
			TileIndex selected = part == LockPart::Middle ? tile : part == LockPart::Lower ? lower : upper;
			{ Backup<CompanyID> water(_current_company, OWNER_WATER); WaterCost("water-remove-lock", _tile_type_water_procs.clear_tile_proc(selected, {})); }
			WaterCost("remove-lock-test", Command<CMD_LANDSCAPE_CLEAR>::Do({}, selected));
			WaterCost("remove-lock", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, selected));
			for (TileIndex t : {tile, upper, lower}) WaterState("restored-lock", t);
		}
	}
	/* NE locks: a bridge too low over the lower part (the error uses the middle height), then a sloped upper end. */
	for (uint x : {96u, 108u}) {
		tile = TileXY(x, 132); WaterLand(tile);
		for (int y = -3; y <= 4; ++y) for (int dx = -3; dx <= 4; ++dx) SetTileHeight(tile + TileDiffXY(dx, y), 3 + (dx <= 0));
	}
	WaterCost("low-bridge", Command<CMD_BUILD_BRIDGE>::Do(DoCommandFlag::Execute, TileXY(97, 134), TileXY(97, 130), TRANSPORT_ROAD, 0, ROADTYPE_ROAD));
	WaterCost("low-bridge-lock", Command<CMD_BUILD_LOCK>::Do({}, TileXY(96, 132)));
	SetTileHeight(TileXY(107, 132), 5);
	WaterCost("sloped-end-lock", CmdBuildLock(DoCommandFlag::Execute, TileXY(108, 132)));
	for (uint x : {107u, 108u, 109u}) WaterState("sloped-end", TileXY(x, 132));
	/* An NE lock site whose lower end is the last sea column, and that sea tile, without freeform edges. */
	for (uint y = 99; y <= 101; ++y) for (uint x = 251; x <= 253; ++x) MakeClear(TileXY(x, y), CLEAR_GRASS, 3);
	for (uint y = 100; y <= 101; ++y) for (uint x = 252; x <= 253; ++x) SetTileHeight(TileXY(x, y), 1);
	for (bool freeform : {false, true}) {
		Backup<bool> edges(_settings_game.construction.freeform_edges, freeform);
		WaterCost(freeform ? "edge-lock-freeform" : "edge-lock", Command<CMD_BUILD_LOCK>::Do({}, TileXY(253, 100)));
		WaterCost(freeform ? "edge-clear-freeform" : "edge-clear", _tile_type_water_procs.clear_tile_proc(TileXY(254, 104), {}));
	}
	for (WaterClass wc : {WaterClass::Sea, WaterClass::Canal, WaterClass::River}) {
		tile = TileXY(120 + to_underlying(wc) * 12, 120); WaterLand(tile);
		MakeWater(tile, wc == WaterClass::Canal ? Owner(_current_company) : OWNER_WATER, wc, 7);
		MakeWaterKeepingClass(tile, _current_company); WaterState("restored-class", tile);
		WaterCost("terraform", _tile_type_water_procs.terraform_tile_proc(tile, {}, 3, SLOPE_FLAT));
		WaterCost("above-water", _tile_type_water_procs.check_build_above_proc(tile, {}, AXIS_X, 0));
		SetTileHeight(tile, 4); MakeWaterKeepingClass(tile, _current_company); WaterState("sloped-class", tile);
	}
	tile = TileXY(160, 160); WaterLand(tile);
	{
		Backup<GameMode> mode(_game_mode, GM_EDITOR);
		for (int y = -2; y <= 2; ++y) for (int x = -2; x <= 2; ++x) SetTropicZone(tile + TileDiffXY(x, y), TROPICZONE_DESERT);
		WaterCost("editor-sea", Command<CMD_BUILD_CANAL>::Do(DoCommandFlag::Execute, tile, tile, WaterClass::Sea, false));
		WaterState("editor-sea", tile);
		WaterCost("editor-river", Command<CMD_BUILD_CANAL>::Do(DoCommandFlag::Execute, tile, tile, WaterClass::River, false));
		WaterState("editor-river", tile);
		fmt::print(stderr, "WATER-CMD desert {}\n", GetTropicZone(tile) == TROPICZONE_NORMAL);
		MakeRiverAndModifyDesertZoneAround(tile);
		WaterLand(tile, 0);
		WaterCost("editor-sea-zero", Command<CMD_BUILD_CANAL>::Do(DoCommandFlag::Execute, tile, tile, WaterClass::Sea, false));
		WaterState("editor-sea-zero", tile);
		mode.Restore();
	}
	WaterLand(tile, 0); MakeSea(tile); MakeWaterKeepingClass(tile, _current_company); WaterState("sea-zero", tile);
	MakeShore(tile); SetTileHeight(tile, 1); WaterCost("coast", _tile_type_water_procs.clear_tile_proc(tile, DoCommandFlag::NoWater));
	MakeCanal(tile, _current_company, 0); ChangeTileOwner(tile, _current_company, CompanyID(0)); WaterState("transfer", tile);
	WaterLand(tile); MakeCanal(tile, _current_company, 0); MakeCanal(tile + 1, _current_company, 0);
	WaterCost("bankrupt-depot", Command<CMD_BUILD_SHIP_DEPOT>::Do(DoCommandFlag::Execute, tile, AXIS_X));
	ChangeTileOwner(tile, _current_company, INVALID_OWNER);
	WaterState("bankrupt", tile); WaterState("bankrupt-other", tile + 1);
	tile = TileXY(180, 160); WaterLand(tile);
	MakeCanal(tile, _current_company, 0); MakeCanal(tile + 1, CompanyID(0), 0);
	WaterCost("second-clear-failure", CmdBuildShipDepot(DoCommandFlag::Execute, tile, AXIS_X));
	WaterState("partial-first", tile); WaterState("partial-second", tile + 1);
	bool checked_docking = false;
	for (Station *st : Station::Iterate()) {
		for (TileIndex t : st->docking_station) {
			if (!IsWaterTile(t) || !IsDockingTile(t)) continue;
			MakeWaterKeepingClass(t, GetTileOwner(t));
			ChangeTileOwner(t, GetTileOwner(t), Owner(0));
			fmt::print(stderr, "WATER-CMD docking {}\n", IsDockingTile(t));
			checked_docking = true; break;
		}
		if (checked_docking) break;
	}
	fmt::print(stderr, "WATER-CMD complete\n");
	local.Restore(); company.Restore();
	return true;
}
/* Company 1's station, crossing, water and tunnel/bridge tiles on flat height-1 land (#156). */
static const struct { const char *name; int x, y; } _owner_tiles[] = {
	{"drive-bus", 2, 1}, {"drive-truck", 3, 1}, {"road-waypoint", 4, 1}, {"crossing", 6, 1}, {"rail", 6, 2}, {"foreign-crossing", 6, 3},
	{"bay", 1, 5}, {"rail-station", 2, 7}, {"rail-waypoint", 5, 7}, {"airport", 1, 9}, {"lock-upper", 10, 3}, {"lock", 10, 4},
	{"lock-lower", 10, 5}, {"dock", 13, 4}, {"dock-water", 13, 5}, {"canal", 12, 6}, {"depot", 8, 7}, {"depot-other", 9, 7},
	{"buoy", 11, 7}, {"rail-bridge", 6, 9}, {"road-bridge", 7, 9}, {"rail-tunnel", 9, 12}, {"road-tunnel", 9, 13},
};
static void OwnerState(const char *phase, TileIndex base, uint industry)
{
	for (const Company *c : Company::Iterate()) {
		const auto &infra = c->infrastructure;
		fmt::print(stderr, "WATER-CMD owner-infra {} {} {} {} {} {} {} {}\n", phase, c->index.base(), infra.rail[RAILTYPE_RAIL], infra.road[ROADTYPE_ROAD], infra.signal, infra.water, infra.station, infra.airport);
	}
	for (const auto &[name, x, y] : _owner_tiles) {
		Tile t{base + TileDiffXY(x, y)};
		fmt::print(stderr, "WATER-CMD owner {} {} {} {} {} {} {} {} {} {} {}\n", phase, name, GetTileType(t), t.m1(), t.m2(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8());
	}
	for (uint id = industry; id < industry + 2; ++id) {
		const Industry *i = Industry::Get(id);
		fmt::print(stderr, "WATER-CMD owner-industry {} {} {} {} {}\n", phase, id, i->founder.base(), i->exclusive_supplier.base(), i->exclusive_consumer.base());
	}
	fmt::print(stderr, "WATER-CMD rng {} {}\n", _random.state[0], _random.state[1]);
}
static void OwnerBuild(TileIndex base, uint industry)
{
	auto at = [base](int x, int y) { return base + TileDiffXY(x, y); };
	const DoCommandFlags exec = DoCommandFlag::Execute;
	for (int y = 0; y < 16; ++y) for (int x = 0; x < 16; ++x) MakeClear(at(x, y), CLEAR_GRASS, 3);
	/* Inner corners one level up: a lock/dock plateau and a tunnel hill. */
	for (int y = 1; y <= 4; ++y) for (int x = 9; x <= 15; ++x) SetTileHeight(at(x, y), 2);
	for (int y = 12; y <= 14; ++y) for (int x = 10; x <= 13; ++x) SetTileHeight(at(x, y), 2);
	for (int y = 1; y <= 3; ++y) WaterCost("owner-rail", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, at(6, y), RAILTYPE_RAIL, TRACK_Y, false));
	for (int x = 1; x <= 7; ++x) WaterCost("owner-road", Command<CMD_BUILD_ROAD>::Do(exec, at(x, 1), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	{
		Backup<CompanyID> other(_current_company, CompanyID(0));
		for (int x = 5; x <= 7; ++x) WaterCost("owner-road", Command<CMD_BUILD_ROAD>::Do(exec, at(x, 3), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	}
	for (auto [x, type] : {std::pair{2, RoadStopType::Bus}, std::pair{3, RoadStopType::Truck}}) {
		WaterCost("owner-stop", Command<CMD_BUILD_ROAD_STOP>::Do(exec, at(x, 1), 1, 1, type, true, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, StationID::Invalid(), false));
	}
	WaterCost("owner-stop", Command<CMD_BUILD_ROAD_WAYPOINT>::Do(exec, at(4, 1), AXIS_X, 1, 1, ROADSTOP_CLASS_WAYP, 0, StationID::Invalid(), false));
	WaterCost("owner-stop", Command<CMD_BUILD_ROAD_STOP>::Do(exec, at(1, 5), 1, 1, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, StationID::Invalid(), false));
	WaterCost("owner-station", Command<CMD_BUILD_RAIL_STATION>::Do(exec, at(1, 7), RAILTYPE_RAIL, AXIS_X, 1, 3, STAT_CLASS_DFLT, 0, StationID::Invalid(), false));
	WaterCost("owner-rail", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, at(5, 7), RAILTYPE_RAIL, TRACK_X, false));
	WaterCost("owner-station", Command<CMD_BUILD_RAIL_WAYPOINT>::Do(exec, at(5, 7), AXIS_X, 1, 1, STAT_CLASS_WAYP, 0, StationID::Invalid(), false));
	WaterCost("owner-station", Command<CMD_BUILD_AIRPORT>::Do(exec, at(1, 9), AT_SMALL, 0, StationID::Invalid(), false));
	WaterCost("owner-water", Command<CMD_BUILD_CANAL>::Do(exec, at(8, 5), at(14, 7), WaterClass::Canal, false));
	WaterCost("owner-water", Command<CMD_BUILD_LOCK>::Do(exec, at(10, 4)));
	WaterCost("owner-station", Command<CMD_BUILD_DOCK>::Do(exec, at(13, 4), StationID::Invalid(), false));
	WaterCost("owner-water", Command<CMD_BUILD_SHIP_DEPOT>::Do(exec, at(8, 7), AXIS_X));
	WaterCost("owner-station", Command<CMD_BUILD_BUOY>::Do(exec, at(11, 7)));
	WaterCost("owner-bridge", Command<CMD_BUILD_BRIDGE>::Do(exec, at(6, 13), at(6, 9), TRANSPORT_RAIL, 0, RAILTYPE_RAIL));
	WaterCost("owner-bridge", Command<CMD_BUILD_BRIDGE>::Do(exec, at(7, 13), at(7, 9), TRANSPORT_ROAD, 0, ROADTYPE_ROAD));
	WaterCost("owner-tunnel", Command<CMD_BUILD_TUNNEL>::Do(exec, at(9, 12), TRANSPORT_RAIL, RAILTYPE_RAIL));
	WaterCost("owner-tunnel", Command<CMD_BUILD_TUNNEL>::Do(exec, at(9, 13), TRANSPORT_ROAD, ROADTYPE_ROAD));
	/* The first industry has every role; the second only supplies exclusively. */
	Industry *i = Industry::Get(industry);
	i->founder = i->exclusive_supplier = i->exclusive_consumer = _current_company;
	Industry::Get(industry + 1)->exclusive_supplier = _current_company;
	UpdateSignalsInBuffer(); // as after each top-level command; Do() leaves the buffer filled
}
static bool OwnerScenario([[maybe_unused]] std::span<std::string_view> args)
{
	Backup<CompanyID> company(_current_company, CompanyID(1));
	Backup<CompanyID> local(_local_company, CompanyID(0));
	/* A merger into a live company, then a bankruptcy, through the real company transfer. */
	for (auto [phase, base, industry, next] : {std::tuple{"merge", TileXY(182, 114), 0u, Owner(0)}, std::tuple{"bankrupt", TileXY(188, 138), 2u, INVALID_OWNER}}) {
		OwnerBuild(base, industry);
		OwnerState("built", base, industry);
		ChangeOwnershipOfCompanyItems(CompanyID(1), next);
		OwnerState(phase, base, industry);
	}
	fmt::print(stderr, "WATER-CMD complete\n");
	local.Restore(); company.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv(); IConsole::CmdRegister("water_scenario", WaterScenario);
	IConsole::CmdRegister("owner_scenario", OwnerScenario);
}
