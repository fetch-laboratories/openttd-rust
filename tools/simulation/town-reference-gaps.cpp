/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town-reference-gaps.cpp Controlled services for unchanged town tunnel bodies, absent from the save corpus. */
#include "../../src/stdafx.h"
#include "../../src/tests/town_protocol.hpp"
#include "../../src/rust/town_ffi.h"
#include <algorithm>
#include <array>
#include <cassert>
#include <cstdlib>
#include <cstdio>
#include <initializer_list>
#include <unordered_map>
#include <vector>

namespace TownReference {
using TileIndex = uint32_t;
using TileIndexDiff = int32_t;
/* The shared FFI declares native DiagDirection; fixture scalars convert explicitly. */
static_assert(sizeof(DiagDirection) == sizeof(uint8_t));
using RoadBits = uint32_t;
using RoadType = uint32_t;
static_assert(sizeof(Slope) == sizeof(uint8_t)); // Shared FFI declares native Slope.
constexpr uint32_t DIAGDIR_END = 4, MP_ROAD = 2, MP_RAILWAY = 1, MP_STATION = 5, MP_TUNNELBRIDGE = 9;
constexpr uint32_t TRANSPORT_ROAD = 1, DRD_NONE = 0;
struct Town {
	uint32_t index = 17;
	struct { uint32_t population = 0; } cache;
};
struct { struct { bool allow_town_level_crossings = true; } economy; } _settings_game;
enum class DoCommandFlag { Auto = 1, NoWater = 2, Execute = 4 };
struct Flags {
	uint32_t bits = 0;
	Flags() = default;
	Flags(std::initializer_list<DoCommandFlag> flags) { for (auto flag : flags) this->Set(flag); }
	bool Test(DoCommandFlag flag) const { return (this->bits & static_cast<uint32_t>(flag)) != 0; }
	Flags &Set(DoCommandFlag flag) { this->bits |= static_cast<uint32_t>(flag); return *this; }
};
using DoCommandFlags = Flags;
enum { CMD_BUILD_ROAD, CMD_BUILD_TUNNEL };
template <int> constexpr uint32_t GetCommandFlags() { return 0; }
static Flags CommandFlagsToDCFlags(uint32_t) { return {}; }
struct Cost { bool ok; bool Succeeded() const { return this->ok; } };
struct Tile {
	uint32_t type = 0, slope = 0, z = 4, roads = 0, stop = 0, facing = 0, transport = TRANSPORT_ROAD;
	bool valid = true;
};
struct World {
	Town town;
	uint32_t start = 512 * 1024 + 512, direction = 0, length = 0, draws = 0, effects = 0;
	bool road_test = true, tunnel_test = true, tunnel_execute = true;
	std::unordered_map<uint32_t, Tile> tiles;
	std::vector<std::array<uint32_t, 7>> trace;
	OpenTTDTownState *owner = nullptr;
};
static World *world;
static Tile At(TileIndex tile)
{
	if (tile >= 1024 * 1024 || tile % 1024 == 1023 || tile / 1024 == 1023) return Tile{.valid = false};
	auto it = world->tiles.find(tile);
	return it == world->tiles.end() ? Tile{} : it->second;
}
/* A callable map service suppresses ADL into the real map's dimension-dependent
 * TileOffsByDiagDir. Direction-only helpers use the canonical native header. */
static constexpr auto TileOffsByDiagDir = [](DiagDirection dir) -> TileIndexDiff { return std::array<int32_t, 4>{-1, 1024, 1, -1024}[dir]; };
static RoadBits DiagDirToRoadBits(DiagDirection dir) { return 1 << (3 - dir); }
static TileIndex TileAddByDiagDir(TileIndex tile, DiagDirection dir) { return tile + TileOffsByDiagDir(dir); }
static Slope InclinedSlope(DiagDirection dir) { return static_cast<Slope>(std::array<uint32_t, 4>{12, 6, 3, 9}[dir]); }
static Slope GetTileSlope(TileIndex tile) { return static_cast<Slope>(At(tile).slope); }
static uint32_t GetTileZ(TileIndex tile) { return At(tile).z; }
static bool IsValidTile(TileIndex tile) { return At(tile).valid; }
static bool IsTileType(TileIndex tile, uint32_t type) { return At(tile).type == type; }
static bool IsSteepSlope(Slope slope) { return (slope & 16) != 0; }
static bool IsSlopeWithOneCornerRaised(Slope slope) { return slope == 1 || slope == 2 || slope == 4 || slope == 8; }
static RoadBits GetTownRoadBits(TileIndex tile) { return At(tile).roads; }
static uint32_t GetTunnelBridgeTransportType(TileIndex tile) { return At(tile).transport; }
static DiagDirection GetTunnelBridgeDirection(TileIndex tile) { return static_cast<DiagDirection>(At(tile).facing); }
static bool IsDriveThroughStopTile(TileIndex tile) { return At(tile).stop == 1; }
static uint32_t GetDriveThroughStopAxis(TileIndex tile) { return At(tile).facing; }
static bool IsBayRoadStopTile(TileIndex tile) { return At(tile).stop == 2; }
static DiagDirection GetBayRoadStopDir(TileIndex tile) { return static_cast<DiagDirection>(At(tile).facing); }
static bool IsRoadDepot(TileIndex tile) { return At(tile).stop == 3; }
static DiagDirection GetRoadDepotDirection(TileIndex tile) { return static_cast<DiagDirection>(At(tile).facing); }
static void RoadInfo(uint32_t type) { world->trace.push_back({24, type, 0, 0, 0, 0, 0}); }
/* One active town-buildable road type, served by a controlled shared service. */
static RoadType GetTownRoadType() { RoadInfo(0); return 0; }
static Cost RoadCommand(TileIndex tile, RoadBits bits, RoadType rt, uint32_t town)
{
	world->trace.push_back({1, town, tile, bits, rt, 0, 0});
	return {world->road_test};
}
static Cost TunnelCommand(TileIndex tile, RoadType rt, bool execute)
{
	world->trace.push_back({5, 0, tile, rt, execute, 0, 0});
	/* A failing execute can leave effects; the original ignores its return. */
	if (execute) world->effects++;
	return {execute ? world->tunnel_execute : world->tunnel_test};
}
template <int command> struct Command {
	static Cost Do(Flags flags, TileIndex tile, uint32_t a, uint32_t b, uint32_t = 0, uint32_t town = 0)
	{
		if constexpr (command == CMD_BUILD_ROAD) {
			assert(flags.bits == 3);
			return RoadCommand(tile, a, b, town);
		} else {
			assert(a == TRANSPORT_ROAD && (flags.bits == 0 || flags.bits == 4));
			return TunnelCommand(tile, b, (flags.bits & 4) != 0);
		}
	}
};

#include "town-tunnel-original.hpp"

/* Rust retains this fixture shell only as an opaque identity; the native FFI
 * tag is global Town, while the reference body uses TownReference::Town. */
static OpenTTDTownState *ResolveOwner(uint32_t id) noexcept { assert(id == world->town.index); return world->owner; }
static ::Town *ResolveShell(uint32_t id) noexcept { assert(id == world->town.index); return reinterpret_cast<::Town *>(&world->town); }
static uint32_t Random(void *) noexcept { world->draws++; return 0; }
static uint32_t Industry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static uint32_t Read_map_log_x() noexcept { return 10; }
static uint8_t Read_tile_type(uint32_t id) noexcept { return uint8_t(At(id).type); }
static bool Read_tile_valid(uint32_t id) noexcept { return IsValidTile(id); }
static uint8_t Read_tile_slope(uint32_t id) noexcept { return uint8_t(GetTileSlope(id)); }
static int32_t Read_tile_z(uint32_t id) noexcept { return int32_t(GetTileZ(id)); }
static uint8_t Read_road_bits(uint32_t id, uint8_t type, bool straight) noexcept { assert(type == 0 && straight); return uint8_t(GetTownRoadBits(id)); }
static OpenTTDTownRoadChoice Read_road_info(uint8_t id) noexcept { RoadInfo(id); return {.introduction_date = 0, .max_speed = 50, .town_build = true}; }
static bool BuildRoad(uint32_t town, uint32_t tile, uint32_t bits, uint32_t type, uint32_t mode) noexcept { assert(mode == 0); return RoadCommand(tile, bits, type, town).Succeeded(); }
static bool BuildTunnel(uint32_t tile, uint32_t type, uint32_t execute) noexcept { return TunnelCommand(tile, type, execute != 0).Succeeded(); }
static bool Candidate()
{
	AutoRestoreBackup<decltype(GetRustClockState().calendar_year)> calendar_year_scope(GetRustClockState().calendar_year, decltype(GetRustClockState().calendar_year){1950});
	AutoRestoreBackup<uint64_t> road_mask(*openttd_rust_road_mask(), 1);
	openttd_rust_town_simulation(world->owner)->population = world->town.cache.population;
	auto leaves = TownProtocolServices();
	leaves.map_log_x = Read_map_log_x; leaves.road_info = Read_road_info;
	leaves.resolve_owner = ResolveOwner;
	auto house_services = *leaves.houses;
	leaves.houses = &house_services;
	leaves.build_tunnel = BuildTunnel;
	OpenTTDSharedServices services = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
	AutoRestoreBackup<decltype(GetRustClockState().calendar_date)> calendar_date_scope(GetRustClockState().calendar_date, decltype(GetRustClockState().calendar_date){0});
	services.town_by_id = [](uint32_t id) noexcept { return reinterpret_cast<OpenTTDTownHandle *>(ResolveShell(id)); };
	services.tile_type = Read_tile_type; services.tile_valid = Read_tile_valid;
	services.tile_slope = Read_tile_slope; services.tile_z = Read_tile_z; services.road_bits = Read_road_bits;
	services.tunnel_transport = [](uint32_t id) noexcept { return uint8_t(GetTunnelBridgeTransportType(id)); };
	services.tunnel_bridge_direction = [](uint32_t id) noexcept { return uint8_t(GetTunnelBridgeDirection(id)); };
	services.is_drive_through_stop = [](uint32_t id) noexcept { return IsDriveThroughStopTile(id); };
	services.drive_through_axis = [](uint32_t id) noexcept { return uint8_t(GetDriveThroughStopAxis(id)); };
	services.is_bay_stop = [](uint32_t id) noexcept { return IsBayRoadStopTile(id); };
	services.bay_stop_direction = [](uint32_t id) noexcept { return uint8_t(GetBayRoadStopDir(id)); };
	services.is_road_depot = [](uint32_t id) noexcept { return IsRoadDepot(id); };
	services.road_depot_direction = [](uint32_t id) noexcept { return uint8_t(GetRoadDepotDirection(id)); };
	leaves.allow_crossings = []() noexcept { return _settings_game.economy.allow_town_level_crossings; };
	bool road_ok = false;
	OpenTTDCommandCostServices costs{}; costs.success = [](const OpenTTDCommandCost *out) noexcept { return *reinterpret_cast<const bool *>(out); };
	OpenTTDRailCommandServices rail{}; rail.cost = &costs;
	OpenTTDRoadCommandServices roads{}; roads.do_build = [](const OpenTTDRoadCommandServices *, OpenTTDRoadCommandScratch *, OpenTTDCommandCost *out, OpenTTDCommandCost *, uint16_t flags, uint32_t tile, uint8_t bits, uint8_t rt, uint8_t, uint16_t town) noexcept {
		assert(flags == 10); *reinterpret_cast<bool *>(out) = BuildRoad(town, tile, bits, rt, 0);
	}; roads.rail_commands = &rail;
	OpenTTDRoadCommandContext road{&roads, nullptr, reinterpret_cast<OpenTTDCommandCost *>(&road_ok), nullptr};
	return openttd_test_town_tunnel(&leaves, &services, &road, world->town.index, world->start, world->direction);
}
static uint32_t Along(const World &w, int distance) { return w.start + distance * TileOffsByDiagDir(static_cast<DiagDirection>(w.direction)); }
static World Fixture(uint32_t dir, uint32_t length, bool mountain)
{
	World w;
	w.direction = dir; w.length = length;
	w.tiles[Along(w, -1)].roads = DiagDirToRoadBits(static_cast<DiagDirection>(dir));
	for (uint32_t i = 0; i <= length; i++) {
		auto &t = w.tiles[Along(w, i)];
		t.z = i == 0 || i == length ? 4 : 5;
		t.slope = InclinedSlope(static_cast<DiagDirection>(dir));
	}
	if (!mountain) w.tiles[Along(w, 1)].type = MP_STATION;
	return w;
}
static uint32_t cases;
static void Compare(World input, const char *label)
{
	World reference = input, candidate = input;
	world = &reference;
	bool expected = GrowTownWithTunnel(&world->town, world->start, static_cast<DiagDirection>(world->direction));
	world = &candidate;
	candidate.owner = openttd_rust_town_new();
	bool actual = Candidate();
	openttd_rust_town_destroy(candidate.owner);
	if (expected != actual || reference.trace != candidate.trace || reference.effects != candidate.effects || reference.draws != candidate.draws) {
		std::fprintf(stderr, "tunnel mismatch: case=%u %s dir=%u length=%u population=%u result=%u/%u trace=%zu/%zu effects=%u/%u RNG=%u/%u\n", cases, label, input.direction, input.length, input.town.cache.population, expected, actual, reference.trace.size(), candidate.trace.size(), reference.effects, candidate.effects, reference.draws, candidate.draws);
		for (size_t i = 0; i < std::max(reference.trace.size(), candidate.trace.size()); i++) {
			std::fprintf(stderr, "trace[%zu]", i);
			for (const auto *trace : {&reference.trace, &candidate.trace}) {
				std::fprintf(stderr, " |");
				if (i < trace->size()) for (auto value : (*trace)[i]) std::fprintf(stderr, " %u", value);
			}
			std::fprintf(stderr, "\n");
		}
		std::exit(1);
	}
	/* Emit the oracle-derived ordered transcript, not expected values duplicated
 * from Rust. Python retains this small evidence beside the generated bodies. */
	std::printf("%u %s dir=%u length=%u population=%u result=%u effects=%u RNG=%u", cases++, label, input.direction, input.length, input.town.cache.population, expected, reference.effects, reference.draws);
	for (const auto &entry : reference.trace) { std::printf(" |"); for (auto value : entry) std::printf(" %u", value); }
	std::puts("");
}
static void Run()
{
	for (uint32_t dir = 0; dir < 4; dir++) {
		for (bool mountain : {false, true}) {
			for (uint32_t length : {1, 2, 4, 5, 6, 7, 8, 9, 255, 256, 257}) {
				for (uint32_t population : {0, 999, 1000, 249000, 250000}) {
					auto w = Fixture(dir, length, mountain); w.town.cache.population = population; Compare(w, mountain ? "mountain-length" : "obstruction-length");
				}
			}
		}
		for (uint32_t slope = 0; slope < 32; slope++) {
			auto w = Fixture(dir, 4, true); w.tiles[w.start].slope = slope; Compare(w, "entrance-slope");
			w = Fixture(dir, 4, true); w.tiles[Along(w, 2)].slope = slope; Compare(w, "mountain-slope");
		}
		auto w = Fixture(dir, 4, false); w.tiles[Along(w, -1)].roads = 0; Compare(w, "disconnected");
		for (uint32_t place : {1, 5}) {
			for (uint32_t type : {0U, MP_ROAD, MP_RAILWAY, MP_STATION, MP_TUNNELBRIDGE}) {
				for (uint32_t stop = 0; stop < 4; stop++) {
					for (uint32_t facing = 0; facing < 4; facing++) {
						w = Fixture(dir, 4, true); auto &t = w.tiles[Along(w, place)]; t.type = type; t.stop = stop; t.facing = facing;
						Compare(w, place == 1 ? "start-continuation" : "exit-continuation");
						if (type == MP_TUNNELBRIDGE) { t.transport = 0; Compare(w, "rail-tunnel"); }
					}
				}
			}
			w = Fixture(dir, 4, true); w.tiles[Along(w, place)].valid = false; Compare(w, "invalid-continuation");
		}
		for (uint32_t place : {1, 2, 3, 4}) { w = Fixture(dir, 4, true); w.tiles[Along(w, place)].valid = false; Compare(w, "invalid-mountain-end"); }
		_settings_game.economy.allow_town_level_crossings = false;
		w = Fixture(dir, 4, true); w.tiles[Along(w, 5)].type = MP_RAILWAY; Compare(w, "crossing-disabled");
		_settings_game.economy.allow_town_level_crossings = true;
		for (uint32_t failure = 0; failure < 3; failure++) {
			w = Fixture(dir, 4, true);
			if (failure == 0) w.road_test = false;
			if (failure == 1) w.tunnel_test = false;
			if (failure == 2) w.tunnel_execute = false;
			Compare(w, "command-failure");
		}
	}
	std::printf("PASS %u unchanged-reference tunnel cases\n", cases);
}

} // namespace TownReference

int main()
{
	TownReference::Run();
}
