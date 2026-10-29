/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_yapf_protocol.hpp Heap comparison and owner/lifetime ABI evidence. */
#ifndef SHIP_YAPF_PROTOCOL_HPP
#define SHIP_YAPF_PROTOCOL_HPP
#include "../rust/ship_yapf_ffi.h"
#include "follower_fixture.hpp"
#include <array>
#include <vector>
#include <memory>
#include <cassert>
#include "../misc/binaryheap.hpp"
namespace ShipYapfProbe {
inline uint32_t draws;
inline uint32_t Random(void *) noexcept { ++draws; return 0x9abcdef0; }
inline uint32_t destination;
inline bool snake, region_chain;
inline OpenTTDShipYapfDestination Destination(const void *) noexcept { return {.tile = destination, .station = false}; }
inline OpenTTDShipTile Tile(const void *, uint32_t, uint8_t) noexcept { return {0, 0, 1, 0, 0}; }
inline OpenTTDWaterPatch Patch(uint32_t tile) noexcept
{
	if (region_chain) return {static_cast<int32_t>(tile), 0, 1};
	return snake ? OpenTTDWaterPatch{0, 0, 1} : OpenTTDWaterPatch{static_cast<int32_t>(tile % 32 / 16), static_cast<int32_t>(tile / 32 / 16), 1};
}
struct Cursor { bool used = false; };
inline void *VisitNew(OpenTTDWaterPatch) noexcept { return new Cursor{}; }
inline uint8_t VisitNext(void *cursor, OpenTTDWaterPatch from, OpenTTDWaterPatch *out) noexcept
{
	auto &state = *static_cast<Cursor *>(cursor);
	if (!region_chain || state.used) return 0;
	state.used = true; *out = {from.x + 1, 0, 1}; return 1;
}
inline void VisitDestroy(void *cursor) noexcept { delete static_cast<Cursor *>(cursor); }
inline void Report(uint32_t, uint8_t, uint8_t, uint32_t, uint32_t, uint32_t, int32_t, int32_t) noexcept {}
inline uint32_t Industry(int32_t, int32_t, uint32_t *) noexcept { return 0; }

/* Save snapshots cannot distinguish every equal-priority heap shape. Compare
	* insertion, middle removal and strict improvement directly to unchanged C++.
	* This is only heap evidence; world/game semantics remain the ships corpus. */
template <typename Check> void Run(Check check, uint32_t limit_follows)
{
	struct Node { int32_t estimate; bool operator<(const Node &other) const { return this->estimate < other.estimate; } };
	std::array<Node, 64> nodes{};
	std::array<int32_t, 64> costs{};
	std::array<bool, 64> included{};
	CBinaryHeapT<Node> heap{2048};
	std::vector<uint32_t> commands, expected;
	std::vector<int32_t> values;
	uint32_t random = 17;
	for (size_t step = 0; step < 2000; ++step) {
		random = random * 1664525 + 1013904223;
		const size_t id = (random >> 16) % nodes.size();
		const uint32_t operation = !included[id] ? 0 : (random & 1) ? 1 : 2;
		const int32_t cost = static_cast<int32_t>((random >> 8) % 7);
		commands.push_back((operation << 30) | static_cast<uint32_t>(id)); values.push_back(cost);
		if (operation == 0) { heap.Include(&nodes[id]); included[id] = true; }
		else if (operation == 1) { heap.Remove(heap.FindIndex(nodes[id])); included[id] = false; }
		else { heap.Remove(heap.FindIndex(nodes[id])); nodes[id].estimate = cost; heap.Include(&nodes[id]); }
		expected.push_back(heap.IsEmpty() ? UINT32_MAX : static_cast<uint32_t>(heap.Begin() - nodes.data()));
	}
	std::vector<uint32_t> actual(commands.size());
	openttd_rust_ship_heap_probe(costs.data(), costs.size(), commands.data(), values.data(), commands.size(), actual.data());
	check(actual == expected);
	using PathOwner = std::unique_ptr<OpenTTDShipPath, decltype(&openttd_rust_ship_path_destroy)>;
	PathOwner path{openttd_rust_ship_path_new(), openttd_rust_ship_path_destroy};
	openttd_rust_ship_path_push(path.get(), 255); openttd_rust_ship_path_push(path.get(), 13);
	PathOwner copy{openttd_rust_ship_path_clone(path.get()), openttd_rust_ship_path_destroy};
	openttd_rust_ship_path_set(copy.get(), 0, 2); openttd_rust_ship_path_pop(copy.get());
	check(openttd_rust_ship_path_size(copy.get()) == 1 && openttd_rust_ship_path_get(copy.get(), 0) == 2);
	check(openttd_rust_ship_path_size(path.get()) == 2 && openttd_rust_ship_path_get(path.get(), 0) == 255 && openttd_rust_ship_path_get(path.get(), 1) == 13);
	openttd_rust_ship_path_clear(path.get());
	OpenTTDShipYapfInput input{32, 32, 10 + 32 * 20, 10 + 32 * 24, 100, 20, 100, 0, 1 << 9, 1, 0, 0, 0, 0};
	const OpenTTDShipYapfLeaves leaves{Destination, Tile, Patch, VisitNew, VisitNext, VisitDestroy, Report};
	FollowerFixture map{32, 32};
	auto &services = map.shared;
	services.random = Random;
	for (uint32_t t = 0; t < map.base.size(); ++t) map.Tile(t, MP_WATER);
	destination = input.dest_tile;
	auto result = openttd_rust_ship_choose(path.get(), &input, &leaves, &services, nullptr, input.tile, 1 << 1, 0, &destination, 1);
	check(result.found == 1 && result.direction == 1 && result.origin == 1 && openttd_rust_ship_path_size(path.get()) == 0 && draws == 0);
	destination = input.tile - 4 * 32;
	result = openttd_rust_ship_reverse(&input, &leaves, &services, nullptr, 0, &destination, 1);
	check(result.found == 1 && result.origin == 9 && draws == 0);
	result = openttd_rust_ship_reverse(&input, &leaves, &services, nullptr, 1, &destination, 1);
	check(result.found == 1 && result.direction == 9 && draws == 0);
	using RegionOwner = std::unique_ptr<OpenTTDShipRegionPath, decltype(&openttd_rust_ship_regions_destroy)>;
	RegionOwner regions{openttd_rust_ship_regions(&input, &leaves, input.tile, 5, &destination, 1), openttd_rust_ship_regions_destroy};
	check(openttd_rust_ship_regions_size(regions.get()) == 1 && openttd_rust_ship_regions_get(regions.get(), 0).label == 1);
	/* A one-tile-wide winding river exceeds the low-level node limit. All
	 * transitions now enter the production follower over canonical tile layouts.
	 * Region reporting remains injected to isolate low-level retry control. */
	snake = true; draws = 0;
	map.width = 128; map.base.assign(128 * 128, {}); map.extended.assign(128 * 128, {});
	for (uint32_t y = 1; y < 127; y += 2) {
		for (uint32_t x = 1; x <= 126; ++x) map.Tile(x + y * 128, MP_WATER);
		if (y < 125) map.Tile(((y / 2 & 1) ? 1 : 126) + (y + 1) * 128, MP_WATER);
	}
	input = {128, 128, 1 + 128, 1 + 128 * 101, 100, 20, 100, 0, 0, 8, 0, 0, 0, 0};
	destination = input.dest_tile;
	result = openttd_rust_ship_choose(path.get(), &input, &leaves, &services, nullptr, input.tile, 1 << 8, 0, &destination, 1);
	check(result.found == 0 && result.stats[3] == 1 && result.stats[2] == 1 && result.stats[7] == 1);
	/* The long winding route reaches the same fixed limit as the unchanged
	 * generic search. The restricted retry additionally searches its start patch. */
	check(result.stats[1] >= limit_follows && result.stats[1] <= limit_follows + 256);
	check(result.stats[8] > 0 && result.stats[8] <= 8 && draws == result.stats[8]);
	snake = false;
	/* Map-derived high-level node limit, with a synthetic one-edge region graph. */
	region_chain = true; draws = 0;
	for (uint32_t t = 0; t < map.base.size(); ++t) map.Tile(t, MP_WATER);
	input = {128, 128, 600, 0, 100, 20, 100, 0, 0, 1, 0, 0, 0, 0};
	destination = 0; openttd_rust_ship_path_clear(path.get());
	result = openttd_rust_ship_choose(path.get(), &input, &leaves, &services, nullptr, input.tile, 1 << 1, 0, &destination, 1);
	check(result.found == 0 && result.stats[0] == 257 && result.stats[11] == 1 && result.stats[1] == 0);
	check(result.stats[7] == 1 && result.stats[8] == 8 && draws == 8);
	region_chain = false;
}
}
#endif
