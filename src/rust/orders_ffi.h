/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file orders_ffi.h Canonical order, shared-list and timetable owners. */
#ifndef RUST_ORDERS_FFI_H
#define RUST_ORDERS_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"

/** Opaque live native entity, distinct from Rust-owned boundary state. */
struct OrderList;
using OpenTTDOrderList = OrderList;
#include <cstddef>
#include <cstdint>
struct OpenTTDOrderFields {
	uint8_t type, flags;
	uint16_t dest;
	uint8_t refit_cargo;
	uint16_t wait_time, travel_time, max_speed;
};
struct OpenTTDConsistState {
	int32_t current_order_time, lateness_counter;
	uint64_t timetable_start, depot_unbunching_last_departure, depot_unbunching_next_departure;
	int32_t round_trip_time;
	uint8_t cur_real_order_index, cur_implicit_order_index;
	uint16_t vehicle_flags;
};
struct OpenTTDVehicleOrderState {
	OpenTTDOrderFields current_order;
	void *orders, *next_shared, *previous_shared;
};
struct OpenTTDOrderListState {
	uint8_t num_manual_orders;
	uint32_t num_vehicles;
	void *first_shared;
	int32_t timetable_duration, total_duration;
};
struct OpenTTDOrderBackupState {
	uint32_t user, tile;
	uint16_t group;
	const void *clone;
};
struct OpenTTDOrderVector;
struct OpenTTDOrdersClosest { uint32_t tile; uint16_t destination; uint8_t reverse, found; };
/* Immutable typed service table, borrowed synchronously without copying.
 * Services terminate on escaping exceptions; Rust borrows end before callbacks. */
