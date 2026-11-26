/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station and depot visits of a vehicle: the link refresher (`LinkRefresher`,
//! `linkgraph/refresh.cpp`), both `IncreaseStats` overloads (`station_cmd.cpp`), and
//! `VehicleEnteredDepotThisTick`, `CalcPercentVehicleFilled`, `VehicleEnterDepot`, the tail of
//! `Vehicle::BeginLoading` (its order half is `orders.rs`'s `begin_loading_orders`),
//! `Vehicle::CancelReservation`, `Vehicle::LeaveStation`, `Vehicle::ResetRefitCaps` and
//! `Vehicle::HandleLoading` (`vehicle.cpp`).
//!
//! Every `VehicleRef` names a live vehicle of the serialized game state for the synchronous
//! entry, and every table this file reaches is immutable for the process. Station, graph and
//! cargo-list pointers come from those tables and are live for the entry; no Rust borrow of
//! them spans a callback.
use super::{VehicleRef, VehicleServices};
use crate::base_consist::VehicleFlag;
use crate::base_station_base;
use crate::cargo_type::CargoType;
use crate::cargopacket::MoveToAction;
use crate::debug::debug;
use crate::linkgraph::live;
use crate::linkgraph::refresh::{RefreshFlag, RefreshFlags};
use crate::linkgraph_schedule::{self, Services as ScheduleServices};
use crate::linkgraph_type::{EdgeUpdateModes, INVALID_LINK_GRAPH, LinkGraphID};
use crate::news_func::add_vehicle_advice_news_item;
use crate::news_type::AdviceType;
use crate::order_type::{
    OrderDepotActionFlag, OrderDepotTypeFlag, OrderLoadType, OrderType, OrderUnloadType,
    VehicleOrderID,
};
use crate::script_event_types::ScriptEvent;
use crate::services::Services as SharedServices;
use crate::services_assert::{assert_with, not_reached};
use crate::ship_control::{Ship, Tables, update_cache};
use crate::station::{BaseStation, Station};
use crate::station_service::{Links, Loading, World, has_rating, prepare_unload};
use crate::station_type::StationID;
use crate::strings_func::get_encoded_string;
use crate::strings_type::StringID;
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::{
    STR_NEWS_ORDER_REFIT_FAILED, STR_NEWS_TRAIN_IS_WAITING, STR_PERCENT_DOWN, STR_PERCENT_NONE,
    STR_PERCENT_UP, STR_PERCENT_UP_DOWN,
};
use crate::tile_type::TileType;
use crate::track_type::TrackBits;
use crate::vehicle_base::VehState;
use crate::vehicle_gui::get_window_class_for_vehicle_type;
use crate::vehicle_transactions::refit;
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::widgets::vehicle_widget::VehicleViewWidgets;
use crate::window_func::{
    invalidate_window_data, set_window_classes_dirty, set_window_dirty, set_window_widget_dirty,
};
use crate::window_type::{WindowClass, WindowNumber};
use crate::witness;
use std::collections::BTreeSet;

#[repr(C)]
pub struct Services {
    /// `HideFillingPercent(&v->fill_percent_te_id)`.
    pub hide_filling_percent: extern "C" fn(VehicleRef),
    /// `v->last_loading_station`.
    pub last_loading_station: extern "C" fn(VehicleRef) -> u16,
    /// `v->last_loading_tick`.
    pub last_loading_tick: extern "C" fn(VehicleRef) -> u64,
    pub links: &'static Links,
    pub loading: &'static Loading,
    pub refit: &'static refit::Services,
    pub(crate) schedule: &'static ScheduleServices,
    /// `v->last_loading_station = station`.
    pub set_last_loading_station: extern "C" fn(VehicleRef, u16),
    /// `v->last_loading_tick = tick`.
    pub set_last_loading_tick: extern "C" fn(VehicleRef, u64),
    /// `v->trip_occupancy = occupancy`.
    pub set_trip_occupancy: extern "C" fn(VehicleRef, i8),
    pub shared: &'static SharedServices,
    pub station: &'static World,
    /// `&st->goods[cargo].GetOrCreateData().cargo`.
    pub station_cargo: extern "C" fn(*mut Station, u8) -> *mut crate::cargo_storage::List,
}
impl Services {
    pub(crate) fn vehicles(&self) -> &VehicleServices {
        self.refit.common.vehicles
    }
    /// The vehicle resolver table the link table holds.
    #[allow(unsafe_code)]
    fn resolver(&self) -> &crate::newgrf_vehicle::Services {
        // SAFETY: The link table's resolver pointer names the process-wide resolver table.
        unsafe { &*self.links.vehicle_resolver }
    }
    /// The vehicle table's maintenance table.
    #[allow(unsafe_code)]
    fn maintenance(&self) -> &super::maintenance::Services {
        // SAFETY: The vehicle table's maintenance pointer names the process-wide table.
        unsafe { &*self.vehicles().maintenance }
    }
    fn orders(&self) -> &'static crate::orders::Leaves {
        self.refit.common.orders
    }
    /// `Station::GetIfValid`.
    fn station_if_valid(&self, id: StationID) -> *mut BaseStation {
        let st = (self.station.get)(id.0);
        if st.is_null() || !(self.station.is_station)(st) {
            std::ptr::null_mut()
        } else {
            st
        }
    }
    /// `Station::Get(id)`.
    fn station_base(&self, id: StationID) -> *mut BaseStation {
        (self.station.station)(id.0).base
    }
}

/// `v->orders`, the vehicle's order list or null, read in place.
fn orders(v: VehicleRef) -> *mut std::ffi::c_void {
    crate::vehicle_base::order_list(v.0.as_ptr())
}

/// `v->owner == _local_company`.
fn is_local_company(v: VehicleRef, s: &Services) -> bool {
    v.owner().0 == (s.shared.local_company)()
}

/// Arguments of `IncreaseStats(Station *, CargoType, StationID, uint, uint, uint32_t,
/// EdgeUpdateModes)`.
#[derive(Clone, Copy)]
pub(crate) struct IncreaseStats {
    /// Station to get the link stats from.
    pub(crate) st: *mut BaseStation,
    /// Cargo to increase stat for.
    pub(crate) cargo: CargoType,
    /// Station the consist will be travelling to next.
    pub(crate) next_station_id: StationID,
    /// Capacity to add to link stat.
    pub(crate) capacity: u32,
    /// Usage to add to link stat.
    pub(crate) usage: u32,
    /// Travel time to add to the link stat.
    pub(crate) time: u32,
    /// Update modes to be used.
    pub(crate) modes: EdgeUpdateModes,
}

