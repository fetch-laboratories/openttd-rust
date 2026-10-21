/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_cost_ffi.h Native stack-owned command results shared by Rust ports. */
#ifndef RUST_COMMAND_COST_FFI_H
#define RUST_COMMAND_COST_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
enum ExpensesType : uint8_t;

/* Opaque live native CommandCost. Rust cannot construct, inspect or destroy it.
 * Every pointer denotes C++ stack storage that outlives the synchronous call;
 * mutations/moves preserve all native fields, including encoded error params.
 * These ordinary services are noexcept; environmental failures terminate. */
struct OpenTTDCommandCost;
enum ExpensesType : uint8_t;
struct OpenTTDCommandCostServices {
	void (*zero)(OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(zero);
	void (*error)(OpenTTDCommandCost *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(error);
	void (*expense_money)(OpenTTDCommandCost *, uint8_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(expense_money);
	void (*error_param)(OpenTTDCommandCost *, uint32_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(error_param);
	void (*add)(OpenTTDCommandCost *, OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(add);
	void (*move)(OpenTTDCommandCost *, OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(move);
	void (*amount)(OpenTTDCommandCost *, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(amount);
	bool (*success)(const OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(success);
	uint32_t (*error_message)(const OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(error_message);
	int64_t (*money)(const OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(money);
	ExpensesType (*expense)(const OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(expense);
	void (*make_error)(OpenTTDCommandCost *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(make_error);
};
const OpenTTDCommandCostServices &GetRustCommandCostServices() noexcept;
#endif /* RUST_COMMAND_COST_FFI_H */
