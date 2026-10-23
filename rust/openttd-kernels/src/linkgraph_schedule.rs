/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole `LinkGraphSchedule` owner. Workers only touch native job snapshots.
#![allow(unsafe_code)]
use crate::{timer_game_common::SECONDS_PER_DAY, witness};
use std::{cell::UnsafeCell, collections::VecDeque, ffi::c_void};

#[repr(C)]
struct Graph {
    _opaque: [u8; 0],
}
#[repr(C)]
struct Job {
    _opaque: [u8; 0],
}
#[repr(C)]
struct Time {
    date: i32,
    fract: u16,
    interval: u16,
}
#[repr(C)]
struct Queues {
    schedule: *const *mut Graph,
    schedule_len: usize,
    running: *const *mut Job,
    running_len: usize,
}
unsafe extern "C" {
    safe fn RustLinkGraphSize(arg0: *const Graph) -> u16;
    safe fn OpenTTDScheduleAssertGraph(arg0: *const Graph);
    safe fn OpenTTDScheduleCanAllocate() -> bool;
    safe fn OpenTTDScheduleAllocationFailure();
    safe fn OpenTTDScheduleNewJob(arg0: *const Graph) -> *mut Job;
    safe fn OpenTTDScheduleSpawn(arg0: *mut Job);
    safe fn OpenTTDScheduleDue(arg0: *const Job) -> bool;
    safe fn OpenTTDScheduleCompleted(arg0: *const Job) -> bool;
    safe fn OpenTTDScheduleGraphIndex(arg0: *const Job) -> u16;
    safe fn OpenTTDScheduleDestroyJob(arg0: *mut Job);
    safe fn OpenTTDScheduleGraphValid(arg0: u16) -> bool;
    safe fn OpenTTDScheduleGraphGet(arg0: u16) -> *mut Graph;
    safe fn OpenTTDScheduleAbortJob(arg0: *mut Job);
    safe fn OpenTTDScheduleNextGraph(arg0: *mut u32) -> *mut Graph;
    safe fn OpenTTDScheduleNextJob(arg0: *mut u32) -> *mut Job;
    safe fn OpenTTDScheduleShiftGraph(arg0: *mut Graph, arg1: i32);
    safe fn OpenTTDScheduleShiftJob(arg0: *mut Job, arg1: i32);
    safe fn OpenTTDScheduleTimeNow() -> Time;
    safe fn OpenTTDSchedulePostPause(arg0: bool);
    safe fn OpenTTDScheduleSetPause();
    safe fn OpenTTDScheduleBeginMeasurement(arg0: *mut c_void, arg1: bool);
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScheduleTime" ["probe"] => Time { date, fract, interval };
    "OpenTTDScheduleQueues" ["probe"] => Queues { schedule, schedule_len, running, running_len };

}
struct State {
    schedule: VecDeque<*mut Graph>,
    running: VecDeque<*mut Job>,
}
struct Global(UnsafeCell<State>);
// SAFETY: native game-state mutex serializes main/game-thread entries. Workers
// cannot access queues. Closures never call native code or expose owner borrows.
unsafe impl Sync for Global {}
static STATE: Global = Global(UnsafeCell::new(State {
    schedule: VecDeque::new(),
    running: VecDeque::new(),
}));
fn access<T>(f: impl FnOnce(&mut State) -> T) -> T {
    // SAFETY: entry contract above; no callback/reentry can occur inside f.
    unsafe { f(&mut *STATE.0.get()) }
}
// `linkgraphschedule.h`; asserted in native facade.
const SPAWN_JOIN_TICK: u16 = 21;

