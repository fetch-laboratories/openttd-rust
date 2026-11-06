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


static constinit const OpenTTDRoadStopServices _rust_road_stop_services{
	.vehicle = &_rust_vehicle_services,
	.ground = &_rust_ground_services<RoadVehicle>,
	.is_bus = RustVehicleRead<bool, RustRoadIsBus>,
	.pool = []() noexcept { return _roadstop_pool.descriptor; },
};

const OpenTTDRoadStopServices &GetRustRoadStopServices() noexcept
{
	return _rust_road_stop_services;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRoadStopServices.hpp"
bool CheckAbi_roadstop() { return AbiServicesComplete(GetRustRoadStopServices()); }
