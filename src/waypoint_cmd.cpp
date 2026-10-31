/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file waypoint_cmd.cpp %Command Handling for waypoints. */

#include "stdafx.h"

#include "command_func.h"
#include "landscape.h"
#include "bridge_map.h"
#include "town.h"
#include "waypoint_base.h"
#include "pathfinder/yapf/yapf_cache.h"
#include "pathfinder/water_regions.h"
#include "tilehighlight_func.h"
#include "strings_func.h"
#include "viewport_func.h"
#include "viewport_kdtree.h"
#include "station_kdtree.h"
#include "window_func.h"
#include "timer/timer_game_calendar.h"
#include "vehicle_func.h"
#include "string_func.h"
#include "company_func.h"
#include "newgrf_debug.h"
#include "newgrf_station.h"
#include "newgrf_roadstop.h"
#include "company_base.h"
#include "water.h"
#include "company_gui.h"
#include "waypoint_cmd.h"
#include "landscape_cmd.h"
#include "station_layout_type.h"

#include "widgets/misc_widget.h"

#include "table/strings.h"

#include "rust/command_services.h"
#include "rust/waypoint_command_ffi.h"
#include "rust/waypoint_layout.h"

#include "safeguards.h"

/**
 * Update the virtual coords needed to draw the waypoint sign.
 */
void Waypoint::UpdateVirtCoord()
{
	Point pt = RemapCoords2(TileX(this->xy) * TILE_SIZE, TileY(this->xy) * TILE_SIZE);
	if (this->sign.kdtree_valid) _viewport_sign_kdtree.Remove(ViewportSignKdtreeItem::MakeWaypoint(this->index));

	this->sign.UpdatePosition(pt.x, pt.y - 32 * ZOOM_BASE, GetString(STR_WAYPOINT_NAME, this->index));

	_viewport_sign_kdtree.Insert(ViewportSignKdtreeItem::MakeWaypoint(this->index));

	/* Recenter viewport */
	InvalidateWindowData(WC_WAYPOINT_VIEW, this->index);
}

/**
 * Move the waypoint main coordinate somewhere else.
 * @param new_xy new tile location of the sign
 */
void Waypoint::MoveSign(TileIndex new_xy)
{
	if (this->xy == new_xy) return;

	this->BaseStation::MoveSign(new_xy);
}

/**
 * Find a deleted waypoint close to a tile.
 * @param tile to search from
 * @param str  the string to get the 'type' of
 * @param cid previous owner of the waypoint
 * @param is_road whether to find a road waypoint
 * @return the deleted nearby waypoint
 */

/**
 * Get the axis for a new rail waypoint. This means that if it is a valid
 * tile to build a waypoint on it returns a valid Axis, otherwise an
 * invalid one.
 * @param tile the tile to look at.
 * @return the axis for the to-be-build waypoint.
 */
Axis GetAxisForNewRailWaypoint(TileIndex tile)
{
	return static_cast<Axis>(openttd_rust_waypoint_rail_axis(&GetRustStationCommandServices(), tile.base()));
}

/**
 * Get the axis for a new road waypoint. This means that if it is a valid
 * tile to build a waypoint on it returns a valid Axis, otherwise an
 * invalid one.
 * @param tile the tile to look at.
 * @return the axis for the to-be-build waypoint.
 */
Axis GetAxisForNewRoadWaypoint(TileIndex tile)
{
	return static_cast<Axis>(openttd_rust_waypoint_road_axis(&GetRustStationCommandServices(), tile.base()));
}


/**
 * Check whether the given tile is suitable for a waypoint.
 * @param tile the tile to check for suitability
 * @param axis the axis of the waypoint
 * @param waypoint Waypoint the waypoint to check for is already joined to. If we find another waypoint it can join to it will throw an error.
 */


