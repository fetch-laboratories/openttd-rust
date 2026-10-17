/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town-tunnel-reference.cpp Controlled services for unchanged town tunnel bodies, absent from the save corpus. */
#include "../../src/tests/town_protocol.hpp"
#include "../../src/rust/town_ffi.h"
#include <algorithm>
#include <array>
#include <cassert>
#include <cstdlib>
#include <cstdio>
#include <initializer_list>
#include <memory>
#include <map>
#include <numeric>
#include <span>
#include <limits>
#include <unordered_map>
#include <vector>

using TileIndex = uint32_t;
using TileIndexDiff = int32_t;
/* The shared FFI declares native DiagDirection; fixture scalars convert explicitly. */
static_assert(sizeof(DiagDirection) == sizeof(uint8_t));
using RoadBits = uint32_t;
using RoadType = uint32_t;
static_assert(sizeof(Slope) == sizeof(uint8_t)); // Shared FFI declares native Slope.
constexpr uint32_t DIAGDIR_END = 4, MP_ROAD = 2, MP_RAILWAY = 1, MP_STATION = 5, MP_TUNNELBRIDGE = 9;
constexpr uint32_t TRANSPORT_ROAD = 1, DRD_NONE = 0;
constexpr size_t NUM_HOUSE_ZONES = 5;
enum class HouseZone : uint8_t { TownEdge, TownOutskirt, TownOuterSuburb, TownInnerSuburb, TownCentre };
static uint8_t to_underlying(HouseZone zone) { return static_cast<uint8_t>(zone); }
template <typename T> T ClampTo(uint64_t value) { return static_cast<T>(std::min<uint64_t>(value, std::numeric_limits<T>::max())); }
struct Town {
	uint32_t index = 17;
	struct { uint32_t population = 0, num_houses = 0; std::array<uint32_t, 5> squared_town_zone_radius{}; } cache;
	struct SuppliedHistory {
		uint32_t production = 0, transported = 0;
#include "town-percent-original.hpp"
	};
	std::array<int16_t, 15> ratings{};
	struct { uint16_t bits = 0; void Set(uint8_t company) { this->bits |= 1 << company; } } have_ratings;
};
template <typename T> T SumHistory(std::span<const T>);
struct { struct { bool allow_town_level_crossings = true; uint8_t dist_local_authority = 20; } economy; struct { uint8_t town_council_tolerance = 0; } difficulty; } _settings_game;
enum class DoCommandFlag { Auto = 1, NoWater = 2, Execute = 4, NoModifyTownRating = 8, NoTestTownRating = 16 };
struct Flags {
	uint32_t bits = 0;
	Flags() = default;
	Flags(std::initializer_list<DoCommandFlag> flags) { for (auto flag : flags) this->Set(flag); }
	bool Test(DoCommandFlag flag) const { return (this->bits & static_cast<uint32_t>(flag)) != 0; }
	Flags &Set(DoCommandFlag flag) { this->bits |= static_cast<uint32_t>(flag); return *this; }
};
using DoCommandFlags = Flags;
static bool _town_rating_test = false;
static std::map<const Town *, int> _town_test_ratings;
static uint8_t _current_company;
static struct { struct { bool value = false; } magic_bulldozer; } _cheats;
static uint32_t rating_valid_calls, rating_dirty_calls;
struct Company { static bool IsValidID(uint8_t company) { rating_valid_calls++; return company == 0 || company == 4; } };
constexpr uint32_t WC_TOWN_AUTHORITY = 1;
static void SetWindowDirty(uint32_t, uint32_t) { rating_dirty_calls++; }
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
static TileIndexDiff TileOffsByDiagDir(DiagDirection dir) { return std::array<int32_t, 4>{-1, 1024, 1, -1024}[dir]; }
static DiagDirection ReverseDiagDir(DiagDirection dir) { return static_cast<DiagDirection>((dir + 2) & 3); }
static uint32_t DiagDirToAxis(DiagDirection dir) { return dir & 1; }
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
static void RoadMask() { world->trace.push_back({21, 0, 0, 0, 0, 0, 0}); }
static void RoadInfo(uint32_t type) { world->trace.push_back({24, type, 0, 0, 0, 0, 0}); }
/* One active town-buildable type is a controlled shared service, not a second
 * copy of the migrated road-type-selection algorithm. */
