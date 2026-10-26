/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Route-link prediction (`LinkRefresher`, `linkgraph/refresh.cpp`) and both `IncreaseStats`
//! overloads (`station_cmd.cpp`). Graph allocation, nodes, merging and edges stay native.
//! Prediction frames are private to one entry. Temporary cargo and subtype writes happen in
//! source order around the native subtype and capacity callbacks and are restored before the
//! next vehicle.
// Truncating casts are the original's `uint` to `uint16_t` stores (below the
// stored value) and its `Map::LogX()` byte; `if_not_else` keeps source order.
#![allow(unsafe_code, clippy::cast_possible_truncation, clippy::if_not_else)]
use super::{VehicleRef, VehicleServices, VehicleType};
use crate::vehicle_base::{VS_CRASHED, VS_HIDDEN, VS_STOPPED};
use crate::{
    base_consist::{VF_LOADING_FINISHED, VF_STOP_LOADING},
    cargo_storage::List,
    cargo_type::{CARGO_AUTO_REFIT, INVALID_CARGO, NUM_CARGO},
    cargopacket::MTA_LOAD,
    linkgraph::live,
    linkgraph_schedule::{self, Services as ScheduleServices},
    linkgraph_type::{EdgeUpdateModes, INVALID_LINK_GRAPH},
    order_type::{
        INVALID_VEH_ORDER_ID, ODATF_HALT, ODATF_NEAREST_DEPOT, ODTF_PART_OF_ORDERS, OLFB_NO_LOAD,
        OT_CONDITIONAL, OT_DUMMY, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_IMPLICIT, OT_LOADING,
        OUFB_NO_UNLOAD,
    },
    orders::VisitOrders,
    services::{CommandCost, Services as SharedServices},
    station::{BaseStation, Station},
    station_map::is_station_road_stop_tile,
    station_service::{Links, Loading, Scope, World, has_rating, prepare_unload, scope_context},
    station_type::INVALID_STATION,
    tile_type::MP_STATION,
    track_type::TRACK_BIT_DEPOT,
    train::VRF_LEAVING_STATION,
    vehicle_transactions::refit,
    witness,
};
use std::{collections::BTreeSet, ffi::c_void};

/// One consist part's `cargo_cap`, `refit_cap` and `cargo_type`.
#[allow(clippy::struct_field_names)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capacity {
    pub capacity: u16,
    pub remaining: u16,
    pub cargo: u8,
}

/// Front loading fields; none changes during one prediction or arrival.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Prediction {
    pub loading_tick: u64,
    pub last_loading: u16,
    pub last_station: u16,
    pub ticks: u16,
}

/// `STR_PERCENT_UP`, `STR_PERCENT_DOWN`, `STR_PERCENT_UP_DOWN`, `STR_PERCENT_NONE`.
#[repr(C)]
pub struct PercentStrings {
    pub up: u32,
    pub down: u32,
    pub up_down: u32,
    pub none: u32,
}

/// Two native `CommandCost` stack slots for one nested depot refit.
#[repr(C)]
pub struct Costs {
    pub out: *mut CommandCost,
    pub temporary: *mut CommandCost,
}

#[repr(C)]
pub struct Services {
    pub shared: &'static SharedServices,
    pub station: &'static World,
    pub links: &'static Links,
    pub loading: &'static Loading,
    pub refit: &'static refit::Services,
    pub(crate) schedule: &'static ScheduleServices,
    pub capacity: extern "C" fn(VehicleRef) -> Capacity,
    pub prediction: extern "C" fn(VehicleRef) -> Prediction,
    /// Virtual `GetDisplayMaxSpeed()`.
    pub display_speed: extern "C" fn(VehicleRef) -> i32,
    /// `BaseStation::xy`.
    pub location: extern "C" fn(*const BaseStation) -> u32,
    pub percent: PercentStrings,
    /// `v->last_loading_station = station`.
    pub set_last_loading: extern "C" fn(VehicleRef, u16),
    /// `v->last_loading_tick = tick`.
    pub set_loading_tick: extern "C" fn(VehicleRef, u64),
    /// Arrival window invalidation and `Station::MarkTilesDirty(true)`.
    pub arrival_dirty: extern "C" fn(VehicleRef),
    /// `&st->goods[cargo].GetOrCreateData().cargo`.
    pub station_cargo: extern "C" fn(*mut Station, u8) -> *mut List,
    /// `Debug(misc, 1, "cancelling cargo reservation")`.
    pub reservation_log: extern "C" fn(),
    /// `HideFillingPercent(&v->fill_percent_te_id)`.
    pub hide_filling: extern "C" fn(VehicleRef),
    /// `v->trip_occupancy = value`.
    pub set_occupancy: extern "C" fn(VehicleRef, u8),
    /// Station randomisation and animation triggers for a departing train.

