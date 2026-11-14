/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The ship pathfinding (`pathfinder/yapf/yapf_ship.cpp`) and its water region
//! search (`yapf_ship_regions.cpp`), over YAPF's main loop (`yapf_base.hpp`),
//! tile origin (`yapf_common.hpp`) and node list (`nodelist.hpp`); and the
//! ship's path cache, which the C++ `Ship` shell holds through `ShipPathCache`.
//! Lookup maps are never iterated; arena identity survives open-node replacement.
//! Heap ties follow `CBinaryHeapT`, including removal's upward then downward repair.
//!
//! Each entry is a synchronous call under the game-state mutex. A search loads
//! the map descriptor and resolves the water region owner once, at entry, and
//! forms `&mut Regions` only for the length of each `water_regions` call. No
//! service a search calls loads, resets or yields the map, replaces the region
//! owner, or reenters the search, the path cache or the ship controller.
use crate::binaryheap::BinaryHeap;
use crate::bitmath_func::{find_first_bit, has_bit, kill_first_bit_u16};
use crate::debug::debug;
use crate::direction_func::{diag_dir_difference, reverse_diag_dir};
use crate::direction_type::{DiagDirDiff, DiagDirection, Direction};
use crate::engine_type::apply_water_class_speed_frac;
use crate::follow_track::CFollowTrackT;
use crate::game_state::GameState;
use crate::map_func::World;
use crate::node_hash::NodeMap;
use crate::order_type::OrderType;
use crate::pathfinder_func::calc_closest_station_tile;
use crate::pathfinder_type::{YAPF_TILE_CORNER_LENGTH, YAPF_TILE_LENGTH};
use crate::rail_type::RailTypes;
use crate::services::Services;
use crate::ship_control::{Leaves, get_effective_water_class, is_ship_destination_tile};
use crate::station_type::{StationID, StationType};
use crate::tile_type::{TILE_HEIGHT, TileIndex};
use crate::track_func::{
    diagdir_reaches_trackdirs, find_first_trackdir, has_trackdir, is_diagonal_trackdir,
    is_valid_trackdir, next_trackdir, remove_first_trackdir, reverse_trackdir,
    track_status_to_trackdir_bits, trackdir_crosses_trackdirs, trackdir_to_track,
    trackdir_to_trackdir_bits, vehicle_exit_dir,
};
use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
use crate::transport_type::TransportType;
use crate::vehicle::{OpenTTDVehicle, VehicleRef};
use crate::vehicle_type::VehicleType;
use crate::water_map::WaterClass;
use crate::water_regions::{
    INVALID_WATER_REGION_PATCH, Regions, WATER_REGION_NUMBER_OF_TILES, WaterRegionPatchDesc,
    WaterVisit, calculate_water_region_patch_hash, get_water_region_center_tile,
    get_water_region_info, get_water_region_patch_info, visit_water_region_patch_neighbours,
};
use crate::yapf_common::octile_distance_cost;
use crate::yapf_node::{CYapfNodeKeyExitDir, Node};
use std::hash::{Hash, Hasher};

const NUMBER_OR_WATER_REGIONS_LOOKAHEAD: i32 = 4;
/// 4 possible exit dirs per tile.
const MAX_SHIP_PF_NODES: i32 =
    (NUMBER_OR_WATER_REGIONS_LOOKAHEAD + 1) * WATER_REGION_NUMBER_OF_TILES * 4;

/// The length of the (aimless) path assigned when a ship is lost.
const SHIP_LOST_PATH_LENGTH: i32 = 8;

const DIRECT_NEIGHBOUR_COST: i32 = 100;
const NODES_PER_REGION: i32 = 4;
const MAX_NUMBER_OF_NODES: i32 = 65536;

/// `ShipPathCache`: the trackdirs a ship follows before it searches again, last
/// first. The original's `ShipPathElement` holds only its trackdir.
#[derive(Clone, Default)]
pub struct ShipPathCache(Vec<Trackdir>);
impl ShipPathCache {
    /// `back`.
    fn back(&self) -> Trackdir {
        self.0[self.0.len() - 1]
    }
}

/// What a search reads from the ship, its engine and `_settings_game.pf.yapf`,
/// filled once per entry by [`ship_input`] or by a native test. Nothing a search
/// does can change them.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Input {
    pub tile: u32,
    pub dest_tile: u32,
    pub order_type: u8,
    pub order_destination: u16,
    pub direction: u8,
    pub max_speed: u16,
    pub ocean_speed_frac: u8,
    pub canal_speed_frac: u8,
    pub ship_curve45_penalty: u32,
    pub ship_curve90_penalty: u32,
    pub unit_number: u16,
}

/// The ship a search runs for, as the ship controller supplies it.
/// `ship_control.rs`'s pass replaces it with that file's own handle.
pub(crate) struct Ship<'a> {
    pub(crate) v: VehicleRef,
    /// `v->state`.
    pub(crate) state: TrackBits,
    /// `v->GetVehicleTrackdir()`, whose ship body is in an export of `ship_control.rs`.
    pub(crate) trackdir: Trackdir,
    /// The controller's leaves, for `IsShipDestinationTile`.
    pub(crate) leaves: &'a Leaves,
}

/// What `ShipInput` reads, in the order the original first reads it: the ship's
/// tile and destination, its current order, its direction, its engine's speed
/// and speed fractions, the two curve penalties and its unit number.
fn ship_input(s: &Services, v: &Ship<'_>) -> Input {
    let tile = v.v.tile();
    let dest_tile = v.v.dest_tile();
    let order = v.v.current_order();
    let direction = v.v.direction();
    let svi = (crate::movement::table(s).vehicle().engine)(v.v);
    let settings = s.settings();
    // SAFETY: The settings catalog holds canonical process-lifetime addresses.
    let (ship_curve45_penalty, ship_curve90_penalty) = unsafe {
        (
            settings.ship_curve45_penalty.read(),
            settings.ship_curve90_penalty.read(),
        )
    };
    Input {
        tile,
        dest_tile,
        order_type: order.kind(),
        order_destination: order.destination,
        direction,
        max_speed: svi.ship_speed(),
        ocean_speed_frac: svi.ship_speed_fraction(true),
        canal_speed_frac: svi.ship_speed_fraction(false),
        ship_curve45_penalty,
        ship_curve90_penalty,
        unit_number: v.v.unit_number(s),
    }
}

/// `const Ship *v` as a search reads it, with the entry's shared table, map
/// descriptor and water region owner.
pub(crate) struct Vehicle<'a> {
    s: &'a Services,
    world: World<'a>,
    regions: *mut Regions,
    input: Input,
    state: TrackBits,
    trackdir: Trackdir,
    leaves: &'a Leaves,
}
impl<'a> Vehicle<'a> {
    pub(crate) fn new(
        s: &'a Services,
        input: Input,
        state: TrackBits,
        trackdir: Trackdir,
        leaves: &'a Leaves,
    ) -> Self {
        // SAFETY: The shared table returns the static water table, whose service
        // returns the current map's region owner.
        let regions = unsafe { ((*(s.water_services)()).water_regions)() };
        Self {
            s,
            world: World::new(s),
            regions,
            input,
            state,
            trackdir,
            leaves,
        }
    }
    #[inline]
    fn check(&self, condition: bool) {
        crate::movement::native_assert(self.s, condition);
    }
    /// `GetWaterRegionPatchInfo`.
    #[inline]
    fn get_water_region_patch_info(&self, tile: TileIndex) -> WaterRegionPatchDesc {
        // SAFETY: As the module contract.
        get_water_region_patch_info(unsafe { &mut *self.regions }, &self.world, self.s, tile)
    }
}

/// A node key, with the retained scratch of its node list.
trait NodeKey: Copy + Eq + Hash + 'static {
    fn scratch() -> &'static GameState<Option<NodeList<Self>>>;
}

