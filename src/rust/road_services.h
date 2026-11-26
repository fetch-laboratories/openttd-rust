/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_services.h The road vehicle leaves and the original values Rust names. */

#include "road_command_services.h"
#include "services_road.h"

static_assert(RVS_USING_SECOND_BAY == 1 && RVS_ENTERED_STOP == 2 && RVS_DRIVE_SIDE == 4 && RVS_IN_ROAD_STOP == 5 && RVS_IN_DT_ROAD_STOP == 6);
static_assert(RVSB_IN_DEPOT == 254 && RVSB_WORMHOLE == 255 && RVSB_IN_ROAD_STOP == 32 && RVSB_IN_ROAD_STOP_END == 48 && RVSB_IN_DT_ROAD_STOP == 64 && RVSB_IN_DT_ROAD_STOP_END == 80);
static_assert(RVSB_DRIVE_SIDE == 16 && RVSB_TRACKDIR_MASK == 0x0F && RVSB_ROAD_STOP_TRACKDIR_MASK == 0x09);
static_assert(RDE_NEXT_TILE == 0x80 && RDE_TURNED == 0x40 && RV_OVERTAKE_TIMEOUT == 35);
static_assert(RVC_DEFAULT_START_FRAME == 0 && RVC_TURN_AROUND_START_FRAME == 1 && RVC_DEPOT_START_FRAME == 6 && RVC_START_FRAME_AFTER_LONG_TRAM == 21);
static_assert(RVC_TURN_AROUND_START_FRAME_SHORT_TRAM == 16 && RVC_DRIVE_THROUGH_STOP_FRAME == 11 && RVC_DEPOT_STOP_FRAME == 11);
static_assert(to_underlying(RoadStopType::Bus) == 0 && to_underlying(RoadStopType::Truck) == 1);
static_assert(PROP_ROADVEH_SHORTEN_FACTOR == 0x23 && PROP_ROADVEH_CARGO_AGE_PERIOD == 0x22 && PROP_ROADVEH_SPEED == 0x15 && PROP_ROADVEH_RUNNING_COST_FACTOR == 0x09);

/** Construct the road command context `CanBuildTramTrackOnTile` runs `CmdBuildRoad` in, in the controller's slot. */
static const OpenTTDRoadCommandContext *OPENTTD_RUST_CALL RoadSvcBuildRoadContext(OpenTTDRoadCommandSlot *slot) noexcept
{
	if (slot->built) slot->road.~RustRoadCommandContext();
	slot->built = true;
	return &std::construct_at(&slot->road)->view;
}

static constinit const OpenTTDRoadLeaves _rust_road_leaves{
	.build_road_context = RoadSvcBuildRoadContext,
	.ground = &_rust_ground_services,
	.shared = &_rust_shared_services,
	.vehicle = &_rust_vehicle_services,
};

static const OpenTTDRoadLeaves &GetRoadLeaves()
{
	return _rust_road_leaves;
}
