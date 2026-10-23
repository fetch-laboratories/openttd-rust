/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tunnelbridge_command_ffi.h Whole tunnel/bridge command owner. */
#ifndef RUST_TUNNELBRIDGE_COMMAND_FFI_H
#define RUST_TUNNELBRIDGE_COMMAND_FFI_H
#include "road_command_ffi.h"
#include "water_command_ffi.h"
#include "company_ffi.h"
#include "object_ffi.h"
#include "../bridge_type.h"
/* The game-state mutex serializes calls. Costs are live native stack slots;
 * tables are immutable, map/pool identities remain native. No world reference
 * survives reentry. Direct noexcept leaves terminate on environmental failure;
 * no ordinary-play service throws, and Rust panics abort across this boundary. */
struct OpenTTDBridgeAvailability { int32_t year; uint8_t min; uint16_t max; };
struct OpenTTDTunnelBridgeStrings {
	uint32_t bridge_border;
	uint32_t map_edge;
	uint32_t same_end;
	uint32_t straight;
	uint32_t too_long;
	uint32_t wrong_slope;
	uint32_t unequal_height;
	uint32_t demolish_bridge;
	uint32_t already_built;
	uint32_t authority;
	uint32_t other_owner;
	uint32_t too_low;
	uint32_t too_high;
	uint32_t tunnel_site;
	uint32_t water;
	uint32_t tunnel_border;
	uint32_t tunnel_way;
	uint32_t tunnel_long;
	uint32_t excavate;
	uint32_t demolish_tunnel;
};
struct OpenTTDTunnelBridgeServices {
	const OpenTTDRailCommandServices *rail;
	const OpenTTDRoadCommandServices *road;
	const OpenTTDRoadGroundServices *road_ground;
	const OpenTTDWaterCommandServices *water;
	const OpenTTDCompanyServices *company;
	OpenTTDBridgeAvailability (*availability)(BridgeType type) noexcept OPENTTD_SERVICE_DEFAULT(availability);
	uint16_t (*speed)(BridgeType type) noexcept OPENTTD_SERVICE_DEFAULT(speed);
	uint16_t (*price)(BridgeType type) noexcept OPENTTD_SERVICE_DEFAULT(price);
	uint16_t (*max_bridge_length)() noexcept OPENTTD_SERVICE_DEFAULT(max_bridge_length);
	uint8_t (*max_bridge_height)() noexcept OPENTTD_SERVICE_DEFAULT(max_bridge_height);
	uint16_t (*max_tunnel_length)() noexcept OPENTTD_SERVICE_DEFAULT(max_tunnel_length);
	bool (*crossing_tunnels)() noexcept OPENTTD_SERVICE_DEFAULT(crossing_tunnels);
	uint32_t (*other_bridge_end)(uint32_t t) noexcept OPENTTD_SERVICE_DEFAULT(other_bridge_end);
	uint32_t (*northern_bridge_end)(uint32_t t) noexcept OPENTTD_SERVICE_DEFAULT(northern_bridge_end);
	BridgeType (*bridge_type)(uint32_t t) noexcept OPENTTD_SERVICE_DEFAULT(bridge_type);
	bool (*is_tunnel)(uint32_t t) noexcept OPENTTD_SERVICE_DEFAULT(is_tunnel);
	bool (*tunnel_in_way)(uint32_t t, int32_t z, DiagDirection d) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_in_way);
	void (*set_middle)(uint32_t t, Axis a) noexcept OPENTTD_SERVICE_DEFAULT(set_middle);
	void (*clear_middle)(uint32_t t) noexcept OPENTTD_SERVICE_DEFAULT(clear_middle);
	void (*set_reservation)(uint32_t t, bool reserved) noexcept OPENTTD_SERVICE_DEFAULT(set_reservation);
	void (*make_rail_bridge)(uint32_t t, uint8_t o, BridgeType b, DiagDirection d, RailType rt) noexcept OPENTTD_SERVICE_DEFAULT(make_rail_bridge);
	void (*make_road_bridge)(uint32_t t, uint8_t o, uint8_t road, uint8_t tram, BridgeType b, DiagDirection d, RoadType rt, RoadType tt) noexcept OPENTTD_SERVICE_DEFAULT(make_road_bridge);
	void (*make_aqueduct)(uint32_t t, uint8_t o, DiagDirection d) noexcept OPENTTD_SERVICE_DEFAULT(make_aqueduct);
	void (*make_rail_tunnel)(uint32_t t, uint8_t o, DiagDirection d, RailType rt) noexcept OPENTTD_SERVICE_DEFAULT(make_rail_tunnel);
	void (*make_road_tunnel)(uint32_t t, uint8_t o, DiagDirection d, RoadType rt, RoadType tt) noexcept OPENTTD_SERVICE_DEFAULT(make_road_tunnel);
	void (*mark_bridge)(uint32_t start, uint32_t end, DiagDirection d, uint32_t z) noexcept OPENTTD_SERVICE_DEFAULT(mark_bridge);
	void (*mark_dirty_height)(uint32_t t, int32_t h) noexcept OPENTTD_SERVICE_DEFAULT(mark_dirty_height);
	void (*terraform)(OpenTTDCommandCost *out, uint16_t flags, uint32_t t, Slope slope) noexcept OPENTTD_SERVICE_DEFAULT(terraform);
	const OpenTTDObjectServices *objects;
	OpenTTDTunnelBridgeStrings strings;
};
const OpenTTDTunnelBridgeServices &GetRustTunnelBridgeServices() noexcept;
extern "C" {
int32_t openttd_rust_bridge_length_cost(int32_t);
Foundation openttd_rust_bridge_foundation(uint8_t, Axis);
bool openttd_rust_bridge_flat_ramp(const OpenTTDSharedServices *, uint8_t, Axis);
void openttd_rust_bridge_available(const OpenTTDTunnelBridgeServices *, OpenTTDCommandCost *, BridgeType, uint32_t, uint16_t);
void openttd_rust_build_bridge(const OpenTTDTunnelBridgeServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, TransportType, BridgeType, uint8_t);
void openttd_rust_build_tunnel(const OpenTTDTunnelBridgeServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, TransportType, uint8_t);
void openttd_rust_tunnelbridge_clear(const OpenTTDTunnelBridgeServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint16_t);
void openttd_rust_tunnelbridge_terraform(const OpenTTDTunnelBridgeServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint16_t, int32_t, uint8_t);
void openttd_rust_tunnelbridge_build_above(const OpenTTDTunnelBridgeServices *, OpenTTDCommandCost *, uint32_t, uint16_t, Axis, int32_t);
uint32_t openttd_rust_tunnel_endtile();
int32_t openttd_rust_bridge_height(const OpenTTDSharedServices *, uint32_t);
}
#endif /* RUST_TUNNELBRIDGE_COMMAND_FFI_H */
