/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file movement_ffi.h Call-local movement policy services. */
#ifndef RUST_MOVEMENT_FFI_H
#define RUST_MOVEMENT_FFI_H
#include "ground_ffi.h"
#include "rail_command_ffi.h"
#include "train_ffi.h"
#include "road_ffi.h"
#include "ship_control_ffi.h"
#include "vehicle_ffi.h"
#include "../bridge_type.h"
#include "orders_ffi.h"
enum class WaterTileType : uint8_t;
struct OpenTTDIndustryTileServices;
struct OpenTTDObjectServices;
struct OpenTTDTownLeaves;
struct OpenTTDTrainServices;
struct OpenTTDTunnelBridgeServices;
/* Only consumed entry-local owner identities/scalars; never a Vehicle/map view.
 * Shells survive the selected depot/admission/sound/viewport calls. No pointer
 * is retained after this synchronous entry and no Rust borrow crosses a call. */
struct OpenTTDMovementVehicle { OpenTTDTrainHandle train; OpenTTDRoadState *road; OpenTTDShipState *ship; VehicleType kind; };
struct OpenTTDMovementServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDOrdersLeaves *orders;
	const OpenTTDTrainServices *train;
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDTunnelBridgeServices *tunnel_bridge;
	uint64_t (*road_compatible)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_compatible);
	uint16_t (*road_type_speed)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_type_speed);
	RoadTramType (*road_kind)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_kind);
	OpenTTDMovementVehicle (*resolve)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	void (*depot_invalidate)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(depot_invalidate);
	void (*native_assert)(bool) noexcept OPENTTD_SERVICE_DEFAULT(native_assert);
	const OpenTTDObjectServices *objects;
	const OpenTTDTownLeaves *town;
	const OpenTTDIndustryTileServices *industry_tiles;
};
extern const OpenTTDMovementServices _rust_movement_services;
const OpenTTDMovementServices &GetRustMovementServices() noexcept;
extern "C" {
uint16_t openttd_rust_road_trackdirs(const OpenTTDMovementServices *, uint32_t, RoadTramType);
uint8_t openttd_rust_movement_tunnel_visibility(uint8_t);
uint8_t openttd_rust_movement_enter(const OpenTTDMovementServices *, OpenTTDVehicle *, uint32_t, int32_t, int32_t);
int32_t openttd_rust_movement_depot_ticks(const OpenTTDMovementServices *, OpenTTDTrainHandle);
uint32_t openttd_rust_movement_track_status(const OpenTTDMovementServices *, uint32_t, TransportType, uint32_t, DiagDirection);
int32_t openttd_rust_movement_pixel_z(const OpenTTDMovementServices *, int32_t, int32_t, bool);
int32_t openttd_rust_movement_outside_z(const OpenTTDMovementServices *, int32_t, int32_t);
OpenTTDFoundationSlope openttd_rust_movement_foundation_slope(const OpenTTDMovementServices *, uint32_t);
Foundation openttd_rust_movement_foundation(const OpenTTDMovementServices *, uint32_t, Slope);
Foundation openttd_rust_rail_foundation(Slope, TrackBits, void (*)(bool) noexcept);
Foundation openttd_rust_road_foundation(Slope, RoadBits, void (*)(bool) noexcept);
}
#endif
