/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file linkgraph_schedule_staging.hpp Native-only LGRS exception and reference staging. */
#ifndef RUST_LINKGRAPH_SCHEDULE_STAGING_HPP
#define RUST_LINKGRAPH_SCHEDULE_STAGING_HPP
#include "linkgraph_schedule_ffi.h"

/* Save/load runs serialized with the game loop. Workers never touch these queues;
 * export and both copies below invoke no callback, yield or pool reset. Discard
 * exported spans before SlObject, which may recurse through SlError/SlNullPointers.
 * Nested fixup uses active itself and publishes nulls before returning; the outer
 * destructor therefore publishes the same cleaned vectors, never stale references.
 * Only native frames cross SlObject; partial decoded state commits even on unwind. */
struct ScheduleStaging {
	inline static ScheduleStaging *active = nullptr;
	std::vector<LinkGraph *> schedule;
	std::vector<LinkGraphJob *> running;
	bool import;
	explicit ScheduleStaging(bool import) : import(import)
	{
		OpenTTDScheduleQueues q;
		openttd_rust_schedule_export(&q);
		if (q.schedule_len != 0) this->schedule.assign(q.schedule, q.schedule + q.schedule_len);
		if (q.running_len != 0) this->running.assign(q.running, q.running + q.running_len);
		active = this;
	}
	~ScheduleStaging()
	{
		if (this->import) this->Import();
		active = nullptr;
	}
	void Import() const
	{
		OpenTTDScheduleQueues q{this->schedule.data(), this->schedule.size(), this->running.data(), this->running.size()};
		openttd_rust_schedule_import(&q);
	}
};
#endif /* RUST_LINKGRAPH_SCHEDULE_STAGING_HPP */
