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
#include "follower_fixture.hpp"
#include <algorithm>
#include <array>
#include <memory>
#include <vector>

namespace WaterProbe {
inline std::array<bool, 32 * 32> water;
inline uint8_t WaterType(uint32_t tile) noexcept
{
	/* The follower queries past this fixture's map; its Map::MaxX column is void, as generated. */
	if (tile >= water.size() || tile % 32 == 31) return MP_VOID;
	return water[tile] ? MP_WATER : MP_CLEAR;
}

/* These callbacks deliberately mutate a warmed cache between next calls, a gap
 * the ship AI corpus cannot reach: YAPF visitors do not execute world commands. */
template <typename Check> void Run(Check check)
{
	/* The cache's 32 by 32 map has a readable void southern guard for follower probes. */
	FollowerFixture map{32, 64};
	for (uint32_t tile = static_cast<uint32_t>(water.size()); tile < map.tiles.size; ++tile) map.Tile(tile, MP_VOID);
	auto &world = map.shared;
	world.tile_type = WaterType;
	const auto fill = [&](bool value) { water.fill(value); for (uint32_t t = 0; t < water.size(); ++t) map.Tile(t, t % 32 == 31 ? MP_VOID : value ? MP_WATER : MP_CLEAR); };
	/* Each tile of the north-west region, in its local index order. */
	const auto region_labels = [&](OpenTTDWaterRegions *owner) {
		std::array<uint8_t, 256> labels{};
		for (uint32_t local = 0; local < labels.size(); ++local) labels[local] = openttd_rust_water_label(owner, &world, local % 16 + local / 16 * 32);
		return labels;
	};
	using Owner = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>;
	Owner owner{openttd_rust_water_new(&world, 32, 32), openttd_rust_water_destroy};
	fill(false);
	check(openttd_rust_water_label(owner.get(), &world, 0) == 0);
	fill(true);
	openttd_rust_water_invalidate(owner.get(), &world, 0);
	check(openttd_rust_water_label(owner.get(), &world, 0) == 1);
	/* A warm region keeps its label without reading the map again until invalidated. */
	fill(false);
	check(openttd_rust_water_label(owner.get(), &world, 0) == 1);
	fill(true);
	/* One patch: every tile has the first label. */
	const auto single = region_labels(owner.get());
	check(std::all_of(single.begin(), single.end(), [](auto label) { return label == 1; }));
	/* Both southern edges are traversable: a whole visit reaches the SE and then the SW neighbour. */
	OpenTTDWaterPatch next{};
	OpenTTDWaterVisit whole{};
	check(openttd_rust_water_visit_next(owner.get(), &world, &whole, {0, 0, 1}, &next) && next.x == 0 && next.y == 1 && next.label == 1);
	check(openttd_rust_water_visit_next(owner.get(), &world, &whole, {0, 0, 1}, &next) && next.x == 1 && next.y == 0 && next.label == 1);
	check(openttd_rust_water_visit_next(owner.get(), &world, &whole, {0, 0, 1}, &next) == 0);
	OpenTTDWaterVisit visit{};
	check(openttd_rust_water_visit_next(owner.get(), &world, &visit, {0, 0, 1}, &next) && next.x == 0 && next.y == 1);
	/* Const-reference arguments may be mutated through a caller-owned alias. */
	OpenTTDWaterVisit aliased{};
	check(openttd_rust_water_visit_next(owner.get(), &world, &aliased, {0, 0, 1}, &next) && next.x == 0 && next.y == 1);
	check(openttd_rust_water_visit_next(owner.get(), &world, &aliased, {0, 0, 0}, &next) == 0);
	/* Reentrant visitor removes the east connection. The next side must refresh. */
	for (uint32_t y = 0; y < 16; y++) {
		water[16 + y * 32] = false; map.Tile(16 + y * 32, MP_CLEAR);
		openttd_rust_water_invalidate(owner.get(), &world, 16 + y * 32);
	}
	check(openttd_rust_water_visit_next(owner.get(), &world, &visit, {0, 0, 1}, &next) == 0);
	fill(false);
	water[0] = true; water[2] = true; map.Tile(0, MP_WATER); map.Tile(2, MP_WATER);
	openttd_rust_water_invalidate(owner.get(), &world, 0);
	/* Two patches: the two water tiles take labels 1 and 2, every land tile none. */
	const auto pair = region_labels(owner.get());
	check(pair[0] == 1 && pair[2] == 2 && std::all_of(pair.begin() + 3, pair.end(), [](auto label) { return label == 0; }) && pair[1] == 0);
	/* Replacement map gets fresh private validity and labels. */
	owner.reset(openttd_rust_water_new(&world, 32, 32));
	fill(false);
	check(openttd_rust_water_label(owner.get(), &world, 0) == 0);
}
}
#endif /* WATER_REGIONS_PROTOCOL_HPP */
