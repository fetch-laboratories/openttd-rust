/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The road pathfinding (`pathfinder/yapf/yapf_road.cpp`), over YAPF's main
//! loop (`yapf_base.hpp`), tile origin (`yapf_common.hpp`) and node list
//! (`nodelist.hpp`). The road vehicle controller keeps the only persistent path.
//! Lookup maps are never iterated; arena identity survives open-node replacement.
//! Heap ties follow `CBinaryHeapT`, including removal's upward then downward repair.
//!
//! Each entry is a synchronous call under the game-state mutex. A search loads
//! the map descriptor once, at entry: no service it calls loads, resets or
//! yields the map, or reenters the search or the road vehicle owner.
#![allow(unsafe_code)]
use crate::binaryheap::BinaryHeap;
use crate::bitmath_func::{find_first_bit, kill_first_bit_u16};
use crate::debug::debug;
use crate::direction_func::reverse_diag_dir;
use crate::direction_type::DiagDirection;
use crate::follow_track::{CFollowTrackT, RoadContext, get_speed_limit_with_road};
use crate::game_state::GameState;
use crate::map_func::World;
use crate::node_hash::NodeMap;
use crate::order_type::OrderType;
use crate::pathfinder_func::{calc_closest_station_tile, get_trackdir_bits_for_road};
use crate::pathfinder_type::{
    FindDepotData, YAPF_ROADVEH_PATH_CACHE_DESTINATION_LIMIT, YAPF_ROADVEH_PATH_CACHE_SEGMENTS,
    YAPF_TILE_CORNER_LENGTH, YAPF_TILE_LENGTH,
};
use crate::rail_type::RailTypes;
use crate::road::{PathElement, State};
use crate::road_type::{RoadTramType, RoadTypes};
use crate::services::Services;
use crate::station_map::{get_station_type, is_road_waypoint};
use crate::station_type::{StationID, StationType};
use crate::tile_type::{TILE_SIZE, TileIndex, TileType};
use crate::tilearea::TileArea;
use crate::track_func::{
    diag_dir_to_diag_trackdir, diagdir_reaches_trackdirs, has_trackdir, is_diagonal_trackdir,
    track_status_to_trackdir_bits, trackdir_to_exitdir, trackdir_to_trackdir_bits,
};
use crate::track_type::{Trackdir, TrackdirBits};
use crate::transport_type::TransportType;
use crate::vehicle::{OpenTTDVehicle, VehicleRef};
use crate::yapf_common::octile_distance_cost;
use crate::yapf_node::CYapfNodeKeyExitDir;
use std::cell::OnceCell;

/// What the search reads from the vehicle and `_settings_game.pf.yapf`, filled
/// by [`road_input`] or by a native test.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Input {
    pub tile: u32,
    pub dest_tile: u32,
    pub max_nodes: i32,
    pub slope: i32,
    pub crossing: i32,
    pub stop: i32,
    pub occupied: i32,
    pub bay: i32,
    pub curve: i32,
    pub order_destination: u16,
    pub order_type: u8,
    pub bus: u8,
    pub articulated: u8,
    pub trackdir: u8,
}
/// A search's chosen trackdir or depot, with the statistics `FindPath` reports.
#[repr(C)]
pub struct Result {
    pub tile: u32,
    pub cost: i32,
    pub direction: u8,
    pub found: u8,
    pub rounds: i32,
    pub open: i32,
    pub closed: i32,
    pub calcs: i32,
    pub distance: i32,
}

/// `CYapfRoadNode`: yapf node for road vehicles.
#[derive(Clone, Copy)]
struct CYapfRoadNode {
    key: CYapfNodeKeyExitDir,
    parent: Option<usize>,
    cost: i32,
    estimate: i32,
    is_choice: bool,
    segment_last_tile: TileIndex,
    segment_last_td: Trackdir,
    /// `PfDetectDestinationTile` at the segment end, where `PfCalcCost` has
    /// already observed it. The map cannot change during the search, so the
    /// memo equals a repeated observation.
    destination: Option<bool>,
}
impl CYapfRoadNode {
    /// `CYapfRoadNode::Set`.
    #[inline]
    fn set(parent: Option<usize>, tile: TileIndex, td: Trackdir, is_choice: bool) -> Self {
        Self {
            key: CYapfNodeKeyExitDir::set(tile, td),
            parent,
            cost: 0,
            estimate: 0,
            is_choice,
            segment_last_tile: tile,
            segment_last_td: td,
            destination: None,
        }
    }
}

/// The node storage, open and closed lists and open-list heap that the
/// original's `Tpf pf` allocates on every search. The capacity is kept between
/// searches, because allocating it per search costs a measured 0.35 billion
/// instructions under the Opus play scenario (#278). Each search empties it on
/// entry, so nothing carries from one search to the next.
#[derive(Default)]
struct Scratch {
    items: Vec<CYapfRoadNode>,
    open_nodes: NodeMap<CYapfNodeKeyExitDir, usize>,
    closed_nodes: NodeMap<CYapfNodeKeyExitDir, usize>,
    open_queue: BinaryHeap,
}
// An active search owns the taken scratch; native heap probes never touch this slot.
static SCRATCH: GameState<Option<Scratch>> = GameState::new(None);

/// `CYapfRoad` and `CYapfRoadAnyDepot` in one type: `depot` stands for the
/// `CYapfRoadAnyDepot` instantiation, whose destination is any road depot.
struct CYapfRoad<'a> {
    s: &'a Services,
    world: World<'a>,
    vehicle: VehicleRef,
    settings: Input,
    depot: bool,
    // `CYapfCostRoadT`.
    max_cost: i32,
    // `CYapfDestinationTileRoadT`.
    dest_tile: TileIndex,
    dest_trackdirs: TrackdirBits,
    dest_station: StationID,
    station_type: StationType,
    non_artic: bool,
    // `CYapfBaseT`.
    nodes: Scratch,
    best_dest_node: Option<usize>,
    best_intermediate_node: Option<usize>,
    max_search_nodes: i32,
    stats_cost_calcs: i32,
    num_steps: i32,
    reuse_scratch: bool,
    // The vehicle's road kind, a follower initialised for it, its road context,
    // and its `(GetDisplayMaxSpeed(), current_order.GetMaxSpeed())`: values no
    // search callback can change, read once instead of once per tile step.
    tram: OnceCell<RoadTramType>,
    follower: OnceCell<CFollowTrackT>,
    road: OnceCell<RoadContext>,
    speeds: OnceCell<(i32, u16)>,
}
impl<'a> CYapfRoad<'a> {
    fn new(
        s: &'a Services,
        vehicle: VehicleRef,
        settings: Input,
        depot: bool,
        mut nodes: Scratch,
        reuse_scratch: bool,
    ) -> Self {
        nodes.items.clear();
        nodes.open_nodes.clear();
        nodes.closed_nodes.clear();
        nodes.open_queue = BinaryHeap::reuse(std::mem::take(&mut nodes.open_queue));
        Self {
            s,
            world: World::new(s),
            vehicle,
            settings,
            depot,
            max_cost: 0,
            dest_tile: TileIndex::INVALID,
            dest_trackdirs: TrackdirBits::TRACKDIR_BIT_NONE,
            dest_station: StationID::INVALID,
            station_type: StationType(0),
            non_artic: false,
            nodes,
            best_dest_node: None,
            best_intermediate_node: None,
            max_search_nodes: settings.max_nodes,
            stats_cost_calcs: 0,
            num_steps: 0,
            reuse_scratch,
            tram: OnceCell::new(),
            follower: OnceCell::new(),
            road: OnceCell::new(),
            speeds: OnceCell::new(),
        }
    }
    /// A search over the retained scratch. The caller holds the game-state
    /// mutex for the complete search; no global borrow survives this
    /// acquisition or any of the search's native queries.
    unsafe fn reused(s: &'a Services, vehicle: VehicleRef, settings: Input, depot: bool) -> Self {
        // SAFETY: Native production entries serialize access. Ownership moves
        // out of the slot before any query; first use has no allocation yet.
        let nodes = unsafe { (*SCRATCH.get()).take().unwrap_or_default() };
        Self::new(s, vehicle, settings, depot, nodes, true)
    }

