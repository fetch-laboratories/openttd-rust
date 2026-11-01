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
struct OpenTTDMovementFoundation { const void *spec; uint16_t kind; bool callback; };
struct OpenTTDMovementObjectFoundation { bool enabled; bool no_foundation; };
enum class WaterTileType : uint8_t;
struct OpenTTDTrainServices;
struct OpenTTDTunnelBridgeServices;
/* Only consumed entry-local owner identities/scalars; never a Vehicle/map view.
 * Shells survive the selected depot/admission/sound/viewport calls. No pointer
 * is retained after this synchronous entry and no Rust borrow crosses a call. */
struct OpenTTDMovementVehicle { OpenTTDTrainHandle train; OpenTTDRoadState *road; OpenTTDShipState *ship; VehicleType kind; };
struct OpenTTDMovementPose { int32_t z; uint8_t direction; };
struct OpenTTDMovementDepotPosition { uint32_t tile; int32_t x, y; };
struct OpenTTDMovementIncline { int32_t z; uint16_t flags; };
struct OpenTTDMovementFirstSpeed { OpenTTDVehicle *first; uint16_t speed; };
struct OpenTTDMovementServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDOrdersLeaves *orders;
	const OpenTTDRailServices *rail;
	const OpenTTDRoadGroundServices *road;
	const OpenTTDRailMapServices *rail_map;
	const OpenTTDTrainServices *train;
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDTunnelBridgeServices *tunnel_bridge;
	uint64_t (*road_compatible)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_compatible);
	uint16_t (*road_type_speed)(RoadType) noexcept OPENTTD_SERVICE_DEFAULT(road_type_speed);
	RoadTramType (*road_kind)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_kind);
	const OpenTTDRoadLeaves *road_vehicle;
	OpenTTDMovementVehicle (*resolve)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	OpenTTDMovementPose (*pose)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(pose);
	OpenTTDMovementDepotPosition (*depot_position)(OpenTTDTrainHandle) noexcept OPENTTD_SERVICE_DEFAULT(depot_position);
	OpenTTDMovementIncline (*incline)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(incline);
	OpenTTDVehicle *(*first)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first);
	OpenTTDMovementFirstSpeed (*first_speed)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_speed);
	void (*write_ground_flags)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(write_ground_flags);
	uint8_t (*train_engine_class)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(train_engine_class);
	void (*depot_invalidate)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(depot_invalidate);
	void (*native_assert)(bool) noexcept OPENTTD_SERVICE_DEFAULT(native_assert);
	bool (*plain_rail)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(plain_rail);
	bool (*is_tunnel)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_tunnel);
	bool (*rail_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_station);
	bool (*road_crossing)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_crossing);
	bool (*tile_flat)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_flat);
	uint32_t (*add_by_diag_dir)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(add_by_diag_dir);
	bool (*rail_depot_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_depot_tile);
	bool (*has_tile_road_type)(uint32_t, RoadTramType) noexcept OPENTTD_SERVICE_DEFAULT(has_tile_road_type);
	bool (*oneway_signal)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(oneway_signal);
	bool (*crossing_barred)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(crossing_barred);
	bool (*buoy)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(buoy);
	WaterTileType (*water_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_type);
	DiagDirection (*lock_direction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(lock_direction);
	Axis (*ship_depot_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_depot_axis);
	bool (*owned_land)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(owned_land);
	OpenTTDMovementObjectFoundation (*object_foundation)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(object_foundation);
	OpenTTDMovementFoundation (*house_foundation)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(house_foundation);
	bool (*house_boolean)(const void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_boolean);
	OpenTTDMovementFoundation (*industry_foundation)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_foundation);
	uint16_t (*industry_callback)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_callback);
	bool (*industry_boolean)(const void *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_boolean);
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
