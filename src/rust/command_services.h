/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_services.h Shared native result storage for ordinary command calls. */
#ifndef RUST_COMMAND_SERVICES_H
#define RUST_COMMAND_SERVICES_H
#include "command_cost_services.h"
#include "command_services_ffi.h"
#include "../command_type.h"
#include "../vehicle_func.h"
/* Canonical native check; legacy compound consumers retain their result policy. */
inline CommandCost RustEnsureNoVehicleOnGround(uint32_t tile) noexcept { return EnsureNoVehicleOnGround(TileIndex{tile}); }
#endif /* RUST_COMMAND_SERVICES_H */
