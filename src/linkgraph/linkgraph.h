/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraph.h Declaration of link graph classes used for cargo distribution. */

#ifndef LINKGRAPH_H
#define LINKGRAPH_H

#include "../rust/pool.hpp"
#include "../station_base.h"
#include "../cargotype.h"
#include "../timer/timer_game_economy.h"
#include "../saveload/saveload.h"
#include "linkgraph_type.h"
#include <utility>
#include "../rust/linkgraph_ffi.h"
#include "../rust/services_ffi.h"
#include <span>

class LinkGraph;

/**
 * Type of the pool for link graph components. Each station can be in at up to
 * 32 link graphs. So we allow for plenty of them to be created.
 */
using LinkGraphPool = RustPool<LinkGraph, LinkGraphID, 32>;
/** The actual pool with link graphs. */
extern LinkGraphPool _link_graph_pool;

/**
 * A connected component of a link graph. Contains a complete set of stations
 * connected by links as nodes and edges. Each component also holds a copy of
 * the link graph settings at the time of its creation. The global settings
 * might change between the creation and join time so we can't rely on them.
 */
class LinkGraph : public LinkGraphPool::PoolItem<&_link_graph_pool> {
public:
	/**
	 * An edge in the link graph. Corresponds to a link between two stations.
	 */
	struct BaseEdge : OpenTTDLiveEdge {
		BaseEdge(NodeID dest_node = INVALID_NODE) { openttd_rust_live_edge_init(this, dest_node); }
	};
	static uint32_t EdgeTravelTime(const OpenTTDLiveEdge &edge) { return static_cast<uint32_t>(edge.travel_time_sum / edge.capacity); }

	/**
	 * Node of the link graph. contains all relevant information from the associated
	 * station. It's copied so that the link graph job can work on its own data set
	 * in a separate thread.
	 */
	/** Stack-only node handle; it owns no allocation. */
	struct BaseNode {
		OpenTTDLiveGraph *owner;
		NodeID id;
		const OpenTTDLiveNode &Record() const { return this->owner->nodes[this->id]; }
		uint Supply() const { return this->Record().supply; }
		StationID Station() const { return StationID(this->Record().station); }
		void UpdateSupply(uint supply) { openttd_rust_live_node_update_supply(this->owner, this->id, supply); }
		void UpdateLocation(TileIndex xy) { openttd_rust_live_node_location(this->owner, this->id, xy.base()); }
		void UpdateEdge(NodeID to, uint capacity, uint usage, uint32_t time, EdgeUpdateModes modes) { assert(capacity > 0); assert(usage <= capacity); openttd_rust_live_node_update_edge(this->owner, this->id, to, capacity, usage, time, modes.base(), GetRustLiveGraphServices().assertions); }
		void AddEdge(NodeID to, uint capacity, uint usage, uint32_t time, EdgeUpdateModes modes) { assert(!this->HasEdgeTo(to)); openttd_rust_live_node_add_edge(this->owner, this->id, to, capacity, usage, time, modes.base()); }
		bool HasEdgeTo(NodeID to) const { const auto &edges = this->Record().edges; auto it = std::ranges::lower_bound(edges, to, {}, &OpenTTDLiveEdge::dest_node); return it != edges.end() && it->dest_node == to; }
		const OpenTTDLiveEdge &operator[](NodeID to) const { const auto &edges = this->Record().edges; auto it = std::ranges::lower_bound(edges, to, {}, &OpenTTDLiveEdge::dest_node); assert(it != edges.end() && it->dest_node == to); return *it; }
		std::span<const OpenTTDLiveEdge> Edges() const { return this->Record().edges; }
	};

	typedef std::vector<BaseNode> NodeVector;

	/** Minimum effective distance for timeout calculation. */
	static const uint MIN_TIMEOUT_DISTANCE = 32;

	/** Number of days before deleting links served only by vehicles stopped in depot. */
	static constexpr TimerGameEconomy::Date STALE_LINK_DEPOT_TIMEOUT{1024};

	/** Minimum number of days between subsequent compressions of a LG. */
	static constexpr TimerGameEconomy::Date COMPRESSION_INTERVAL{256};

