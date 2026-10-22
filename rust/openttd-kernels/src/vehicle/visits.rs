/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Route-link prediction (`LinkRefresher`, `linkgraph/refresh.cpp`) and both
//! `IncreaseStats` overloads (`station_cmd.cpp`). Native shells keep vehicle,
//! station and link-graph pool identity; graph allocation, nodes, merging,
//! scheduling and edges stay native operations. Entries are serialized by the
//! game-state mutex. Prediction frames (capacities, refit descriptions and the
//! shared seen-hop set) are private to one entry: no callback can reach them.
//! Temporary cargo/subtype writes happen in source order around the native
//! subtype and capacity callbacks and are restored before the next vehicle.
// Truncating casts are the original's `uint` to `uint16_t` stores (below the
// stored value) and its `Map::LogX()` byte; `if_not_else` keeps source order.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::if_not_else)]
use super::{VehicleRef, VehicleServices, VehicleType};
use crate::{
    cargo_type::{CARGO_AUTO_REFIT, INVALID_CARGO, NUM_CARGO},
    linkgraph::NativeGraph,
    linkgraph_type::EdgeUpdateModes,
    map_func::Map,
    order_type::{
        INVALID_VEH_ORDER_ID, OLFB_NO_LOAD, OT_CONDITIONAL, OT_GOTO_DEPOT, OT_GOTO_STATION,
        OT_IMPLICIT,
    },
    orders::VisitOrders,
    services::Services as SharedServices,
    station::BaseStation,
    station_service::{Links, Loading, World},
    station_type::INVALID_STATION,
    vehicle_transactions::refit,
    witness,
};
use std::collections::BTreeSet;

/// One consist part's `cargo_cap`, `refit_cap` and `cargo_type`.
#[allow(clippy::struct_field_names)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capacity {
    pub capacity: u16,
    pub remaining: u16,
    pub cargo: u8,
}

/// Front fields the refresher reads; none changes during one prediction.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Prediction {
    pub last_loading: u16,
    pub last_station: u16,
    pub ticks: u16,
}

#[repr(C)]
pub struct Services {
    pub shared: &'static SharedServices,
    pub station: &'static World,
    pub links: &'static Links,
    pub loading: &'static Loading,
    pub refit: &'static refit::Services,
    pub capacity: extern "C" fn(VehicleRef) -> Capacity,
    pub prediction: extern "C" fn(VehicleRef) -> Prediction,
    /// Virtual `GetDisplayMaxSpeed()`.
    pub display_speed: extern "C" fn(VehicleRef) -> i32,
    /// `BaseStation::xy`.
    pub location: extern "C" fn(*const BaseStation) -> u32,
    /// `LinkGraph::CanAllocateItem()`.
    pub graph_available: extern "C" fn() -> bool,
    /// `new LinkGraph(cargo)`, then `LinkGraphSchedule::instance.Queue`.
    pub allocate_graph: extern "C" fn(u8) -> *mut NativeGraph,
    /// `Debug(misc, 0, "Can't allocate link graph")`.
    pub allocation_failure: extern "C" fn(),
    /// `ge.link_graph = lg->index; ge.node = lg->AddNode(st);`
    pub join_graph: extern "C" fn(*mut NativeGraph, *mut BaseStation, u8),
    /// `LinkGraph::Size()`.
    pub graph_size: extern "C" fn(*mut NativeGraph) -> u16,
    /// `LinkGraphSchedule::instance.Dequeue(from)`, then `into->Merge(from)`.
    pub merge_graph: extern "C" fn(*mut NativeGraph, *mut NativeGraph),
    /// `(*lg)[from].UpdateEdge(to, capacity, usage, time, modes)`.
    pub update_edge: extern "C" fn(*mut NativeGraph, u16, u16, u32, u32, u32, EdgeUpdateModes),
}
impl Services {
    fn vehicles(&self) -> &VehicleServices {
        self.refit.common.vehicles
    }
    fn orders(&self) -> VisitOrders {
        VisitOrders::new(self.refit.common.orders)
    }
    /// `Station::GetIfValid`.
    fn station_if_valid(&self, id: u16) -> *mut BaseStation {
        let st = (self.station.get)(id);
        if st.is_null() || !(self.station.is_station)(st) {
            std::ptr::null_mut()
        } else {
            st
        }
    }
}