/// `IncreaseStats(Station *, CargoType, StationID, uint, uint, uint32_t, EdgeUpdateModes)`:
/// increase capacity for a link stat given by station `st`, cargo and next station.
#[allow(unsafe_code)]
pub(crate) fn increase_stats(s: &Services, a: IncreaseStats) {
    let l = s.links;
    let st = a.st;
    let cargo = a.cargo.0;
    let st2 = s.station_base(a.next_station_id);
    let first_id = (l.graphs.graph_id)(st, cargo);
    let second_id = (l.graphs.graph_id)(st2, cargo);
    let mut lg = std::ptr::null_mut();
    if first_id == INVALID_LINK_GRAPH {
        if second_id == INVALID_LINK_GRAPH {
            if (l.graphs.available)() {
                witness::hit(witness::VISITS_GRAPH_NEW);
                lg = (l.graphs.allocate)(cargo);
                linkgraph_schedule::queue(s.schedule, lg);
                // SAFETY: The newly allocated live graph; no borrow spans the station write.
                unsafe { live::join(lg, (l.graphs.station_id)(st2), l.graphs) };
            } else {
                witness::hit(witness::VISITS_GRAPH_FAILURE);
                debug!(misc, 0, "Can't allocate link graph");
            }
        } else {
            lg = l.graphs.get(LinkGraphID(second_id));
        }
        if !lg.is_null() {
            // SAFETY: As above.
            unsafe { live::join(lg, (l.graphs.station_id)(st), l.graphs) };
        }
    } else if second_id == INVALID_LINK_GRAPH {
        lg = l.graphs.get(LinkGraphID(first_id));
        witness::hit(witness::VISITS_GRAPH_JOIN);
        // SAFETY: The station's live graph.
        unsafe { live::join(lg, (l.graphs.station_id)(st2), l.graphs) };
    } else {
        lg = l.graphs.get(LinkGraphID(first_id));
        if first_id != second_id {
            let lg2 = l.graphs.get(LinkGraphID(second_id));
            // SAFETY: Two distinct live graphs; each size read ends at once.
            if unsafe { (*lg).size() < (*lg2).size() } {
                witness::hit(witness::VISITS_GRAPH_MERGE_SECOND);
                linkgraph_schedule::dequeue(lg);
                // SAFETY: As above.
                unsafe { live::merge(lg2, lg, l.graphs) }; // Updates GoodsEntries of lg
                lg = lg2;
            } else {
                witness::hit(witness::VISITS_GRAPH_MERGE_FIRST);
                linkgraph_schedule::dequeue(lg2);
                // SAFETY: As above.
                unsafe { live::merge(lg, lg2, l.graphs) }; // Updates GoodsEntries of lg2
            }
        }
    }
    if !lg.is_null() {
        // The goods entries' nodes are read after merging, which rewrites them.
        let from = (l.graphs.node)(st, cargo);
        let to = (l.graphs.node)(st2, cargo);
        // SAFETY: The live graph; the edge update calls nothing that reaches it.
        unsafe {
            (&mut *lg).nodes[usize::from(from)].update_edge(
                to,
                a.capacity,
                a.usage,
                a.time,
                a.modes,
                crate::timer_game_common::economy_date().0,
                l.graphs.assertions,
            );
        }
    }
}

/// `v->cargo`: the vehicle's cargo list, through the resolver's scalar binding.
#[allow(unsafe_code)]
fn cargo_list(
    v: VehicleRef,
    r: &crate::newgrf_vehicle::Services,
) -> *mut crate::cargo_storage::List {
    // SAFETY: The resolver slot reads the live record's shell.
    unsafe { (r.cargo_owner)(v.0.as_ptr()) }.cast_mut()
}

/// `v->cargo.StoredCount()`.
#[allow(unsafe_code)]
fn stored_count(v: VehicleRef, r: &crate::newgrf_vehicle::Services) -> u32 {
    // SAFETY: The live vehicle's list; the read ends at once.
    unsafe { crate::cargo_storage::stored_count(cargo_list(v, r)) }
}

/// `IncreaseStats(Station *, const Vehicle *, StationID, uint32_t)`: increase capacity for
/// all link stats associated with vehicles in the given consist.
fn increase_vehicle_stats(
    st: *mut BaseStation,
    front: VehicleRef,
    next_station_id: StationID,
    time: u32,
    s: &Services,
) {
    let r = s.resolver();
    let mut part = Some(front);
    while let Some(v) = part {
        if v.refit_cap() > 0 {
            /* The cargo count can indeed be higher than the refit_cap if
             * wagons have been auto-replaced and subsequently auto-
             * refitted to a higher capacity. The cargo gets redistributed
             * among the wagons in that case.
             * As usage is not such an important figure anyway we just
             * ignore the additional cargo then.*/
            witness::hit(witness::VISITS_ARRIVAL_STATS);
            increase_stats(
                s,
                IncreaseStats {
                    st,
                    cargo: v.cargo_type(r),
                    next_station_id,
                    capacity: u32::from(v.refit_cap()),
                    usage: u32::from(v.refit_cap()).min(stored_count(v, r)),
                    time,
                    modes: EdgeUpdateModes::INCREASE,
                },
            );
        }
        part = v.next();
    }
}

/// `LinkRefresher::RefitDesc`: simulated cargo type and capacity for prediction of future
/// links.
#[derive(Clone, Copy)]
struct RefitDesc {
    /// Cargo type the vehicle will be carrying.
    cargo: CargoType,
    /// Capacity the vehicle will have.
    capacity: u16,
    /// Capacity remaining from before the previous refit.
    remaining: u16,
}

/// `LinkRefresher::Hop`: a hop the refresh algorithm might evaluate. If the same hop is seen
/// again the evaluation is stopped. The derived order is the original's defaulted
/// `operator<=>` over the fields in declaration order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Hop {
    /// Last order where vehicle could interact with cargo or absolute first order.
    from: VehicleOrderID,
    /// Next order to be processed.
    to: VehicleOrderID,
    /// Cargo the consist is probably carrying or `INVALID_CARGO` if unknown.
    cargo: CargoType,
}

/// `LinkRefresher::HopSet`.
type HopSet = BTreeSet<Hop>;

/// `LinkRefresher`: utility to refresh links a consist will visit. A copy shares only the
/// seen-hop set, which every method takes from its caller.
#[derive(Clone)]
struct Refresher {
    /// Vehicle for which the links should be refreshed.
    vehicle: VehicleRef,
    /// Current added capacities per cargo type in the consist.
    capacities: [u32; CargoType::NUM_CARGO.0 as usize],
    /// Current state of capacity remaining from previous refits versus overall capacity per
    /// vehicle in the consist.
    refit_capacities: Vec<RefitDesc>,
    /// Cargo given in last refit order.
    cargo: CargoType,
    /// If the refresher is allowed to merge or extend link graphs.
    allow_merge: bool,
    /// If the vehicle is full loading.
    is_full_loading: bool,
}