/// `NodeList`: the node storage, the hash tables of open and closed nodes, and
/// the open nodes' priority queue, which the original builds on every search
/// as `CShipNodeList` and `WaterRegionNodeList` inside `Tpf pf`. The capacity
/// is kept between searches of one node type, because allocating both lists
/// per search costs a measured 65.0 million instructions, 0.47%, under the
/// `play-padhattan-ridge-2016-manual` scenario (#977). Each search empties its
/// list on entry, so nothing carries from one search to the next.
struct NodeList<K> {
    /// Storage of the nodes.
    items: Vec<Node<K>>,
    /// Hash table of pointers to open nodes.
    open_nodes: NodeMap<K, usize>,
    /// Hash table of pointers to closed nodes.
    closed_nodes: NodeMap<K, usize>,
    /// Priority queue of pointers to open nodes.
    open_queue: BinaryHeap,
}
impl<K> Default for NodeList<K> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            open_nodes: NodeMap::default(),
            closed_nodes: NodeMap::default(),
            open_queue: BinaryHeap::default(),
        }
    }
}
impl<K: NodeKey> NodeList<K> {
    /// The scratch of this node type, empty. The caller holds the game-state
    /// mutex; ownership moves out of the slot, so a search started meanwhile
    /// gets its own storage.
    fn take() -> Self {
        // SAFETY: Native production entries serialize access; the slot borrow
        // ends here, before any query.
        let mut nodes = unsafe { (*K::scratch().get()).take().unwrap_or_default() };
        nodes.items.clear();
        nodes.open_nodes.clear();
        nodes.closed_nodes.clear();
        nodes.open_queue = BinaryHeap::reuse(std::mem::take(&mut nodes.open_queue));
        nodes
    }
    /// `OpenCount`: return number of open nodes.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "HashTable::Count returns the original's int item count"
    )]
    fn open_count(&self) -> i32 {
        self.open_nodes.len() as i32
    }
    /// `ClosedCount`: return number of closed nodes.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "HashTable::Count returns the original's int item count"
    )]
    fn closed_count(&self) -> i32 {
        self.closed_nodes.len() as i32
    }
    /// `InsertOpenNode`: insert given item as open node (into `open_nodes` and
    /// `open_queue`).
    #[inline]
    fn insert_open_node(&mut self, world: &World<'_>, item: usize) {
        let key = self.items[item].key;
        world.assert_with(|| !self.closed_nodes.contains_key(&key));
        self.open_nodes.insert(key, item);
        let items = &self.items;
        self.open_queue
            .include(item, |a, b| items[a].less(&items[b]));
    }
    /// `GetBestOpenNode`: return the best open node.
    #[inline]
    fn get_best_open_node(&self) -> Option<usize> {
        self.open_queue.first()
    }
    /// `FindOpenNode`: return the open node specified by a key or `None` if not found.
    #[inline]
    fn find_open_node(&self, key: &K) -> Option<usize> {
        self.open_nodes.get(key).copied()
    }
    /// `PopOpenNode`: remove and return the open node specified by a key.
    #[inline]
    fn pop_open_node(&mut self, world: &World<'_>, key: &K) -> usize {
        let item = self.open_nodes.remove(key);
        world.check(item.is_some());
        let Some(item) = item else {
            crate::services_assert::not_reached()
        };
        let items = &self.items;
        self.open_queue
            .remove(item, |a, b| items[a].less(&items[b]));
        item
    }
    /// `InsertClosedNode`: close node.
    #[inline]
    fn insert_closed_node(&mut self, world: &World<'_>, item: usize) {
        let key = self.items[item].key;
        world.assert_with(|| !self.open_nodes.contains_key(&key));
        self.closed_nodes.insert(key, item);
    }
    /// `FindClosedNode`: return the closed node specified by a key or `None` if
    /// not found.
    #[inline]
    fn find_closed_node(&self, key: &K) -> Option<usize> {
        self.closed_nodes.get(key).copied()
    }
}

/// `CYapfBaseT`'s state. This file's copy of YAPF's base, beside
/// `road_yapf.rs`'s and `rail_yapf.rs`'s, which #971 merges into one.
/// No search reads `GetBestNode` after `FindPath` fails, so the base keeps no
/// `best_intermediate_node`; and no ship search has a segment cost cache, so
/// it keeps no cache statistics.
struct CYapfBaseT<'v, K: NodeKey> {
    /// Node list multi-container.
    nodes: NodeList<K>,
    /// The destination node found at last round.
    best_dest_node: Option<usize>,
    /// Maximum number of nodes we are allowed to visit before we give up.
    max_search_nodes: i32,
    /// Vehicle that we are trying to drive.
    vehicle: &'v Vehicle<'v>,
    num_steps: i32,
}
impl<'v, K: NodeKey> CYapfBaseT<'v, K> {
    fn new(vehicle: &'v Vehicle<'v>, max_nodes: i32) -> Self {
        Self {
            nodes: NodeList::take(),
            best_dest_node: None,
            max_search_nodes: max_nodes,
            vehicle,
            num_steps: 0,
        }
    }
}
impl<K: NodeKey> Drop for CYapfBaseT<'_, K> {
    fn drop(&mut self) {
        // SAFETY: Production entries hold the game-state mutex; no borrow of
        // the slot spans a query. A later search can replace this scratch.
        unsafe { *K::scratch().get() = Some(std::mem::take(&mut self.nodes)) };
    }
}

/// `CYapfBaseT`'s main loop and node evaluation, over the derived search's
/// `Pf*` callbacks.
trait Yapf<'v> {
    type Key: NodeKey;
    type TrackFollower;
    /// The witness hit when the search reaches its node limit.
    const NODE_LIMIT_WITNESS: usize;

    fn base(&self) -> &CYapfBaseT<'v, Self::Key>;
    fn base_mut(&mut self) -> &mut CYapfBaseT<'v, Self::Key>;
    fn pf_detect_destination(&self, n: &Node<Self::Key>) -> bool;
    fn pf_follow_node(&mut self, old_node: usize);
    fn pf_calc_cost(&self, n: &mut Node<Self::Key>, follower: &Self::TrackFollower) -> bool;
    fn pf_calc_estimate(&self, n: &mut Node<Self::Key>) -> bool;
    fn transport_type_char(&self) -> char;

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
            let base = self.base_mut();
            base.num_steps = base.num_steps.wrapping_add(1);
            let Some(best_open_node) = base.nodes.get_best_open_node() else {
                break;
            };

            if self.pf_detect_destination(&self.base().nodes.items[best_open_node]) {
                self.base_mut().best_dest_node = Some(best_open_node);
                break;
            }

            self.pf_follow_node(best_open_node);
            let base = self.base_mut();
            if base.max_search_nodes != 0 && base.nodes.closed_count() >= base.max_search_nodes {
                crate::witness::hit(Self::NODE_LIMIT_WITNESS);
                break;
            }

            let key = base.nodes.items[best_open_node].key;
            base.nodes.pop_open_node(&base.vehicle.world, &key);
            base.nodes
                .insert_closed_node(&base.vehicle.world, best_open_node);
        }

        let base = self.base();
        let destination_found = base.best_dest_node.is_some();

        debug!(
            yapf,
            3,
            "[YAPF{}]{}{:4} - {} rounds - {} open - {} closed - CHR {:4.1}% - C {} D {}",
            self.transport_type_char(),
            if destination_found { '-' } else { '!' },
            base.vehicle.input.unit_number,
            base.num_steps,
            base.nodes.open_count(),
            base.nodes.closed_count(),
            // No ship search has a segment cost cache, so no cache hits.
            0.0_f32,
            base.best_dest_node.map_or(-1, |i| base.nodes.items[i].cost),
            base.best_dest_node.map_or(-1, |i| {
                let n = &base.nodes.items[i];
                n.estimate.wrapping_sub(n.cost)
            })
        );

        destination_found
    }
    /// `GetBestNode`: the best node that has reached the destination, if the
    /// path was found.
    #[inline]
    fn get_best_node(&self) -> Option<usize> {
        self.base().best_dest_node
    }
    /// `AddStartupNode`: add new node (created by `CreateNewNode` and filled
    /// with data) into open list.
    fn add_startup_node(&mut self, n: Node<Self::Key>) {
        let base = self.base_mut();
        base.vehicle.check(n.parent.is_none());
        base.vehicle.check(base.num_steps == 0);

        /* insert the new node only if it is not there */
        if base.nodes.find_open_node(&n.key).is_none() {
            let item = base.nodes.items.len();
            base.nodes.items.push(n);
            base.nodes.insert_open_node(&base.vehicle.world, item);
        }
    }
    /// `AddNewNode`: called by `PfFollowNode` for each child node. Nodes are
    /// evaluated here and added into open list. The original's choice of
    /// `best_intermediate_node` goes with that field.
    fn add_new_node(&mut self, mut n: Node<Self::Key>, follower: &Self::TrackFollower) {
        self.base().vehicle.check(n.parent.is_some());

        /* evaluate the node */
        let mut valid = self.pf_calc_cost(&mut n, follower);
        if valid {
            valid = self.pf_calc_estimate(&mut n);
        }

        /* have the cost or estimate callbacks marked this node as invalid? */
        if !valid {
            return;
        }

        let base = self.base_mut();
        /* check new node against open list */
        if let Some(open_node) = base.nodes.find_open_node(&n.key) {
            /* another node exists with the same key in the open list
             * is it better than new one? */
            if n.estimate < base.nodes.items[open_node].estimate {
                /* update the old node by value from new one */
                base.nodes.pop_open_node(&base.vehicle.world, &n.key);
                base.nodes.items[open_node] = n;
                /* add the updated old node back to open list */
                base.nodes.insert_open_node(&base.vehicle.world, open_node);
            }
            return;
        }

        /* check new node against closed list */
        if let Some(closed_node) = base.nodes.find_closed_node(&n.key) {
            /* another node exists with the same key in the closed list
             * is it better than new one? */
            let node_est = n.estimate;
            let closed_est = base.nodes.items[closed_node].estimate;
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
        let item = base.nodes.items.len();
        base.nodes.items.push(n);
        base.nodes.insert_open_node(&base.vehicle.world, item);
    }
}

/// `CYapfShipNode`: yapf node for ships.
type CYapfShipNode = Node<CYapfNodeKeyExitDir>;

