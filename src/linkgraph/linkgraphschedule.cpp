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
#include "../rust/services_linkgraph.h"

#include "../safeguards.h"

/**
 * Static instance of LinkGraphSchedule.
 * Note: This instance is created on task start.
 *       Lazy creation on first usage results in a data race between the CDist threads.
 */
/* static */ LinkGraphSchedule LinkGraphSchedule::instance;

static_assert(LinkGraphSchedule::SPAWN_JOIN_TICK == 21 && EconomyTime::SECONDS_PER_DAY == 2);
using ScheduleDate = TimerGameEconomy::Date;
constinit const OpenTTDScheduleServices LinkGraphSchedule::services = {
	.graph_owner = LiveGraphOwner,
	.assert_graph = +[]([[maybe_unused]] const LinkGraph *g) noexcept { assert(g == LinkGraph::Get(g->index)); },
	.can_allocate = +[]() noexcept { return LinkGraphJob::CanAllocateItem(); },
	.allocation_failure = +[]() noexcept { NOT_REACHED(); },
	.new_job = +[](const LinkGraph *g) noexcept { return new LinkGraphJob(*g); },
	.spawn = +[](LinkGraphJob *j) noexcept { j->SpawnThread(); },
	.due = +[](const LinkGraphJob *j) noexcept { return j->IsScheduledToBeJoined(); },
	.completed = +[](const LinkGraphJob *j) noexcept { return j->IsJobCompleted(); },
	.graph_index = RustLinkGraphJobGraphIndex,
	.destroy_job = +[](LinkGraphJob *j) noexcept { delete j; },
	.graph_valid = RustLinkGraphValid,
	.graph_get = RustLinkGraphGet,
	.abort_job = +[](LinkGraphJob *j) noexcept { j->AbortJob(); },
	.next_graph = RustLinkGraphNext,
	.next_job = +[](uint32_t *from) noexcept -> LinkGraphJob * {
		for (LinkGraphJob *j : LinkGraphJob::Iterate(*from)) { *from = j->index.base() + 1U; return j; }
		return nullptr;
	},
	.shift_job = +[](LinkGraphJob *j, int32_t interval) noexcept { j->ShiftJoinDate(ScheduleDate(interval)); },
	.post_pause = RustPostPause,
	.begin_measurement = +[](void *storage, bool client) noexcept {
		if (client) static_cast<std::optional<PerformanceMeasurer> *>(storage)->emplace(PFE_GL_LINKGRAPH);
		else PerformanceMeasurer::SetInactive(PFE_GL_LINKGRAPH);
	},
};
OpenTTDScheduleTime RustScheduleTime() noexcept { return {TimerGameEconomy::date.base(), TimerGameEconomy::date_fract, _settings_game.linkgraph.recalc_interval}; }
void LinkGraphSchedule::Queue(LinkGraph *g) { openttd_rust_schedule_queue(&services, g); }
void LinkGraphSchedule::Dequeue(LinkGraph *g) { openttd_rust_schedule_dequeue(g); }
void LinkGraphSchedule::SpawnAll() { openttd_rust_schedule_spawn_all(&services); }
void LinkGraphSchedule::Clear() { openttd_rust_schedule_clear(&services); }
void LinkGraphSchedule::ShiftDates(TimerGameEconomy::Date interval) { openttd_rust_schedule_shift(&services, interval.base()); }

void AfterLoad_LinkGraphPauseControl() { openttd_rust_schedule_after_load(&LinkGraphSchedule::services); }

LinkGraphSchedule::LinkGraphSchedule() = default;
LinkGraphSchedule::~LinkGraphSchedule() { this->Clear(); }

/**
 * Run all handlers for the given Job.
 * @param job Pointer to a link graph job.
 */
/* static */ void LinkGraphSchedule::Run(LinkGraphJob *job)
{
	if (job->IsJobAborted()) return;
	job->RunRust();
	if (job->IsJobAborted()) return;
	/*
	 * Readers of this variable in another thread may see an out of date value.
	 * However this is OK as this will only happen just as a job is completing,
	 * and the real synchronisation is provided by the thread join operation.
	 * In the worst case the main thread will be paused for longer than
	 * strictly necessary before joining.
	 * This is just a hint variable to avoid performing the join excessively
	 * early and blocking the main thread.
	 */

	job->job_completed.store(true, std::memory_order_release);
}