    /// Road stop randomisation and animation triggers for a departing road vehicle.

    /// Virtual `PlayLeaveStationSound()`.
    pub leave_sound: extern "C" fn(VehicleRef),
    /// Depot window invalidation (`InvalidateWindowData` only off rails).
    pub depot_windows: extern "C" fn(VehicleRef, bool),
    /// Ship `UpdateViewport(true, true)` and its depot window.
    pub ship_depot_viewport: extern "C" fn(VehicleRef),
    /// `TriggerVehicleRandomisation(v, VehicleRandomTrigger::Depot)`.
    /// `InvalidateWindowData(WC_VEHICLE_VIEW, v->index)`.
    pub view_invalidate: extern "C" fn(VehicleRef),
    /// Runs the scope with two native `CommandCost` stack slots.
    pub with_costs: extern "C" fn(*mut c_void, Scope),
    /// The local company's refit-failure advice.
    pub refit_failed_news: extern "C" fn(VehicleRef),
    /// The local company's `ShowCostOrIncomeAnimation`.
    pub cost_animation: extern "C" fn(VehicleRef, i64),
    /// The local company's waiting advice, then `ScriptEventVehicleWaitingInDepot`.
    pub waiting_news: extern "C" fn(VehicleRef),
}
impl Services {
    /// Preserve C++ assertions, including when entered directly from Rust.
    fn check(&self, condition: impl FnOnce() -> bool) {
        let native = self.refit.common.transactions;
        if native.assertions {
            // SAFETY: the shared native assertion leaf retains no state.
            unsafe { (native.check)(condition()) };
        }
    }
    pub(crate) fn vehicles(&self) -> &VehicleServices {
        self.refit.common.vehicles
    }
    fn maintenance(&self) -> &super::maintenance::Services {
        // SAFETY: the static vehicle table's maintenance table is immutable and static.
        unsafe { &*self.vehicles().maintenance }
    }
    /// `v->index`.
    fn id(&self, v: VehicleRef) -> u32 {
        // SAFETY: a live shell; the scalar read retains no borrow.
        unsafe { (self.refit.common.base.groups.vehicle_id)(v.0.as_ptr().cast()) }
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
    let st2 = (s.station.station)(next).base;
    let first_id = (l.graphs.graph_id)(st, cargo);
    let second_id = (l.graphs.graph_id)(st2, cargo);
    let mut graph = std::ptr::null_mut();
    if first_id == INVALID_LINK_GRAPH {
        if second_id == INVALID_LINK_GRAPH {
            if (l.graphs.available)() {
                witness::hit(witness::VISITS_GRAPH_NEW);
                graph = (l.graphs.allocate)(cargo);
                linkgraph_schedule::openttd_rust_schedule_queue(s.schedule, graph);
                unsafe {
                    live::join(graph, (l.graphs.station_id)(st2), l.graphs);
                }
            } else {
                witness::hit(witness::VISITS_GRAPH_FAILURE);
                (l.graphs.allocation_failure)();
            }
        } else {
            graph = (l.graphs.get)(second_id);
        }
        if !graph.is_null() {
            unsafe {
                live::join(graph, (l.graphs.station_id)(st), l.graphs);
            }
        }
    } else if second_id == INVALID_LINK_GRAPH {
        graph = (l.graphs.get)(first_id);
        witness::hit(witness::VISITS_GRAPH_JOIN);
        unsafe {
            live::join(graph, (l.graphs.station_id)(st2), l.graphs);
        }
    } else {
        graph = (l.graphs.get)(first_id);
        if first_id != second_id {
            let second = (l.graphs.get)(second_id);
            if unsafe {
                (&*live::resolve(graph, l.graphs)).size()
                    < (&*live::resolve(second, l.graphs)).size()
            } {
                witness::hit(witness::VISITS_GRAPH_MERGE_SECOND);
                unsafe {
                    linkgraph_schedule::openttd_rust_schedule_dequeue(graph);
                    live::openttd_rust_live_graph_merge(
                        second,
                        graph,
                        (s.shared.economy_date)(),
                        l.graphs,
                    );
                }
                graph = second;
            } else {
                witness::hit(witness::VISITS_GRAPH_MERGE_FIRST);
                unsafe {
                    linkgraph_schedule::openttd_rust_schedule_dequeue(second);
                    live::openttd_rust_live_graph_merge(
                        graph,
                        second,
                        (s.shared.economy_date)(),
                        l.graphs,
                    );
                }
            }
        }
    }
    if !graph.is_null() {
        // Merging rewrites both goods entries, so the nodes are read afterwards.
        let from = (l.graphs.node)(st, cargo);
        let to = (l.graphs.node)(st2, cargo);
        unsafe {
            (&mut *live::resolve(graph, l.graphs)).nodes[usize::from(from)].update_edge(
                to,
                capacity,
                usage,
                time,
                modes,
                (s.shared.economy_date)(),
                l.graphs.assertions,
            );
        }
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
    /// Original copy-assignment reuses the fixed-length refit vector's storage.
    /// Prediction changes payloads, never the number of consist parts.
    fn restore(&mut self, backup: &Self) {
        self.vehicle = backup.vehicle;
        self.capacities = backup.capacities;
        self.refits.clone_from(&backup.refits);
        self.cargo = backup.cargo;
        self.allow_merge = backup.allow_merge;
        self.full_loading = backup.full_loading;
        self.prediction = backup.prediction;
    }

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
            let amount = unsafe {
                crate::engine_capabilities::openttd_rust_engine_capacity(
                    r.capabilities,
                    e,
                    shell,
                    &raw mut mail_capacity,
                )
            };

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
        s.check(|| !s.orders().list(self.vehicle).is_null());
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
        s.check(|| !s.orders().list(self.vehicle).is_null());
        let o = s.orders();
        let list = o.list(self.vehicle);
        let next_station = o.order(list, next).destination;
        let cur_order = o.order(list, cur);
        let st = s.station_if_valid(cur_order.destination);
        if st.is_null()
            || next_station == INVALID_STATION
            || next_station == (s.links.graphs.station_id)(st)
        {
            return;
        }
        let st_to = (s.station.station)(next_station).base;
        let v = self.vehicle;
        let map = s.shared.map();
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
            if !self.allow_merge
                && (s.links.graphs.graph_id)(st, cargo) != (s.links.graphs.graph_id)(st_to, cargo)
            {
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
                && (s.links.graphs.station_id)(st) == self.prediction.last_station
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
        s.check(|| !s.orders().list(self.vehicle).is_null());
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
                            self.restore(&backup);
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
            s.check(|| c.cargo < NUM_CARGO);
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

/// `Vehicle::ResetRefitCaps`.
pub(crate) fn reset_refit_caps(front: VehicleRef, s: &Services) {
    let r = s.refit;
    let mut part = Some(front);
    while let Some(v) = part {
        let shell = v.0.as_ptr();
        (r.set_refit_cap)(shell, (r.cargo_cap)(shell));
        part = (s.vehicles().next)(v);
    }
}

/// `Vehicle::CancelReservation`.
pub(crate) fn cancel_reservation(front: VehicleRef, next: u16, st: *mut Station, s: &Services) {
    let mut part = Some(front);
    while let Some(v) = part {
        let c = (s.loading.cargo_capacity)(v);
        let cargo = c.owner.cast_mut();
        // SAFETY: the live vehicle's cargo owner; each call ends before the next.
        if unsafe { crate::cargo_storage::openttd_rust_cargo_list_action_count(cargo, MTA_LOAD) }
            > 0
        {
            witness::hit(witness::VISITS_RESERVATION);
            (s.reservation_log)();
            let destination = (s.station_cargo)(st, c.cargo);
            // `VehicleCargoList::Return(UINT_MAX, dest, next, v->tile)`.
            unsafe {
                crate::cargo_storage::openttd_rust_cargo_list_move(
                    cargo,
                    destination,
                    s.refit.cargo,
                    0,
                    u32::MAX,
                    next,
                    INVALID_STATION,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    (s.vehicles().tile)(v),
                );
            }
        }
        // SAFETY: as above.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_keep(cargo) };
        part = (s.vehicles().next)(v);
    }
}

/// `CalcPercentVehicleFilled`; the colour string is computed only when wanted.
pub(crate) fn percent_filled(front: VehicleRef, colour: Option<&mut u32>, s: &Services) -> u8 {
    let mut count = 0_i32;
    let mut max = 0_i32;
    let mut cars = 0_i32;
    let mut unloading = 0_i32;
    let mut loading = false;

    let o = s.orders();
    let order = o.current(front);
    let is_loading = order.kind() == OT_LOADING;

    // The station may be null when the colour string is not wanted.
    let st = s.station_if_valid((s.vehicles().last_station)(front));
    s.check(|| colour.is_none() || (!st.is_null() && is_loading));

    let order_no_load = is_loading && order.load() == OLFB_NO_LOAD;
    let order_full_load = is_loading && order.is_full_load();
    let wants_colour = colour.is_some();

    let mut part = Some(front);
    while let Some(v) = part {
        count = count.wrapping_add((s.loading.stored_count)(v).cast_signed());
        let p = (s.loading.part)(v);
        max = max.wrapping_add(i32::from(p.capacity));
        if p.capacity != 0 && wants_colour {
            unloading = unloading.wrapping_add(i32::from(p.unloading));
            loading |= !order_no_load
                && (order_full_load || has_rating((s.station.owner)(st), p.cargo))
                && o.flags(front) & VF_LOADING_FINISHED == 0
                && o.flags(front) & VF_STOP_LOADING == 0;
            cars = cars.wrapping_add(1);
        }
        part = (s.vehicles().next)(v);
    }

    if let Some(colour) = colour {
        let p = &s.percent;
        *colour = if unloading == 0 && loading {
            p.up
        } else if unloading == 0 && !loading {
            p.none
        } else if cars == unloading || !loading {
            p.down
        } else {
            p.up_down
        };
    }

    // Train without capacity.
    if max == 0 {
        return 100;
    }

    if count.wrapping_mul(2) < max {
        // Less than 50%; round up, so that 0% means really empty. `CeilDiv`
        // takes both operands as `uint`.
        let (a, b) = (count.wrapping_mul(100).cast_unsigned(), max.cast_unsigned());
        (a.wrapping_add(b).wrapping_sub(1) / b) as u8
    } else {
        // More than 50%; round down, so that 100% means really full.
        (count.wrapping_mul(100) / max).cast_unsigned() as u8
    }
}

/// # Safety
/// `front` is a live front vehicle; `colour` is null or writable; `s` is the static table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_percent_filled(
    front: VehicleRef,
    colour: *mut u32,
    s: *const Services,
) -> u8 {
    percent_filled(front, unsafe { colour.as_mut() }, unsafe { &*s })
}

/// `Vehicle::BeginLoading`.
pub(crate) fn begin_loading(v: VehicleRef, s: &Services) {
    s.check(|| {
        (s.shared.tile_type)((s.vehicles().tile)(v)) == MP_STATION
            || (s.vehicles().vehicle_type)(v) == VehicleType::SHIP.0
    });
    let front = (s.prediction)(v);
    witness::hit(witness::VISITS_BEGIN_LOADING);
    let travel_time = (s.shared.tick_counter)().wrapping_sub(front.loading_tick);
    let o = s.orders();
    o.begin_loading(v);

    let order = o.current(v);
    if front.last_loading != INVALID_STATION
        && front.last_loading != front.last_station
        && (order.load() != OLFB_NO_LOAD || order.unload() != OUFB_NO_UNLOAD)
    {
        // `IncreaseStats(Station::Get(...), this, ..., travel_time)` narrows to uint32.
        increase_vehicle_stats(
            (s.station.station)(front.last_loading).base,
            v,
            front.last_station,
            travel_time as u32,
            s,
        );
    }

    prepare_unload(v, s.station, s.loading);

    (s.arrival_dirty)(v);
    (s.vehicles().write_speed)(v, 0);
    (s.maintenance().mark_dirty)(v);
}

/// `Vehicle::LeaveStation`.
pub(crate) fn leave_station(v: VehicleRef, s: &Services) {
    s.check(|| s.orders().current(v).kind() == OT_LOADING);
    let l = s.loading;
    // `delete this->cargo_payment`; the destructor clears the vehicle's pointer.
    (l.delete_payment)((l.get_payment)(v));
    s.check(|| (l.get_payment)(v).is_null());

    let o = s.orders();
    // Only update the timetable if the vehicle was supposed to stop here.
    if o.current(v).non_stop() != 0 {
        o.update_timetable(v, false);
    }

    let order = o.current(v);
    if order.load() != OLFB_NO_LOAD || order.unload() != OUFB_NO_UNLOAD {
        let front = (s.prediction)(v);
        if order.can_leave_with_cargo(front.last_loading != INVALID_STATION) {
            witness::hit(witness::VISITS_LEAVE_LOADED);
            // Refresh next hop stats at least once during the stop, with
            // refit_cap == cargo_cap for each vehicle in the consist.
            reset_refit_caps(v, s);
            refresh_links(v, true, false, s);

            // The vehicle could load here or stop with cargo loaded.
            (s.set_last_loading)(v, front.last_station);
            (s.set_loading_tick)(v, (s.shared.tick_counter)());
        } else {
            // It had to unload or transfer everything and leaves empty.
            witness::hit(witness::VISITS_LEAVE_EMPTY);
            (s.set_last_loading)(v, INVALID_STATION);
        }
    }

    o.make_leave_station(v);
    let st = (s.station.station)((s.vehicles().last_station)(v));
    cancel_reservation(v, INVALID_STATION, st.native, s);
    // SAFETY: the live station's queue owner; the vehicle shell is its key.
    unsafe {
        crate::station_service::openttd_rust_station_queue_remove(st.state, v.0.as_ptr().cast());
    }

    (s.hide_filling)(v);
    (s.set_occupancy)(v, percent_filled(v, None, s));

    let kind = VehicleType((s.vehicles().vehicle_type)(v));
    if kind == VehicleType::TRAIN && (s.vehicles().status)(v) & VS_CRASHED == 0 {
        // Trigger station animation (trains only).
        let tile = (s.vehicles().tile)(v);
        if (s.shared.tile_type)(tile) == MP_STATION {
            crate::animated_tile::rail_event(
                st.base,
                tile,
                crate::station_type::SAT_VEHICLE_DEPARTS,
                crate::station_type::SRT_VEHICLE_DEPARTS,
            );
        }
        let train = (s.refit.train_handle)(v.0.as_ptr());
        train.set_flags(train.get_flags() | VRF_LEAVING_STATION);
    }
    if kind == VehicleType::ROAD && (s.vehicles().status)(v) & VS_CRASHED == 0 {
        // Trigger road stop animation.
        let tile = (s.vehicles().tile)(v);
        if is_station_road_stop_tile(s.shared, tile) {
            crate::animated_tile::road_event(
                st.base,
                tile,
                crate::station_type::SAT_VEHICLE_DEPARTS,
                crate::station_type::SRT_VEHICLE_DEPARTS,
            );
        }
    }

    (s.maintenance().mark_dirty)(v);
}

/// `Vehicle::HandleLoading`.
pub(crate) fn handle_loading(v: VehicleRef, mode: bool, s: &Services) {
    let o = s.orders();
    let order = o.current(v);
    match order.kind() {
        OT_LOADING => {
            let wait_time = i32::from(order.timetabled_wait())
                .wrapping_sub(o.lateness(v))
                .max(0);

            // Not the first call for this tick, or still loading.
            if mode || o.flags(v) & VF_LOADING_FINISHED == 0 || o.time(v) < wait_time {
                return;
            }

            witness::hit(witness::VISITS_LOADING_DEPART);
            (s.leave_sound)(v);

            leave_station(v, s);

            // Only advance to next order if we just loaded at the current one.
            let order = o.get(v, o.implicit(v));
            if order.is_none_or(|order| {
                (order.kind() != OT_IMPLICIT && order.kind() != OT_GOTO_STATION)
                    || order.destination != (s.vehicles().last_station)(v)
            }) {
                return;
            }
        }
        OT_DUMMY => witness::hit(witness::VISITS_DUMMY),
        _ => return,
    }

    o.increment_implicit(v);
}

/// `VehicleEnteredDepotThisTick`.
fn entered_depot_this_tick(v: VehicleRef, s: &Services) {
    let vs = s.vehicles();
    // Stop in the depot if the vehicle was in 'stopping' state.
    // SAFETY: the serial game thread's pending map; no access survives the call.
    unsafe {
        crate::fleet::pending::openttd_rust_fleet_pending_add(
            s.id(v),
            (vs.status)(v) & VS_STOPPED == 0,
        );
    }
    // Always stop, so no path out of the depot is reserved before autoreplace.
    (vs.write_status)(v, (vs.status)(v) | VS_STOPPED);
}

/// `VehicleEnterDepot`.
pub(crate) fn enter_depot(v: VehicleRef, s: &Services) {
    s.check(|| {
        // SAFETY: a live serialized shell; First() does not retain a borrow.
        (unsafe { (s.refit.common.transactions.first)(v.0.as_ptr().cast()) }) == v.0.as_ptr().cast()
    });
    witness::hit(witness::VISITS_DEPOT);
    let vs = s.vehicles();
    let m = s.maintenance();
    let kind = VehicleType((vs.vehicle_type)(v));
    s.orders().dirty_vehicle_windows(v);
    match kind {
        VehicleType::TRAIN => {
            let train = (s.refit.train_handle)(v.0.as_ptr());
            crate::train::enter_depot(train, s.refit.train, s.shared);
        }
        VehicleType::ROAD => {}
        VehicleType::SHIP => {
            let id = s.id(v);
            let ship = s.refit.ship;
            // SAFETY: the live ship's canonical state owner and static tables.
            unsafe {
                crate::ship_control::openttd_rust_ship_set_state(
                    (ship.state_owner)(id),
                    TRACK_BIT_DEPOT,
                );
                crate::ship_control::openttd_rust_ship_cache(v, id, ship, s.shared);
            }
            (s.ship_depot_viewport)(v);
        }
        VehicleType::AIRCRAFT => {
            let aircraft = (s.refit.aircraft_handle)(v.0.as_ptr());
            // SAFETY: the live aircraft handle and its static controller table.
            unsafe {
                crate::aircraft::enter_hangar(s.refit.aircraft, aircraft);
            };
        }
        _ => unreachable!(),
    }
    (m.view_dirty)(v);

    // Trains update the depot list when the first unit enters and call this
    // when the last unit enters.
    (s.depot_windows)(v, kind != VehicleType::TRAIN);

    (vs.write_status)(v, (vs.status)(v) | VS_HIDDEN);
    (vs.write_speed)(v, 0);

    // SAFETY: a live shell and the static vehicle table; callbacks may reenter.
    unsafe { super::maintenance::openttd_rust_vehicle_service_depot(v, vs) };

    entered_depot_this_tick(v, s);

    // After a vehicle trigger, the graphics and properties could change.
    unsafe {
        crate::newgrf_vehicle::trigger(s.vehicles(), v, crate::newgrf_vehicle::Trigger::Depot);
    };
    (m.mark_dirty)(v);

    (s.view_invalidate)(v);

    let o = s.orders();
    if o.current(v).kind() != OT_GOTO_DEPOT {
        return;
    }
    (m.view_dirty)(v);

    let real_order = o.get(v, o.real(v));
    let current = o.current(v);

    // Heading for another depot: keep driving. Nearest/manual depot orders
    // accept every depot.
    if current.depot_type() & ODTF_PART_OF_ORDERS != 0
        && real_order.is_some_and(|order| order.action() & ODATF_NEAREST_DEPOT == 0)
        && if kind == VehicleType::AIRCRAFT {
            current.destination != (s.shared.station_index)((vs.tile)(v))
        } else {
            (vs.dest_tile)(v) != (vs.tile)(v)
        }
    {
        witness::hit(witness::VISITS_DEPOT_ELSEWHERE);
        return;
    }

    if current.is_refit() {
        depot_refit(v, current.refit, s);
    }

    if o.current(v).depot_type() & ODTF_PART_OF_ORDERS != 0 {
        // Part of orders.
        o.delete_implicit(v);
        o.update_timetable(v, true);
        o.increment_implicit(v);
    }
    if o.current(v).action() & ODATF_HALT != 0 {
        witness::hit(witness::VISITS_DEPOT_HALT);
        // Vehicles are always stopped on entering depots. Do not restart this one.
        // SAFETY: as in `entered_depot_this_tick`.
        unsafe { crate::fleet::pending::openttd_rust_fleet_pending_add(s.id(v), false) };
        // The link across this stop can't be predicted.
        (s.set_last_loading)(v, INVALID_STATION);

        o.reset_unbunching(v);

        (s.waiting_news)(v);
    }

    // If we've entered our unbunching depot, record the round trip duration.
    o.measure_unbunching(v);

    o.make_dummy(v);
}

/// The current order's depot refit, under the owner's company.
fn depot_refit(v: VehicleRef, cargo: u8, s: &Services) {
    let l = s.loading;
    let r = s.refit;
    let base = r.common.base;
    let id = s.id(v);
    let mut body = |frame: *mut c_void| {
        // SAFETY: `with_costs` supplies two live native stack costs for this scope.
        let costs = unsafe { &*frame.cast::<Costs>() };
        let company = (l.current_company)();
        (l.set_current_company)((s.vehicles().owner)(v));
        // `Command<CMD_REFIT_VEHICLE>::Do(DoCommandFlag::Execute, v->index, cargo, 0xFF, false, false, 0)`.
        crate::command::nested(
            base.commands,
            base.shared,
            crate::services::Cost(costs.out, base.cost),
            0,
            1,
            // SAFETY: static refit services and the scope's live native costs.
            |flags| unsafe {
                refit::refit(
                    r,
                    costs.out,
                    costs.temporary,
                    flags,
                    id,
                    cargo,
                    0xff,
                    false,
                    false,
                    0,
                )
            },
        );
        (l.set_current_company)(company);

        if !(base.cost.success)(costs.out) {
            witness::hit(witness::VISITS_DEPOT_REFIT_FAILED);
            // SAFETY: as in `entered_depot_this_tick`.
            unsafe { crate::fleet::pending::openttd_rust_fleet_pending_add(id, false) };
            (s.refit_failed_news)(v);
        } else {
            witness::hit(witness::VISITS_DEPOT_REFIT);
            let cost = (base.cost.money)(costs.out);
            if cost != 0 {
                (l.subtract_profit)(v, cost.wrapping_shl(8));
                (s.cost_animation)(v, cost);
            }
        }
    };
    let (context, scope) = scope_context(&mut body);
    (s.with_costs)(context, scope);
}

macro_rules! visit_entry {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) => $body:ident) => {
        $(#[$doc])*
        /// # Safety
        /// `v` is a live front vehicle; `s` is the static production table.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(v: VehicleRef, $($arg: $ty,)* s: *const Services) {
            $body(v, $($arg,)* unsafe { &*s });
        }
    };
}
visit_entry!(openttd_rust_visit_leave_station() => leave_station);
visit_entry!(openttd_rust_visit_handle_loading(mode: bool) => handle_loading);
visit_entry!(openttd_rust_visit_enter_depot() => enter_depot);
visit_entry!(openttd_rust_visit_cancel_reservation(next: u16, st: *mut Station) => cancel_reservation);

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleVisits" ["probe"] => Services {
        shared,
        station,
        links,
        loading,
        refit,
        schedule,
        capacity,
        prediction,
        display_speed,
        location,
        percent,
        set_last_loading,
        set_loading_tick,
        arrival_dirty,
        station_cargo,
        reservation_log,
        hide_filling,
        set_occupancy,

