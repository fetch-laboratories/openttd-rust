/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file roadstop.cpp Implementation of the roadstop base class. */

#include "stdafx.h"
#include "roadveh.h"
#include "core/pool_func.hpp"
#include "core/container_func.hpp"
#include "roadstop_base.h"
#include "station_base.h"
#include "vehicle_func.h"
#include "rust/services_road.h"
#include "rust/services_station.h"
#include "rust/vehicle_services.h"

#include "safeguards.h"

/** The pool of roadstops. */
RoadStopPool _roadstop_pool("RoadStop");
INSTANTIATE_POOL_METHODS(RoadStop)


/**
 * Get the next road stop accessible by this vehicle.
 * @param v the vehicle to get the next road stop for.
 * @return the next road stop accessible.
 */
RoadStop *RoadStop::GetNextRoadStop(const RoadVehicle *v) const
{
	for (RoadStop *rs = this->next; rs != nullptr; rs = rs->next) {
		/* The vehicle cannot go to this roadstop (different roadtype) */
		if (!HasTileAnyRoadType(rs->xy, v->compatible_roadtypes)) continue;
		/* The vehicle is articulated and can therefore not go to a standard road stop. */
		if (IsBayRoadStopTile(rs->xy) && v->HasArticulatedPart()) continue;

		/* The vehicle can actually go to this road stop. So, return it! */
		return rs;
	}

	return nullptr;
}

static OpenTTDRoadStopDriveRead OPENTTD_RUST_CALL RoadStopDriveRead(OpenTTDVehicle *handle) noexcept
{
	const RoadVehicle *rv = RoadVehicle::From(VehicleOf(handle));
	return {.total_length = RustGroundCachedTotalLength(&rv->gcache), .direction = RustVehicleDirection(rv)};
}

/** The Rebuild filter reads; only road vehicles have the road fields. */
static OpenTTDRoadStopRebuildRead OPENTTD_RUST_CALL RoadStopRebuildRead(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	VehicleType type = RustVehicleVehicleType(v);
	if (type != VEH_ROAD) return {.road = nullptr, .total_length = 0, .vehicle_type = type, .direction = 0, .status = 0, .primary = false};
	const RoadVehicle *rv = RoadVehicle::From(v);
	return {.road = RoadStateOf(rv), .total_length = RustGroundCachedTotalLength(&rv->gcache), .vehicle_type = type,
		.direction = RustVehicleDirection(v), .status = RustVehicleStatus(v), .primary = RustVehiclePrimary(v)};
}

static constinit const OpenTTDRoadStopServices _rust_road_stop_services{
	.vehicle = &_rust_vehicle_services,
	.ground = &_rust_ground_services<RoadVehicle>,
	.primary = [](uint32_t tile, RoadStopType type) noexcept { return RustStationByTile(tile)->GetPrimaryRoadStop(type); },
	.head = [](BaseStation *st, bool truck) noexcept { return truck ? Station::From(st)->truck_stops : Station::From(st)->bus_stops; },
	.next = [](const RoadStop *stop) noexcept { return stop->next; },
	.tile = [](const RoadStop *stop) noexcept { return stop->xy.base(); },
	.state = [](const RoadStop *stop) noexcept { return stop->GetRustState(); },
	.pool_next = [](uint32_t from) noexcept -> OpenTTDRoadStopPoolItem {
		for (const RoadStop *rs : RoadStop::Iterate(from)) return {rs->GetRustState(), rs->xy.base(), rs->index.base()};
		return {};
	},
	.drive_read = RoadStopDriveRead,
	.rebuild_read = RoadStopRebuildRead,
};

const OpenTTDRoadStopServices &GetRustRoadStopServices() noexcept
{
	return _rust_road_stop_services;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRoadStopServices.hpp"
bool CheckAbi_roadstop() { return AbiServicesComplete(GetRustRoadStopServices()); }
