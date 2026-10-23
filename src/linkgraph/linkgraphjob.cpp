/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraphjob.cpp Definition of link graph job classes used for cargo distribution. */

#include "../stdafx.h"
#include "../core/pool_func.hpp"
#include "../window_func.h"
#include "linkgraphjob.h"
#include "linkgraphschedule.h"
#ifdef WITH_RUST
#include "../rust/linkgraph_ffi.h"
#include "../timer/timer_game_tick.h"
#endif

#include "../safeguards.h"

/* Initialize the link-graph-job-pool */
LinkGraphJobPool _link_graph_job_pool("LinkGraphJob");
INSTANTIATE_POOL_METHODS(LinkGraphJob)

/**
 * Static instance of an invalid path.
 * Note: This instance is created on task start.
 *       Lazy creation on first usage results in a data race between the CDist threads.
 */
#ifndef WITH_RUST
/* static */ Path *Path::invalid_path = new Path(INVALID_NODE, true);
#endif

/**
 * Create a link graph job from a link graph. The link graph will be copied so
 * that the calculations don't interfere with the normal operations on the
 * original. The job is immediately started.
 * @param orig Original LinkGraph to be copied.
 */
LinkGraphJob::LinkGraphJob(const LinkGraph &orig) :
		/* Copying the link graph here also copies its index member.
		 * This is on purpose. */
		link_graph(orig),
		settings(_settings_game.linkgraph),
		join_date(TimerGameEconomy::date + (_settings_game.linkgraph.recalc_time / EconomyTime::SECONDS_PER_DAY))
{
}

/**
 * Erase all flows originating at a specific station.
 * @param from StationID to erase flows for.
 */
void LinkGraphJob::EraseFlows(StationID from)
{
	for (NodeID node_id = 0; node_id < this->Size(); ++node_id) {
		(*this)[node_id].flows.erase(from);
	}
}

/**
 * Spawn a thread if possible and run the link graph job in the thread. If
 * that's not possible run the job right now in the current thread.
 */
void LinkGraphJob::SpawnThread()
{
	if (!StartNewThread(&this->thread, "ottd:linkgraph", &(LinkGraphSchedule::Run), this)) {
		/* Of course this will hang a bit.
		 * On the other hand, if you want to play games which make this hang noticeably
		 * on a platform without threads then you'll probably get other problems first.
		 * OK:
		 * If someone comes and tells me that this hangs for them, I'll implement a
		 * smaller grained "Step" method for all handlers and add some more ticks where
		 * "Step" is called. No problem in principle. */
		LinkGraphSchedule::Run(this);
	}
}

/**
 * Join the calling thread with this job's thread if threading is enabled.
 */
void LinkGraphJob::JoinThread()
{
	if (this->thread.joinable()) {
		this->thread.join();
	}
}

#ifdef WITH_RUST
/** Call-scoped native leaves; no owned graph query reenters Rust from a leaf. */
struct CargoFlowContext { LinkGraphJob *job; CargoType cargo; };
static uint32_t ReadCargoFlowJob(void *context, uint8_t op, uint16_t node, uint16_t edge_index) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	switch (op) {
		case 0: return LinkGraph::IsValidID(job.LinkGraphIndex());
		case 3: return Station::IsValidID(job[node].Station());
		case 4: return static_cast<uint32_t>(job[node].edges.size());
		case 5: return job[node].edges[edge_index].Flow();
		case 6: return job[node].edges[edge_index].base.dest_node;
		case 8: return _settings_game.linkgraph.GetDistributionType(ctx.cargo) == DT_MANUAL;
		case 9: return Station::Get(job[node].Station())->goods[ctx.cargo].GetData().IsEmpty();
		case 10: return Station::Get(job[node].Station())->goods[ctx.cargo].link_graph.base();
		case 11: return Station::Get(job[node].Station())->goods[ctx.cargo].node;
		case 12: return job.LinkGraphIndex().base();
		default: NOT_REACHED();
	}
}

static OpenTTDCargoFlowMap *CargoJobFlows(void *context, uint16_t node) noexcept
{
	return (*static_cast<CargoFlowContext *>(context)->job)[node].flows.RustState();
}

static OpenTTDCargoFlowMap *CargoLiveFlows(void *context, uint16_t node) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	return Station::Get(job[node].Station())->goods[ctx.cargo].GetOrCreateData().flows.RustState();
}

static void CargoFlowReroute(void *context, uint16_t node, uint16_t via) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	Station *st = Station::Get(job[node].Station());
	RerouteCargo(st, ctx.cargo, StationID(via), st->index);
}

static void CargoFlowFinish(void *context, uint16_t node, uint8_t clear) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	Station *st = Station::Get(job[node].Station());
	if (clear != 0) st->goods[ctx.cargo].ClearData();
	InvalidateWindowData(WC_STATION_VIEW, st->index, ctx.cargo);
}

