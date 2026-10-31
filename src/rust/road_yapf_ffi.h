/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_yapf_ffi.h Complete road search and canonical road-cache construction. */
#ifndef RUST_ROAD_YAPF_FFI_H
#define RUST_ROAD_YAPF_FFI_H
#include "service_defaults.hpp"
#include "road_ffi.h"
extern "C" {
struct OpenTTDRoadYapfInput {
	uint32_t map_x, map_y, tile, dest_tile;
	int32_t max_nodes, slope, crossing, stop, occupied, bay, curve;
	uint16_t order_destination;
	uint8_t order_type, bus, articulated, trackdir;
};
struct OpenTTDRoadYapfTile {
	uint16_t station;
	uint8_t type, station_type, depot, depot_dir, crossing, waypoint, drive_through;
};
struct OpenTTDRoadYapfLeaves {
	const OpenTTDSharedServices *shared; ///< Shared world and movement owner.
	OpenTTDRoadYapfTile (OPENTTD_RUST_CALL *tile)(const void *, uint32_t, uint8_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	uint32_t (OPENTTD_RUST_CALL *closest)(const void *, uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(closest);
};
struct OpenTTDRoadYapfResult { uint32_t tile; int32_t cost; uint8_t direction, found; int32_t rounds, open, closed, calcs, distance; };
/* Input is copied, and the opaque vehicle context stays live for the call. Leaves use only shared
 * map, pool, settings and follower services and cannot reenter the search or road owner. The search
 * owns its arena, lookup maps, exact-order heap and reconstruction, and writes the road vehicle's
 * path directly. Tile costs, including road-stop occupancy, are read only in calc_cost; traversal
 * reads ground-vehicle center Z and speed, then CFollowTrackRoad limits for old_tile. */
OpenTTDRoadYapfResult OPENTTD_RUST_CALL openttd_rust_road_yapf_choose(OpenTTDRoadState *, const OpenTTDRoadYapfInput *, const OpenTTDRoadYapfLeaves *, const void *, uint32_t, uint8_t, uint16_t, uint8_t);
OpenTTDRoadYapfResult OPENTTD_RUST_CALL openttd_rust_road_yapf_depot(const OpenTTDRoadYapfInput *, const OpenTTDRoadYapfLeaves *, const void *, int32_t);
/* Evidence-only actual-heap probe. Input/Leaves initialized; no leaf is invoked.
 * costs[nodes], commands[count], values[count] readable and output[count] writable
 * disjoint spans. Indices/membership satisfy the original heap preconditions. */
void OPENTTD_RUST_CALL openttd_rust_road_yapf_heap_probe(const OpenTTDRoadYapfInput *, const OpenTTDRoadYapfLeaves *, const int32_t *, size_t, const uint32_t *, const int32_t *, size_t, uint32_t *);
}
#endif