impl Refresher {
    /// `LinkRefresher::LinkRefresher`: constructor for link refreshing algorithm.
    fn new(vehicle: VehicleRef, allow_merge: bool, is_full_loading: bool, s: &Services) -> Self {
        let r = s.resolver();
        let mut refresher = Self {
            vehicle,
            capacities: [0; CargoType::NUM_CARGO.0 as usize],
            refit_capacities: Vec::new(),
            cargo: CargoType::INVALID_CARGO,
            allow_merge,
            is_full_loading,
        };
        /* Assemble list of capacities and set last loading stations to 0. */
        let mut part = Some(vehicle);
        while let Some(v) = part {
            let desc = RefitDesc {
                cargo: v.cargo_type(r),
                capacity: v.cargo_cap(),
                remaining: v.refit_cap(),
            };
            refresher.refit_capacities.push(desc);
            if desc.remaining > 0 {
                assert_with(|| desc.cargo < CargoType::NUM_CARGO);
                let total = refresher.capacity(desc.cargo);
                *total = total.wrapping_add(u32::from(desc.remaining));
            }
            part = v.next();
        }
        refresher
    }

    /// The original's copy-assignment `*this = backup`, which reuses the refit vector's
    /// storage: prediction changes its payloads, never the number of consist parts.
    fn restore(&mut self, backup: &Self) {
        self.vehicle = backup.vehicle;
        self.capacities = backup.capacities;
        self.refit_capacities.clone_from(&backup.refit_capacities);
        self.cargo = backup.cargo;
        self.allow_merge = backup.allow_merge;
        self.is_full_loading = backup.is_full_loading;
    }

    /// `capacities[cargo]`.
    fn capacity(&mut self, cargo: CargoType) -> &mut u32 {
        &mut self.capacities[usize::from(cargo.0)]
    }

