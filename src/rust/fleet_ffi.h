struct OpenTTDEngineCapabilitiesServices;
struct OpenTTDSharedServices;
/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file fleet_ffi.h Canonical fleet state and typed synchronous control. */
#ifndef RUST_FLEET_FFI_H
#define RUST_FLEET_FFI_H
#include "service_defaults.hpp"
#include "../livery.h"
#include <cstdint>
#include <cstddef>
#include "command_services_ffi.h"
#include "byte_span_ffi.h"
/* Group, statistics, renewal and company-head owners are stable Rust allocations whose prefix holds
 * the C++ scalar objects that views alias. Vehicle::group_id stays in the C++ shell, read and written
 * through group_id/set_group_id; GRPS/ERNW/PLYR adapters keep widths, references and indexed
 * pool identity. Name, children and engine maps are Rust containers. Commands write the
 * caller-owned stack CommandCost they are given. */
struct OpenTTDVehicle;
struct OpenTTDCommandCost;
struct OpenTTDVehicleServices;
struct OpenTTDVehicleSelectionServices;
/* A group pool lookup's step: the shell, its Rust owner, its statistics' owner and its index; null for no group. */
struct Group;
struct EngineRenew;
struct FleetGroupFields;
struct FleetStatisticsFields;
struct FleetRenewFields;
struct OpenTTDFleetGroupRecord { Group *shell; FleetGroupFields *group; FleetStatisticsFields *statistics; uint16_t index; };
/* EngineRenew::Iterate(from)'s next step: the shell and its index; null after the last. */
struct OpenTTDFleetRenewRecord { EngineRenew *shell; uint16_t index; };
struct OpenTTDFleetGroupServices {
	/* Tables. */
	const OpenTTDVehicleSelectionServices *(*selection)() noexcept OPENTTD_SERVICE_DEFAULT(selection);
	const OpenTTDSharedServices *shared = nullptr;
	const OpenTTDVehicleServices *vehicles;
	/* Group pool. */
	bool (*group_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(group_can_allocate);
	void (*group_delete)(Group *shell) noexcept OPENTTD_SERVICE_DEFAULT(group_delete);
	OpenTTDFleetGroupRecord (*group_get)(uint16_t index) noexcept OPENTTD_SERVICE_DEFAULT(group_get);
	OpenTTDFleetGroupRecord (*group_get_if_valid)(uint16_t index) noexcept OPENTTD_SERVICE_DEFAULT(group_get_if_valid);
	OpenTTDFleetGroupRecord (*group_new)(uint8_t owner, uint8_t vt) noexcept OPENTTD_SERVICE_DEFAULT(group_new);
	OpenTTDFleetGroupRecord (*next_group)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_group);
	/* GroupStatistics. */
	FleetStatisticsFields * (*group_statistics_get)(uint8_t company, uint16_t id_g, uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(group_statistics_get);
	/* EngineRenew pool. */
	OpenTTDFleetRenewRecord (*next_renew)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_renew);
	bool (*renew_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(renew_can_allocate);
	void (*renew_delete)(EngineRenew *shell) noexcept OPENTTD_SERVICE_DEFAULT(renew_delete);
	EngineRenew * (*renew_new)() noexcept OPENTTD_SERVICE_DEFAULT(renew_new);
	FleetRenewFields * (*renew_state)(EngineRenew *shell) noexcept OPENTTD_SERVICE_DEFAULT(renew_state);
	/* Vehicle members. */
	uint16_t (*group_id)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(group_id);
	bool (*is_engine_countable)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(is_engine_countable);
	void (*set_group_id)(OpenTTDVehicle *v, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(set_group_id);
};
struct OpenTTDVehicleBuildServices;
struct OpenTTDFleetTransactionServices {
	/* Tables. */
	const OpenTTDVehicleBuildServices &(*build)() noexcept OPENTTD_SERVICE_DEFAULT(build);
	const OpenTTDEngineCapabilitiesServices *capabilities;
	const OpenTTDCommandServices *commands;
	const OpenTTDVehicleServices *vehicles;
	/* Unported native bodies. */
	void (*change_vehicle_news)(uint32_t from_index, uint32_t to_index) noexcept OPENTTD_SERVICE_DEFAULT(change_vehicle_news);
	void (*change_vehicle_view_window)(uint32_t from_index, uint32_t to_index) noexcept OPENTTD_SERVICE_DEFAULT(change_vehicle_view_window);
	void (*change_vehicle_viewports)(uint32_t from_index, uint32_t to_index) noexcept OPENTTD_SERVICE_DEFAULT(change_vehicle_viewports);
	void (*copy_vehicle_config_and_statistics)(OpenTTDVehicle *v, OpenTTDVehicle *src) noexcept OPENTTD_SERVICE_DEFAULT(copy_vehicle_config_and_statistics);
	uint8_t (*get_best_fitting_sub_type)(OpenTTDVehicle *v_from, OpenTTDVehicle *v_for, uint8_t dest_cargo_type) noexcept OPENTTD_SERVICE_DEFAULT(get_best_fitting_sub_type);
	void (*invalidate_autoreplace_window)(uint16_t e, uint16_t id_g) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_autoreplace_window);
};
const OpenTTDFleetGroupServices &FleetGroupServices();
const OpenTTDFleetTransactionServices &FleetTransactionServices();
extern "C" {
void *openttd_rust_fleet_group_create();
void openttd_rust_fleet_group_destroy(void *);
void *openttd_rust_fleet_stats_create();
void openttd_rust_fleet_stats_destroy(void *);
void *openttd_rust_fleet_renew_create();
void openttd_rust_fleet_renew_destroy(void *);
void *openttd_rust_fleet_head_create();
void openttd_rust_fleet_head_destroy(void *);
void openttd_rust_fleet_stats_copy(void *, const void *);
void openttd_rust_fleet_head_copy(void *, const void *);
size_t openttd_rust_fleet_name(const void *, uint8_t *, size_t);
void openttd_rust_fleet_set_name(void *, const uint8_t *, size_t);
uint32_t openttd_rust_fleet_child(const void *, uint32_t);
void openttd_rust_fleet_child_change(void *, uint32_t, bool);
uint16_t openttd_rust_fleet_engine_count(const void *, uint16_t);
void openttd_rust_fleet_engine_change(void *, uint16_t, int32_t);
void openttd_rust_fleet_stats_clear(void *);
void openttd_rust_fleet_update_children(const OpenTTDFleetGroupServices *);
void openttd_rust_fleet_count_vehicle(const OpenTTDFleetGroupServices *, void *, int32_t);
void openttd_rust_fleet_count_engine(const OpenTTDFleetGroupServices *, void *, int32_t);
void openttd_rust_fleet_update_afterload(const OpenTTDFleetGroupServices *);
void openttd_rust_fleet_update_profits(const OpenTTDFleetGroupServices *);
void openttd_rust_fleet_update_autoreplace(const OpenTTDFleetGroupServices *, uint8_t);
uint16_t openttd_rust_fleet_create_group(const OpenTTDFleetGroupServices *, uint16_t, uint8_t, uint16_t, void *);
void openttd_rust_fleet_delete_group(const OpenTTDFleetGroupServices *, uint16_t, uint16_t, void *);
void openttd_rust_fleet_alter_group(const OpenTTDFleetGroupServices *, uint16_t, uint8_t, uint16_t, uint16_t, OpenTTDByteSpan, void *);
uint16_t openttd_rust_fleet_add_vehicle_group(const OpenTTDFleetGroupServices *, uint16_t, uint16_t, uint32_t, bool, uint8_t, uint8_t, uint8_t, uint32_t, void *);
void openttd_rust_fleet_add_shared_group(const OpenTTDFleetGroupServices *, uint16_t, uint16_t, uint8_t, void *);
void openttd_rust_fleet_remove_vehicles_group(const OpenTTDFleetGroupServices *, uint16_t, uint16_t, void *);
void openttd_rust_fleet_set_livery(const OpenTTDFleetGroupServices *, uint16_t, uint16_t, bool, uint8_t, void *);
void openttd_rust_fleet_flag_command(const OpenTTDFleetGroupServices *, uint16_t, uint16_t, uint8_t, bool, bool, void *);
uint32_t openttd_rust_fleet_sum_engines(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint16_t);
uint32_t openttd_rust_fleet_sum_vehicles(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint8_t);
uint32_t openttd_rust_fleet_sum_min_age(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint8_t);
int64_t openttd_rust_fleet_sum_profit(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint8_t);
uint8_t openttd_rust_fleet_contains_abi(const OpenTTDFleetGroupServices *, uint16_t, uint16_t);
extern "C++" inline bool openttd_rust_fleet_contains(const OpenTTDFleetGroupServices *a0, uint16_t a1, uint16_t a2) { return openttd_rust_fleet_contains_abi(a0, a1, a2) != 0; }
uint16_t openttd_rust_fleet_replacement(const OpenTTDFleetGroupServices *, EngineRenew *, uint16_t, uint16_t, bool *);
void openttd_rust_fleet_remove_all_replacements(const OpenTTDFleetGroupServices *, EngineRenew **);
uint8_t openttd_rust_fleet_valid_abi(const OpenTTDFleetTransactionServices *, uint16_t, uint16_t, uint8_t);
extern "C++" inline bool openttd_rust_fleet_valid(const OpenTTDFleetTransactionServices *a0, uint16_t a1, uint16_t a2, uint8_t a3) { return openttd_rust_fleet_valid_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_fleet_autoreplace(const OpenTTDFleetTransactionServices *, OpenTTDCommandCost *, uint16_t, uint32_t);
void openttd_rust_fleet_set_rule(const OpenTTDFleetTransactionServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint16_t, uint16_t, uint8_t);
void openttd_rust_fleet_pending_clear();
}
#endif /* RUST_FLEET_FFI_H */
