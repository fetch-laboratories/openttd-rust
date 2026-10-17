/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_cost_ffi.h Native stack-owned command results shared by Rust ports. */
#ifndef RUST_COMMAND_COST_FFI_H
#define RUST_COMMAND_COST_FFI_H
#include <cstdint>

/* Opaque live native CommandCost. Rust cannot construct, inspect or destroy it.
 * Every pointer denotes C++ stack storage that outlives the synchronous call;
 * mutations/moves preserve all native fields, including encoded error params.
 * These ordinary services are noexcept; environmental failures terminate. */
struct OpenTTDCommandCost;
struct OpenTTDCommandCostServices {
	void (*zero)(OpenTTDCommandCost *) noexcept;
	void (*error)(OpenTTDCommandCost *, uint32_t) noexcept;
	void (*expense_money)(OpenTTDCommandCost *, uint8_t, int64_t) noexcept;
	void (*error_param)(OpenTTDCommandCost *, uint32_t, uint64_t) noexcept;
	void (*add)(OpenTTDCommandCost *, OpenTTDCommandCost *) noexcept;
	void (*move)(OpenTTDCommandCost *, OpenTTDCommandCost *) noexcept;
	void (*amount)(OpenTTDCommandCost *, int64_t) noexcept;
	bool (*success)(const OpenTTDCommandCost *) noexcept;
	uint32_t (*error_message)(const OpenTTDCommandCost *) noexcept;
	int64_t (*money)(const OpenTTDCommandCost *) noexcept;
	void (*make_error)(OpenTTDCommandCost *, uint32_t) noexcept;
};
const OpenTTDCommandCostServices &GetRustCommandCostServices() noexcept;
#endif /* RUST_COMMAND_COST_FFI_H */