    /// `LinkRefresher::HandleRefit`: handle refit orders by updating capacities and
    /// `refit_capacities`. Returns true if any vehicle was refit; false if none was.
    #[allow(
        unsafe_code,
        clippy::cast_possible_truncation,
        reason = "the original's `uint` amount stored into the `uint16_t` RefitDesc fields"
    )]
    fn handle_refit(&mut self, refit_cargo: CargoType, s: &Services) -> bool {
        let r = s.refit;
        let vs = s.vehicles();
        let resolver = s.resolver();
        self.cargo = refit_cargo;
        let mut refit_it = 0;
        let mut any_refit = false;
        let mut part = Some(self.vehicle);
        while let Some(v) = part {
            let e = (vs.engine)(v);
            if !crate::bitmath_func::has_bit64(e.refit_mask(), self.cargo.0) {
                refit_it += 1;
                part = v.next();
                continue;
            }
            any_refit = true;
            witness::hit(witness::VISITS_REFIT);

            /* Back up the vehicle's cargo type */
            let temp_cargo_type = v.cargo_type(resolver);
            let temp_subtype = v.cargo_subtype(resolver);
            v.set_cargo_type(vs, self.cargo);
            // SAFETY: `GetBestFittingSubType(v, v, cargo)` takes the live shell twice; it
            // changes and restores the cargo type and subtype, invalidates NewGRF caches and
            // runs cargo suffix callbacks, and no Rust borrow of the vehicle spans the call.
            let subtype = unsafe {
                (r.common.transactions.subtype)(v.shell().cast(), v.shell().cast(), self.cargo.0)
            };
            v.set_cargo_subtype(vs, subtype);

            let mut mail_capacity = 0_u16;
            // SAFETY: The vehicle's live engine and the live vehicle; `mail_capacity` is
            // writable for the call.
            let amount = unsafe {
                crate::engine_capabilities::determine_capacity(
                    r.capabilities,
                    e.0.as_ptr(),
                    v.0.as_ptr(),
                    &raw mut mail_capacity,
                )
            };

            /* Restore the original cargo type */
            v.set_cargo_type(vs, temp_cargo_type);
            v.set_cargo_subtype(vs, temp_subtype);

            /* Skip on next refit. */
            let it = self.refit_capacities[refit_it];
            if self.cargo != it.cargo && it.remaining > 0 {
                let total = self.capacity(it.cargo);
                *total = total.wrapping_sub(u32::from(it.remaining));
                self.refit_capacities[refit_it].remaining = 0;
            } else if amount < u32::from(it.remaining) {
                let total = self.capacity(it.cargo);
                *total = total.wrapping_sub(u32::from(it.remaining) - amount);
                self.refit_capacities[refit_it].remaining = amount as u16;
            }
            self.refit_capacities[refit_it].capacity = amount as u16;
            self.refit_capacities[refit_it].cargo = self.cargo;

            refit_it += 1;

            /* Special case for aircraft with mail. */
            if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                let it = self.refit_capacities[refit_it];
                witness::hit(witness::VISITS_AIRCRAFT_MAIL);
                if mail_capacity < it.remaining {
                    let total = self.capacity(it.cargo);
                    *total = total.wrapping_sub(u32::from(it.remaining - mail_capacity));
                    self.refit_capacities[refit_it].remaining = mail_capacity;
                }
                self.refit_capacities[refit_it].capacity = mail_capacity;
                break; // aircraft have only one vehicle
            }
            part = v.next();
        }
        any_refit
    }

    /// `LinkRefresher::ResetRefit`: restore capacities and `refit_capacities` as vehicle might
    /// have been able to load now.
    fn reset_refit(&mut self) {
        for it in &mut self.refit_capacities {
            if it.remaining == it.capacity {
                continue;
            }
            witness::hit(witness::VISITS_RESET_REFIT);
            let total = &mut self.capacities[usize::from(it.cargo.0)];
            // The `int` difference of the two `uint16_t` fields is added to `uint`.
            *total =
                total.wrapping_add(u32::from(it.capacity).wrapping_sub(u32::from(it.remaining)));
            it.remaining = it.capacity;
        }
    }

    /// `LinkRefresher::PredictNextOrder`: predict the next order the vehicle will execute and
    /// resolve conditionals by recursion and return next non-conditional order in list.
    fn predict_next_order(
        &self,
        cur: VehicleOrderID,
        mut next: VehicleOrderID,
        mut flags: RefreshFlags,
        mut num_hops: u32,
        seen_hops: &mut HopSet,
        s: &Services,
    ) -> VehicleOrderID {
        let o = s.orders();
        assert_with(|| !orders(self.vehicle).is_null());
        let orderlist = orders(self.vehicle);
        let span = crate::orders::OrderListRef::of(orderlist, o);

        /* next is good if it's either nullptr (then the caller will stop the
         * evaluation) or if it's not conditional and the caller allows it to be
         * chosen (by setting RefreshFlag::UseNext). */
        while next < span.get_num_orders()
            && (!flags.test(RefreshFlag::UseNext)
                || span
                    .order(usize::from(next.0))
                    .view()
                    .is_type(OrderType::OT_CONDITIONAL))
        {
            /* After the first step any further non-conditional order is good,
             * regardless of previous RefreshFlag::UseNext settings. The case of cur and next or
             * their respective stations being equal is handled elsewhere. */
            flags = flags.set(RefreshFlag::UseNext);

            if span
                .order(usize::from(next.0))
                .view()
                .is_type(OrderType::OT_CONDITIONAL)
            {
                let skip_to = span.get_next_decision_node(
                    span.order(usize::from(next.0))
                        .view()
                        .condition_skip_to_order(),
                    num_hops,
                );
                if skip_to != VehicleOrderID::INVALID_VEH_ORDER_ID
                    && num_hops < u32::from(span.get_num_orders().0)
                {
                    /* Make copies of capacity tracking lists. There is potential
                     * for optimization here: If the vehicle never refits we don't
                     * need to copy anything. Also, if we've seen the branched link
                     * before we don't need to branch at all. */
                    witness::hit(witness::VISITS_CONDITIONAL_BRANCH);
                    let mut branch = self.clone();
                    branch.refresh_links(
                        cur,
                        skip_to,
                        flags,
                        num_hops.wrapping_add(1),
                        seen_hops,
                        s,
                    );
                }
            }

            /* Reassign next with the following stop. This can be a station or a
             * depot.*/
            next = span.get_next_decision_node(span.get_next(next), num_hops);
            num_hops = num_hops.wrapping_add(1);
        }
        next
    }

    /// `LinkRefresher::RefreshStats`: refresh link stats for the given pair of orders.
    fn refresh_stats(&self, cur: VehicleOrderID, next: VehicleOrderID, s: &Services) {
        let o = s.orders();
        let v = self.vehicle;
        assert_with(|| !orders(v).is_null());
        let orderlist = orders(v);
        let span = crate::orders::OrderListRef::of(orderlist, o);

        let next_station = span
            .order(usize::from(next.0))
            .view()
            .destination()
            .to_station_id();
        let st = s.station_if_valid(
            span.order(usize::from(cur.0))
                .view()
                .destination()
                .to_station_id(),
        );
        if st.is_null()
            || next_station == StationID::INVALID
            || next_station == StationID((s.links.graphs.station_id)(st))
        {
            return;
        }
        let st_to = s.station_base(next_station);
        for cargo in 0..CargoType::NUM_CARGO.0 {
            /* Refresh the link and give it a minimum capacity. */

            let cargo_quantity = self.capacities[usize::from(cargo)];
            if cargo_quantity == 0 {
                continue;
            }

            if v.get_display_max_speed() == 0 {
                continue;
            }

            /* If not allowed to merge link graphs, make sure the stations are
             * already in the same link graph. */
            if !self.allow_merge
                && (s.links.graphs.graph_id)(st, cargo) != (s.links.graphs.graph_id)(st_to, cargo)
            {
                witness::hit(witness::VISITS_MERGE_BLOCKED);
                continue;
            }

            /* A link is at least partly restricted if a vehicle can't load at its source. */
            let restricted_mode =
                if span.order(usize::from(cur.0)).view().load_type() == OrderLoadType::NoLoad {
                    EdgeUpdateModes::RESTRICTED
                } else {
                    EdgeUpdateModes::UNRESTRICTED
                };
            /* This estimates the travel time of the link as the time needed
             * to travel between the stations at half the max speed of the consist.
             * The result is in tiles/tick (= 2048 km-ish/h). */
            let time_estimate = s
                .shared
                .map()
                .distance_manhattan(base_station_base::xy(st), base_station_base::xy(st_to))
                .wrapping_mul(4096)
                / v.get_display_max_speed().cast_unsigned();

            /* If the vehicle is currently full loading, increase the capacities at the station
             * where it is loading by an estimate of what it would have transported if it wasn't
             * loading. Don't do that if the vehicle has been waiting for longer than the entire
             * order list is supposed to take, though. If that is the case the total duration is
             * probably far off and we'd greatly overestimate the capacity by increasing.*/
            let stats = |capacity, time, modes: EdgeUpdateModes| IncreaseStats {
                st,
                cargo: CargoType(cargo),
                next_station_id: next_station,
                capacity,
                usage: 0,
                time,
                modes: modes.with(restricted_mode),
            };
            if self.is_full_loading
                && !orders(v).is_null()
                && StationID((s.links.graphs.station_id)(st)) == v.last_station_visited()
                && crate::orders::order_list(v, o)
                    .unwrap()
                    .get_total_duration()
                    > v.current_order_time()
            {
                let effective_capacity =
                    cargo_quantity.wrapping_mul(u32::from((s.loading.load_unload_ticks)(v)));
                if effective_capacity
                    > crate::orders::order_list(v, o)
                        .unwrap()
                        .get_total_duration()
                        .cast_unsigned()
                {
                    witness::hit(witness::VISITS_FULL_LOAD_INCREASE);
                    increase_stats(
                        s,
                        stats(
                            effective_capacity
                                / crate::orders::order_list(v, o)
                                    .unwrap()
                                    .get_total_duration()
                                    .cast_unsigned(),
                            0,
                            EdgeUpdateModes::INCREASE,
                        ),
                    );
                } else if s.shared.random_range(
                    crate::orders::order_list(v, o)
                        .unwrap()
                        .get_total_duration()
                        .cast_unsigned(),
                ) < effective_capacity
                {
                    witness::hit(witness::VISITS_FULL_LOAD_RANDOM);
                    increase_stats(s, stats(1, 0, EdgeUpdateModes::INCREASE));
                } else {
                    witness::hit(witness::VISITS_FULL_LOAD_REFRESH);
                    increase_stats(
                        s,
                        stats(cargo_quantity, time_estimate, EdgeUpdateModes::REFRESH),
                    );
                }
            } else {
                increase_stats(
                    s,
                    stats(cargo_quantity, time_estimate, EdgeUpdateModes::REFRESH),
                );
            }
        }
    }

    /// `LinkRefresher::RefreshLinks`: iterate over orders starting at `cur` and `next` and
    /// refresh links associated with them. `cur` and `next` can be equal. If they're not they
    /// must be "neighbours" in their order list, which means `next` must be directly
    /// reachable from `cur` without passing any further `OT_GOTO_STATION` or `OT_IMPLICIT`
    /// orders in between.
    fn refresh_links(
        &mut self,
        mut cur: VehicleOrderID,
        mut next: VehicleOrderID,
        mut flags: RefreshFlags,
        num_hops: u32,
        seen_hops: &mut HopSet,
        s: &Services,
    ) {
        let o = s.orders();
        assert_with(|| !orders(self.vehicle).is_null());
        let orderlist = orders(self.vehicle);
        let span = crate::orders::OrderListRef::of(orderlist, o);
        while next < span.get_num_orders() {
            let next_order = span.order(usize::from(next.0)).view();

            if (next_order.is_type(OrderType::OT_GOTO_DEPOT)
                || next_order.is_type(OrderType::OT_GOTO_STATION))
                && next_order.is_refit()
            {
                flags = flags.set(RefreshFlag::WasRefit);
                if !next_order.is_auto_refit() {
                    self.handle_refit(next_order.refit_cargo(), s);
                } else if !flags.test(RefreshFlag::InAutorefit) {
                    flags = flags.set(RefreshFlag::InAutorefit);
                    witness::hit(witness::VISITS_AUTOREFIT);
                    let backup = self.clone();
                    for cargo in 0..CargoType::NUM_CARGO.0 {
                        if (s.station.cargo_valid)(cargo) && self.handle_refit(CargoType(cargo), s)
                        {
                            self.refresh_links(cur, next, flags, num_hops, seen_hops, s);
                            self.restore(&backup);
                        }
                    }
                }
            }

            /* Only reset the refit capacities if the "previous" next is a station,
             * meaning that either the vehicle was refit at the previous station or
             * it wasn't at all refit during the current hop. */
            flags = if flags.test(RefreshFlag::WasRefit)
                && (next_order.is_type(OrderType::OT_GOTO_STATION)
                    || next_order.is_type(OrderType::OT_IMPLICIT))
            {
                flags.set(RefreshFlag::ResetRefit)
            } else {
                flags.reset(RefreshFlag::ResetRefit)
            };

            next = self.predict_next_order(cur, next, flags, num_hops, seen_hops, s);
            if next == VehicleOrderID::INVALID_VEH_ORDER_ID {
                break;
            }
            let hop = Hop {
                from: cur,
                to: next,
                cargo: self.cargo,
            };
            if !seen_hops.insert(hop) {
                witness::hit(witness::VISITS_SEEN_HOP);
                break;
            }

            let next_order = span.order(usize::from(next.0)).view();

            /* Don't use the same order again, but choose a new one in the next round. */
            flags = flags.reset(RefreshFlag::UseNext);

            /* Skip resetting and link refreshing if next order won't do anything with cargo. */
            if !next_order.is_type(OrderType::OT_GOTO_STATION)
                && !next_order.is_type(OrderType::OT_IMPLICIT)
            {
                continue;
            }

            if flags.test(RefreshFlag::ResetRefit) {
                self.reset_refit();
                flags = flags
                    .reset(RefreshFlags::from(RefreshFlag::ResetRefit).set(RefreshFlag::WasRefit));
            }

            let cur_order = span.order(usize::from(cur.0)).view();

            if cur_order.is_type(OrderType::OT_GOTO_STATION)
                || cur_order.is_type(OrderType::OT_IMPLICIT)
            {
                if cur_order.can_leave_with_cargo(flags.test(RefreshFlag::HasCargo)) {
                    flags = flags.set(RefreshFlag::HasCargo);
                    self.refresh_stats(cur, next, s);
                    witness::hit(witness::VISITS_STATS);
                } else {
                    flags = flags.reset(RefreshFlag::HasCargo);
                }
            }

            /* "cur" is only assigned here if the stop is a station so that
             * whenever stats are to be increased two stations can be found. */
            cur = next;
        }
    }
}

