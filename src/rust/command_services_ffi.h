/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_services_ffi.h Direct ordinary command leaves with native result storage. */
#ifndef RUST_COMMAND_SERVICES_FFI_H
#define RUST_COMMAND_SERVICES_FFI_H
#include "command_cost_ffi.h"
struct OpenTTDCommandServices {
	void (*landscape_clear)(OpenTTDCommandCost *, uint32_t, uint16_t) noexcept;
	void (*ensure_no_vehicle)(OpenTTDCommandCost *, uint32_t) noexcept;
};
const OpenTTDCommandServices &GetRustCommandServices() noexcept;
#endif /* RUST_COMMAND_SERVICES_FFI_H */