static SHIP_NODES: GameState<Option<NodeList<CYapfNodeKeyExitDir>>> = GameState::new(None);
impl NodeKey for CYapfNodeKeyExitDir {
    fn scratch() -> &'static GameState<Option<NodeList<Self>>> {
        &SHIP_NODES
    }
}

/// `CYapfShip`: the ship search, with the state of its
/// `CYapfDestinationTileWaterT` and `CYapfFollowShipT` modules.
struct CYapfShip<'v> {
    base: CYapfBaseT<'v, CYapfNodeKeyExitDir>,
    // `CYapfDestinationTileWaterT`.
    dest_tile: TileIndex,
    dest_trackdirs: TrackdirBits,
    dest_station: StationID,
    has_intermediate_dest: bool,
    intermediate_dest_tile: TileIndex,
    intermediate_dest_region_patch: WaterRegionPatchDesc,
    // `CYapfFollowShipT`.
    /// The high level path whose regions the search may enter; empty for no
    /// restriction. The original copies each entry's region.
    water_region_corridor: &'v [WaterRegionPatchDesc],
}
impl<'v> CYapfShip<'v> {
    fn new(v: &'v Vehicle<'v>, max_nodes: i32) -> Self {
        Self {
            base: CYapfBaseT::new(v, max_nodes),
            dest_tile: TileIndex::INVALID,
            dest_trackdirs: TrackdirBits::TRACKDIR_BIT_NONE,
            dest_station: StationID::INVALID,
            has_intermediate_dest: false,
            intermediate_dest_tile: TileIndex::INVALID,
            intermediate_dest_region_patch: WaterRegionPatchDesc {
                x: 0,
                y: 0,
                label: INVALID_WATER_REGION_PATCH,
            },
            water_region_corridor: &[],
        }
    }
    #[inline]
    fn v(&self) -> &'v Vehicle<'v> {
        self.base.vehicle
    }

    // `CYapfOriginTileT`.

    /// `SetOrigin`: set origin tile / trackdir mask. The ship node does not keep
    /// the original's `is_choice`.
    fn set_origin(&mut self, tile: TileIndex, trackdirs: TrackdirBits) {
        let mut tdb = trackdirs;
        while tdb != TrackdirBits::TRACKDIR_BIT_NONE {
            let td = Trackdir(find_first_bit(u64::from(tdb.0)));
            self.add_startup_node(CYapfShipNode::set(None, CYapfNodeKeyExitDir::set(tile, td)));
            tdb = TrackdirBits(kill_first_bit_u16(tdb.0));
        }
    }

    // `CYapfBaseT`.

    /// `AddMultipleNodes`: add multiple nodes - direct children of the given node.
    #[inline]
    fn add_multiple_nodes(&mut self, parent: usize, tf: &CFollowTrackT) {
        let mut rtds = tf.new_td_bits;
        while rtds != TrackdirBits::TRACKDIR_BIT_NONE {
            let td = Trackdir(find_first_bit(u64::from(rtds.0)));
            let n = CYapfShipNode::set(Some(parent), CYapfNodeKeyExitDir::set(tf.new_tile, td));
            self.add_new_node(n, tf);
            rtds = TrackdirBits(kill_first_bit_u16(rtds.0));
        }
    }

    // `CYapfDestinationTileWaterT`.

    /// `SetDestination`.
    fn set_destination(&mut self) {
        let v = self.v();
        if OrderType(v.input.order_type) == OrderType::OT_GOTO_STATION {
            self.dest_station = StationID(v.input.order_destination);
            // SAFETY: The shared table returns the static station command table.
            let commands = unsafe { &*(v.s.station_commands)() };
            self.dest_tile = calc_closest_station_tile(
                v.s,
                commands,
                &v.world,
                self.dest_station,
                TileIndex(v.input.tile),
                StationType::Dock,
            );
            self.dest_trackdirs = TrackdirBits::INVALID_TRACKDIR_BIT;
        } else {
            self.dest_station = StationID::INVALID;
            self.dest_tile = TileIndex(v.input.dest_tile);
            self.dest_trackdirs = TrackdirBits(track_status_to_trackdir_bits(
                crate::movement::table(v.s).get_tile_track_status_in_world(
                    &v.world,
                    TileIndex(v.input.dest_tile),
                    TransportType::TRANSPORT_WATER,
                    0,
                    DiagDirection::INVALID_DIAGDIR,
                ),
            ));
        }
    }
    /// `SetIntermediateDestination`.
    fn set_intermediate_destination(&mut self, water_region_patch: WaterRegionPatchDesc) {
        crate::witness::hit(crate::witness::SHIP_YAPF_INTERMEDIATE);
        self.has_intermediate_dest = true;
        self.intermediate_dest_tile =
            get_water_region_center_tile(self.v().world.dimensions, water_region_patch.into());
        self.intermediate_dest_region_patch = water_region_patch;
    }
    /// `PfDetectDestinationTile`.
    #[inline]
    fn pf_detect_destination_tile(&self, tile: TileIndex, trackdir: Trackdir) -> bool {
        let v = self.v();
        if self.has_intermediate_dest {
            /* GetWaterRegionInfo is much faster than GetWaterRegionPatchInfo so we try that first. */
            if get_water_region_info(v.world.dimensions, tile)
                != self.intermediate_dest_region_patch
            {
                return false;
            }
            return v.get_water_region_patch_info(tile) == self.intermediate_dest_region_patch;
        }

        if self.dest_station != StationID::INVALID {
            return v.world.is_docking_tile(tile)
                && is_ship_destination_tile(v.leaves, tile, self.dest_station);
        }

        tile == self.dest_tile
            && (self.dest_trackdirs & TrackdirBits(trackdir_to_trackdir_bits(trackdir.0)))
                != TrackdirBits::TRACKDIR_BIT_NONE
    }

    // `CYapfFollowShipT`.

    /// `RestrictSearch`: restricts the search by creating corridor or water
    /// regions through which the ship is allowed to travel.
    fn restrict_search(&mut self, path: &'v [WaterRegionPatchDesc]) {
        crate::witness::hit(crate::witness::SHIP_YAPF_RETRIES);
        self.water_region_corridor = path;
    }

    // `CYapfCostShipT`.

    /// `CurveCost`.
    #[inline]
    fn curve_cost(&self, td1: Trackdir, td2: Trackdir) -> i32 {
        let v = self.v();
        v.check(is_valid_trackdir(td1.0));
        v.check(is_valid_trackdir(td2.0));

        if has_trackdir(trackdir_crosses_trackdirs(td1.0), td2.0, |ok| v.check(ok)) {
            /* 90-deg curve penalty. */
            return v.input.ship_curve90_penalty.cast_signed();
        } else if td2.0 != next_trackdir(td1.0, |ok| v.check(ok)) {
            /* 45-deg curve penalty. */
            return v.input.ship_curve45_penalty.cast_signed();
        }
        0
    }
}

/// `IsPreferredShipDirection`: whether the provided direction is a preferred
/// direction for a given tile. This is used to separate ships travelling in
/// opposite directions.
#[inline]
fn is_preferred_ship_direction(map: crate::map_func::Map, tile: TileIndex, td: Trackdir) -> bool {
    let odd_x = map.tile_x(tile) & 1 != 0;
    let odd_y = map.tile_y(tile) & 1 != 0;
    if td == Trackdir::TRACKDIR_X_NE {
        return odd_y;
    }
    if td == Trackdir::TRACKDIR_X_SW {
        return !odd_y;
    }
    if td == Trackdir::TRACKDIR_Y_NW {
        return odd_x;
    }
    if td == Trackdir::TRACKDIR_Y_SE {
        return !odd_x;
    }
    (odd_x ^ odd_y)
        ^ has_bit(
            (TrackdirBits::TRACKDIR_BIT_RIGHT_N
                | TrackdirBits::TRACKDIR_BIT_LEFT_S
                | TrackdirBits::TRACKDIR_BIT_UPPER_W
                | TrackdirBits::TRACKDIR_BIT_LOWER_E)
                .0,
            td.0,
        )
}

