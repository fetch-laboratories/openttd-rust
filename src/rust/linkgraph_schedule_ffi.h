/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file linkgraph_schedule_ffi.h Canonical scheduler queues; native jobs remain opaque. */
#ifndef RUST_LINKGRAPH_SCHEDULE_FFI_H
#define RUST_LINKGRAPH_SCHEDULE_FFI_H
#include "linkgraph_ffi.h"
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
 * before SlObject. Import only after native save/load/fixup returns normally. */
extern "C" {
uint16_t RustLinkGraphSize(const LinkGraph *) noexcept;
void OpenTTDScheduleAssertGraph(const LinkGraph *) noexcept;
bool OpenTTDScheduleCanAllocate() noexcept;
void OpenTTDScheduleAllocationFailure() noexcept;
LinkGraphJob *OpenTTDScheduleNewJob(const LinkGraph *) noexcept;
void OpenTTDScheduleSpawn(LinkGraphJob *) noexcept;
bool OpenTTDScheduleDue(const LinkGraphJob *) noexcept;
bool OpenTTDScheduleCompleted(const LinkGraphJob *) noexcept;
uint16_t OpenTTDScheduleGraphIndex(const LinkGraphJob *) noexcept;
void OpenTTDScheduleDestroyJob(LinkGraphJob *) noexcept;
bool OpenTTDScheduleGraphValid(uint16_t) noexcept;
LinkGraph *OpenTTDScheduleGraphGet(uint16_t) noexcept;
void OpenTTDScheduleAbortJob(LinkGraphJob *) noexcept;
LinkGraph *OpenTTDScheduleNextGraph(uint32_t *) noexcept;
LinkGraphJob *OpenTTDScheduleNextJob(uint32_t *) noexcept;
void OpenTTDScheduleShiftGraph(LinkGraph *, int32_t) noexcept;
void OpenTTDScheduleShiftJob(LinkGraphJob *, int32_t) noexcept;
OpenTTDScheduleTime OpenTTDScheduleTimeNow() noexcept;
void OpenTTDSchedulePostPause(bool) noexcept;
void OpenTTDScheduleSetPause() noexcept;
void OpenTTDScheduleBeginMeasurement(void *, bool) noexcept;
void openttd_rust_schedule_queue(LinkGraph *);
void openttd_rust_schedule_dequeue(LinkGraph *);
void openttd_rust_schedule_spawn_next();
bool openttd_rust_schedule_unfinished();
void openttd_rust_schedule_join_next();
void openttd_rust_schedule_spawn_all();
void openttd_rust_schedule_clear();
void openttd_rust_schedule_shift(int32_t);
void openttd_rust_schedule_tick(bool, void *);
void openttd_rust_schedule_pause(bool, bool);
void openttd_rust_schedule_after_load();
void openttd_rust_schedule_export(OpenTTDScheduleQueues *);
void openttd_rust_schedule_import(const OpenTTDScheduleQueues *);
}
#endif /* RUST_LINKGRAPH_SCHEDULE_FFI_H */
