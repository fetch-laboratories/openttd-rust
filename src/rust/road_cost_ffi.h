/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_cost_ffi.h Shared road price policy owned by Rust. */
#ifndef RUST_ROAD_COST_FFI_H
#define RUST_ROAD_COST_FFI_H
#include "services_ffi.h"
extern "C" {
int64_t openttd_rust_road_build_cost(const OpenTTDSharedServices *, uint8_t);
int64_t openttd_rust_road_clear_cost(const OpenTTDSharedServices *, uint8_t);
int64_t openttd_rust_road_convert_cost(const OpenTTDSharedServices *, uint8_t, uint8_t);
}
#endif /* RUST_ROAD_COST_FFI_H */