        leave_sound,
        depot_windows,
        ship_depot_viewport,
        view_invalidate,
        with_costs,
        refit_failed_news,
        cost_animation,
        waiting_news,
    };
    "OpenTTDVisitCosts" ["probe"] => Costs { out, temporary, };
    "OpenTTDVisitPercentStrings" ["probe"] => PercentStrings { up, down, up_down, none, };
    "OpenTTDVisitPrediction" ["probe"] => Prediction { loading_tick, last_loading, last_station, ticks, };
    "OpenTTDVisitCapacity" ["probe"] => Capacity { capacity, remaining, cargo, };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autorefit_restore_reuses_storage_and_restores_payload() {
        // These identities are only compared, never dereferenced by restore.
        let vehicle = VehicleRef(std::ptr::NonNull::dangling());
        let mut frame = Refresher {
            vehicle,
            capacities: [37; NUM_CARGO as usize],
            refits: vec![
                RefitDesc {
                    cargo: 2,
                    capacity: 113,
                    remaining: 71,
                },
                RefitDesc {
                    cargo: 9,
                    capacity: 211,
                    remaining: 0,
                },
            ],
            cargo: 9,
            allow_merge: true,
            full_loading: true,
            prediction: Prediction {
                loading_tick: 123_456,
                last_loading: 17,
                last_station: 31,
                ticks: 4095,
            },
        };
        // The original makes one backup allocation, outside the cargo loop.
        let backup = frame.clone();
        let storage = frame.refits.as_ptr();
        let capacity = frame.refits.capacity();
        for _ in 0..NUM_CARGO {
            let mut other_identity = 0_u8;
            frame.vehicle = VehicleRef(std::ptr::NonNull::from(&mut other_identity).cast());
            frame.capacities.fill(0);
            frame.refits.fill(RefitDesc {
                cargo: 0,
                capacity: 0,
                remaining: 0,
            });
            frame.cargo = 0;
            frame.allow_merge = false;
            frame.full_loading = false;
            frame.prediction = Prediction {
                loading_tick: 0,
                last_loading: 0,
                last_station: 0,
                ticks: 0,
            };
            frame.restore(&backup);
            assert_eq!(frame.refits.as_ptr(), storage);
            assert_eq!(frame.refits.capacity(), capacity);
            assert_eq!(frame.refits.len(), backup.refits.len());
            assert_eq!(frame.vehicle.0, vehicle.0);
            assert_eq!(frame.capacities, backup.capacities);
            for (actual, expected) in frame.refits.iter().zip(&backup.refits) {
                assert_eq!(
                    (actual.cargo, actual.capacity, actual.remaining),
                    (expected.cargo, expected.capacity, expected.remaining)
                );
            }
            assert_eq!(frame.cargo, backup.cargo);
            assert_eq!(frame.allow_merge, backup.allow_merge);
            assert_eq!(frame.full_loading, backup.full_loading);
            assert_eq!(
                frame.prediction.loading_tick,
                backup.prediction.loading_tick
            );
            assert_eq!(
                frame.prediction.last_loading,
                backup.prediction.last_loading
            );
            assert_eq!(
                frame.prediction.last_station,
                backup.prediction.last_station
            );
            assert_eq!(frame.prediction.ticks, backup.prediction.ticks);
        }
    }
}
