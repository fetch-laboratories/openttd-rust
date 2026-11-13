/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_yapf_protocol.hpp Heap comparison, path ownership and production ship searches over fixture maps. */
#ifndef SHIP_YAPF_PROTOCOL_HPP
#define SHIP_YAPF_PROTOCOL_HPP
#include "../rust/ship_yapf_ffi.h"
#include "../rust/ship_control_ffi.h"
#include "../rust/ground_ffi.h"
#include "../rust/engine_capabilities_ffi.h"
#include "follower_fixture.hpp"
#include <array>
#include <string_view>
#include <vector>
#include <memory>
#include <cassert>
#include "../misc/binaryheap.hpp"
extern "C" {
void openttd_rust_witness_enable();
const char *openttd_rust_witness_name(size_t, size_t *);
uint64_t openttd_rust_witness_count(size_t);
}
namespace ShipYapfProbe {
inline uint32_t draws;
inline uint32_t Random(void *) noexcept { ++draws; return 0x9abcdef0; }
inline OpenTTDWaterRegions *regions;
inline OpenTTDWaterGroundServices water{.water_regions = []() noexcept { return regions; }};

/** The current count of the branch witness `ship_yapf.<name>`. */
inline uint64_t Witness(std::string_view name)
{
	size_t length;
	for (size_t i = 0; const char *witness = openttd_rust_witness_name(i, &length); ++i) {
		if (std::string_view(witness, length).substr(0, 10) == "ship_yapf." && std::string_view(witness, length).substr(10) == name) return openttd_rust_witness_count(i);
	}
	std::abort();
}
/** The ship_yapf witnesses one search hits, as the differences around it. */
struct Stats {
	static constexpr std::array<std::string_view, 7> names{"region_nodes", "track_nodes", "retries", "track_limits", "lost", "random_draws", "region_limits"};
	std::array<uint64_t, names.size()> before;
	Stats() { for (size_t i = 0; i < names.size(); ++i) this->before[i] = Witness(names[i]); }
	uint64_t operator[](std::string_view name) const
	{
		for (size_t i = 0; i < names.size(); ++i) if (names[i] == name) return Witness(name) - this->before[i];
		std::abort();
	}
};

/** One-tile-wide river rows on every odd y below `rows`, from x = 1 to width - 2, each joined to the next at alternating ends. */
inline void CarveRiver(FollowerFixture &map, uint32_t width, uint32_t rows)
{
	for (uint32_t t = 0; t < map.tiles.size; ++t) map.Tile(t, MP_CLEAR);
	for (uint32_t y = 1; y < rows; y += 2) {
		for (uint32_t x = 1; x <= width - 2; ++x) map.Tile(x + y * width, MP_WATER);
		if (y < rows - 2) map.Tile(((y / 2 & 1) ? 1 : width - 2) + (y + 1) * width, MP_WATER);
	}
}

/* Save snapshots cannot distinguish every equal-priority heap shape. Compare
 * insertion, middle removal and strict improvement directly to unchanged C++.
 * This is only heap evidence; world/game semantics remain the ships corpus. */
template <typename Check> void Run(Check check, uint32_t limit_follows)
{
	openttd_rust_witness_enable();
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
	openttd_test_ship_heap_probe(costs.data(), costs.size(), commands.data(), values.data(), commands.size(), actual.data());
	check(actual == expected);
	using PathOwner = std::unique_ptr<OpenTTDShipPath, decltype(&openttd_rust_ship_path_destroy)>;
	PathOwner path{openttd_rust_ship_path_new(), openttd_rust_ship_path_destroy};
	openttd_rust_ship_path_push(path.get(), 255); openttd_rust_ship_path_push(path.get(), 13);
	PathOwner copy{openttd_rust_ship_path_clone(path.get()), openttd_rust_ship_path_destroy};
	openttd_rust_ship_path_set(copy.get(), 0, 2); openttd_rust_ship_path_pop(copy.get());
	check(openttd_rust_ship_path_size(copy.get()) == 1 && openttd_rust_ship_path_get(copy.get(), 0) == 2);
	check(openttd_rust_ship_path_size(path.get()) == 2 && openttd_rust_ship_path_get(path.get(), 0) == 255 && openttd_rust_ship_path_get(path.get(), 1) == 13);
	openttd_rust_ship_path_clear(path.get());

	/* The searches read the map, a real water region owner and the probe's input. The controller's
	 * leaves serve only station orders, which no case gives. */
	static const OpenTTDEngineCapabilitiesServices capabilities{};
	OpenTTDShipLeaves leaves{};
	leaves.capabilities = &capabilities;
	using RegionOwner = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>;
	RegionOwner owner{nullptr, openttd_rust_water_destroy};
	const auto install = [&](FollowerFixture &map, uint32_t width, uint32_t height) {
		map.shared.random = Random;
		/* No fixture tile is a lock. */
		map.shared.water_middle_lock = [](uint32_t) noexcept { return false; };
		map.shared.water_services = []() noexcept -> const OpenTTDWaterGroundServices * { return &water; };
		owner.reset(openttd_rust_water_new(&map.shared, width, height));
		regions = owner.get();
	};

	/* A 32 by 32 sea: a ship on (10, 20) heading south-east. */
	{
		FollowerFixture map{32, 32};
		for (uint32_t t = 0; t < map.tiles.size; ++t) map.Tile(t, MP_WATER);
		install(map, 32, 32);
		draws = 0;
		OpenTTDShipYapfProbe probe{};
		probe.input = {.tile = 10 + 32 * 20, .dest_tile = 10 + 32 * 24, .direction = DIR_SE, .max_speed = 100, .ship_curve45_penalty = 20, .ship_curve90_penalty = 100};
		probe.state = TRACK_BIT_Y;
		probe.trackdir = TRACKDIR_Y_SE;
		auto direction = openttd_test_ship_choose_ship_track(&map.shared, &leaves, &probe, probe.input.tile, 1 << TRACKDIR_Y_SE, 0, path.get());
		check(probe.path_found == 1 && direction == TRACKDIR_Y_SE && probe.best_origin_dir == TRACKDIR_Y_SE && openttd_rust_ship_path_size(path.get()) == 0 && draws == 0);
		/* The destination four tiles behind: the reverse origin is the better one. */
		probe.input.dest_tile = probe.input.tile - 4 * 32;
		check(openttd_test_ship_check_ship_reverse(&map.shared, &leaves, &probe, nullptr) == 1 && draws == 0);
		/* A blocked ship may reverse into Y_NW, LOWER_W or RIGHT_N. The water beside it is land,
		 * so that only Y_NW leads anywhere, as the reverse trackdir the case observes. */
		for (uint32_t t : {probe.input.tile - 1, probe.input.tile + 1}) {
			map.Tile(t, MP_CLEAR);
			openttd_rust_water_invalidate(owner.get(), &map.shared, t);
		}
		uint8_t trackdir = INVALID_TRACKDIR;
		check(openttd_test_ship_check_ship_reverse(&map.shared, &leaves, &probe, &trackdir) == 1 && trackdir == TRACKDIR_Y_NW && draws == 0);
		std::array<OpenTTDWaterPatch, 5> patches{};
		check(openttd_test_ship_find_water_region_path(&map.shared, &leaves, &probe, probe.input.tile, 5, patches.data(), patches.size()) == 1 && patches[0].label == 1);
	}

	/* A one-tile-wide winding river exceeds the low-level node limit, then a failed retry. The
	 * region cache is stale on purpose: each region was queried while the map was open sea and
	 * the river was carved without invalidating it, which production never does. The high-level
	 * path is then five regions straight south from the ship, the unrestricted search follows the
	 * river past the limit, and the restricted retry is shut inside a corridor that the river
	 * leaves at once. The case observes that retry's failure, which a consistent cache cannot give. */
	{
		FollowerFixture map{256, 256};
		for (uint32_t t = 0; t < map.tiles.size; ++t) map.Tile(t, MP_WATER);
		install(map, 256, 256);
		for (uint32_t y = 0; y < 256; y += 16) {
			for (uint32_t x = 0; x < 256; x += 16) check(openttd_rust_water_label(owner.get(), &map.shared, x + y * 256) == 1);
		}
		CarveRiver(map, 256, 255);
		draws = 0;
		OpenTTDShipYapfProbe probe{};
		probe.input = {.tile = 1 + 256, .dest_tile = 1 + 256 * 101, .direction = DIR_SW, .max_speed = 100, .ship_curve45_penalty = 20, .ship_curve90_penalty = 100};
		probe.state = TRACK_BIT_X;
		probe.trackdir = TRACKDIR_X_SW;
		openttd_rust_ship_path_clear(path.get());
		const Stats stats;
		openttd_test_ship_choose_ship_track(&map.shared, &leaves, &probe, probe.input.tile, 1 << TRACKDIR_X_SW, 0, path.get());
		check(probe.path_found == 0 && stats["track_limits"] == 1 && stats["retries"] == 1 && stats["lost"] == 1);
		/* The long winding route reaches the same fixed limit as the unchanged
		 * generic search. The restricted retry additionally searches its start patch. */
		check(stats["track_nodes"] >= limit_follows && stats["track_nodes"] <= limit_follows + 256);
		check(stats["random_draws"] > 0 && stats["random_draws"] <= 8 && draws == stats["random_draws"]);
	}

	/* Map-derived high-level node limit: on 128 by 128 tiles the river is one chain of more than
	 * 256 region patches. The destination is at its head and the ship at its tail, with eight
	 * tiles of river ahead. */
	{
		FollowerFixture map{128, 128};
		CarveRiver(map, 128, 127);
		install(map, 128, 128);
		draws = 0;
		OpenTTDShipYapfProbe probe{};
		probe.input = {.tile = 118 + 128 * 125, .dest_tile = 1 + 128, .direction = DIR_SW, .max_speed = 100, .ship_curve45_penalty = 20, .ship_curve90_penalty = 100};
		probe.state = TRACK_BIT_X;
		probe.trackdir = TRACKDIR_X_SW;
		openttd_rust_ship_path_clear(path.get());
		const Stats stats;
		openttd_test_ship_choose_ship_track(&map.shared, &leaves, &probe, probe.input.tile, 1 << TRACKDIR_X_SW, 0, path.get());
		check(probe.path_found == 0 && stats["region_nodes"] == 257 && stats["region_limits"] == 1 && stats["track_nodes"] == 0);
		check(stats["lost"] == 1 && stats["random_draws"] == 8 && draws == 8);
	}
	regions = nullptr;
}
}
#endif
