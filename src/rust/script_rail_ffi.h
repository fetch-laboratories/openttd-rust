/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_rail_ffi.h Rust-owned `ScriptRail` decisions; see `script_rail.rs`. */
#ifndef OPENTTD_RUST_SCRIPT_RAIL_FFI_H
#define OPENTTD_RUST_SCRIPT_RAIL_FFI_H
#include <stdint.h>
#include "call_ffi.h"
struct OpenTTDScriptHostServices;
/** `DragPlan`; see `script_rail.rs`. */
struct OpenTTDScriptRailDrag {
	uint8_t run;
	uint8_t track;
	uint32_t to;
};
/** `SignalPlan`; see `script_rail.rs`. */
struct OpenTTDScriptRailSignal {
	uint8_t run;
	uint8_t track;
	uint8_t signal;
	uint8_t cycles;
};
extern "C" {
uint32_t OPENTTD_RUST_CALL openttd_rust_script_rail_tracks(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_rail_current_type();
void OPENTTD_RUST_CALL openttd_rust_script_rail_set_current_type(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_rail_type(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_rail_depot_front_tile(const OpenTTDScriptHostServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_rail_station_direction(const OpenTTDScriptHostServices *, uint32_t);
OpenTTDScriptRailDrag OPENTTD_RUST_CALL openttd_rust_script_rail_drag(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t, uint8_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_depot(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_waypoint(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_build_track(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_remove_track(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_rail_signal_type(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
OpenTTDScriptRailSignal OPENTTD_RUST_CALL openttd_rust_script_rail_build_signal(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_remove_signal(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_is_rail_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_is_level_crossing_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_is_depot_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_is_station_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_is_waypoint_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_is_type_available(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_train_can_run(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_tiles_connected(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_convert(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_rail_station(const OpenTTDScriptHostServices *, uint32_t, uint32_t, int64_t, int64_t);
}
#endif /* OPENTTD_RUST_SCRIPT_RAIL_FFI_H */