/// `LinkRefresher::Run`: refresh all links the given vehicle will visit.
pub(crate) fn refresh_links(v: VehicleRef, allow_merge: bool, is_full_loading: bool, s: &Services) {
    let o = s.orders();
    /* If there are no orders we can't predict anything.*/
    if orders(v).is_null() {
        return;
    }

    /* Make sure the first order is a useful order. */
    let first = crate::orders::OrderListRef::of(orders(v), o)
        .get_next_decision_node(v.cur_implicit_order_index(), 0);
    if first == VehicleOrderID::INVALID_VEH_ORDER_ID {
        return;
    }

    let mut seen_hops = HopSet::new();
    let mut refresher = Refresher::new(v, allow_merge, is_full_loading, s);

    let flags = if v.last_loading_station(s) == StationID::INVALID {
        RefreshFlags::default()
    } else {
        RefreshFlags::from(RefreshFlag::HasCargo)
    };
    witness::hit(witness::VISITS_REFRESH);
    refresher.refresh_links(first, first, flags, 0, &mut seen_hops, s);
}

/// `Vehicle::ResetRefitCaps`: reset all `refit_cap` in the consist to `cargo_cap`.
pub(crate) fn reset_refit_caps(front: VehicleRef) {
    let mut part = Some(front);
    while let Some(v) = part {
        v.set_refit_cap(v.cargo_cap());
        part = v.next();
    }
}

/// `Vehicle::CancelReservation`: return all reserved cargo packets to the station and reset
/// all packets staged for transfer.
#[allow(unsafe_code)]
pub(crate) fn cancel_reservation(
    front: VehicleRef,
    next: StationID,
    st: *mut Station,
    s: &Services,
) {
    let r = s.resolver();
    let mut part = Some(front);
    while let Some(v) = part {
        let cargo = cargo_list(v, r);
        // SAFETY: The live vehicle's list; each call ends before the next.
        if unsafe { crate::cargo_storage::action_count(cargo, MoveToAction::MTA_LOAD) } > 0 {
            witness::hit(witness::VISITS_RESERVATION);
            debug!(misc, 1, "cancelling cargo reservation");
            let dest = (s.station_cargo)(st, v.cargo_type(r).0);
            // SAFETY: The vehicle's and the station's live lists and the static cargo table.
            unsafe {
                crate::cargo_storage::vehicle_return(
                    cargo,
                    dest,
                    *s.refit.cargo,
                    u32::MAX,
                    next,
                    v.tile(),
                );
            }
        }
        // SAFETY: As above.
        unsafe { crate::cargo_storage::keep_all(cargo) };
        part = v.next();
    }
}

/// `CalcPercentVehicleFilled`: calculates how full a vehicle is. When `colour` is wanted it
/// receives the string to show depending on if we are unloading or loading. Percentages are
/// rounded towards 50%, so that 0% and 100% are only returned if the vehicle is completely
/// empty or full. This is useful for both display and conditional orders.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "CalcPercentVehicleFilled returns its `uint` and `int` quotients as `uint8_t`"
)]
pub(crate) fn percent_filled(front: VehicleRef, colour: Option<&mut StringID>, s: &Services) -> u8 {
    let r = s.resolver();
    let mut count = 0_i32;
    let mut max = 0_i32;
    let mut cars = 0_i32;
    let mut unloading = 0_i32;
    let mut loading = false;

    let is_loading = front.current_order().is_type(OrderType::OT_LOADING);

    /* The station may be nullptr when the (colour) string does not need to be set. */
    let st = s.station_if_valid(front.last_station_visited());
    assert_with(|| colour.is_none() || (!st.is_null() && is_loading));

    let order_no_load = is_loading && front.current_order().load_type() == OrderLoadType::NoLoad;
    let order_full_load = is_loading && front.current_order().is_full_load();

    // The station's goods owner, resolved once when a rating is first read.
    let mut service = None;
    /* Count up max and used */
    let mut part = Some(front);
    while let Some(v) = part {
        count = count.wrapping_add(stored_count(v, r).cast_signed());
        max = max.wrapping_add(i32::from(v.cargo_cap()));
        if v.cargo_cap() != 0 && colour.is_some() {
            unloading = unloading.wrapping_add(i32::from(
                v.vehicle_flags().test(VehicleFlag::CARGO_UNLOADING),
            ));
            loading |= !order_no_load
                && (order_full_load
                    || has_rating(
                        *service.get_or_insert_with(|| (s.station.owner)(st)),
                        v.cargo_type(r),
                    ))
                && !front.vehicle_flags().test(VehicleFlag::LOADING_FINISHED)
                && !front.vehicle_flags().test(VehicleFlag::STOP_LOADING);
            cars = cars.wrapping_add(1);
        }
        part = v.next();
    }

    if let Some(colour) = colour {
        *colour = if unloading == 0 && loading {
            STR_PERCENT_UP
        } else if unloading == 0 && !loading {
            STR_PERCENT_NONE
        } else if cars == unloading || !loading {
            STR_PERCENT_DOWN
        } else {
            STR_PERCENT_UP_DOWN
        };
    }

    /* Train without capacity */
    if max == 0 {
        return 100;
    }

    /* Return the percentage */
    if count.wrapping_mul(2) < max {
        /* Less than 50%; round up, so that 0% means really empty. */
        crate::math::ceil_div(count.wrapping_mul(100).cast_unsigned(), max.cast_unsigned()) as u8
    } else {
        /* More than 50%; round down, so that 100% means really full. */
        (count.wrapping_mul(100) / max) as u8
    }
}

