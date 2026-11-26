/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_ffi.h Shared typed vehicle, ground-vehicle and map services for Rust ports. */
#ifndef RUST_VEHICLE_FFI_H
#define RUST_VEHICLE_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
enum VehicleType : uint8_t;
enum Direction : uint8_t;
#include "call_ffi.h"
#include "pool_ffi.h"
/** Typed vehicle handles identify shared records; legacy void* contexts identify
 * native shells. Handles stay live for the synchronous call; a service may destroy
 * them. No owner reference spans a service that mutates or reenters Rust. */
struct OpenTTDTrainState;
struct OpenTTDVehicle;
struct OpenTTDTrainHandle { OpenTTDVehicle *record; OpenTTDTrainState *owner; };
struct OpenTTDVehicleOnTile { uint32_t tile; OpenTTDVehicle *current; };
struct OpenTTDVehicleNear {
	int32_t left, top, right, bottom;
	uint32_t hxmin, hxmax, hymin, hymax, hx, hy;
	OpenTTDVehicle *current_veh;
};
/* The vehicle tables C++ fills for Rust are defined in vehicle_services_table.h and
 * vehicle_scheduler_table.h, which only the C++ that fills or calls them includes. */
struct OpenTTDVehicleServices;
struct OpenTTDGroundServices;
struct OpenTTDVehicleSchedulerServices;
struct OpenTTDVehicleSchedulerTables;
extern const OpenTTDVehicleServices _rust_vehicle_services;
/** The one immutable process-wide GroundVehicle table, for road vehicles. */
extern const OpenTTDGroundServices _rust_ground_services;
const OpenTTDVehicleServices &GetRustVehicleServices() noexcept;

const OpenTTDVehicleSchedulerServices &GetRustVehicleSchedulerServices() noexcept;
struct OpenTTDCommandServices;
struct OpenTTDCommandCost;
struct OpenTTDSharedServices;
extern "C" {
void openttd_rust_occupancy_ground(const OpenTTDCommandServices *, OpenTTDCommandCost *, uint32_t);
void openttd_rust_occupancy_wormhole(const OpenTTDCommandServices *, OpenTTDCommandCost *, uint32_t, uint32_t, const OpenTTDVehicle *);
void openttd_rust_occupancy_update(const OpenTTDSharedServices *, OpenTTDVehicle *, bool);
void openttd_rust_occupancy_reset(const OpenTTDVehicleSchedulerServices *);
OpenTTDVehicleOnTile openttd_rust_occupancy_on_tile(const OpenTTDSharedServices *, uint32_t);
void openttd_rust_occupancy_on_advance(OpenTTDVehicleOnTile *);
OpenTTDVehicleNear openttd_rust_occupancy_near(int32_t, int32_t, uint32_t);
void openttd_rust_occupancy_near_advance(OpenTTDVehicleNear *);
}
#endif /* RUST_VEHICLE_FFI_H */
