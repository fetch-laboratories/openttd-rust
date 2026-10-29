/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole `LinkGraphSchedule` owner. Workers only touch native job snapshots.
#![allow(unsafe_code)]
use crate::game_state::GameState;
use crate::{
    linkgraph::{NativeGraph as Graph, live},
    timer_game_common::SECONDS_PER_DAY,
    witness,
};
use std::{collections::VecDeque, ffi::c_void};

#[repr(C)]
struct Job {
    _opaque: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Time {
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
#[repr(C)]
pub(crate) struct Services {
    graph_owner: extern "C" fn(*mut Graph) -> *mut live::Graph,
    assert_graph: extern "C" fn(*const Graph),
    can_allocate: extern "C" fn() -> bool,
    allocation_failure: extern "C" fn(),
    new_job: extern "C" fn(*const Graph) -> *mut Job,
    spawn: extern "C" fn(*mut Job),
    due: extern "C" fn(*const Job) -> bool,
    completed: extern "C" fn(*const Job) -> bool,
    graph_index: extern "C" fn(*const Job) -> u16,
    destroy_job: extern "C" fn(*mut Job),
    graph_valid: extern "C" fn(u16) -> bool,
    pub(crate) graph_get: extern "C" fn(u16) -> *mut Graph,
    abort_job: extern "C" fn(*mut Job),
    next_graph: extern "C" fn(*mut u32) -> *mut Graph,
    next_job: extern "C" fn(*mut u32) -> *mut Job,
    shift_job: extern "C" fn(*mut Job, i32),
    post_pause: extern "C" fn(crate::openttd::PauseMode, bool),
    begin_measurement: extern "C" fn(*mut c_void, bool),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScheduleServices" ["linkgraph_schedule"] => Services { graph_owner, assert_graph, can_allocate, allocation_failure, new_job, spawn, due, completed, graph_index, destroy_job, graph_valid, graph_get, abort_job, next_graph, next_job, shift_job, post_pause, begin_measurement };
    "OpenTTDScheduleTime" ["linkgraph_schedule"] => Time { date, fract, interval };
    "OpenTTDScheduleQueues" ["linkgraph_schedule"] => Queues { schedule, schedule_len, running, running_len };

}
struct State {
    schedule: VecDeque<*mut Graph>,
    running: VecDeque<*mut Job>,
}
// Link graph job workers never access these queues.
static STATE: GameState<State> = GameState::new(State {
    schedule: VecDeque::new(),
    running: VecDeque::new(),
});
fn access<T>(f: impl FnOnce(&mut State) -> T) -> T {
    // SAFETY: entry contract above; no callback/reentry can occur inside f.
    unsafe { f(&mut *STATE.get()) }
}
// `linkgraphschedule.h`; asserted in native facade.
const SPAWN_JOIN_TICK: u16 = 21;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn openttd_rust_schedule_queue(s: &Services, graph: *mut Graph) {
    (s.assert_graph)(graph);
    access(|q| q.schedule.push_back(graph));
    witness::hit(witness::SCHEDULE_QUEUE);
}
#[unsafe(no_mangle)]
pub(crate) extern "C" fn openttd_rust_schedule_dequeue(graph: *mut Graph) {
    access(|q| q.schedule.retain(|&g| g != graph));
}
fn spawn_next(s: &Services) {
    let Some(mut next) = access(|q| q.schedule.front().copied()) else {
        return;
    };
    let first = next;
    while unsafe { (&*(s.graph_owner)(next)).size() } < 2 {
        next = access(|q| {
            q.schedule.rotate_left(1);
            q.schedule[0]
        });
        witness::hit(witness::SCHEDULE_ROTATE);
        if next == first {
            return;
        }
    }
    (s.assert_graph)(next);
    access(|q| q.schedule.pop_front());
    if (s.can_allocate)() {
        let job = (s.new_job)(next);
        (s.spawn)(job); // may synchronously compute; no owner borrow survives.
        access(|q| q.running.push_back(job));
        witness::hit(witness::SCHEDULE_SPAWN);
    } else {
        (s.allocation_failure)();
    }
}
fn unfinished(s: &Services) -> bool {
    access(|q| q.running.front().copied()).is_some_and(|next| (s.due)(next) && !(s.completed)(next))
}
fn join_next(s: &Services) {
    let Some(next) = access(|q| q.running.front().copied()) else {
        return;
    };
    if !(s.due)(next) {
        return;
    }
    access(|q| q.running.pop_front());
    let id = (s.graph_index)(next);
    (s.destroy_job)(next); // joins and applies results; may reenter this owner.
    if (s.graph_valid)(id) {
        let graph = (s.graph_get)(id);
        openttd_rust_schedule_dequeue(graph);
        openttd_rust_schedule_queue(s, graph);
        witness::hit(witness::SCHEDULE_REQUEUE);
    }
    witness::hit(witness::SCHEDULE_JOIN);
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_spawn_all(s: &Services) {
    let mut i = 0;
    while let Some(job) = access(|q| q.running.get(i).copied()) {
        (s.spawn)(job);
        witness::hit(witness::SCHEDULE_RELOAD_JOB);
        i += 1;
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_clear(s: &Services) {
    let mut i = 0;
    while let Some(job) = access(|q| q.running.get(i).copied()) {
        (s.abort_job)(job);
        i += 1;
    }
    access(|q| {
        q.running.clear();
        q.schedule.clear();
    });
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_shift(s: &Services, interval: i32) {
    let mut from = 0;
    loop {
        let graph = (s.next_graph)(&raw mut from);
        if graph.is_null() {
            break;
        }
        // SAFETY: pool iteration yields a live shell; no borrow crosses a callback.
        unsafe { live::openttd_rust_live_graph_shift((s.graph_owner)(graph), interval) };
    }
    from = 0;
    loop {
        let job = (s.next_job)(&raw mut from);
        if job.is_null() {
            break;
        }
        (s.shift_job)(job, interval);
    }
}
pub(crate) fn tick(s: &Services, time: Time, client: bool, measurement: *mut c_void) {
    if time.fract != SPAWN_JOIN_TICK {
        return;
    }
    let interval = i32::from(time.interval) / SECONDS_PER_DAY;
    let offset = time.date % interval;
    if offset == 0 {
        spawn_next(s);
    } else if offset == interval / 2 {
        (s.begin_measurement)(measurement, client);
        join_next(s);
    }
}
pub(crate) fn pause(s: &Services, time: Time, paused: bool, unpaused: bool) {
    if paused {
        if !unfinished(s) {
            (s.post_pause)(crate::openttd::PauseMode::LINK_GRAPH, false);
        }
    } else if unpaused {
        if time.fract != SPAWN_JOIN_TICK - 2 {
            return;
        }
        let interval = i32::from(time.interval) / SECONDS_PER_DAY;
        if time.date % interval == interval / 2 && unfinished(s) {
            (s.post_pause)(crate::openttd::PauseMode::LINK_GRAPH, true);
            witness::hit(witness::SCHEDULE_PAUSE);
        }
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_after_load(s: &Services) {
    if unfinished(s) {
        crate::session::set_pause(
            crate::session::pause() | crate::openttd::PauseMode::LINK_GRAPH.bit(),
        );
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
