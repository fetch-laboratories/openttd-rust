/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file linkgraph_schedule_ffi.h Canonical graph/job record queues and native scheduler operations. */
#ifndef RUST_LINKGRAPH_SCHEDULE_FFI_H
#define RUST_LINKGRAPH_SCHEDULE_FFI_H
#include "linkgraph_ffi.h"
enum class PauseMode : uint8_t;
struct OpenTTDLiveGraph;
class LinkGraphJob;
struct OpenTTDScheduleQueues {
	OpenTTDLiveGraph *const *schedule; size_t schedule_len;
	OpenTTDLinkGraphJobRecord *const *running; size_t running_len;
};

/* Game-state mutex serializes every entry; workers never access this owner.
 * Queue record pointers retain the native index + 1 encoding during load fixup.
 * No queue borrow spans a callback. Leaves are noexcept; panics abort.
 * Exported spans live only until the next owner mutation; copy in native code
 * before SlObject. Native staging publishes partial state on return and unwind, including recursive null cleanup. */
struct OpenTTDSharedServices;
struct OpenTTDScheduleServices {
	const OpenTTDSharedServices *shared;
	void (OPENTTD_RUST_CALL *assert_graph)(const OpenTTDLiveGraph *) noexcept;
	bool (OPENTTD_RUST_CALL *can_allocate)() noexcept;
	void (OPENTTD_RUST_CALL *allocation_failure)() noexcept;
	OpenTTDLinkGraphJobRecord *(OPENTTD_RUST_CALL *new_job)(const OpenTTDLiveGraph *) noexcept;
	void (OPENTTD_RUST_CALL *spawn)(OpenTTDLinkGraphJobRecord *) noexcept;
	void (OPENTTD_RUST_CALL *destroy_job)(OpenTTDLinkGraphJobRecord *) noexcept;
	bool (OPENTTD_RUST_CALL *graph_valid)(uint16_t) noexcept;
	OpenTTDLiveGraph *(OPENTTD_RUST_CALL *graph_get)(uint16_t) noexcept;
	OpenTTDLiveGraph *(OPENTTD_RUST_CALL *next_graph)(uint32_t *) noexcept;
	OpenTTDLinkGraphJobRecord *(OPENTTD_RUST_CALL *next_job)(uint32_t *) noexcept;
	void (OPENTTD_RUST_CALL *begin_measurement)(void *, bool) noexcept;
};
extern "C" {
void openttd_rust_schedule_queue(const OpenTTDScheduleServices *, OpenTTDLiveGraph *);
void openttd_rust_schedule_dequeue(OpenTTDLiveGraph *);
void openttd_rust_schedule_spawn_all(const OpenTTDScheduleServices *);
void openttd_rust_schedule_clear();
void openttd_rust_schedule_shift(const OpenTTDScheduleServices *, int32_t);
void openttd_rust_schedule_after_load();
void openttd_rust_schedule_export(OpenTTDScheduleQueues *);
void openttd_rust_schedule_import(const OpenTTDScheduleQueues *);
}
#endif /* RUST_LINKGRAPH_SCHEDULE_FFI_H */
