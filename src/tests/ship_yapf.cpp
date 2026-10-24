/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_yapf.cpp Heap comparison and ship ownership ABI check. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "ship_yapf_protocol.hpp"
#include "../ship.h"
#include "../pathfinder/yapf/yapf_base.hpp"
#include "../pathfinder/yapf/yapf_node.hpp"
#include "../pathfinder/yapf/nodelist.hpp"
/* The unchanged generic reference search checks the exact follow-before-limit
 * timing that the injected low-level fixture reaches at 5120 closed nodes. */
namespace ShipYapfLimitReference {
struct Key {
	uint32_t index;
	int CalcHash() const { return this->index; }
	bool operator==(const Key &other) const { return this->index == other.index; }
};
struct Node : CYapfNodeT<Key, Node> {
	void Set(uint32_t index, Node *parent)
	{
		this->key = {index};
		this->hash_next = nullptr;
		this->parent = parent;
		this->cost = this->estimate = 0;
	}
};
struct Follower {};
class Search;
struct Types {
	using Tpf = Search;
	using TrackFollower = Follower;
	using NodeList = ::NodeList<Node, 10, 12>;
	using VehicleType = Ship;
};
class Search : public CYapfBaseT<Types> {
public:
	uint32_t follows = 0, goal;
	Search(int limit = 5120, uint32_t goal = 6000) : goal(goal)
	{
		this->max_search_nodes = limit;
		Node &node = this->CreateNewNode();
		node.Set(0, nullptr);
		this->AddStartupNode(node);
	}
	bool PfNodeCacheFetch(Node &) { return false; }
	void PfFollowNode(Node &parent)
	{
		++this->follows;
		Node &node = this->CreateNewNode();
		node.Set(parent.key.index + 1, &parent);
		this->AddNewNode(node, Follower{});
	}
	bool PfCalcCost(Node &node, const Follower *) { node.cost = node.parent->cost + 100; return true; }
	bool PfCalcEstimate(Node &node) { node.estimate = node.cost; return true; }
	bool PfDetectDestination(Node &node) { return node.key.index == this->goal; }
	char TransportTypeChar() const { return 'w'; }
};
}
TEST_CASE("Ship YAPF Rust heap and path ownership")
{
	ShipYapfLimitReference::Search reference;
	REQUIRE_FALSE(reference.FindPath(nullptr));
	ShipYapfProbe::Run([](bool value) { CHECK(value); }, reference.follows);
}
TEST_CASE("Ship YAPF fixed limit timing against unchanged reference search")
{
	ShipYapfLimitReference::Search reference;
	CHECK_FALSE(reference.FindPath(nullptr));
	CHECK(reference.follows == 5121);
	CHECK(reference.nodes.ClosedCount() == 5120);
	CHECK(reference.nodes.GetBestOpenNode()->key.index == 5120);
	for (int limit : {256, 65536}) {
		ShipYapfLimitReference::Search region_reference{limit, static_cast<uint32_t>(limit + 2)};
		CHECK_FALSE(region_reference.FindPath(nullptr));
		CHECK(region_reference.follows == static_cast<uint32_t>(limit + 1));
		CHECK(region_reference.nodes.ClosedCount() == limit);
	}
	/* Detecting a destination precedes the following/limit check. */
	ShipYapfLimitReference::Search boundary{5120, 5120};
	CHECK(boundary.FindPath(nullptr));
	CHECK(boundary.follows == 5120);
}
#endif