static OpenTTDCargoFlowServices CargoFlowServices(CargoFlowContext *context)
{
	LinkGraphJob *job = context->job;
	LinkGraph *live = job == nullptr ? nullptr : GetRustLiveGraphServices().get_if_valid(job->LinkGraphIndex().base());
	return {
		.snapshot = job == nullptr ? nullptr : job->Graph().RustState(),
		.live_graph = live == nullptr ? nullptr : live->RustState(),
		.context = context,
		.read = ReadCargoFlowJob,
		.job_flows = CargoJobFlows,
		.live_flows = CargoLiveFlows,
		.reroute = CargoFlowReroute,
		.finish = CargoFlowFinish,
	};
}
#endif /* WITH_RUST */

/**
 * Join the link graph job and destroy it.
 */
LinkGraphJob::~LinkGraphJob()
{
	this->JoinThread();

	/* Don't update stuff from other pools, when everything is being removed.
	 * Accessing other pools may be invalid. */
	if (CleaningPool()) return;

	/* If the job has been aborted, the job state is invalid.
	 * This should never be reached, as once the job has been marked as aborted
	 * the only valid job operation is to clear the LinkGraphJob pool. */
	assert(!this->IsJobAborted());

#ifdef WITH_RUST
	CargoFlowContext context{this, this->Cargo()};
	auto services = CargoFlowServices(&context);
	openttd_rust_flow_apply(&services);
#else
	/* Link graph has been merged into another one. */
	if (!LinkGraph::IsValidID(this->link_graph.index)) return;

	uint16_t size = this->Size();
	for (NodeID node_id = 0; node_id < size; ++node_id) {
		NodeAnnotation &from = this->nodes[node_id];

		/* The station can have been deleted. Remove all flows originating from it then. */
		Station *st = Station::GetIfValid(from.base.station);
		if (st == nullptr) {
			this->EraseFlows(from.base.station);
			continue;
		}

		/* Link graph merging and station deletion may change around IDs. Make
		 * sure that everything is still consistent or ignore it otherwise. */
		GoodsEntry &ge = st->goods[this->Cargo()];
		if (ge.link_graph != this->link_graph.index || ge.node != node_id) {
			this->EraseFlows(from.base.station);
			continue;
		}

		LinkGraph *lg = LinkGraph::Get(ge.link_graph);
		FlowStatMap &flows = from.flows;
		FlowStatMap &geflows = ge.GetOrCreateData().flows;

		for (const auto &edge : from.edges) {
			if (edge.Flow() == 0) continue;
			NodeID dest_id = edge.base.dest_node;
			StationID to = this->nodes[dest_id].base.station;
			Station *st2 = Station::GetIfValid(to);
			if (st2 == nullptr || st2->goods[this->Cargo()].link_graph != this->link_graph.index ||
					st2->goods[this->Cargo()].node != dest_id ||
					!(*lg)[node_id].HasEdgeTo(dest_id) ||
					(*lg)[node_id][dest_id].LastUpdate() == EconomyTime::INVALID_DATE) {
				/* Edge has been removed. Delete flows. */
				std::vector<StationID> erased = flows.DeleteFlows(to);
				/* Delete old flows for source stations which have been deleted
				 * from the new flows. This avoids flow cycles between old and
				 * new flows. */
				for (const StationID &station : erased) geflows.erase(station);
			} else if ((*lg)[node_id][dest_id].last_unrestricted_update == EconomyTime::INVALID_DATE) {
				/* Edge is fully restricted. */
				flows.RestrictFlows(to);
			}
		}

		/* Swap shares and invalidate ones that are completely deleted. Don't
		 * really delete them as we could then end up with unroutable cargo
		 * somewhere. Do delete them and also reroute relevant cargo if
		 * automatic distribution has been turned off for that cargo. */
		for (FlowStatMap::iterator it(geflows.begin()); it != geflows.end();) {
			FlowStatMap::iterator new_it = flows.find(it->first);
			if (new_it == flows.end()) {
				if (_settings_game.linkgraph.GetDistributionType(this->Cargo()) != DT_MANUAL) {
					it->second.Invalidate();
					++it;
				} else {
					FlowStat shares(StationID::Invalid(), 1);
					it->second.SwapShares(shares);
					geflows.erase(it++);
					for (FlowStat::SharesMap::const_iterator shares_it(shares.GetShares()->begin());
							shares_it != shares.GetShares()->end(); ++shares_it) {
						RerouteCargo(st, this->Cargo(), shares_it->second, st->index);
					}
				}
			} else {
				it->second.SwapShares(new_it->second);
				flows.erase(new_it);
				++it;
			}
		}
		geflows.insert(flows.begin(), flows.end());
		if (ge.GetData().IsEmpty()) ge.ClearData();
		InvalidateWindowData(WC_STATION_VIEW, st->index, this->Cargo());
	}
#endif /* WITH_RUST */
}

/**
 * Initialize the link graph job: Resize nodes and edges and populate them.
 * This is done after the constructor so that we can do it in the calculation
 * thread without delaying the main game.
 */
void LinkGraphJob::Init()
{
	uint size = this->Size();
	this->nodes.reserve(size);
	for (uint i = 0; i < size; ++i) {
		this->nodes.emplace_back(this->link_graph[i], this->link_graph.Size());
	}
}