#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_queue(graph: *mut Graph) {
    OpenTTDScheduleAssertGraph(graph);
    access(|q| q.schedule.push_back(graph));
    witness::hit(witness::SCHEDULE_QUEUE);
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_dequeue(graph: *mut Graph) {
    access(|q| q.schedule.retain(|&g| g != graph));
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_spawn_next() {
    let Some(mut next) = access(|q| q.schedule.front().copied()) else {
        return;
    };
    let first = next;
    while RustLinkGraphSize(next) < 2 {
        next = access(|q| {
            q.schedule.rotate_left(1);
            q.schedule[0]
        });
        witness::hit(witness::SCHEDULE_ROTATE);
        if next == first {
            return;
        }
    }
    OpenTTDScheduleAssertGraph(next);
    access(|q| q.schedule.pop_front());
    if OpenTTDScheduleCanAllocate() {
        let job = OpenTTDScheduleNewJob(next);
        OpenTTDScheduleSpawn(job); // may synchronously compute; no owner borrow survives.
        access(|q| q.running.push_back(job));
        witness::hit(witness::SCHEDULE_SPAWN);
    } else {
        OpenTTDScheduleAllocationFailure();
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_unfinished() -> bool {
    access(|q| q.running.front().copied())
        .is_some_and(|next| OpenTTDScheduleDue(next) && !OpenTTDScheduleCompleted(next))
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_join_next() {
    let Some(next) = access(|q| q.running.front().copied()) else {
        return;
    };
    if !OpenTTDScheduleDue(next) {
        return;
    }
    access(|q| q.running.pop_front());
    let id = OpenTTDScheduleGraphIndex(next);
    OpenTTDScheduleDestroyJob(next); // joins and applies results; may reenter this owner.
    if OpenTTDScheduleGraphValid(id) {
        let graph = OpenTTDScheduleGraphGet(id);
        openttd_rust_schedule_dequeue(graph);
        openttd_rust_schedule_queue(graph);
        witness::hit(witness::SCHEDULE_REQUEUE);
    }
    witness::hit(witness::SCHEDULE_JOIN);
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_spawn_all() {
    let mut i = 0;
    while let Some(job) = access(|q| q.running.get(i).copied()) {
        OpenTTDScheduleSpawn(job);
        witness::hit(witness::SCHEDULE_RELOAD_JOB);
        i += 1;
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_clear() {
    let mut i = 0;
    while let Some(job) = access(|q| q.running.get(i).copied()) {
        OpenTTDScheduleAbortJob(job);
        i += 1;
    }
    access(|q| {
        q.running.clear();
        q.schedule.clear();
    });
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_shift(interval: i32) {
    let mut from = 0;
    loop {
        let graph = OpenTTDScheduleNextGraph(&raw mut from);
        if graph.is_null() {
            break;
        }
        OpenTTDScheduleShiftGraph(graph, interval);
    }
    from = 0;
    loop {
        let job = OpenTTDScheduleNextJob(&raw mut from);
        if job.is_null() {
            break;
        }
        OpenTTDScheduleShiftJob(job, interval);
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_tick(client: bool, measurement: *mut c_void) {
    let time = OpenTTDScheduleTimeNow();
    if time.fract != SPAWN_JOIN_TICK {
        return;
    }
    let interval = i32::from(time.interval) / SECONDS_PER_DAY;
    let offset = time.date % interval;
    if offset == 0 {
        openttd_rust_schedule_spawn_next();
    } else if offset == interval / 2 {
        OpenTTDScheduleBeginMeasurement(measurement, client);
        openttd_rust_schedule_join_next();
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_pause(paused: bool, unpaused: bool) {
    if paused {
        if !openttd_rust_schedule_unfinished() {
            OpenTTDSchedulePostPause(false);
        }
    } else if unpaused {
        let time = OpenTTDScheduleTimeNow();
        if time.fract != SPAWN_JOIN_TICK - 2 {
            return;
        }
        let interval = i32::from(time.interval) / SECONDS_PER_DAY;
        if time.date % interval == interval / 2 && openttd_rust_schedule_unfinished() {
            OpenTTDSchedulePostPause(true);
            witness::hit(witness::SCHEDULE_PAUSE);
        }
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_after_load() {
    if openttd_rust_schedule_unfinished() {
        OpenTTDScheduleSetPause();
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn openttd_rust_schedule_export(out: *mut Queues) {
    let queues = access(|q| Queues {
        schedule: q.schedule.make_contiguous().as_ptr(),
        schedule_len: q.schedule.len(),
        running: q.running.make_contiguous().as_ptr(),
        running_len: q.running.len(),
    });
    // SAFETY: native caller provides writable output, copies before mutation.
    unsafe {
        out.write(queues);
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn openttd_rust_schedule_import(input: &Queues) {
    // SAFETY: scoped native staging provides live arrays (empty spans are skipped).
    access(|q| unsafe {
        q.schedule.clear();
        q.running.clear();
        if input.schedule_len != 0 {
            q.schedule.extend(std::slice::from_raw_parts(
                input.schedule,
                input.schedule_len,
            ));
        }
        if input.running_len != 0 {
            q.running
                .extend(std::slice::from_raw_parts(input.running, input.running_len));
        }
    });
    witness::hit(witness::SCHEDULE_IMPORT);
}
