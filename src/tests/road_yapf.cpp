/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_yapf.cpp Native road heap ordering and ABI evidence. */
#include "../stdafx.h"
#include "fixture_map.hpp"
#include "../3rdparty/catch2/catch.hpp"
#include "../misc/binaryheap.hpp"
#include "../pathfinder/pathfinder_type.h"
#include "../track_func.h"
#include "../roadveh.h"
#ifdef WITH_RUST
#include "../rust/road_yapf_ffi.h"
#include "../rust/movement_ffi.h"
#include "../rust/station_catchment_ffi.h"
#include "../tile_type.h"
#include "../landscape.h"
#include "../safeguards.h"

namespace {
struct HeapNode { int32_t estimate; bool operator<(const HeapNode &other) const { return this->estimate < other.estimate; } };
OpenTTDRoadYapfTile ProbeTile(const void *, uint32_t, uint8_t, bool) noexcept { OpenTTDRoadYapfTile tile{}; tile.type = 5; tile.station_type = 2; return tile; }
uint16_t fixture_dirs = 1;
OpenTTDFixtureMap movement_map{6, 4096};
/* Every tile carries the road bits and one-way state the callbacks report. */
void SetFixtureDirs(uint16_t dirs)
{
	fixture_dirs = dirs;
	constexpr uint8_t bits[]{10, 5, 9, 6, 3, 12};
	for (uint32_t t = 0; t < movement_map.size; ++t) {
		movement_map.base[t].type = MP_ROAD << 4;
		movement_map.base[t].m5 = bits[std::countr_zero(static_cast<unsigned>(fixture_dirs)) & 7] | ((fixture_dirs > 255 ? 1 : 2) << 4);
	}
}
const OpenTTDSharedServices *MovementWorld()
{
	static OpenTTDSharedServices world{};
	static OpenTTDMovementServices movement{};
	static OpenTTDStationCatchmentServices stations{};
	world.station_catchment = []() noexcept -> const OpenTTDStationCatchmentServices * { return &stations; };
	stations.station_if_valid = [](uint16_t) noexcept -> BaseStation * { return nullptr; };
	world.movement = &movement;
	world.map = &movement_map.statics;
	world.tile_type = [](uint32_t) noexcept -> uint8_t { return 2; };
	world.map_max_y = []() noexcept -> uint32_t { return 63; };
	world.road_tile_kind = [](uint32_t) noexcept { return static_cast<RoadTileType>(0); };
	world.normal_road_bits = [](uint32_t, uint8_t) noexcept -> RoadBits {
		constexpr RoadBits bits[]{static_cast<RoadBits>(10), static_cast<RoadBits>(5), static_cast<RoadBits>(9), static_cast<RoadBits>(6), static_cast<RoadBits>(3), static_cast<RoadBits>(12)};
		return bits[std::countr_zero(static_cast<unsigned>(fixture_dirs)) & 7];
	};
	world.disallowed_road_directions = [](uint32_t) noexcept -> uint8_t { return fixture_dirs > 255 ? 1 : 2; };
	world.has_road_works = [](uint32_t) noexcept { return false; };
	movement.shared = &world;
	movement.road_kind = [](OpenTTDVehicle *) noexcept { return RTT_ROAD; };
	movement.has_tile_road_type = [](uint32_t, RoadTramType) noexcept { return true; };
	SetFixtureDirs(fixture_dirs);
	return &world;
}
uint32_t RoadProbeClosest(const void *, uint16_t, uint8_t) noexcept { return 0; }
const OpenTTDRoadYapfLeaves probe_leaves{
	.shared = MovementWorld(),
	.tile = ProbeTile,
	.closest = RoadProbeClosest,
};
}

TEST_CASE("Road YAPF native heap ties and arbitrary removal", "[road][rust]")
{
	/* Equal estimates, both internal/last removal, open-node replacement and
	 * mixed-cost pops: save chunks do not expose the intermediate heap order. */
	const std::array<int32_t, 8> costs{100, 100, 100, 50, 100, 200, 50, 100};
	std::array<HeapNode, 8> nodes;
	for (size_t i = 0; i < nodes.size(); ++i) nodes[i].estimate = costs[i];
	const std::array<uint32_t, 23> commands{0,0,0,0,0,0,0,0,1,1,0,0,2,2,2,2,2,2,2,2,0,0,2};
	const std::array<int32_t, 23> values{0,1,2,3,4,5,6,7,1,7,1,7,0,0,0,0,0,0,0,0,1,2,0};
	std::array<uint32_t, commands.size()> actual{}, expected{};
	CBinaryHeapT<HeapNode> heap(8);
	for (size_t i = 0; i < commands.size(); ++i) {
		if (commands[i] == 0) heap.Include(&nodes[values[i]]);
		if (commands[i] == 1) heap.Remove(heap.FindIndex(nodes[values[i]]));
		if (commands[i] == 2) expected[i] = heap.Shift() - nodes.data();
		else expected[i] = heap.IsEmpty() ? UINT32_MAX : heap.Begin() - nodes.data();
	}
	const OpenTTDRoadYapfInput input{};
	const auto &leaves = probe_leaves;
	openttd_rust_road_yapf_heap_probe(&input, &leaves, costs.data(), costs.size(), commands.data(), values.data(), commands.size(), actual.data());
	REQUIRE(actual == expected);
}