	/**
	 * Scale a value from a link graph of age orig_age for usage in one of age
	 * target_age. Make sure that the value stays > 0 if it was > 0 before.
	 * @param val Value to be scaled.
	 * @param target_age Age of the target link graph.
	 * @param orig_age Age of the original link graph.
	 * @return scaled value.
	 */
	static inline uint Scale(uint val, TimerGameEconomy::Date target_age, TimerGameEconomy::Date orig_age)
	{
		return openttd_rust_live_graph_scale(val, target_age.base(), orig_age.base());
	}

	LinkGraph() : owner(openttd_rust_live_graph_new_empty()) { this->owner->shell = this; _link_graph_pool.BindRecordIfOwned(this, this->owner); }
	LinkGraph(CargoType cargo) : owner(openttd_rust_live_graph_new(cargo)) { this->owner->shell = this; _link_graph_pool.BindRecordIfOwned(this, this->owner); }
	LinkGraph(const LinkGraph &) = delete;
	~LinkGraph() { _link_graph_pool.RetireUnboundRecord(this->owner); openttd_rust_live_graph_delete(this->owner); }
	LinkGraph &operator=(const LinkGraph &) = delete;
	void Init(uint size) { assert(this->Size() == 0); openttd_rust_live_graph_init(this->owner, size); }
	void ShiftDates(TimerGameEconomy::Date interval) { openttd_rust_live_graph_shift(this->owner, interval.base()); }
	void Compress() { openttd_rust_live_graph_compress(this->owner); }
	void Merge(LinkGraph *other) { openttd_rust_live_graph_merge(this->owner, other->owner, &GetRustLiveGraphServices()); }
	OpenTTDLiveGraph *RustState() const { return this->owner; }
	OpenTTDLiveGraph &Record() { return *this->owner; }
	const OpenTTDLiveGraph &Record() const { return *this->owner; }

	/* Splitting link graphs is intentionally not implemented.
	 * The overhead in determining connectedness would probably outweigh the
	 * benefit of having to deal with smaller graphs. In real world examples
	 * networks generally grow. Only rarely a network is permanently split.
	 * Reacting to temporary splits here would obviously create performance
	 * problems and detecting the temporary or permanent nature of splits isn't
	 * trivial. */

	/**
	 * Get a node with the specified id.
	 * @param num ID of the node.
	 * @return the Requested node.
	 */
	inline BaseNode operator[](NodeID num) const { return {this->owner, num}; }

	/**
	 * Get the current size of the component.
	 * @return Size.
	 */
	inline NodeID Size() const { return static_cast<NodeID>(this->Record().nodes.size()); }

	/**
	 * Get date of last compression.
	 * @return Date of last compression.
	 */
	inline TimerGameEconomy::Date LastCompression() const { return TimerGameEconomy::Date{this->Record().last_compression}; }

	/**
	 * Get the cargo type this component's link graph refers to.
	 * @return Cargo type.
	 */
	inline CargoType Cargo() const { return this->Record().cargo; }

	/**
	 * Scale a value to its monthly equivalent, based on last compression.
	 * @param base Value to be scaled.
	 * @return Scaled value.
	 */
	inline uint Monthly(uint base) const
	{
		return openttd_rust_live_graph_monthly(this->owner, base);
	}

	NodeID AddNode(const ::Station *st) { return openttd_rust_live_graph_add_station(this->owner, GetRustLiveGraphServices().station_id(st), &GetRustLiveGraphServices()); }
	void RemoveNode(NodeID id) { assert(id < this->Size()); openttd_rust_live_graph_remove(this->owner, id, &GetRustLiveGraphServices()); }

protected:
	friend SaveLoadTable GetLinkGraphDesc();
	friend SaveLoadTable GetLinkGraphJobDesc();
	friend class SlLinkgraphNode;
	friend class SlLinkgraphEdge;
	friend class LinkGraphJob;

	OpenTTDLiveGraph *owner; ///< Sole owner, including for immutable job snapshots.
};

#endif /* LINKGRAPH_H */