/// # Safety
/// `front` is a live front vehicle; `colour` is null or writable; `s` is the static table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_percent_filled(
    front: VehicleRef,
    colour: *mut u32,
    s: *const Services,
) -> u8 {
    // SAFETY: The caller's contract; `StringID` is a transparent `u32`.
    percent_filled(
        front,
        unsafe { colour.cast::<StringID>().as_mut() },
        unsafe { &*s },
    )
}

/// `Vehicle::BeginLoading`: prepare everything to begin the loading when arriving at a
/// station.
#[allow(
    clippy::cast_possible_truncation,
    reason = "BeginLoading's TimerGameTick::Ticks travel time passed as IncreaseStats' uint32_t"
)]
pub(crate) fn begin_loading(v: VehicleRef, s: &Services) {
    assert_with(|| {
        crate::map_func::World::new(s.shared).is_tile_type(v.tile(), TileType::MP_STATION)
            || v.vehicle_type() == VehicleType::VEH_SHIP
    });
    witness::hit(witness::VISITS_BEGIN_LOADING);

    let travel_time = (s.shared.tick_counter)().wrapping_sub(v.last_loading_tick(s));
    let o = s.orders();
    crate::orders::begin_loading(v, o);

    if v.last_loading_station(s) != StationID::INVALID
        && v.last_loading_station(s) != v.last_station_visited()
        && (v.current_order().load_type() != OrderLoadType::NoLoad
            || v.current_order().unload_type() != OrderUnloadType::NoUnload)
    {
        increase_vehicle_stats(
            s.station_base(v.last_loading_station(s)),
            v,
            v.last_station_visited(),
            travel_time as u32,
            s,
        );
    }

    prepare_unload(v, s.station, s.loading);

    let index = WindowNumber::from(v.index(s.resolver()));
    set_window_dirty(
        get_window_class_for_vehicle_type(v.vehicle_type()),
        WindowNumber::from(v.owner()),
    );
    set_window_widget_dirty(
        WindowClass::WC_VEHICLE_VIEW,
        index,
        VehicleViewWidgets::WID_VV_START_STOP.0,
    );
    set_window_dirty(WindowClass::WC_VEHICLE_DETAILS, index);
    set_window_dirty(
        WindowClass::WC_STATION_VIEW,
        WindowNumber::from(v.last_station_visited()),
    );

    (s.station.mark_tiles)(s.station_base(v.last_station_visited()), true);
    v.set_cur_speed(0);
    (s.maintenance().mark_dirty)(v);
}

/// `Vehicle::LeaveStation`: perform all actions when leaving a station.
#[allow(
    unsafe_code,
    clippy::cast_possible_wrap,
    reason = "`trip_occupancy = CalcPercentVehicleFilled(..)` converts uint8_t to int8_t"
)]
pub(crate) fn leave_station(v: VehicleRef, s: &Services) {
    assert_with(|| v.current_order().is_type(OrderType::OT_LOADING));

    let l = s.loading;
    // `delete this->cargo_payment`.
    (l.delete_payment)((l.get_payment)(v));
    assert_with(|| (l.get_payment)(v).is_null()); // cleared by ~CargoPayment

    let o = s.orders();
    /* Only update the timetable if the vehicle was supposed to stop here. */
    if v.current_order().non_stop().any() {
        crate::orders::update_vehicle_timetable(v, false, o);
    }

    if v.current_order().load_type() != OrderLoadType::NoLoad
        || v.current_order().unload_type() != OrderUnloadType::NoUnload
    {
        if v.current_order()
            .can_leave_with_cargo(v.last_loading_station(s) != StationID::INVALID)
        {
            witness::hit(witness::VISITS_LEAVE_LOADED);
            /* Refresh next hop stats to make sure we've done that at least once
             * during the stop and that refit_cap == cargo_cap for each vehicle in
             * the consist. */
            reset_refit_caps(v);
            refresh_links(v, true, false, s);

            /* if the vehicle could load here or could stop with cargo loaded set the last loading station */
            v.set_last_loading_station(s, v.last_station_visited());
            v.set_last_loading_tick(s, (s.shared.tick_counter)());
        } else {
            /* if the vehicle couldn't load and had to unload or transfer everything
             * set the last loading station to invalid as it will leave empty. */
            witness::hit(witness::VISITS_LEAVE_EMPTY);
            v.set_last_loading_station(s, StationID::INVALID);
        }
    }

    crate::orders::make_leave_station(v);
    let st = (s.station.station)(v.last_station_visited().0);
    cancel_reservation(v, StationID::INVALID, st.native, s);
    // SAFETY: The live station's queue owner; the vehicle shell is its key.
    unsafe { crate::station_service::loading_vehicles_remove(st.state, v.shell().cast()) };

    (s.hide_filling_percent)(v);
    v.set_trip_occupancy(s, percent_filled(v, None, s) as i8);

    let world = crate::map_func::World::new(s.shared);
    if v.vehicle_type() == VehicleType::VEH_TRAIN && !v.vehstatus().test(VehState::Crashed) {
        /* Trigger station animation (trains only) */
        if world.is_tile_type(v.tile(), TileType::MP_STATION) {
            crate::animated_tile::rail_event(
                st.base,
                v.tile().0,
                crate::station_type::SAT_VEHICLE_DEPARTS,
                crate::station_type::SRT_VEHICLE_DEPARTS,
            );
        }

        let train = crate::train::Handle::from_vehicle(v);
        train.set_flags(
            train
                .flags()
                .set(crate::train::VehicleRailFlag::LeavingStation),
        );
    }
    if v.vehicle_type() == VehicleType::VEH_ROAD && !v.vehstatus().test(VehState::Crashed) {
        /* Trigger road stop animation */
        if world.is_station_road_stop_tile(v.tile()) {
            crate::animated_tile::road_event(
                st.base,
                v.tile().0,
                crate::station_type::SAT_VEHICLE_DEPARTS,
                crate::station_type::SRT_VEHICLE_DEPARTS,
            );
        }
    }

    (s.maintenance().mark_dirty)(v);
}

