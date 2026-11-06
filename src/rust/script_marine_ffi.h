/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_marine_ffi.h Rust-owned `ScriptMarine` decisions; see `script_marine.rs`. */
#ifndef OPENTTD_RUST_SCRIPT_MARINE_FFI_H
#define OPENTTD_RUST_SCRIPT_MARINE_FFI_H
#include <stdint.h>
#include "call_ffi.h"
struct OpenTTDScriptHostServices;
extern "C" {
int64_t OPENTTD_RUST_CALL openttd_rust_script_marine_build_cost(uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_water_depot(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_is_water_depot_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_is_dock_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_is_buoy_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_is_lock_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_is_canal_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_marine_tiles_connected(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
}
#endif /* OPENTTD_RUST_SCRIPT_MARINE_FFI_H */
