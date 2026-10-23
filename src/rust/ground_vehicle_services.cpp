/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_vehicle_services.cpp Native property, storage and shared-world leaves. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../train.h"
#include "../roadveh.h"
#include "../depot_map.h"
#include "vehicle_services.h"
#include "engine_services.h"
#include "services_rail.h"
#include "services_map.h"
#include "services_road.h"
#include "road_ffi.h"
#include "ground_vehicle_ffi.h"
#include "../safeguards.h"

static OpenTTDGroundOwner GroundResolve(OpenTTDVehicle *shell) noexcept
{
	Vehicle *v = VehicleOf(shell);
	if (v->type == VEH_TRAIN) { Train *t = Train::From(v); return {t->rust_ground.state, {t, t->GetRustState()}, nullptr, v->type}; }
	return {RoadVehicle::From(v)->rust_ground.state, {nullptr, nullptr}, RoadVehicle::From(v)->GetRustState(), v->type};
}
static OpenTTDGroundPowerInput GroundPowerInput(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return {v->type == VEH_TRAIN ? RustVehicleTile(v) : 0, RustVehicleSubtype(v)}; }
static OpenTTDGroundWeightInput GroundWeightInput(OpenTTDVehicle *shell) noexcept { Vehicle *v = VehicleOf(shell); return {CargoSpec::Get(v->cargo_type), RustVehicleCargoOwner(v)}; }
static OpenTTDGroundAccelerationInput GroundAccelerationInput(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return {v->type == VEH_TRAIN ? RustVehicleTile(v) : 0, RustVehicleSpeed(v), RustVehicleStatus(v), v->type == VEH_ROAD ? RoadVehicle::From(v)->roadtype : RoadType{0}}; }
static OpenTTDGroundSpeedInput GroundSpeedInput(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return {RustVehicleSpeed(v), RustVehicleSubspeed(v)}; }
static uint64_t GroundCargoWeight(const CargoSpec *cargo, uint32_t count) noexcept { return cargo->WeightOfNUnits(count); }
static uint64_t GroundTrainCargoWeight(const CargoSpec *cargo, uint32_t count) noexcept { return cargo->WeightOfNUnitsInTrain(count); }
static bool GroundPowered(uint64_t types, RailType type) noexcept { return RustRailPowered(RailTypes(types), type); }
static uint16_t GroundRoadSpeed(OpenTTDVehicle *shell) noexcept { const RoadVehicle *v = RoadVehicle::From(VehicleOf(shell)); return GetRoadTypeInfo(GetRoadType(v->tile, RustRoadTramType(v->roadtype)))->max_speed; }
static uint32_t GroundSteepness(VehicleType type) noexcept { return type == VEH_TRAIN ? _settings_game.vehicle.train_slope_steepness : _settings_game.vehicle.roadveh_slope_steepness; }
static void GroundViewport(OpenTTDVehicle *shell, bool force, bool delta) noexcept { Vehicle *v = VehicleOf(shell); if (v->type == VEH_TRAIN) RustVehicleViewport(Train::From(v), force, delta); else RustVehicleViewport(RoadVehicle::From(v), force, delta); }
static bool GroundDepotTile(uint32_t tile, TransportType type) noexcept { return IsDepotTypeTile(TileIndex(tile), type); }

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
		.resolve = GroundResolve,
		.first = RustGroundFirst,
		.power_input = GroundPowerInput,
		.weight_input = GroundWeightInput,
		.acceleration_input = GroundAccelerationInput,
		.speed_input = GroundSpeedInput,
		.capacity = RustEngineDetermineCapacity,
		.cargo_weight = GroundCargoWeight,
		.train_cargo_weight = GroundTrainCargoWeight,
		.engine_power = RustGroundEnginePower,
		.engine_weight = RustGroundEngineWeight,
		.engine_te = RustGroundEngineTE,
		.engine_drag = RustGroundEngineDrag,
		.wagon_power = RustEngineWagonPower,
		.wagon_weight = RustEngineWagonWeight,
		.powered = GroundPowered,
		.rail_type = RustRailType,
		.rail_acceleration = RustRailAccelerationType,
		.rail_speed = RustRailMaxSpeed,
		.road_speed = GroundRoadSpeed,
		.tram = SharedRoadTypeIsTram,
		.steepness = GroundSteepness,
		.display_max_speed = RustGroundDisplayMaxSpeed,
		.slope_z = SharedSlopePixelZ,
		.viewport = GroundViewport,
		.details_dirty = RustVehicleDetailsDirty,
		.base_crash = RustVehicleBaseCrash,
		.depot_tile = GroundDepotTile,
	};
	return services;
}
static_assert(TRACK_BIT_X == 1 && TRACK_BIT_Y == 2 && TRACK_BIT_WORMHOLE == 0x40 && TRACK_BIT_DEPOT == 0x80 && TRACK_BIT_MASK == 0x3f && TRACKDIR_BIT_MASK == 0x3f3f);
static_assert(GROUND_ACCELERATION == 9800 && GVF_GOINGUP_BIT == 0 && GVF_GOINGDOWN_BIT == 1);
static_assert(PROP_TRAIN_POWER == 0x0B && PROP_TRAIN_WEIGHT == 0x16 && PROP_TRAIN_TRACTIVE_EFFORT == 0x1F);
static_assert(PROP_ROADVEH_POWER == 0x13 && PROP_ROADVEH_WEIGHT == 0x14 && PROP_ROADVEH_TRACTIVE_EFFORT == 0x18);
static_assert(to_underlying(VehicleAccelerationModel::Normal) == 0 && to_underlying(VehicleAccelerationModel::Maglev) == 2);
static_assert(to_underlying(VehicleRailFlag::PoweredWagon) == 3 && to_underlying(VehicleRailFlag::Reversing) == 0 && to_underlying(VehicleRailFlag::Stuck) == 8);
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDGroundPhysicsServices.hpp"
bool CheckAbi_ground_physics() { return AbiServicesComplete(GetRustGroundPhysicsServices()); }
#endif /* WITH_RUST */
