/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_movement.h Typed native movement observations and leaves. */
#ifndef RUST_SERVICES_MOVEMENT_H
#define RUST_SERVICES_MOVEMENT_H
#include "movement_ffi.h"
#include "vehicle_services.h"
#include "services_road.h"
#include "services_rail.h"
#include "../train.h"
#include "../ship.h"
#include "../window_func.h"
inline OpenTTDMovementVehicle RustMovementResolve(OpenTTDVehicle *handle) noexcept
{
	Vehicle *v = VehicleOf(handle);
	return {.train = v->type == VEH_TRAIN ? TrainHandle(Train::From(v)) : OpenTTDTrainHandle{},
		.road = v->type == VEH_ROAD ? RoadStateOf(RoadVehicle::From(v)) : nullptr,
		.ship = v->type == VEH_SHIP ? Ship::From(v)->GetRustState() : nullptr, .kind = v->type};
}
inline RoadTramType RustMovementRoadKind(OpenTTDVehicle *handle) noexcept { return RustRoadTramType(RoadVehicle::From(VehicleOf(handle))->roadtype); }
inline OpenTTDMovementPose RustMovementPose(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.z = RustVehicleZ(v), .direction = RustVehicleDirection(v)};
}
inline OpenTTDMovementDepotPosition RustMovementDepotPosition(OpenTTDTrainHandle handle) noexcept
{
	const Train *v = static_cast<const Train *>(handle.shell);
	return {.tile = RustVehicleTile(v), .x = RustVehicleX(v), .y = RustVehicleY(v)};
}
inline OpenTTDMovementIncline RustMovementIncline(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.z = RustVehicleZ(v), .flags = v->GetGroundVehicleFlags()};
}
inline void RustMovementWriteGroundFlags(OpenTTDVehicle *handle, uint16_t flags) noexcept { VehicleOf(handle)->GetGroundVehicleFlags() = flags; }
inline OpenTTDVehicle *RustMovementFirst(OpenTTDVehicle *handle) noexcept { return RustVehicle(RustVehicleFirst(VehicleOf(handle))); }
inline OpenTTDMovementFirstSpeed RustMovementFirstSpeed(OpenTTDVehicle *handle) noexcept
{
	Vehicle *first = RustVehicleFirst(VehicleOf(handle));
	return {.first = RustVehicle(first), .speed = RustVehicleSpeed(first)};
}
inline uint8_t RustMovementTrainEngineClass(OpenTTDVehicle *handle) noexcept { return RailVehInfo(VehicleOf(handle)->engine_type)->engclass; }
inline void RustMovementDepotInvalidate(OpenTTDVehicle *handle) noexcept { RustInvalidateDepotWindow(VehicleOf(handle)->tile); }
inline void RustMovementAssert(bool condition) noexcept { assert(condition); }
#endif
