/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_protocol.hpp Default real enum/record ABI calls, also on Windows. */
#ifndef TESTS_GROUND_PROTOCOL_HPP
#define TESTS_GROUND_PROTOCOL_HPP
#include "../rust/ground_ffi.h"
#include "../core/enum_type.hpp"
namespace GroundProbe {
static inline uint32_t marker;
inline bool Run()
{
	const OpenTTDGroundABICalls calls{
		.fences = []() noexcept { return OpenTTDFieldFences{.ne = 0xFEDCBA98, .se = 0x89ABCDEF, .sw = 0x76543210, .nw = 0x12345678}; },
		.surface = []() noexcept { return OpenTTDClearSurface{.ground = static_cast<ClearGround>(240), .density = 0xCD, .snow = true}; },
		.field = []() noexcept { return OpenTTDFieldGround{.industry = 0xFEDC, .type = 0x98765432}; },
		.rail_snow = []() noexcept { return OpenTTDRailSnowGround{.z = -42, .slope = static_cast<Slope>(238), .tracks = static_cast<TrackBits>(237), .plain = true}; },
		.rail_fence = []() noexcept { return OpenTTDRailFenceGround{.tracks = static_cast<TrackBits>(236), .owner = 235, .plain = false}; },
		.foundation = []() noexcept { return OpenTTDFoundationSlope{.z = 123456, .slope = static_cast<Slope>(234)}; },
		.road_snow = []() noexcept { return OpenTTDRoadSnowGround{.z = -123456, .snow = true}; },
		.tunnel = []() noexcept { return OpenTTDTunnelGround{.z = 789, .snow = false}; },
		.airport = []() noexcept { return OpenTTDAirportGround{.station = nullptr, .tile = 0x12345678, .width = 0xABCD, .height = 0xDCBA}; },
		.victim = []() noexcept { return OpenTTDFloodVictim{.status = 233, .type = static_cast<VehicleType>(226)}; },
		.aircraft = []() noexcept { return OpenTTDAircraftFloodPose{.tile = 0x87654321, .z = -789, .subtype = 232}; },
		/* Marker is only an opaque nonnull pointer value; neither language dereferences it. */
		.ground = []() noexcept { return OpenTTDGroundFloodPose{.z = 777, .first = reinterpret_cast<OpenTTDVehicle *>(&marker)}; },
		.enums = [](ClearGround c, RailGroundType r, Slope s, TrackBits b, Track t, Foundation f, Roadside side, RoadBits rb, RoadType rt, TreeGround tree, WaterClass water, FloodingBehaviour flood, DiagDirection d, LandscapeType l, VehicleType v) noexcept {
			return to_underlying(c) == 240 && to_underlying(r) == 239 && to_underlying(s) == 238 && to_underlying(b) == 237 && to_underlying(t) == 227 && to_underlying(f) == 228 && to_underlying(side) == 229 && to_underlying(rb) == 230 && to_underlying(rt) == 231 && to_underlying(tree) == 232 && to_underlying(water) == 233 && to_underlying(flood) == 234 && to_underlying(d) == 235 && to_underlying(l) == 236 && to_underlying(v) == 226;
		},
	};
	return openttd_rust_ground_abi_calls(&calls);
}
} // namespace GroundProbe
#endif /* TESTS_GROUND_PROTOCOL_HPP */
