/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
/** @file water-console.cpp Identical real-map water command and restoration inputs. */
#include "stdafx.h"
#include "console_internal.h"
#include "console_func.h"
#include "command_func.h"
#include "water_cmd.h"
#include "vehicle_cmd.h"
#include "water.h"
#include "water_map.h"
#include "clear_map.h"
#include "company_base.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "depot_base.h"
#include "landscape_cmd.h"
#include "tile_cmd.h"
#include "station_base.h"
#include "openttd.h"
extern const TileTypeProcs _tile_type_water_procs;
static void WaterCost(const char *name, CommandCost cost)
{
	fmt::print(stderr, "WATER-CMD cost {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage());
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
	WaterLand(tile);
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
			TileIndex selected = part == LockPart::Middle ? tile : part == LockPart::Lower ? lower : upper;
			WaterCost("remove-lock-test", Command<CMD_LANDSCAPE_CLEAR>::Do({}, selected));
			WaterCost("remove-lock", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, selected));
			for (TileIndex t : {tile, upper, lower}) WaterState("restored-lock", t);
		}
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
		mode.Restore();
	}
	WaterLand(tile, 0); MakeSea(tile); MakeWaterKeepingClass(tile, _current_company); WaterState("sea-zero", tile);
	MakeShore(tile); SetTileHeight(tile, 1); WaterCost("coast", _tile_type_water_procs.clear_tile_proc(tile, DoCommandFlag::NoWater));
	MakeCanal(tile, _current_company, 0); _tile_type_water_procs.change_tile_owner_proc(tile, _current_company, CompanyID(0)); WaterState("transfer", tile);
	WaterLand(tile); MakeCanal(tile, _current_company, 0); MakeCanal(tile + 1, _current_company, 0);
	WaterCost("bankrupt-depot", Command<CMD_BUILD_SHIP_DEPOT>::Do(DoCommandFlag::Execute, tile, AXIS_X));
	_tile_type_water_procs.change_tile_owner_proc(tile, _current_company, INVALID_OWNER);
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
			fmt::print(stderr, "WATER-CMD docking {} {}\n", st->docking_station.Contains(t), IsDockingTile(t));
			checked_docking = true; break;
		}
		if (checked_docking) break;
	}
	fmt::print(stderr, "WATER-CMD complete\n");
	local.Restore(); company.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv(); IConsole::CmdRegister("water_scenario", WaterScenario);
}
