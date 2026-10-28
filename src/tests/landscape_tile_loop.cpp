/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape_tile_loop.cpp Actual native allocation/layout and live shared reads. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../rust/tile_type_access.hpp"
#include "../rust/movement_ffi.h"
#include "../rust/landscape_command_ffi.h"
#include "../rust/landscape_generation_ffi.h"
#include "../rust/world_generation_ffi.h"
#include <thread>
#include "../rust/generation_sea_set.hpp"
#include "abi_layout.hpp"
#include "../tile_map.h"
#include "../road_type.h"
#include "../rust/landscape_loop_ffi.h"
#include "landscape_protocol.hpp"
#include "ground_protocol.hpp"
#endif
#include "../safeguards.h"

#ifdef WITH_RUST
extern "C" uint8_t openttd_rust_shared_tile_type(OpenTTDTileTypeAllocation, uint8_t (*callback)(uint32_t) noexcept, uint32_t tile);
extern "C" bool openttd_rust_map_read_probe(OpenTTDTileTypeAllocation, const OpenTTDSharedServices *, const OpenTTDClearServices *, const OpenTTDTreeServices *, uint32_t, void (*)(uint32_t) noexcept);
TEST_CASE("Rust map fields preserve native helpers, mutation and reset")
{
	for (const uint size : {64U, 128U}) {
		Map::Allocate(size, 64);
		const auto allocation = GetRustTileTypeAllocation();
		CHECK(allocation.extended != nullptr);
		for (const TileIndex tile : {TileXY(1, 1), TileIndex{Map::Size() - 1}}) {
			for (const TileType kind : {MP_CLEAR, MP_RAILWAY, MP_ROAD, MP_TREES, MP_WATER, MP_VOID}) {
				for (uint value = 0; value < 256; value++) {
					Tile t{tile};
					t.type() = (kind << 4) | (value & 3);
					t.m1() = value; t.m2() = value * 257; t.m3() = value;
					t.m4() = value; t.m5() = kind == MP_ROAD ? value & 63 : value; t.m6() = value;
					t.m8() = value * 257;
					SetTileHeight(tile, value);
					CHECK(openttd_rust_map_read_probe(allocation, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base(), [](uint32_t) noexcept {}));
					auto unchecked = allocation;
					unchecked.assertions = false;
					CHECK(openttd_rust_map_read_probe(unchecked, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base(), [](uint32_t) noexcept {}));
				}
			}
		}
	}
	Map::Allocate(64, 64);
}
TEST_CASE("Rust movement map context rereads native mutations within an entry")
{
	for (const uint size : {64U, 128U}) {
		Map::Allocate(size, 64);
		for (const bool assertions : {false, true}) {
			auto allocation = GetRustTileTypeAllocation();
			allocation.assertions = assertions;
			for (const TileIndex tile : {TileXY(1, 1), TileIndex{Map::Size() - 1}}) {
				for (const bool change_type : {false, true}) {
					Tile t{tile};
					t.type() = MP_ROAD << 4;
					t.m3() = 3; t.m4() = INVALID_ROADTYPE; t.m5() = 3;
					t.m6() = 0; t.m8() = INVALID_ROADTYPE << 6;
					const auto mutate_fields = [](uint32_t index) noexcept {
						Tile t{TileIndex{index}};
						t.m3() = 12; t.m4() = 0; t.m5() = 0x3C;
						t.m6() = 6 << 3; t.m8() = 0;
					};
					const auto mutate_type = [](uint32_t index) noexcept {
						Tile t{TileIndex{index}};
						t.type() = MP_CLEAR << 4;
					};
					CHECK(openttd_rust_map_read_probe(allocation, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base(), change_type ? +mutate_type : +mutate_fields));
				}
			}
		}
	}
	Map::Allocate(64, 64);
}

TEST_CASE("Rust ground enum and consumed-record native calls")
{
	CHECK(GroundProbe::Run());
}

TEST_CASE("Rust landscape scalar native handler calls")
{
	LandscapeProbe::Run([](bool result) { CHECK(result); });
}

