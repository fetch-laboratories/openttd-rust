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
#include "../rust/movement_ffi.h"
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
inline OpenTTDShipFollow Follow(const void *, uint32_t tile, uint8_t td) noexcept
{
	if (!snake) return {static_cast<uint32_t>(tile + (td == 1 ? 32 : -32)), 0, static_cast<uint16_t>(1 << td), 1};
	const uint32_t next = tile + (td == 8 ? 1 : td == 0 ? -1 : 128);
	const uint32_t x = next % 128, y = next / 128;
	const uint8_t direction = (y & 1) ? (x == 126 ? 1 : 8) : (x == 1 ? 1 : 0);
	return {next, 0, static_cast<uint16_t>(1 << direction), 1};
}
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
	const OpenTTDShipYapfLeaves leaves{Destination, Follow, Tile, Patch, VisitNew, VisitNext, VisitDestroy, Report};
	OpenTTDSharedServices services = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
	static OpenTTDSharedServices world{};
	world.tile_type = [](uint32_t) noexcept -> uint8_t { return 9; };
	world.tunnel_transport = [](uint32_t) noexcept -> uint8_t { return 2; };
	world.tunnel_bridge_direction = [](uint32_t) noexcept -> uint8_t { return 1; };
	static OpenTTDMovementServices movement{};
	movement.shared = &world;
	services.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
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
	/* Injected single-successor track graph isolates fixed-limit/retry control.
	 * The patch query intentionally reports one patch for all graph vertices;
	 * this is boundary/algorithm evidence, not a reachable-map simulation claim. */
	snake = true; draws = 0;
	input = {128, 128, 1 + 128, 1 + 128 * 90, 100, 20, 100, 0, 0, 8, 0, 0, 0, 0};
	destination = input.dest_tile;
	result = openttd_rust_ship_choose(path.get(), &input, &leaves, &services, nullptr, input.tile, 1 << 8, 0, &destination, 1);
	check(result.found == 0 && result.stats[3] == 1 && result.stats[2] == 1 && result.stats[7] == 1);
	/* The fixed-limit search follows as often as the unchanged reference search; the
	 * restricted retry then follows the 15 tiles left in the 16-tile start region. */
	check(result.stats[1] == limit_follows + 15 && result.stats[8] == 8 && draws == 8 && openttd_rust_ship_path_size(path.get()) == 7);
	snake = false;
	/* Map-derived high-level node limit, with a synthetic one-edge region graph. */
	region_chain = true; draws = 0;
	input = {128, 128, 600, 0, 100, 20, 100, 0, 0, 1, 0, 0, 0, 0};
	destination = 0; openttd_rust_ship_path_clear(path.get());
	result = openttd_rust_ship_choose(path.get(), &input, &leaves, &services, nullptr, input.tile, 1 << 1, 0, &destination, 1);
	check(result.found == 0 && result.stats[0] == 257 && result.stats[11] == 1 && result.stats[1] == 0);
	check(result.stats[7] == 1 && result.stats[8] == 8 && draws == 8);
	region_chain = false;
}
}
#endif
