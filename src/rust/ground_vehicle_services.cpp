/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_vehicle_services.cpp Native property, storage and shared-world leaves. */
#include "../stdafx.h"
#include "../train.h"
#include "../roadveh.h"
#include "vehicle_services.h"
#include "services_cargo.h"
#include "engine_services.h"
#include "services_rail.h"
#include "services_map.h"
#include "road_ffi.h"
#include "ground_vehicle_ffi.h"
#include "../safeguards.h"

static uint64_t GroundCargoWeight(uint8_t cargo, uint32_t count) noexcept { return CargoSpec::Get(cargo)->WeightOfNUnits(count); }
static uint64_t GroundTrainCargoWeight(uint8_t cargo, uint32_t count) noexcept { return CargoSpec::Get(cargo)->WeightOfNUnitsInTrain(count); }
static uint32_t GroundSteepness(VehicleType type) noexcept { return type == VEH_TRAIN ? _settings_game.vehicle.train_slope_steepness : _settings_game.vehicle.roadveh_slope_steepness; }
static void GroundViewport(OpenTTDVehicle *shell, bool force, bool delta) noexcept { Vehicle *v = VehicleOf(shell); if (v->GetType() == VEH_TRAIN) RustVehicleUpdateViewport(Train::From(v), force, delta); else RustVehicleUpdateViewport(RoadVehicle::From(v), force, delta); }

const OpenTTDGroundPhysicsServices &GetRustGroundPhysicsServices() noexcept
{
	static const OpenTTDGroundPhysicsServices services{
		.vehicle = &GetRustVehicleServices(),
		.road = &GetRustRoadLeaves(),
#if !defined(NDEBUG) || defined(WITH_ASSERT)
		.assertions = true,
#else
		.assertions = false,
#endif
		.capabilities = &GetRustEngineCapabilitiesServices(),
		.cargo_weight = GroundCargoWeight,
		.train_cargo_weight = GroundTrainCargoWeight,
		.powered = RustRailPoweredMask,
		.rail_type = RustRailType,
		.rail_acceleration = RustRailAccelerationType,
		.rail_speed = RustRailMaxSpeed,
		.steepness = GroundSteepness,
		.viewport = GroundViewport,
		.base_crash = RustVehicleBaseCrash,
	};
	return services;
}
static_assert(TRACK_BIT_X == 1 && TRACK_BIT_Y == 2 && TRACK_BIT_WORMHOLE == 0x40 && TRACK_BIT_DEPOT == 0x80 && TRACK_BIT_MASK == 0x3f && TRACKDIR_BIT_MASK == 0x3f3f);
static_assert(GROUND_ACCELERATION == 9800 && GVF_GOINGUP_BIT == 0 && GVF_GOINGDOWN_BIT == 1 && GVF_SUPPRESS_IMPLICIT_ORDERS == 2);
static_assert(PROP_TRAIN_POWER == 0x0B && PROP_TRAIN_WEIGHT == 0x16 && PROP_TRAIN_TRACTIVE_EFFORT == 0x1F);
static_assert(PROP_ROADVEH_POWER == 0x13 && PROP_ROADVEH_WEIGHT == 0x14 && PROP_ROADVEH_TRACTIVE_EFFORT == 0x18);
static_assert(to_underlying(VehicleAccelerationModel::Normal) == 0 && to_underlying(VehicleAccelerationModel::Maglev) == 2);
static_assert(to_underlying(VehicleRailFlag::PoweredWagon) == 3 && to_underlying(VehicleRailFlag::Reversing) == 0 && to_underlying(VehicleRailFlag::Stuck) == 8);
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDGroundPhysicsServices.hpp"
bool CheckAbi_ground_physics() { return AbiServicesComplete(GetRustGroundPhysicsServices()); }