    #[inline]
    fn check(&self, condition: bool) {
        crate::movement::native_assert(self.s, condition);
    }
    /// `GetRoadTramType(v->roadtype)`.
    #[inline]
    fn road_tram_type(&self) -> RoadTramType {
        *self
            .tram
            .get_or_init(|| (crate::movement::table(self.s).road_kind)(self.vehicle))
    }
    /// `TrackFollower F(Yapf().GetVehicle())`.
    #[inline]
    fn track_follower(&self) -> CFollowTrackT {
        *self.follower.get_or_init(|| {
            CFollowTrackT::vehicle(
                self.s,
                TransportType::TRANSPORT_ROAD,
                Some(self.vehicle),
                RailTypes::INVALID_RAILTYPES,
                true,
            )
        })
    }
    #[inline]
    fn road_context(&self) -> &RoadContext {
        self.road.get_or_init(|| RoadContext {
            tram: self.road_tram_type().0,
            compatible: RoadTypes((crate::movement::table(self.s).road_compatible)(
                self.vehicle,
            )),
        })
    }

    // `NodeList`.

    /// `NodeList::InsertOpenNode`: insert given item as open node (into
    /// `open_nodes` and `open_queue`).
    #[inline]
    fn insert_open_node(&mut self, item: usize) {
        let key = self.nodes.items[item].key;
        self.world
            .assert_with(|| !self.nodes.closed_nodes.contains_key(&key));
        self.nodes.open_nodes.insert(key, item);
        let items = &self.nodes.items;
        self.nodes
            .open_queue
            .include(item, |a, b| items[a].estimate < items[b].estimate);
    }
    /// `NodeList::PopOpenNode`: remove and return the open node specified by a key.
    #[inline]
    fn pop_open_node(&mut self, key: CYapfNodeKeyExitDir) -> usize {
        let item = self.nodes.open_nodes.remove(&key);
        self.check(item.is_some());
        let Some(item) = item else {
            crate::services_assert::not_reached()
        };
        let items = &self.nodes.items;
        self.nodes
            .open_queue
            .remove(item, |a, b| items[a].estimate < items[b].estimate);
        item
    }
    /// `NodeList::InsertClosedNode`: close node.
    #[inline]
    fn insert_closed_node(&mut self, item: usize) {
        let key = self.nodes.items[item].key;
        self.world
            .assert_with(|| !self.nodes.open_nodes.contains_key(&key));
        self.nodes.closed_nodes.insert(key, item);
    }
    /// `NodeList::OpenCount`.
    fn open_count(&self) -> i32 {
        self.nodes.open_nodes.len() as i32
    }
    /// `NodeList::ClosedCount`.
    fn closed_count(&self) -> i32 {
        self.nodes.closed_nodes.len() as i32
    }

    // `CYapfOriginTileT`.

    /// `SetOrigin`: set origin tile / trackdir mask.
    fn set_origin(&mut self, tile: TileIndex, trackdirs: TrackdirBits) {
        let is_choice = kill_first_bit_u16(trackdirs.0) != TrackdirBits::TRACKDIR_BIT_NONE.0;
        let mut tdb = trackdirs;
        while tdb != TrackdirBits::TRACKDIR_BIT_NONE {
            let td = Trackdir(find_first_bit(u64::from(tdb.0)));
            let node = CYapfRoadNode::set(None, tile, td, is_choice);
            self.add_startup_node(node);
            tdb = TrackdirBits(kill_first_bit_u16(tdb.0));
        }
    }

    // `CYapfBaseT`.

