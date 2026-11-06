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
using LinkGraphJobPool = RustPool<LinkGraphJob, LinkGraphJobID, 32>;
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
	bool IsJobAborted() const { return this->Record().aborted.load(std::memory_order_acquire); }
	CargoType Cargo() const { return this->Record().graph->cargo; }
	LinkGraphID LinkGraphIndex() const { return LinkGraphID(this->Record().graph_index); }
	OpenTTDLiveGraph *Graph() const { return this->Record().graph; }
private:
	OpenTTDLinkGraphJobRecord *record;
	std::thread thread{};
	void JoinThread();
	bool StartThread();
};

#endif /* LINKGRAPHJOB_H */