#ifndef WITH_RUST
/**
 * Add this path as a new child to the given base path, thus making this path
 * a "fork" of the base path.
 * @param base Path to fork from.
 * @param cap Maximum capacity of the new leg.
 * @param free_cap Remaining free capacity of the new leg.
 * @param dist Distance of the new leg.
 */
void Path::Fork(Path *base, uint cap, int free_cap, uint dist)
{
	this->capacity = std::min(base->capacity, cap);
	this->free_capacity = std::min(base->free_capacity, free_cap);
	this->distance = base->distance + dist;
	assert(this->distance > 0);
	if (this->parent != base) {
		this->Detach();
		this->parent = base;
		this->parent->num_children++;
	}
	this->origin = base->origin;
}

/**
 * Push some flow along a path and register the path in the nodes it passes if
 * successful.
 * @param new_flow Amount of flow to push.
 * @param job Link graph job this node belongs to.
 * @param max_saturation Maximum saturation of edges.
 * @return Amount of flow actually pushed.
 */
uint Path::AddFlow(uint new_flow, LinkGraphJob &job, uint max_saturation)
{
	if (this->parent != nullptr) {
		LinkGraphJob::EdgeAnnotation &edge = job[this->parent->node][this->node];
		if (max_saturation != UINT_MAX) {
			uint usable_cap = edge.base.capacity * max_saturation / 100;
			if (usable_cap > edge.Flow()) {
				new_flow = std::min(new_flow, usable_cap - edge.Flow());
			} else {
				return 0;
			}
		}
		new_flow = this->parent->AddFlow(new_flow, job, max_saturation);
		if (this->flow == 0 && new_flow > 0) {
			job[this->parent->node].paths.push_front(this);
		}
		edge.AddFlow(new_flow);
	}
	this->flow += new_flow;
	return new_flow;
}

/**
 * Create a leg of a path in the link graph.
 * @param n Id of the link graph node this path passes.
 * @param source If true, this is the first leg of the path.
 */
Path::Path(NodeID n, bool source) :
	distance(source ? 0 : UINT_MAX),
	capacity(source ? UINT_MAX : 0),
	free_capacity(source ? INT_MAX : INT_MIN),
	flow(0), node(n), origin(source ? n : INVALID_NODE),
	num_children(0), parent(nullptr)
{}


#endif /* !WITH_RUST */

#ifdef WITH_RUST
static_assert(DT_MANUAL == 0 && DT_ASYMMETRIC == 1 && DT_SYMMETRIC == 2);
static_assert(StationID::Invalid().base() == UINT16_MAX);
static_assert(Ticks::DAY_TICKS == 74);

/** Copy only job-local immutable inputs; Rust owns all algorithm state. */
void LinkGraphJob::RunRust()
{
	this->Init();
	const auto &s = this->settings;
	auto runtime = this->JoinDate() - s.recalc_time / CalendarTime::SECONDS_PER_DAY - this->LastCompression() + 1;
	OpenTTDLinkGraphSettings settings{s.accuracy, s.demand_distance, s.demand_size, s.short_path_saturation, static_cast<uint32_t>(s.GetDistributionType(this->Cargo())),
		IsCargoInClass(this->Cargo(), CargoClass::Passengers) || IsCargoInClass(this->Cargo(), CargoClass::Mail) || IsCargoInClass(this->Cargo(), CargoClass::Express), Map::MaxX(), Map::MaxY(), static_cast<uint32_t>(runtime.base())};
	auto abort = [](const void *ctx) noexcept -> uint8_t { return static_cast<const LinkGraphJob *>(ctx)->IsJobAborted(); };
	std::unique_ptr<OpenTTDLinkGraphResult, decltype(&openttd_rust_linkgraph_destroy)> result(
		openttd_rust_live_graph_run(this->link_graph.RustState(), &settings, this, abort), &openttd_rust_linkgraph_destroy);
	if (this->IsJobAborted()) return;
	size_t count = 0;
	const auto *shares = openttd_rust_linkgraph_shares(result.get(), &count);
	for (size_t i = 0; i < count;) {
		const auto &first = shares[i];
		FlowStat::SharesMap map;
		do {
			if (shares[i].has_share != 0) map.emplace(shares[i].cumulative, StationID(shares[i].via));
			++i;
		} while (i < count && shares[i].node == first.node && shares[i].origin == first.origin);
		this->nodes[first.node].flows.emplace(StationID(first.origin), FlowStat(std::move(map), first.unrestricted));
	}
	const auto *flows = openttd_rust_linkgraph_edges(result.get());
	size_t edge_id = 0;
	for (auto &node : this->nodes) {
		for (auto &edge : node.edges) edge.flow = flows[edge_id++];
	}
}
#endif /* WITH_RUST */

#ifdef WITH_RUST
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoFlowServices.hpp"
bool CheckAbi_linkgraphjob() { CargoFlowContext context{nullptr, INVALID_CARGO}; return AbiServicesComplete(CargoFlowServices(&context)); }
#endif
