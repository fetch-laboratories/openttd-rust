/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraphjob.cpp Definition of link graph job classes used for cargo distribution. */

#include "../stdafx.h"
#include "linkgraphjob.h"
#include "../rust/services_linkgraph.h"
#include "linkgraphschedule.h"
#include "../rust/linkgraph_ffi.h"
#include "../rust/linkgraph_schedule_ffi.h"
#include "../timer/timer_game_tick.h"

#include "../safeguards.h"

/* Initialize the link-graph-job-pool */
LinkGraphJobPool _link_graph_job_pool("LinkGraphJob");

LinkGraphJob::LinkGraphJob() : record(openttd_rust_linkgraph_job_new(nullptr, LinkGraphID::Invalid().base(), reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph))) { this->record->shell = this; _link_graph_job_pool.BindRecord(this->index.base(), this, this->record); }
LinkGraphJob::LinkGraphJob(const LinkGraph &orig) : record(openttd_rust_linkgraph_job_new(orig.RustState(), orig.index.base(), reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph))) { this->record->shell = this; _link_graph_job_pool.BindRecord(this->index.base(), this, this->record); }

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

/** GoodsEntry::ClearData, for the job's application. */
static void GoodsEntryClearData(GoodsEntry *ge) noexcept { ge->ClearData(); }

/** GoodsEntry::GetData().IsEmpty(), for the job's application. */
static bool GoodsEntryDataIsEmpty(const GoodsEntry *ge) noexcept { return ge->GetData().IsEmpty(); }

/** &GoodsEntry::GetOrCreateData().flows, for the job's application. */
static OpenTTDCargoFlowMap *GoodsEntryGetOrCreateDataFlows(GoodsEntry *ge) noexcept { return ge->GetOrCreateData().flows.RustState(); }

static OpenTTDCargoFlowServices CargoFlowServices(LinkGraphJob *job)
{
	return {
		.clear_data = GoodsEntryClearData,
		.data_is_empty = GoodsEntryDataIsEmpty,
		.get_or_create_data_flows = GoodsEntryGetOrCreateDataFlows,
		.job = job == nullptr ? nullptr : &job->Record(),
		.links = &GetRustStationLinks(),
		.stations = &GetRustStationServices(),
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
		auto services = CargoFlowServices(this);
		openttd_rust_flow_apply(&services);
	}
	_link_graph_job_pool.RetireRecord(this->index.base(), this, this->record);
	openttd_rust_linkgraph_job_delete(this->record);
}


static_assert(DT_MANUAL == 0 && DT_ASYMMETRIC == 1 && DT_SYMMETRIC == 2);
static_assert(StationID::Invalid().base() == UINT16_MAX);
static_assert(Ticks::DAY_TICKS == 74);

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoFlowServices.hpp"
bool CheckAbi_linkgraphjob() { return AbiServicesComplete(CargoFlowServices(nullptr)); }
