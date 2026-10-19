/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape-loop-reference.cpp Unchanged scheduler versus Rust for unreachable map phases. */
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <initializer_list>
#include <memory>
#include "rust/landscape_loop_ffi.h"

static unsigned int log_x, log_y;
static constexpr uint8_t native_types[]{3, 5, 10, 3}; // Void dispatches to Rust water.
static uint64_t observed_hash, observed_count;
static bool rust_run, reenter, nested;
static uint32_t starting_cursor;
struct BaseTile { uint8_t type = 0, height = 0; uint16_t m2 = 0; uint8_t m1 = 0, m3 = 0, m4 = 0, m5 = 0; };
static_assert(sizeof(BaseTile) == 8 && alignof(BaseTile) == 2 && offsetof(BaseTile, type) == 0);
static std::unique_ptr<BaseTile[]> tiles;
static bool raw_backend, mutate;
static uint32_t feedback;

struct TileIndex {
	uint32_t value = 0;
	TileIndex() = default;
	explicit TileIndex(uint32_t value) : value(value) {}
	uint32_t base() const { return this->value; }
	bool operator!=(uint32_t other) const { return this->value != other; }
};
struct Map {
	static unsigned int LogX() { return log_x; }
	static unsigned int LogY() { return log_y; }
};
struct TimerGameTick { static inline uint64_t counter; };
struct PerformanceAccumulator { explicit PerformanceAccumulator(int) {} };
static constexpr int PFE_GL_LANDSCAPE = 0;
static constexpr unsigned int MIN_MAP_SIZE_BITS = 6, MAX_MAP_SIZE_BITS = 12;
static constexpr uint32_t TILE_UPDATE_FREQUENCY_LOG = 8, TILE_UPDATE_FREQUENCY = 256;
template<typename T, size_t N> constexpr size_t lengthof(T (&)[N]) { return N; }
using uint = unsigned int;

static void NativeVisit(uint32_t tile, uint8_t kind) noexcept;
static uint8_t Type(uint32_t tile) noexcept { return tiles[tile].type >> 4; }
static void House(uint32_t tile) noexcept { NativeVisit(tile, 3); }
static void LandscapeStationLoop(uint32_t tile) noexcept { NativeVisit(tile, 5); }
static void Object(uint32_t tile) noexcept { NativeVisit(tile, 10); }
static void ReferenceHouse(TileIndex tile) { House(tile.base()); }
static void ReferenceStation(TileIndex tile) { LandscapeStationLoop(tile.base()); }
static void ReferenceObject(TileIndex tile) { Object(tile.base()); }
static uint8_t GetTileType(uint32_t tile) { return Type(tile); }
static uint8_t GetTileType(TileIndex tile) { return Type(tile.base()); }
struct TileTypeProcs { void (*tile_loop_proc)(TileIndex); };
static const TileTypeProcs procs[3] = {{ReferenceHouse}, {ReferenceStation}, {ReferenceObject}};
static const TileTypeProcs * const _tile_type_procs[16] = {nullptr, nullptr, nullptr, &procs[0], nullptr, &procs[1], nullptr, nullptr, nullptr, nullptr, &procs[2]};
static TileIndex _cur_tileloop_tile;
#include <cassert>
#include "landscape-loop-reference.inc"