TEST_CASE("Road YAPF origin masks match shared track constants", "[road][rust]")
{
	const auto &leaves = probe_leaves;
	OpenTTDRoadYapfInput input{};
	input.map_x = input.map_y = 64;
	input.dest_tile = 1;
	input.order_type = 1;
	OpenTTDRoadState *state = openttd_rust_road_new();
	for (uint8_t entry = 0; entry < 4; ++entry) {
		for (uint8_t td = 0; td < 14; ++td) {
			if (td == 6 || td == 7) continue;
			uint16_t tracks = 1U << td;
			SetFixtureDirs(tracks);
			auto result = openttd_rust_road_yapf_choose(state, &input, &leaves, &tracks, 0, entry, TRACKDIR_BIT_Y_NW, true);
			bool reachable = (tracks & DiagdirReachesTrackdirs(static_cast<DiagDirection>(entry))) != 0;
			REQUIRE(result.direction == (reachable ? td : static_cast<uint8_t>(TRACKDIR_Y_NW)));
			REQUIRE(result.found == static_cast<uint8_t>(reachable));
		}
	}
	openttd_rust_road_destroy(state);
}

/* Loaded games clamp max_search_nodes. The generic native search still checks
 * low limits and open-key replacement, now using the production follower on a
 * concrete junction grid instead of an injected transition callback. */
#undef WITH_RUST
#include "../pathfinder/yapf/yapf_node_road.hpp"
#define WITH_RUST
#include "../pathfinder/yapf/yapf_base.hpp"
#include "../pathfinder/follow_track.hpp"
#include "follower_fixture.hpp"
#include "follower_probe.hpp"
namespace RoadLimit {
inline uint32_t goal, start = 32 + 32 * 64;
OpenTTDRoadYapfTile ProbeTile(const void *, uint32_t tile, uint8_t, bool) noexcept
{
	OpenTTDRoadYapfTile out{}; out.type = 2; out.depot = tile == goal; out.depot_dir = DIAGDIR_SW; return out;
}
OpenTTDFollowTrackState Follow(uint32_t tile, uint8_t td)
{
	OpenTTDFollowTrackState ft{};
	const auto &s = FollowerFixture::active->shared;
	openttd_test_follower_init(&ft, &s, TRANSPORT_ROAD, reinterpret_cast<Vehicle *>(FollowerFixture::active->Vehicle()), 0, 0, true, false, true);
	if (!openttd_test_follower_follow(&s, &ft, tile, td)) ft.new_td_bits = TRACKDIR_BIT_NONE;
	return ft;
}
struct Follower {};
class Search;
struct Types { using Tpf = Search; using TrackFollower = Follower; using NodeList = CRoadNodeList; using VehicleType = RoadVehicle; };
class Search : public CYapfBaseT<Types> {
public:
	int max_cost;
	Search(int limit, int max_cost) : max_cost(max_cost) {
		this->max_search_nodes = limit;
		Node &node = this->CreateNewNode(); node.Set(nullptr, TileIndex(start), TRACKDIR_X_NE, true); this->AddStartupNode(node);
	}
	bool PfNodeCacheFetch(Node &) { return false; }
	bool PfDetectDestination(Node &node) { return node.segment_last_tile.base() == goal; }
	void PfFollowNode(Node &parent) {
		const auto ft = Follow(parent.segment_last_tile.base(), parent.segment_last_td);
		for (Trackdir td : SetTrackdirBitIterator(ft.new_td_bits)) {
			Node &node = this->CreateNewNode(); node.Set(&parent, ft.new_tile, td, true); this->AddNewNode(node, Follower{});
		}
	}
	bool PfCalcCost(Node &node, const Follower *) {
		int segment_cost = 0;
		TileIndex tile = node.key.tile;
		Trackdir td = node.key.td;
		for (;;) {
			segment_cost += IsDiagonalTrackdir(td) ? 100 : 71;
			if (tile.base() == goal) break;
			if (this->max_cost > 0 && node.parent->cost + segment_cost > this->max_cost) return false;
			const auto ft = Follow(tile.base(), td);
			if (ft.new_td_bits == TRACKDIR_BIT_NONE || KillFirstBit(ft.new_td_bits) != TRACKDIR_BIT_NONE) break;
			const Trackdir next_td = static_cast<Trackdir>(FindFirstBit(ft.new_td_bits));
			if (ft.new_tile == node.key.tile && next_td == node.key.td) return false;
			// The flat grid has no slope/speed penalty. The depot approach can
			// have one continuation, so PfCalcCost walks to its segment exit.
			segment_cost += ft.tiles_skipped * YAPF_TILE_LENGTH;
			tile = ft.new_tile;
			td = next_td;
		}
		node.segment_last_tile = tile;
		node.segment_last_td = td;
		node.cost = node.parent->cost + segment_cost;
		return true;
	}
	bool PfCalcEstimate(Node &node) { node.estimate = node.cost; return true; }
	char TransportTypeChar() const { return 'r'; }
	int CostCalcs() const { return this->stats_cost_calcs; }
};
}
TEST_CASE("Road YAPF low-limit timing and exit-key replacement match reference", "[road][rust]")
{
	FollowerFixture map;
	for (uint32_t t = 0; t < map.tiles.size; ++t) map.Tile(t, MP_ROAD, 15);
	auto leaves = probe_leaves; leaves.shared = &map.shared; leaves.tile = RoadLimit::ProbeTile;
	for (int limit : {0, 1, 2, 7}) for (uint32_t distance : {2U, 10U}) for (int max_cost : {0, 70, 71, 100}) {
		CAPTURE(limit, distance, max_cost);
		RoadLimit::goal = RoadLimit::start - distance;
		map.tiles.base[RoadLimit::goal].m5 = 0x82;
		RoadLimit::Search reference(limit, max_cost);
		bool found = reference.FindPath(nullptr);
		auto *best = reference.GetBestNode();
		OpenTTDRoadYapfInput input{}; input.map_x = input.map_y = 64; input.max_nodes = limit; input.tile = RoadLimit::start;
		auto result = openttd_rust_road_yapf_depot(&input, &leaves, map.Vehicle(), max_cost);
		REQUIRE(result.found == static_cast<uint8_t>(found));
		REQUIRE(result.tile == (best != nullptr ? best->segment_last_tile.base() : UINT32_MAX));
		REQUIRE(result.cost == (best != nullptr ? best->cost : 0));
		REQUIRE(result.rounds == reference.num_steps);
		REQUIRE(result.open == reference.nodes.OpenCount());
		REQUIRE(result.closed == reference.nodes.ClosedCount());
		REQUIRE(result.calcs == reference.CostCalcs());
		map.tiles.base[RoadLimit::goal].m5 = 15;
	}
}

