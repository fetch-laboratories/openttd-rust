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
#include "../safeguards.h"

namespace {
struct HeapNode { int32_t estimate; bool operator<(const HeapNode &other) const { return this->estimate < other.estimate; } };
OpenTTDRoadYapfTile ProbeTile(const void *, uint32_t, uint8_t, bool) noexcept { OpenTTDRoadYapfTile tile{}; tile.type = 5; tile.station_type = 2; return tile; }
OpenTTDRoadYapfFollow RoadProbeFollow(const void *, uint32_t, uint8_t) noexcept { return {}; }
uint16_t Tracks(const void *ctx, uint32_t, uint8_t) noexcept { return *static_cast<const uint16_t *>(ctx); }
OpenTTDRoadYapfTraversal Traversal(const void *, uint32_t, uint32_t) noexcept { OpenTTDRoadYapfTraversal out{}; out.max_speed = INT32_MAX; return out; }
uint32_t RoadProbeClosest(const void *, uint16_t, uint8_t) noexcept { return 0; }
OpenTTDRoadYapfArea Area(const void *, uint16_t) noexcept { return {}; }
const OpenTTDRoadYapfLeaves probe_leaves{
	.tile = ProbeTile,
	.follow = RoadProbeFollow,
	.tracks = Tracks,
	.traversal = Traversal,
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
			auto result = openttd_rust_road_yapf_choose(state, &input, &leaves, &tracks, 0, entry, TRACKDIR_BIT_Y_NW, true);
			bool reachable = (tracks & DiagdirReachesTrackdirs(static_cast<DiagDirection>(entry))) != 0;
			REQUIRE(result.direction == (reachable ? td : static_cast<uint8_t>(TRACKDIR_Y_NW)));
			REQUIRE(result.found == static_cast<uint8_t>(reachable));
		}
	}
	openttd_rust_road_destroy(state);
}

/* Loaded settings clamp max_search_nodes to >=500. This deliberately injected
 * scalar follower graph reaches small limits and open-key replacement, using
 * the unchanged generic reference loop rather than a second search algorithm. */
#undef WITH_RUST
#include "../pathfinder/yapf/yapf_node_road.hpp"
#define WITH_RUST
#include "../pathfinder/yapf/yapf_base.hpp"
namespace RoadLimit {
inline uint32_t goal;
inline std::vector<std::pair<uint32_t, uint8_t>> trace;
OpenTTDRoadYapfTile ProbeTile(const void *, uint32_t tile, uint8_t, bool) noexcept
{
	OpenTTDRoadYapfTile out{}; out.type = 2; out.depot = tile == goal; return out;
}
OpenTTDRoadYapfFollow Follow(const void *, uint32_t tile, uint8_t td) noexcept
{
	trace.emplace_back(tile, td);
	return {tile + 1, 0, TRACKDIR_BIT_X_NE | TRACKDIR_BIT_LOWER_E, true};
}
uint16_t Tracks(const void *, uint32_t, uint8_t) noexcept { return TRACKDIR_BIT_X_NE; }
struct Follower {};
class Search;
struct Types {
	using Tpf = Search;
	using TrackFollower = Follower;
	using NodeList = CRoadNodeList;
	using VehicleType = RoadVehicle;
};
class Search : public CYapfBaseT<Types> {
public:
	int max_cost;
	Search(int limit, int max_cost) : max_cost(max_cost) {
		this->max_search_nodes = limit;
		Node &node = this->CreateNewNode(); node.Set(nullptr, TileIndex(0), TRACKDIR_X_NE, true);
		this->AddStartupNode(node);
	}
	bool PfNodeCacheFetch(Node &) { return false; }
	bool PfDetectDestination(Node &node) { return node.segment_last_tile.base() == goal; }
	void PfFollowNode(Node &parent) {
		auto follower = Follow(nullptr, parent.segment_last_tile.base(), parent.segment_last_td);
		for (Trackdir td : {TRACKDIR_X_NE, TRACKDIR_LOWER_E}) {
			Node &node = this->CreateNewNode(); node.Set(&parent, TileIndex(follower.tile), td, true);
			this->AddNewNode(node, Follower{});
		}
	}
	bool PfCalcCost(Node &node, const Follower *) {
		node.cost = node.parent->cost + (IsDiagonalTrackdir(node.key.td) ? 100 : 71);
		if (this->PfDetectDestination(node)) return true;
		if (this->max_cost > 0 && node.cost > this->max_cost) return false;
		Follow(nullptr, node.key.tile.base(), node.key.td);
		return true;
	}
	bool PfCalcEstimate(Node &node) { node.estimate = node.cost; return true; }
	char TransportTypeChar() const { return 'r'; }
	int CostCalcs() const { return this->stats_cost_calcs; }
};
}
TEST_CASE("Road YAPF low-limit timing and exit-key replacement match reference", "[road][rust]")
{
	auto leaves = probe_leaves;
	leaves.tile = RoadLimit::ProbeTile; leaves.follow = RoadLimit::Follow; leaves.tracks = RoadLimit::Tracks;
	for (int limit : {0, 1, 2, 7}) {
		for (uint32_t destination : {2U, 10U}) for (int max_cost : {0, 70, 71, 100}) {
			RoadLimit::goal = destination; RoadLimit::trace.clear();
			RoadLimit::Search reference(limit, max_cost);
			bool found = reference.FindPath(nullptr);
			auto expected_trace = RoadLimit::trace;
			auto *best = reference.GetBestNode();
			OpenTTDRoadYapfInput input{}; input.map_x = input.map_y = 64; input.max_nodes = limit;
			RoadLimit::trace.clear();
			auto result = openttd_rust_road_yapf_depot(&input, &leaves, nullptr, max_cost);
			REQUIRE(result.found == static_cast<uint8_t>(found));
			REQUIRE(RoadLimit::trace == expected_trace);
			REQUIRE(result.tile == (best != nullptr ? best->segment_last_tile.base() : UINT32_MAX));
			REQUIRE(result.cost == (best != nullptr ? best->cost : 0));
			REQUIRE(result.rounds == reference.num_steps);
			REQUIRE(result.open == reference.nodes.OpenCount());
			REQUIRE(result.closed == reference.nodes.ClosedCount());
			REQUIRE(result.calcs == reference.CostCalcs());
		}
	}
}

