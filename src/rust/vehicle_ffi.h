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
enum CallbackID : uint16_t;
enum VehicleSoundEvent : uint8_t;
enum Direction : uint8_t;
#include "call_ffi.h"
#include "pool_ffi.h"
/** Typed vehicle handles identify shared records; legacy void* contexts identify
 * native shells. Handles stay live for the synchronous call; a service may destroy
 * them. No owner reference spans a service that mutates or reenters Rust. */
struct OpenTTDTrainState;
struct OpenTTDVehicle;
struct OpenTTDTrainHandle { OpenTTDVehicle *record; OpenTTDTrainState *owner; };
struct OpenTTDOrdersLeaves;
/** Canonical stable NOSAVE links; only the Rust occupancy owner mutates them. */
struct OpenTTDVehicleTileLinks {
	OpenTTDVehicle *next = nullptr;
	OpenTTDVehicle **prev = nullptr;
	OpenTTDVehicle **current = nullptr;
};
struct OpenTTDVehicleOnTile { uint32_t tile; OpenTTDVehicle *current; };
struct OpenTTDVehicleNear {
	int32_t left, top, right, bottom;
	uint32_t hxmin, hxmax, hymin, hymax, hx, hy;
	OpenTTDVehicle *current;
};
struct OpenTTDEngine;
struct OpenTTDVehicleNewPosition {
	int32_t x;
	int32_t y;
	uint32_t tile;
};
struct OpenTTDVehicleMaintenanceServices;
struct OpenTTDVehicleResolverServices;
struct OpenTTDVehicleVisits;
struct OpenTTDVehicleServices {
	void (*assert_next)(OpenTTDVehicle *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(assert_next);
	OpenTTDVehicle *(*record_from_shell)(void *) noexcept OPENTTD_SERVICE_DEFAULT(record_from_shell);
	OpenTTDVehicleTileLinks *(OPENTTD_RUST_CALL *tile_links)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tile_links);
	uint32_t (*map_log_x)() noexcept OPENTTD_SERVICE_DEFAULT(map_log_x);
	bool assertions;
	OpenTTDTrainHandle (*train_handle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(train_handle);
	bool (OPENTTD_RUST_CALL *waiting_for_unbunching)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(waiting_for_unbunching);
	void (OPENTTD_RUST_CALL *leave_unbunching_depot)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(leave_unbunching_depot);
	void (OPENTTD_RUST_CALL *reset_depot_unbunching)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reset_depot_unbunching);
	void (*enter_depot)(OpenTTDVehicle *, const OpenTTDVehicleVisits *) noexcept OPENTTD_SERVICE_DEFAULT(enter_depot);
	void (OPENTTD_RUST_CALL *pathfinding_result)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(pathfinding_result);
	OpenTTDVehicleNewPosition (OPENTTD_RUST_CALL *new_position)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(new_position);
	void (OPENTTD_RUST_CALL *base_viewport)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(base_viewport);
	const OpenTTDVehicleResolverServices *(*newgrf)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf);
	int32_t (OPENTTD_RUST_CALL *property)(const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, uint8_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(property);
	uint16_t (OPENTTD_RUST_CALL *callback)(const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, CallbackID) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	void (*set_max_age)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_max_age);
	void (OPENTTD_RUST_CALL *length_changed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(length_changed);
	bool (OPENTTD_RUST_CALL *play_vehicle_sound)(OpenTTDVehicle *, VehicleSoundEvent) noexcept OPENTTD_SERVICE_DEFAULT(play_vehicle_sound);
	void (OPENTTD_RUST_CALL *play_sound)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(play_sound);
	void (OPENTTD_RUST_CALL *order_free)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_free);
	void (OPENTTD_RUST_CALL *order_dummy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_dummy);
	void (OPENTTD_RUST_CALL *start_stop_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(start_stop_dirty);
	void (OPENTTD_RUST_CALL *destroy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	void (OPENTTD_RUST_CALL *set_dest_tile)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_dest_tile);
	const OpenTTDEngine *(OPENTTD_RUST_CALL *engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	uint8_t (OPENTTD_RUST_CALL *order_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_type);
	uint16_t (OPENTTD_RUST_CALL *order_destination)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_destination);
	uint8_t (OPENTTD_RUST_CALL *order_nonstop)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_nonstop);
	void (OPENTTD_RUST_CALL *write_profit)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(write_profit);
	void (OPENTTD_RUST_CALL *write_economy_service)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_economy_service);
	void (OPENTTD_RUST_CALL *write_calendar_service)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_calendar_service);
	void (OPENTTD_RUST_CALL *write_breakdowns)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_breakdowns);
	void (OPENTTD_RUST_CALL *write_reliability)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_reliability);
	int64_t (OPENTTD_RUST_CALL *profit)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(profit);
	void (OPENTTD_RUST_CALL *make_dummy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(make_dummy);
	void (OPENTTD_RUST_CALL *check_orders_only)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(check_orders_only);
	const OpenTTDVehicleMaintenanceServices *maintenance;
	const OpenTTDVehicleVisits *(*visits)() noexcept OPENTTD_SERVICE_DEFAULT(visits);
	const OpenTTDOrdersLeaves *(*orders)() noexcept OPENTTD_SERVICE_DEFAULT(orders);
	OpenTTDVehicle *(*ship_get)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_get);
	uint32_t (*first_sprite)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_sprite);
	void (*update_delta)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(update_delta);
	bool (*can_allocate)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
};
struct OpenTTDGroundPhysicsServices;
struct OpenTTDGroundServices {
	const OpenTTDGroundPhysicsServices *physics;
	void (OPENTTD_RUST_CALL *update_viewport)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(update_viewport);
	bool (OPENTTD_RUST_CALL *has_articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated);
};
extern const OpenTTDVehicleServices _rust_vehicle_services;
const OpenTTDVehicleServices &GetRustVehicleServices() noexcept;

