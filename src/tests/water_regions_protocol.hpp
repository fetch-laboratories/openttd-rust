/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file water_regions_protocol.hpp Native owner/visitor lifetime and reentry probe. */
#ifndef WATER_REGIONS_PROTOCOL_HPP
#define WATER_REGIONS_PROTOCOL_HPP
#include "../rust/water_regions_ffi.h"
#include "../rust/movement_ffi.h"
#include <algorithm>
#include <array>
#include <memory>
#include <vector>

namespace WaterProbe {
inline std::array<bool, 32 * 32> water;
inline uint32_t tracks_calls, follow_calls;
inline uint8_t WaterType(uint32_t tile) noexcept { ++tracks_calls; return water[tile] ? 6 : 0; }
inline uint32_t OPENTTD_RUST_CALL WaterProbeFollow(uint32_t tile, uint8_t dir, uint8_t *bridge) noexcept
{
	++follow_calls;
	*bridge = 0;
	/* Synthetic four-neighbour graph, accepting the real movement trackdirs. */
	constexpr uint8_t exit[]{0, 1, 0, 1, 2, 1, 0, 0, 2, 3, 3, 2, 3, 0};
	dir = exit[dir];
	const int x = tile % 32, y = tile / 32;
	constexpr int dx[] = {-1, 0, 1, 0}, dy[] = {0, 1, 0, -1};
	const int nx = x + dx[dir], ny = y + dy[dir];
	if (nx < 0 || ny < 0 || nx >= 32 || ny >= 32 || !water[nx + ny * 32]) return UINT32_MAX;
	return nx + ny * 32;
}
inline uint32_t OPENTTD_RUST_CALL Aqueduct(uint32_t) noexcept { return UINT32_MAX; }
inline void OPENTTD_RUST_CALL WaterProbeDebug(uint8_t, int32_t, int32_t) noexcept {}

/* These callbacks deliberately mutate a warmed cache between next calls, a gap
 * the ship AI corpus cannot reach: YAPF visitors do not execute world commands. */
template <typename Check> void Run(Check check)
{
	static OpenTTDSharedServices world{};
	world.tile_type = WaterType;
	world.map_log_x = []() noexcept -> uint32_t { return 5; };
	static OpenTTDMovementServices movement{};
	movement.shared = &world;
	movement.water_type = [](uint32_t) noexcept { return static_cast<WaterTileType>(0); };
	movement.tile_flat = [](uint32_t) noexcept { return true; };
	world.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	const OpenTTDWaterLeaves leaves{.shared = &world, .follow = WaterProbeFollow, .aqueduct = Aqueduct, .debug = WaterProbeDebug};
	using Owner = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>;
	using Cursor = std::unique_ptr<OpenTTDWaterVisit, decltype(&openttd_rust_water_visit_destroy)>;
	Owner owner{openttd_rust_water_new(32, 32), openttd_rust_water_destroy};
	water.fill(false);
	check(openttd_rust_water_label(owner.get(), &leaves, 0) == 0);
	water.fill(true);
	openttd_rust_water_invalidate(owner.get(), &leaves, 0);
	check(openttd_rust_water_label(owner.get(), &leaves, 0) == 1);
	const auto queries = tracks_calls;
	check(openttd_rust_water_label(owner.get(), &leaves, 0) == 1 && tracks_calls == queries);
	OpenTTDWaterSnapshot snapshot{};
	openttd_rust_water_snapshot(owner.get(), &leaves, 0, &snapshot);
	check(snapshot.patches == 1 && snapshot.edges[1] == UINT16_MAX && snapshot.edges[2] == UINT16_MAX);
	check(std::all_of(std::begin(snapshot.labels), std::end(snapshot.labels), [](auto label) { return label == 1; }));
	Cursor visit{openttd_rust_water_visit_new(owner.get(), &leaves, {0, 0, 1}), openttd_rust_water_visit_destroy};
	OpenTTDWaterPatch next{};
	check(openttd_rust_water_visit_next(owner.get(), &leaves, visit.get(), {0, 0, 1}, &next) && next.x == 0 && next.y == 1);
	/* Const-reference arguments may be mutated through a caller-owned alias. */
	Cursor aliased{openttd_rust_water_visit_new(owner.get(), &leaves, {0, 0, 1}), openttd_rust_water_visit_destroy};
	check(openttd_rust_water_visit_next(owner.get(), &leaves, aliased.get(), {0, 0, 1}, &next) && next.x == 0 && next.y == 1);
	check(openttd_rust_water_visit_next(owner.get(), &leaves, aliased.get(), {0, 0, 0}, &next) == 0);
	aliased.reset();
	/* Reentrant visitor removes the east connection. The next side must refresh. */
	for (uint32_t y = 0; y < 16; y++) {
		water[16 + y * 32] = false;
		openttd_rust_water_invalidate(owner.get(), &leaves, 16 + y * 32);
	}
	check(openttd_rust_water_visit_next(owner.get(), &leaves, visit.get(), {0, 0, 1}, &next) == 0);
	visit.reset(); // Same cleanup used when a C++ visitor throws.
	water.fill(false);
	water[0] = true; water[2] = true;
	openttd_rust_water_invalidate(owner.get(), &leaves, 0);
	openttd_rust_water_snapshot(owner.get(), &leaves, 0, &snapshot);
	check(snapshot.patches == 2 && snapshot.labels[0] == 1 && snapshot.labels[2] == 2 && snapshot.labels[1] == 0);
	/* Replacement map gets fresh private validity and labels. */
	owner.reset(openttd_rust_water_new(32, 32));
	water.fill(false);
	check(openttd_rust_water_label(owner.get(), &leaves, 0) == 0);
}
}
#endif /* WATER_REGIONS_PROTOCOL_HPP */
