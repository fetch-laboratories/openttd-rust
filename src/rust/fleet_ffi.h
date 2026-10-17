/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file fleet_ffi.h Canonical fleet state and typed synchronous control. */
#ifndef RUST_FLEET_FFI_H
#define RUST_FLEET_FFI_H
#include <cstdint>
#include <cstddef>
#include "command_services_ffi.h"
/* Serial game-thread calls only. Group, statistics, renewal and company-head
 * owners are stable Rust allocations with C++ scalar object lifetimes in their
 * prefix; C++ views alias canonical storage. Vehicle::group_id stays C++ shell
 * storage that Rust reads and writes through vehicle_group/set_membership.
 * GRPS/ERNW/PLYR adapters retain widths, references and indexed pool identity.
 * Name/children/engine maps and pending IDs are Rust containers. Names have
 * call-local GUI/save exports; ordered child views do not allocate.
 * Typed synchronous entries own group/rule/replacement and tick-end policy.
 * Native commands retain full caller-owned stack CommandCost objects and their
 * original move/AddCost behavior. No owner borrow spans reentry/destruction.
 * Panics and escaping environmental exceptions abort. ABI 390-396 pin the owner
 * prefixes, the three service tables and the native cost slots. */
struct OpenTTDVehicle;
struct OpenTTDFleetGroupServices {
	void * (*group)(uint16_t id) noexcept;
	uint32_t (*next_group)(uint32_t from) noexcept;
	uint32_t (*next_company)(uint32_t from) noexcept;
	OpenTTDVehicle * (*next_vehicle)(uint32_t from) noexcept;
	void * (*vehicle)(uint32_t id) noexcept;
	uint32_t (*vehicle_id)(void *shell) noexcept;
	uint8_t (*vehicle_type)(void *shell) noexcept;
	uint8_t (*vehicle_owner)(void *shell) noexcept;
	uint16_t (*vehicle_group)(void *shell) noexcept;
	uint16_t (*vehicle_engine)(void *shell) noexcept;
	int64_t (*profit)(void *shell) noexcept;
	bool (*old_enough)(void *shell) noexcept;
	bool (*primary)(void *shell) noexcept;
	bool (*countable)(void *shell) noexcept;
	bool (*ground)(void *shell) noexcept;
	bool (*front)(void *shell) noexcept;
	void * (*next_part)(void *shell) noexcept;
	void * (*first_shared)(void *shell) noexcept;
	void * (*next_shared)(void *shell) noexcept;
	void (*set_membership)(void *shell, uint16_t id) noexcept;
	void (*invalidate_cache)(void *shell) noexcept;
	void (*viewport)(void *shell) noexcept;
	void * (*stats)(uint8_t company, uint16_t id, uint8_t type) noexcept;
	void * (*head)(uint8_t company) noexcept;
	void * (*renew_state)(void *shell) noexcept;
	void * (*next_renew)(uint32_t from) noexcept;
	uint32_t (*renew_id)(void *shell) noexcept;
	uint8_t (*engine_type)(uint16_t engine) noexcept;
	uint8_t (*current_company)() noexcept;
	bool (*buildable_type)(uint8_t type) noexcept;
	bool (*can_allocate)() noexcept;
	uint16_t (*allocate)(uint8_t owner, uint8_t type) noexcept;
	uint16_t (*use_number)(uint8_t owner) noexcept;
	void (*release_number)(uint8_t owner, uint16_t number) noexcept;
	const uint8_t * (*company_livery)(uint8_t owner) noexcept;
	bool (*keep_length)(uint8_t owner) noexcept;
	void (*delete_group)(uint16_t id) noexcept;
	void (*invalid_parent)(uint16_t id, uint16_t parent) noexcept;
	void (*clear_backup)(uint16_t id) noexcept;
	void (*remove_rule)(uint8_t owner, uint16_t engine, uint16_t id, uint32_t flags) noexcept;
	void (*remove_vehicles)(uint16_t id, uint32_t flags) noexcept;
	void (*delete_child)(uint16_t id, uint32_t flags) noexcept;
	void (*add_to_group)(uint16_t id, uint32_t vehicle, uint32_t flags) noexcept;
	size_t (*utf8_length)(const uint8_t *text, size_t length) noexcept;
	void (*list_dirty)(uint8_t owner, uint8_t type) noexcept;
	void (*list_set_dirty)(uint8_t owner, uint8_t type) noexcept;
	void (*colour_dirty)(uint8_t owner, uint8_t type) noexcept;
	void (*replace_dirty)(uint8_t type) noexcept;
	void (*replace_invalidate)(uint8_t type) noexcept;
	void (*alter_dirty)(uint8_t type) noexcept;
	void (*vehicle_dirty)(uint32_t id) noexcept;
	void (*depot_dirty)(void *shell) noexcept;
	void (*close_replace)(uint8_t type) noexcept;
	void (*screen_dirty)() noexcept;
	bool (*list_generate)(void *context) noexcept;
	void (*list_push)(void *context, void *shell) noexcept;
	size_t (*list_size)(void *context) noexcept;
	void * (*list_at)(void *context, size_t index) noexcept;
	void * (*renew_allocate)() noexcept;
	bool (*renew_can_allocate)() noexcept;
	void (*renew_delete)(void *shell) noexcept;
	uint32_t recursion_error;
};
struct OpenTTDFleetCosts { void *result; void *replace; void *build; void *copy; void *temporary; void *seeds; };
struct OpenTTDFleetTransactionServices {
	void (*cost_zero)(void *out) noexcept;
	void (*cost_vehicles)(void *out) noexcept;
	void (*cost_error)(void *out, uint32_t error) noexcept;
	void (*cost_add)(void *out, void *item) noexcept;
	void (*cost_move)(void *out, void *item) noexcept;
	void (*cost_amount)(void *out, int64_t amount) noexcept;
	bool (*success)(void *out) noexcept;
	uint32_t (*error)(void *out) noexcept;
	int64_t (*money)(void *out) noexcept;
	const OpenTTDCommandServices *commands;
	bool (*rear)(void *shell) noexcept;
	bool (*articulated)(void *shell) noexcept;
	bool (*crashed)(void *shell) noexcept;
	bool (*stopped)(void *shell) noexcept;
	bool (*chain_depot)(void *shell) noexcept;
	void * (*first)(void *shell) noexcept;
	void * (*next_unit)(void *shell) noexcept;
	void * (*prev_unit)(void *shell) noexcept;
	uint16_t (*length)(void *shell) noexcept;
	bool (*flipped)(void *shell) noexcept;
	uint8_t (*cargo_type)(void *shell) noexcept;
	bool (*can_carry)(void *shell) noexcept;
	bool (*stopped_in_depot)(void *shell) noexcept;
	uint8_t (*max_length)() noexcept;
	void (*check)(bool condition) noexcept;
	bool (*needs_renew)(void *shell, bool settings) noexcept;
	bool (*engine_valid)(uint16_t id) noexcept;
	bool (*company_valid)(uint8_t id) noexcept;
	bool (*engine_buildable)(uint16_t id, uint8_t type, uint8_t company) noexcept;
	uint64_t (*rail_compatible)(uint16_t id) noexcept;
	uint64_t (*road_powered)(uint16_t id) noexcept;
	bool (*wagon)(uint16_t id) noexcept;
	bool (*tram)(uint16_t id) noexcept;
	uint8_t (*plane)(uint16_t id) noexcept;
	uint64_t (*refit_mask)(uint16_t id, bool initial) noexcept;
	void (*refit_masks)(uint16_t id, uint64_t *union_mask, uint64_t *available) noexcept;
	uint64_t (*vehicle_cargo)(void *shell, uint8_t *cargo) noexcept;
	uint64_t (*default_cargo)(uint16_t id) noexcept;
	void * (*orders)(void *shell) noexcept;
	size_t (*order_count)(void *orders) noexcept;
	uint8_t (*order_count_id)(void *orders) noexcept;
	void * (*order_at)(void *orders, size_t index) noexcept;
	bool (*order_refit)(void *order) noexcept;
	bool (*order_auto)(void *order) noexcept;
	uint8_t (*order_cargo)(void *order) noexcept;
	bool (*local)() noexcept;
	void (*refit_news)(void *shell, int32_t order) noexcept;
	void * (*build)(void *out, void *shell, uint16_t engine) noexcept;
	void (*refit)(void *out, void *shell, uint8_t cargo, uint8_t subtype) noexcept;
	uint8_t (*subtype)(void *old, void *replacement, uint8_t cargo) noexcept;
	bool (*reverse_probability)(void *shell) noexcept;
	void (*reverse)(void *shell) noexcept;
	void (*start_stop)(void *out, void *shell, bool evaluate) noexcept;
	void (*move)(void *out, void *shell, void *after, uint32_t flags, bool whole) noexcept;
	void (*sell)(void *out, void *shell, uint32_t flags) noexcept;
	void (*clone_order)(void *out, void *old, void *replacement) noexcept;
	void (*copy_group)(void *out, void *old, void *replacement) noexcept;
	void (*copy_configuration)(void *old, void *replacement) noexcept;
	void (*viewports)(void *old, void *replacement) noexcept;
	void (*view_window)(void *old, void *replacement) noexcept;
	void (*news)(void *old, void *replacement) noexcept;
	void (*transfer_cargo)(void *old, void *replacement, bool chain) noexcept;
	void (*capacity)(void *shell) noexcept;
	void (*event)(void *old, void *replacement) noexcept;
	void (*save_rng)(void *seeds) noexcept;
	void (*restore_rng)(void *seeds) noexcept;
	void (*rule_window)(uint16_t engine, uint16_t group) noexcept;
	bool assertions;
	uint32_t unavailable;
	uint32_t too_long;
	uint32_t too_long_replacement;
	uint32_t nothing;
};
struct OpenTTDFleetPendingServices {
	void (*set_current)(uint8_t company) noexcept;
	void (*restart)(void *shell) noexcept;
	int32_t (*x)(void *shell) noexcept;
	int32_t (*y)(void *shell) noexcept;
	int32_t (*z)(void *shell) noexcept;
	uint32_t (*reserve)(uint8_t company) noexcept;
	void (*subtract)(int64_t amount) noexcept;
	void (*command)(void *out, uint32_t id) noexcept;
	void (*animation)(int32_t x, int32_t y, int32_t z, int64_t amount) noexcept;
	void (*length_news)(uint32_t id) noexcept;
	void (*failed_news)(uint32_t id, uint32_t error) noexcept;
	uint32_t cash;
	uint32_t limit;
};
const OpenTTDFleetGroupServices &FleetGroupServices();
const OpenTTDFleetTransactionServices &FleetTransactionServices();
/** #139 capacity traversal service, shared as the fleet next-part service. */
void *CargoCapacityNextPart(void *shell) noexcept;
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
