/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole `LinkGraphSchedule` owner. Workers only touch native job snapshots.
#![allow(unsafe_code)]
use crate::game_state::GameState;
use crate::{
    linkgraph::{
        job::JobRecord as Job,
        live::{self, Graph},
    },
    timer_game_common::SECONDS_PER_DAY,
    witness,
};
use std::{collections::VecDeque, ffi::c_void, sync::atomic::Ordering};

#[repr(C)]
struct Queues {
    schedule: *const *mut Graph,
    schedule_len: usize,
    running: *const *mut Job,
    running_len: usize,
}
#[repr(C)]
pub(crate) struct Services {
    allocation_failure: extern "C" fn(),
    assert_graph: extern "C" fn(*const Graph),
    begin_measurement: extern "C" fn(*mut c_void, bool),
    can_allocate: extern "C" fn() -> bool,
    destroy_job: extern "C" fn(*mut Job),
    pub(crate) graph_get: extern "C" fn(u16) -> *mut Graph,
    graph_valid: extern "C" fn(u16) -> bool,
    new_job: extern "C" fn(*const Graph) -> *mut Job,
    next_graph: extern "C" fn(*mut u32) -> *mut Graph,
    next_job: extern "C" fn(*mut u32) -> *mut Job,
    shared: &'static crate::services::Services,
    start_thread: extern "C" fn(*mut Job) -> bool,
    vehicle: *const crate::vehicle::VehicleServices,
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScheduleServices" ["linkgraph_schedule"] => Services { allocation_failure, assert_graph, begin_measurement, can_allocate, destroy_job, graph_get, graph_valid, new_job, next_graph, next_job, shared, start_thread, vehicle };
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
fn spawn_job(s: &Services, job: *mut Job) {
    if (s.start_thread)(job) {
        return;
    }
    if unsafe { (*job).aborted.load(Ordering::Acquire) } {
        return;
    }
    // The source fallback observes cargo classes and map dimensions at worker entry.
    let cargo = unsafe { (*(*job).graph).cargo };
    let resolver = unsafe { ((*s.vehicle).newgrf)() };
    let classes = unsafe { ((*resolver).cargo_classes)(cargo) };
    unsafe {
        crate::linkgraph::job::openttd_rust_linkgraph_job_run(
            job,
            classes,
            (s.shared.map_max_x)(),
            (s.shared.map_max_y)(),
        );
    }
}
fn spawn_next(s: &Services) {
    let Some(mut next) = access(|q| q.schedule.front().copied()) else {
        return;
    };
    let first = next;
    while unsafe { (&*next).size() } < 2 {
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
        spawn_job(s, job); // may synchronously compute; no owner borrow survives.
        access(|q| q.running.push_back(job));
        witness::hit(witness::SCHEDULE_SPAWN);
    } else {
        (s.allocation_failure)();
    }
}
fn unfinished() -> bool {
    access(|q| q.running.front().copied()).is_some_and(|next| unsafe {
        (*next).join_date <= crate::timer_game_common::economy_date().0
            && !(*next).completed.load(Ordering::Acquire)
    })
}
fn join_next(s: &Services) {
    let Some(next) = access(|q| q.running.front().copied()) else {
        return;
    };
    if unsafe { (*next).join_date } > crate::timer_game_common::economy_date().0 {
        return;
    }
    access(|q| q.running.pop_front());
    let id = unsafe { (*next).graph_index.0 };
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
        spawn_job(s, job);
        witness::hit(witness::SCHEDULE_RELOAD_JOB);
        i += 1;
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_clear() {
    let mut i = 0;
    while let Some(job) = access(|q| q.running.get(i).copied()) {
        unsafe {
            (*job).aborted.store(true, Ordering::Release);
        }
        i += 1;
    }
    access(|q| {
        q.running.clear();
        q.schedule.clear();
    });
}
#[unsafe(no_mangle)]
pub(crate) extern "C" fn openttd_rust_schedule_shift(s: &Services, interval: i32) {
    let mut from = 0;
    loop {
        let graph = (s.next_graph)(&raw mut from);
        if graph.is_null() {
            break;
        }
        // SAFETY: pool iteration yields a live shell; no borrow crosses a callback.
        unsafe { live::openttd_rust_live_graph_shift(graph, interval) };
    }
    from = 0;
    loop {
        let job = (s.next_job)(&raw mut from);
        if job.is_null() {
            break;
        }
        unsafe {
            (*job).join_date = (*job).join_date.wrapping_add(interval);
        }
    }
}
pub(crate) fn tick(
    s: &Services,
    shared: &crate::services::Services,
    client: bool,
    measurement: *mut c_void,
) {
    if crate::timer_game_common::economy_fraction() != SPAWN_JOIN_TICK {
        return;
    }
    let interval = i32::from(unsafe { shared.settings().recalc_interval.read() }) / SECONDS_PER_DAY;
    let offset = crate::timer_game_common::economy_date().0 % interval;
    if offset == 0 {
        spawn_next(s);
    } else if offset == interval / 2 {
        (s.begin_measurement)(measurement, client);
        join_next(s);
    }
}
/// Observer installed by `linkgraph-schedule-reference.cpp`; null posts as before.
#[cfg(feature = "native-test-support")]
pub(crate) static PAUSE_OBSERVER: GameState<Option<extern "C" fn(bool)>> = GameState::new(None);
/// `Command<CMD_PAUSE>::Post(PauseMode::LinkGraph, paused)`, the scheduler's only pause post.
fn post_pause(s: &Services, paused: bool) {
    // The game-free scheduler fixture records this post in place of the command chain it lacks.
    #[cfg(feature = "native-test-support")]
    if let Some(observer) = unsafe { *PAUSE_OBSERVER.get() } {
        observer(paused);
        return;
    }
    crate::command_post::pause(s.shared, crate::openttd::PauseMode::LINK_GRAPH, paused);
}
pub(crate) fn pause(
    s: &Services,
    shared: &crate::services::Services,
    paused: bool,
    unpaused: bool,
) {
    if paused {
        if !unfinished() {
            post_pause(s, false);
        }
    } else if unpaused {
        if crate::timer_game_common::economy_fraction() != SPAWN_JOIN_TICK - 2 {
            return;
        }
        let interval =
            i32::from(unsafe { shared.settings().recalc_interval.read() }) / SECONDS_PER_DAY;
        if crate::timer_game_common::economy_date().0 % interval == interval / 2 && unfinished() {
            post_pause(s, true);
            witness::hit(witness::SCHEDULE_PAUSE);
        }
    }
}
#[unsafe(no_mangle)]
extern "C" fn openttd_rust_schedule_after_load() {
    if unfinished() {
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