    /// `AddStartupNode`: add new node (created by `CreateNewNode` and filled
    /// with data) into open list.
    fn add_startup_node(&mut self, n: CYapfRoadNode) {
        self.check(n.parent.is_none());
        self.check(self.num_steps == 0);

        /* insert the new node only if it is not there */
        if !self.nodes.open_nodes.contains_key(&n.key) {
            let item = self.nodes.items.len();
            self.nodes.items.push(n);
            self.insert_open_node(item);
        }
    }
    /// `AddMultipleNodes`: add multiple nodes - direct children of the given node.
    #[inline]
    fn add_multiple_nodes(&mut self, parent: usize, tf: &CFollowTrackT) {
        let is_choice =
            kill_first_bit_u16(tf.new_td_bits.0) != TrackdirBits::TRACKDIR_BIT_NONE.0;
        let mut rtds = tf.new_td_bits;
        while rtds != TrackdirBits::TRACKDIR_BIT_NONE {
            let td = Trackdir(find_first_bit(u64::from(rtds.0)));
            let n = CYapfRoadNode::set(Some(parent), tf.new_tile, td, is_choice);
            self.add_new_node(n);
            rtds = TrackdirBits(kill_first_bit_u16(rtds.0));
        }
    }
    /// `AddNewNode`: called by `PfFollowNode` for each child node. Nodes are
    /// evaluated here and added into open list.
    fn add_new_node(&mut self, mut n: CYapfRoadNode) {
        self.check(n.parent.is_some());

        /* evaluate the node; the road search has no segment cost cache */
        self.stats_cost_calcs = self.stats_cost_calcs.wrapping_add(1);

        let mut valid = self.pf_calc_cost(&mut n);

        if valid {
            valid = self.pf_calc_estimate(&mut n);
        }

        /* have the cost or estimate callbacks marked this node as invalid? */
        if !valid {
            return;
        }

        /* The new node can be set as the best intermediate node only once we're
         * certain it will be finalized by being inserted into the open list. */
        let set_intermediate = self.max_search_nodes > 0
            && self.best_intermediate_node.is_none_or(|i| {
                let best = &self.nodes.items[i];
                best.estimate.wrapping_sub(best.cost) > n.estimate.wrapping_sub(n.cost)
            });

        /* check new node against open list */
        if let Some(&open_node) = self.nodes.open_nodes.get(&n.key) {
            /* another node exists with the same key in the open list
             * is it better than new one? */
            if n.estimate < self.nodes.items[open_node].estimate {
                /* update the old node by value from new one */
                self.pop_open_node(n.key);
                self.nodes.items[open_node] = n;
                /* add the updated old node back to open list */
                self.insert_open_node(open_node);
                if set_intermediate {
                    self.best_intermediate_node = Some(open_node);
                }
            }
            return;
        }

        /* check new node against closed list */
        if let Some(&closed_node) = self.nodes.closed_nodes.get(&n.key) {
            /* another node exists with the same key in the closed list
             * is it better than new one? */
            let node_est = n.estimate;
            let closed_est = self.nodes.items[closed_node].estimate;
            if node_est < closed_est {
                /* If this assert occurs, you have probably problem in
                 * your Tderived::PfCalcCost() or Tderived::PfCalcEstimate().
                 * The problem could be:
                 *  - PfCalcEstimate() gives too large numbers
                 *  - PfCalcCost() gives too small numbers
                 *  - You have used negative cost penalty in some cases (cost bonus) */
                crate::services_assert::not_reached();
            }
            return;
        }
        /* the new node is really new
         * add it to the open list */
        let item = self.nodes.items.len();
        self.nodes.items.push(n);
        self.insert_open_node(item);
        if set_intermediate {
            self.best_intermediate_node = Some(item);
        }
    }
    /// `FindPath`: main pathfinder routine:
    ///   - set startup node(s)
    ///   - main loop that stops if:
    ///      - the destination was found
    ///      - or the open list is empty (no route to destination).
    ///      - or the maximum amount of loops reached - `max_search_nodes` (default = 10000)
    ///
    /// Returns true if the path was found.
    fn find_path(&mut self) -> bool {
        loop {
            self.num_steps = self.num_steps.wrapping_add(1);
            let Some(best_open_node) = self.nodes.open_queue.first() else {
                break;
            };

            if self.pf_detect_destination(&self.nodes.items[best_open_node]) {
                self.best_dest_node = Some(best_open_node);
                break;
            }

            self.pf_follow_node(best_open_node);
            if self.max_search_nodes != 0 && self.closed_count() >= self.max_search_nodes {
                break;
            }

            self.pop_open_node(self.nodes.items[best_open_node].key);
            self.insert_closed_node(best_open_node);
        }

        let destination_found = self.best_dest_node.is_some();

        debug!(
            yapf,
            3,
            "[YAPF{}]{}{:4} - {} rounds - {} open - {} closed - CHR {:4.1}% - C {} D {}",
            // `TransportTypeChar`
            'r',
            if destination_found { '-' } else { '!' },
            self.vehicle.unit_number(self.s),
            self.num_steps,
            self.open_count(),
            self.closed_count(),
            // The road search has no segment cost cache, so no cache hits.
            0.0_f32,
            self.best_dest_node.map_or(-1, |i| self.nodes.items[i].cost),
            self.best_dest_node.map_or(-1, |i| {
                let n = &self.nodes.items[i];
                n.estimate.wrapping_sub(n.cost)
            })
        );

        destination_found
    }
    /// `GetBestNode`: if path was found return the best node that has reached
    /// the destination. Otherwise return the best visited node (which was
    /// nearest to the destination).
    #[inline]
    fn get_best_node(&self) -> Option<usize> {
        self.best_dest_node.or(self.best_intermediate_node)
    }

    // `CYapfCostRoadT`.

    /// `SlopeCost`.
    #[inline]
    fn slope_cost(&self, tile: TileIndex, next_tile: TileIndex) -> i32 {
        let movement = crate::movement::table(self.s);
        let map = self.world.dimensions;

        /* height of the center of the current tile */
        let x1 = (map.tile_x(tile) * TILE_SIZE).cast_signed();
        let y1 = (map.tile_y(tile) * TILE_SIZE).cast_signed();
        let z1 = movement.get_slope_pixel_z(
            x1 + (TILE_SIZE / 2).cast_signed(),
            y1 + (TILE_SIZE / 2).cast_signed(),
            true,
        );

        /* height of the center of the next tile */
        let x2 = (map.tile_x(next_tile) * TILE_SIZE).cast_signed();
        let y2 = (map.tile_y(next_tile) * TILE_SIZE).cast_signed();
        let z2 = movement.get_slope_pixel_z(
            x2 + (TILE_SIZE / 2).cast_signed(),
            y2 + (TILE_SIZE / 2).cast_signed(),
            true,
        );

        if z2 - z1 > 1 {
            /* Slope up */
            return self.settings.slope;
        }
        0
    }
    /// `OneTileCost`: return one tile cost.
    #[inline]
    fn one_tile_cost(&self, tile: TileIndex, trackdir: Trackdir) -> i32 {
        let mut cost: i32 = 0;
        /* set base cost */
        if is_diagonal_trackdir(trackdir.0) {
            cost = cost.wrapping_add(YAPF_TILE_LENGTH);
            match self.world.kind(tile) {
                TileType::MP_ROAD => {
                    /* Increase the cost for level crossings */
                    if self.world.level_crossing(tile.0) {
                        cost = cost.wrapping_add(self.settings.crossing);
                    }
                }

                TileType::MP_STATION => {
                    if self.world.asserted(|m| is_road_waypoint(m, tile)) {
                        return cost;
                    }

                    let rs = crate::road_stop::at(self.s, tile);
                    if self.world.is_drive_through_stop_tile(tile) {
                        /* Increase the cost for drive-through road stops */
                        cost = cost.wrapping_add(self.settings.stop);
                        let dir = DiagDirection(trackdir_to_exitdir(trackdir.0));
                        let offset = self
                            .world
                            .dimensions
                            .tile_offs_by_diag_dir(dir, |ok| self.check(ok));
                        if !crate::road_stop::is_drive_through_road_stop_continuation(
                            self.s,
                            tile,
                            TileIndex(tile.0.wrapping_sub(offset.cast_unsigned())),
                        ) {
                            /* When we're the first road stop in a 'queue' of them we increase
                             * cost based on the fill percentage of the whole queue. */
                            let entry = crate::road_stop::entry_read(rs, dir);
                            cost = cost.wrapping_add(
                                (u32::from(entry.occupied)
                                    .wrapping_mul(self.settings.occupied.cast_unsigned())
                                    / u32::from(entry.length))
                                .cast_signed(),
                            );
                        }
                    } else {
                        /* Increase cost for filled road stops */
                        let busy_bays = u32::from(!crate::road_stop::is_free_bay(rs, 0))
                            + u32::from(!crate::road_stop::is_free_bay(rs, 1));
                        cost = cost.wrapping_add(
                            (self.settings.bay.cast_unsigned().wrapping_mul(busy_bays) / 2)
                                .cast_signed(),
                        );
                    }
                }

                _ => {}
            }
        } else {
            /* non-diagonal trackdir */
            cost = YAPF_TILE_CORNER_LENGTH.wrapping_add(self.settings.curve);
        }
        cost
    }
    /// `PfCalcCost`: called by YAPF to calculate the cost from the origin to
    /// the given node. Calculates only the cost of given node, adds it to the
    /// parent node cost and stores the result into `Node::cost` member.
    #[inline]
    fn pf_calc_cost(&self, n: &mut CYapfRoadNode) -> bool {
        let mut segment_cost: i32 = 0;
        let mut tiles: u32 = 0;
        /* start at n.key.tile / n.key.td and walk to the end of segment */
        let mut tile = n.key.tile;
        let mut trackdir = n.key.td;
        let parent_cost = n.parent.map_or(0, |parent| self.nodes.items[parent].cost);

        loop {
            /* base tile cost depending on distance between edges */
            segment_cost = segment_cost.wrapping_add(self.one_tile_cost(tile, trackdir));

            /* we have reached the vehicle's destination - segment should end here to avoid target skipping */
            let destination = self.pf_detect_destination_tile(tile, trackdir);
            n.destination = Some(destination);
            if destination {
                break;
            }

            /* Finish if we already exceeded the maximum path cost (i.e. when
             * searching for the nearest depot). */
            if self.max_cost > 0 && parent_cost.wrapping_add(segment_cost) > self.max_cost {
                return false;
            }

            /* stop if we have just entered the depot */
            if self.world.road_depot(tile.0)
                && trackdir.0
                    == diag_dir_to_diag_trackdir(
                        reverse_diag_dir(DiagDirection(self.world.road_depot_dir(tile.0)), |ok| {
                            self.check(ok);
                        })
                        .0,
                        |ok| self.check(ok),
                    )
            {
                /* next time we will reverse and leave the depot */
                break;
            }

            /* if there are no reachable trackdirs on new tile, we have end of road */
            let mut f = self.track_follower();
            if !f.follow_with_road(self.s, tile, trackdir, Some(self.road_context())) {
                break;
            }

            /* if there are more trackdirs available & reachable, we are at the end of segment */
            if kill_first_bit_u16(f.new_td_bits.0) != TrackdirBits::TRACKDIR_BIT_NONE.0 {
                break;
            }

            let new_td = Trackdir(find_first_bit(u64::from(f.new_td_bits.0)));

            /* stop if RV is on simple loop with no junctions */
            if f.new_tile == n.key.tile && new_td == n.key.td {
                return false;
            }

            /* if we skipped some tunnel tiles, add their cost */
            segment_cost =
                segment_cost.wrapping_add(f.tiles_skipped.wrapping_mul(YAPF_TILE_LENGTH));
            tiles = tiles.wrapping_add(f.tiles_skipped.wrapping_add(1).cast_unsigned());

            /* add hilly terrain penalty */
            segment_cost = segment_cost.wrapping_add(self.slope_cost(tile, f.new_tile));

            /* add min/max speed penalties */
            let min_speed: i32 = 0;
            let (display_max_speed, order_max_speed) = *self.speeds.get_or_init(|| {
                let movement = crate::movement::table(self.s);
                // SAFETY: The movement table's orders leaves have process lifetime.
                let orders = unsafe { &*movement.orders };
                (
                    (movement.train().physics.display_max_speed)(self.vehicle),
                    crate::orders::VisitOrders::new(orders)
                        .current(self.vehicle)
                        .max_speed(),
                )
            });
            let max_veh_speed = display_max_speed.min(i32::from(order_max_speed) * 2);
            let max_speed = get_speed_limit_with_road(
                self.s,
                TransportType::TRANSPORT_ROAD,
                Some(self.vehicle),
                f.old_tile,
                Some(self.road_tram_type()),
            );
            if max_speed < max_veh_speed {
                segment_cost = segment_cost.wrapping_add(
                    YAPF_TILE_LENGTH
                        .wrapping_mul(max_veh_speed.wrapping_sub(max_speed))
                        .wrapping_mul(4_i32.wrapping_add(f.tiles_skipped))
                        / max_veh_speed,
                );
            }
            if min_speed > max_veh_speed {
                segment_cost = segment_cost
                    .wrapping_add(YAPF_TILE_LENGTH.wrapping_mul(min_speed - max_veh_speed));
            }

            /* move to the next tile */
            n.destination = None;
            tile = f.new_tile;
            trackdir = new_td;
            if tiles > crate::map_type::MAX_MAP_SIZE {
                break;
            }
        }

        /* save end of segment back to the node */
        n.segment_last_tile = tile;
        n.segment_last_td = trackdir;

        /* save also tile cost */
        n.cost = parent_cost.wrapping_add(segment_cost);
        true
    }

