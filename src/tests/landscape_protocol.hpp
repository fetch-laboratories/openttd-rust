/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape_protocol.hpp Actual scalar native-to-Rust-to-native ABI calls. */
#ifndef TESTS_LANDSCAPE_PROTOCOL_HPP
#define TESTS_LANDSCAPE_PROTOCOL_HPP
#include "../../tools/simulation/session-fixture.hpp"
#include "fixture_map.hpp"
#include "../tile_cmd.h"
#include "../rust/landscape_loop_ffi.h"
#include "../rust/station_lifecycle_ffi.h"
#include "../rust/roadstop_resolver_ffi.h"
#include "../rust/tile_owner_ffi.h"
#include "../rust/abi_ffi.h"
#include <array>
#include <type_traits>

namespace LandscapeProbe {
static_assert(std::is_same_v<TileLoopProc, void(uint32_t) noexcept>);
static inline uint32_t calls, reads, seen, expected_tile;
static inline bool passed;
static constexpr std::array<uint8_t, 4> NATIVE_TYPES{5, 10, 10, 5}; // Other kinds dispatch to Rust owners.
static uint8_t Type(uint32_t tile) noexcept
{
	passed &= tile == expected_tile;
	return NATIVE_TYPES[reads++ % NATIVE_TYPES.size()];
}
template<uint8_t Kind> static void Visit(uint32_t tile_index) noexcept
{
	/* Exactly the local strong-index construction used by every native handler. */
	const TileIndex tile{tile_index};
	passed &= tile.base() == expected_tile && Kind == NATIVE_TYPES[calls % NATIVE_TYPES.size()];
	seen |= 1U << Kind;
	calls++;
	expected_tile = (expected_tile >> 1) ^ ((0U - (expected_tile & 1U)) & 0xD8FU);
}
static OpenTTDObjectLoopPolicy ProbeObjectPolicy(uint32_t tile) noexcept
{
	Visit<10>(tile);
	return {nullptr, nullptr, 0, 0, 0};
}
static bool ProbeObjectOnWater(uint32_t) noexcept { return false; }
template<typename Check> void Run(Check check)
{
	calls = reads = seen = 0;
	expected_tile = 1;
	passed = true;
	/* A zero-size map selects the callback backend for every tile read. */
	const OpenTTDFixtureMap callbacks{0};
	auto shared = OpenTTDSharedServices{};
	shared.map = &callbacks.statics;
	shared.tile_type = Type;
	shared.station_type = [](uint32_t) noexcept { return StationType::End; };
	/* The Rust station loop's first native call is GetByTile. */
	OpenTTDRoadStopResolverServices roadstops{};
	roadstops.by_tile = [](uint32_t tile) noexcept -> BaseStation * { Visit<5>(tile); return nullptr; };
	auto station = OpenTTDStationLifecycleServices{};
	station.shared = &shared;
	station.roadstop = &roadstops;
	const OpenTTDIndustryTileServices object_tiles{.on_water = ProbeObjectOnWater};
	auto objects = OpenTTDObjectServices{};
	objects.shared = &shared;
	objects.industry_tiles = &object_tiles;
	objects.loop_policy = ProbeObjectPolicy;
	const OpenTTDLandscapeLoopServices services{
		.shared = &shared,
		.trees = nullptr,
		.industry = {},
		.ground = {},
		.town = nullptr,
		.station = &station,
		.objects = &objects,
	};
	const OpenTTDTreeLoopSettings settings{.tick_counter = 1, .size_x = 64, .climate = 0, .extra = 0, .ambient = false, .ambient_callback = false};
	const auto saved_cursor = *openttd_rust_tile_loop_cursor();
	*openttd_rust_tile_loop_cursor() = 1;
	SessionFixture fixture;
	fixture.loop = {settings, &services};
	check(fixture.Run());
	check(passed && calls == 16 && reads == calls && seen == 0x420 && *openttd_rust_tile_loop_cursor() == expected_tile);
	*openttd_rust_tile_loop_cursor() = saved_cursor;
}
} // namespace LandscapeProbe
#endif /* TESTS_LANDSCAPE_PROTOCOL_HPP */