static uint32_t Random(void *) noexcept { std::abort(); }
static uint32_t Industry(int32_t, int32_t, uint32_t *) noexcept { std::abort(); }
static OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
static OpenTTDLandscapeLoopServices services{
	.shared = &shared,
	.tile_types = {nullptr, 0},
	.trees = nullptr,
	.industry = {},
	.ground = {},
	.house_loop = House,
	.station_loop = LandscapeStationLoop,
	.object_loop = Object,
};
static void RustRun()
{
	const OpenTTDTreeLoopSettings settings{
		.tick_counter = TimerGameTick::counter,
		.size_x = 1U << log_x,
		.climate = 0,
		.extra = 0,
		.ambient = false,
	};
	/* Each nested entry has its own immutable table, like the native facade. */
	auto batch_services = services;
	batch_services.tile_types = raw_backend ? OpenTTDTileTypeAllocation{reinterpret_cast<const uint8_t *>(tiles.get()), 1U << (log_x + log_y)} : OpenTTDTileTypeAllocation{nullptr, 0};
	openttd_rust_run_tile_loop(log_x, log_y, settings, &batch_services);
}
static void NativeVisit(uint32_t tile, uint8_t kind) noexcept
{
	observed_hash = (observed_hash ^ (static_cast<uint64_t>(tile) << 4) ^ kind) * UINT64_C(1099511628211);
	observed_count++;
	if (mutate && tile != 0) {
		uint32_t next = (tile >> 1) ^ (-(static_cast<int32_t>(tile & 1)) & feedback);
		tiles[next].type = static_cast<uint8_t>((native_types[(next + 1) & 3] << 4) | (tiles[next].type & 15));
	}
	if (reenter && !nested) {
		nested = true;
		/* Both entries must retain the pre-batch canonical cursor until return. */
		uint32_t cursor = rust_run ? *openttd_rust_tile_loop_cursor() : _cur_tileloop_tile.base();
		if (cursor != starting_cursor) std::abort();
		if (rust_run) RustRun(); else RunTileLoop();
	}
}
int main()
{
	shared.tile_type = Type;
	unsigned cases = 0;
	for (log_x = 6; log_x <= 12; log_x++) {
		for (log_y = 6; log_y <= 12; log_y++) {
			tiles = std::make_unique<BaseTile[]>(1U << (log_x + log_y));
			for (uint32_t tile = 0; tile < (1U << (log_x + log_y)); tile++) tiles[tile].type = static_cast<uint8_t>((native_types[tile % 4] << 4) | (tile & 15));
			for (uint64_t tick : {UINT64_C(0), UINT64_C(1), UINT64_C(255), UINT64_C(256), UINT64_MAX}) {
				for (uint32_t start : {1U, (1U << (log_x + log_y)) - 1, 0x5A5U}) {
					for (bool enter : {false, true}) {
						TimerGameTick::counter = tick;
						reenter = enter;
						starting_cursor = start;
						_cur_tileloop_tile = TileIndex{start};
						observed_count = observed_hash = 0;
						nested = rust_run = false;
						RunTileLoop();
						const uint64_t expected_hash = observed_hash, expected_count = observed_count;
						const uint32_t expected_cursor = _cur_tileloop_tile.base();
						for (bool raw : {false, true}) {
							raw_backend = raw;
							*openttd_rust_tile_loop_cursor() = start;
							observed_count = observed_hash = 0;
							nested = false;
							rust_run = true;
							RustRun();
							if (observed_hash != expected_hash || observed_count != expected_count || *openttd_rust_tile_loop_cursor() != expected_cursor) std::abort();
							cases++;
						}
					}
				}
			}
		}
	}
	/* The first callback mutates the NEXT LFSR tile. Nested entries mutate the
	 * same live allocation too; no descriptor spans a reset inside the batch. */
	log_x = log_y = 6;
	feedback = 0xD8F;
	TimerGameTick::counter = 1;
	mutate = true;
	for (bool raw : {false, true}) {
		raw_backend = raw;
		uint64_t expected_hash = 0, expected_count = 0;
		uint32_t expected_cursor = 0;
		for (bool rust : {false, true}) {
			/* Reset BETWEEN entries and reacquire; no stale pointer is read. */
			tiles = std::make_unique<BaseTile[]>(4096);
			for (uint32_t tile = 0; tile < 4096; tile++) tiles[tile].type = static_cast<uint8_t>(native_types[tile % 4] << 4);
			rust_run = rust;
			reenter = true;
			nested = false;
			starting_cursor = 1;
			observed_count = observed_hash = 0;
			if (rust) {
				*openttd_rust_tile_loop_cursor() = 1;
				RustRun();
				if (observed_hash != expected_hash || observed_count != expected_count || *openttd_rust_tile_loop_cursor() != expected_cursor) std::abort();
			} else {
				_cur_tileloop_tile = TileIndex{1};
				RunTileLoop();
				expected_hash = observed_hash;
				expected_count = observed_count;
				expected_cursor = _cur_tileloop_tile.base();
			}
		}
	}
	std::printf("landscape scheduler: %u unchanged-body map/phase/cursor/reentry cases plus ordered mutation/reset passed\n", cases);
}