static RoadType GetTownRoadType() { RoadMask(); RoadInfo(0); return 0; }
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

enum class TownRatingCheckType : uint8_t { RoadRemove, TunnelBridgeRemove, End };
static constexpr uint8_t to_underlying(TownRatingCheckType type) { return static_cast<uint8_t>(type); }
constexpr int TOWN_COUNCIL_PERMISSIVE = 3, RATING_VERYPOOR = -200, RATING_MINIMUM = -1000;
constexpr int RATING_ROAD_NEEDED_LENIENT = 16, RATING_ROAD_NEEDED_NEUTRAL = 64, RATING_ROAD_NEEDED_HOSTILE = 112, RATING_ROAD_NEEDED_PERMISSIVE = RATING_MINIMUM;
constexpr int RATING_TUNNEL_BRIDGE_NEEDED_LENIENT = 144, RATING_TUNNEL_BRIDGE_NEEDED_NEUTRAL = 208, RATING_TUNNEL_BRIDGE_NEEDED_HOSTILE = 400, RATING_TUNNEL_BRIDGE_NEEDED_PERMISSIVE = RATING_MINIMUM;
constexpr uint32_t STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS = 19;
struct CommandCost { bool ok = true; uint32_t error = 0; uint64_t parameter = 0; };
static CommandCost CommandCostWithParam(uint32_t error, uint64_t parameter) { return {false, error, parameter}; }
static Town *authority_town;
static OpenTTDTownState *authority_owner;
static uint32_t authority_closest_calls;
static Town *ClosestTownFromTile(TileIndex tile, uint32_t threshold)
{
	assert(tile == 129 && threshold == _settings_game.economy.dist_local_authority);
	authority_closest_calls++; return authority_town;
}

#include "town-tunnel-original.hpp"