/// `IncreaseStats(Station *, CargoType, StationID, uint, uint, uint32_t, EdgeUpdateModes)`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn increase_stats(
    st: *mut BaseStation,
    cargo: u8,
    next: u16,
    capacity: u32,
    usage: u32,
    time: u32,
    modes: EdgeUpdateModes,
    s: &Services,
) {
    let l = s.links;
    // `Station::Get(next_station_id)`.
    let st2 = (s.station.get)(next);
    let mut graph = (l.graph)(st, cargo);
    let second = (l.graph)(st2, cargo);
    if graph.is_null() {
        if second.is_null() {
            if (s.graph_available)() {
                witness::hit(witness::VISITS_GRAPH_NEW);
                graph = (s.allocate_graph)(cargo);
                (s.join_graph)(graph, st2, cargo);
            } else {
                witness::hit(witness::VISITS_GRAPH_FAILURE);
                (s.allocation_failure)();
            }
        } else {
            graph = second;
        }
        if !graph.is_null() {
            (s.join_graph)(graph, st, cargo);
        }
    } else if second.is_null() {
        witness::hit(witness::VISITS_GRAPH_JOIN);
        (s.join_graph)(graph, st2, cargo);
    } else if graph != second {
        if (s.graph_size)(graph) < (s.graph_size)(second) {
            witness::hit(witness::VISITS_GRAPH_MERGE_SECOND);
            (s.merge_graph)(second, graph);
            graph = second;
        } else {
            witness::hit(witness::VISITS_GRAPH_MERGE_FIRST);
            (s.merge_graph)(graph, second);
        }
    }
    if !graph.is_null() {
        // Merging rewrites both goods entries, so the nodes are read afterwards.
        let from = (l.node)(st, cargo);
        let to = (l.node)(st2, cargo);
        (s.update_edge)(graph, from, to, capacity, usage, time, modes);
    }
}

/// `IncreaseStats(Station *, const Vehicle *, StationID, uint32_t)`.
fn increase_vehicle_stats(
    st: *mut BaseStation,
    front: VehicleRef,
    next: u16,
    time: u32,
    s: &Services,
) {
    let mut part = Some(front);
    while let Some(v) = part {
        let c = (s.capacity)(v);
        if c.remaining > 0 {
            // The stored count can exceed refit_cap after autoreplace/autorefit.
            witness::hit(witness::VISITS_ARRIVAL_STATS);
            let usage = u32::from(c.remaining).min((s.loading.stored_count)(v));
            increase_stats(
                st,
                c.cargo,
                next,
                u32::from(c.remaining),
                usage,
                time,
                EdgeUpdateModes::INCREASE,
                s,
            );
        }
        part = (s.vehicles().next)(v);
    }
}

/// # Safety
/// `st` and `front` are live native shells; `s` is the static production table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_increase_vehicle_stats(
    st: *mut BaseStation,
    front: VehicleRef,
    next: u16,
    time: u32,
    s: *const Services,
) {
    increase_vehicle_stats(st, front, next, time, unsafe { &*s });
}

/// `LinkRefresher::RefreshFlag` bits.
const USE_NEXT: u8 = 1 << 0;
const HAS_CARGO: u8 = 1 << 1;
const WAS_REFIT: u8 = 1 << 2;
const RESET_REFIT: u8 = 1 << 3;
const IN_AUTOREFIT: u8 = 1 << 4;

/// `LinkRefresher::RefitDesc`.
#[derive(Clone, Copy)]
struct RefitDesc {
    cargo: u8,
    capacity: u16,
    remaining: u16,
}

/// One `LinkRefresher` frame; copies share only the seen-hop set.
#[derive(Clone)]
struct Refresher {
    vehicle: VehicleRef,
    capacities: [u32; NUM_CARGO as usize],
    refits: Vec<RefitDesc>,
    cargo: u8,
    allow_merge: bool,
    full_loading: bool,
    prediction: Prediction,
}

/// `LinkRefresher::Hop`: from, to, cargo.
type HopSet = BTreeSet<(u8, u8, u8)>;

impl Refresher {
    fn capacity(&mut self, cargo: u8) -> &mut u32 {
        &mut self.capacities[usize::from(cargo)]
    }

