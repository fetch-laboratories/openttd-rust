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

/* Game-state mutex serializes queue entries; workers never access the queues.
 * The worker entry uses only its private job and immutable services.
 * Queue record pointers retain the native index + 1 encoding during load fixup.
 * No queue borrow spans a callback. Leaves are noexcept; panics abort.
 * Exported spans live only until the next owner mutation; copy in native code
 * before SlObject. Native staging publishes partial state on return and unwind, including recursive null cleanup. */
struct OpenTTDSharedServices;
struct OpenTTDVehicleServices;
struct OpenTTDScheduleServices {
	void (OPENTTD_RUST_CALL *allocation_failure)() noexcept;
	void (OPENTTD_RUST_CALL *assert_graph)(const OpenTTDLiveGraph *) noexcept;
	void (OPENTTD_RUST_CALL *begin_measurement)(bool) noexcept;
	bool (OPENTTD_RUST_CALL *can_allocate)() noexcept;
	void (OPENTTD_RUST_CALL *destroy_job)(OpenTTDLinkGraphJobRecord *) noexcept;
	void (OPENTTD_RUST_CALL *end_measurement)() noexcept;
	/* Constant-initialized addresses; access only in live game/fixture entries. */
	const RustPoolDescriptor *const *graph_pool;
	const RustPoolDescriptor *const *job_pool;
	OpenTTDLinkGraphJobRecord *(OPENTTD_RUST_CALL *new_job)(const OpenTTDLiveGraph *) noexcept;
	const OpenTTDSharedServices *shared;
	bool (OPENTTD_RUST_CALL *start_thread)(OpenTTDLinkGraphJobRecord *) noexcept;
	const OpenTTDVehicleServices *vehicle;
};
extern "C" {
void openttd_rust_schedule_run(const OpenTTDScheduleServices *, OpenTTDLinkGraphJobRecord *);
void openttd_rust_schedule_queue(const OpenTTDScheduleServices *, OpenTTDLiveGraph *);
void openttd_rust_schedule_spawn_all(const OpenTTDScheduleServices *);
void openttd_rust_schedule_clear();
void openttd_rust_schedule_shift(const OpenTTDScheduleServices *, int32_t);
void openttd_rust_schedule_after_load();
void openttd_rust_schedule_export(OpenTTDScheduleQueues *);
void openttd_rust_schedule_import(const OpenTTDScheduleQueues *);
}
#endif /* RUST_LINKGRAPH_SCHEDULE_FFI_H */