/// `Vehicle::HandleLoading`: handle the loading of the vehicle; when not it skips through
/// dummy orders and does nothing in all other cases. `mode` is whether this is the non-first
/// call for this vehicle in this tick. Its head, the dispatch on the order type, is inlined
/// into `VehicleServices::handle_loading` (`vehicle.rs`), so this body stays out of line.
#[inline(never)]
pub(crate) fn handle_loading(v: VehicleRef, mode: bool, s: &Services) {
    let o = s.orders();
    match v.current_order().kind() {
        OrderType::OT_LOADING => {
            let wait_time = i32::from(v.current_order().timetabled_wait())
                .wrapping_sub(v.lateness_counter())
                .max(0);

            /* Not the first call for this tick, or still loading */
            if mode
                || !v.vehicle_flags().test(VehicleFlag::LOADING_FINISHED)
                || v.current_order_time() < wait_time
            {
                return;
            }

            witness::hit(witness::VISITS_LOADING_DEPART);
            (s.vehicles().play_leave_station_sound)(v);

            leave_station(v, s);

            /* Only advance to next order if we just loaded at the current one */
            let order = crate::orders::get_order(v, v.cur_implicit_order_index(), o)
                .map(crate::orders::OrderRef::view);
            if order.is_none_or(|order| {
                (!order.is_type(OrderType::OT_IMPLICIT)
                    && !order.is_type(OrderType::OT_GOTO_STATION))
                    || order.destination().to_station_id() != v.last_station_visited()
            }) {
                return;
            }
        }

        OrderType::OT_DUMMY => witness::hit(witness::VISITS_DUMMY),

        _ => return,
    }

    crate::orders::increment_implicit_order_index(v, o);
}

/// `_vehicles_to_autoreplace[index] = value`.
#[allow(unsafe_code)]
fn set_vehicle_to_autoreplace(index: VehicleID, value: bool) {
    // SAFETY: The serial game thread's pending map; no access survives the call.
    unsafe { crate::fleet::pending::add(index.0, value) };
}

/// `VehicleEnteredDepotThisTick`: adds a vehicle to the list of vehicles that visited a depot
/// this tick.
fn entered_depot_this_tick(v: VehicleRef, index: VehicleID) {
    /* Vehicle should stop in the depot if it was in 'stopping' state */
    set_vehicle_to_autoreplace(index, !v.vehstatus().test(VehState::Stopped));

    /* We ALWAYS set the stopped state. Even when the vehicle does not plan on
     * stopping in the depot, so we stop it to ensure that it will not reserve
     * the path out of the depot before we might autoreplace it to a different
     * engine. The new engine would not own the reserved path we store that we
     * stopped the vehicle, so autoreplace can start it again */
    v.set_vehstatus(v.vehstatus().set(VehState::Stopped));
}

/// `VehicleEnterDepot`: vehicle entirely entered the depot, update its status, orders,
/// vehicle windows, service it, etc.
#[allow(unsafe_code)]
pub(crate) fn enter_depot(v: VehicleRef, s: &Services) {
    /* Always work with the front of the vehicle */
    // `v->First()` returns the stored link, which may be null; only the assertion reads it.
    assert_with(|| v.first_link() == Some(v));
    witness::hit(witness::VISITS_DEPOT);

    let vs = s.vehicles();
    let index = v.index(s.resolver());
    let view = WindowNumber::from(index);
    match v.vehicle_type() {
        VehicleType::VEH_TRAIN => {
            let t = crate::train::Handle::from_vehicle(v);
            set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);
            /* Clear path reservation */
            let pbs = crate::pbs::Pbs::acquire(s.shared);
            (pbs.0.set_depot)(t.vehicle().tile().0, false);
            if (pbs.0.show_reservation)() {
                (s.shared.mark_dirty)(t.vehicle().tile().0);
            }

            crate::signal::context(s.shared).update_signals_on_segment(
                t.vehicle().tile(),
                crate::direction_type::DiagDirection::INVALID_DIAGDIR,
                t.vehicle().owner(),
            );
            t.set_wait_counter(0);
            t.set_force_proceed(crate::train::TrainForceProceeding::TFP_NONE);
            t.set_flags(t.flags().reset(crate::train::VehicleRailFlag::Reversed));
            crate::train::consist_changed(s.refit.train, s.shared, t, crate::train::CCF_ARRANGE);
        }

        VehicleType::VEH_ROAD => set_window_classes_dirty(WindowClass::WC_ROADVEH_LIST),

        VehicleType::VEH_SHIP => {
            set_window_classes_dirty(WindowClass::WC_SHIPS_LIST);
            // `Ship::From(v)`, whose owner the record's `ship` link names.
            let (ship, leaves) = (Ship::from_vehicle(v), s.refit.ship);
            ship.set_state(TrackBits::TRACK_BIT_DEPOT);
            update_cache(&Tables::new(leaves, s.shared), ship);
            (leaves.update_viewport)(v, true, true);
            set_window_dirty(WindowClass::WC_VEHICLE_DEPOT, WindowNumber::from(v.tile()));
        }

        VehicleType::VEH_AIRCRAFT => {
            set_window_classes_dirty(WindowClass::WC_AIRCRAFT_LIST);
            let aircraft = (s.refit.aircraft_handle)(v.0.as_ptr());
            // SAFETY: The live aircraft and its static controller table.
            unsafe { crate::aircraft::handle_aircraft_enter_hangar(s.refit.aircraft, aircraft) };
        }
        _ => not_reached(),
    }
    set_window_dirty(WindowClass::WC_VEHICLE_VIEW, view);

    if v.vehicle_type() != VehicleType::VEH_TRAIN {
        /* Trains update the vehicle list when the first unit enters the depot and calls VehicleEnterDepot() when the last unit enters.
         * We only increase the number of vehicles when the first one enters, so we will not need to search for more vehicles in the depot */
        invalidate_window_data(
            WindowClass::WC_VEHICLE_DEPOT,
            WindowNumber::from(v.tile()),
            0,
            false,
        );
    }
    set_window_dirty(WindowClass::WC_VEHICLE_DEPOT, WindowNumber::from(v.tile()));

    v.set_vehstatus(v.vehstatus().set(VehState::Hidden));
    v.set_cur_speed(0);

    super::maintenance::vehicle_service_in_depot(v, vs);

    /* Store that the vehicle entered a depot this tick */
    entered_depot_this_tick(v, index);

    /* After a vehicle trigger, the graphics and properties of the vehicle could change. */
    // SAFETY: The live vehicle and the static vehicle table.
    unsafe { crate::newgrf_vehicle::trigger(vs, v, crate::newgrf_vehicle::Trigger::Depot) };
    (s.maintenance().mark_dirty)(v);

    invalidate_window_data(WindowClass::WC_VEHICLE_VIEW, view, 0, false);

    if v.current_order().is_type(OrderType::OT_GOTO_DEPOT) {
        set_window_dirty(WindowClass::WC_VEHICLE_VIEW, view);

        let o = s.orders();
        let real_order = crate::orders::get_order(v, v.cur_real_order_index(), o)
            .map(crate::orders::OrderRef::view);

        /* Test whether we are heading for this depot. If not, do nothing.
         * Note: The target depot for nearest-/manual-depot-orders is only updated on junctions, but we want to accept every depot. */
        if v.current_order()
            .depot_type()
            .test(OrderDepotTypeFlag::PartOfOrders)
            && real_order.is_some_and(|real_order| {
                !real_order
                    .depot_action_flags()
                    .test(OrderDepotActionFlag::NearestDepot)
            })
            && if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                v.current_order().destination().to_station_id()
                    != crate::map_func::World::new(s.shared).get_station_index(v.tile())
            } else {
                v.dest_tile() != v.tile()
            }
        {
            /* We are heading for another depot, keep driving. */
            witness::hit(witness::VISITS_DEPOT_ELSEWHERE);
            return;
        }

        if v.current_order().is_refit() {
            depot_refit(v, index, s);
        }

        if v.current_order()
            .depot_type()
            .test(OrderDepotTypeFlag::PartOfOrders)
        {
            /* Part of orders */
            crate::orders::delete_unreached_implicit_orders(v, o);
            crate::orders::update_vehicle_timetable(v, true, o);
            crate::orders::increment_implicit_order_index(v, o);
        }
        if v.current_order()
            .depot_action_flags()
            .test(OrderDepotActionFlag::Halt)
        {
            depot_halt(v, index, s);
        }

        /* If we've entered our unbunching depot, record the round trip duration. */
        crate::orders::measure_unbunching(v, o);

        crate::orders::make_dummy(v);
    }
}