/* Saves do not record rejected traversal queries or endpoint-cache validity.
 * These callbacks describe a tiny immutable graph; the actual Rust search runs
 * unchanged. D marks destination-only reads, T includes tile cost facts, and
 * V/N identifies the old/next tiles passed to the accepted traversal query. */
namespace RoadOrder {
enum Mode { Rejected, Junction, Loop, Destination, MaxCost, DepotEntry, Accepted, LongSegment, Startup };
struct Context { Mode mode; std::vector<std::pair<char, uint32_t>> trace; };
OpenTTDRoadYapfTile Tile(const void *ctx, uint32_t tile, uint8_t, bool cost) noexcept
{
	auto &c = *const_cast<Context *>(static_cast<const Context *>(ctx));
	c.trace.emplace_back(cost ? 'T' : 'D', tile);
	OpenTTDRoadYapfTile out{}; out.type = 2;
	out.depot = (tile == 0 && c.mode == Startup) || (tile == 1 && (c.mode == Destination || c.mode == DepotEntry)) || tile == 2;
	out.depot_dir = DIAGDIR_SW;
	return out;
}
OpenTTDRoadYapfFollow Follow(const void *ctx, uint32_t tile, uint8_t) noexcept
{
	auto &c = *const_cast<Context *>(static_cast<const Context *>(ctx));
	c.trace.emplace_back('F', tile);
	if (tile == 0) return {1, 0, TRACKDIR_BIT_X_NE, true}; // Outer expansion must not request facts.
	if (c.mode == Rejected || c.mode == DepotEntry) return {};
	if (c.mode == Loop) return {1, 0, TRACKDIR_BIT_X_NE, true};
	return {2, c.mode == LongSegment ? 4096 : 2, static_cast<uint16_t>(c.mode == Junction ? TRACKDIR_BIT_X_NE | TRACKDIR_BIT_LOWER_E : TRACKDIR_BIT_X_NE), true};
}
OpenTTDRoadYapfTraversal Traversal(const void *ctx, uint32_t tile, uint32_t next) noexcept
{
	auto &c = *const_cast<Context *>(static_cast<const Context *>(ctx));
	c.trace.emplace_back('V', tile); c.trace.emplace_back('N', next);
	return {.current_z = 0, .next_z = 8, .display_speed = 100, .max_speed = 40, .min_speed = 90, .order_speed = 40};
}
}
TEST_CASE("Road YAPF observes accepted traversal once after transient exits", "[road][rust]")
{
	using namespace RoadOrder;
	auto leaves = probe_leaves;
	leaves.tile = RoadOrder::Tile; leaves.follow = RoadOrder::Follow; leaves.tracks = RoadLimit::Tracks; leaves.traversal = RoadOrder::Traversal;
	for (Mode mode : {Rejected, Junction, Loop, Destination, MaxCost, DepotEntry, Accepted, LongSegment, Startup}) {
		CAPTURE(mode);
		Context context{mode, {}};
		OpenTTDRoadYapfInput input{}; input.map_x = input.map_y = 64; input.slope = 17; input.dest_tile = 10;
		OpenTTDRoadYapfResult result;
		if (mode == DepotEntry) {
			auto *state = openttd_rust_road_new();
			result = openttd_rust_road_yapf_choose(state, &input, &leaves, &context, 0, DIAGDIR_NE, TRACKDIR_BIT_X_NE, false);
			openttd_rust_road_destroy(state);
		} else {
			result = openttd_rust_road_yapf_depot(&input, &leaves, &context, mode == MaxCost ? 99 : 0);
		}
		using Trace = std::vector<std::pair<char, uint32_t>>;
		if (mode == Accepted || mode == LongSegment) {
			const Trace expected{{'D', 0}, {'F', 0}, {'T', 1}, {'F', 1}, {'V', 1}, {'N', 2}, {mode == LongSegment ? 'D' : 'T', 2}};
			REQUIRE(context.trace == expected);
			REQUIRE(result.found == 1);
			REQUIRE(result.tile == 2);
			REQUIRE(result.cost == (mode == Accepted ? 1717 : 615717));
		} else {
			REQUIRE(std::none_of(context.trace.begin(), context.trace.end(), [](auto item) { return item.first == 'V' || item.first == 'N'; }));
			REQUIRE(std::count(context.trace.begin(), context.trace.end(), std::pair{'T', 1U}) == (mode == Startup ? 0 : 1));
			if (mode == Destination || mode == MaxCost || mode == Startup) {
				REQUIRE(std::count(context.trace.begin(), context.trace.end(), std::pair{'F', 1U}) == 0);
			}
			if (mode == DepotEntry) REQUIRE(std::count(context.trace.begin(), context.trace.end(), std::pair{'F', 1U}) == 1);
			if (mode == Destination) REQUIRE(result.cost == 100);
			if (mode == Startup) { REQUIRE(result.found == 1); REQUIRE(result.cost == 0); }
			if (mode == Loop || mode == MaxCost) REQUIRE(result.found == 0);
		}
	}
}

#endif
