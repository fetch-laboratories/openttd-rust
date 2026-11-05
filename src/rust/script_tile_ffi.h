/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_tile_ffi.h Rust-owned `ScriptMap` and `ScriptTile` decisions; see `script_tile.rs`. */
#ifndef OPENTTD_RUST_SCRIPT_TILE_FFI_H
#define OPENTTD_RUST_SCRIPT_TILE_FFI_H
#include <stdint.h>
#include "call_ffi.h"
struct OpenTTDScriptHostServices;
extern "C" {
uint8_t OPENTTD_RUST_CALL openttd_rust_script_map_is_valid_tile(const OpenTTDScriptHostServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_size(const OpenTTDScriptHostServices *);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_size_x(const OpenTTDScriptHostServices *);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_size_y(const OpenTTDScriptHostServices *);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_tile_x(const OpenTTDScriptHostServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_tile_y(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_map_tile_index(const OpenTTDScriptHostServices *, int64_t, int64_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_distance_manhattan(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_distance_max(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_distance_square(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_map_distance_from_edge(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_buildable(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_buildable_rectangle(const OpenTTDScriptHostServices *, uint32_t, int64_t, int64_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_sea(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_river(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_water(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_coast(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_station(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_steep_slope(uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_halftile_slope(uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_has_tree(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_farm(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_rock(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_rough(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_snow(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_desert(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_is_house(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_tile_terrain_type(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_tile_slope(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_tile_complement_slope(uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_tile_min_height(const OpenTTDScriptHostServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_tile_max_height(const OpenTTDScriptHostServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_tile_corner_height(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
int32_t OPENTTD_RUST_CALL openttd_rust_script_tile_owner(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tile_has_transport_type(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_tile_build_cost(uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_tile_plant_tree_rectangle(const OpenTTDScriptHostServices *, uint32_t, int64_t, int64_t);
}
#endif /* OPENTTD_RUST_SCRIPT_TILE_FFI_H */
