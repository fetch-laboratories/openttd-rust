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
struct CargoSpec;
struct OpenTTDGroundState;
struct OpenTTDRoadLeaves;
struct OpenTTDRoadState;
struct OpenTTDGroundOwner { OpenTTDGroundState *state; OpenTTDTrainHandle train; OpenTTDRoadState *road; VehicleType kind; };
struct OpenTTDGroundPowerInput { uint32_t tile; uint8_t subtype; };
struct OpenTTDGroundWeightInput { const CargoSpec *cargo; OpenTTDCargoList *owner; };
struct OpenTTDGroundAccelerationInput { uint32_t tile; uint16_t speed; uint8_t status; RoadType roadtype; };
struct OpenTTDGroundSpeedInput { uint16_t speed; uint8_t subspeed; };
struct OpenTTDGroundPhysicsServices {
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDRoadLeaves *road;
	bool assertions;
	OpenTTDGroundOwner (*resolve)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	OpenTTDVehicle *(*first)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first);
	OpenTTDGroundPowerInput (*power_input)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(power_input);
	OpenTTDGroundWeightInput (*weight_input)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(weight_input);
	OpenTTDGroundAccelerationInput (*acceleration_input)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(acceleration_input);
	OpenTTDGroundSpeedInput (*speed_input)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(speed_input);
	uint32_t (*capacity)(const OpenTTDEngine *, OpenTTDVehicle *, uint16_t *) noexcept OPENTTD_SERVICE_DEFAULT(capacity);
	uint64_t (*cargo_weight)(const CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_weight);
	uint64_t (*train_cargo_weight)(const CargoSpec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(train_cargo_weight);
	uint16_t (*engine_power)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_power);
	uint16_t (*engine_weight)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_weight);
	uint8_t (*engine_te)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_te);
	uint8_t (*engine_drag)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_drag);
	uint16_t (*wagon_power)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(wagon_power);
	uint8_t (*wagon_weight)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(wagon_weight);
	bool (*powered)(uint64_t, RailType) noexcept OPENTTD_SERVICE_DEFAULT(powered);
	RailType (*rail_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_type);
	VehicleAccelerationModel (*rail_acceleration)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_acceleration);
	uint16_t (*rail_speed)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_speed);
	uint16_t (*road_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_speed);
	bool (*tram)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(tram);
	uint32_t (*steepness)(VehicleType) noexcept OPENTTD_SERVICE_DEFAULT(steepness);
	int32_t (*display_max_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(display_max_speed);
	void (*viewport)(OpenTTDVehicle *, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	void (*details_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(details_dirty);
	uint32_t (*base_crash)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(base_crash);
	bool (*depot_tile)(uint32_t, TransportType) noexcept OPENTTD_SERVICE_DEFAULT(depot_tile);
};
const OpenTTDGroundPhysicsServices &GetRustGroundPhysicsServices() noexcept;
extern "C" {
void openttd_rust_ground_power_changed(const OpenTTDGroundPhysicsServices *, const Vehicle *);
void openttd_rust_ground_cargo_changed(const OpenTTDGroundPhysicsServices *, const Vehicle *);
int32_t openttd_rust_ground_acceleration(const OpenTTDGroundPhysicsServices *, const Vehicle *);
bool openttd_rust_ground_chain_in_depot(const OpenTTDGroundPhysicsServices *, const Vehicle *);
uint32_t openttd_rust_ground_crash(const OpenTTDGroundPhysicsServices *, const Vehicle *, bool);
uint16_t openttd_rust_ground_weight(const OpenTTDGroundPhysicsServices *, const Vehicle *, bool);
VehicleAccelerationModel openttd_rust_ground_acceleration_type(const OpenTTDGroundPhysicsServices *, const Vehicle *);
}
#endif /* RUST_GROUND_VEHICLE_FFI_H */