impl<'v> Yapf<'v> for CYapfShip<'v> {
    type Key = CYapfNodeKeyExitDir;
    type TrackFollower = CFollowTrackT;
    const NODE_LIMIT_WITNESS: usize = crate::witness::SHIP_YAPF_TRACK_LIMITS;

    fn base(&self) -> &CYapfBaseT<'v, Self::Key> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut CYapfBaseT<'v, Self::Key> {
        &mut self.base
    }
    /// `PfDetectDestination`: called by YAPF to detect if node ends in the
    /// desired destination.
    #[inline]
    fn pf_detect_destination(&self, n: &CYapfShipNode) -> bool {
        self.pf_detect_destination_tile(n.key.tile, n.key.td)
    }
    /// `PfFollowNode`: called by YAPF to move from the given node to the next
    /// tile. For each reachable trackdir on the new tile creates new node,
    /// initializes it and adds it to the open list by calling `AddNewNode`.
    #[inline]
    fn pf_follow_node(&mut self, old_node: usize) {
        crate::witness::hit(crate::witness::SHIP_YAPF_TRACK_NODES);
        let v = self.v();
        let old = self.base.nodes.items[old_node];
        // `CFollowTrackWater` reads nothing from its vehicle, so it follows without one.
        let mut follower = CFollowTrackT::vehicle(
            v.s,
            TransportType::TRANSPORT_WATER,
            None,
            RailTypes::INVALID_RAILTYPES,
            true,
        );
        if follower.follow(v.s, old.key.tile, old.key.td)
            && (self.water_region_corridor.is_empty() || {
                let region = get_water_region_info(v.world.dimensions, follower.new_tile);
                self.water_region_corridor.iter().any(|p| region == *p)
            })
        {
            self.add_multiple_nodes(old_node, &follower);
        }
    }
    /// `PfCalcCost`: called by YAPF to calculate the cost from the origin to
    /// the given node. Calculates only the cost of given node, adds it to the
    /// parent node cost and stores the result into `Node::cost` member.
    #[inline]
    fn pf_calc_cost(&self, n: &mut CYapfShipNode, follower: &CFollowTrackT) -> bool {
        let v = self.v();
        let world = &v.world;
        // `AddNewNode` has asserted the parent, which the original dereferences.
        let Some(parent) = n.parent else {
            crate::services_assert::not_reached()
        };
        let parent = self.base.nodes.items[parent];
        let tile = n.key.tile;
        let td = n.key.td;

        /* Base tile cost depending on distance. */
        let mut c = if is_diagonal_trackdir(td.0) {
            YAPF_TILE_LENGTH
        } else {
            YAPF_TILE_CORNER_LENGTH
        };
        /* Additional penalty for curves. */
        c = c.wrapping_add(self.curve_cost(parent.key.td, td));

        if world.is_docking_tile(tile) {
            /* Check docking tile for occupancy. */
            let mut count: u32 = 0;
            crate::movement::table(v.s)
                .vehicle()
                .for_each_on_tile(tile.0, |other| {
                    /* Ignore other vehicles (aircraft) and ships inside depot. */
                    if other.vehicle_type() == VehicleType::VEH_SHIP.0
                        && other.status() & crate::vehicle_base::VS_HIDDEN == 0
                    {
                        count += 1;
                    }
                });
            c = c.wrapping_add_unsigned(
                count
                    .wrapping_mul(3)
                    .wrapping_mul(YAPF_TILE_LENGTH.cast_unsigned()),
            );
        }

        /* Encourage separation between ships traveling in different directions. */
        if !is_preferred_ship_direction(world.dimensions, tile, td) {
            c = c.wrapping_add(YAPF_TILE_LENGTH);
        }

        /* Skipped tile cost for aqueducts. */
        c = c.wrapping_add(YAPF_TILE_LENGTH.wrapping_mul(follower.tiles_skipped));

        /* Ocean/canal speed penalty. */
        let speed_frac = if get_effective_water_class(world, tile) == WaterClass::Sea {
            v.input.ocean_speed_frac
        } else {
            v.input.canal_speed_frac
        };
        if speed_frac > 0 {
            let speed_frac = i32::from(speed_frac);
            c = c.wrapping_add(
                YAPF_TILE_LENGTH
                    .wrapping_mul(1_i32.wrapping_add(follower.tiles_skipped))
                    .wrapping_mul(speed_frac)
                    / (256 - speed_frac),
            );
        }

        /* Lock penalty. */
        if (v.s.water_middle_lock)(tile.0) {
            let canal_speed = apply_water_class_speed_frac(
                u32::from(v.input.max_speed),
                false,
                v.input.ocean_speed_frac,
                v.input.canal_speed_frac,
            );
            /* Cost is proportional to the vehicle's speed as the vehicle stops in the lock. */
            c = c.wrapping_add_unsigned(
                TILE_HEIGHT
                    .wrapping_mul(YAPF_TILE_LENGTH.cast_unsigned())
                    .wrapping_mul(canal_speed)
                    / 128,
            );
        }

        /* Apply it. */
        n.cost = parent.cost.wrapping_add(c);
        true
    }
    /// `PfCalcEstimate`: called by YAPF to calculate cost estimate. Calculates
    /// distance to the destination adds it to the actual cost from origin and
    /// stores the sum to the `Node::estimate`.
    #[inline]
    fn pf_calc_estimate(&self, n: &mut CYapfShipNode) -> bool {
        let v = self.v();
        let destination_tile = if self.has_intermediate_dest {
            self.intermediate_dest_tile
        } else {
            self.dest_tile
        };

        if self.pf_detect_destination(n) {
            n.estimate = n.cost;
            return true;
        }

        n.estimate = n.cost.wrapping_add(octile_distance_cost(
            v.world.dimensions,
            n.key.tile,
            n.key.td,
            destination_tile,
        ));
        v.world.assert_with(|| {
            n.parent
                .is_some_and(|parent| n.estimate >= self.base.nodes.items[parent].estimate)
        });
        true
    }
    /// `TransportTypeChar`: return debug report character to identify the
    /// transportation type.
    #[inline]
    fn transport_type_char(&self) -> char {
        'w'
    }
}

/// `GetRandomTrackdir`: returns a random trackdir out of a set of trackdirs.
fn get_random_trackdir(v: &Vehicle<'_>, mut trackdirs: TrackdirBits) -> Trackdir {
    crate::witness::hit(crate::witness::SHIP_YAPF_RANDOM_DRAWS);
    // RandomRange consumes one complete shared word, including for a zero maximum.
    let strip_amount = v.s.random_range(trackdirs.0.count_ones());
    for _ in 0..strip_amount {
        remove_first_trackdir(&mut trackdirs, |ok| v.check(ok));
    }
    Trackdir(find_first_trackdir(trackdirs.0))
}

/// `GetRandomFollowUpTileTrackdir`: returns a random tile/trackdir that can be
/// reached from the current tile/trackdir, or tile/`INVALID_TRACK` if none is
/// available.
fn get_random_follow_up_tile_trackdir(
    v: &Vehicle<'_>,
    tile: TileIndex,
    dir: Trackdir,
) -> (TileIndex, Trackdir) {
    // `CFollowTrackWater` reads nothing from its vehicle, so it follows without one.
    let mut follower = CFollowTrackT::vehicle(
        v.s,
        TransportType::TRANSPORT_WATER,
        None,
        RailTypes::INVALID_RAILTYPES,
        true,
    );
    if follower.follow(v.s, tile, dir) {
        let mut dirs = follower.new_td_bits;
        let dirs_without_90_degree = dirs & !TrackdirBits(trackdir_crosses_trackdirs(dir.0));
        if dirs_without_90_degree != TrackdirBits::TRACKDIR_BIT_NONE {
            dirs = dirs_without_90_degree;
        }
        return (follower.new_tile, get_random_trackdir(v, dirs));
    }
    (follower.new_tile, Trackdir::INVALID_TRACKDIR)
}

/// `CreateRandomPath`: creates a random path, avoids 90 degree turns.
fn create_random_path(
    v: &Vehicle<'_>,
    path_cache: &mut ShipPathCache,
    path_length: i32,
) -> Trackdir {
    let mut tile_dir = (TileIndex(v.input.tile), v.trackdir);
    for _ in 0..path_length {
        tile_dir = get_random_follow_up_tile_trackdir(v, tile_dir.0, tile_dir.1);
        if tile_dir.1 == Trackdir::INVALID_TRACKDIR {
            break;
        }
        path_cache.0.push(tile_dir.1);
    }

    if path_cache.0.is_empty() {
        return Trackdir::INVALID_TRACKDIR;
    }

    /* Reverse the path so we can take from the end. */
    path_cache.0.reverse();

    let result = path_cache.back();
    path_cache.0.pop();
    result
}

