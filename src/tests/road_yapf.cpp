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
#include "../pathfinder/pathfinder_type.h"
#include "../track_func.h"
#include "../roadveh.h"
#include "../rust/road_yapf_ffi.h"
#include "../rust/movement_ffi.h"
#include "../tile_type.h"
#include "../landscape.h"
#include "../order_type.h"
#include "reference-yapf_node_road.hpp"
#include "../pathfinder/yapf/yapf_base.hpp"
#include "../pathfinder/follow_track.hpp"
#include "follower_fixture.hpp"
#include "follower_probe.hpp"
#include "../safeguards.h"

TEST_CASE("Road YAPF origin masks match shared track constants", "[road][rust]")
{
	/* Each case's start tile carries one road trackdir, as its road bits and one-way state
	 * leave it. Its exit neighbour is a full junction and the destination tile, so a
	 * reachable origin finds the destination one step on. */
	constexpr uint32_t start = 32 + 32 * 64;
	constexpr uint8_t bits[]{10, 5, 9, 6, 3, 12};
	constexpr int32_t exit_offset[]{-1, 64, 1, -64};
	OpenTTDRoadState *state = openttd_rust_road_new();
	for (uint8_t entry = 0; entry < 4; ++entry) {
		for (uint8_t td = 0; td < 14; ++td) {
			if (td == 6 || td == 7) continue;
			CAPTURE(entry, td);
			FollowerFixture map;
			map.Tile(start, MP_ROAD, bits[td & 7] | ((td > 7 ? 1 : 2) << 4));
			const uint32_t exit = start + exit_offset[TrackdirToExitdir(static_cast<Trackdir>(td))];
			map.Tile(exit, MP_ROAD, ROAD_ALL);
			OpenTTDRoadYapfInput input{};
			input.dest_tile = exit;
			input.order_type = OT_NOTHING;
			auto result = openttd_rust_road_yapf_choose(state, &input, &map.shared, map.Vehicle(), start, entry, TRACKDIR_BIT_Y_NW, true);
			bool reachable = ((1U << td) & DiagdirReachesTrackdirs(static_cast<DiagDirection>(entry))) != 0;
			REQUIRE(result.direction == (reachable ? td : static_cast<uint8_t>(TRACKDIR_Y_NW)));
			REQUIRE(result.found == static_cast<uint8_t>(reachable));
		}
	}
	openttd_rust_road_destroy(state);
}

/* Loaded games clamp max_search_nodes. The generic native search still checks
 * low limits and open-key replacement, now using the production follower on a
 * concrete junction grid instead of an injected transition callback. */
namespace RoadLimit {
inline uint32_t goal, start = 32 + 32 * 64;
OpenTTDTestFollow Follow(uint32_t tile, uint8_t td)
{
	return OpenTTDTestFollowOnce(FollowerFixture::active->shared, TRANSPORT_ROAD, FollowerFixture::active->Vehicle(), 0, 0, true, tile, td);
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
	for (int limit : {0, 1, 2, 7}) for (uint32_t distance : {2U, 10U}) for (int max_cost : {0, 70, 71, 100}) {
		CAPTURE(limit, distance, max_cost);
		RoadLimit::goal = RoadLimit::start - distance;
		map.tiles.base[RoadLimit::goal].m5 = 0x82;
		RoadLimit::Search reference(limit, max_cost);
		bool found = reference.FindPath(nullptr);
		auto *best = reference.GetBestNode();
		OpenTTDRoadYapfInput input{}; input.max_nodes = limit; input.tile = RoadLimit::start;
		auto result = openttd_rust_road_yapf_depot(&input, &map.shared, map.Vehicle(), max_cost);
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
inline std::vector<char> limits;
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
		/* The goal is a depot facing south-west. In Junction mode the junction takes the goal's
		 * tile, there is no depot, and the junction's exits are unenterable, so that no
		 * continuation is accepted. */
		if (mode == Junction) {
			map.tiles.base[128].m5 = ROAD_ALL;
			map.Tile(127, MP_CLEAR);
		} else if (mode != Rejected) {
			map.tiles.base[mode == Startup ? 130 : mode == Destination ? 129 : 128].m5 = 0x80 | DIAGDIR_SW;
		}
		/* Raising tile 128's north corner gives it SLOPE_N under ROAD_X, so its
		 * centre is above flat tile 129 and the accepted step pays the uphill penalty. */
		map.tiles.base[128].height = 1;
		map.shared.apply_foundation = GetRustSharedServices().apply_foundation;
		RoadOrder::limits.clear();
		map.physics.display_max_speed = [](OpenTTDVehicle *) noexcept -> int32_t { RoadOrder::limits.push_back('D'); return 100; };
		map.orders.vehicle = [](void *) noexcept { RoadOrder::limits.push_back('O'); return &FollowerFixture::active->order_state; };
		map.movement.road_type_speed = [](RoadType) noexcept -> uint16_t { RoadOrder::limits.push_back('R'); return 40; };
		OpenTTDRoadYapfInput input{}; input.tile = 130; input.slope = 17;
		auto result = openttd_rust_road_yapf_depot(&input, &map.shared, map.Vehicle(), mode == MaxCost ? 99 : 0);
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
