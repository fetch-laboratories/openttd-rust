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
#ifndef WITH_RUST
#include "init.h"
#include "demands.h"
#include "mcf.h"
#include "flowmapper.h"
#endif
#include "../framerate_type.h"
#include "../command_func.h"
#include "../network/network.h"
#include "../misc_cmd.h"
#ifdef WITH_RUST
#include "../rust/linkgraph_schedule_ffi.h"
#include "../rust/services_ffi.h"
#include "../rust/services_linkgraph.h"
#endif

#include "../safeguards.h"

/**
 * Static instance of LinkGraphSchedule.
 * Note: This instance is created on task start.
 *       Lazy creation on first usage results in a data race between the CDist threads.
 */
/* static */ LinkGraphSchedule LinkGraphSchedule::instance;

#ifdef WITH_RUST
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
	.next_graph = +[](uint32_t *from) noexcept -> LinkGraph * {
		for (LinkGraph *g : LinkGraph::Iterate(*from)) { *from = g->index.base() + 1U; return g; }
		return nullptr;
	},
	.next_job = +[](uint32_t *from) noexcept -> LinkGraphJob * {
		for (LinkGraphJob *j : LinkGraphJob::Iterate(*from)) { *from = j->index.base() + 1U; return j; }
		return nullptr;
	},
	.shift_job = +[](LinkGraphJob *j, int32_t interval) noexcept { j->ShiftJoinDate(ScheduleDate(interval)); },
	.post_pause = +[](bool pause) noexcept { Command<CMD_PAUSE>::Post(PauseMode::LinkGraph, pause); },
	.set_pause = +[]() noexcept { _pause_mode.Set(PauseMode::LinkGraph); },
	.begin_measurement = +[](void *storage, bool client) noexcept {
		if (client) static_cast<std::optional<PerformanceMeasurer> *>(storage)->emplace(PFE_GL_LINKGRAPH);
		else PerformanceMeasurer::SetInactive(PFE_GL_LINKGRAPH);
	},
};
static OpenTTDScheduleTime ScheduleTime() { return {_rust_shared_services.economy_date(), TimerGameEconomy::date_fract, _settings_game.linkgraph.recalc_interval}; }
void LinkGraphSchedule::Queue(LinkGraph *g) { openttd_rust_schedule_queue(&services, g); }
void LinkGraphSchedule::Dequeue(LinkGraph *g) { openttd_rust_schedule_dequeue(g); }
void LinkGraphSchedule::SpawnAll() { openttd_rust_schedule_spawn_all(&services); }
void LinkGraphSchedule::Clear() { openttd_rust_schedule_clear(&services); }
void LinkGraphSchedule::ShiftDates(TimerGameEconomy::Date interval) { openttd_rust_schedule_shift(&services, interval.base()); }
void StateGameLoop_LinkGraphPauseControl() { openttd_rust_schedule_pause(&LinkGraphSchedule::services, ScheduleTime(), _pause_mode.Test(PauseMode::LinkGraph), _pause_mode.None()); }
void AfterLoad_LinkGraphPauseControl() { openttd_rust_schedule_after_load(&LinkGraphSchedule::services); }
void OnTick_LinkGraph()
{
	std::optional<PerformanceMeasurer> measurement;
	openttd_rust_schedule_tick(&LinkGraphSchedule::services, ScheduleTime(), _networking && !_network_server, &measurement);
}
LinkGraphSchedule::LinkGraphSchedule() = default;
LinkGraphSchedule::~LinkGraphSchedule() { this->Clear(); }
#else
/**
 * Start the next job in the schedule.
 */
void LinkGraphSchedule::SpawnNext()
{
	if (this->schedule.empty()) return;
	LinkGraph *next = this->schedule.front();
	LinkGraph *first = next;
	while (next->Size() < 2) {
		this->schedule.splice(this->schedule.end(), this->schedule, this->schedule.begin());
		next = this->schedule.front();
		if (next == first) return;
	}
	assert(next == LinkGraph::Get(next->index));
	this->schedule.pop_front();
	if (LinkGraphJob::CanAllocateItem()) {
		LinkGraphJob *job = new LinkGraphJob(*next);
		job->SpawnThread();
		this->running.push_back(job);
	} else {
		NOT_REACHED();
	}
}

/**
 * Check if the next job is supposed to be finished, but has not yet completed.
 * @return True if job should be finished by now but is still running, false if not.
 */
bool LinkGraphSchedule::IsJoinWithUnfinishedJobDue() const
{
	if (this->running.empty()) return false;
	const LinkGraphJob *next = this->running.front();
	return next->IsScheduledToBeJoined() && !next->IsJobCompleted();
}

/**
 * Join the next finished job, if available.
 */
void LinkGraphSchedule::JoinNext()
{
	if (this->running.empty()) return;
	LinkGraphJob *next = this->running.front();
	if (!next->IsScheduledToBeJoined()) return;
	this->running.pop_front();
	LinkGraphID id = next->LinkGraphIndex();
	delete next; // implicitly joins the thread
	if (LinkGraph::IsValidID(id)) {
		LinkGraph *lg = LinkGraph::Get(id);
		this->Dequeue(lg); // Dequeue to avoid double-queueing recycled IDs.
		this->Queue(lg);
	}
}

#endif /* !WITH_RUST */

