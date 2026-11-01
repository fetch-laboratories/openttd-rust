/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file landscape_tile_loop.cpp Actual native allocation/layout and live shared reads. */
#include "../stdafx.h"
#include "fixture_map.hpp"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../rust/tile_type_access.hpp"
#include "../rust/movement_ffi.h"
#include "../rust/landscape_command_ffi.h"
#include "../rust/landscape_generation_ffi.h"
#include "../rust/world_generation_ffi.h"
#include "../rust/town_data_ffi.h"
#include "../rust/session_ffi.h"
#include "../core/backup_type.hpp"
#include <thread>
#include <cmath>
#include "../rust/generation_sea_set.hpp"
#include "abi_layout.hpp"
#include "../tile_map.h"
#include "../slope_func.h"
#include "../landscape.h"
#include "../timer/timer_game_calendar.h"
#include "../rust/snow_line_ffi.h"
#include "../heightmap.h"
#include "../road_type.h"
#include "../station_type.h"
#include "../rust/landscape_loop_ffi.h"
#include "landscape_protocol.hpp"
#include "ground_protocol.hpp"
#include "world_generation_probe.hpp"
#endif
#include "../safeguards.h"

#ifdef WITH_RUST
static uint snow_line_deleted;
TEST_CASE("Rust snow-line owner preserves native allocation, dates and serialized threads")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-landscape.inc"
	AutoRestoreBackup height(_settings_game.game_creation.snow_line_height, uint8_t{7});
	AutoRestoreBackup date(TimerGameCalendar::date, TimerGameCalendar::date);
	ClearSnowLine();
	TimerGameCalendar::date = TimerGameCalendar::Date{-1};
	for (uint8_t value : {0, 255, 19}) {
		_settings_game.game_creation.snow_line_height = value;
		CHECK_FALSE(IsSnowLineSet());
		CHECK(GetSnowLine() == value); CHECK(openttd_test_snow_line_highest(&GetRustSharedServices()) == value); CHECK(LowestSnowLine() == value);
	}
	auto table = std::make_unique<SnowLine>();
	for (uint month = 0; month < 12; month++) for (uint day = 0; day < 32; day++) table->table[month][day] = static_cast<uint8_t>(month * 37 + day * 11);
	table->highest_value = 3; table->lowest_value = 249;
	std::thread install([&] { SetSnowLine(std::move(table)); }); install.join();
	CHECK(table == nullptr); CHECK(IsSnowLineSet());
	_settings_game.game_creation.snow_line_height = 255;
	std::thread read([&] {
		CHECK(openttd_test_snow_line_highest(&GetRustSharedServices()) == 3); CHECK(LowestSnowLine() == 249);
		for (int year : {1999, 2000, 2001, 2100}) {
			auto first = TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year}, 0, 1);
			auto end = TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year + 1}, 0, 1);
			for (auto day = first; day < end; ++day) {
				TimerGameCalendar::date = day;
				auto ymd = TimerGameCalendar::ConvertDateToYMD(day);
				CHECK(GetSnowLine() == static_cast<uint8_t>(ymd.month * 37 + ymd.day * 11));
			}
		}
	}); read.join();
	_settings_game.game_creation.snow_line_height = 19;
	snow_line_deleted = 0;
	auto support = GetRustSharedServices().snow_line_support;
	support.destroy = [](SnowLine *old) noexcept {
		snow_line_deleted++;
		CHECK(IsSnowLineSet() == (snow_line_deleted == 1));
		delete old;
	};
	auto replacement = std::make_unique<SnowLine>();
	replacement->table[0][1] = 255; replacement->highest_value = 0; replacement->lowest_value = 255;
	TimerGameCalendar::date = TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{2000}, 0, 1);
	openttd_rust_snow_line_set(&support, replacement.release());
	CHECK(snow_line_deleted == 1); CHECK(GetSnowLine() == 255); CHECK(openttd_test_snow_line_highest(&GetRustSharedServices()) == 0); CHECK(LowestSnowLine() == 255);
	openttd_rust_snow_line_set(&support, nullptr);
	CHECK(snow_line_deleted == 2); CHECK(GetSnowLine() == 19);
	SetSnowLine(std::make_unique<SnowLine>());
	std::thread clear([] { ClearSnowLine(); }); clear.join();
	CHECK_FALSE(IsSnowLineSet());
	SetSnowLine(nullptr); ClearSnowLine(); ClearSnowLine();
	CHECK(GetSnowLine() == 19);
}
TEST_CASE("Town import coordinates preserve native float conversions")
{
	const OpenTTDTownDataServices services{.shared = &GetRustSharedServices(), .founding = nullptr, .strings = {}};
	for (const uint size : {64U, 128U, 4096U}) {
		Map::Allocate(size, 64);
		const float boundary = 0x1p-25f / static_cast<float>(Map::MaxX());
		for (const float input : {std::nextafter(boundary, 0.0f), boundary, std::nextafter(boundary, 1.0f), 0.0001f, 0.01f, 0.3f, 0.5f, 0.99f}) {
			/* Volatile keeps the original's runtime float-to-uint conversion. */
			volatile float stored = input;
			float x = stored, y = stored;
			CHECK(openttd_test_town_data_target(&services, HM_CLOCKWISE, x, y) == TileXY(x * Map::MaxX(), y * Map::MaxY()).base());
			CHECK(openttd_test_town_data_target(&services, HM_COUNTER_CLOCKWISE, x, y) == TileXY((1 - y * Map::MaxX()), x * Map::MaxY()).base());
		}
	}
	Map::Allocate(64, 64);
}
TEST_CASE("Session tile fixtures restore storage mode across nested entries")
{
	static bool nested;
	nested = false;
	SessionFixture fixture;
	fixture.roadstops.by_tile = [](uint32_t) noexcept -> BaseStation * {
		if (nested) return nullptr;
		nested = true;
		SessionFixture inner;
		CHECK(inner.Run());
		return nullptr;
	};
	CHECK(fixture.Run());
	CHECK(nested);
	CHECK(fixture.Run());
}
extern "C" uint8_t openttd_rust_shared_tile_type(const OpenTTDMapStatics *, uint8_t (*callback)(uint32_t) noexcept, uint32_t tile);
extern "C" bool openttd_rust_map_read_probe(const OpenTTDMapStatics *, const OpenTTDSharedServices *, const OpenTTDClearServices *, const OpenTTDTreeServices *, uint32_t, void (*)(uint32_t) noexcept);
TEST_CASE("Rust map fields preserve native helpers, mutation and reset")
{
	for (const uint size : {64U, 128U}) {
		Map::Allocate(size, 64);
		const auto statics = _rust_map_statics;
		CHECK(*statics.extended != nullptr);
		for (const TileIndex tile : {TileXY(1, 1), TileIndex{Map::Size() - 1}}) {
			for (const TileType kind : {MP_CLEAR, MP_RAILWAY, MP_ROAD, MP_TREES, MP_STATION, MP_WATER, MP_VOID, MP_TUNNELBRIDGE}) {
				for (uint value = 0; value < 256; value++) {
					Tile t{tile};
					t.type() = (kind << 4) | (value & 3);
					t.m1() = value; t.m2() = value * 257; t.m3() = value;
					t.m4() = value; t.m5() = kind == MP_ROAD ? value & 63 : value; t.m6() = value;
					t.m8() = value * 257;
					SetTileHeight(tile, value);
					CHECK(openttd_rust_map_read_probe(&statics, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base(), [](uint32_t) noexcept {}));
					auto unchecked = statics;
					unchecked.assertions = false;
					CHECK(openttd_rust_map_read_probe(&unchecked, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base(), [](uint32_t) noexcept {}));
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
			auto statics = _rust_map_statics;
			statics.assertions = assertions;
			for (const TileIndex tile : {TileXY(1, 1), TileIndex{Map::Size() - 1}}) {
				for (const TileType kind : {MP_ROAD, MP_STATION, MP_TUNNELBRIDGE}) {
					for (const bool change_type : {false, true}) {
						Tile t{tile};
						t.type() = kind << 4;
						t.m3() = 3; t.m4() = INVALID_ROADTYPE;
						t.m5() = kind == MP_TUNNELBRIDGE ? TRANSPORT_ROAD << 2 : 3;
						t.m6() = kind == MP_STATION ? static_cast<uint8_t>(StationType::Truck) << 3 : 0;
						t.m8() = INVALID_ROADTYPE << 6;
						const auto mutate_fields = [](uint32_t index) noexcept {
							Tile t{TileIndex{index}};
							t.m3() = 12; t.m4() = 0; t.m5() = 0x3C;
							t.m6() = 6 << 3; t.m8() = 0;
						};
						const auto mutate_type = [](uint32_t index) noexcept {
							Tile t{TileIndex{index}};
							t.type() = MP_CLEAR << 4;
						};
						CHECK(openttd_rust_map_read_probe(&statics, &GetRustSharedServices(), &GetRustClearServices(), &GetRustTreeServices(), tile.base(), change_type ? +mutate_type : +mutate_fields));
					}
				}
			}
		}
	}
	Map::Allocate(64, 64);
}

TEST_CASE("Rust production pixel entries observe live native corners and reallocation")
{
	const Slope slopes[]{SLOPE_FLAT, SLOPE_W, SLOPE_S, SLOPE_SW, SLOPE_E, SLOPE_EW, SLOPE_SE, SLOPE_WSE, SLOPE_N, SLOPE_NW, SLOPE_NS, SLOPE_ENW, SLOPE_NE, SLOPE_SEN, SLOPE_NWS, SLOPE_STEEP_W, SLOPE_STEEP_S, SLOPE_STEEP_E, SLOPE_STEEP_N};
	for (uint width : {64U, 128U}) {
		Map::Allocate(width, 64);
		for (TileIndex t : {TileXY(1, 1), TileXY(Map::MaxX(), Map::MaxY())}) {
			const uint x = TileX(t), y = TileY(t);
			const uint east = std::min(x + 1, Map::MaxX()), south = std::min(y + 1, Map::MaxY());
			for (Slope slope : slopes) {
				SetTileHeight(t, 3 + GetSlopeZInCorner(slope, CORNER_N));
				SetTileHeight(TileXY(east, y), 3 + GetSlopeZInCorner(slope, CORNER_W));
				SetTileHeight(TileXY(x, south), 3 + GetSlopeZInCorner(slope, CORNER_E));
				SetTileHeight(TileXY(east, south), 3 + GetSlopeZInCorner(slope, CORNER_S));
				for (TileType kind : {MP_CLEAR, MP_TREES, MP_STATION, MP_VOID}) {
					Tile{t}.type() = kind << 4;
					auto [native_slope, z] = GetTilePixelSlope(t);
					for (int frac : {0, 8, 15}) {
						const int px = x * TILE_SIZE + frac, py = y * TILE_SIZE + frac;
						const int expected = kind == MP_STATION ? GetTileMaxPixelZ(t) : z + GetPartialPixelZ(frac, frac, native_slope);
						CHECK(openttd_rust_movement_pixel_z(&GetRustMovementServices(), px, py, false) == expected);
					}
				}
			}
			Tile{t}.type() = MP_CLEAR << 4;
			SetTileHeight(t, 9);
			SetTileHeight(TileXY(east, y), 9);
			SetTileHeight(TileXY(x, south), 9);
			SetTileHeight(TileXY(east, south), 9);
			auto [slope, z] = GetTilePixelSlope(t);
			CHECK(openttd_rust_movement_pixel_z(&GetRustMovementServices(), x * TILE_SIZE, y * TILE_SIZE, false) == z + static_cast<int>(GetPartialPixelZ(0, 0, slope)));
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
#include "abi-layout-session.inc"
	Map::Allocate(128, 64);
	const auto *statics = &_rust_map_statics;
	const auto &shared = GetRustSharedServices();
	const uint32_t count = *statics->size;
	CHECK(count == Map::Size());
	CHECK(*statics->base != nullptr);
	const OpenTTDFixtureMap empty{0};
	/* Raw storage covers every byte, including values that semantic setters
	 * cannot assign to these edge tiles. */
	for (const TileIndex tile : {TileIndex{0}, TileIndex{Map::Size() - 1}}) {
		for (uint byte = 0; byte < 256; byte++) {
			Tile(tile).type() = byte;
			CHECK(openttd_rust_shared_tile_type(statics, shared.tile_type, tile.base()) == static_cast<uint8_t>(GetTileType(tile)));
			CHECK(openttd_rust_shared_tile_type(&empty.statics, shared.tile_type, tile.base()) == static_cast<uint8_t>(GetTileType(tile)));
		}
	}
	/* Real native writes between accesses must be observed without a new snapshot.
	 * Use an interior tile so these setters are valid with either edge setting. */
	const TileIndex live_tile = TileXY(1, 1);
	SetTileType(live_tile, MP_ROAD);
	CHECK(openttd_rust_shared_tile_type(statics, shared.tile_type, live_tile.base()) == MP_ROAD);
	SetTileType(live_tile, MP_CLEAR);
	CHECK(openttd_rust_shared_tile_type(statics, shared.tile_type, live_tile.base()) == MP_CLEAR);
	/* Reset between reader entries; the next entry loads the live allocation. */
	Map::Allocate(64, 64);
	CHECK(*statics->size == Map::Size());
	CHECK(*statics->size != count);
	const TileIndex next_tile = TileXY(Map::MaxX() - 1, Map::MaxY() - 1);
	SetTileType(next_tile, MP_INDUSTRY);
	CHECK(openttd_rust_shared_tile_type(statics, shared.tile_type, next_tile.base()) == MP_INDUSTRY);
	Map::Allocate(64, 64);
}

TEST_CASE("Rust shared tile-type out-of-range preserves callback policy")
{
	Map::Allocate(64, 64);
	auto statics = _rust_map_statics;
	/* Exercise the checked backend without executing native asserts-off UB. */
	statics.assertions = true;
	const OpenTTDFixtureMap empty{0};
	/* The original native accessor stays the production oracle. This fixture
	 * records fallback rather than executing source UB with assertions disabled. */
	static uint calls;
	static uint32_t observed;
	calls = 0;
	auto callback = [](uint32_t tile) noexcept -> uint8_t { calls++; observed = tile; return 231; };
	CHECK(openttd_rust_shared_tile_type(&statics, callback, Map::Size()) == 231);
	CHECK(calls == 1);
	CHECK(observed == Map::Size());
	CHECK(openttd_rust_shared_tile_type(&statics, callback, UINT32_MAX) == 231);
	CHECK(calls == 2);
	CHECK(observed == UINT32_MAX);
	CHECK(openttd_rust_shared_tile_type(&empty.statics, callback, 0) == 231);
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
	auto services = OpenTTDWorldServices{};
	services.exit_game = []() noexcept { return false; };
	std::thread second([&] { aborted = openttd_rust_world_aborted(&services); *openttd_rust_generating_world() = false; });
	second.join();
	CHECK_FALSE(_generating_world);
	CHECK(aborted);
	_generating_world = before;
}
static int world_callback_value;
TEST_CASE("Rust world generation replaces transient callbacks before invoking them")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_EDITOR);
	AutoRestoreBackup<SwitchMode> switching(_switch_mode, SM_NONE);
	auto services = OpenTTDWorldServices{};
	services.callback = [](OpenTTDWorldFrame *, GWDoneProc *proc) noexcept { CHECK(_switch_mode == SM_EDITOR); proc(); return true; };
	world_callback_value = 0;
	openttd_test_world_set_abort_callback([] { world_callback_value = 1; });
	openttd_test_world_set_abort_callback([] { world_callback_value = 2; });
	CHECK(openttd_rust_world_handle_abort(&services, nullptr));
	CHECK(world_callback_value == 2);
	openttd_test_world_set_abort_callback(nullptr);
	CHECK(openttd_rust_world_handle_abort(&services, nullptr));
	CHECK(world_callback_value == 2);
	CHECK(_switch_mode == SM_EDITOR);
}
#endif
