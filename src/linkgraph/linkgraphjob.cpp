/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraphjob.cpp Definition of link graph job classes used for cargo distribution. */

#include "../stdafx.h"
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

LinkGraphJob::LinkGraphJob() : record(openttd_rust_linkgraph_job_new(nullptr, LinkGraphID::Invalid().base(), reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph))) { this->record->shell = this; _link_graph_job_pool.BindRecordIfOwned(this, this->record); }
LinkGraphJob::LinkGraphJob(const LinkGraph &orig) : record(openttd_rust_linkgraph_job_new(orig.RustState(), orig.index.base(), reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph))) { this->record->shell = this; _link_graph_job_pool.BindRecordIfOwned(this, this->record); }

/** Start the native worker; Rust executes the synchronous failure fallback. */
bool LinkGraphJob::StartThread()
{
	return StartNewThread(&this->thread, "ottd:linkgraph", &openttd_rust_schedule_run, &LinkGraphSchedule::services, &this->Record());
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
static bool CargoDataIsEmpty(void *context, uint16_t node) noexcept
{
	const auto &ctx = *static_cast<CargoFlowContext *>(context);
	return Station::Get(StationID(ctx.job->Graph()->nodes[node].station))->goods[ctx.cargo].GetData().IsEmpty();
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
	LinkGraph *graph = job == nullptr ? nullptr : LinkGraph::GetIfValid(job->LinkGraphIndex());
	OpenTTDLiveGraph *live = graph == nullptr ? nullptr : graph->RustState();
	return {
		.job = job == nullptr ? nullptr : &job->Record(),
		.live_graph = live,
		.context = context,
		.stations = &GetRustStationServices(),
		.links = &GetRustStationLinks(),
		.data_is_empty = CargoDataIsEmpty,
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
	_link_graph_job_pool.RetireUnboundRecord(this->record);
	openttd_rust_linkgraph_job_delete(this->record);
}


static_assert(DT_MANUAL == 0 && DT_ASYMMETRIC == 1 && DT_SYMMETRIC == 2);
static_assert(StationID::Invalid().base() == UINT16_MAX);
static_assert(Ticks::DAY_TICKS == 74);

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoFlowServices.hpp"
bool CheckAbi_linkgraphjob() { CargoFlowContext context{nullptr, INVALID_CARGO}; return AbiServicesComplete(CargoFlowServices(&context)); }