    // `CYapfDestinationTileRoadT` and `CYapfDestinationAnyDepotRoadT`.

    /// `SetDestination`.
    fn set_destination(&mut self) {
        let order_type = OrderType(self.settings.order_type);
        if order_type == OrderType::OT_GOTO_STATION {
            self.dest_station = StationID(self.settings.order_destination);
            self.station_type = if self.settings.bus != 0 {
                StationType::Bus
            } else {
                StationType::Truck
            };
            self.dest_tile = calc_closest_station_tile(
                self.s,
                self.station_commands(),
                &self.world,
                self.dest_station,
                TileIndex(self.vehicle.tile()),
                self.station_type,
            );
            self.non_artic = self.settings.articulated == 0;
            self.dest_trackdirs = TrackdirBits::INVALID_TRACKDIR_BIT;
        } else if order_type == OrderType::OT_GOTO_WAYPOINT {
            self.dest_station = StationID(self.settings.order_destination);
            self.station_type = StationType::RoadWaypoint;
            self.dest_tile = calc_closest_station_tile(
                self.s,
                self.station_commands(),
                &self.world,
                self.dest_station,
                TileIndex(self.vehicle.tile()),
                self.station_type,
            );
            self.non_artic = self.settings.articulated == 0;
            self.dest_trackdirs = TrackdirBits::INVALID_TRACKDIR_BIT;
        } else {
            self.dest_station = StationID::INVALID;
            self.dest_tile = TileIndex(self.settings.dest_tile);
            self.dest_trackdirs = TrackdirBits(track_status_to_trackdir_bits(
                crate::movement::table(self.s).get_tile_track_status_in_world(
                    &self.world,
                    self.dest_tile,
                    TransportType::TRANSPORT_ROAD,
                    u32::from(self.road_tram_type().0),
                    DiagDirection::INVALID_DIAGDIR,
                ),
            ));
        }
    }
    fn station_commands(&self) -> &'a crate::station_cmd::CommandServicesTable {
        // SAFETY: The shared table returns the static station command table.
        unsafe { &*(self.s.station_commands)() }
    }
    /// `GetDestinationStation`.
    fn get_destination_station(&self) -> *mut crate::station::BaseStation {
        if self.dest_station == StationID::INVALID {
            return std::ptr::null_mut();
        }
        // SAFETY: The shared table returns the static catchment table.
        let catchment = unsafe { &*(self.s.station_catchment)() };
        (catchment.station_if_valid)(self.dest_station)
    }
    /// `PfDetectDestination`: called by YAPF to detect if node ends in the
    /// desired destination.
    #[inline]
    fn pf_detect_destination(&self, n: &CYapfRoadNode) -> bool {
        n.destination.unwrap_or_else(|| {
            self.pf_detect_destination_tile(n.segment_last_tile, n.segment_last_td)
        })
    }
    /// `PfDetectDestinationTile`.
    #[inline]
    fn pf_detect_destination_tile(&self, tile: TileIndex, trackdir: Trackdir) -> bool {
        if self.depot {
            return self.world.road_depot(tile.0);
        }

        if self.dest_station != StationID::INVALID {
            return self.world.kind(tile) == TileType::MP_STATION
                && self.world.get_station_index(tile) == self.dest_station
                && (self.station_type == self.world.asserted(|m| get_station_type(m, tile)))
                && (self.non_artic || self.world.is_drive_through_stop_tile(tile));
        }

        tile == self.dest_tile && has_trackdir(self.dest_trackdirs.0, trackdir.0, |ok| self.check(ok))
    }
    /// `PfCalcEstimate`: called by YAPF to calculate cost estimate. Calculates
    /// distance to the destination adds it to the actual cost from origin and
    /// stores the sum to the `Node::estimate`.
    #[inline]
    fn pf_calc_estimate(&self, n: &mut CYapfRoadNode) -> bool {
        if self.depot || self.pf_detect_destination(n) {
            n.estimate = n.cost;
            return true;
        }

        n.estimate = n.cost.wrapping_add(octile_distance_cost(
            self.world.dimensions,
            n.segment_last_tile,
            n.segment_last_td,
            self.dest_tile,
        ));
        self.world.assert_with(|| {
            n.parent
                .is_some_and(|parent| n.estimate >= self.nodes.items[parent].estimate)
        });
        true
    }

    // `CYapfFollowRoadT`.

    /// `PfFollowNode`: called by YAPF to move from the given node to the next
    /// tile. For each reachable trackdir on the new tile creates new node,
    /// initializes it and adds it to the open list by calling `AddNewNode`.
    #[inline]
    fn pf_follow_node(&mut self, old_node: usize) {
        let n = self.nodes.items[old_node];
        let mut f = self.track_follower();
        if f.follow_with_road(
            self.s,
            n.segment_last_tile,
            n.segment_last_td,
            Some(self.road_context()),
        ) {
            self.add_multiple_nodes(old_node, &f);
        }
    }
    /// `ChooseRoadTrack`.
    ///
    /// # Safety
    /// `path_cache` is the vehicle's live road owner, which no service reenters.
    unsafe fn choose_road_track(
        &mut self,
        tile: TileIndex,
        enterdir: DiagDirection,
        path_found: &mut bool,
        path_cache: *mut State,
    ) -> Trackdir {
        /* Handle special case - when next tile is destination tile.
         * However, when going to a station the (initial) destination
         * tile might not be a station, but a junction, in which case
         * this method forces the vehicle to jump in circles. */
        if tile.0 == self.settings.dest_tile
            && OrderType(self.settings.order_type) != OrderType::OT_GOTO_STATION
        {
            /* choose diagonal trackdir reachable from enterdir */
            return Trackdir(diag_dir_to_diag_trackdir(enterdir.0, |ok| self.check(ok)));
        }
        /* our source tile will be the next vehicle tile (should be the given one) */
        let src_tile = tile;
        /* get available trackdirs on the start tile */
        let mut src_trackdirs = get_trackdir_bits_for_road(
            crate::movement::table(self.s),
            tile,
            self.road_tram_type(),
        );
        /* select reachable trackdirs only */
        src_trackdirs &= TrackdirBits(diagdir_reaches_trackdirs(enterdir.0));

        /* set origin and destination nodes */
        self.set_origin(src_tile, src_trackdirs);
        self.set_destination();

        /* find the best path */
        *path_found = self.find_path();

        /* if path not found - return INVALID_TRACKDIR */
        let mut next_trackdir = Trackdir::INVALID_TRACKDIR;
        if let Some(mut node) = self.get_best_node() {
            let mut steps: u32 = 0;
            let mut n = node;
            while let Some(parent) = self.nodes.items[n].parent {
                steps += 1;
                n = parent;
            }

            /* path was found or at least suggested
             * walk through the path back to its origin */
            while let Some(parent) = self.nodes.items[node].parent {
                steps -= 1;
                let item = self.nodes.items[node];
                if item.is_choice && steps < YAPF_ROADVEH_PATH_CACHE_SEGMENTS.cast_unsigned() {
                    // SAFETY: No service runs during this short exclusive cache access.
                    unsafe {
                        (*path_cache).path.push(PathElement {
                            trackdir: item.key.td.0,
                            tile: item.key.tile.0,
                        });
                    }
                }
                node = parent;
            }
            /* return trackdir from the best origin node (one of start nodes) */
            let best_next_node = self.nodes.items[node];
            self.check(best_next_node.key.tile == tile);
            next_trackdir = best_next_node.key.td;

            /* Check if target is a station, and cached path leads to within YAPF_ROADVEH_PATH_CACHE_DESTINATION_LIMIT
             * tiles of the dest tile */
            let st = self.get_destination_station();
            if !st.is_null() {
                let stop = crate::road_stop::primary_for_vehicle(self.s, st, self.vehicle);
                // SAFETY: The shared table returns the static road stop table.
                let stops = unsafe { &*(self.s.road_stops)() };
                if !stop.is_null()
                    && (self
                        .world
                        .is_drive_through_stop_tile(TileIndex((stops.tile)(stop)))
                        || !crate::road_stop::next_for_vehicle(self.s, stop, self.vehicle)
                            .is_null())
                {
                    /* Destination station has at least 2 usable road stops, or first is a drive-through stop,
                     * trim end of path cache within a number of tiles of road stop tile area */
                    let station_type = if crate::road_stop::is_bus(self.s, self.vehicle) {
                        StationType::Bus
                    } else {
                        StationType::Truck
                    };
                    let mut non_cached_area: TileArea =
                        (self.station_commands().tile_area)(st, station_type.0);
                    non_cached_area.expand(
                        YAPF_ROADVEH_PATH_CACHE_DESTINATION_LIMIT,
                        self.world.dimensions,
                        self.world.size_y(),
                    );

                    /* Find the first tile not contained by the non-cacheable area, and remove from the cache. */
                    // SAFETY: The shared queries have returned; no callback overlaps the borrow.
                    let path = unsafe { &mut (*path_cache).path };
                    let it = path
                        .iter()
                        .position(|pc| {
                            !non_cached_area.contains(
                                TileIndex(pc.tile),
                                self.world.dimensions,
                                |ok| self.check(ok),
                            )
                        })
                        .unwrap_or(path.len());
                    path.drain(..it);
                }
            }
        }
        next_trackdir
    }
    /// `FindNearestDepot`: find the best depot for a road vehicle.
    fn find_nearest_depot(&mut self, tile: TileIndex, td: Trackdir, max_distance: i32) -> FindDepotData {
        /* Set origin. */
        self.set_origin(tile, TrackdirBits(trackdir_to_trackdir_bits(td.0)));
        self.max_cost = max_distance;

        /* Find the best path and return if no depot is found. */
        if !self.find_path() {
            return FindDepotData::default();
        }

        /* Return the cost of the best path and its depot. A found path has a best node. */
        self.get_best_node()
            .map_or_else(FindDepotData::default, |n| {
                let n = &self.nodes.items[n];
                FindDepotData::new(n.segment_last_tile, n.cost.cast_unsigned(), false)
            })
    }

    /// The search's chosen direction and result, with its statistics and best node.
    fn result(&self, direction: Trackdir, found: bool) -> Result {
        let best = self.get_best_node().map(|i| self.nodes.items[i]);
        Result {
            tile: best.map_or(TileIndex::INVALID, |n| n.segment_last_tile).0,
            cost: best.map_or(0, |n| n.cost),
            direction: direction.0,
            found: u8::from(found),
            rounds: self.num_steps,
            open: self.open_count(),
            closed: self.closed_count(),
            calcs: self.stats_cost_calcs,
            distance: best.map_or(0, |n| n.estimate.wrapping_sub(n.cost)),
        }
    }
}

