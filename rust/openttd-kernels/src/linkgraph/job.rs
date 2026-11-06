/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Job-private snapshots and results. The worker touches only its record and snapshot;
//! acquire/release atomics synchronize cancellation and the completion hint. The native
//! thread is joined before publication or destruction. No owner borrow spans callbacks.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
use super::{Settings, live::Graph};
use crate::cargotype::{CC_EXPRESS, CC_MAIL, PASSENGERS};
use crate::{
    cargo_flow::FlowMap, industrytype::NativeVec, linkgraph_type::LinkGraphID,
    timer_game_common::INVALID_DATE,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// Opaque native job shell, retaining the platform thread handle.
#[repr(C)]
pub struct NativeJob {
    _private: [u8; 0],
}

/// Typed job view; its records are live only from binding through post-join retirement.
pub(crate) type Pool =
    crate::pool::TypedPool<crate::linkgraph_type::LinkGraphJobID, JobRecord, NativeJob>;

crate::abi::shared_records! {
    fn abi_record;
    #[derive(Clone, Copy)]
    "OpenTTDLinkGraphJobSettings" ["station_service"] => pub struct SnapshotSettings {
        pub recalc_time: u16,
        pub recalc_interval: u16,
        pub distribution_pax: u8,
        pub distribution_mail: u8,
        pub distribution_armoured: u8,
        pub distribution_default: u8,
        pub accuracy: u8,
        pub demand_size: u8,
        pub demand_distance: u8,
        pub short_path_saturation: u8,
    }
    "OpenTTDLinkGraphJobNode" ["station_service"] => pub struct JobNode {
        pub flows: *mut FlowMap => "OpenTTDCargoFlowMap *",
    }
    "OpenTTDLinkGraphJobRecord" ["station_service"] => pub struct JobRecord {
        /// Stable native thread-shell identity through join and publication.
        pub shell: *mut NativeJob => "LinkGraphJob *",
        pub graph: *mut Graph,
        pub settings: SnapshotSettings,
        pub graph_index: LinkGraphID,
        pub join_date: i32,
        pub nodes: NativeVec<JobNode> => "RustVector<OpenTTDLinkGraphJobNode>",
        pub edge_flows: NativeVec<u32> => "RustVector<uint32_t>",
        pub completed: AtomicBool => "std::atomic<bool>",
        pub aborted: AtomicBool => "std::atomic<bool>",
    }
}
impl SnapshotSettings {
    /// `LinkGraphSettings::GetDistributionType`, with source class precedence.
    fn distribution(self, classes: u16) -> u8 {
        use crate::cargotype::{CC_ARMOURED, CC_MAIL, PASSENGERS, is_cargo_in_class};
        if is_cargo_in_class(classes, PASSENGERS) {
            self.distribution_pax
        } else if is_cargo_in_class(classes, CC_MAIL) {
            self.distribution_mail
        } else if is_cargo_in_class(classes, CC_ARMOURED) {
            self.distribution_armoured
        } else {
            self.distribution_default
        }
    }
}
impl Drop for JobNode {
    fn drop(&mut self) {
        // SAFETY: each node owns one private map, retired after the worker join.
        drop(unsafe { Box::from_raw(self.flows) });
    }
}
impl Drop for JobRecord {
    fn drop(&mut self) {
        // Source nodes/flows are destroyed before their immutable graph snapshot.
        drop(std::mem::take(&mut self.nodes));
        drop(std::mem::take(&mut self.edge_flows));
        // SAFETY: constructor transferred this independent snapshot to the job.
        drop(unsafe { Box::from_raw(self.graph) });
    }
}
/// The constructor observes settings under the game-state mutex. `graph` is null
/// for indexed save loading, whose fields are populated before starting workers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_linkgraph_job_new(
    graph: *const Graph,
    graph_index: LinkGraphID,
    settings: *const SnapshotSettings,
) -> *mut JobRecord {
    let settings = unsafe { *settings };
    let loading = graph.is_null();
    let graph = if loading {
        super::live::openttd_rust_live_graph_new_empty()
    } else {
        Box::into_raw(Box::new(unsafe { &*graph }.clone()))
    };
    let join_date = if loading {
        INVALID_DATE
    } else {
        crate::timer_game_common::economy_date().0.wrapping_add(
            i32::from(settings.recalc_time) / crate::timer_game_common::SECONDS_PER_DAY,
        )
    };
    Box::into_raw(Box::new(JobRecord {
        shell: std::ptr::null_mut(),
        graph,
        settings,
        graph_index,
        join_date,
        nodes: NativeVec::new(),
        edge_flows: NativeVec::new(),
        completed: AtomicBool::new(false),
        aborted: AtomicBool::new(false),
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_linkgraph_job_delete(job: *mut JobRecord) {
    drop(unsafe { Box::from_raw(job) });
}
/// Called at the source worker entry, including the synchronous start-failure fallback.
pub(crate) unsafe fn run(job: *mut JobRecord, cargo_classes: u16, map_max_x: u32, map_max_y: u32) {
    if unsafe { (*job).aborted.load(Ordering::Acquire) } {
        return;
    }
    let graph = unsafe { (*job).graph };
    let settings = unsafe { (*job).settings };
    let runtime = unsafe { (*job).join_date }
        .wrapping_sub(i32::from(settings.recalc_time) / crate::timer_game_common::SECONDS_PER_DAY)
        .wrapping_sub(unsafe { (*graph).last_compression })
        .wrapping_add(1);
    let input = Settings {
        accuracy: u32::from(settings.accuracy),
        demand_distance: u32::from(settings.demand_distance),
        demand_size: u32::from(settings.demand_size),
        saturation: u32::from(settings.short_path_saturation),
        distribution: u32::from(settings.distribution(cargo_classes)),
        express: u32::from(cargo_classes & (PASSENGERS | CC_MAIL | CC_EXPRESS) != 0),
        map_max_x,
        map_max_y,
        runtime: runtime as u32,
    };
    let result = unsafe {
        super::live::run(graph, &input, || {
            // Borrow only the atomic subobject while worker-owned results are mutable.
            (&*std::ptr::addr_of!((*job).aborted)).load(Ordering::Acquire)
        })
    };
    if unsafe { (*job).aborted.load(Ordering::Acquire) } {
        return;
    }
    let mut nodes = (0..unsafe { &*graph }.size())
        .map(|_| JobNode {
            flows: Box::into_raw(Box::default()),
        })
        .collect::<Vec<_>>();
    let mut shares = result.shares.as_slice();
    while let Some(first) = shares.first() {
        let count = shares.partition_point(|s| s.node == first.node && s.origin == first.origin);
        unsafe { &mut *nodes[first.node as usize].flows }.import_shares(&shares[..count]);
        shares = &shares[count..];
    }
    unsafe {
        (*job).nodes = NativeVec::from_vec(std::mem::take(&mut nodes));
        (*job).edge_flows = NativeVec::from_vec(result.edge_flows);
        if !(*job).aborted.load(Ordering::Acquire) {
            (*job).completed.store(true, Ordering::Release);
        }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "LinkGraphSettings" ["station_service"] => SnapshotSettings {
        recalc_time, recalc_interval, distribution_pax, distribution_mail,
        distribution_armoured, distribution_default, accuracy, demand_size,
        demand_distance, short_path_saturation,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cargo_flow, linkgraph::live, linkgraph_type::EdgeUpdateModes};

    #[test]
    fn worker_uses_private_inputs_and_prestart_abort_publishes_no_result() {
        // The two-station expectation is the pinned computation ABI fixture's
        // 10 units from station 12 to station 24, over one 20-unit link.
        for abort in [false, true] {
            let graph = live::openttd_rust_live_graph_new_empty();
            let mut settings = SnapshotSettings {
                recalc_time: 60,
                recalc_interval: 4,
                distribution_pax: 1,
                distribution_mail: 1,
                distribution_armoured: 1,
                distribution_default: 1,
                accuracy: 2,
                demand_size: 100,
                demand_distance: 100,
                short_path_saturation: 80,
            };
            unsafe {
                (*graph).cargo = 0;
                live::openttd_rust_live_graph_init(graph, 2);
                let nodes = &mut (*graph).nodes;
                nodes[0].supply = 10;
                nodes[0].station = 12;
                nodes[0].xy = 129;
                nodes[1].demand = 1;
                nodes[1].station = 24;
                nodes[1].xy = 132;
                live::openttd_rust_live_node_add_edge(
                    graph,
                    0,
                    1,
                    20,
                    0,
                    0,
                    EdgeUpdateModes::UNRESTRICTED,
                );
            }
            let job = unsafe {
                openttd_rust_linkgraph_job_new(graph, LinkGraphID(42), &raw const settings)
            };
            unsafe {
                (*job).join_date = 59;
                (*job).aborted.store(abort, Ordering::Release);
                // Live graph growth/destruction and later settings changes cannot
                // invalidate a worker's graph, edge or settings snapshot.
                (&mut (*graph).nodes)[0].supply = 999;
                live::openttd_rust_live_graph_init(graph, 200);
                live::openttd_rust_live_graph_delete(graph);
            }
            settings.distribution_pax = 0;
            let address = job as usize;
            std::thread::spawn(move || unsafe {
                run(address as *mut JobRecord, PASSENGERS, 127, 127);
            })
            .join()
            .unwrap();
            unsafe {
                assert_eq!((*job).settings.distribution_pax, 1);
                assert_eq!(settings.distribution_pax, 0);
                assert_eq!((*job).completed.load(Ordering::Acquire), !abort);
                assert_eq!((*job).nodes.len(), if abort { 0 } else { 2 });
                if !abort {
                    assert_eq!((*job).edge_flows.as_slice(), [10]);
                    for node in &(*job).nodes {
                        let origin = cargo_flow::openttd_rust_flow_map_at(node.flows, 12, 1);
                        assert_eq!(origin.found, 1);
                        let share = cargo_flow::openttd_rust_flow_share(origin.flow, 0, 0);
                        assert_eq!((share.station, share.cumulative), (24, 10));
                    }
                }
                openttd_rust_linkgraph_job_delete(job);
            }
        }
    }
}