struct BaseStation;
struct OpenTTDDepotServices;
struct OpenTTDStationCommandServices;
struct OpenTTDVehicle;
struct OpenTTDVehicleSchedulerServices;
struct OpenTTDVehicleServices;
struct OpenTTDVehicleTransactionServices;
struct OpenTTDOrdersLeaves {
	/* Pool operations and the owners of C++ shells, until the order pools move (#906). */
	void *(*allocate_list)() noexcept OPENTTD_SERVICE_DEFAULT(allocate_list);
	OpenTTDOrderBackupState *(*backup)(void *) noexcept OPENTTD_SERVICE_DEFAULT(backup);
	uint8_t (*backup_can_allocate_item)() noexcept OPENTTD_SERVICE_DEFAULT(backup_can_allocate_item);
	OpenTTDConsistState *(*backup_consist)(void *) noexcept OPENTTD_SERVICE_DEFAULT(backup_consist);
	OpenTTDOrderVector *(*backup_vector)(void *) noexcept OPENTTD_SERVICE_DEFAULT(backup_vector);
	void *(*create_backup)(void *v, uint32_t user) noexcept OPENTTD_SERVICE_DEFAULT(create_backup);
	void (*delete_backup)(void *) noexcept OPENTTD_SERVICE_DEFAULT(delete_backup);
	void (*delete_list)(void *) noexcept OPENTTD_SERVICE_DEFAULT(delete_list);
	OpenTTDOrderListState *(*list)(void *) noexcept OPENTTD_SERVICE_DEFAULT(list);
	uint8_t (*list_can_allocate_item)() noexcept OPENTTD_SERVICE_DEFAULT(list_can_allocate_item);
	void *(*next_backup)(uint32_t *cursor) noexcept OPENTTD_SERVICE_DEFAULT(next_backup);
	OpenTTDOrderVector *(*vector)(void *) noexcept OPENTTD_SERVICE_DEFAULT(vector);
	/* Members still stored in C++ shells: one reader or writer each. */
	void (*backup_clear_name)(void *backup) noexcept OPENTTD_SERVICE_DEFAULT(backup_clear_name);
	OpenTTDByteSpan (*backup_name)(const void *backup) noexcept OPENTTD_SERVICE_DEFAULT(backup_name);
	uint16_t (*backup_service_interval)(const void *backup) noexcept OPENTTD_SERVICE_DEFAULT(backup_service_interval);
	void (*backup_set_name)(void *backup, const void *v) noexcept OPENTTD_SERVICE_DEFAULT(backup_set_name);
	void (*backup_set_service_interval)(void *backup, uint16_t interval) noexcept OPENTTD_SERVICE_DEFAULT(backup_set_service_interval);
	void (*vehicle_set_name)(void *v, const void *backup) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_set_name);
	void (*vehicle_set_service_interval)(void *v, uint16_t interval) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_set_service_interval);
	/* Lookups still stored in C++ pools (#634, #638). */
	BaseStation *(*base_station_get_if_valid)(uint16_t index) noexcept OPENTTD_SERVICE_DEFAULT(base_station_get_if_valid);
	uint32_t (*depot_get)(uint16_t index) noexcept OPENTTD_SERVICE_DEFAULT(depot_get);
	BaseStation *(*waypoint_get_if_valid)(uint16_t index) noexcept OPENTTD_SERVICE_DEFAULT(waypoint_get_if_valid);
	/* Unported bodies. */
	void (*debug_check_sanity)(void *list) noexcept OPENTTD_SERVICE_DEFAULT(debug_check_sanity);
	uint32_t (*get_vehicle_cannot_use_station_reason)(void *v, uint16_t st) noexcept OPENTTD_SERVICE_DEFAULT(get_vehicle_cannot_use_station_reason);
	void (*invalidate_vehicle_order)(void *v, int32_t data) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_vehicle_order);
	/* Commands and the C++ vector the export of GetNextStoppingStation fills. */
	void (*append_station)(void *next_station, uint16_t station) noexcept OPENTTD_SERVICE_DEFAULT(append_station);
	void (*clear_order_backup_unsafe)(uint32_t tile, uint32_t user) noexcept OPENTTD_SERVICE_DEFAULT(clear_order_backup_unsafe);
	/* Tables. */
	const OpenTTDVehicleServices *base;
	const OpenTTDDepotServices *depots;
	const OpenTTDVehicleSchedulerServices *scheduler;
	const OpenTTDStationCommandServices *station_commands;
	const OpenTTDVehicleTransactionServices *vehicle_transactions;
};
extern "C" {
void openttd_rust_order_free(void *);
void openttd_rust_order_station(void *, uint16_t);
void openttd_rust_order_depot(void *, uint16_t, uint8_t, uint8_t, uint8_t, uint8_t);
void openttd_rust_order_waypoint(void *, uint16_t);
void openttd_rust_order_loading(void *, uint8_t);
void openttd_rust_order_dummy(void *);
void openttd_rust_order_conditional(void *, uint8_t);
void openttd_rust_order_implicit(void *, uint16_t);
void openttd_rust_order_refit(void *, uint8_t);
uint8_t openttd_rust_order_equals(const void *, const void *);
void openttd_rust_order_assign(void *, const void *);
uint16_t openttd_rust_order_convert(const void *);
uint32_t openttd_rust_order_location(const void *, OpenTTDVehicle *, uint8_t, const OpenTTDOrdersLeaves *);
uint32_t openttd_rust_order_distance(uint8_t, uint8_t, OpenTTDVehicle *, int32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_list_init(void *, OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_list_recalculate(void *, const OpenTTDOrdersLeaves *);
void openttd_rust_list_free(void *, uint8_t, const OpenTTDOrdersLeaves *);
void openttd_rust_list_stopping(void *, void *, OpenTTDVehicle *, uint8_t, uint32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_list_insert(void *, const void *, uint8_t, const OpenTTDOrdersLeaves *);
uint8_t openttd_rust_list_complete(void *, const OpenTTDOrdersLeaves *);
void openttd_rust_add_shared(OpenTTDVehicle *, OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_delete_implicit(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
uint8_t openttd_rust_wait_unbunching(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_leave_unbunching(void *, const OpenTTDOrdersLeaves *);
void openttd_rust_increment_implicit(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_increment_real(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_update_real(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_insert_order(OpenTTDVehicle *, const void *, uint8_t, const OpenTTDOrdersLeaves *);
void openttd_rust_delete_vehicle_orders(OpenTTDVehicle *, uint8_t, uint8_t, const OpenTTDOrdersLeaves *);
uint8_t openttd_rust_process_orders(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
void openttd_rust_remove_destination(uint8_t, uint16_t, uint8_t, const OpenTTDOrdersLeaves *);
void openttd_rust_command_insert(uint16_t, uint32_t, uint8_t, const void *, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_delete(uint16_t, uint32_t, uint8_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_skip(uint16_t, uint32_t, uint8_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_move(uint16_t, uint32_t, uint8_t, uint8_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_modify(uint16_t, uint32_t, uint8_t, uint8_t, uint16_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_clone(uint16_t, uint8_t, uint32_t, uint32_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_refit(uint16_t, uint32_t, uint8_t, uint8_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_command_clear_backup(uint16_t, uint32_t, uint32_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_timetable_change(uint16_t, uint32_t, uint8_t, uint8_t, uint16_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_timetable_bulk(uint16_t, uint32_t, uint8_t, uint16_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_timetable_on_time(uint16_t, uint32_t, uint8_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_timetable_start(uint16_t, uint32_t, uint8_t, uint64_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_timetable_autofill(uint16_t, uint32_t, uint8_t, uint8_t, void *, const OpenTTDOrdersLeaves *);
void openttd_rust_capture_backup(void *, OpenTTDVehicle *, uint32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_create(OpenTTDVehicle *, uint32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_restore(OpenTTDVehicle *, uint32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_clear_user(uint32_t, uint32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_reset(uint32_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_tile_clear(uint32_t, uint8_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_group_clear(uint16_t, const OpenTTDOrdersLeaves *);
void openttd_rust_backup_vehicle_clear(OpenTTDVehicle *, const OpenTTDOrdersLeaves *);
uint64_t openttd_rust_orders_start_tick(int32_t, const OpenTTDOrdersLeaves *);
int32_t openttd_rust_orders_start_date(uint64_t, const OpenTTDOrdersLeaves *);
OpenTTDConsistState *openttd_rust_consist_new();
void openttd_rust_consist_delete(OpenTTDConsistState *);
void openttd_rust_consist_copy(OpenTTDConsistState *, const OpenTTDConsistState *);
void openttd_rust_consist_reset(OpenTTDConsistState *);
OpenTTDVehicleOrderState *openttd_rust_vehicle_orders_new();
void openttd_rust_vehicle_orders_delete(OpenTTDVehicleOrderState *);
OpenTTDOrderListState *openttd_rust_orderlist_new();
void openttd_rust_orderlist_delete(OpenTTDOrderListState *);
OpenTTDOrderBackupState *openttd_rust_orderbackup_new();
void openttd_rust_orderbackup_delete(OpenTTDOrderBackupState *);
OpenTTDOrderVector *openttd_rust_order_vector_new(void (*)(void *, size_t) noexcept, void (*)(void *, size_t) noexcept);
void openttd_rust_order_vector_delete(OpenTTDOrderVector *);
size_t openttd_rust_order_vector_size(const OpenTTDOrderVector *);
OpenTTDOrderFields *openttd_rust_order_vector_data(const OpenTTDOrderVector *);
void openttd_rust_order_vector_resize(OpenTTDOrderVector *, size_t);
void openttd_rust_order_vector_insert(OpenTTDOrderVector *, size_t, const void *);
void openttd_rust_order_vector_erase(OpenTTDOrderVector *, size_t);
void openttd_rust_order_vector_assign(OpenTTDOrderVector *, const void *, size_t);
/* C++ establishes Order/DestinationID lifetimes in Rust-allocated cells.
 * Trivial destruction precedes freeing/replacing their allocation. */
void openttd_orders_construct(void *, size_t) noexcept;
void openttd_orders_destroy(void *, size_t) noexcept;
}
const OpenTTDOrdersLeaves &GetRustOrdersLeaves();
#endif /* RUST_ORDERS_FFI_H */
