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
#if defined(_MSC_VER)
#define OPENTTD_VEHICLE_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_VEHICLE_CALL __attribute__((cdecl))
#else
#define OPENTTD_VEHICLE_CALL
#endif
/** Opaque handle: a live `Vehicle *` of the port's type, valid for one synchronous call.
 * Each service is one typed noexcept function defined once; it resolves its
 * vehicle once and reads only the fields it returns. Rust holds no reference into
 * a vehicle or the map across a service call; an escaping exception terminates. */
struct OpenTTDTrainState;
struct OpenTTDTrainHandle { void *shell; OpenTTDTrainState *owner; };
struct OpenTTDVehicle;
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
/** Captured immutable engine context, valid across synchronous vehicle callbacks. */
struct OpenTTDEngine;
struct OpenTTDEngineForLength {
	const OpenTTDEngine *engine;
	uint8_t grf_version;
};
struct OpenTTDVehicleNewPosition {
	int32_t x;
	int32_t y;
	uint32_t tile;
};
struct OpenTTDVehiclePreviousTile {
	uint32_t tile;
	bool exists;
};
struct OpenTTDVehicleCoordinates {
	int32_t x;
	int32_t y;
	int32_t z;
};
struct OpenTTDVehicleXY {
	int32_t x;
	int32_t y;
};
struct OpenTTDVehicleSpeedInputs {
	uint16_t speed;
	uint16_t max_speed;
	uint8_t acceleration;
	uint8_t status;
	uint8_t subspeed;
};
struct OpenTTDVehicleDirectionProgress {
	uint8_t direction;
	uint8_t progress;
};
struct OpenTTDVehicleTickStatus {
	uint8_t tick;
	uint8_t status;
};
struct OpenTTDVehicleDayCost {
	int64_t profit;
	uint8_t running;
};
/** Read-only named scalar addresses, valid only while the synchronous caller
 * preserves this shell. No Vehicle layout or native reference is exposed. */