static uint64_t Leaf(uint32_t op, uint32_t, uint32_t, uint32_t, uint32_t) noexcept { assert(op == 21); RoadMask(); return 1; }
static OpenTTDTownState *State(uint32_t id) noexcept { assert(id == world->town.index); return world->owner; }
static void Stations(uint32_t, uint32_t, void *, void (*)(void *, const OpenTTDStation *, uint32_t)) noexcept {}
static uint32_t Random(void *) noexcept { world->draws++; return 0; }
static uint32_t Industry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static uint32_t Read_map_log_x() noexcept { return 10; }
static uint8_t Read_tile_type(uint32_t id) noexcept { return uint8_t(At(id).type); }
static bool Read_tile_valid(uint32_t id) noexcept { return IsValidTile(id); }
static uint8_t Read_tile_slope(uint32_t id) noexcept { return uint8_t(GetTileSlope(id)); }
static int32_t Read_tile_z(uint32_t id) noexcept { return int32_t(GetTileZ(id)); }
static uint8_t Read_road_bits(uint32_t id, uint8_t type, bool straight) noexcept { assert(type == 0 && straight); return uint8_t(GetTownRoadBits(id)); }
static OpenTTDTownRoadChoice Read_road_info(uint8_t id) noexcept { RoadInfo(id); return {.introduction_date = 0, .max_speed = 50, .town_build = true}; }
static uint64_t RoadTypes() noexcept { return Leaf(21, 0, 0, 0, 0); }
static bool BuildRoad(uint32_t town, uint32_t tile, uint32_t bits, uint32_t type, uint32_t mode) noexcept { assert(mode == 0); return RoadCommand(tile, bits, type, town).Succeeded(); }
static bool BuildTunnel(uint32_t tile, uint32_t type, uint32_t execute) noexcept { return TunnelCommand(tile, type, execute != 0).Succeeded(); }
static bool Candidate()
{
	openttd_rust_town_simulation(world->owner)->population = world->town.cache.population;
	auto leaves = TownProtocolServices();
	leaves.map_log_x = Read_map_log_x; leaves.road_info = Read_road_info;
	leaves.state = State; leaves.stations = Stations; leaves.road_mask = RoadTypes; leaves.build_road = BuildRoad; leaves.build_tunnel = BuildTunnel;
	OpenTTDSharedServices services = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
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
	return openttd_rust_town_tunnel(&leaves, &services, world->town.index, world->start, world->direction);
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
static void CompareHistories()
{
	uint32_t checked = 0;
	for (const size_t count : {1U, 3U, 4U, 61U}) {
		for (const uint32_t production : {0U, 1U, 0x7FFFFFFFU, 0x80000000U, UINT32_MAX}) {
			for (const uint32_t transported : {0U, 1U, 0x7FFFFFFFU, 0x80000000U, UINT32_MAX}) {
				for (const uint32_t stride : {0U, 17U, 0x40000000U, 0x80000000U}) {
					std::array<Town::SuppliedHistory, 61> reference{};
					std::array<OpenTTDTownSuppliedHistory, 61> candidate{};
					for (size_t i = 0; i < count; ++i) {
						const auto p = production + static_cast<uint32_t>(i) * stride;
						const auto t = transported + static_cast<uint32_t>(i) * stride;
						reference[i] = {p, t}; candidate[i] = {p, t};
					}
					const auto expected = SumHistory<Town::SuppliedHistory>(std::span{reference.data(), count});
					const auto actual = openttd_rust_town_sum_history(candidate.data(), count);
					if (actual.production != expected.production || actual.transported != expected.transported) {
						std::fprintf(stderr, "history mismatch count=%zu production=%u transported=%u stride=%u expected=%u/%u actual=%u/%u\n",
							count, production, transported, stride, expected.production, expected.transported, actual.production, actual.transported);
						std::exit(1);
					}
					checked++;
				}
			}
		}
	}
	std::printf("\nPASS %u unchanged-reference history cases\n", checked);
}
static bool RatingCompanyValid(uint8_t company) noexcept { return Company::IsValidID(company); }
static void RatingDirty(uint32_t town) noexcept { SetWindowDirty(WC_TOWN_AUTHORITY, town); }
static void CompareRatings()
{
	Town town;
	auto owner = std::unique_ptr<OpenTTDTownState, decltype(&openttd_rust_town_destroy)>(openttd_rust_town_new(), openttd_rust_town_destroy);
	auto leaves = TownProtocolServices(); leaves.dirty_authority = RatingDirty;
	OpenTTDSharedServices services = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
	services.company_valid = RatingCompanyValid;
	uint32_t checked = 0;
	auto reset = [&](int16_t initial) {
		town.ratings.fill(initial); town.have_ratings.bits = 0;
		auto *state = openttd_rust_town_simulation(owner.get());
		std::fill_n(state->ratings, 15, initial); state->have_ratings = 0;
	};
	auto mode = [&](bool enabled) { SetTownRatingTestMode(enabled); openttd_rust_town_rating_test(enabled); };
	auto check = [&](int add, int maximum, uint8_t company, bool no_modify = false, bool bulldozer = false, bool null_town = false) {
		_current_company = company; _cheats.magic_bulldozer.value = bulldozer;
		rating_valid_calls = rating_dirty_calls = 0;
		ChangeTownRating(null_town ? nullptr : &town, add, maximum, no_modify ? Flags{DoCommandFlag::NoModifyTownRating} : Flags{});
		const auto expected_valid = rating_valid_calls, expected_dirty = rating_dirty_calls;
		rating_valid_calls = rating_dirty_calls = 0;
		openttd_rust_town_change_rating(&leaves, &services, null_town ? nullptr : owner.get(), town.index,
			reinterpret_cast<uintptr_t>(&town), company, add, maximum, no_modify, bulldozer);
		const auto *state = openttd_rust_town_simulation(owner.get());
		bool equal = expected_valid == rating_valid_calls && expected_dirty == rating_dirty_calls && state->have_ratings == town.have_ratings.bits && std::equal(town.ratings.begin(), town.ratings.end(), state->ratings);
		if (company == 0 || company == 4) equal &= GetRating(&town) == openttd_rust_town_rating(owner.get(), reinterpret_cast<uintptr_t>(&town), company);
		if (!equal) {
			std::fprintf(stderr, "rating mismatch case=%u add=%d max=%d company=%u mode=%u no-modify=%u cheat=%u null=%u\n", checked, add, maximum, company, _town_rating_test, no_modify, bulldozer, null_town);
			std::exit(1);
		}
		checked++;
	};
	for (const int16_t initial : {-32768, -1000, 0, 1000, 32767}) {
		for (const int add : {-40000, -1, 0, 1, 40000}) {
			for (const int maximum : {-50000, -1000, 0, 1000, 50000}) {
				reset(initial); check(add, maximum, 0); check(add, maximum, 4);
				reset(initial); mode(true); check(add, maximum, 0);
				mode(true); check(add, maximum, 4); mode(false); check(-add, maximum, 0); mode(false);
				check(add, maximum, 0, true); check(add, maximum, 255); check(add, maximum, 4, false, true); check(add, maximum, 0, false, false, true);
			}
		}
	}
	/* Source keys only the native shell address, including across company and
	 * owner reuse within nested test mode; starting a new outer mode clears it. */
	reset(10); mode(true); check(100, 1000, 0); mode(true);
	owner.reset(openttd_rust_town_new()); reset(-100); check(1, 1000, 4); mode(false); check(-1, -1000, 0); mode(false);
	mode(true); check(1, 1000, 4); mode(false);
	std::printf("\nPASS %u unchanged-reference rating cases\n", checked);
}
static void CompareRadii()
{
	Town town;
	auto owner = std::unique_ptr<OpenTTDTownState, decltype(&openttd_rust_town_destroy)>(openttd_rust_town_new(), openttd_rust_town_destroy);
	std::vector<uint32_t> counts;
	for (uint32_t houses = 0; houses <= 100; ++houses) counts.push_back(houses);
	for (uint32_t houses : {1000U, 10000U, 0x1000000U, 0x7FFFFFFFU, 0x80000000U, UINT32_MAX}) counts.push_back(houses);
	for (uint32_t houses : counts) {
		town.cache.num_houses = houses; UpdateTownRadius(&town);
		auto *state = openttd_rust_town_simulation(owner.get()); state->num_houses = houses;
		openttd_rust_town_update_radius(owner.get());
		if (!std::equal(town.cache.squared_town_zone_radius.begin(), town.cache.squared_town_zone_radius.end(), state->radius)) {
			std::fprintf(stderr, "radius mismatch houses=%u\n", houses); std::exit(1);
		}
	}
	std::printf("\nPASS %zu unchanged-reference radius cases\n", counts.size());
}
static void CompareAuthority()
{
	Town town;
	auto owner = std::unique_ptr<OpenTTDTownState, decltype(&openttd_rust_town_destroy)>(openttd_rust_town_new(), openttd_rust_town_destroy);
	authority_owner = owner.get();
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, Random, Industry);
	shared.current_company = +[]() noexcept { return _current_company; };
	shared.company_valid = +[](uint8_t id) noexcept { return Company::IsValidID(id); };
	shared.closest_town = +[](uint32_t tile, uint32_t threshold) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(ClosestTownFromTile(tile, threshold)); };
	shared.town_owner = +[](const OpenTTDTownHandle *) noexcept { return authority_owner; };
	const OpenTTDTownAuthorityServices a{
		.council_tolerance = +[]() noexcept { return _settings_game.difficulty.town_council_tolerance; },
		.bulldozer = +[]() noexcept { return _cheats.magic_bulldozer.value; },
		.authority_distance = +[]() noexcept { return _settings_game.economy.dist_local_authority; },
		.town_index = +[](const OpenTTDTownHandle *t) noexcept { return static_cast<uint16_t>(reinterpret_cast<const Town *>(t)->index); },
		.refusal = STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS,
	};
	OpenTTDCommandCostServices costs{
		.zero = +[](OpenTTDCommandCost *) noexcept { std::abort(); },
		.error = +[](OpenTTDCommandCost *, uint32_t) noexcept { std::abort(); },
		.expense_money = +[](OpenTTDCommandCost *, uint8_t, int64_t) noexcept { std::abort(); },
		.error_param = +[](OpenTTDCommandCost *, uint32_t, uint64_t) noexcept { std::abort(); },
		.add = +[](OpenTTDCommandCost *, OpenTTDCommandCost *) noexcept { std::abort(); },
		.move = +[](OpenTTDCommandCost *, OpenTTDCommandCost *) noexcept { std::abort(); },
		.amount = +[](OpenTTDCommandCost *, int64_t) noexcept { std::abort(); },
		.success = +[](const OpenTTDCommandCost *) noexcept -> bool { std::abort(); },
		.error_message = +[](const OpenTTDCommandCost *) noexcept -> uint32_t { std::abort(); },
		.money = +[](const OpenTTDCommandCost *) noexcept -> int64_t { std::abort(); },
	};
	costs.zero = +[](OpenTTDCommandCost *out) noexcept { *reinterpret_cast<CommandCost *>(out) = {}; };
	costs.error_param = +[](OpenTTDCommandCost *out, uint32_t error, uint64_t parameter) noexcept { *reinterpret_cast<CommandCost *>(out) = CommandCostWithParam(error, parameter); };
	auto leaves = TownProtocolServices(); leaves.dirty_authority = RatingDirty;
	uint32_t checked = 0;
	for (uint8_t company : {0, 4, 255}) for (uint8_t council = 0; council < 4; ++council) {
		for (bool no_test : {false, true}) for (bool cheat : {false, true}) for (bool absent : {false, true}) {
			for (int16_t rating : {-32768, -1001, -1000, -999, -201, -200, -199, 15, 16, 17, 63, 64, 65, 111, 112, 113, 143, 144, 145, 207, 208, 209, 399, 400, 401, 32767}) for (bool testing : {false, true}) {
				_current_company = company; _settings_game.difficulty.town_council_tolerance = council;
				_cheats.magic_bulldozer.value = cheat; authority_town = absent ? nullptr : &town;
				town.ratings.fill(rating); auto *state = openttd_rust_town_simulation(owner.get()); std::fill_n(state->ratings, 15, rating);
				Flags flags; if (no_test) flags.Set(DoCommandFlag::NoTestTownRating);
				if (testing) {
					SetTownRatingTestMode(true); openttd_rust_town_rating_test(true);
					SetTownRatingTestMode(true); openttd_rust_town_rating_test(true);
					ChangeTownRating(&town, -40000, -1000, Flags{});
					openttd_rust_town_change_rating(&leaves, &shared, owner.get(), town.index, reinterpret_cast<uintptr_t>(&town), company, -40000, -1000, false, cheat);
				}
				for (uint8_t kind = 0; kind < 3; ++kind) {
					rating_valid_calls = authority_closest_calls = 0;
					CommandCost expected = kind == 2 ? CheckIfAuthorityAllowsNewStation(129, flags) : CheckforTownRating(flags, authority_town, static_cast<TownRatingCheckType>(kind));
					uint32_t valid_calls = rating_valid_calls, closest_calls = authority_closest_calls;
					rating_valid_calls = authority_closest_calls = 0; CommandCost actual;
					if (kind == 2) openttd_rust_town_authority_station(&a, &shared, &costs, reinterpret_cast<OpenTTDCommandCost *>(&actual), 129, no_test);
					else openttd_rust_town_authority_remove(&a, &shared, &costs, reinterpret_cast<OpenTTDCommandCost *>(&actual), reinterpret_cast<const OpenTTDTownHandle *>(authority_town), no_test, kind);
					if (actual.ok != expected.ok || actual.error != expected.error || actual.parameter != expected.parameter || rating_valid_calls != valid_calls || authority_closest_calls != closest_calls) {
						std::fprintf(stderr, "authority mismatch case=%u kind=%u rating=%d company=%u council=%u\n", checked, kind, rating, company, council); std::exit(1);
					}
					checked++;
				}
				if (testing) {
					SetTownRatingTestMode(false); openttd_rust_town_rating_test(false);
					SetTownRatingTestMode(false); openttd_rust_town_rating_test(false);
				}
			}
		}
	}
	std::printf("\nPASS %u unchanged-reference authority cases\n", checked);
}
static void ComparePercentTransported()
{
	uint32_t checked = 0;
	for (uint32_t production : {0U, 1U, 255U, 256U, 0x1000000U, UINT32_MAX}) {
		for (uint32_t transported : {0U, 1U, 255U, 256U, 0x1000000U, UINT32_MAX}) {
			Town::SuppliedHistory original{production, transported};
			OpenTTDTownSuppliedHistory candidate{production, transported};
			if (original.PctTransported() != openttd_rust_town_percent_transported(&candidate)) {
				std::fprintf(stderr, "transported percent mismatch production=%u transported=%u\n", production, transported); std::exit(1);
			}
			checked++;
		}
	}
	std::printf("\nPASS %u unchanged-reference percent cases\n", checked);
}
int main()
{
	CompareHistories();
	CompareRatings();
	CompareRadii();
	CompareAuthority();
	ComparePercentTransported();
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
