/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_services_ffi.h Direct ordinary command leaves with native result storage. */
#ifndef RUST_COMMAND_SERVICES_FFI_H
#define RUST_COMMAND_SERVICES_FFI_H
#include "service_defaults.hpp"
#include "command_cost_ffi.h"
struct OpenTTDOccupancyCommands;
struct OpenTTDCompanyFinanceServices;
struct OpenTTDCompanyServices;
struct OpenTTDLandscapeCommandServices;
struct OpenTTDRailMapServices;
struct OpenTTDSharedServices;
struct OpenTTDSignalServices;
struct OpenTTDCommandServices {
	const OpenTTDCompanyFinanceServices *finance;
	bool (*infinite_money)() noexcept OPENTTD_SERVICE_DEFAULT(infinite_money);
	void (*money_error)(OpenTTDCommandCost *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(money_error);
	void (*infrastructure_dirty)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(infrastructure_dirty);
	const OpenTTDLandscapeCommandServices *(*landscape_commands)() noexcept OPENTTD_SERVICE_DEFAULT(landscape_commands);
	const OpenTTDCompanyServices *(*company)() noexcept OPENTTD_SERVICE_DEFAULT(company);
	const OpenTTDOccupancyCommands *(*occupancy)() noexcept OPENTTD_SERVICE_DEFAULT(occupancy);
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDRailMapServices *rail_map;
	const OpenTTDSignalServices *signals;
	void (*assert_condition)(bool) noexcept OPENTTD_SERVICE_DEFAULT(assert_condition);
	void (*storage_discard)(void *) noexcept OPENTTD_SERVICE_DEFAULT(storage_discard);
	bool (*shift_pressed)() noexcept OPENTTD_SERVICE_DEFAULT(shift_pressed);
	uint8_t (*pause_level)() noexcept OPENTTD_SERVICE_DEFAULT(pause_level);
	int32_t (*desync_level)() noexcept OPENTTD_SERVICE_DEFAULT(desync_level);
	void (*last_build_coordinate)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(last_build_coordinate);
	uint32_t not_enough_cash;
	void (*paused_error)(uint32_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(paused_error);
	void (*error_message)(uint32_t, int32_t, int32_t, const OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(error_message);
	void (*estimate)(int64_t, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(estimate);
	void (*cost_animation)(int32_t, int32_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(cost_animation);
};
/** One typed `Command<Tcmd>` invocation; see `command.rs` `Frame`. */
struct OpenTTDCommandFrame {
	OpenTTDCommandCost *results[3];
	bool (*run)(OpenTTDCommandFrame *, uint16_t, uint8_t) noexcept;
	int64_t (*additional_money)(OpenTTDCommandFrame *) noexcept;
	void (*log)(OpenTTDCommandFrame *, bool) noexcept;
	void (*send)(OpenTTDCommandFrame *, uint8_t) noexcept;
	void (*set_client_ids)(OpenTTDCommandFrame *) noexcept;
	void (*assert_client_ids)(OpenTTDCommandFrame *) noexcept;
	void (*callback)(OpenTTDCommandFrame *, uint8_t) noexcept;
};
const OpenTTDCommandServices &GetRustCommandServices() noexcept;
extern "C" {
bool openttd_rust_command_stack_probe(const OpenTTDCommandCostServices *);
bool openttd_rust_command_allowed_while_paused(const OpenTTDCommandServices *, uint8_t);
bool openttd_rust_command_do(const OpenTTDCommandServices *, OpenTTDCommandFrame *, uint32_t, uint16_t);
uint8_t openttd_rust_command_execute(const OpenTTDCommandServices *, OpenTTDCommandFrame *, uint8_t, uint8_t, uint16_t, bool, uint32_t);
uint8_t openttd_rust_command_post(const OpenTTDCommandServices *, OpenTTDCommandFrame *, uint8_t, uint8_t, uint16_t, uint32_t, bool, bool, uint32_t, bool);
void openttd_rust_command_subtract_money(const OpenTTDCommandServices *, const OpenTTDCommandCostServices *, const OpenTTDCommandCost *);
bool openttd_rust_command_check_money(const OpenTTDCommandServices *, const OpenTTDCommandCostServices *, OpenTTDCommandCost *);
}
#endif /* RUST_COMMAND_SERVICES_FFI_H */