struct OpenTTDVehiclePositionObservations {
	const uint8_t *subtype;
	const int32_t *z;
};
struct OpenTTDVehicleMovementObservations {
	OpenTTDVehiclePositionObservations position;
	const int32_t *x;
	const int32_t *y;
	const uint16_t *speed;
	const Direction *direction;
};
struct OpenTTDVehicleAltitudeInputs { uint16_t max_speed; uint8_t direction; };
struct OpenTTDVehicleMaintenanceServices;
struct OpenTTDVehicleResolverServices;
struct OpenTTDVehicleVisits;
struct OpenTTDVehicleServices {
	OpenTTDVehicleTileLinks *(OPENTTD_VEHICLE_CALL *tile_links)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tile_links);
	uint32_t (*map_log_x)() noexcept OPENTTD_SERVICE_DEFAULT(map_log_x);
	bool assertions;
	OpenTTDTrainHandle (*train_handle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(train_handle);
	void (OPENTTD_VEHICLE_CALL *write_counters)(OpenTTDVehicle *, uint8_t, uint8_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_counters);
	bool (OPENTTD_VEHICLE_CALL *waiting_for_unbunching)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(waiting_for_unbunching);
	void (OPENTTD_VEHICLE_CALL *leave_unbunching_depot)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(leave_unbunching_depot);
	void (OPENTTD_VEHICLE_CALL *reset_depot_unbunching)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(reset_depot_unbunching);
	void (*enter_depot)(OpenTTDVehicle *, const OpenTTDVehicleVisits *) noexcept OPENTTD_SERVICE_DEFAULT(enter_depot);
	void (OPENTTD_VEHICLE_CALL *pathfinding_result)(OpenTTDVehicle *, bool) noexcept OPENTTD_SERVICE_DEFAULT(pathfinding_result);
	OpenTTDVehicleNewPosition (OPENTTD_VEHICLE_CALL *new_position)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(new_position);
	void (OPENTTD_VEHICLE_CALL *write_xy)(OpenTTDVehicle *, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_xy);
	void (OPENTTD_VEHICLE_CALL *base_viewport)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(base_viewport);
	const OpenTTDVehicleResolverServices *(*newgrf)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf);
	int32_t (OPENTTD_VEHICLE_CALL *property)(const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, uint8_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(property);
	uint16_t (OPENTTD_VEHICLE_CALL *callback)(const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, CallbackID) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	void (OPENTTD_VEHICLE_CALL *unknown_callback_result)(const OpenTTDEngine *, CallbackID, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(unknown_callback_result);
	void (OPENTTD_VEHICLE_CALL *length_changed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(length_changed);
	bool (OPENTTD_VEHICLE_CALL *play_vehicle_sound)(OpenTTDVehicle *, VehicleSoundEvent) noexcept OPENTTD_SERVICE_DEFAULT(play_vehicle_sound);
	void (OPENTTD_VEHICLE_CALL *play_sound)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(play_sound);
	void (OPENTTD_VEHICLE_CALL *large_explosion)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(large_explosion);
	void (OPENTTD_VEHICLE_CALL *order_free)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_free);
	void (OPENTTD_VEHICLE_CALL *order_dummy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_dummy);
	void (OPENTTD_VEHICLE_CALL *start_stop_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(start_stop_dirty);
	void (OPENTTD_VEHICLE_CALL *destroy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	uint8_t (OPENTTD_VEHICLE_CALL *check_orders)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(check_orders);
	uint8_t (OPENTTD_VEHICLE_CALL *day)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(day);
	void (OPENTTD_VEHICLE_CALL *write_day)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_day);
	void (OPENTTD_VEHICLE_CALL *write_direction)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_direction);
	void (OPENTTD_VEHICLE_CALL *write_speed)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_speed);
	uint16_t (OPENTTD_VEHICLE_CALL *last_station)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(last_station);
	void (OPENTTD_VEHICLE_CALL *write_last_station)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_last_station);
	void (OPENTTD_VEHICLE_CALL *set_dest_tile)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_dest_tile);
	void (OPENTTD_VEHICLE_CALL *write_dest)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_dest);
	void (OPENTTD_VEHICLE_CALL *write_progress)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_progress);
	void (OPENTTD_VEHICLE_CALL *write_max_speed)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_max_speed);
	OpenTTDVehiclePreviousTile (OPENTTD_VEHICLE_CALL *previous_tile)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(previous_tile);
	void (OPENTTD_VEHICLE_CALL *detach_last)(OpenTTDVehicle *, OpenTTDVehicle *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(detach_last);
	OpenTTDVehicle *(OPENTTD_VEHICLE_CALL *next)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next);
	uint16_t (OPENTTD_VEHICLE_CALL *engine_id)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_id);
	const OpenTTDEngine *(OPENTTD_VEHICLE_CALL *engine)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	OpenTTDEngineForLength (OPENTTD_VEHICLE_CALL *engine_for_length)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(engine_for_length);
	void (OPENTTD_VEHICLE_CALL *write_tick)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_tick);
	uint8_t (OPENTTD_VEHICLE_CALL *direction)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(direction);
	uint8_t (OPENTTD_VEHICLE_CALL *status)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(status);
	uint32_t (OPENTTD_VEHICLE_CALL *tile)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	uint8_t (OPENTTD_VEHICLE_CALL *progress)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(progress);
	uint32_t (OPENTTD_VEHICLE_CALL *dest_tile)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(dest_tile);
	uint8_t (OPENTTD_VEHICLE_CALL *order_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_type);
	uint16_t (OPENTTD_VEHICLE_CALL *order_destination)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_destination);
	uint8_t (OPENTTD_VEHICLE_CALL *order_nonstop)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_nonstop);
	uint8_t (OPENTTD_VEHICLE_CALL *subtype)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(subtype);
	int32_t (OPENTTD_VEHICLE_CALL *x)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(x);
	int32_t (OPENTTD_VEHICLE_CALL *y)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(y);
	int32_t (OPENTTD_VEHICLE_CALL *z)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(z);
	uint8_t (OPENTTD_VEHICLE_CALL *tick)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tick);
	uint8_t (OPENTTD_VEHICLE_CALL *owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	uint16_t (OPENTTD_VEHICLE_CALL *speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(speed);
	uint8_t (OPENTTD_VEHICLE_CALL *subspeed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(subspeed);
	uint8_t (OPENTTD_VEHICLE_CALL *running)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(running);
	int32_t (OPENTTD_VEHICLE_CALL *order_time)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(order_time);
	uint8_t (OPENTTD_VEHICLE_CALL *vehicle_type)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_type);
	void (OPENTTD_VEHICLE_CALL *write_x)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_x);
	void (OPENTTD_VEHICLE_CALL *write_y)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_y);
	void (OPENTTD_VEHICLE_CALL *write_z)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_z);
	void (OPENTTD_VEHICLE_CALL *write_tile)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_tile);
	void (OPENTTD_VEHICLE_CALL *write_status)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_status);
	void (OPENTTD_VEHICLE_CALL *write_subspeed)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_subspeed);
	void (OPENTTD_VEHICLE_CALL *write_breakdown_counter)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_breakdown_counter);
	void (OPENTTD_VEHICLE_CALL *write_running)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_running);
	void (OPENTTD_VEHICLE_CALL *write_order_time)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_order_time);
	void (OPENTTD_VEHICLE_CALL *write_profit)(OpenTTDVehicle *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(write_profit);
	void (OPENTTD_VEHICLE_CALL *write_economy_service)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_economy_service);
	void (OPENTTD_VEHICLE_CALL *write_calendar_service)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_calendar_service);
	void (OPENTTD_VEHICLE_CALL *write_breakdowns)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_breakdowns);
	void (OPENTTD_VEHICLE_CALL *write_reliability)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_reliability);
	void (OPENTTD_VEHICLE_CALL *write_cargo_age)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_cargo_age);
	OpenTTDVehicleCoordinates (OPENTTD_VEHICLE_CALL *coordinates)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(coordinates);
	OpenTTDVehicleXY (OPENTTD_VEHICLE_CALL *xy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(xy);
	OpenTTDVehicleSpeedInputs (OPENTTD_VEHICLE_CALL *speed_inputs)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(speed_inputs);
	OpenTTDVehicleDirectionProgress (OPENTTD_VEHICLE_CALL *direction_progress)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(direction_progress);
	OpenTTDVehicleTickStatus (OPENTTD_VEHICLE_CALL *tick_status)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(tick_status);
	OpenTTDVehicleDayCost (OPENTTD_VEHICLE_CALL *day_cost)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(day_cost);
	void (OPENTTD_VEHICLE_CALL *make_dummy)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(make_dummy);
	void (*loading)(OpenTTDVehicle *, bool, const OpenTTDVehicleVisits *) noexcept OPENTTD_SERVICE_DEFAULT(loading);
	void (OPENTTD_VEHICLE_CALL *check_orders_only)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(check_orders_only);
	void (OPENTTD_VEHICLE_CALL *write_coordinates)(OpenTTDVehicle *, int32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_coordinates);
	OpenTTDVehicleAltitudeInputs (OPENTTD_VEHICLE_CALL *altitude_inputs)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(altitude_inputs);
	OpenTTDVehiclePositionObservations (OPENTTD_VEHICLE_CALL *position_observations)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(position_observations);
	OpenTTDVehicleMovementObservations (OPENTTD_VEHICLE_CALL *movement_observations)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(movement_observations);
	uint16_t (OPENTTD_VEHICLE_CALL *max_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(max_speed);
	const OpenTTDVehicleMaintenanceServices *maintenance;
	const OpenTTDVehicleVisits *(*visits)() noexcept OPENTTD_SERVICE_DEFAULT(visits);
};
struct OpenTTDGroundViewportNext {
	OpenTTDVehicle *next;
	uint8_t final_progress;
};
struct OpenTTDGroundPhysicsServices;
struct OpenTTDGroundServices {
	const OpenTTDGroundPhysicsServices *physics;
	void (OPENTTD_VEHICLE_CALL *cache_begin)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cache_begin);
	OpenTTDGroundViewportNext (OPENTTD_VEHICLE_CALL *update_viewport_then_next)(OpenTTDVehicle *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(update_viewport_then_next);
	void (OPENTTD_VEHICLE_CALL *turn)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(turn);
	void (OPENTTD_VEHICLE_CALL *go_to_depot_service)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(go_to_depot_service);
	void (OPENTTD_VEHICLE_CALL *write_first_engine)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_first_engine);
	void (OPENTTD_VEHICLE_CALL *write_length)(OpenTTDVehicle *, uint8_t, OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_length);
	uint8_t (OPENTTD_VEHICLE_CALL *cached_length)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cached_length);
	bool (OPENTTD_VEHICLE_CALL *has_articulated)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(has_articulated);
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
	Vehicle *const *slots;
	size_t slots_len;
	const size_t *first_unused;
};
/** Named actual scalar members. No reference into the shell crosses a callback. */
struct OpenTTDVehiclePostTick {
	VehicleType type;
	const uint16_t *period;
	uint16_t *counter;
	uint32_t *motion;
	const uint16_t *speed;
	const VehicleType *front_type;
	const uint8_t *subtype;
	const uint8_t *tick;
	const uint8_t *front_status;
	const uint8_t *status;
};
struct OpenTTDVehicleDayCallback { uint8_t day; bool has_engine; };
struct OpenTTDVehicleSchedulerTables;
struct OpenTTDVehicleSchedulerServices {
	OpenTTDVehiclePoolCursor (OPENTTD_VEHICLE_CALL *pool)() noexcept OPENTTD_SERVICE_DEFAULT(pool);
	bool assertions;
	/** Runs the virtual Tick; fills the bundle only when the vehicle survives. */
	bool (OPENTTD_VEHICLE_CALL *tick)(OpenTTDVehicle *, OpenTTDVehiclePostTick *) noexcept OPENTTD_SERVICE_DEFAULT(tick);
	void (OPENTTD_VEHICLE_CALL *calendar_day)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(calendar_day);
	void (OPENTTD_VEHICLE_CALL *economy_day)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(economy_day);
	OpenTTDVehicleDayCallback (OPENTTD_VEHICLE_CALL *day_callback_read)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(day_callback_read);
	void (OPENTTD_VEHICLE_CALL *first_dirty)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_dirty);
	OpenTTDCargoList *(OPENTTD_VEHICLE_CALL *cargo_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_owner);
	const OpenTTDVehicleServices *vehicle; ///< Base vehicle table, immutable for the process.
	const OpenTTDSharedServices *shared; ///< Shared table, immutable for the process.
	void (OPENTTD_VEHICLE_CALL *with_loading_timer)(const OpenTTDVehicleSchedulerTables *, void (OPENTTD_VEHICLE_CALL *)(const OpenTTDVehicleSchedulerTables *)) noexcept OPENTTD_SERVICE_DEFAULT(with_loading_timer);
	void (OPENTTD_VEHICLE_CALL *reset_accumulators)() noexcept OPENTTD_SERVICE_DEFAULT(reset_accumulators);
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
void openttd_rust_occupancy_train(const OpenTTDOccupancyCommands *, OpenTTDCommandCost *, uint32_t, uint8_t);
void openttd_rust_occupancy_wormhole(const OpenTTDOccupancyCommands *, OpenTTDCommandCost *, uint32_t, uint32_t, const OpenTTDVehicle *);
void openttd_rust_occupancy_update(const OpenTTDVehicleServices *, OpenTTDVehicle *, bool);
void openttd_rust_occupancy_reset(const OpenTTDVehicleSchedulerServices *);
OpenTTDVehicleOnTile openttd_rust_occupancy_on_tile(const OpenTTDVehicleServices *, uint32_t);
void openttd_rust_occupancy_on_advance(const OpenTTDVehicleServices *, OpenTTDVehicleOnTile *);
OpenTTDVehicleNear openttd_rust_occupancy_near(const OpenTTDVehicleServices *, int32_t, int32_t, uint32_t);
void openttd_rust_occupancy_near_advance(const OpenTTDVehicleServices *, OpenTTDVehicleNear *);
}
extern "C" {
OpenTTDVehicle *openttd_rust_vehicle_cursor_next(const OpenTTDVehicleSchedulerServices *, OpenTTDVehiclePoolCursor *, size_t *);
void openttd_rust_vehicle_calendar_day(const OpenTTDVehicleSchedulerTables *, bool, size_t);
void openttd_rust_vehicle_ticks(const OpenTTDVehicleSchedulerTables *, bool, size_t, void *);
}
#endif /* RUST_VEHICLE_FFI_H */
