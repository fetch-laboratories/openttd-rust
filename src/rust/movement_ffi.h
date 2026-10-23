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
struct OpenTTDMovementFoundation { const void *spec; uint16_t kind; bool callback; };
struct OpenTTDMovementObjectFoundation { bool enabled; bool no_foundation; };
struct OpenTTDMovementServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDRailServices *rail;
	const OpenTTDRoadGroundServices *road;
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
int32_t openttd_rust_movement_pixel_z(const OpenTTDMovementServices *, int32_t, int32_t, bool);
int32_t openttd_rust_movement_outside_z(const OpenTTDMovementServices *, int32_t, int32_t);
OpenTTDFoundationSlope openttd_rust_movement_foundation_slope(const OpenTTDMovementServices *, uint32_t);
Foundation openttd_rust_movement_foundation(const OpenTTDMovementServices *, uint32_t, Slope);
Foundation openttd_rust_rail_foundation(Slope, TrackBits);
Foundation openttd_rust_road_foundation(Slope, RoadBits);
}
#endif