/// The current order's halt in `VehicleEnterDepot` (`vehicle.cpp:1623-1639`).
fn depot_halt(v: VehicleRef, index: VehicleID, s: &Services) {
    witness::hit(witness::VISITS_DEPOT_HALT);
    /* Vehicles are always stopped on entering depots. Do not restart this one. */
    set_vehicle_to_autoreplace(index, false);
    /* Invalidate last_loading_station. As the link from the station
     * before the stop to the station after the stop can't be predicted
     * we shouldn't construct it when the vehicle visits the next stop. */
    v.set_last_loading_station(s, StationID::INVALID);

    /* Clear unbunching data. */
    crate::orders::reset_depot_unbunching(v);

    /* Announce that the vehicle is waiting to players and AIs. */
    if is_local_company(v, s) {
        add_vehicle_advice_news_item(
            AdviceType::VehicleWaiting,
            &get_encoded_string(
                StringID(STR_NEWS_TRAIN_IS_WAITING.0 + u32::from(v.vehicle_type().0)),
                &[Integer(u64::from(index.0))],
            ),
            index,
        );
    }
    crate::ai::new_event(v.owner(), &ScriptEvent::vehicle_waiting_in_depot(index));
}

/// The current order's refit in `VehicleEnterDepot` (`vehicle.cpp:1598-1615`), under the
/// owner's company.
#[allow(unsafe_code)]
fn depot_refit(v: VehicleRef, index: VehicleID, s: &Services) {
    let l = s.loading;
    let r = s.refit;
    let base = r.common.base;
    let cur_company = (l.current_company)();
    (l.set_current_company)(v.owner().0);
    // SAFETY: Both native results live on this stack for the synchronous command.
    unsafe {
        crate::command::with_cost(base.cost, |cost| {
            crate::command::with_cost(base.cost, |temporary| {
                let refit_cargo = v.current_order().refit_cargo();
                // `Command<CMD_REFIT_VEHICLE>::Do(DoCommandFlag::Execute, v->index, v->current_order.GetRefitCargo(), 0xFF, false, false, 0)`.
                crate::command::nested(
                    base.commands,
                    base.shared,
                    cost,
                    0,
                    crate::command_type::DoCommandFlag::EXECUTE,
                    // SAFETY: The static refit table and the scope's live native costs.
                    |flags| {
                        refit::refit(
                            r,
                            cost.0,
                            temporary.0,
                            flags,
                            index.0,
                            refit_cargo.0,
                            0xff,
                            false,
                            false,
                            0,
                        )
                    },
                );
            });
            (l.set_current_company)(cur_company);

            depot_refit_result(v, index, cost, s);
        });
    }
}

/// The outcome of the depot refit command in `VehicleEnterDepot` (`vehicle.cpp:1603-1614`),
/// read from its native result `cost`.
fn depot_refit_result(
    v: VehicleRef,
    index: VehicleID,
    cost: crate::services::Cost<'_>,
    s: &Services,
) {
    if cost.ok() {
        witness::hit(witness::VISITS_DEPOT_REFIT);
        let cost = cost.money();
        if cost != 0 {
            // `Money` subtraction saturates.
            let vs = s.vehicles();
            v.set_profit_this_year(
                vs,
                v.profit_this_year(vs).saturating_sub(cost.wrapping_shl(8)),
            );
            if is_local_company(v, s) {
                (s.shared.cost_animation)(v.x_pos(), v.y_pos(), v.z_pos(), cost);
            }
        }
    } else {
        witness::hit(witness::VISITS_DEPOT_REFIT_FAILED);
        set_vehicle_to_autoreplace(index, false);
        if is_local_company(v, s) {
            /* Notify the user that we stopped the vehicle */
            add_vehicle_advice_news_item(
                AdviceType::RefitFailed,
                &get_encoded_string(STR_NEWS_ORDER_REFIT_FAILED, &[Integer(u64::from(index.0))]),
                index,
            );
        }
    }
}

/// # Safety
/// `v` is a live front vehicle at a station; `s` is the static production table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_visit_leave_station(v: VehicleRef, s: *const Services) {
    // SAFETY: The caller's contract.
    leave_station(v, unsafe { &*s });
}

/// # Safety
/// `v` is a live front vehicle in a depot; `s` is the static production table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_visit_enter_depot(v: VehicleRef, s: *const Services) {
    // SAFETY: The caller's contract.
    enter_depot(v, unsafe { &*s });
}

/// # Safety
/// `v` is a live front vehicle; `st` is a live station; `s` is the static production table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_visit_cancel_reservation(
    v: VehicleRef,
    next: u16,
    st: *mut Station,
    s: *const Services,
) {
    // SAFETY: The caller's contract.
    cancel_reservation(v, StationID(next), st, unsafe { &*s });
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleVisits" ["probe"] => Services {
        hide_filling_percent,
        last_loading_station,
        last_loading_tick,
        links,
        loading,
        refit,
        schedule,
        set_last_loading_station,
        set_last_loading_tick,
        set_trip_occupancy,
        shared,
        station,
        station_cargo,
    };
}

#[allow(unsafe_code)]
#[cfg(test)]
mod tests;