/* Vehicle/type limits are observed once per accepted continuation, after
 * transient rejection, destination, junction and cost-limit checks. */
namespace RoadOrder {
inline uint32_t goal;
inline std::vector<char> limits;
OpenTTDRoadYapfTile Tile(const void *, uint32_t tile, uint8_t, bool) noexcept {
	OpenTTDRoadYapfTile out{}; out.type = FollowerFixture::active->shared.tile_type(tile); out.depot = tile == goal; out.depot_dir = DIAGDIR_SW; return out;
}
}
TEST_CASE("Road YAPF observes accepted traversal once after transient exits", "[road][rust]")
{
	enum Mode { Rejected, Junction, Destination, MaxCost, Accepted, Startup };
	for (Mode mode : {Rejected, Junction, Destination, MaxCost, Accepted, Startup}) {
		CAPTURE(mode);
		FollowerFixture map;
		/* One-way NE road: a missing next tile cannot be replaced by a reversal. */
		for (uint32_t t : {130U, 129U, 128U, 127U}) map.Tile(t, MP_ROAD, ROAD_X | 0x20);
		if (mode == Rejected) map.Tile(128, MP_CLEAR);
		if (mode == Junction) map.tiles.base[128].m5 = 15;
		RoadOrder::goal = mode == Startup ? 130 : mode == Destination ? 129 : 128;
		/* Raising tile 128's north corner gives it SLOPE_N under ROAD_X, so its
		 * centre is above flat tile 129 and the accepted step pays the uphill penalty. */
		map.tiles.base[128].height = 1;
		map.shared.apply_foundation = GetRustSharedServices().apply_foundation;
		OpenTTDRoadGroundServices road{};
		road.all_bits = [](uint32_t t) noexcept { return static_cast<RoadBits>(FollowerFixture::active->tiles.base[t].m5 & 15); };
		map.movement.road = &road;
		RoadOrder::limits.clear();
		map.physics.display_max_speed = [](OpenTTDVehicle *) noexcept -> int32_t { RoadOrder::limits.push_back('D'); return 100; };
		map.movement.order_speed = [](OpenTTDVehicle *) noexcept -> uint16_t { RoadOrder::limits.push_back('O'); return 40; };
		map.movement.road_type_speed = [](RoadType) noexcept -> uint16_t { RoadOrder::limits.push_back('R'); return 40; };
		auto leaves = probe_leaves; leaves.shared = &map.shared; leaves.tile = RoadOrder::Tile;
		OpenTTDRoadYapfInput input{}; input.map_x = input.map_y = 64; input.tile = 130; input.slope = 17;
		auto result = openttd_rust_road_yapf_depot(&input, &leaves, map.Vehicle(), mode == MaxCost ? 99 : 0);
		if (mode == Accepted) {
			CHECK((RoadOrder::limits == std::vector<char>{'D', 'O', 'R'}));
			CHECK(result.found == 1); CHECK(result.cost == 417);
		} else {
			CHECK(RoadOrder::limits.empty());
			if (mode == Destination) { CHECK(result.found == 1); CHECK(result.cost == 100); }
			if (mode == Startup) { CHECK(result.found == 1); CHECK(result.cost == 0); }
			if (mode == Rejected || mode == MaxCost) CHECK(result.found == 0);
		}
	}
}

#endif
