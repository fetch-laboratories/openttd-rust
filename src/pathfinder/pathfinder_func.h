/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file pathfinder_func.h General functions related to pathfinders. */

#ifndef PATHFINDER_FUNC_H
#define PATHFINDER_FUNC_H

#include "../rust/movement_ffi.h"

#include "../tile_cmd.h"
#include "../waypoint_base.h"

/**
 * Wrapper around GetTileTrackStatus() and TrackStatusToTrackdirBits(), as for
 * single tram bits GetTileTrackStatus() returns 0. The reason for this is
 * that there are no half-tile TrackBits in OpenTTD.
 * This tile, however, is a valid tile for trams, one on which they can
 * reverse safely. To "fix" this, pretend that if we are on a half-tile, we
 * are in fact on a straight tram track tile. CFollowTrackT will make sure
 * the pathfinders cannot exit on the wrong side and allows reversing on such
 * tiles.
 */
inline TrackdirBits GetTrackdirBitsForRoad(TileIndex tile, RoadTramType rtt)
{
	static const OpenTTDMovementServices * const movement = &GetRustMovementServices();
	return static_cast<TrackdirBits>(openttd_rust_road_trackdirs(movement, tile.base(), rtt));
}

#endif /* PATHFINDER_FUNC_H */