struct Vehicle;
struct OpenTTDCargoList;
struct OpenTTDCargoStorageServices;
struct OpenTTDSharedServices;
struct OpenTTDStationServices;
struct OpenTTDStationLoading;
struct OpenTTDFleetPendingServices;
struct OpenTTDFleetTransactionServices;
struct OpenTTDFleetGroupServices;
/** Call-scoped actual indexing storage; growth invalidates slots but not first_unused. */
struct OpenTTDVehiclePoolCursor {
	const RustPoolSlot *slots;
	size_t slots_len;
	const size_t *first_unused;
};
struct OpenTTDVehicleSchedulerTables;
struct OpenTTDVehicleSchedulerServices {
	OpenTTDVehiclePoolCursor (OPENTTD_RUST_CALL *pool)() noexcept OPENTTD_SERVICE_DEFAULT(pool);
	bool assertions;
	/** Runs the virtual Tick; true when the vehicle survives. */
	bool (OPENTTD_RUST_CALL *tick)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tick);
	void (OPENTTD_RUST_CALL *calendar_day)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(calendar_day);
	void (OPENTTD_RUST_CALL *economy_day)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(economy_day);
	OpenTTDCargoList *(OPENTTD_RUST_CALL *cargo_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_owner);
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
	const OpenTTDSharedServices *shared; ///< Shared table, immutable for the process.
	void (OPENTTD_RUST_CALL *with_loading_timer)(const OpenTTDVehicleSchedulerTables *, void (OPENTTD_RUST_CALL *)(const OpenTTDVehicleSchedulerTables *)) noexcept OPENTTD_SERVICE_DEFAULT(with_loading_timer);
	void (OPENTTD_RUST_CALL *reset_accumulators)() noexcept OPENTTD_SERVICE_DEFAULT(reset_accumulators);
};
struct OpenTTDCommandCost;
struct OpenTTDVehicleSchedulerTables {
	const OpenTTDVehicleSchedulerServices *scheduler;
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDStationServices *station;
	const OpenTTDStationLoading *loading;
	const OpenTTDFleetPendingServices *pending;
	const OpenTTDFleetTransactionServices *transaction;
	const OpenTTDFleetGroupServices *group;
	const OpenTTDVehicleVisits *visits;
	OpenTTDCommandCost *refit_cost;
	OpenTTDCommandCost *refit_temporary;
};
const OpenTTDVehicleSchedulerServices &GetRustVehicleSchedulerServices() noexcept;
struct OpenTTDCommandCostServices;
struct OpenTTDCommandCost;
struct OpenTTDOccupancyCommands {
	const OpenTTDVehicleServices *vehicles;
	int32_t (*max_pixel_z)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(max_pixel_z);
	const OpenTTDCommandCostServices *costs;
	uint32_t vehicle_error;
};
const OpenTTDOccupancyCommands *GetRustOccupancyCommands() noexcept;
extern "C" {
void openttd_rust_occupancy_ground(const OpenTTDOccupancyCommands *, OpenTTDCommandCost *, uint32_t);
void openttd_rust_occupancy_wormhole(const OpenTTDOccupancyCommands *, OpenTTDCommandCost *, uint32_t, uint32_t, const OpenTTDVehicle *);
void openttd_rust_occupancy_update(const OpenTTDVehicleServices *, OpenTTDVehicle *, bool);
void openttd_rust_occupancy_reset(const OpenTTDVehicleSchedulerServices *);
OpenTTDVehicleOnTile openttd_rust_occupancy_on_tile(const OpenTTDVehicleServices *, uint32_t);
void openttd_rust_occupancy_on_advance(const OpenTTDVehicleServices *, OpenTTDVehicleOnTile *);
OpenTTDVehicleNear openttd_rust_occupancy_near(const OpenTTDVehicleServices *, int32_t, int32_t, uint32_t);
void openttd_rust_occupancy_near_advance(const OpenTTDVehicleServices *, OpenTTDVehicleNear *);
}
#endif /* RUST_VEHICLE_FFI_H */
