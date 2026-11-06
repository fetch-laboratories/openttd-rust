/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraphjob.h Declaration of link graph job classes used for cargo distribution. */

#ifndef LINKGRAPHJOB_H
#define LINKGRAPHJOB_H

#include "../thread.h"
#include "linkgraph.h"
#include <atomic>

class LinkGraphJob;

/** Type of the pool for link graph jobs. */
using LinkGraphJobPool = Pool<LinkGraphJob, LinkGraphJobID, 32>;
/** The actual pool with link graph jobs. */
extern LinkGraphJobPool _link_graph_job_pool;

/**
 * Class for calculation jobs to be run on link graphs.
 */
class LinkGraphJob : public LinkGraphJobPool::PoolItem<&_link_graph_job_pool>{
public:
	friend SaveLoadTable GetLinkGraphJobDesc();
	friend class LinkGraphSchedule;

	LinkGraphJob();
	LinkGraphJob(const LinkGraph &orig);
	~LinkGraphJob();
	OpenTTDLinkGraphJobRecord &Record() { return *this->record; }
	const OpenTTDLinkGraphJobRecord &Record() const { return *this->record; }
	bool IsJobCompleted() const { return this->Record().completed.load(std::memory_order_acquire); }
	bool IsJobAborted() const { return this->Record().aborted.load(std::memory_order_acquire); }
	void AbortJob() { this->Record().aborted.store(true, std::memory_order_release); }
	bool IsScheduledToBeJoined() const { return this->JoinDate() <= TimerGameEconomy::date; }
	TimerGameEconomy::Date JoinDate() const { return TimerGameEconomy::Date{this->Record().join_date}; }
	void ShiftJoinDate(TimerGameEconomy::Date interval) { this->Record().join_date += interval.base(); }
	NodeID Size() const { return static_cast<NodeID>(this->Record().graph->nodes.size()); }
	CargoType Cargo() const { return this->Record().graph->cargo; }
	LinkGraphID LinkGraphIndex() const { return LinkGraphID(this->Record().graph_index); }
	OpenTTDLiveGraph *Graph() const { return this->Record().graph; }
	void RunRust();
private:
	OpenTTDLinkGraphJobRecord *record;
	std::thread thread{};
	void JoinThread();
	void SpawnThread();
};

#endif /* LINKGRAPHJOB_H */
