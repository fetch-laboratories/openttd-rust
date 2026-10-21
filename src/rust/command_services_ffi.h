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
struct Train;
struct OpenTTDCompanyFinanceServices;
struct OpenTTDCommandServices {
	bool (*enter)() noexcept OPENTTD_SERVICE_DEFAULT(enter);
	void (*leave)() noexcept OPENTTD_SERVICE_DEFAULT(leave);
	const OpenTTDCompanyFinanceServices *finance;
	bool (*infinite_money)() noexcept OPENTTD_SERVICE_DEFAULT(infinite_money);
	void (*money_error)(OpenTTDCommandCost *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(money_error);
	void (*infrastructure_dirty)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(infrastructure_dirty);
	void (*landscape_clear)(OpenTTDCommandCost *, uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(landscape_clear);
	void (*tile_ownership)(OpenTTDCommandCost *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_ownership);
	void (*ownership)(OpenTTDCommandCost *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(ownership);
	void (*no_vehicle)(OpenTTDCommandCost *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(no_vehicle);
	void (*no_train)(OpenTTDCommandCost *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(no_train);
	void (*wormhole_free)(OpenTTDCommandCost *, uint32_t, uint32_t, const Train *) noexcept OPENTTD_SERVICE_DEFAULT(wormhole_free);
	void (*allow_remove_road)(OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t, uint8_t, uint16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(allow_remove_road);
};
const OpenTTDCommandServices &GetRustCommandServices() noexcept;
extern "C" {
void openttd_rust_command_before(const OpenTTDCommandServices *, bool, bool);
void openttd_rust_command_after(const OpenTTDCommandServices *, const OpenTTDCommandCostServices *, OpenTTDCommandCost *, uint16_t, bool, bool);
void openttd_rust_command_subtract_money(const OpenTTDCommandServices *, const OpenTTDCommandCostServices *, const OpenTTDCommandCost *);
bool openttd_rust_command_check_money(const OpenTTDCommandServices *, const OpenTTDCommandCostServices *, OpenTTDCommandCost *);
}
#endif /* RUST_COMMAND_SERVICES_FFI_H */
