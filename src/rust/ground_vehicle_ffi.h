struct OpenTTDEngineCapabilitiesServices;
/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_vehicle_ffi.h Typed ground dynamics leaves; owners and borrows stay call scoped. */
#ifndef RUST_GROUND_VEHICLE_FFI_H
#define RUST_GROUND_VEHICLE_FFI_H
#include "vehicle_ffi.h"
#include "train_ffi.h"
#include "../engine_type.h"
struct Vehicle;
struct OpenTTDRoadLeaves;
struct OpenTTDGroundPhysicsServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDRoadLeaves *road;
	bool assertions;
	const OpenTTDEngineCapabilitiesServices *capabilities;
	uint64_t (*cargo_weight)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_weight);
	uint64_t (*train_cargo_weight)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(train_cargo_weight);
	bool (*powered)(uint64_t, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered);
	RailType (*rail_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type);
	VehicleAccelerationModel (*rail_acceleration)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_acceleration);
	uint16_t (*rail_speed)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_speed);
	uint32_t (*steepness)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(steepness);
	void (*viewport)(OpenTTDVehicle *, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	uint32_t (*base_crash)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(base_crash);
};
const OpenTTDGroundPhysicsServices &GetRustGroundPhysicsServices() noexcept;
extern "C" {
void openttd_rust_ground_power_changed(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *);
void openttd_rust_ground_cargo_changed(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *);
int32_t openttd_rust_ground_acceleration(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *);
uint8_t openttd_rust_ground_chain_in_depot_abi(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *);
extern "C++" inline bool openttd_rust_ground_chain_in_depot(const OpenTTDGroundPhysicsServices *a0, const OpenTTDVehicle *a1) { return openttd_rust_ground_chain_in_depot_abi(a0, a1) != 0; }
uint32_t openttd_rust_ground_crash(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *, bool);
uint16_t openttd_rust_ground_max_weight(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *);
VehicleAccelerationModel openttd_rust_ground_acceleration_type(const OpenTTDGroundPhysicsServices *, const OpenTTDVehicle *);
}
#endif /* RUST_GROUND_VEHICLE_FFI_H */
