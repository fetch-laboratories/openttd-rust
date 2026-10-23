/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape-loop-reference.cpp Unchanged scheduler versus Rust for unreachable map phases. */
#include <cstdint>
#include <algorithm>
#include <bit>
#include <limits>
#include <tuple>
#include <cassert>
#include <optional>
#include "rust/movement_ffi.h"
#include "slope_type.h"
#include "track_type.h"
#include "road_type.h"
#include <cstdio>
#include <cstdlib>
#include <initializer_list>
#include <memory>
#include "rust/landscape_loop_ffi.h"

static unsigned int log_x, log_y;
using uint = unsigned int;
static constexpr uint TILE_SIZE = 16, TILE_HEIGHT = 8;
static constexpr uint8_t MP_VOID = 7;
#define NOT_REACHED() std::abort()
#include "movement-foundation-reference.inc"

static constexpr uint8_t native_types[]{5, 10, 10, 5}; // Other kinds dispatch to Rust owners.
static uint64_t observed_hash, observed_count;
static bool rust_run, reenter, nested;
static uint32_t starting_cursor;
using BaseTile = TileBaseStorage;
static_assert(sizeof(BaseTile) == 8 && alignof(BaseTile) == 2 && offsetof(BaseTile, type) == 0);
static std::unique_ptr<BaseTile[]> tiles;
static std::unique_ptr<TileExtendedStorage[]> extended;
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
	static unsigned int SizeX() { return 1U << log_x; }
	static unsigned int SizeY() { return 1U << log_y; }
};
struct TimerGameTick { static inline uint64_t counter; };
struct PerformanceAccumulator { explicit PerformanceAccumulator(int) {} };
static constexpr int PFE_GL_LANDSCAPE = 0;
static constexpr unsigned int MIN_MAP_SIZE_BITS = 6, MAX_MAP_SIZE_BITS = 12;
static constexpr uint32_t TILE_UPDATE_FREQUENCY_LOG = 8, TILE_UPDATE_FREQUENCY = 256;
template<typename T, size_t N> constexpr size_t lengthof(T (&)[N]) { return N; }
static void NativeVisit(uint32_t tile, uint8_t kind) noexcept;
static uint8_t Type(uint32_t tile) noexcept { return tiles[tile].type >> 4; }
static void LandscapeStationLoop(uint32_t tile) noexcept { NativeVisit(tile, 5); }
static void Object(uint32_t tile) noexcept { NativeVisit(tile, 10); }
static void ReferenceStation(TileIndex tile) { LandscapeStationLoop(tile.base()); }
static void ReferenceObject(TileIndex tile) { Object(tile.base()); }
static uint8_t GetTileType(uint32_t tile) { return Type(tile); }
static uint8_t GetTileType(TileIndex tile) { return Type(tile.base()); }
static Slope movement_slope;
static uint32_t outside_x, outside_y;
static std::tuple<Slope, int> GetTilePixelSlopeOutsideMap(int x, int y) { outside_x = x; outside_y = y; return {movement_slope, 24}; }
static int GetSlopePixelZ(int x, int y, bool ground) { assert(!ground); return 24 + GetPartialPixelZ(x & 15, y & 15, movement_slope); }
struct TileTypeProcs { void (*tile_loop_proc)(TileIndex); int (*get_slope_z_proc)(TileIndex, uint, uint, bool) = nullptr; };
static int GetSlopePixelZ_Void(TileIndex, uint, uint, bool);
static const TileIndex INVALID_TILE{UINT32_MAX};
static const TileTypeProcs movement_void{nullptr, GetSlopePixelZ_Void};
static const TileTypeProcs procs[2] = {{ReferenceStation}, {ReferenceObject}};
static const TileTypeProcs * const _tile_type_procs[16] = {nullptr, nullptr, nullptr, nullptr, nullptr, &procs[0], nullptr, &movement_void, nullptr, nullptr, &procs[1]};
#include "movement-coordinate-reference.inc"
static TileIndex _cur_tileloop_tile;
#include <cassert>
#include "landscape-loop-reference.inc"

