/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_tunnel.cpp Implementation of ScriptTunnel. */

#include "../../stdafx.h"
#include "script_tunnel.hpp"
#include "script_rail.hpp"
#include "../script_instance.hpp"
#include "../../tunnel_map.h"
#include "../../landscape_cmd.h"
#include "../../road_cmd.h"
#include "../../tunnelbridge_cmd.h"

#include "../../safeguards.h"

/**
 * Helper function to connect a just built tunnel to nearby roads.
 * @param instance The script instance we have to built the road for.
 */
static void _DoCommandReturnBuildTunnel2(class ScriptInstance &instance)
{
	if (!ScriptTunnel::_BuildTunnelRoad2()) {
		ScriptInstance::DoCommandReturn(instance);
		return;
	}

	/* This can never happen, as in test-mode this callback is never executed,
	 *  and in execute-mode, the other callback is called. */
	NOT_REACHED();
}

/**
 * Helper function to connect a just built tunnel to nearby roads.
 * @param instance The script instance we have to built the road for.
 */
static void _DoCommandReturnBuildTunnel1(class ScriptInstance &instance)
{
	if (!ScriptTunnel::_BuildTunnelRoad1()) {
		ScriptInstance::DoCommandReturn(instance);
		return;
	}

	/* This can never happen, as in test-mode this callback is never executed,
	 *  and in execute-mode, the other callback is called. */
	NOT_REACHED();
}

/* static */ bool ScriptTunnel::BuildTunnel(ScriptVehicle::VehicleType vehicle_type, TileIndex start)
{
	EnforceDeityOrCompanyModeValid(false);
	if (!openttd_rust_script_tunnel_build(&GetRustScriptHostServices(), static_cast<uint32_t>(vehicle_type), start.base())) return false;

	if (vehicle_type == ScriptVehicle::VT_RAIL) {
		/* For rail we do nothing special */
		return ScriptObject::Command<CMD_BUILD_TUNNEL>::Do(start, TRANSPORT_RAIL, ScriptRail::GetCurrentRailType());
	} else {
		ScriptObject::SetCallbackVariable(0, start.base());
		return ScriptObject::Command<CMD_BUILD_TUNNEL>::Do(&::_DoCommandReturnBuildTunnel1, start, TRANSPORT_ROAD, ScriptRoad::GetCurrentRoadType());
	}
}

/* static */ bool ScriptTunnel::_BuildTunnelRoad1()
{
	EnforceDeityOrCompanyModeValid(false);

	/* Build the piece of road on the 'start' side of the tunnel */
	TileIndex end(ScriptObject::GetCallbackVariable(0));
	TileIndex start{ScriptTunnel::GetOtherTunnelEnd(end)};
	auto piece = openttd_rust_script_tunnelbridge_road_piece(&GetRustScriptHostServices(), end.base(), start.base(), 1);

	return ScriptObject::Command<CMD_BUILD_ROAD>::Do(&::_DoCommandReturnBuildTunnel2, TileIndex(piece.tile), static_cast<RoadBits>(piece.bits), ScriptRoad::GetRoadType(), DRD_NONE, TownID::Invalid());
}

/* static */ bool ScriptTunnel::_BuildTunnelRoad2()
{
	EnforceDeityOrCompanyModeValid(false);

	/* Build the piece of road on the 'end' side of the tunnel */
	TileIndex end(ScriptObject::GetCallbackVariable(0));
	TileIndex start{ScriptTunnel::GetOtherTunnelEnd(end)};
	auto piece = openttd_rust_script_tunnelbridge_road_piece(&GetRustScriptHostServices(), end.base(), start.base(), 0);

	return ScriptObject::Command<CMD_BUILD_ROAD>::Do(TileIndex(piece.tile), static_cast<RoadBits>(piece.bits), ScriptRoad::GetRoadType(), DRD_NONE, TownID::Invalid());
}

/* static */ bool ScriptTunnel::RemoveTunnel(TileIndex tile)
{
	EnforceCompanyModeValid(false);
	EnforcePrecondition(false, IsTunnelTile(tile));

	return ScriptObject::Command<CMD_LANDSCAPE_CLEAR>::Do(tile);
}