impl Drop for CYapfRoad<'_> {
    fn drop(&mut self) {
        if self.reuse_scratch {
            // SAFETY: Only serialized production entries set reuse_scratch.
            // No borrow of the slot spans callbacks, and no native pointer is
            // stored here. An outer search can replace a nested call's cache.
            unsafe { *SCRATCH.get() = Some(std::mem::take(&mut self.nodes)) };
        }
    }
}

/// What `RoadInput` reads, in its order: the vehicle's tile and destination,
/// the seven `pf.yapf` settings, the current order, `IsBus()`,
/// `HasArticulatedPart()`, then the vehicle's trackdir. `GetVehicleTrackdir`
/// belongs to the road vehicle controller, so the caller supplies it.
fn road_input(s: &Services, v: VehicleRef, trackdir: Trackdir) -> Input {
    let settings = s.settings();
    let tile = v.tile();
    let dest_tile = v.dest_tile();
    // SAFETY: The settings catalog holds canonical process-lifetime addresses.
    let (max_nodes, slope, crossing, stop, occupied, bay, curve) = unsafe {
        (
            settings.max_search_nodes.read(),
            settings.road_slope_penalty.read(),
            settings.road_crossing_penalty.read(),
            settings.road_stop_penalty.read(),
            settings.road_stop_occupied_penalty.read(),
            settings.road_stop_bay_occupied_penalty.read(),
            settings.road_curve_penalty.read(),
        )
    };
    let order = v.current_order();
    Input {
        tile,
        dest_tile,
        max_nodes: max_nodes.cast_signed(),
        slope: slope.cast_signed(),
        crossing: crossing.cast_signed(),
        stop: stop.cast_signed(),
        occupied: occupied.cast_signed(),
        bay: bay.cast_signed(),
        curve: curve.cast_signed(),
        order_destination: order.destination,
        order_type: order.kind(),
        bus: u8::from(crate::road_stop::is_bus(s, v)),
        articulated: u8::from(v.has_articulated_part()),
        trackdir: trackdir.0,
    }
}