/// `ChooseShipTrack`.
pub(crate) fn choose_ship_track(
    v: &Vehicle<'_>,
    tile: TileIndex,
    forward_dirs: TrackdirBits,
    reverse_dirs: TrackdirBits,
    path_found: &mut bool,
    path_cache: &mut ShipPathCache,
    best_origin_dir: &mut Trackdir,
) -> Trackdir {
    let high_level_path = find_water_region_path(v, tile, NUMBER_OR_WATER_REGIONS_LOOKAHEAD + 1);
    if high_level_path.is_empty() {
        *path_found = false;
        crate::witness::hit(crate::witness::SHIP_YAPF_LOST);
        /* Make the ship move around aimlessly. This prevents repeated pathfinder calls and clearly indicates that the ship is lost. */
        return create_random_path(v, path_cache, SHIP_LOST_PATH_LENGTH);
    }

    /* Try one time without restricting the search area, which generally results in better and more natural looking paths.
     * However the pathfinder can hit the node limit in certain situations such as long aqueducts or maze-like terrain.
     * If that happens we run the pathfinder again, but restricted only to the regions provided by the region pathfinder. */
    for attempt in 0..2 {
        let mut pf = CYapfShip::new(v, MAX_SHIP_PF_NODES);

        /* Set origin and destination nodes */
        pf.set_origin(TileIndex(v.input.tile), forward_dirs | reverse_dirs);
        pf.set_destination();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "the original's static_cast<int> of the path's size"
        )]
        let is_intermediate_destination =
            high_level_path.len() as i32 >= NUMBER_OR_WATER_REGIONS_LOOKAHEAD + 1;
        if is_intermediate_destination {
            pf.set_intermediate_destination(high_level_path[high_level_path.len() - 1]);
        }

        /* Restrict the search area to prevent the low level pathfinder from expanding too many nodes. This can happen
         * when the terrain is very "maze-like" or when the high level path "teleports" via a very long aqueduct. */
        if attempt > 0 {
            pf.restrict_search(&high_level_path);
        }

        /* Find best path. */
        *path_found = pf.find_path();
        let node = pf.get_best_node();
        if attempt == 0 && !*path_found {
            continue; // Try again with restricted search area.
        }

        /* Make the ship move around aimlessly. This prevents repeated pathfinder calls and clearly indicates that the ship is lost.
         * `GetBestNode` has a node exactly when the path was found. */
        let Some(mut node) = node else {
            crate::witness::hit(crate::witness::SHIP_YAPF_LOST);
            return create_random_path(v, path_cache, SHIP_LOST_PATH_LENGTH);
        };
        let items = &pf.base.nodes.items;
        if pf.dest_station != StationID::INVALID
            && !is_intermediate_destination
            && items[node].key.tile != pf.dest_tile
        {
            crate::witness::hit(crate::witness::SHIP_YAPF_ALTERNATE_DOCKING);
        }

        /* Return only the path within the current water region if an intermediate destination was returned. If not, cache the entire path
         * to the final destination tile. The low-level pathfinder might actually prefer a different docking tile in a nearby region. Without
         * caching the full path the ship can get stuck in a loop. */
        let end_water_patch = v.get_water_region_patch_info(items[node].key.tile);
        v.world
            .assert_with(|| v.get_water_region_patch_info(tile) == high_level_path[0]);
        let start_water_patch = high_level_path[0];
        while let Some(parent) = items[node].parent {
            let node_water_patch = v.get_water_region_patch_info(items[node].key.tile);

            let node_water_patch_on_high_level_path = high_level_path.contains(&node_water_patch);
            let add_full_path = !is_intermediate_destination && node_water_patch != end_water_patch;

            /* The cached path must always lead to a region patch that's on the high level path.
             * This is what can happen when that's not the case https://github.com/OpenTTD/OpenTTD/issues/12176. */
            if add_full_path
                || !node_water_patch_on_high_level_path
                || node_water_patch == start_water_patch
            {
                path_cache.0.push(items[node].key.td);
            } else {
                crate::witness::hit(crate::witness::SHIP_YAPF_CACHE_TRUNCATIONS);
                path_cache.0.clear();
            }
            node = parent;
        }
        v.check(items[node].key.tile == TileIndex(v.input.tile));

        /* Return INVALID_TRACKDIR to trigger a ship reversal if that is the best option. */
        *best_origin_dir = items[node].key.td;
        if (TrackdirBits(trackdir_to_trackdir_bits(best_origin_dir.0)) & forward_dirs)
            == TrackdirBits::TRACKDIR_BIT_NONE
        {
            crate::witness::hit(crate::witness::SHIP_YAPF_REVERSE_ORIGINS);
            path_cache.0.clear();
            return Trackdir::INVALID_TRACKDIR;
        }

        /* A empty path means we are already at the destination. The pathfinder shouldn't have been called at all.
         * Return a random reachable trackdir to hopefully nudge the ship out of this strange situation. */
        if path_cache.0.is_empty() {
            return create_random_path(v, path_cache, 1);
        }

        /* Take out the last trackdir as the result. */
        let result = path_cache.back();
        path_cache.0.pop();

        /* Clear path cache when in final water region patch. This is to allow ships to spread over different docking tiles dynamically. */
        if start_water_patch == end_water_patch {
            crate::witness::hit(crate::witness::SHIP_YAPF_FINAL_REGION_CLEARS);
            path_cache.0.clear();
        }

        return result;
    }

    crate::services_assert::not_reached()
}

/// `CheckShipReverse`: check whether a ship should reverse to reach its
/// destination. Called when leaving depot. `trackdir` receives the best of all
/// possible reversed trackdirs. Returns true if the reverse direction is better.
pub(crate) fn check_ship_reverse(v: &Vehicle<'_>, trackdir: Option<&mut Trackdir>) -> bool {
    let mut path_found = false;
    let mut dummy_cache = ShipPathCache::default();
    let mut best_origin_dir = Trackdir::INVALID_TRACKDIR;

    let Some(trackdir) = trackdir else {
        crate::witness::hit(crate::witness::SHIP_YAPF_REVERSE_CALLS);
        /* The normal case, typically called when ships leave a dock. */
        let reverse_dir = Trackdir(reverse_trackdir(v.trackdir.0));
        let forward_dirs = TrackdirBits(trackdir_to_trackdir_bits(v.trackdir.0));
        let reverse_dirs = TrackdirBits(trackdir_to_trackdir_bits(reverse_dir.0));
        choose_ship_track(
            v,
            TileIndex(v.input.tile),
            forward_dirs,
            reverse_dirs,
            &mut path_found,
            &mut dummy_cache,
            &mut best_origin_dir,
        );
        let reverse = path_found && best_origin_dir == reverse_dir;
        if reverse {
            crate::witness::hit(crate::witness::SHIP_YAPF_REVERSE_CHOSEN);
        }
        return reverse;
    };

    crate::witness::hit(crate::witness::SHIP_YAPF_BLOCKED_CALLS);
    /* This gets called when a ship suddenly can't move forward, e.g. due to terraforming. */
    let check = |ok| v.check(ok);
    let entry = reverse_diag_dir(
        vehicle_exit_dir(Direction(v.input.direction), v.state, check),
        check,
    );
    let reverse_dirs = TrackdirBits(
        diagdir_reaches_trackdirs(entry.0)
            & track_status_to_trackdir_bits(
                crate::movement::table(v.s).get_tile_track_status_in_world(
                    &v.world,
                    TileIndex(v.input.tile),
                    TransportType::TRANSPORT_WATER,
                    0,
                    entry,
                ),
            ),
    );
    choose_ship_track(
        v,
        TileIndex(v.input.tile),
        TrackdirBits::TRACKDIR_BIT_NONE,
        reverse_dirs,
        &mut path_found,
        &mut dummy_cache,
        &mut best_origin_dir,
    );
    *trackdir = if path_found && best_origin_dir != Trackdir::INVALID_TRACKDIR {
        best_origin_dir
    } else {
        crate::witness::hit(crate::witness::SHIP_YAPF_BLOCKED_FALLBACKS);
        get_random_trackdir(v, reverse_dirs)
    };
    true
}

/// `YapfShipChooseTrack`: ship controller helper - path finder invoker.
pub(crate) fn yapf_ship_choose_track(
    s: &Services,
    v: &Ship<'_>,
    tile: TileIndex,
    path_found: &mut bool,
    path_cache: &mut ShipPathCache,
) -> Track {
    crate::witness::hit(crate::witness::SHIP_YAPF_CHOOSE_CALLS);
    let v = Vehicle::new(s, ship_input(s, v), v.state, v.trackdir, v.leaves);
    let mut best_origin_dir = Trackdir::INVALID_TRACKDIR;
    let origin_dirs = TrackdirBits(trackdir_to_trackdir_bits(v.trackdir.0));
    let td_ret = choose_ship_track(
        &v,
        tile,
        origin_dirs,
        TrackdirBits::TRACKDIR_BIT_NONE,
        path_found,
        path_cache,
        &mut best_origin_dir,
    );
    if td_ret != Trackdir::INVALID_TRACKDIR {
        Track(trackdir_to_track(td_ret.0))
    } else {
        Track::INVALID_TRACK
    }
}

/// `YapfShipCheckReverse`.
pub(crate) fn yapf_ship_check_reverse(
    s: &Services,
    v: &Ship<'_>,
    trackdir: Option<&mut Trackdir>,
) -> bool {
    let v = Vehicle::new(s, ship_input(s, v), v.state, v.trackdir, v.leaves);
    check_ship_reverse(&v, trackdir)
}

/// `WaterRegionPatchKey`: yapf node key that represents a single patch of
/// interconnected water within a water region. It compares and hashes by
/// `CalcHash`, which `Set` computes once: `Map::SizeX()` cannot change during
/// a search.
#[derive(Clone, Copy)]
struct WaterRegionPatchKey {
    water_region_patch: WaterRegionPatchDesc,
    hash: i32,
}
impl WaterRegionPatchKey {
    /// `Set`.
    #[inline]
    fn set(map: crate::map_func::Map, water_region_patch: WaterRegionPatchDesc) -> Self {
        Self {
            water_region_patch,
            hash: calculate_water_region_patch_hash(map, water_region_patch),
        }
    }
}
/// `operator==`: compares `CalcHash`.
impl PartialEq for WaterRegionPatchKey {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl Eq for WaterRegionPatchKey {}
impl Hash for WaterRegionPatchKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.hash);
    }
}