    /// `LinkRefresher::HandleRefit`.
    fn handle_refit(&mut self, refit_cargo: u8, s: &Services) -> bool {
        self.cargo = refit_cargo;
        let r = s.refit;
        let mut index = 0;
        let mut any_refit = false;
        let mut part = Some(self.vehicle);
        while let Some(v) = part {
            let shell = v.0.as_ptr();
            let e = (r.vehicle_engine)(shell);
            if (r.refit_engine)(e).mask >> self.cargo & 1 == 0 {
                index += 1;
                part = (s.vehicles().next)(v);
                continue;
            }
            any_refit = true;
            witness::hit(witness::VISITS_REFIT);

            // Back up the vehicle's cargo type and subtype; the subtype callback
            // observes the new cargo type with the old subtype.
            let temp = (r.cargo_kind)(shell);
            (r.set_cargo_kind)(shell, self.cargo, temp.subtype);
            // SAFETY: `shell` is a live vehicle; the callback reads both shells only.
            let subtype =
                unsafe { (r.common.transactions.subtype)(shell.cast(), shell.cast(), self.cargo) };
            (r.set_cargo_kind)(shell, self.cargo, subtype);

            let mut mail_capacity = 0_u16;
            let amount = (r.capacity)(e, shell, &raw mut mail_capacity);

            // Restore the original cargo type.
            (r.set_cargo_kind)(shell, temp.cargo, temp.subtype);

            // Skip on next refit.
            let desc = self.refits[index];
            if self.cargo != desc.cargo && desc.remaining > 0 {
                let total = self.capacity(desc.cargo);
                *total = total.wrapping_sub(u32::from(desc.remaining));
                self.refits[index].remaining = 0;
            } else if amount < u32::from(desc.remaining) {
                let total = self.capacity(desc.cargo);
                *total = total.wrapping_sub(u32::from(desc.remaining) - amount);
                self.refits[index].remaining = amount as u16;
            }
            self.refits[index].capacity = amount as u16;
            self.refits[index].cargo = self.cargo;

            index += 1;

            // Special case for aircraft with mail.
            if (s.vehicles().vehicle_type)(v) == VehicleType::AIRCRAFT.0 {
                let desc = self.refits[index];
                witness::hit(witness::VISITS_AIRCRAFT_MAIL);
                if mail_capacity < desc.remaining {
                    let total = self.capacity(desc.cargo);
                    *total = total.wrapping_sub(u32::from(desc.remaining - mail_capacity));
                    self.refits[index].remaining = mail_capacity;
                }
                self.refits[index].capacity = mail_capacity;
                break; // aircraft have only one vehicle
            }
            part = (s.vehicles().next)(v);
        }
        any_refit
    }

    /// `LinkRefresher::ResetRefit`.
    fn reset_refit(&mut self) {
        for desc in &mut self.refits {
            if desc.remaining == desc.capacity {
                continue;
            }
            witness::hit(witness::VISITS_RESET_REFIT);
            let total = &mut self.capacities[usize::from(desc.cargo)];
            // `int` difference added to `uint`: wrapping, like the original.
            *total = total
                .wrapping_add(u32::from(desc.capacity).wrapping_sub(u32::from(desc.remaining)));
            desc.remaining = desc.capacity;
        }
    }

    /// `LinkRefresher::PredictNextOrder`.
    fn predict_next_order(
        &self,
        cur: u8,
        mut next: u8,
        mut flags: u8,
        mut num_hops: u32,
        seen: &mut HopSet,
        s: &Services,
    ) -> u8 {
        let o = s.orders();
        let list = o.list(self.vehicle);
        while next < o.count(list)
            && (flags & USE_NEXT == 0 || o.order(list, next).kind() == OT_CONDITIONAL)
        {
            // After the first step any further non-conditional order is good.
            flags |= USE_NEXT;

            let order = o.order(list, next);
            if order.kind() == OT_CONDITIONAL {
                let skip_to = o.decision(list, order.condition_skip_to(), num_hops);
                if skip_to != INVALID_VEH_ORDER_ID && num_hops < u32::from(o.count(list)) {
                    // Copies of the capacity tracking lists; the hop set is shared.
                    witness::hit(witness::VISITS_CONDITIONAL_BRANCH);
                    let mut branch = self.clone();
                    branch.refresh_links(cur, skip_to, flags, num_hops.wrapping_add(1), seen, s);
                }
            }

            // Reassign next with the following stop: a station or a depot.
            next = o.decision(list, o.next(list, next), num_hops);
            num_hops = num_hops.wrapping_add(1);
        }
        next
    }

