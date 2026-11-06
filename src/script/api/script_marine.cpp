/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_marine.cpp Implementation of ScriptMarine. */

#include "../../stdafx.h"
#include "script_marine.hpp"
#include "script_station.hpp"
#include "../../station_base.h"
#include "../../dock_cmd.h"
#include "../../landscape_cmd.h"
#include "../../station_cmd.h"
#include "../../tile_cmd.h"
#include "../../water_cmd.h"
#include "../../waypoint_cmd.h"

#include "../../safeguards.h"


/* static */ bool ScriptMarine::BuildWaterDepot(TileIndex tile, TileIndex front)
{
	EnforceCompanyModeValid(false);
	uint8_t axis = openttd_rust_script_marine_water_depot(&GetRustScriptHostServices(), tile.base(), front.base());
	if (axis == INVALID_AXIS) return false;

	return ScriptObject::Command<CMD_BUILD_SHIP_DEPOT>::Do(tile, static_cast<Axis>(axis));
}

/* static */ bool ScriptMarine::BuildDock(TileIndex tile, StationID station_id)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, station_id == ScriptStation::STATION_NEW || station_id == ScriptStation::STATION_JOIN_ADJACENT || ScriptStation::IsValidStation(station_id));

	return ScriptObject::Command<CMD_BUILD_DOCK>::Do(tile, ScriptStation::IsValidStation(station_id) ? station_id : StationID::Invalid(), station_id != ScriptStation::STATION_JOIN_ADJACENT);
}

/* static */ bool ScriptMarine::BuildBuoy(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));

	return ScriptObject::Command<CMD_BUILD_BUOY>::Do(tile);
}

/* static */ bool ScriptMarine::BuildLock(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));

	return ScriptObject::Command<CMD_BUILD_LOCK>::Do(tile);
}

/* static */ bool ScriptMarine::BuildCanal(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));

	return ScriptObject::Command<CMD_BUILD_CANAL>::Do(tile, tile, WaterClass::Canal, false);
}

/* static */ bool ScriptMarine::RemoveWaterDepot(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, IsWaterDepotTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}

/* static */ bool ScriptMarine::RemoveDock(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, IsDockTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}

/* static */ bool ScriptMarine::RemoveBuoy(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, IsBuoyTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}

/* static */ bool ScriptMarine::RemoveLock(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, IsLockTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}

/* static */ bool ScriptMarine::RemoveCanal(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, ::IsValidTile(tile));
	EnforcePrecondition(false, IsCanalTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}
