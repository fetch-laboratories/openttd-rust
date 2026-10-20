/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_services.h Native lazy stack result storage, never viewed from Rust. */
#ifndef RUST_GROUND_SERVICES_H
#define RUST_GROUND_SERVICES_H
#include "ground_ffi.h"
#include "command_services.h"
#include "../command_type.h"
#include <optional>
struct OpenTTDGroundCall { std::optional<CommandCost> cost; };
inline CommandCost *NativeGroundCost(OpenTTDCommandCost *cost) noexcept { return reinterpret_cast<CommandCost *>(cost); }
inline OpenTTDCommandCost *RustGroundPrepare(OpenTTDGroundCall *call) noexcept { return RustCommandCost(&call->cost.emplace()); }
inline void RustGroundRelease(OpenTTDGroundCall *call) noexcept { call->cost.reset(); }
inline const OpenTTDGroundCostServices &GetRustGroundCostServices() noexcept
{
	static const OpenTTDGroundCostServices services{.commands = &GetRustCommandServices(), .costs = &GetRustCommandCostServices(), .prepare = RustGroundPrepare, .release = RustGroundRelease};
	return services;
}
#endif /* RUST_GROUND_SERVICES_H */