TEST_CASE("Rust shared tile-type allocation and all native byte values")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-landscape.inc"
#include "abi-layout-genworld.inc"
	Map::Allocate(128, 64);
	const auto allocation = GetRustTileTypeAllocation();
	const auto &shared = GetRustSharedServices();
	CHECK(allocation.count == Map::Size());
	CHECK(allocation.base != nullptr);
	/* Raw storage covers every byte, including values that semantic setters
	 * cannot assign to these edge tiles. */
	for (const TileIndex tile : {TileIndex{0}, TileIndex{Map::Size() - 1}}) {
		for (uint byte = 0; byte < 256; byte++) {
			Tile(tile).type() = byte;
			CHECK(openttd_rust_shared_tile_type(allocation, shared.tile_type, tile.base()) == static_cast<uint8_t>(GetTileType(tile)));
			CHECK(openttd_rust_shared_tile_type({.base = nullptr, .extended = nullptr, .count = 0, .assertions = false}, shared.tile_type, tile.base()) == static_cast<uint8_t>(GetTileType(tile)));
		}
	}
	/* Real native writes between accesses must be observed without a new snapshot.
	 * Use an interior tile so these setters are valid with either edge setting. */
	const TileIndex live_tile = TileXY(1, 1);
	SetTileType(live_tile, MP_ROAD);
	CHECK(openttd_rust_shared_tile_type(allocation, shared.tile_type, live_tile.base()) == MP_ROAD);
	SetTileType(live_tile, MP_CLEAR);
	CHECK(openttd_rust_shared_tile_type(allocation, shared.tile_type, live_tile.base()) == MP_CLEAR);
	/* Reset between reader entries; the next entry acquires a live allocation. */
	Map::Allocate(64, 64);
	const auto next = GetRustTileTypeAllocation();
	CHECK(next.count == Map::Size());
	CHECK(next.count != allocation.count);
	const TileIndex next_tile = TileXY(Map::MaxX() - 1, Map::MaxY() - 1);
	SetTileType(next_tile, MP_INDUSTRY);
	CHECK(openttd_rust_shared_tile_type(next, shared.tile_type, next_tile.base()) == MP_INDUSTRY);
	Map::Allocate(64, 64);
}

TEST_CASE("Rust shared tile-type out-of-range preserves callback policy")
{
	Map::Allocate(64, 64);
	auto allocation = GetRustTileTypeAllocation();
	/* Exercise the checked backend without executing native asserts-off UB. */
	allocation.assertions = true;
	/* The original native accessor stays the production oracle. This fixture
	 * records fallback rather than executing source UB with assertions disabled. */
	static uint calls;
	static uint32_t observed;
	calls = 0;
	auto callback = [](uint32_t tile) noexcept -> uint8_t { calls++; observed = tile; return 231; };
	CHECK(openttd_rust_shared_tile_type(allocation, callback, allocation.count) == 231);
	CHECK(calls == 1);
	CHECK(observed == allocation.count);
	CHECK(openttd_rust_shared_tile_type(allocation, callback, UINT32_MAX) == 231);
	CHECK(calls == 2);
	CHECK(observed == UINT32_MAX);
	CHECK(openttd_rust_shared_tile_type({.base = nullptr, .extended = nullptr, .count = 0, .assertions = false}, callback, 0) == 231);
	CHECK(calls == 3);
	CHECK(observed == 0);
}
#endif

#ifdef WITH_RUST
TEST_CASE("Rust generation sea identity preserves native flattening iteration")
{
	RustGenerationSeaSet boundary;
	for (uint count : {1U, 1025U, 7U, 64U}) {
		std::unordered_set<TileIndex> source;
		boundary.Construct();
		for (uint i = 0; i < count; i++) {
			uint tile = (i * 197 + 64) & 8191;
			source.insert(TileIndex{tile}); boundary.Insert(tile);
			CHECK(boundary.Contains(tile)); CHECK(boundary.Size() == source.size());
		}
		uint32_t tile = boundary.First();
		/* FlowRiver consumes this stream directly as its two terraform calls per
		 * tile. Reconstructing after a large search must restore source buckets. */
		for (TileIndex expected : source) { CHECK(tile == expected.base()); tile = boundary.Next(); }
		CHECK(tile == INVALID_TILE.base());
		boundary.Destroy();
	}
}
#endif

#ifdef WITH_RUST
TEST_CASE("Rust world generation shares native lvalues across serialized threads")
{
	bool *address = &_generating_world;
	bool before = _generating_world;
	bool same_address = false, aborted = false;
	std::thread first([&] { same_address = openttd_rust_generating_world() == address; _generating_world = true; AbortGeneratingWorld(); });
	first.join();
	CHECK(_generating_world);
	CHECK(same_address);
	OpenTTDWorldServices services{.exit_game = []() noexcept { return false; }};
	std::thread second([&] { aborted = openttd_rust_world_aborted(&services); *openttd_rust_generating_world() = false; });
	second.join();
	CHECK_FALSE(_generating_world);
	CHECK(aborted);
	_generating_world = before;
}
static int world_callback_value;
TEST_CASE("Rust world generation replaces transient callbacks before invoking them")
{
	OpenTTDWorldServices services{
		.game_mode = []() noexcept { return GM_EDITOR; },
		.callback = [](OpenTTDWorldFrame *, GWDoneProc *proc) noexcept { proc(); return true; },
		.abort_mode = [](SwitchMode mode) noexcept { CHECK(mode == SM_EDITOR); },
	};
	world_callback_value = 0;
	openttd_rust_world_set_abort_callback([] { world_callback_value = 1; });
	openttd_rust_world_set_abort_callback([] { world_callback_value = 2; });
	CHECK(openttd_rust_world_handle_abort(&services, nullptr));
	CHECK(world_callback_value == 2);
	openttd_rust_world_set_abort_callback(nullptr);
	CHECK(openttd_rust_world_handle_abort(&services, nullptr));
	CHECK(world_callback_value == 2);
}
#endif
