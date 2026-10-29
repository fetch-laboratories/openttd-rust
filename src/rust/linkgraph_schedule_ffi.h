/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file linkgraph_schedule_ffi.h Canonical scheduler queues; native jobs remain opaque. */
#ifndef RUST_LINKGRAPH_SCHEDULE_FFI_H
#define RUST_LINKGRAPH_SCHEDULE_FFI_H
#include "linkgraph_ffi.h"
enum class PauseMode : uint8_t;
struct OpenTTDLiveGraph;
class LinkGraphJob;
struct OpenTTDScheduleTime { int32_t date; uint16_t fract, interval; };
struct OpenTTDScheduleQueues {
	LinkGraph *const *schedule; size_t schedule_len;
	LinkGraphJob *const *running; size_t running_len;
};

/* Game-state mutex serializes every entry; workers never access this owner.
 * Queue pointers preserve native identity (including encoded load references).
 * No queue borrow spans a callback. Leaves are noexcept; panics abort.
 * Exported spans live only until the next owner mutation; copy in native code
 * before SlObject. Native staging publishes partial state on return and unwind, including recursive null cleanup. */
struct OpenTTDScheduleServices {
	OpenTTDLiveGraph *(OPENTTD_RUST_CALL *graph_owner)(LinkGraph *) noexcept;
	void (OPENTTD_RUST_CALL *assert_graph)(const LinkGraph *) noexcept;
	bool (OPENTTD_RUST_CALL *can_allocate)() noexcept;
	void (OPENTTD_RUST_CALL *allocation_failure)() noexcept;
	LinkGraphJob *(OPENTTD_RUST_CALL *new_job)(const LinkGraph *) noexcept;
	void (OPENTTD_RUST_CALL *spawn)(LinkGraphJob *) noexcept;
	bool (OPENTTD_RUST_CALL *due)(const LinkGraphJob *) noexcept;
	bool (OPENTTD_RUST_CALL *completed)(const LinkGraphJob *) noexcept;
	uint16_t (OPENTTD_RUST_CALL *graph_index)(const LinkGraphJob *) noexcept;
	void (OPENTTD_RUST_CALL *destroy_job)(LinkGraphJob *) noexcept;
	bool (OPENTTD_RUST_CALL *graph_valid)(uint16_t) noexcept;
	LinkGraph *(OPENTTD_RUST_CALL *graph_get)(uint16_t) noexcept;
	void (OPENTTD_RUST_CALL *abort_job)(LinkGraphJob *) noexcept;
	LinkGraph *(OPENTTD_RUST_CALL *next_graph)(uint32_t *) noexcept;
	LinkGraphJob *(OPENTTD_RUST_CALL *next_job)(uint32_t *) noexcept;
	void (OPENTTD_RUST_CALL *shift_job)(LinkGraphJob *, int32_t) noexcept;
	void (OPENTTD_RUST_CALL *post_pause)(PauseMode, bool) noexcept;
	void (OPENTTD_RUST_CALL *begin_measurement)(void *, bool) noexcept;
};
extern "C" {
void openttd_rust_schedule_queue(const OpenTTDScheduleServices *, LinkGraph *);
void openttd_rust_schedule_dequeue(LinkGraph *);
void openttd_rust_schedule_spawn_all(const OpenTTDScheduleServices *);
void openttd_rust_schedule_clear(const OpenTTDScheduleServices *);
void openttd_rust_schedule_shift(const OpenTTDScheduleServices *, int32_t);
void openttd_rust_schedule_after_load(const OpenTTDScheduleServices *);
void openttd_rust_schedule_export(OpenTTDScheduleQueues *);
void openttd_rust_schedule_import(const OpenTTDScheduleQueues *);
}
#endif /* RUST_LINKGRAPH_SCHEDULE_FFI_H */