static WATER_REGION_NODES: GameState<Option<NodeList<WaterRegionPatchKey>>> = GameState::new(None);
impl NodeKey for WaterRegionPatchKey {
    fn scratch() -> &'static GameState<Option<NodeList<Self>>> {
        &WATER_REGION_NODES
    }
}

/// `ManhattanDistance`.
#[inline]
fn manhattan_distance(a: &WaterRegionPatchKey, b: &WaterRegionPatchKey) -> u32 {
    ((a.water_region_patch.x - b.water_region_patch.x).abs()
        + (a.water_region_patch.y - b.water_region_patch.y).abs())
    .wrapping_mul(DIRECT_NEIGHBOUR_COST)
    .cast_unsigned()
}

/// `WaterRegionNode`: yapf node for water regions.
type WaterRegionNode = Node<WaterRegionPatchKey>;

/// `YapfShipRegions`: water region based YAPF implementation for ships.
struct YapfShipRegions<'v> {
    base: CYapfBaseT<'v, WaterRegionPatchKey>,
    dest: WaterRegionPatchKey,
}
impl<'v> YapfShipRegions<'v> {
    fn new(v: &'v Vehicle<'v>, max_nodes: i32) -> Self {
        Self {
            base: CYapfBaseT::new(v, max_nodes),
            dest: WaterRegionPatchKey {
                water_region_patch: WaterRegionPatchDesc {
                    x: 0,
                    y: 0,
                    label: INVALID_WATER_REGION_PATCH,
                },
                hash: 0,
            },
        }
    }
    /// `WaterRegionNode::GetDiagDirFromParent`.
    #[inline]
    fn get_diag_dir_from_parent(&self, n: &WaterRegionNode) -> DiagDirection {
        let Some(parent) = n.parent else {
            return DiagDirection::INVALID_DIAGDIR;
        };
        let parent = &self.base.nodes.items[parent];
        let dx = n.key.water_region_patch.x - parent.key.water_region_patch.x;
        let dy = n.key.water_region_patch.y - parent.key.water_region_patch.y;
        if dx > 0 && dy == 0 {
            return DiagDirection::DIAGDIR_SW;
        }
        if dx < 0 && dy == 0 {
            return DiagDirection::DIAGDIR_NE;
        }
        if dx == 0 && dy > 0 {
            return DiagDirection::DIAGDIR_SE;
        }
        if dx == 0 && dy < 0 {
            return DiagDirection::DIAGDIR_NW;
        }
        DiagDirection::INVALID_DIAGDIR
    }
    /// `AddOrigin`.
    fn add_origin(&mut self, water_region_patch: WaterRegionPatchDesc) {
        if water_region_patch.label == INVALID_WATER_REGION_PATCH {
            return;
        }
        if !self.has_origin(water_region_patch) {
            let key =
                WaterRegionPatchKey::set(self.base.vehicle.world.dimensions, water_region_patch);
            self.add_startup_node(WaterRegionNode::set(None, key));
        }
    }
    /// `HasOrigin`. The original keeps `origin_keys`, the keys `AddOrigin` has
    /// added; until the search runs the open list holds exactly those keys.
    fn has_origin(&self, water_region_patch: WaterRegionPatchDesc) -> bool {
        let key = WaterRegionPatchKey::set(self.base.vehicle.world.dimensions, water_region_patch);
        self.base.nodes.find_open_node(&key).is_some()
    }
    /// `SetDestination`.
    fn set_destination(&mut self, water_region_patch: WaterRegionPatchDesc) {
        self.dest =
            WaterRegionPatchKey::set(self.base.vehicle.world.dimensions, water_region_patch);
    }
}
impl<'v> Yapf<'v> for YapfShipRegions<'v> {
    type Key = WaterRegionPatchKey;
    /// We don't need a follower but YAPF requires one.
    type TrackFollower = ();
    const NODE_LIMIT_WITNESS: usize = crate::witness::SHIP_YAPF_REGION_LIMITS;

    fn base(&self) -> &CYapfBaseT<'v, Self::Key> {
        &self.base
    }
    fn base_mut(&mut self) -> &mut CYapfBaseT<'v, Self::Key> {
        &mut self.base
    }
    /// `PfFollowNode`: each neighbour of the visit becomes a new node.
    #[inline]
    fn pf_follow_node(&mut self, old_node: usize) {
        crate::witness::hit(crate::witness::SHIP_YAPF_REGION_NODES);
        let v = self.base.vehicle;
        let mut visit = WaterVisit::new();
        loop {
            // The visit reads the followed node's patch as it is now, as the
            // original's const reference does.
            let water_region_patch = self.base.nodes.items[old_node].key.water_region_patch;
            // SAFETY: As the module contract.
            let Some(next) = visit_water_region_patch_neighbours(
                unsafe { &mut *v.regions },
                v.s,
                &mut visit,
                water_region_patch,
            ) else {
                break;
            };
            let key = WaterRegionPatchKey::set(v.world.dimensions, next);
            self.add_new_node(WaterRegionNode::set(Some(old_node), key), &());
        }
    }
    /// `PfDetectDestination`.
    #[inline]
    fn pf_detect_destination(&self, n: &WaterRegionNode) -> bool {
        n.key == self.dest
    }
    /// `PfCalcCost`.
    #[inline]
    fn pf_calc_cost(&self, n: &mut WaterRegionNode, (): &()) -> bool {
        // `AddNewNode` has asserted the parent, which the original dereferences.
        let Some(parent) = n.parent else {
            crate::services_assert::not_reached()
        };
        let parent = &self.base.nodes.items[parent];
        n.cost = parent
            .cost
            .wrapping_add_unsigned(manhattan_distance(&n.key, &parent.key));

        /* Incentivise zigzagging by adding a slight penalty when the search continues in the same direction. */
        if parent.parent.is_some() {
            let v = self.base.vehicle;
            let dir_diff = diag_dir_difference(
                self.get_diag_dir_from_parent(parent),
                self.get_diag_dir_from_parent(n),
                |ok| v.check(ok),
            );
            if dir_diff != DiagDirDiff::DIAGDIRDIFF_90LEFT
                && dir_diff != DiagDirDiff::DIAGDIRDIFF_90RIGHT
            {
                n.cost = n.cost.wrapping_add(1);
            }
        }

        true
    }
    /// `PfCalcEstimate`.
    #[inline]
    fn pf_calc_estimate(&self, n: &mut WaterRegionNode) -> bool {
        if self.pf_detect_destination(n) {
            n.estimate = n.cost;
            return true;
        }

        n.estimate = n
            .cost
            .wrapping_add_unsigned(manhattan_distance(&n.key, &self.dest));

        true
    }
    #[inline]
    fn transport_type_char(&self) -> char {
        '^'
    }
}

/// `FindWaterRegionPath`: finds a path at the water region level. Note that
/// the starting region is always included if the path was found. Returns a
/// path of water region patches, or an empty vector if no path was found.
pub(crate) fn find_water_region_path(
    v: &Vehicle<'_>,
    start_tile: TileIndex,
    max_returned_path_length: i32,
) -> Vec<WaterRegionPatchDesc> {
    let start_water_region_patch = v.get_water_region_patch_info(start_tile);

    /* We reserve 4 nodes (patches) per water region. The vast majority of water regions have 1 or 2 regions so this should be a pretty
     * safe limit. We cap the limit at 65536 which is at a region size of 16x16 is equivalent to one node per region for a 4096x4096 map. */
    let node_limit = (v
        .world
        .types
        .count()
        .wrapping_mul(NODES_PER_REGION.cast_unsigned())
        .cast_signed()
        / WATER_REGION_NUMBER_OF_TILES)
        .min(MAX_NUMBER_OF_NODES);
    let mut pf = YapfShipRegions::new(v, node_limit);
    pf.set_destination(start_water_region_patch);

    if OrderType(v.input.order_type) == OrderType::OT_GOTO_STATION {
        let station_id = StationID(v.input.order_destination);
        let station = (v.s.base_station)(station_id.0);
        // SAFETY: The shared table returns the static station command table.
        let commands = unsafe { &*(v.s.station_commands)() };
        for tile in (commands.tile_area)(station, StationType::Dock.0).tiles(v.world.dimensions) {
            if v.world.is_docking_tile(tile) && is_ship_destination_tile(v.leaves, tile, station_id)
            {
                pf.add_origin(v.get_water_region_patch_info(tile));
            }
        }
    } else {
        let tile = TileIndex(v.input.dest_tile);
        pf.add_origin(v.get_water_region_patch_info(tile));
    }

    /* If origin and destination are the same we simply return that water patch. */
    let mut path = vec![start_water_region_patch];
    path.reserve(usize::try_from(max_returned_path_length).unwrap_or(0));
    if pf.has_origin(start_water_region_patch) {
        return path;
    }

    /* Find best path. */
    if !pf.find_path() {
        return Vec::new(); // Path not found.
    }

    let items = &pf.base.nodes.items;
    let mut node = pf.get_best_node();
    for _ in 0..max_returned_path_length - 1 {
        if let Some(n) = node {
            node = items[n].parent;
            if let Some(parent) = node {
                path.push(items[parent].key.water_region_patch);
            }
        }
    }

    v.check(!path.is_empty());
    path
}

