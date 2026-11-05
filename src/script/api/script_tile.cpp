/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_tile.cpp Implementation of ScriptTile. */

#include "../../stdafx.h"
#include "script_tile.hpp"
#include "script_map.hpp"
#include "script_town.hpp"
#include "../../station_func.h"
#include "../../town.h"
#include "../../tree_map.h"
#include "../../landscape_cmd.h"
#include "../../terraform_cmd.h"
#include "../../tree_cmd.h"

#include "../../safeguards.h"

/* static */ SQInteger ScriptTile::GetCargoAcceptance(TileIndex tile, CargoType cargo_type, SQInteger width, SQInteger height, SQInteger radius)
{
	if (!::IsValidTile(tile) || width <= 0 || height <= 0 || radius < 0 || !ScriptCargo::IsValidCargo(cargo_type)) return -1;

	CargoArray acceptance = ::GetAcceptanceAroundTiles(tile, width, height, _settings_game.station.modified_catchment ? radius : (int)CA_UNMODIFIED);
	return acceptance[cargo_type];
}

/* static */ SQInteger ScriptTile::GetCargoProduction(TileIndex tile, CargoType cargo_type, SQInteger width, SQInteger height, SQInteger radius)
{
	if (!::IsValidTile(tile) || width <= 0 || height <= 0 || radius < 0 || !ScriptCargo::IsValidCargo(cargo_type)) return -1;

	CargoArray produced = ::GetProductionAroundTiles(tile, width, height, _settings_game.station.modified_catchment ? radius : (int)CA_UNMODIFIED);
	return produced[cargo_type];
}

/* static */ bool ScriptTile::RaiseTile(TileIndex tile, Slope slope)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, tile < ScriptMap::GetMapSize());

	return ScriptObject::Command<CMD_TERRAFORM_LAND>::Do(tile, (::Slope)slope, true);
}

/* static */ bool ScriptTile::LowerTile(TileIndex tile, Slope slope)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, tile < ScriptMap::GetMapSize());

	return ScriptObject::Command<CMD_TERRAFORM_LAND>::Do(tile, (::Slope)slope, false);
}

/* static */ bool ScriptTile::LevelTiles(TileIndex start_tile, TileIndex end_tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, start_tile < ScriptMap::GetMapSize());
	EnforcePrecondition(false, end_tile < ScriptMap::GetMapSize());

	return ScriptObject::Command<CMD_LEVEL_LAND>::Do(end_tile, start_tile, false, LM_LEVEL);
}

/* static */ bool ScriptTile::DemolishTile(TileIndex tile)
{
	EnforceDeityOrCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}

/* static */ bool ScriptTile::PlantTree(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));

	return ScriptObject::Command<CMD_PLANT_TREE>::Do(tile, tile, TREE_INVALID, false);
}

/* static */ bool ScriptTile::PlantTreeRectangle(TileIndex tile, SQInteger width, SQInteger height)
{
	EnforceCompanyModeValid(false);
	TileIndex end_tile{openttd_rust_script_tile_plant_tree_rectangle(&GetRustScriptHostServices(), tile.base(), width, height)};
	if (end_tile == INVALID_TILE) return false;

	return ScriptObject::Command<CMD_PLANT_TREE>::Do(tile, end_tile, TREE_INVALID, false);
}

/* static */ bool ScriptTile::IsWithinTownInfluence(TileIndex tile, TownID town_id)
{
	return ScriptTown::IsWithinTownInfluence(town_id, tile);
}

/* static */ TownID ScriptTile::GetTownAuthority(TileIndex tile)
{
	if (!::IsValidTile(tile)) return TownID::Invalid();

	Town *town = ::ClosestTownFromTile(tile, _settings_game.economy.dist_local_authority);
	if (town == nullptr) return TownID::Invalid();

	return town->index;
}

/* static */ TownID ScriptTile::GetClosestTown(TileIndex tile)
{
	if (!::IsValidTile(tile)) return TownID::Invalid();

	Town *town = ::ClosestTownFromTile(tile, UINT_MAX);
	if (town == nullptr) return TownID::Invalid();

	return town->index;
}
