/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraphschedule.cpp Definition of link graph schedule used for cargo distribution. */

#include "../stdafx.h"
#include "linkgraphschedule.h"
#include "linkgraphjob.h"
#include "../framerate_type.h"
#include "../command_func.h"
#include "../network/network.h"
#include "../misc_cmd.h"
#include "../rust/linkgraph_schedule_ffi.h"
#include "../rust/command_services.h"
#include "../rust/services_ffi.h"
#include "../rust/vehicle_ffi.h"
#include "../rust/services_linkgraph.h"

#include "../safeguards.h"

/**
 * Static instance of LinkGraphSchedule.
 * Note: This instance is created on task start.
 *       Lazy creation on first usage results in a data race between the CDist threads.
 */
/* static */ LinkGraphSchedule LinkGraphSchedule::instance;

static_assert(LinkGraphSchedule::SPAWN_JOIN_TICK == 21 && EconomyTime::SECONDS_PER_DAY == 2);
constinit const OpenTTDScheduleServices LinkGraphSchedule::services = {
	.allocation_failure = +[]() noexcept { NOT_REACHED(); },
	.assert_graph = +[]([[maybe_unused]] const OpenTTDLiveGraph *g) noexcept { assert(g->shell == LinkGraph::Get(g->shell->index)); },
	.begin_measurement = +[](void *storage, bool client) noexcept {
		if (client) static_cast<std::optional<PerformanceMeasurer> *>(storage)->emplace(PFE_GL_LINKGRAPH);
		else PerformanceMeasurer::SetInactive(PFE_GL_LINKGRAPH);
	},
	.can_allocate = +[]() noexcept { return LinkGraphJob::CanAllocateItem(); },
	.destroy_job = +[](OpenTTDLinkGraphJobRecord *j) noexcept { delete j->shell; },
	.graph_pool = &_link_graph_pool.descriptor,
	.job_pool = &_link_graph_job_pool.descriptor,
	.new_job = +[](const OpenTTDLiveGraph *g) noexcept { return &(new LinkGraphJob(*g->shell))->Record(); },
	.shared = &_rust_shared_services,
	.start_thread = +[](OpenTTDLinkGraphJobRecord *j) noexcept { return j->shell->StartThread(); },
	.vehicle = &_rust_vehicle_services,
};
void LinkGraphSchedule::Queue(LinkGraph *g) { openttd_rust_schedule_queue(&services, g->RustState()); }
void LinkGraphSchedule::SpawnAll() { openttd_rust_schedule_spawn_all(&services); }
void LinkGraphSchedule::Clear() { openttd_rust_schedule_clear(); }
void LinkGraphSchedule::ShiftDates(TimerGameEconomy::Date interval) { openttd_rust_schedule_shift(&services, interval.base()); }

void AfterLoad_LinkGraphPauseControl() { openttd_rust_schedule_after_load(); }

LinkGraphSchedule::LinkGraphSchedule() = default;
LinkGraphSchedule::~LinkGraphSchedule() { this->Clear(); }
