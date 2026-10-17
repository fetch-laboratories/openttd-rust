/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape_protocol.hpp Actual scalar native-to-Rust-to-native ABI calls. */
#ifndef TESTS_LANDSCAPE_PROTOCOL_HPP
#define TESTS_LANDSCAPE_PROTOCOL_HPP
#include "../tile_cmd.h"
#include "../rust/landscape_loop_ffi.h"
#include "../rust/abi_ffi.h"
#include <array>
#include <type_traits>

namespace LandscapeProbe {
static_assert(std::is_same_v<TileLoopProc, void(uint32_t) noexcept>);
static_assert(std::is_same_v<decltype(&openttd_rust_run_tile_loop), void (*)(unsigned int, unsigned int, OpenTTDTreeLoopSettings, const OpenTTDLandscapeLoopServices *)>);
static_assert(std::is_same_v<decltype(OpenTTDLandscapeLoopServices::house_loop), TileLoopProc *>);
static inline uint32_t calls, reads, seen, expected_tile;
static inline bool passed;
static constexpr std::array<uint8_t, 4> NATIVE_TYPES{3, 5, 10, 3}; // Void dispatches to Rust water.
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
template<typename Check> void Run(Check check)
{
	const std::array<size_t, 13> layout{sizeof(OpenTTDGroundLoopServices), alignof(OpenTTDGroundLoopServices), offsetof(OpenTTDGroundLoopServices, clear), offsetof(OpenTTDGroundLoopServices, rail), offsetof(OpenTTDGroundLoopServices, road), offsetof(OpenTTDGroundLoopServices, tunnel), offsetof(OpenTTDGroundLoopServices, water), offsetof(OpenTTDGroundLoopServices, costs), offsetof(OpenTTDGroundLoopServices, call), offsetof(OpenTTDGroundLoopServices, climate), offsetof(OpenTTDGroundLoopServices, editor), offsetof(OpenTTDGroundLoopServices, rebuild), offsetof(OpenTTDGroundLoopServices, ambient_callback)};
	for (uint8_t item = 0; item < layout.size(); item++) check(openttd_rust_abi_layout(802, item) == layout[item]);
	check(openttd_rust_abi_layout(802, layout.size()) == SIZE_MAX);
	calls = reads = seen = 0;
	expected_tile = 1;
	passed = true;
	auto shared = OpenTTDSharedServices{};
	shared.tile_type = Type;
	const OpenTTDLandscapeLoopServices services{
		.shared = &shared,
		.tile_types = {nullptr, 0},
		.trees = nullptr,
		.industry = {},
		.ground = {},
		.house_loop = Visit<3>,
		.station_loop = Visit<5>,
		.object_loop = Visit<10>,
	};
	const OpenTTDTreeLoopSettings settings{.tick_counter = 1, .size_x = 64, .climate = 0, .extra = 0, .ambient = false};
	const auto saved_cursor = *openttd_rust_tile_loop_cursor();
	*openttd_rust_tile_loop_cursor() = 1;
	openttd_rust_run_tile_loop(6, 6, settings, &services);
	check(passed && calls == 16 && reads == calls && seen == 0x428 && *openttd_rust_tile_loop_cursor() == expected_tile);
	*openttd_rust_tile_loop_cursor() = saved_cursor;
}
} // namespace LandscapeProbe
#endif /* TESTS_LANDSCAPE_PROTOCOL_HPP */