    /// `LinkRefresher::RefreshStats`.
    fn refresh_stats(&self, cur: u8, next: u8, s: &Services) {
        let o = s.orders();
        let list = o.list(self.vehicle);
        let next_station = o.order(list, next).destination;
        let cur_order = o.order(list, cur);
        let st = s.station_if_valid(cur_order.destination);
        if st.is_null()
            || next_station == INVALID_STATION
            || next_station == (s.links.station_id)(st)
        {
            return;
        }
        let st_to = (s.station.get)(next_station);
        let v = self.vehicle;
        let map = Map {
            log_x: (s.shared.map_log_x)() as u8,
        };
        for cargo in 0..NUM_CARGO {
            // Refresh the link and give it a minimum capacity.
            let cargo_quantity = self.capacities[usize::from(cargo)];
            if cargo_quantity == 0 {
                continue;
            }

            if (s.display_speed)(v) == 0 {
                continue;
            }

            // If not allowed to merge link graphs, the stations must already share one.
            if !self.allow_merge && (s.links.graph)(st, cargo) != (s.links.graph)(st_to, cargo) {
                witness::hit(witness::VISITS_MERGE_BLOCKED);
                continue;
            }

            // A link is at least partly restricted if a vehicle can't load at its source.
            let restricted_mode = if cur_order.load() != OLFB_NO_LOAD {
                EdgeUpdateModes::UNRESTRICTED
            } else {
                EdgeUpdateModes::RESTRICTED
            };
            // Travel time at half the consist's maximum speed, in tiles/tick.
            let distance = map.distance_manhattan((s.location)(st), (s.location)(st_to));
            let time_estimate = distance.wrapping_mul(4096) / (s.display_speed)(v).cast_unsigned();

            // While full loading, add an estimate of what would have been
            // transported, unless the wait already exceeds the whole order list.
            if self.full_loading
                && !list.is_null()
                && (s.links.station_id)(st) == self.prediction.last_station
                && o.total(list) > o.time(v)
            {
                let total = o.total(list);
                let effective_capacity =
                    cargo_quantity.wrapping_mul(u32::from(self.prediction.ticks));
                if effective_capacity > total.cast_unsigned() {
                    witness::hit(witness::VISITS_FULL_LOAD_INCREASE);
                    increase_stats(
                        st,
                        cargo,
                        next_station,
                        effective_capacity / total.cast_unsigned(),
                        0,
                        0,
                        EdgeUpdateModes::INCREASE.with(restricted_mode),
                        s,
                    );
                } else if s.shared.random_range(total.cast_unsigned()) < effective_capacity {
                    witness::hit(witness::VISITS_FULL_LOAD_RANDOM);
                    increase_stats(
                        st,
                        cargo,
                        next_station,
                        1,
                        0,
                        0,
                        EdgeUpdateModes::INCREASE.with(restricted_mode),
                        s,
                    );
                } else {
                    witness::hit(witness::VISITS_FULL_LOAD_REFRESH);
                    increase_stats(
                        st,
                        cargo,
                        next_station,
                        cargo_quantity,
                        0,
                        time_estimate,
                        EdgeUpdateModes::REFRESH.with(restricted_mode),
                        s,
                    );
                }
            } else {
                increase_stats(
                    st,
                    cargo,
                    next_station,
                    cargo_quantity,
                    0,
                    time_estimate,
                    EdgeUpdateModes::REFRESH.with(restricted_mode),
                    s,
                );
            }
        }
    }

