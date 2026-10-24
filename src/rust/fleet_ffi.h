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
#include <cstdint>
#include <cstddef>
#include "command_services_ffi.h"
/* Group, statistics, renewal and company-head owners are stable Rust allocations whose prefix holds
 * the C++ scalar objects that views alias. Vehicle::group_id stays in the C++ shell, read and written
 * through vehicle_group/set_membership; GRPS/ERNW/PLYR adapters keep widths, references and indexed
 * pool identity. Name, children and engine maps and pending IDs are Rust containers. Commands keep
 * caller-owned stack CommandCost objects with their original move and AddCost behavior. */
struct OpenTTDVehicle;
struct OpenTTDFleetGroupServices {
	void * (*group)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(group);
	uint32_t (*next_group)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_group);
	uint32_t (*next_company)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_company);
	OpenTTDVehicle * (*next_vehicle)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_vehicle);
	void * (*vehicle)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(vehicle);
	uint32_t (*vehicle_id)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_id);
	uint8_t (*vehicle_type)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_type);
	uint8_t (*vehicle_owner)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_owner);
	uint16_t (*vehicle_group)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_group);
	uint16_t (*vehicle_engine)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_engine);
	int64_t (*profit)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(profit);
	bool (*old_enough)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(old_enough);
	bool (*primary)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(primary);
	bool (*countable)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(countable);
	bool (*ground)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(ground);
	bool (*front)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(front);
	void * (*next_part)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(next_part);
	void * (*first_shared)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(first_shared);
	void * (*next_shared)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(next_shared);
	void (*set_membership)(void *shell, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(set_membership);
	void (*invalidate_cache)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_cache);
	void (*viewport)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(viewport);
	void * (*stats)(uint8_t company, uint16_t id, uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(stats);
	void * (*head)(uint8_t company) noexcept OPENTTD_SERVICE_DEFAULT(head);
	void * (*renew_state)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(renew_state);
	void * (*next_renew)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_renew);
	uint32_t (*renew_id)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(renew_id);
	uint8_t (*engine_type)(uint16_t engine) noexcept OPENTTD_SERVICE_DEFAULT(engine_type);
	uint8_t (*current_company)() noexcept OPENTTD_SERVICE_DEFAULT(current_company);
	bool (*buildable_type)(uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(buildable_type);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	uint16_t (*allocate)(uint8_t owner, uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	uint16_t (*use_number)(uint8_t owner) noexcept OPENTTD_SERVICE_DEFAULT(use_number);
	void (*release_number)(uint8_t owner, uint16_t number) noexcept OPENTTD_SERVICE_DEFAULT(release_number);
	const uint8_t * (*company_livery)(uint8_t owner) noexcept OPENTTD_SERVICE_DEFAULT(company_livery);
	bool (*keep_length)(uint8_t owner) noexcept OPENTTD_SERVICE_DEFAULT(keep_length);
	void (*delete_group)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(delete_group);
	void (*invalid_parent)(uint16_t id, uint16_t parent) noexcept OPENTTD_SERVICE_DEFAULT(invalid_parent);
	void (*clear_backup)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(clear_backup);
	void (*remove_rule)(uint8_t owner, uint16_t engine, uint16_t id, uint32_t flags) noexcept OPENTTD_SERVICE_DEFAULT(remove_rule);
	void (*remove_vehicles)(uint16_t id, uint32_t flags) noexcept OPENTTD_SERVICE_DEFAULT(remove_vehicles);
	void (*delete_child)(uint16_t id, uint32_t flags) noexcept OPENTTD_SERVICE_DEFAULT(delete_child);
	void (*add_to_group)(uint16_t id, uint32_t vehicle, uint32_t flags) noexcept OPENTTD_SERVICE_DEFAULT(add_to_group);
	size_t (*utf8_length)(const uint8_t *text, size_t length) noexcept OPENTTD_SERVICE_DEFAULT(utf8_length);
	void (*list_dirty)(uint8_t owner, uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(list_dirty);
	void (*list_set_dirty)(uint8_t owner, uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(list_set_dirty);
	void (*colour_dirty)(uint8_t owner, uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(colour_dirty);
	void (*replace_dirty)(uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(replace_dirty);
	void (*replace_invalidate)(uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(replace_invalidate);
	void (*alter_dirty)(uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(alter_dirty);
	void (*vehicle_dirty)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_dirty);
	void (*depot_dirty)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(depot_dirty);
	void (*close_replace)(uint8_t type) noexcept OPENTTD_SERVICE_DEFAULT(close_replace);
	void (*screen_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(screen_dirty);
	bool (*list_generate)(void *context) noexcept OPENTTD_SERVICE_DEFAULT(list_generate);
	size_t (*list_size)(void *context) noexcept OPENTTD_SERVICE_DEFAULT(list_size);
	void * (*list_at)(void *context, size_t index) noexcept OPENTTD_SERVICE_DEFAULT(list_at);
	void * (*renew_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(renew_allocate);
	bool (*renew_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(renew_can_allocate);
	void (*renew_delete)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(renew_delete);
	uint32_t recursion_error;
};
struct OpenTTDFleetCosts { void *result; void *replace; void *build; void *copy; void *temporary; void *seeds; };
struct OpenTTDVehicleCommonServices;
struct OpenTTDVehicleRefitServices;
struct OpenTTDVehicleBuildServices;
struct OpenTTDFleetTransactionServices {
	const OpenTTDVehicleCommonServices &(*common)() noexcept OPENTTD_SERVICE_DEFAULT(common);
	const OpenTTDVehicleRefitServices &(*refit_services)() noexcept OPENTTD_SERVICE_DEFAULT(refit_services);
	const OpenTTDVehicleBuildServices &(*build_services)() noexcept OPENTTD_SERVICE_DEFAULT(build_services);
	void (*cost_zero)(void *out) noexcept OPENTTD_SERVICE_DEFAULT(cost_zero);
	void (*cost_vehicles)(void *out) noexcept OPENTTD_SERVICE_DEFAULT(cost_vehicles);
	void (*cost_error)(void *out, uint32_t error) noexcept OPENTTD_SERVICE_DEFAULT(cost_error);
	void (*cost_add)(void *out, void *item) noexcept OPENTTD_SERVICE_DEFAULT(cost_add);
	void (*cost_move)(void *out, void *item) noexcept OPENTTD_SERVICE_DEFAULT(cost_move);
	void (*cost_amount)(void *out, int64_t amount) noexcept OPENTTD_SERVICE_DEFAULT(cost_amount);
	bool (*success)(void *out) noexcept OPENTTD_SERVICE_DEFAULT(success);
	uint32_t (*error)(void *out) noexcept OPENTTD_SERVICE_DEFAULT(error);
	int64_t (*money)(void *out) noexcept OPENTTD_SERVICE_DEFAULT(money);
	const OpenTTDCommandServices *commands;
	bool (*rear)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(rear);
	bool (*articulated)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(articulated);
	bool (*crashed)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(crashed);
	bool (*stopped)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(stopped);
	bool (*chain_depot)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(chain_depot);
	void * (*first)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(first);
	void * (*next_unit)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(next_unit);
	void * (*prev_unit)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(prev_unit);
	uint16_t (*length)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(length);
	uint8_t (*cargo_type)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(cargo_type);
	bool (*can_carry)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(can_carry);
	bool (*stopped_in_depot)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(stopped_in_depot);
	uint8_t (*max_length)() noexcept OPENTTD_SERVICE_DEFAULT(max_length);
	void (*check)(bool condition) noexcept OPENTTD_SERVICE_DEFAULT(check);
	bool (*needs_renew)(void *shell, bool settings) noexcept OPENTTD_SERVICE_DEFAULT(needs_renew);
	bool (*engine_valid)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(engine_valid);
	bool (*company_valid)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(company_valid);
	bool (*engine_buildable)(uint16_t id, uint8_t type, uint8_t company) noexcept OPENTTD_SERVICE_DEFAULT(engine_buildable);
	uint64_t (*rail_compatible)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(rail_compatible);
	uint64_t (*road_powered)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(road_powered);
	bool (*wagon)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(wagon);
	bool (*tram)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(tram);
	uint8_t (*plane)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(plane);
	uint64_t (*refit_mask)(uint16_t id, bool initial) noexcept OPENTTD_SERVICE_DEFAULT(refit_mask);
	void (*refit_masks)(uint16_t id, uint64_t *union_mask, uint64_t *available) noexcept OPENTTD_SERVICE_DEFAULT(refit_masks);
	uint64_t (*vehicle_cargo)(void *shell, uint8_t *cargo) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	uint64_t (*default_cargo)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(default_cargo);
	void * (*orders)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(orders);
	size_t (*order_count)(void *orders) noexcept OPENTTD_SERVICE_DEFAULT(order_count);
	uint8_t (*order_count_id)(void *orders) noexcept OPENTTD_SERVICE_DEFAULT(order_count_id);
	void * (*order_at)(void *orders, size_t index) noexcept OPENTTD_SERVICE_DEFAULT(order_at);
	bool (*order_refit)(void *order) noexcept OPENTTD_SERVICE_DEFAULT(order_refit);
	bool (*order_auto)(void *order) noexcept OPENTTD_SERVICE_DEFAULT(order_auto);
	uint8_t (*order_cargo)(void *order) noexcept OPENTTD_SERVICE_DEFAULT(order_cargo);
	bool (*local)() noexcept OPENTTD_SERVICE_DEFAULT(local);
	void (*refit_news)(void *shell, int32_t order) noexcept OPENTTD_SERVICE_DEFAULT(refit_news);
	uint8_t (*subtype)(void *old, void *replacement, uint8_t cargo) noexcept OPENTTD_SERVICE_DEFAULT(subtype);
	void (*reverse)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(reverse);
	void (*copy_configuration)(void *old, void *replacement) noexcept OPENTTD_SERVICE_DEFAULT(copy_configuration);
	void (*viewports)(void *old, void *replacement) noexcept OPENTTD_SERVICE_DEFAULT(viewports);
	void (*view_window)(void *old, void *replacement) noexcept OPENTTD_SERVICE_DEFAULT(view_window);
	void (*news)(void *old, void *replacement) noexcept OPENTTD_SERVICE_DEFAULT(news);
	void (*transfer_cargo)(void *old, void *replacement, bool chain) noexcept OPENTTD_SERVICE_DEFAULT(transfer_cargo);
	void (*event)(void *old, void *replacement) noexcept OPENTTD_SERVICE_DEFAULT(event);
	void (*save_rng)(void *seeds) noexcept OPENTTD_SERVICE_DEFAULT(save_rng);
	void (*restore_rng)(void *seeds) noexcept OPENTTD_SERVICE_DEFAULT(restore_rng);
	void (*rule_window)(uint16_t engine, uint16_t group) noexcept OPENTTD_SERVICE_DEFAULT(rule_window);
	bool assertions;
	uint32_t unavailable;
	uint32_t too_long;
	uint32_t too_long_replacement;
	uint32_t nothing;
};
struct OpenTTDFleetPendingServices {
	void (*set_current)(uint8_t company) noexcept OPENTTD_SERVICE_DEFAULT(set_current);
	void (*restart)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(restart);
	int32_t (*x)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(x);
	int32_t (*y)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(y);
	int32_t (*z)(void *shell) noexcept OPENTTD_SERVICE_DEFAULT(z);
	uint32_t (*reserve)(uint8_t company) noexcept OPENTTD_SERVICE_DEFAULT(reserve);
	void (*subtract)(int64_t amount) noexcept OPENTTD_SERVICE_DEFAULT(subtract);
	void (*command)(void *out, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(command);
	void (*animation)(int32_t x, int32_t y, int32_t z, int64_t amount) noexcept OPENTTD_SERVICE_DEFAULT(animation);
	void (*length_news)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(length_news);
	void (*failed_news)(uint32_t id, uint32_t error) noexcept OPENTTD_SERVICE_DEFAULT(failed_news);
	uint32_t cash;
	uint32_t limit;
};
const OpenTTDFleetGroupServices &FleetGroupServices();
const OpenTTDFleetTransactionServices &FleetTransactionServices();
/** Cargo capacity traversal service (cargo_storage.rs), shared as the fleet next-part service. */
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
void openttd_rust_fleet_stats_clear_profits(void *);
void openttd_rust_fleet_stats_clear_autoreplace(void *);
void openttd_rust_fleet_update_children(const OpenTTDFleetGroupServices *);
void openttd_rust_fleet_count_vehicle(const OpenTTDFleetGroupServices *, void *, int32_t);
void openttd_rust_fleet_count_engine(const OpenTTDFleetGroupServices *, void *, int32_t);
void openttd_rust_fleet_add_profit(const OpenTTDFleetGroupServices *, void *);
void openttd_rust_fleet_min_age(const OpenTTDFleetGroupServices *, void *);
void openttd_rust_fleet_update_afterload(const OpenTTDFleetGroupServices *);
void openttd_rust_fleet_update_profits(const OpenTTDFleetGroupServices *);
void openttd_rust_fleet_update_autoreplace(const OpenTTDFleetGroupServices *, uint8_t);
void openttd_rust_fleet_company_liveries(const OpenTTDFleetGroupServices *, uint8_t);
uint32_t openttd_rust_fleet_create_group(const OpenTTDFleetGroupServices *, uint32_t, uint8_t, uint16_t, uint16_t *);
uint32_t openttd_rust_fleet_delete_group(const OpenTTDFleetGroupServices *, uint32_t, uint16_t);
uint32_t openttd_rust_fleet_alter_group(const OpenTTDFleetGroupServices *, uint32_t, uint8_t, uint16_t, uint16_t, const uint8_t *, size_t);
uint32_t openttd_rust_fleet_add_vehicle_group(const OpenTTDFleetGroupServices *, uint32_t, uint16_t, uint32_t, bool, void *, bool, uint16_t *);
uint32_t openttd_rust_fleet_add_shared_group(const OpenTTDFleetGroupServices *, uint32_t, uint16_t, uint8_t);
uint32_t openttd_rust_fleet_remove_vehicles_group(const OpenTTDFleetGroupServices *, uint32_t, uint16_t);
uint32_t openttd_rust_fleet_set_livery(const OpenTTDFleetGroupServices *, uint32_t, uint16_t, bool, uint8_t);
uint32_t openttd_rust_fleet_flag_command(const OpenTTDFleetGroupServices *, uint32_t, uint16_t, uint8_t, bool, bool);
void openttd_rust_fleet_set_train_group(const OpenTTDFleetGroupServices *, void *, uint16_t);
void openttd_rust_fleet_update_train_group(const OpenTTDFleetGroupServices *, void *);
uint32_t openttd_rust_fleet_sum_engines(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint16_t);
uint32_t openttd_rust_fleet_sum_vehicles(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint8_t);
uint32_t openttd_rust_fleet_sum_min_age(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint8_t);
int64_t openttd_rust_fleet_sum_profit(const OpenTTDFleetGroupServices *, uint8_t, uint16_t, uint8_t);
void openttd_rust_fleet_remove_company_groups(const OpenTTDFleetGroupServices *, uint8_t);
bool openttd_rust_fleet_contains(const OpenTTDFleetGroupServices *, uint16_t, uint16_t);
uint16_t openttd_rust_fleet_replacement(const OpenTTDFleetGroupServices *, void *, uint16_t, uint16_t, bool *);
uint32_t openttd_rust_fleet_add_replacement(const OpenTTDFleetGroupServices *, void **, uint16_t, uint16_t, uint16_t, bool, uint32_t);
uint32_t openttd_rust_fleet_remove_replacement(const OpenTTDFleetGroupServices *, void **, uint16_t, uint16_t, uint32_t);
void openttd_rust_fleet_remove_all_replacements(const OpenTTDFleetGroupServices *, void **);
bool openttd_rust_fleet_valid(const OpenTTDFleetTransactionServices *, const OpenTTDFleetGroupServices *, uint16_t, uint16_t, uint8_t);
void openttd_rust_fleet_autoreplace(const OpenTTDFleetTransactionServices *, const OpenTTDFleetGroupServices *, const OpenTTDFleetCosts *, uint32_t, uint32_t);
uint32_t openttd_rust_fleet_set_rule(const OpenTTDFleetTransactionServices *, const OpenTTDFleetGroupServices *, uint32_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_fleet_pending_clear();
void openttd_rust_fleet_pending_add(uint32_t, bool);
void openttd_rust_fleet_pending_drain(const OpenTTDFleetPendingServices *, const OpenTTDFleetTransactionServices *, const OpenTTDFleetGroupServices *, void *);
}
#endif /* RUST_FLEET_FFI_H */
