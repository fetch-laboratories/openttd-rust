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
#include "services_map.h"
#include "vehicle_services.h"
#include "services_road.h"
#include "services_rail.h"
#include "../train.h"
#include "../ship.h"
#include "../window_func.h"
#include "../newgrf_sound.h"
static_assert(VEH_TRAIN == 0 && VEH_ROAD == 1 && VEH_SHIP == 2);
static_assert(GVF_GOINGUP_BIT == 0 && GVF_GOINGDOWN_BIT == 1);
static_assert(VSE_TUNNEL == 2 && SND_05_TRAIN_THROUGH_TUNNEL == 3);
static_assert(static_cast<uint8_t>(VehState::TrainSlowing) == 4);
static_assert(static_cast<uint8_t>(VehicleEnterTileState::EnteredStation) == 0 &&
	static_cast<uint8_t>(VehicleEnterTileState::EnteredWormhole) == 1 &&
	static_cast<uint8_t>(VehicleEnterTileState::CannotEnter) == 2);
static_assert(sizeof(decltype(TileIndex{}.base())) == sizeof(uint32_t) && sizeof(Track) == sizeof(uint8_t));
inline OpenTTDNormalRoadStatusTail RustNormalRoadStatusTail(uint32_t tile) noexcept
{
	return {.disallowed = Shared_disallowed_road_directions(tile), .road_works = SharedHasRoadWorks(tile)};
}
inline OpenTTDMovementVehicle RustMovementResolve(OpenTTDVehicle *handle) noexcept
{
	Vehicle *v = VehicleOf(handle);
	return {.train = v->type == VEH_TRAIN ? TrainHandle(Train::From(v)) : OpenTTDTrainHandle{},
		.road = v->type == VEH_ROAD ? RoadStateOf(RoadVehicle::From(v)) : nullptr,
		.ship = v->type == VEH_SHIP ? Ship::From(v)->GetRustState() : nullptr, .kind = v->type};
}
inline RoadTramType RustMovementRoadKind(OpenTTDVehicle *handle) noexcept { return RustRoadTramType(RustRoadVehicleType(RoadVehicle::From(VehicleOf(handle)))); }
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
inline void RustMovementDepotInvalidate(OpenTTDVehicle *handle) noexcept { RustVehicleInvalidateDepot(VehicleOf(handle)); }
#endif
