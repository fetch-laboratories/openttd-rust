/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Scheduler branches the deterministic save-play games cannot reach (#736 T).
//!
//! Moved from `tools/simulation/linkgraph-schedule-reference.cpp`, which ran the pinned
//! original's `LinkGraphSchedule` bodies and these production entries in one game-free fixture
//! and compared their traces. The original's trace is frozen in `schedule-oracle.txt`
//! (`python3 tools/oracle-capture.py linkgraph_schedule`); this test rebuilds the fixture's
//! graph and job shells, their pools and the schedule services around the production entries,
//! and records the same trace entries, whose kinds the oracle header lists.
//!
//! As in the fixture, a worker is synchronous: `start_thread` marks its job completed. The
//! pause post is recorded in place of the command chain the fixture lacks, and the tick and
//! pause entries take the client and pause arguments the session frame derives; the session
//! frame and the command chain are outside this test.
use super::*;
use crate::industrytype::NativeVec;
use crate::linkgraph::{
    NativeGraph,
    job::{NativeJob, SnapshotSettings},
};
use crate::linkgraph_type::LinkGraphID;
use crate::pool::{PoolDescriptor, PoolSlot};
use crate::timer_game_common::{write_economy_date, write_economy_fraction};
use crate::timer_game_economy::EconomyDate;
use std::ptr::null_mut;

/// The fixture's `LinkGraph` shell: its pool index and the tag its trace entries name.
struct GraphShell {
    index: u16,
    tag: u16,
    owner: *mut Graph,
}
/// The fixture's `LinkGraphJob` shell.
struct JobShell {
    index: u16,
    graph: u16,
    record: *mut Job,
}

/// Slots per fixture pool.
const SLOTS: usize = 4;

/// The fixture's globals.
struct Fixture {
    trace: Vec<[i32; 3]>,
    graphs: *mut PoolDescriptor,
    jobs: *mut PoolDescriptor,
    /// Whether a job's destruction recycles graph ID 1, as the fixture's `Recycle` does.
    recycle: bool,
    /// The fixture's static `replacement` graph, emplaced by each recycle.
    replacement: *mut GraphShell,
    services: *const Services,
}
/// Process-wide, as the probe's globals were; the test holds `GLOBAL_LOCK`.
static FIXTURE: GameState<Fixture> = GameState::new(Fixture {
    trace: Vec::new(),
    graphs: null_mut(),
    jobs: null_mut(),
    recycle: false,
    replacement: null_mut(),
    services: std::ptr::null(),
});
/// No production entry is called while the fixture is borrowed.
fn with<R>(f: impl FnOnce(&mut Fixture) -> R) -> R {
    // SAFETY: Serialized by the test lock; the borrow ends with `f`.
    f(unsafe { &mut *FIXTURE.get() })
}
fn graphs() -> *mut PoolDescriptor {
    with(|f| f.graphs)
}
fn jobs() -> *mut PoolDescriptor {
    with(|f| f.jobs)
}

fn observe(entry: [i32; 3]) {
    with(|f| f.trace.push(entry));
}
fn new_pool() -> *mut PoolDescriptor {
    let empty = PoolSlot {
        shell: null_mut(),
        record: null_mut(),
    };
    Box::into_raw(Box::new(PoolDescriptor {
        slots: NativeVec::from_vec(vec![empty; SLOTS]),
        used_bitmap: NativeVec::new(),
        first_free: 0,
        first_unused: SLOTS,
        items: 0,
        checked: 0,
        cleaning: false,
        alloc_cache: null_mut(),
    }))
}
// SAFETY (the slot helpers): `pool` is a live fixture pool and `index` below `SLOTS`; each
// copies one slot in or out, so no borrow outlives the call.
fn slot(pool: *mut PoolDescriptor, index: u16) -> PoolSlot {
    unsafe { *(*pool).slots.as_mut_ptr().add(usize::from(index)) }
}
fn set_slot(pool: *mut PoolDescriptor, index: u16, value: PoolSlot) {
    unsafe { *(*pool).slots.as_mut_ptr().add(usize::from(index)) = value };
}
fn shells<T>(pool: *mut PoolDescriptor) -> Vec<*mut T> {
    (0..SLOTS as u16)
        .map(|index| slot(pool, index).shell.cast::<T>())
        .filter(|shell| !shell.is_null())
        .collect()
}

/// `LinkGraph(id, count, label)`: a canonical record of `count` nodes dated 90, last
/// compressed on 100, published in the graph pool.
fn new_graph(index: u16, count: u16, tag: u16) -> *mut GraphShell {
    let owner = live::openttd_rust_live_graph_new(0);
    let shell = Box::into_raw(Box::new(GraphShell { index, tag, owner }));
    // SAFETY: `owner` is the fresh, uniquely owned record.
    unsafe {
        (*owner).shell = shell.cast::<NativeGraph>();
        (*owner).last_compression = 100;
        live::openttd_rust_live_graph_init(owner, count.into());
        for node in &mut *(*owner).nodes {
            node.last_update = 90;
        }
    }
    set_slot(
        graphs(),
        index,
        PoolSlot {
            shell: shell.cast(),
            record: owner.cast(),
        },
    );
    shell
}
/// `~LinkGraph`: a shell recycled out of its slot leaves the slot to its replacement.
fn delete_graph(shell: *mut GraphShell) {
    // SAFETY: `shell` came from `new_graph` and is deleted once.
    let shell = unsafe { Box::from_raw(shell) };
    if slot(graphs(), shell.index).shell == (&raw const *shell).cast_mut().cast() {
        set_slot(
            graphs(),
            shell.index,
            PoolSlot {
                shell: null_mut(),
                record: null_mut(),
            },
        );
    }
    // SAFETY: The shell's own record, no longer queued.
    unsafe { live::openttd_rust_live_graph_delete(shell.owner) };
}
/// `LinkGraphJob(graph)`: the lowest free job slot, a record never due.
fn new_job(graph: *const GraphShell) -> *mut JobShell {
    // SAFETY: A live fixture graph shell.
    let (graph_index, owner, tag) = unsafe { ((*graph).index, (*graph).owner, (*graph).tag) };
    let mut index = 0;
    while !slot(jobs(), index).shell.is_null() {
        index += 1;
    }
    let settings = SnapshotSettings {
        recalc_time: 0,
        recalc_interval: 0,
        distribution_pax: 0,
        distribution_mail: 0,
        distribution_armoured: 0,
        distribution_default: 0,
        accuracy: 0,
        demand_size: 0,
        demand_distance: 0,
        short_path_saturation: 0,
    };
    // SAFETY: `owner` is the graph's live record.
    let record = unsafe {
        crate::linkgraph::job::openttd_rust_linkgraph_job_new(
            owner,
            LinkGraphID(graph_index),
            &raw const settings,
        )
    };
    let shell = Box::into_raw(Box::new(JobShell {
        index,
        graph: graph_index,
        record,
    }));
    // SAFETY: The fresh, uniquely owned record.
    unsafe {
        (*record).shell = shell.cast::<NativeJob>();
        (*record).join_date = i32::MAX;
    }
    set_slot(
        jobs(),
        index,
        PoolSlot {
            shell: shell.cast(),
            record: record.cast(),
        },
    );
    observe([2, index.into(), tag.into()]);
    shell
}
/// `~LinkGraphJob`, which may recycle graph ID 1.
fn delete_job(shell: *mut JobShell) {
    // SAFETY: `shell` came from `new_job` and is deleted once.
    let shell = unsafe { Box::from_raw(shell) };
    observe([3, shell.index.into(), shell.graph.into()]);
    set_slot(
        jobs(),
        shell.index,
        PoolSlot {
            shell: null_mut(),
            record: null_mut(),
        },
    );
    // SAFETY: The shell's own record, removed from the running queue by its caller.
    unsafe { crate::linkgraph::job::openttd_rust_linkgraph_job_delete(shell.record) };
    if with(|f| f.recycle) {
        recycle();
    }
}
/// `Recycle`: emplace a two-node replacement in graph slot 1 and queue it twice.
fn recycle() {
    let previous = with(|f| std::mem::replace(&mut f.replacement, null_mut()));
    if !previous.is_null() {
        delete_graph(previous);
    }
    let replacement = new_graph(1, 2, 19);
    with(|f| f.replacement = replacement);
    queue(replacement);
    queue(replacement);
}
fn set_due(job: *mut JobShell, due: bool) {
    // SAFETY: A live fixture job and its record.
    unsafe { (*(*job).record).join_date = if due { 0 } else { i32::MAX } };
}
fn set_completed(job: *mut JobShell, completed: bool) {
    // SAFETY: A live fixture job and its record.
    unsafe {
        (*(*job).record)
            .completed
            .store(completed, Ordering::Release);
    }
}

extern "C" fn allocation_failure() {
    unreachable!("the fixture can always allocate a job");
}
extern "C" fn assert_graph(graph: *const Graph) {
    // SAFETY: Production passes a live record whose shell is a fixture graph.
    let shell = unsafe { (*graph).shell.cast::<GraphShell>() };
    // SAFETY: As above.
    let index = unsafe { (*shell).index };
    assert_eq!(slot(graphs(), index).shell, shell.cast::<c_void>());
}
extern "C" fn begin_measurement(measurement: *mut c_void, client: bool) {
    if client {
        // SAFETY: `tick` passes its local measurement flag.
        unsafe { *measurement.cast::<bool>() = true };
        observe([9, 1, 0]);
    } else {
        observe([9, 0, 0]);
    }
}
extern "C" fn can_allocate() -> bool {
    true
}
extern "C" fn destroy_job(job: *mut Job) {
    // SAFETY: Production passes a live record whose shell is a fixture job.
    delete_job(unsafe { (*job).shell.cast::<JobShell>() });
}
extern "C" fn new_job_record(graph: *const Graph) -> *mut Job {
    // SAFETY: Production passes a live record whose shell is a fixture graph.
    let shell = new_job(unsafe { (*graph).shell.cast::<GraphShell>() });
    // SAFETY: The fresh fixture job.
    unsafe { (*shell).record }
}
extern "C" fn start_thread(job: *mut Job) -> bool {
    // SAFETY: Production passes a live record whose shell is a fixture job.
    let index = unsafe { (*(*job).shell.cast::<JobShell>()).index };
    observe([4, index.into(), 0]);
    // SAFETY: As above.
    unsafe { (*job).completed.store(true, Ordering::Release) };
    true
}
fn observe_pause(paused: bool) {
    observe([7, paused.into(), 0]);
}

fn services() -> &'static Services {
    // SAFETY: Installed by the test for its whole run.
    unsafe { &*with(|f| f.services) }
}
fn import(schedule: &[*mut GraphShell], running: &[*mut JobShell]) {
    // SAFETY: Live fixture shells.
    let graphs: Vec<*mut Graph> = schedule.iter().map(|&g| unsafe { (*g).owner }).collect();
    // SAFETY: As above.
    let jobs: Vec<*mut Job> = running.iter().map(|&j| unsafe { (*j).record }).collect();
    let queues = Queues {
        schedule: graphs.as_ptr(),
        schedule_len: graphs.len(),
        running: jobs.as_ptr(),
        running_len: jobs.len(),
    };
    // SAFETY: The arrays outlive the call; import copies them.
    unsafe { openttd_rust_schedule_import(&queues) };
}
/// The fixture's `Capture`: both queue lengths, then each queued graph's tag and each
/// running job with its graph.
fn capture() {
    let mut queues = std::mem::MaybeUninit::<Queues>::uninit();
    // SAFETY: Writable output, copied below before any scheduler call.
    let queues = unsafe {
        openttd_rust_schedule_export(queues.as_mut_ptr());
        queues.assume_init()
    };
    // SAFETY: The exported spans are live until the next scheduler call.
    let (schedule, running) = unsafe {
        (
            std::slice::from_raw_parts(queues.schedule, queues.schedule_len).to_vec(),
            std::slice::from_raw_parts(queues.running, queues.running_len).to_vec(),
        )
    };
    let length = |len: usize| i32::try_from(len).expect("fixture queue length");
    observe([20, length(schedule.len()), length(running.len())]);
    for graph in schedule {
        // SAFETY: Queued records of live fixture graphs.
        observe([
            21,
            unsafe { (*(*graph).shell.cast::<GraphShell>()).tag }.into(),
            0,
        ]);
    }
    for job in running {
        // SAFETY: Running records of live fixture jobs.
        let shell = unsafe { &*(*job).shell.cast::<JobShell>() };
        observe([22, shell.index.into(), shell.graph.into()]);
    }
}
fn queue(graph: *mut GraphShell) {
    // SAFETY: A live fixture graph.
    openttd_rust_schedule_queue(services(), unsafe { (*graph).owner });
}
/// The session's link graph tick at `date` and `fract`; `client` is a networked non-server.
fn tick(date: i32, fract: u16, client: bool) {
    write_economy_date(EconomyDate(date));
    write_economy_fraction(fract);
    let mut measured = false;
    super::tick(
        services(),
        services().shared,
        client,
        (&raw mut measured).cast(),
    );
    // The client's `PerformanceMeasurer` ends with the tick.
    if measured {
        observe([9, 2, 0]);
    }
}
/// The session's pause control on day 1, fraction 19, under pause `bits` (1 link graph,
/// 2 normal); `load` runs the after-load control on an unpaused game instead.
fn pause(bits: u8, load: bool) {
    write_economy_date(EconomyDate(1));
    write_economy_fraction(19);
    if load {
        crate::session::set_pause(0);
        openttd_rust_schedule_after_load();
        if crate::session::pause() & crate::openttd::PauseMode::LINK_GRAPH.bit() != 0 {
            observe([8, 1, 0]);
        }
    } else {
        super::pause(services(), services().shared, bits & 1 != 0, bits == 0);
    }
}