/// `YapfRoadVehicleChooseTrack`: finds the best path for given road vehicle
/// using YAPF.
///
/// # Safety
/// The caller holds the game-state mutex; `v` is the live road vehicle whose
/// road owner `state` is, and no service reenters either.
#[allow(dead_code)] // The road vehicle controller calls it from #964 on.
pub(crate) unsafe fn yapf_road_vehicle_choose_track(
    s: &Services,
    v: VehicleRef,
    state: *mut State,
    tile: TileIndex,
    enterdir: DiagDirection,
    trackdirs: TrackdirBits,
    path_found: &mut bool,
) -> Trackdir {
    // The track search does not read the vehicle's trackdir.
    let input = road_input(s, v, Trackdir::INVALID_TRACKDIR);
    // SAFETY: As this function's contract.
    let mut pf = unsafe { CYapfRoad::reused(s, v, input, false) };
    // SAFETY: As this function's contract.
    let td_ret = unsafe { pf.choose_road_track(tile, enterdir, path_found, state) };

    if td_ret != Trackdir::INVALID_TRACKDIR {
        td_ret
    } else {
        Trackdir(find_first_bit(u64::from(trackdirs.0)))
    }
}

/// `YapfRoadVehicleFindNearestDepot`: used when user sends road vehicle to the
/// nearest depot or if road vehicle needs servicing using YAPF. `trackdir` is
/// the vehicle's `GetVehicleTrackdir()`.
///
/// # Safety
/// The caller holds the game-state mutex; `v` is a live road vehicle and no
/// service reenters it.
#[allow(dead_code)] // The road vehicle controller calls it from #964 on.
pub(crate) unsafe fn yapf_road_vehicle_find_nearest_depot(
    s: &Services,
    v: VehicleRef,
    trackdir: Trackdir,
    max_distance: i32,
) -> FindDepotData {
    let input = road_input(s, v, trackdir);
    // SAFETY: As this function's contract.
    let mut pf = unsafe { CYapfRoad::reused(s, v, input, true) };
    find_nearest_depot(&mut pf, max_distance).0
}

/// The body of `YapfRoadVehicleFindNearestDepot` from the vehicle's tile and
/// trackdir in `Input`, with the search's statistics for the native export.
fn find_nearest_depot(pf: &mut CYapfRoad<'_>, max_distance: i32) -> (FindDepotData, Result) {
    let tile = TileIndex(pf.settings.tile);
    let trackdir = Trackdir(pf.settings.trackdir);

    if !has_trackdir(
        get_trackdir_bits_for_road(crate::movement::table(pf.s), tile, pf.road_tram_type()).0,
        trackdir.0,
        |ok| pf.check(ok),
    ) {
        return (
            FindDepotData::default(),
            pf.result(Trackdir::INVALID_TRACKDIR, false),
        );
    }

    let depot = pf.find_nearest_depot(tile, trackdir, max_distance);
    let found = pf.best_dest_node.is_some();
    (depot, pf.result(Trackdir::INVALID_TRACKDIR, found))
}

/// The native tests' entry to [`yapf_road_vehicle_choose_track`], with their
/// own `Input`. `found` is the caller's `path_found`, which the search writes
/// only when it runs.
///
/// # Safety
/// The caller holds the game-state mutex and supplies the live shared table,
/// an initialised `input`, a live non-null vehicle record and its sole road
/// owner `state`. Services cannot reenter either owner. No pointer escapes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_yapf_choose(
    state: *mut State,
    input: *const Input,
    shared: *const Services,
    vehicle: *const OpenTTDVehicle,
    tile: u32,
    entry: u8,
    dirs: u16,
    found: u8,
) -> Result {
    // SAFETY: As this function's contract.
    let (s, input) = unsafe { (&*shared, *input) };
    // SAFETY: The caller supplies a live non-null vehicle record.
    let v = VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(vehicle.cast_mut()) });
    let mut path_found = found != 0;
    // SAFETY: As this function's contract.
    let mut pf = unsafe { CYapfRoad::reused(s, v, input, false) };
    // SAFETY: As this function's contract.
    let td_ret =
        unsafe { pf.choose_road_track(TileIndex(tile), DiagDirection(entry), &mut path_found, state) };
    let direction = if td_ret != Trackdir::INVALID_TRACKDIR {
        td_ret
    } else {
        Trackdir(find_first_bit(u64::from(dirs)))
    };
    pf.result(direction, path_found)
}
/// The native tests' entry to [`yapf_road_vehicle_find_nearest_depot`], with
/// their own `Input`, whose `tile` and `trackdir` are the vehicle's.
///
/// # Safety
/// As [`openttd_rust_road_yapf_choose`]; no path is mutated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_yapf_depot(
    input: *const Input,
    shared: *const Services,
    vehicle: *const OpenTTDVehicle,
    max_cost: i32,
) -> Result {
    // SAFETY: As this function's contract.
    let (s, input) = unsafe { (&*shared, *input) };
    // SAFETY: The caller supplies a live non-null vehicle record.
    let v = VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(vehicle.cast_mut()) });
    // SAFETY: As this function's contract.
    let mut pf = unsafe { CYapfRoad::reused(s, v, input, true) };
    find_nearest_depot(&mut pf, max_cost).1
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadYapfInput" ["probe"] => Input {
        tile,
        dest_tile,
        max_nodes,
        slope,
        crossing,
        stop,
        occupied,
        bay,
        curve,
        order_destination,
        order_type,
        bus,
        articulated,
        trackdir,
    };
    "OpenTTDRoadYapfResult" ["probe"] => Result {
        tile,
        cost,
        direction,
        found,
        rounds,
        open,
        closed,
        calcs,
        distance,
    };
}

