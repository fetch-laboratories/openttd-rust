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
struct OpenTTDMovementFoundation { const void *spec; uint16_t kind; bool callback; };
struct OpenTTDMovementObjectFoundation { bool enabled; bool no_foundation; };
enum class WaterTileType : uint8_t;
struct OpenTTDTrainServices;
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
	const OpenTTDRailServices *rail;
	const OpenTTDRoadGroundServices *road;
	const OpenTTDRailMapServices *rail_map;
	const OpenTTDTrainServices *train;
	const OpenTTDVehicleServices *vehicle;
	const OpenTTDRoadLeaves *road_vehicle;
	OpenTTDMovementVehicle (*resolve)(OpenTTDVehicle *) noexcept;
	OpenTTDMovementPose (*pose)(OpenTTDVehicle *) noexcept;
	OpenTTDMovementDepotPosition (*depot_position)(OpenTTDTrainHandle) noexcept;
	OpenTTDMovementIncline (*incline)(OpenTTDVehicle *) noexcept;
	OpenTTDVehicle *(*first)(OpenTTDVehicle *) noexcept;
	OpenTTDMovementFirstSpeed (*first_speed)(OpenTTDVehicle *) noexcept;
	void (*write_ground_flags)(OpenTTDVehicle *, uint16_t) noexcept;
	bool (*front_engine)(OpenTTDVehicle *) noexcept;
	uint8_t (*train_engine_class)(OpenTTDVehicle *) noexcept;
	void (*depot_invalidate)(OpenTTDVehicle *) noexcept;
	void (*native_assert)(bool) noexcept;
	bool (*rail_depot_tile)(uint32_t) noexcept;
	bool (*has_tile_road_type)(uint32_t, RoadTramType) noexcept;
	bool (*oneway_signal)(uint32_t, uint8_t) noexcept;
	bool (*crossing_barred)(uint32_t) noexcept;
	bool (*buoy)(uint32_t) noexcept;
	WaterTileType (*water_type)(uint32_t) noexcept;
	DiagDirection (*lock_direction)(uint32_t) noexcept;
	Axis (*ship_depot_axis)(uint32_t) noexcept;
	OpenTTDFoundationSlope (*pixel_slope)(uint32_t) noexcept;
	OpenTTDFoundationSlope (*outside_pixel_slope)(uint32_t, uint32_t) noexcept;
	int32_t (*max_pixel_z)(uint32_t) noexcept;
	OpenTTDFoundationSlope (*apply_foundation)(Foundation, Slope) noexcept;
	bool (*owned_land)(uint32_t) noexcept;
	OpenTTDMovementObjectFoundation (*object_foundation)(uint32_t) noexcept;
	OpenTTDMovementFoundation (*house_foundation)(uint32_t) noexcept;
	uint16_t (*house_callback)(uint32_t, uint16_t) noexcept;
	bool (*house_boolean)(const void *, uint16_t) noexcept;
	OpenTTDMovementFoundation (*industry_foundation)(uint32_t) noexcept;
	uint16_t (*industry_callback)(uint32_t, uint16_t) noexcept;
	bool (*industry_boolean)(const void *, uint16_t) noexcept;
};
const OpenTTDMovementServices &GetRustMovementServices() noexcept;
extern "C" {
uint8_t openttd_rust_movement_enter(const OpenTTDMovementServices *, OpenTTDVehicle *, uint32_t, int32_t, int32_t);
int32_t openttd_rust_movement_depot_ticks(const OpenTTDMovementServices *, OpenTTDTrainHandle);
uint32_t openttd_rust_movement_track_status(const OpenTTDMovementServices *, uint32_t, TransportType, uint32_t, DiagDirection);
int32_t openttd_rust_movement_pixel_z(const OpenTTDMovementServices *, int32_t, int32_t, bool);
int32_t openttd_rust_movement_outside_z(const OpenTTDMovementServices *, int32_t, int32_t);
OpenTTDFoundationSlope openttd_rust_movement_foundation_slope(const OpenTTDMovementServices *, uint32_t);
Foundation openttd_rust_movement_foundation(const OpenTTDMovementServices *, uint32_t, Slope);
Foundation openttd_rust_rail_foundation(Slope, TrackBits);
Foundation openttd_rust_road_foundation(Slope, RoadBits);
}
#endif
