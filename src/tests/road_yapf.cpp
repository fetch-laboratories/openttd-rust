/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road_yapf.cpp Native road heap ordering and ABI evidence. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../misc/binaryheap.hpp"
#include "../track_func.h"
#include "../roadveh.h"
#ifdef WITH_RUST
#include "../rust/road_yapf_ffi.h"
#include "../rust/movement_ffi.h"
#include "../safeguards.h"

namespace {
struct HeapNode { int32_t estimate; bool operator<(const HeapNode &other) const { return this->estimate < other.estimate; } };
OpenTTDRoadYapfTile ProbeTile(const void *, uint32_t, uint8_t, bool) noexcept { OpenTTDRoadYapfTile tile{}; tile.type = 5; tile.station_type = 2; return tile; }
uint16_t fixture_dirs = 1;
OpenTTDFoundationSlope (*height_query)(uint32_t) noexcept = [](uint32_t) noexcept -> OpenTTDFoundationSlope { return {.z = 0, .slope = SLOPE_FLAT}; };
const OpenTTDSharedServices *MovementWorld()
{
	static OpenTTDSharedServices world{};
	static OpenTTDMovementServices movement{};
	world.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	world.tile_type = [](uint32_t) noexcept -> uint8_t { return 2; };
	world.map_log_x = []() noexcept -> uint32_t { return 6; };
	world.road_tile_kind = [](uint32_t) noexcept { return static_cast<RoadTileType>(0); };
	world.normal_road_bits = [](uint32_t, uint8_t) noexcept -> RoadBits {
		constexpr RoadBits bits[]{static_cast<RoadBits>(10), static_cast<RoadBits>(5), static_cast<RoadBits>(9), static_cast<RoadBits>(6), static_cast<RoadBits>(3), static_cast<RoadBits>(12)};
		return bits[std::countr_zero(static_cast<unsigned>(fixture_dirs)) & 7];
	};
	world.disallowed_road_directions = [](uint32_t) noexcept -> uint8_t { return fixture_dirs > 255 ? 1 : 2; };
	world.has_road_works = [](uint32_t) noexcept { return false; };
	movement.shared = &world;
	movement.has_tile_road_type = [](uint32_t, RoadTramType) noexcept { return true; };
	movement.normal_road = [](uint32_t) noexcept { return true; };
	movement.pixel_location = [](int32_t x, int32_t y) noexcept -> OpenTTDPixelLocation { return {.tile = ((uint32_t(y) >> 4) << 6) + (uint32_t(x) >> 4), .kind = 2}; };
	movement.normal_road_status_tail = [](uint32_t) noexcept -> OpenTTDNormalRoadStatusTail { return {.disallowed = uint8_t(fixture_dirs > 255 ? 1 : 2), .road_works = false}; };
	movement.pixel_slope = [](uint32_t tile) noexcept { return height_query(tile); };
	return &world;
}
uint32_t RoadProbeClosest(const void *, uint16_t, uint8_t) noexcept { return 0; }
OpenTTDRoadYapfArea Area(const void *, uint16_t) noexcept { return {}; }
const OpenTTDRoadYapfLeaves probe_leaves{
	.shared = MovementWorld(),
	.road_kind = [](const void *) noexcept { return RTT_ROAD; },
	.tile = ProbeTile,
	.closest = RoadProbeClosest,
	.area = Area,
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
			fixture_dirs = tracks;
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
	openttd_rust_follower_init(&ft, &s, TRANSPORT_ROAD, reinterpret_cast<Vehicle *>(FollowerFixture::active->Vehicle()), 0, 0, true, false, true);
	if (!openttd_rust_follower_follow(&s, &ft, tile, td)) ft.new_td_bits = TRACKDIR_BIT_NONE;
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
		node.cost = node.parent->cost + (IsDiagonalTrackdir(node.key.td) ? 100 : 71);
		if (this->PfDetectDestination(node)) return true;
		if (this->max_cost > 0 && node.cost > this->max_cost) return false;
		return true; // Every interior tile is a junction; a segment ends here.
	}
	bool PfCalcEstimate(Node &node) { node.estimate = node.cost; return true; }
	char TransportTypeChar() const { return 'r'; }
	int CostCalcs() const { return this->stats_cost_calcs; }
};
}
TEST_CASE("Road YAPF low-limit timing and exit-key replacement match reference", "[road][rust]")
{
	FollowerFixture map;
	for (uint32_t t = 0; t < map.base.size(); ++t) map.Tile(t, MP_ROAD, 15);
	auto leaves = probe_leaves; leaves.shared = &map.shared; leaves.tile = RoadLimit::ProbeTile;
	for (int limit : {0, 1, 2, 7}) for (uint32_t distance : {2U, 10U}) for (int max_cost : {0, 70, 71, 100}) {
		RoadLimit::goal = RoadLimit::start - distance;
		map.base[RoadLimit::goal].m5 = 0x82;
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
		map.base[RoadLimit::goal].m5 = 15;
	}
}

/* Production traversal observes heights and vehicle/type limits only after
 * transient rejection, destination, junction and cost-limit checks. */
namespace RoadOrder {
inline uint32_t goal;
inline std::vector<uint32_t> heights;
OpenTTDRoadYapfTile Tile(const void *, uint32_t tile, uint8_t, bool) noexcept {
	OpenTTDRoadYapfTile out{}; out.type = FollowerFixture::active->shared.tile_type(tile); out.depot = tile == goal; out.depot_dir = DIAGDIR_SW; return out;
}
OpenTTDFoundationSlope Height(uint32_t tile) noexcept { heights.push_back(tile); return {tile == 129 ? 0 : 8, SLOPE_FLAT}; }
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
		if (mode == Junction) map.base[128].m5 = 15;
		RoadOrder::goal = mode == Startup ? 130 : mode == Destination ? 129 : 128;
		RoadOrder::heights.clear();
		map.movement.pixel_slope = RoadOrder::Height;
		map.movement.road_type_speed = [](RoadType) noexcept -> uint16_t { return 40; };
		auto leaves = probe_leaves; leaves.shared = &map.shared; leaves.tile = RoadOrder::Tile;
		OpenTTDRoadYapfInput input{}; input.map_x = input.map_y = 64; input.tile = 130; input.slope = 17;
		auto result = openttd_rust_road_yapf_depot(&input, &leaves, map.Vehicle(), mode == MaxCost ? 99 : 0);
		if (mode == Accepted) {
			CHECK((RoadOrder::heights == std::vector<uint32_t>{129, 128}));
			CHECK(result.found == 1); CHECK(result.cost == 417);
		} else {
			CHECK(RoadOrder::heights.empty());
			if (mode == Destination) { CHECK(result.found == 1); CHECK(result.cost == 100); }
			if (mode == Startup) { CHECK(result.found == 1); CHECK(result.cost == 0); }
			if (mode == Rejected || mode == MaxCost) CHECK(result.found == 0);
		}
	}
}

#endif