#[cfg(test)]
mod tests {
    use super::{CYapfRoad, CYapfRoadNode, Input, Scratch, road_input};
    use crate::direction_type::DiagDirection;
    use crate::map_func::TileTypeAllocation;
    use crate::order_type::OrderType;
    use crate::services::Services;
    use crate::tile_type::TileIndex;
    use crate::track_type::{Trackdir, TrackdirBits};
    use crate::vehicle::VehicleRef;
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::ptr::{self, NonNull};

    extern "C" fn unexpected_random(_: *mut c_void) -> u32 {
        unreachable!()
    }
    extern "C" fn unexpected_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        unreachable!()
    }

    thread_local! {
        static FAILED_ASSERTIONS: Cell<u32> = const { Cell::new(0) };
        static CATCHMENT: crate::station::CatchmentServices = crate::station::CatchmentServices {
            station_if_valid: no_station,
            ..crate::test_tables::CATCHMENT
        };
        static STATION_COMMANDS: crate::station_cmd::CommandServicesTable =
            crate::station_cmd::CommandServicesTable {
                tile_area: absent_area,
                ..crate::test_tables::STATION_COMMANDS
            };
    }
    extern "C" fn record_assertion(okay: bool) {
        if !okay {
            FAILED_ASSERTIONS.set(FAILED_ASSERTIONS.get() + 1);
        }
    }
    extern "C" fn road_kind(_: VehicleRef) -> crate::road_type::RoadTramType {
        crate::road_type::RoadTramType::RTT_ROAD
    }
    extern "C" fn no_station(_: crate::station_type::StationID) -> *mut crate::station::BaseStation {
        ptr::null_mut()
    }
    extern "C" fn absent_area(_: *const crate::station::BaseStation, _: u8) -> crate::tilearea::TileArea {
        crate::tilearea::TileArea::new(TileIndex::INVALID, 0, 0)
    }
    extern "C" fn some_station(_: u16) -> *mut crate::station::BaseStation {
        NonNull::dangling().as_ptr()
    }
    extern "C" fn station_sign(_: u16) -> u32 {
        7
    }
    extern "C" fn station_commands() -> *const crate::station_cmd::CommandServicesTable {
        STATION_COMMANDS.with(ptr::from_ref)
    }
    extern "C" fn catchment() -> *const crate::station::CatchmentServices {
        CATCHMENT.with(ptr::from_ref)
    }

    /// A 64 by 64 map read through the fixture's tile-type callback (all
    /// `MP_CLEAR`), with recorded native assertions. The tables live for the
    /// test process.
    fn shared() -> Services {
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: ptr::null(),
                extended: ptr::null(),
                count: 0,
                assertions: true,
            },
            6,
        );
        FAILED_ASSERTIONS.set(0);
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            native_assert: record_assertion,
            road_kind,
            ..crate::test_tables::MOVEMENT
        }));
        let s = Services {
            movement,
            base_station: some_station,
            base_station_xy: station_sign,
            station_commands,
            station_catchment: catchment,
            ..crate::services::fixture(ptr::null_mut(), unexpected_random, unexpected_industry)
        };
        movement.shared = Box::leak(Box::new(s));
        s
    }
    fn input() -> Input {
        Input {
            tile: 0,
            dest_tile: 0,
            max_nodes: 0,
            slope: 0,
            crossing: 0,
            stop: 0,
            occupied: 0,
            bay: 0,
            curve: 0,
            order_destination: 0,
            order_type: 0,
            bus: 0,
            articulated: 0,
            trackdir: 0,
        }
    }
    /// A search for a road vehicle record that lives for the test process.
    fn search(s: &Services, input: Input) -> CYapfRoad<'_> {
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_ROAD,
        );
        CYapfRoad::new(
            s,
            VehicleRef(NonNull::new(record).unwrap()),
            input,
            false,
            Scratch::default(),
            false,
        )
    }

    #[test]
    fn original_heap_ties_and_arbitrary_removal() {
        // Replaces the native heap probe with the same operations on the open-list heap.
        // Expected results were captured from unchanged OpenTTD 15.3
        // (14ec60f248547d4d062a1160f0fc26d742319888), misc/binaryheap.hpp,
        // using the former native fixture's CBinaryHeapT loop. Saves do not
        // expose these intermediate ties, internal/last removals and replacements.
        let s = shared();
        let mut pf = search(&s, input());
        for estimate in [100, 100, 100, 50, 100, 200, 50, 100] {
            let mut node = CYapfRoadNode::set(None, TileIndex(0), Trackdir(0), false);
            node.estimate = estimate;
            pf.nodes.items.push(node);
        }
        let include = |pf: &mut CYapfRoad<'_>, item| {
            let items = &pf.nodes.items;
            pf.nodes
                .open_queue
                .include(item, |a, b| items[a].estimate < items[b].estimate);
        };
        let remove = |pf: &mut CYapfRoad<'_>, item| {
            let items = &pf.nodes.items;
            pf.nodes
                .open_queue
                .remove(item, |a, b| items[a].estimate < items[b].estimate);
        };
        let commands = [
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 2,
        ];
        let values = [
            0, 1, 2, 3, 4, 5, 6, 7, 1, 7, 1, 7, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0,
        ];
        let expected = [
            0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 6, 1, 0, 4, 2, 7, 5, 1, 1, 1,
        ];
        for ((command, value), expected) in commands.into_iter().zip(values).zip(expected) {
            let actual = match command {
                0 => {
                    include(&mut pf, value);
                    pf.nodes.open_queue.first().unwrap()
                }
                1 => {
                    remove(&mut pf, value);
                    pf.nodes.open_queue.first().unwrap()
                }
                2 => {
                    let first = pf.nodes.open_queue.first().unwrap();
                    remove(&mut pf, first);
                    first
                }
                _ => unreachable!(),
            };
            assert_eq!(actual, expected);
        }
    }

    thread_local! {
        static BUS: Cell<bool> = const { Cell::new(false) };
        static ROAD_STOPS: crate::road_stop::RoadStopServices = crate::road_stop::RoadStopServices {
            vehicle: ptr::null(),
            ground: ptr::null(),
            primary: crate::test_tables::unreachable2,
            head: crate::test_tables::unreachable2,
            is_bus,
            next: crate::test_tables::unreachable1,
            tile: crate::test_tables::unreachable1,
            state: crate::test_tables::unreachable1,
            pool_next: crate::test_tables::unreachable1,
        };
    }
    extern "C" fn is_bus(_: VehicleRef) -> bool {
        BUS.get()
    }
    extern "C" fn road_stops() -> *const crate::road_stop::RoadStopServices {
        ROAD_STOPS.with(ptr::from_ref)
    }

    #[test]
    fn road_input_reads_each_setting_and_vehicle_fact_into_its_own_field() {
        // Seven distinct settings, so a swapped pair fails, which the play
        // scenarios cannot show at default settings.
        let values = [101_u32, 102, 103, 104, 105, 106, 107];
        let settings = crate::settings_type::SettingAddresses {
            max_search_nodes: &raw const values[0],
            road_slope_penalty: &raw const values[1],
            road_crossing_penalty: &raw const values[2],
            road_stop_penalty: &raw const values[3],
            road_stop_occupied_penalty: &raw const values[4],
            road_stop_bay_occupied_penalty: &raw const values[5],
            road_curve_penalty: &raw const values[6],
            ..crate::settings_type::SettingAddresses::NULL
        };
        let s = Services {
            settings: &raw const settings,
            road_stops,
            ..shared()
        };
        let mut orders = crate::orders::VehicleOrders {
            current: std::mem::MaybeUninit::new(crate::orders::Order {
                kind: OrderType::OT_GOTO_WAYPOINT.0 | 0x40,
                flags: 0,
                destination: 4321,
                refit: 0,
                wait: 0,
                travel: 0,
                speed: 0,
            }),
            orders: ptr::null_mut(),
            next: ptr::null_mut(),
            previous: ptr::null_mut(),
        };
        let front = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_ROAD,
        );
        let part = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_ROAD,
        );
        // SAFETY: Both records are live and owned by this test.
        unsafe {
            (*front).tile = TileIndex(1111);
            (*front).dest_tile = TileIndex(2222);
            (*front).orders = &raw mut orders;
            (*part).subtype = 1 << crate::vehicle_base::GVSF_ARTICULATED_PART;
        }
        let v = VehicleRef(NonNull::new(front).unwrap());
        for (bus, articulated, trackdir) in [
            (true, true, Trackdir::TRACKDIR_LEFT_S),
            (false, false, Trackdir::TRACKDIR_Y_NW),
        ] {
            BUS.set(bus);
            // SAFETY: Both records are live and owned by this test.
            unsafe { (*front).next = if articulated { part } else { ptr::null_mut() } };
            let input = road_input(&s, v, trackdir);
            assert_eq!(
                (input.tile, input.dest_tile),
                (1111, 2222),
                "tile and destination"
            );
            assert_eq!(
                [
                    input.max_nodes,
                    input.slope,
                    input.crossing,
                    input.stop,
                    input.occupied,
                    input.bay,
                    input.curve,
                ],
                [101, 102, 103, 104, 105, 106, 107],
                "settings in RoadInput's order"
            );
            assert_eq!(
                (input.order_destination, input.order_type),
                (4321, OrderType::OT_GOTO_WAYPOINT.0),
                "the current order's destination and type"
            );
            assert_eq!(
                (input.bus, input.articulated, input.trackdir),
                (u8::from(bus), u8::from(articulated), trackdir.0)
            );
        }
        // SAFETY: The records came from the allocation export and are released once.
        unsafe {
            crate::vehicle_base::openttd_rust_vehicle_record_delete(part);
            crate::vehicle_base::openttd_rust_vehicle_record_delete(front);
        }
    }

    #[test]
    fn lowered_child_estimate_records_pf_calc_estimate_assertion() {
        let s = shared();
        let mut pf = search(&s, input());
        pf.dest_tile = TileIndex(40 + 40 * 64);
        let mut parent = CYapfRoadNode::set(None, TileIndex(0), Trackdir::TRACKDIR_X_NE, false);
        parent.estimate = 10_000;
        pf.nodes.items.push(parent);
        let mut child = CYapfRoadNode::set(
            Some(0),
            TileIndex(39 + 40 * 64),
            Trackdir::TRACKDIR_X_SW,
            false,
        );
        child.destination = Some(false);
        child.cost = 100;
        assert!(pf.pf_calc_estimate(&mut child));
        assert_eq!(FAILED_ASSERTIONS.get(), 1);

        // An estimate at or above the parent's passes.
        pf.nodes.items[0].estimate = child.estimate;
        assert!(pf.pf_calc_estimate(&mut child));
        assert_eq!(FAILED_ASSERTIONS.get(), 1);
    }

    #[test]
    fn best_origin_node_on_another_tile_records_choose_road_track_assertion() {
        let s = shared();
        let mut pf = search(
            &s,
            Input {
                order_type: OrderType::OT_GOTO_STATION.0,
                ..input()
            },
        );
        let mut state = crate::road::State::default();
        // A destination origin node on tile 2, where the vehicle's next tile is
        // tile 3: unreachable from SetOrigin, which places every origin on it.
        let mut origin = CYapfRoadNode::set(None, TileIndex(2), Trackdir::TRACKDIR_X_NE, false);
        origin.destination = Some(true);
        pf.nodes.items.push(origin);
        pf.insert_open_node(0);
        let mut path_found = false;
        // SAFETY: The test owns the road state; no service reenters it.
        let td = unsafe {
            pf.choose_road_track(
                TileIndex(3),
                DiagDirection::DIAGDIR_NE,
                &mut path_found,
                &raw mut state,
            )
        };
        assert!(path_found);
        assert_eq!(td, Trackdir::TRACKDIR_X_NE);
        assert_eq!(FAILED_ASSERTIONS.get(), 1);
        assert!(state.path.is_empty());
    }

    const NOT_REACHED_EXIT: i32 = 86;
    extern "C" fn not_reached() {
        std::process::exit(NOT_REACHED_EXIT);
    }
    extern "C" fn no_assertion(_: bool) {}

    #[test]
    fn closed_node_bettered_by_a_new_node_takes_the_native_unreachable_path() {
        // The native unreachable path ends the process, so the search runs in a
        // child whose `NOT_REACHED` service exits with a code the parent checks.
        const NAME: &str =
            "road_yapf::tests::closed_node_bettered_by_a_new_node_takes_the_native_unreachable_path";
        const CHILD: &str = "OPENTTD_ROAD_YAPF_NOT_REACHED_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(NOT_REACHED_EXIT));
            return;
        }
        let s = Services {
            not_reached,
            ..shared()
        };
        // SAFETY: `s` outlives the child process's only search.
        unsafe { crate::services_assert::openttd_rust_assert_bind(no_assertion, &raw const s) };
        let destination = TileIndex(10 + 10 * 64);
        let mut pf = search(&s, input());
        pf.dest_tile = destination;
        pf.dest_trackdirs = TrackdirBits::INVALID_TRACKDIR_BIT;
        // A closed node keyed by the destination tile and the north-east exit.
        let mut closed = CYapfRoadNode::set(None, destination, Trackdir::TRACKDIR_X_NE, false);
        closed.estimate = 1000;
        pf.nodes.items.push(closed);
        pf.insert_closed_node(0);
        // A curve with the same exit on the destination costs 71 and reads no
        // map field: its estimate betters the closed node's.
        pf.add_new_node(CYapfRoadNode::set(
            Some(0),
            destination,
            Trackdir::TRACKDIR_UPPER_E,
            false,
        ));
    }
}