/// Install the fixture's pools and services, with a two-day `recalc_interval`.
fn install() -> &'static Services {
    // `_settings_game.linkgraph.recalc_interval`: two days.
    let settings: &'static _ = Box::leak(Box::new(crate::settings_type::SettingAddresses {
        recalc_interval: &4,
        ..crate::settings_type::SettingAddresses::NULL
    }));
    let shared: &'static _ = Box::leak(Box::new(crate::services::Services {
        settings,
        ..crate::test_tables::SHARED
    }));
    let (graph_pool, job_pool) = (new_pool(), new_pool());
    let schedule: &'static Services = Box::leak(Box::new(Services {
        allocation_failure,
        assert_graph,
        begin_measurement,
        can_allocate,
        destroy_job,
        graph_pool: Box::leak(Box::new(graph_pool.cast_const())),
        job_pool: Box::leak(Box::new(job_pool.cast_const())),
        new_job: new_job_record,
        shared,
        start_thread,
        vehicle: std::ptr::null(),
    }));
    with(|f| {
        f.trace.clear();
        f.graphs = graph_pool;
        f.jobs = job_pool;
        f.services = schedule;
    });
    // SAFETY: Serialized by the game-state test lock; cleared by `run`.
    unsafe { *PAUSE_OBSERVER.get() = Some(observe_pause) };
    schedule
}
/// After the fixture's `ShiftDates` and `Clear`: each pooled graph's compression and first
/// node dates, then each pooled job's join date and abort flag.
fn observe_dates() {
    for graph in shells::<GraphShell>(graphs()) {
        // SAFETY: Live fixture graphs and their records; copied before recording.
        let (tag, compressed, updated) = unsafe {
            let owner = (*graph).owner;
            (
                (*graph).tag,
                (*owner).last_compression,
                (*owner).nodes.as_slice()[0].last_update,
            )
        };
        observe([30, tag.into(), compressed]);
        observe([31, tag.into(), updated]);
    }
    for job in shells::<JobShell>(jobs()) {
        // SAFETY: Live fixture jobs and their records; copied before recording.
        let (index, join_date, aborted) = unsafe {
            let record = (*job).record;
            (
                (*job).index,
                (*record).join_date,
                (*record).aborted.load(Ordering::Acquire),
            )
        };
        observe([32, index.into(), join_date]);
        observe([33, index.into(), aborted.into()]);
    }
}
/// The fixture's `Run`, step for step; returns its trace and the shells to delete.
fn run(schedule: &'static Services) -> (Vec<[i32; 3]>, [*mut GraphShell; 3]) {
    import(&[], &[]);
    pause(0, false);
    pause(1, false);
    pause(0, true);
    let small = new_graph(0, 1, 10);
    let large = new_graph(1, 2, 11);
    let unqueued = new_graph(2, 3, 12);
    tick(0, 21, false);
    import(&[small], &[]);
    tick(0, 21, false);
    capture();
    import(&[], &[]);
    // A duplicate first pointer ends the rotation early.
    queue(small);
    queue(small);
    queue(large);
    tick(0, 20, false);
    tick(0, 21, false);
    capture();
    // SAFETY: A live fixture graph.
    dequeue(unsafe { (*small).owner });
    // The synchronous spawn completes before the job joins the running queue.
    tick(0, 21, false);
    capture();
    let first = slot(jobs(), 0).shell.cast::<JobShell>();
    let second = new_job(large);
    set_due(second, true);
    import(&[small], &[first, second]);
    // Only the front job, which is not due, is observed; its completion is not read.
    pause(0, false);
    pause(1, false);
    pause(0, true);
    tick(1, 21, false);
    capture();
    set_due(first, true);
    set_completed(first, false);
    pause(0, false);
    pause(1, false);
    pause(0, true);
    pause(2, false);
    set_completed(first, true);
    pause(1, false);
    pause(0, false);
    tick(1, 21, true);
    capture();
    // Destruction recycles graph ID 1 and queues it twice.
    import(&[small], &[second]);
    with(|f| f.recycle = true);
    tick(1, 21, false);
    with(|f| f.recycle = false);
    capture();
    let listed = new_job(small);
    let unlisted = new_job(unqueued);
    import(&[small], &[listed]);
    openttd_rust_schedule_spawn_all(schedule);
    openttd_rust_schedule_shift(schedule, -17);
    openttd_rust_schedule_clear();
    observe_dates();
    capture();
    // Clear must not delete the jobs.
    for job in [listed, unlisted] {
        // SAFETY: A live fixture job.
        assert_eq!(
            slot(jobs(), unsafe { (*job).index }).shell,
            job.cast::<c_void>()
        );
    }
    (
        with(|f| std::mem::take(&mut f.trace)),
        [small, large, unqueued],
    )
}

#[test]
fn schedule_trace_matches_original() {
    let _lock = crate::town::tests::GLOBAL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let saved = (
        crate::timer_game_common::economy_date(),
        crate::timer_game_common::economy_fraction(),
        crate::session::pause(),
    );
    let schedule = install();
    let (trace, graphs) = run(schedule);
    for job in shells::<JobShell>(jobs()) {
        delete_job(job);
    }
    import(&[], &[]);
    let replacement = with(|f| std::mem::replace(&mut f.replacement, null_mut()));
    for graph in [graphs[0], graphs[1], graphs[2], replacement] {
        delete_graph(graph);
    }
    // SAFETY: As in `install`.
    unsafe { *PAUSE_OBSERVER.get() = None };
    with(|f| f.trace.clear());
    write_economy_date(saved.0);
    write_economy_fraction(saved.1);
    crate::session::set_pause(saved.2);

    let oracle = crate::oracle::parse(include_str!("schedule-oracle.txt"));
    let compared = crate::oracle::check(
        &oracle,
        "trace",
        trace
            .iter()
            .enumerate()
            .map(|(index, [kind, a, b])| (format!("entry {index}"), format!("{kind} {a} {b}"))),
    );
    assert_eq!(compared, 47);
}
