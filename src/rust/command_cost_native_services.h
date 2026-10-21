/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_cost_native_services.h Shared native command result operations for Rust adapters. */
#ifndef RUST_COMMAND_COST_NATIVE_SERVICES_H
#define RUST_COMMAND_COST_NATIVE_SERVICES_H
#include "../command_type.h"
#include "../company_func.h"
#include "../strings_func.h"
#include "../table/strings.h"

/* Pointers borrow live C++ command results for the synchronous call. Native
 * assignment preserves every result field; environmental failures terminate. */
inline void OpenTTDCommandCostZero(CommandCost *out) noexcept { *out = CommandCost(); }
inline void OpenTTDCommandCostError(CommandCost *out, StringID message) noexcept { *out = CommandCost(message); }
inline void OpenTTDCommandCostExpenseMoney(CommandCost *out, ExpensesType expense, Money money) noexcept { *out = CommandCost(expense, money); }
inline bool OpenTTDCommandCostSuccess(const CommandCost *cost) noexcept { return cost->Succeeded(); }
/** Encode the source owned-by parameters after the caller's local-company test. */
inline void RustEncodeOwnedBy(CommandCost &cost, Owner owner, TileIndex tile) noexcept
{
	auto params = GetParamsForOwnedBy(owner, tile);
	cost.SetEncodedMessage(GetEncodedStringWithArgs(STR_ERROR_OWNED_BY, params));
}
#endif /* RUST_COMMAND_COST_NATIVE_SERVICES_H */
