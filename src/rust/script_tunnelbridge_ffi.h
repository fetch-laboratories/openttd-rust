/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_tunnelbridge_ffi.h Rust-owned `ScriptBridge`, bridge list and `ScriptTunnel` decisions; see `script_tunnelbridge.rs`. */
#ifndef OPENTTD_RUST_SCRIPT_TUNNELBRIDGE_FFI_H
#define OPENTTD_RUST_SCRIPT_TUNNELBRIDGE_FFI_H
#include <stdint.h>
#include "call_ffi.h"
struct OpenTTDScriptHostServices;
struct OpenTTDScriptList;
struct OpenTTDTunnelBridgeServices;
const OpenTTDTunnelBridgeServices &GetRustTunnelBridgeServices() noexcept;
/** `RoadPiece`; see `script_tunnelbridge.rs`. */
struct OpenTTDScriptRoadPiece {
	uint32_t tile;
	uint8_t bits;
};
extern "C" {
uint32_t OPENTTD_RUST_CALL openttd_rust_script_bridge_type(const OpenTTDScriptHostServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_bridge_max_speed(const OpenTTDTunnelBridgeServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_bridge_price(const OpenTTDTunnelBridgeServices *, uint32_t, int64_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_bridge_max_length(const OpenTTDScriptHostServices *, const OpenTTDTunnelBridgeServices *, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_script_bridge_min_length(const OpenTTDTunnelBridgeServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_bridge_other_end(const OpenTTDScriptHostServices *, const OpenTTDTunnelBridgeServices *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_script_tunnel_other_end(const OpenTTDScriptHostServices *, uint32_t);
OpenTTDScriptRoadPiece OPENTTD_RUST_CALL openttd_rust_script_tunnelbridge_road_piece(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_script_bridge_list(const OpenTTDScriptHostServices *, const OpenTTDTunnelBridgeServices *, OpenTTDScriptList *, uint8_t, int64_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_bridge_is_valid(const OpenTTDTunnelBridgeServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_bridge_is_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_bridge_build(const OpenTTDScriptHostServices *, uint32_t, uint32_t, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tunnel_is_tile(const OpenTTDScriptHostServices *, uint32_t);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_tunnel_build(const OpenTTDScriptHostServices *, uint32_t, uint32_t);
}
#endif /* OPENTTD_RUST_SCRIPT_TUNNELBRIDGE_FFI_H */