// SAFETY for the path cache exports: each owner pointer came from
// `openttd_rust_ship_path_new` or `_clone`, is live and exclusive for the call,
// and is destroyed once; an index is below the cache's size.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ship_path_new() -> *mut ShipPathCache {
    Box::into_raw(Box::default())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_clone(
    p: *const ShipPathCache,
) -> *mut ShipPathCache {
    Box::into_raw(Box::new(unsafe { &*p }.clone()))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_destroy(p: *mut ShipPathCache) {
    if !p.is_null() {
        drop(unsafe { Box::from_raw(p) });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_size(p: *const ShipPathCache) -> usize {
    unsafe { &*p }.0.len()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_get(p: *const ShipPathCache, index: usize) -> u8 {
    unsafe { &*p }.0[index].0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_set(p: *mut ShipPathCache, index: usize, td: u8) {
    unsafe { &mut *p }.0[index] = Trackdir(td);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_push(p: *mut ShipPathCache, td: u8) {
    unsafe { &mut *p }.0.push(Trackdir(td));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_pop(p: *mut ShipPathCache) {
    unsafe { &mut *p }.0.pop();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ship_path_clear(p: *mut ShipPathCache) {
    unsafe { &mut *p }.0.clear();
}

/// `YapfShipChooseTrack` for the C++ ship controller, which passes the ship's
/// `GetState()` and `GetVehicleTrackdir()`, its path cache and `path_found`.
///
/// # Safety
/// The caller holds the game-state mutex and passes the live shared table, the
/// controller's leaves, the ship's live record and its sole path cache, and a
/// writable `path_found`. No service reenters the cache. No pointer escapes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_yapf_ship_choose_track(
    shared: *const Services,
    ship_leaves: *const Leaves,
    v: *const OpenTTDVehicle,
    state: u8,
    trackdir: u8,
    tile: u32,
    path_cache: *mut ShipPathCache,
    path_found: *mut bool,
) -> u8 {
    // SAFETY: As this function's contract.
    let (s, leaves, path_cache, path_found) =
        unsafe { (&*shared, &*ship_leaves, &mut *path_cache, &mut *path_found) };
    let ship = Ship {
        // SAFETY: The caller passes a live non-null vehicle record.
        v: VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(v.cast_mut()) }),
        state: TrackBits(state),
        trackdir: Trackdir(trackdir),
        leaves,
    };
    yapf_ship_choose_track(s, &ship, TileIndex(tile), path_found, path_cache).0
}

/// `YapfShipCheckReverse` for the C++ ship controller. `trackdir_out` is the
/// original's `trackdir`, which may be null.
///
/// # Safety
/// As [`openttd_rust_yapf_ship_choose_track`]; `trackdir_out` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_yapf_ship_check_reverse(
    shared: *const Services,
    ship_leaves: *const Leaves,
    v: *const OpenTTDVehicle,
    state: u8,
    trackdir: u8,
    trackdir_out: *mut u8,
) -> u8 {
    // SAFETY: As this function's contract.
    let (s, leaves) = unsafe { (&*shared, &*ship_leaves) };
    let ship = Ship {
        // SAFETY: The caller passes a live non-null vehicle record.
        v: VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(v.cast_mut()) }),
        state: TrackBits(state),
        trackdir: Trackdir(trackdir),
        leaves,
    };
    let mut out = Trackdir::INVALID_TRACKDIR;
    let reverse = yapf_ship_check_reverse(s, &ship, (!trackdir_out.is_null()).then_some(&mut out));
    if !trackdir_out.is_null() {
        // SAFETY: As this function's contract.
        unsafe { trackdir_out.write(out.0) };
    }
    u8::from(reverse)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDShipYapfInput" ["probe"] => Input {
        tile,
        dest_tile,
        order_type,
        order_destination,
        direction,
        max_speed,
        ocean_speed_frac,
        canal_speed_frac,
        ship_curve45_penalty,
        ship_curve90_penalty,
        unit_number,
    };
}

#[cfg(test)]
mod tests {
    use super::{
        CYapfBaseT, CYapfShip, CYapfShipNode, Input, NodeList, Ship, Vehicle, Yapf, ship_input,
    };
    use crate::map_func::TileTypeAllocation;
    use crate::order_type::OrderType;
    use crate::services::Services;
    use crate::tile_type::TileIndex;
    use crate::track_type::{TrackBits, Trackdir};
    use crate::vehicle::VehicleRef;
    use crate::yapf_node::CYapfNodeKeyExitDir;
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
        static WATER: crate::water::WaterServices = crate::water::fixture();
    }
    extern "C" fn record_assertion(okay: bool) {
        if !okay {
            FAILED_ASSERTIONS.set(FAILED_ASSERTIONS.get() + 1);
        }
    }
    extern "C" fn water_services() -> *const crate::water::WaterServices {
        WATER.with(ptr::from_ref)
    }

    /// A 64 by 64 sea, with recorded native assertions and the fixture's water
    /// region owner. The tables live for the test process.
    fn shared(vehicles: &'static crate::vehicle::VehicleServices) -> Services {
        let base = Box::leak(
            (0..64 * 64)
                .map(|_| crate::map_func::BaseTileLayout {
                    kind: crate::tile_type::MP_WATER << 4,
                    height: 0,
                    m2: 0,
                    m1: 0,
                    m3: 0,
                    m4: 0,
                    m5: 0,
                })
                .collect::<Box<[_]>>(),
        );
        let extended = Box::leak(
            (0..64 * 64)
                .map(|_| crate::map_func::ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect::<Box<[_]>>(),
        );
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: base.as_ptr(),
                extended: extended.as_ptr(),
                count: 64 * 64,
                assertions: true,
            },
            6,
        );
        FAILED_ASSERTIONS.set(0);
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            native_assert: record_assertion,
            vehicle: vehicles,
            ..crate::test_tables::MOVEMENT
        }));
        let s = Services {
            movement,
            water_services,
            ..crate::services::fixture(ptr::null_mut(), unexpected_random, unexpected_industry)
        };
        movement.shared = Box::leak(Box::new(s));
        s
    }
    fn input() -> Input {
        Input {
            tile: 0,
            dest_tile: 0,
            order_type: OrderType::OT_NOTHING.0,
            order_destination: 0,
            direction: 0,
            max_speed: 0,
            ocean_speed_frac: 0,
            canal_speed_frac: 0,
            ship_curve45_penalty: 0,
            ship_curve90_penalty: 0,
            unit_number: 0,
        }
    }
    fn vehicle(s: &Services) -> Vehicle<'_> {
        Vehicle::new(
            s,
            input(),
            TrackBits::TRACK_BIT_NONE,
            Trackdir::TRACKDIR_X_NE,
            &crate::test_tables::SHIP_LEAVES,
        )
    }

    thread_local! {
        static ENGINE: Cell<*mut crate::engine::Lifecycle> = const { Cell::new(ptr::null_mut()) };
    }
    extern "C" fn engine(_: VehicleRef) -> crate::vehicle::EngineRef {
        crate::vehicle::EngineRef::of(ENGINE.get())
    }
    unsafe extern "C" fn unit(_: *mut crate::vehicle::OpenTTDVehicle) -> u16 {
        4242
    }

    #[test]
    fn ship_input_reads_each_setting_and_vehicle_fact_into_its_own_field() {
        // Two distinct penalties, so a swapped pair fails, which the play
        // scenarios cannot show at default settings.
        let penalties = [201_u32, 202];
        let settings = crate::settings_type::SettingAddresses {
            ship_curve45_penalty: &raw const penalties[0],
            ship_curve90_penalty: &raw const penalties[1],
            ..crate::settings_type::SettingAddresses::NULL
        };
        let newgrf = Box::leak(Box::new(std::mem::MaybeUninit::<
            crate::newgrf_vehicle::Services,
        >::uninit()));
        // SAFETY: Only `unit` is read from this table.
        unsafe { (&raw mut (*newgrf.as_mut_ptr()).unit).write(unit) };
        let newgrf: *const crate::newgrf_vehicle::Services = newgrf.as_ptr();
        thread_local! {
            static NEWGRF: Cell<*const crate::newgrf_vehicle::Services> = const { Cell::new(ptr::null()) };
        }
        NEWGRF.set(newgrf);
        extern "C" fn newgrf_services() -> *const crate::newgrf_vehicle::Services {
            NEWGRF.get()
        }
        let vehicles = Box::leak(Box::new(crate::vehicle::VehicleServices {
            engine,
            newgrf: newgrf_services,
            ..crate::test_tables::VEHICLE
        }));
        let s = Services {
            settings: &raw const settings,
            ..shared(vehicles)
        };
        let mut lifecycle = crate::engine::Lifecycle::default();
        lifecycle.vehicle_info_type = crate::vehicle::VehicleType::SHIP;
        lifecycle.vehicle_info.ship = crate::engine::records::ShipVehicleInfo {
            max_speed: 301,
            ocean_speed_frac: 17,
            canal_speed_frac: 23,
            ..Default::default()
        };
        ENGINE.set(&raw mut lifecycle);
        let mut orders = crate::orders::VehicleOrders {
            current: std::mem::MaybeUninit::new(crate::orders::Order {
                kind: OrderType::OT_GOTO_STATION.0 | 0x40,
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
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_SHIP,
        );
        // SAFETY: The record is live and owned by this test.
        unsafe {
            (*record).tile = TileIndex(1111);
            (*record).dest_tile = TileIndex(2222);
            (*record).direction = 5;
            (*record).orders = &raw mut orders;
        }
        let ship = Ship {
            v: VehicleRef(NonNull::new(record).unwrap()),
            state: TrackBits::TRACK_BIT_Y,
            trackdir: Trackdir::TRACKDIR_Y_SE,
            leaves: &crate::test_tables::SHIP_LEAVES,
        };
        let input = ship_input(&s, &ship);
        assert_eq!(
            (input.tile, input.dest_tile),
            (1111, 2222),
            "tile and destination"
        );
        assert_eq!(
            (input.order_type, input.order_destination),
            (OrderType::OT_GOTO_STATION.0, 4321),
            "the current order's type and destination"
        );
        assert_eq!(input.direction, 5, "direction");
        assert_eq!(
            (
                input.max_speed,
                input.ocean_speed_frac,
                input.canal_speed_frac
            ),
            (301, 17, 23),
            "the engine's speed and its ocean and canal fractions"
        );
        assert_eq!(
            (input.ship_curve45_penalty, input.ship_curve90_penalty),
            (201, 202),
            "the 45 and 90 degree curve penalties"
        );
        assert_eq!(input.unit_number, 4242, "unit number");
        ENGINE.set(ptr::null_mut());
        // SAFETY: The record came from the allocation export and is released once.
        unsafe { crate::vehicle_base::openttd_rust_vehicle_record_delete(record) };
    }

    #[test]
    fn lowered_child_estimate_records_pf_calc_estimate_assertion() {
        let _scratch = scratch_lock();
        let s = shared(&crate::test_tables::VEHICLE);
        let v = vehicle(&s);
        let mut pf = CYapfShip::new(&v, 0);
        pf.dest_tile = TileIndex(40 + 40 * 64);
        let mut parent = CYapfShipNode::set(
            None,
            CYapfNodeKeyExitDir::set(TileIndex(0), Trackdir::TRACKDIR_X_NE),
        );
        parent.estimate = 10_000;
        pf.base.nodes.items.push(parent);
        let mut child = CYapfShipNode::set(
            Some(0),
            CYapfNodeKeyExitDir::set(TileIndex(39 + 40 * 64), Trackdir::TRACKDIR_X_SW),
        );
        child.cost = 100;
        assert!(pf.pf_calc_estimate(&mut child));
        assert_eq!(FAILED_ASSERTIONS.get(), 1);

        // An estimate at or above the parent's passes.
        pf.base.nodes.items[0].estimate = child.estimate;
        assert!(pf.pf_calc_estimate(&mut child));
        assert_eq!(FAILED_ASSERTIONS.get(), 1);
    }

    #[test]
    fn invalid_trackdir_records_curve_cost_assertion() {
        let _scratch = scratch_lock();
        let s = shared(&crate::test_tables::VEHICLE);
        let v = vehicle(&s);
        let pf = CYapfShip::new(&v, 0);
        assert_eq!(
            pf.curve_cost(Trackdir::TRACKDIR_X_NE, Trackdir::TRACKDIR_X_NE),
            0
        );
        assert_eq!(FAILED_ASSERTIONS.get(), 0);
        // Trackdir 7 is not valid: `CurveCost` asserts the second trackdir, and
        // the `HasTrackdir` it then calls asserts it again.
        pf.curve_cost(Trackdir::TRACKDIR_X_NE, Trackdir(7));
        assert_eq!(FAILED_ASSERTIONS.get(), 2);
    }

    const NOT_REACHED_EXIT: i32 = 87;
    extern "C" fn not_reached() {
        std::process::exit(NOT_REACHED_EXIT);
    }
    extern "C" fn no_assertion(_: bool) {}

    #[test]
    fn closed_node_bettered_by_a_new_node_takes_the_native_unreachable_path() {
        // The native unreachable path ends the process, so the search runs in a
        // child whose `NOT_REACHED` service exits with a code the parent checks.
        const NAME: &str = "ship_yapf::tests::closed_node_bettered_by_a_new_node_takes_the_native_unreachable_path";
        const CHILD: &str = "OPENTTD_SHIP_YAPF_NOT_REACHED_CHILD";
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
            ..shared(&crate::test_tables::VEHICLE)
        };
        // SAFETY: `s` outlives the child process's only search.
        unsafe { crate::services_assert::openttd_rust_assert_bind(no_assertion, &raw const s) };
        let v = vehicle(&s);
        let mut pf = CYapfShip::new(&v, 0);
        let destination = TileIndex(10 + 10 * 64);
        pf.dest_tile = destination;
        // A closed node keyed by the destination tile and the north-east exit.
        let mut closed = CYapfShipNode::set(
            None,
            CYapfNodeKeyExitDir::set(destination, Trackdir::TRACKDIR_X_NE),
        );
        closed.estimate = 100_000;
        pf.base.nodes.items.push(closed);
        pf.base.nodes.insert_closed_node(&v.world, 0);
        // A node with the same key, whose estimate betters the closed node's.
        let follower = crate::follow_track::CFollowTrackT::vehicle(
            &s,
            crate::transport_type::TransportType::TRANSPORT_WATER,
            None,
            crate::rail_type::RailTypes::INVALID_RAILTYPES,
            true,
        );
        pf.add_new_node(
            CYapfShipNode::set(
                Some(0),
                CYapfNodeKeyExitDir::set(destination, Trackdir::TRACKDIR_X_NE),
            ),
            &follower,
        );
    }

    /// The node list scratch is process-wide: the tests that build a search
    /// take it in turn, as serialized production entries do.
    fn scratch_lock() -> std::sync::MutexGuard<'static, ()> {
        static SCRATCH: std::sync::Mutex<()> = std::sync::Mutex::new(());
        SCRATCH
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn fill(nodes: &mut NodeList<CYapfNodeKeyExitDir>, world: &crate::map_func::World<'_>) {
        for tile in 0..64 {
            let item = nodes.items.len();
            nodes.items.push(CYapfShipNode::set(
                None,
                CYapfNodeKeyExitDir::set(TileIndex(tile), Trackdir::TRACKDIR_X_NE),
            ));
            nodes.insert_open_node(world, item);
        }
        let first = nodes.items[0].key;
        nodes.pop_open_node(world, &first);
        nodes.insert_closed_node(world, 0);
    }
    fn capacities(nodes: &NodeList<CYapfNodeKeyExitDir>) -> (usize, usize, usize, usize) {
        (
            nodes.items.capacity(),
            nodes.open_nodes.capacity(),
            nodes.closed_nodes.capacity(),
            nodes.open_queue.capacity(),
        )
    }

    #[test]
    fn scratch_retains_capacity_across_nested_searches_and_threads() {
        let _scratch = scratch_lock();
        let s = shared(&crate::test_tables::VEHICLE);
        let v = vehicle(&s);
        let mut outer = CYapfBaseT::<CYapfNodeKeyExitDir>::new(&v, 7);
        fill(&mut outer.nodes, &v.world);
        let retained = capacities(&outer.nodes);
        let items = outer.nodes.items.as_ptr() as usize;
        // A search started meanwhile gets its own storage.
        let nested = CYapfBaseT::<CYapfNodeKeyExitDir>::new(&v, 1);
        assert!(nested.nodes.items.is_empty());
        assert_ne!(nested.nodes.items.as_ptr() as usize, items);
        drop(nested);
        assert_eq!(outer.nodes.items.len(), 64);
        drop(outer);
        // The next search, on another game thread, takes the outer search's
        // storage, emptied.
        std::thread::spawn(move || {
            let s = shared(&crate::test_tables::VEHICLE);
            let v = vehicle(&s);
            let mut next = CYapfBaseT::<CYapfNodeKeyExitDir>::new(&v, 2);
            assert_eq!(capacities(&next.nodes), retained);
            assert_eq!(next.nodes.items.as_ptr() as usize, items);
            assert!(
                next.nodes.items.is_empty()
                    && next.nodes.open_nodes.is_empty()
                    && next.nodes.closed_nodes.is_empty()
            );
            assert_eq!(next.nodes.get_best_open_node(), None);
            assert_eq!(next.max_search_nodes, 2);
            fill(&mut next.nodes, &v.world);
            assert_eq!(capacities(&next.nodes), retained);
        })
        .join()
        .unwrap();
    }
}