/**
 * Convert existing rail to waypoint. Eg build a waypoint station over
 * piece of rail
 * @param flags type of operation
 * @param start_tile northern most tile where waypoint will be built
 * @param axis orientation (Axis)
 * @param width width of waypoint
 * @param height height of waypoint
 * @param spec_class custom station class
 * @param spec_index custom station id
 * @param station_to_join station ID to join (NEW_STATION if build new one)
 * @param adjacent allow waypoints directly adjacent to other waypoints.
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildRailWaypoint(DoCommandFlags flags, TileIndex start_tile, Axis axis, uint8_t width, uint8_t height, StationClassID spec_class, uint16_t spec_index, StationID station_to_join, bool adjacent)
{
	CommandCost cost, ret;
	OpenTTDWaypointLayout layout;
	openttd_rust_waypoint_build_rail(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), &layout, flags.base(), start_tile.base(), axis, width, height, spec_class, spec_index, station_to_join.base(), adjacent);
	return cost;
}

/**
 * Build a road waypoint on an existing road.
 * @param flags type of operation.
 * @param start_tile northern most tile where waypoint will be built.
 * @param axis orientation (Axis).
 * @param width width of waypoint.
 * @param height height of waypoint.
 * @param spec_class custom road stop class.
 * @param spec_index custom road stop id.
 * @param station_to_join station ID to join (NEW_STATION if build new one).
 * @param adjacent allow waypoints directly adjacent to other waypoints.
 * @return the cost of this operation or an error.
 */
CommandCost CmdBuildRoadWaypoint(DoCommandFlags flags, TileIndex start_tile, Axis axis, uint8_t width, uint8_t height, RoadStopClassID spec_class, uint16_t spec_index, StationID station_to_join, bool adjacent)
{
	CommandCost cost, ret;
	openttd_rust_waypoint_build_road(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), start_tile.base(), axis, width, height, spec_class, spec_index, station_to_join.base(), adjacent);
	return cost;
}

/**
 * Build a buoy.
 * @param flags operation to perform
 * @param tile tile where to place the buoy
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildBuoy(DoCommandFlags flags, TileIndex tile)
{
	CommandCost cost, ret;
	openttd_rust_waypoint_build_buoy(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base());
	return cost;
}

/**
 * Remove a buoy
 * @param tile TileIndex been queried
 * @param flags operation to perform
 * @pre IsBuoyTile(tile)
 * @return cost or failure of operation
 */

/**
 * Check whether the name is unique amongst the waypoints.
 * @param name The name to check.
 * @return True iff the name is unique.
 */

/**
 * Rename a waypoint.
 * @param flags type of operation
 * @param waypoint_id id of waypoint
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameWaypoint(DoCommandFlags flags, StationID waypoint_id, const std::string &text)
{
	CommandCost cost;
	openttd_rust_waypoint_rename(&GetRustStationCommandServices(), RustCommandCost(&cost), flags.base(), waypoint_id.base(), reinterpret_cast<const uint8_t *>(text.data()), text.size());
	return cost;
}

/**
 * Move a waypoint name.
 * @param flags type of operation
 * @param waypoint_id id of waypoint
 * @param tile to move the waypoint name to
 * @return the cost of this operation or an error and the waypoint ID
 */
std::tuple<CommandCost, StationID> CmdMoveWaypointName(DoCommandFlags flags, StationID waypoint_id, TileIndex tile)
{
	CommandCost cost;
	auto id = openttd_rust_waypoint_move_name(&GetRustStationCommandServices(), RustCommandCost(&cost), flags.base(), waypoint_id.base(), tile.base());
	return {std::move(cost), StationID(id)};
}

/**
 * Callback function that is called after a name is moved
 * @param result of the operation
 * @param waypoint_id ID of the changed waypoint
 */
void CcMoveWaypointName(Commands, const CommandCost &result, StationID waypoint_id)
{
	if (result.Failed()) return;

	ResetObjectToPlace();
	Waypoint *wp = Waypoint::Get(waypoint_id);
	SetViewportCatchmentWaypoint(wp, false);
}
