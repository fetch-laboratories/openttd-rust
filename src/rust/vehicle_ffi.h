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
enum VehicleSoundEvent : uint8_t;
enum Direction : uint8_t;
enum class GRFBug : uint8_t;
#include "call_ffi.h"
#include "pool_ffi.h"
/** Typed vehicle handles identify shared records; legacy void* contexts identify
 * native shells. Handles stay live for the synchronous call; a service may destroy
 * them. No owner reference spans a service that mutates or reenters Rust. */
struct OpenTTDTrainState;
struct OpenTTDVehicle;
struct OpenTTDTrainHandle { OpenTTDVehicle *record; OpenTTDTrainState *owner; };
struct OpenTTDOrdersLeaves;
struct OpenTTDVehicleOnTile { uint32_t tile; OpenTTDVehicle *current; };
struct OpenTTDVehicleNear {
	int32_t left, top, right, bottom;
	uint32_t hxmin, hxmax, hymin, hymax, hx, hy;
	OpenTTDVehicle *current_veh;
};
struct OpenTTDEngine;
struct OpenTTDVehicleMaintenanceServices;
struct OpenTTDVehicleResolverServices;
struct OpenTTDVehicleVisits;
struct OpenTTDVehicleServices {
	void (OPENTTD_RUST_CALL *base_viewport)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(base_viewport);
	bool (*can_allocate)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	void (OPENTTD_RUST_CALL *destroy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	const OpenTTDEngine *(OPENTTD_RUST_CALL *engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	uint32_t (*first_sprite)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_sprite);
	void (OPENTTD_RUST_CALL *length_changed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(length_changed);
	const OpenTTDVehicleMaintenanceServices *maintenance;
	const OpenTTDVehicleResolverServices *(*newgrf)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf);
	const OpenTTDOrdersLeaves *(*orders)() noexcept OPENTTD_SERVICE_DEFAULT(orders);
	void (OPENTTD_RUST_CALL *pathfinding_result)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(pathfinding_result);
	void (*play_leave_station_sound)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(play_leave_station_sound);
	void (OPENTTD_RUST_CALL *play_sound)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(play_sound);
	bool (OPENTTD_RUST_CALL *play_vehicle_sound)(OpenTTDVehicle *, VehicleSoundEvent) noexcept OPENTTD_SERVICE_DEFAULT(play_vehicle_sound);
	int64_t (OPENTTD_RUST_CALL *profit)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(profit);
	OpenTTDVehicle *(*record_from_shell)(void *) noexcept OPENTTD_SERVICE_DEFAULT(record_from_shell);
	void (*set_cargo_subtype)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_subtype);
	void (*set_cargo_type)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_type);
	void (*set_colourmap)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_colourmap);
	void (*set_max_age)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_max_age);
	OpenTTDVehicle *(*ship_get)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_get);
	void (*show_newgrf_vehicle_error)(uint16_t, uint32_t, uint32_t, GRFBug, bool) noexcept OPENTTD_SERVICE_DEFAULT(show_newgrf_vehicle_error);
	void (OPENTTD_RUST_CALL *start_stop_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(start_stop_dirty);
	void (*update_delta)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(update_delta);
	const OpenTTDVehicleVisits *(*visits)() noexcept OPENTTD_SERVICE_DEFAULT(visits);
	void (OPENTTD_RUST_CALL *write_breakdowns)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_breakdowns);
	void (OPENTTD_RUST_CALL *write_calendar_service)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_calendar_service);
	void (OPENTTD_RUST_CALL *write_economy_service)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_economy_service);
	void (OPENTTD_RUST_CALL *write_first_sprite)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_first_sprite);
	void (OPENTTD_RUST_CALL *write_profit)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(write_profit);
	void (OPENTTD_RUST_CALL *write_reliability)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_reliability);
	void (OPENTTD_RUST_CALL *write_sprite_seq)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_sprite_seq);
	void (OPENTTD_RUST_CALL *write_spritenum)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_spritenum);
};
struct OpenTTDGroundPhysicsServices;
struct OpenTTDGroundServices {
	const OpenTTDGroundPhysicsServices *physics;
};
extern const OpenTTDVehicleServices _rust_vehicle_services;
const OpenTTDVehicleServices &GetRustVehicleServices() noexcept;

struct Vehicle;
struct OpenTTDCargoStorageServices;
struct OpenTTDSharedServices;
struct OpenTTDStationServices;
struct OpenTTDStationLoading;
struct OpenTTDFleetPendingServices;
struct OpenTTDFleetTransactionServices;
struct OpenTTDFleetGroupServices;
struct OpenTTDAircraftLeaves;
struct OpenTTDRoadLeaves;
struct OpenTTDShipLeaves;
struct OpenTTDTrainServices;
struct OpenTTDVehicleSchedulerServices {
	const OpenTTDAircraftLeaves *aircraft; ///< Aircraft controller table, immutable for the process.
	const RustPoolDescriptor *const *pool; ///< The vehicle pool's descriptor.
	const OpenTTDRoadLeaves *road; ///< Road vehicle controller table, immutable for the process.
	const OpenTTDSharedServices *shared; ///< Shared table, immutable for the process.
	const OpenTTDShipLeaves *ship; ///< Ship controller table, immutable for the process.
	/** Runs the virtual Tick; true when the vehicle survives. */
	bool (OPENTTD_RUST_CALL *tick)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tick);
	const OpenTTDTrainServices *train; ///< Train controller table, immutable for the process.
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
};
struct OpenTTDCommandCost;
struct OpenTTDVehicleSchedulerTables {
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDFleetGroupServices *group;
	const OpenTTDStationLoading *loading;
	const OpenTTDFleetPendingServices *pending;
	OpenTTDCommandCost *refit_cost;
	OpenTTDCommandCost *refit_temporary;
	const OpenTTDVehicleSchedulerServices *scheduler;
	const OpenTTDStationServices *station;
	const OpenTTDFleetTransactionServices *transaction;
	const OpenTTDVehicleVisits *visits;
};
const OpenTTDVehicleSchedulerServices &GetRustVehicleSchedulerServices() noexcept;
struct OpenTTDCommandServices;
struct OpenTTDCommandCost;
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
