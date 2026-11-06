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
#include "../rust/services_linkgraph.h"
#include "linkgraphschedule.h"
#include "../rust/linkgraph_ffi.h"
#include "../rust/linkgraph_schedule_ffi.h"
#include "../timer/timer_game_tick.h"

#include "../safeguards.h"

/* Initialize the link-graph-job-pool */
LinkGraphJobPool _link_graph_job_pool("LinkGraphJob");
INSTANTIATE_POOL_METHODS(LinkGraphJob)

LinkGraphJob::LinkGraphJob() : record(openttd_rust_linkgraph_job_new(nullptr, LinkGraphID::Invalid().base(), reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph))) { this->record->shell = this; }
LinkGraphJob::LinkGraphJob(const LinkGraph &orig) : record(openttd_rust_linkgraph_job_new(orig.RustState(), orig.index.base(), reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph))) { this->record->shell = this; }

/** Start the native worker; Rust executes the synchronous failure fallback. */
bool LinkGraphJob::StartThread()
{
	return StartNewThread(&this->thread, "ottd:linkgraph", &openttd_rust_schedule_run, &LinkGraphSchedule::services, this->record);
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

/** Call-scoped native leaves; no owned graph query reenters Rust from a leaf. */
struct CargoFlowContext { LinkGraphJob *job; CargoType cargo; };
static uint32_t ReadCargoFlowJob(void *context, uint8_t op, uint16_t node, [[maybe_unused]] uint16_t edge_index) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	switch (op) {
		case 3: return Station::IsValidID(StationID(job.Graph()->nodes[node].station));
		case 8: return _settings_game.linkgraph.GetDistributionType(ctx.cargo) == DT_MANUAL;
		case 9: return Station::Get(StationID(job.Graph()->nodes[node].station))->goods[ctx.cargo].GetData().IsEmpty();
		case 10: return RustStationLinkGraphIndex(Station::Get(StationID(job.Graph()->nodes[node].station)), ctx.cargo);
		case 11: return Station::Get(StationID(job.Graph()->nodes[node].station))->goods[ctx.cargo].node;
		default: NOT_REACHED();
	}
}

static OpenTTDCargoFlowMap *CargoLiveFlows(void *context, uint16_t node) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	return Station::Get(StationID(job.Graph()->nodes[node].station))->goods[ctx.cargo].GetOrCreateData().flows.RustState();
}

static void CargoFlowReroute(void *context, uint16_t node, uint16_t via) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	Station *st = Station::Get(StationID(job.Graph()->nodes[node].station));
	RerouteCargo(st, ctx.cargo, StationID(via), st->index);
}

static void CargoFlowFinish(void *context, uint16_t node, uint8_t clear) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	auto &job = *ctx.job;
	Station *st = Station::Get(StationID(job.Graph()->nodes[node].station));
	if (clear != 0) st->goods[ctx.cargo].ClearData();
	InvalidateWindowData(WC_STATION_VIEW, st->index, ctx.cargo);
}

static OpenTTDCargoFlowServices CargoFlowServices(CargoFlowContext *context)
{
	LinkGraphJob *job = context->job;
	OpenTTDLiveGraph *live = job == nullptr ? nullptr : GetRustLiveGraphServices().get_if_valid(job->LinkGraphIndex().base());
	return {
		.job = job == nullptr ? nullptr : &job->Record(),
		.live_graph = live,
		.context = context,
		.read = ReadCargoFlowJob,
		.live_flows = CargoLiveFlows,
		.reroute = CargoFlowReroute,
		.finish = CargoFlowFinish,
	};
}

/**
 * Join the link graph job and destroy it.
 */
LinkGraphJob::~LinkGraphJob()
{
	this->JoinThread();

	if (!CleaningPool()) {
		assert(!this->IsJobAborted());
		CargoFlowContext context{this, this->Cargo()};
		auto services = CargoFlowServices(&context);
		openttd_rust_flow_apply(&services);
	}
	openttd_rust_linkgraph_job_delete(this->record);
}


static_assert(DT_MANUAL == 0 && DT_ASYMMETRIC == 1 && DT_SYMMETRIC == 2);
static_assert(StationID::Invalid().base() == UINT16_MAX);
static_assert(Ticks::DAY_TICKS == 74);

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoFlowServices.hpp"
bool CheckAbi_linkgraphjob() { CargoFlowContext context{nullptr, INVALID_CARGO}; return AbiServicesComplete(CargoFlowServices(&context)); }