/**
 * Run all handlers for the given Job.
 * @param job Pointer to a link graph job.
 */
/* static */ void LinkGraphSchedule::Run(LinkGraphJob *job)
{
#ifdef WITH_RUST
	if (job->IsJobAborted()) return;
	job->RunRust();
	if (job->IsJobAborted()) return;
#else
	for (const auto &handler : instance.handlers) {
		if (job->IsJobAborted()) return;
		handler->Run(*job);
	}

#endif
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

#ifndef WITH_RUST
/**
 * Start all threads in the running list. This is only useful for save/load.
 * Usually threads are started when the job is created.
 */
void LinkGraphSchedule::SpawnAll()
{
	for (auto &it : this->running) {
		it->SpawnThread();
	}
}

/**
 * Clear all link graphs and jobs from the schedule.
 */
/* static */ void LinkGraphSchedule::Clear()
{
	for (auto &it : instance.running) {
		it->AbortJob();
	}
	instance.running.clear();
	instance.schedule.clear();
}

/**
 * Shift all dates (join dates and edge annotations) of link graphs and link
 * graph jobs by the number of days given.
 * @param interval Number of days to be added or subtracted.
 */
void LinkGraphSchedule::ShiftDates(TimerGameEconomy::Date interval)
{
	for (LinkGraph *lg : LinkGraph::Iterate()) lg->ShiftDates(interval);
	for (LinkGraphJob *lgj : LinkGraphJob::Iterate()) lgj->ShiftJoinDate(interval);
}

/**
 * Create a link graph schedule and initialize its handlers.
 */
LinkGraphSchedule::LinkGraphSchedule()
{
#ifndef WITH_RUST
	this->handlers[0] = std::make_unique<InitHandler>();
	this->handlers[1] = std::make_unique<DemandHandler>();
	this->handlers[2] = std::make_unique<MCFHandler<MCF1stPass>>();
	this->handlers[3] = std::make_unique<FlowMapper>(false);
	this->handlers[4] = std::make_unique<MCFHandler<MCF2ndPass>>();
	this->handlers[5] = std::make_unique<FlowMapper>(true);
#endif
}

/**
 * Delete a link graph schedule and its handlers.
 */
LinkGraphSchedule::~LinkGraphSchedule()
{
	this->Clear();
}

/**
 * Pause the game if in 2 TimerGameEconomy::date_fract ticks, we would do a join with the next
 * link graph job, but it is still running.
 * The check is done 2 TimerGameEconomy::date_fract ticks early instead of 1, as in multiplayer
 * calls to DoCommandP are executed after a delay of 1 TimerGameEconomy::date_fract tick.
 * If we previously paused, unpause if the job is now ready to be joined with.
 */
void StateGameLoop_LinkGraphPauseControl()
{
	if (_pause_mode.Test(PauseMode::LinkGraph)) {
		/* We are paused waiting on a job, check the job every tick. */
		if (!LinkGraphSchedule::instance.IsJoinWithUnfinishedJobDue()) {
			Command<CMD_PAUSE>::Post(PauseMode::LinkGraph, false);
		}
	} else if (_pause_mode.None() &&
			TimerGameEconomy::date_fract == LinkGraphSchedule::SPAWN_JOIN_TICK - 2 &&
			TimerGameEconomy::date.base() % (_settings_game.linkgraph.recalc_interval / EconomyTime::SECONDS_PER_DAY) == (_settings_game.linkgraph.recalc_interval / EconomyTime::SECONDS_PER_DAY) / 2 &&
			LinkGraphSchedule::instance.IsJoinWithUnfinishedJobDue()) {
		/* Perform check two TimerGameEconomy::date_fract ticks before we would join, to make
		 * sure it also works in multiplayer. */
		Command<CMD_PAUSE>::Post(PauseMode::LinkGraph, true);
	}
}

/**
 * Pause the game on load if we would do a join with the next link graph job,
 * but it is still running, and it would not be caught by a call to
 * StateGameLoop_LinkGraphPauseControl().
 */
void AfterLoad_LinkGraphPauseControl()
{
	if (LinkGraphSchedule::instance.IsJoinWithUnfinishedJobDue()) {
		_pause_mode.Set(PauseMode::LinkGraph);
	}
}

/**
 * Spawn or join a link graph job or compress a link graph if any link graph is
 * due to do so.
 */
void OnTick_LinkGraph()
{
	if (TimerGameEconomy::date_fract != LinkGraphSchedule::SPAWN_JOIN_TICK) return;
	TimerGameEconomy::Date offset{TimerGameEconomy::date.base() % (_settings_game.linkgraph.recalc_interval / EconomyTime::SECONDS_PER_DAY)};
	if (offset == 0) {
		LinkGraphSchedule::instance.SpawnNext();
	} else if (offset == (_settings_game.linkgraph.recalc_interval / EconomyTime::SECONDS_PER_DAY) / 2) {
		if (!_networking || _network_server) {
			PerformanceMeasurer::SetInactive(PFE_GL_LINKGRAPH);
			LinkGraphSchedule::instance.JoinNext();
		} else {
			PerformanceMeasurer framerate(PFE_GL_LINKGRAPH);
			LinkGraphSchedule::instance.JoinNext();
		}
	}
}


#endif /* WITH_RUST */
