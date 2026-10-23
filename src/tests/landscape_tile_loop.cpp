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
#include "abi_layout.hpp"
#include "../tile_map.h"
#include "../rust/landscape_loop_ffi.h"
#include "landscape_protocol.hpp"
#include "ground_protocol.hpp"
#endif
#include "../safeguards.h"

#ifdef WITH_RUST
extern "C" bool openttd_rust_map_read_probe(OpenTTDTileTypeAllocation, const OpenTTDSharedServices *, const OpenTTDClearServices *, const OpenTTDTreeServices *, uint32_t);
TEST_CASE("Rust map fields preserve native helpers, mutation and reset")
{
	for (const uint size : {64U, 128U}) {
		Map::Allocate(size, 64);
		const auto allocation = GetRustTileTypeAllocation();
		CHECK(allocation.extended != nullptr);
#if !defined(NDEBUG) || defined(WITH_ASSERT)
		CHECK(allocation.assertions);
#else
		CHECK(!allocation.assertions);
#endif
		for (const TileIndex tile : {TileXY(1, 1), TileIndex{Map::Size() - 1}}) {
			for (const uint8_t kind : {0, 4, 6, 7}) {
				for (uint value = 0; value < 256; value++) {
					Tile t{tile};
					t.type() = (kind << 4) | (value & 3);
					t.m1() = value; t.m2() = value * 257; t.m3() = value;
					t.m4() = value; t.m5() = value; t.m6() = value;
					SetTileHeight(tile, value);
					CHECK(openttd_rust_map_read_probe(allocation, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base()));
					auto unchecked = allocation;
					unchecked.assertions = false;
					CHECK(openttd_rust_map_read_probe(unchecked, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base()));
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
	auto Layout = [](const char *name, std::initializer_list<size_t> values) { CHECK(AbiLayoutMismatch(name, values) == -1); };
#include "abi-layout-landscape.inc"
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
