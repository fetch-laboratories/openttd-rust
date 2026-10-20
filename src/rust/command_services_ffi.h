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
struct OpenTTDCommandServices {
	bool (*depot_available)() noexcept OPENTTD_SERVICE_DEFAULT(depot_available);
	uint16_t (*new_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_depot);
	void (*depot_name)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(depot_name);
	void (*delete_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_depot);
	void (*infrastructure_dirty)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(infrastructure_dirty);
	void (*landscape_clear)(OpenTTDCommandCost *, uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(landscape_clear);
	void (*tile_ownership)(OpenTTDCommandCost *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_ownership);
	void (*ownership)(OpenTTDCommandCost *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(ownership);
	void (*no_vehicle)(OpenTTDCommandCost *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(no_vehicle);
	void (*no_train)(OpenTTDCommandCost *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(no_train);
	void (*wormhole_free)(OpenTTDCommandCost *, uint32_t, uint32_t, const Train *) noexcept OPENTTD_SERVICE_DEFAULT(wormhole_free);
};
const OpenTTDCommandServices &GetRustCommandServices() noexcept;
#endif /* RUST_COMMAND_SERVICES_FFI_H */