static uint32_t Random(void *) noexcept { std::abort(); }
static uint32_t Industry(int32_t, int32_t, uint32_t *) noexcept { std::abort(); }
static OpenTTDObjectLoopPolicy ReferenceObjectPolicy(uint32_t tile) noexcept
{
	Object(tile);
	return {nullptr, nullptr, 0, 0, 0};
}
static bool ReferenceObjectOnWater(uint32_t) noexcept { return false; }
static OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
static const OpenTTDIndustryTileServices object_tiles{.on_water = ReferenceObjectOnWater};
static const OpenTTDObjectServices objects{.shared = &shared, .industry_tiles = &object_tiles, .loop_policy = ReferenceObjectPolicy};
static OpenTTDLandscapeLoopServices services{
	.shared = &shared,
	.tile_types = {},
	.trees = nullptr,
	.industry = {},
	.ground = {},
	.town = nullptr,
	.station_loop = LandscapeStationLoop,
	.objects = &objects,
	.object_frame = nullptr,
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
	batch_services.tile_types = raw_backend ? OpenTTDTileTypeAllocation{.base = tiles.get(), .extended = extended.get(), .count = 1U << (log_x + log_y), .assertions = true} : OpenTTDTileTypeAllocation{};
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
/* These states cannot be built through commands; this extends the existing
 * default gap probe, not a second movement comparison tool. */
static void MovementGap()
{
	const Slope slopes[]{SLOPE_FLAT, SLOPE_W, SLOPE_S, SLOPE_SW, SLOPE_E, SLOPE_EW, SLOPE_SE, SLOPE_WSE, SLOPE_N, SLOPE_NW, SLOPE_NS, SLOPE_ENW, SLOPE_NE, SLOPE_SEN, SLOPE_NWS, SLOPE_STEEP_W, SLOPE_STEEP_S, SLOPE_STEEP_E, SLOPE_STEEP_N};
	unsigned cases = 0;
	for (Slope slope : slopes) for (uint bits = 0; bits < 256; bits++) {
		if (openttd_rust_rail_foundation(slope, static_cast<TrackBits>(bits), [](bool ok) noexcept { assert(ok); }) != GetRailFoundation(slope, static_cast<TrackBits>(bits))) std::abort();
		if (openttd_rust_road_foundation(slope, static_cast<RoadBits>(bits), [](bool ok) noexcept { assert(ok); }) != GetRoadFoundation(slope, static_cast<RoadBits>(bits))) std::abort();
		cases += 2;
	}
	OpenTTDSharedServices world{};
	world.map_log_x = []() noexcept -> uint32_t { return log_x; };
	world.map_max_x = []() noexcept { return Map::SizeX() - 1; };
	world.map_max_y = []() noexcept { return Map::SizeY() - 1; };
	world.tile_type = [](uint32_t) noexcept -> uint8_t { return 0; };
	OpenTTDMovementServices movement{};
	movement.shared = &world;
	movement.pixel_slope = [](uint32_t) noexcept -> OpenTTDFoundationSlope { return {.z = 24, .slope = movement_slope}; };
	movement.outside_pixel_slope = [](uint32_t x, uint32_t y) noexcept -> OpenTTDFoundationSlope { outside_x = x; outside_y = y; return {.z = 24, .slope = movement_slope}; };
	for (uint x_bits : {6U, 12U}) for (uint y_bits : {6U, 12U}) for (Slope slope : slopes) {
		log_x = x_bits; log_y = y_bits;
		movement_slope = slope;
		for (int x : {INT32_MIN, -65537, -16, -1, 0, 1, 15, 16, 1023, 1024, 65535, 65536, INT32_MAX}) for (int y : {INT32_MIN, -16, -1, 0, 15, 1023, 1024, 65535, 65536, INT32_MAX}) {
			outside_x = outside_y = UINT32_MAX;
			int expected = GetSlopePixelZOutsideMap(x, y);
			const uint32_t expected_x = outside_x, expected_y = outside_y;
			outside_x = outside_y = UINT32_MAX;
			if (openttd_rust_movement_outside_z(&movement, x, y) != expected || outside_x != expected_x || outside_y != expected_y) std::abort();
			cases++;
		}
	}
	std::printf("movement foundation/extreme-coordinate gap: %u unchanged-body cases passed\n", cases);
}
int main()
{
	MovementGap();
	shared.tile_type = Type;
	unsigned cases = 0;
	for (log_x = 6; log_x <= 12; log_x++) {
		for (log_y = 6; log_y <= 12; log_y++) {
			tiles = std::make_unique<BaseTile[]>(1U << (log_x + log_y));
			extended = std::make_unique<TileExtendedStorage[]>(1U << (log_x + log_y));
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
			extended = std::make_unique<TileExtendedStorage[]>(4096);
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
