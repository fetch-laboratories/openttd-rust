/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_road_ffi.h Rust-owned `ScriptRoad` decisions; see `script_road.rs`. */
#ifndef OPENTTD_RUST_SCRIPT_ROAD_FFI_H
#define OPENTTD_RUST_SCRIPT_ROAD_FFI_H
#include <stddef.h>
#include <stdint.h>
#include "call_ffi.h"
#include "../tile_type.h"
struct OpenTTDScriptHostServices;
extern "C" {
int64_t OPENTTD_RUST_CALL openttd_rust_script_road_build_cost(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_road_max_speed(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_powered(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_road_current_type();
void OPENTTD_RUST_CALL openttd_rust_script_road_set_current_type(const OpenTTDScriptHostServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_road_can_build_parts(const OpenTTDScriptHostServices *, uint32_t, const TileIndex *, size_t, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_road_can_build_parts_here(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_road_neighbour_count(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_road_depot_front_tile(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_road_station_front_tile(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_road_drive_through_back_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_build(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_depot(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_station(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_remove_station(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_is_road_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_is_depot_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_is_station_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_is_drive_through_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_is_type_available(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_has_type(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_tiles_connected(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_convert(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_remove(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_station_tiles(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_road_remove_depot(const OpenTTDScriptHostServices *, uint32_t);
}
#endif /* OPENTTD_RUST_SCRIPT_ROAD_FFI_H */
