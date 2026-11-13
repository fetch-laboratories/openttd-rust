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
	uint32_t tile, dest_tile;
	int32_t max_nodes, slope, crossing, stop, occupied, bay, curve;
	uint16_t order_destination;
	uint8_t order_type, bus, articulated, trackdir;
};
struct OpenTTDRoadYapfResult { uint32_t tile; int32_t cost; uint8_t direction, found; int32_t rounds, open, closed, calcs, distance; };
/* Input is copied, and the canonical vehicle record stays live for the call. The search reads the
 * map in place and calls only shared services that cannot reenter it or the road owner, and writes
 * the road vehicle's path directly. */
OpenTTDRoadYapfResult OPENTTD_RUST_CALL openttd_rust_road_yapf_choose(OpenTTDRoadState *, const OpenTTDRoadYapfInput *, const OpenTTDSharedServices *, const OpenTTDVehicle *, uint32_t, uint8_t, uint16_t, uint8_t);
OpenTTDRoadYapfResult OPENTTD_RUST_CALL openttd_rust_road_yapf_depot(const OpenTTDRoadYapfInput *, const OpenTTDSharedServices *, const OpenTTDVehicle *, int32_t);

}
#endif