    /// `LinkRefresher::RefreshLinks`.
    fn refresh_links(
        &mut self,
        mut cur: u8,
        mut next: u8,
        mut flags: u8,
        num_hops: u32,
        seen: &mut HopSet,
        s: &Services,
    ) {
        let o = s.orders();
        let list = o.list(self.vehicle);
        while next < o.count(list) {
            let next_order = o.order(list, next);

            if (next_order.kind() == OT_GOTO_DEPOT || next_order.kind() == OT_GOTO_STATION)
                && next_order.is_refit()
            {
                flags |= WAS_REFIT;
                if next_order.refit != CARGO_AUTO_REFIT {
                    self.handle_refit(next_order.refit, s);
                } else if flags & IN_AUTOREFIT == 0 {
                    flags |= IN_AUTOREFIT;
                    witness::hit(witness::VISITS_AUTOREFIT);
                    let backup = self.clone();
                    for cargo in 0..NUM_CARGO {
                        if (s.station.cargo_valid)(cargo) && self.handle_refit(cargo, s) {
                            self.refresh_links(cur, next, flags, num_hops, seen, s);
                            self.clone_from(&backup);
                        }
                    }
                }
            }

            // Only reset the refit capacities if the "previous" next is a station.
            if flags & WAS_REFIT != 0
                && (next_order.kind() == OT_GOTO_STATION || next_order.kind() == OT_IMPLICIT)
            {
                flags |= RESET_REFIT;
            } else {
                flags &= !RESET_REFIT;
            }

            next = self.predict_next_order(cur, next, flags, num_hops, seen, s);
            if next == INVALID_VEH_ORDER_ID {
                break;
            }
            if !seen.insert((cur, next, self.cargo)) {
                witness::hit(witness::VISITS_SEEN_HOP);
                break;
            }

            let next_order = o.order(list, next);

            // Don't use the same order again, but choose a new one in the next round.
            flags &= !USE_NEXT;

            // Skip resetting and link refreshing if next order won't do anything with cargo.
            if next_order.kind() != OT_GOTO_STATION && next_order.kind() != OT_IMPLICIT {
                continue;
            }

            if flags & RESET_REFIT != 0 {
                self.reset_refit();
                flags &= !(RESET_REFIT | WAS_REFIT);
            }

            let cur_order = o.order(list, cur);
            if cur_order.kind() == OT_GOTO_STATION || cur_order.kind() == OT_IMPLICIT {
                if cur_order.can_leave_with_cargo(flags & HAS_CARGO != 0) {
                    flags |= HAS_CARGO;
                    self.refresh_stats(cur, next, s);
                    witness::hit(witness::VISITS_STATS);
                } else {
                    flags &= !HAS_CARGO;
                }
            }

            // "cur" is only assigned here if the stop is a station.
            cur = next;
        }
    }
}

/// `LinkRefresher::Run`.
pub(crate) fn refresh_links(v: VehicleRef, allow_merge: bool, is_full_loading: bool, s: &Services) {
    let o = s.orders();
    let list = o.list(v);
    // If there are no orders we can't predict anything.
    if list.is_null() {
        return;
    }

    // Make sure the first order is a useful order.
    let first = o.decision(list, o.implicit(v), 0);
    if first == INVALID_VEH_ORDER_ID {
        return;
    }

    // Assemble the list of capacities.
    let mut refresher = Refresher {
        vehicle: v,
        capacities: [0; NUM_CARGO as usize],
        refits: Vec::new(),
        cargo: INVALID_CARGO,
        allow_merge,
        full_loading: is_full_loading,
        prediction: (s.prediction)(v),
    };
    let mut part = Some(v);
    while let Some(u) = part {
        let c = (s.capacity)(u);
        refresher.refits.push(RefitDesc {
            cargo: c.cargo,
            capacity: c.capacity,
            remaining: c.remaining,
        });
        if c.remaining > 0 {
            let total = refresher.capacity(c.cargo);
            *total = total.wrapping_add(u32::from(c.remaining));
        }
        part = (s.vehicles().next)(u);
    }

    let flags = if refresher.prediction.last_loading != INVALID_STATION {
        HAS_CARGO
    } else {
        0
    };
    witness::hit(witness::VISITS_REFRESH);
    let mut seen_hops = HopSet::new();
    refresher.refresh_links(first, first, flags, 0, &mut seen_hops, s);
}

/// # Safety
/// `v` is a live front vehicle; `s` is the static production table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_refresh_links(
    v: VehicleRef,
    allow_merge: bool,
    is_full_loading: bool,
    s: *const Services,
) {
    refresh_links(v, allow_merge, is_full_loading, unsafe { &*s });
}
