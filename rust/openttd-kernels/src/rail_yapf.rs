/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The rail pathfinding (`pathfinder/yapf/yapf_rail.cpp`), with its cost
//! (`yapf_costrail.hpp`, `yapf_costbase.hpp`), destinations (`yapf_destrail.hpp`),
//! segment cost cache (`yapf_costcache.hpp`), node (`yapf_node_rail.hpp`),
//! two-way origin (`yapf_common.hpp`), main loop (`yapf_base.hpp`) and node
//! list (`nodelist.hpp`). Lookup maps are never iterated. Heap ties follow
//! `CBinaryHeapT`.
//!
//! Each entry is a synchronous call under the game-state mutex. A search loads
//! the map descriptor once, at entry: no service it calls loads, resets or
//! yields the map. The station triggers of a path reservation run `NewGRF`
//! callbacks, which cannot reach the search; still, no reference into the node
//! storage, the segment storage or the global caches is live across any
//! service call: callers copy what they need before the call and re-read after
//! it.
#![allow(unsafe_code)]
use crate::binaryheap::BinaryHeap;
use crate::bitmath_func::{find_first_bit, kill_first_bit, kill_first_bit_u16};
use crate::debug::debug;
use crate::direction_func::{diag_dir_to_axis, reverse_diag_dir};
use crate::direction_type::DiagDirection;
use crate::follow_track::{CFollowTrackT, ErrorCode, get_speed_limit};
use crate::game_state::GameState;
use crate::map_func::World;
use crate::node_hash::NodeMap;
use crate::order_type::{ODATF_NEAREST_DEPOT, OrderType};
use crate::pathfinder_func::calc_closest_station_tile;
use crate::pathfinder_type::{
    FindDepotData, FindDepotDataRecord, YAPF_INFINITE_PENALTY, YAPF_TILE_CORNER_LENGTH,
    YAPF_TILE_LENGTH,
};
use crate::pbs::{PBSTileInfo, Pbs};
use crate::rail_map::is_pbs_signal;
use crate::rail_type::{RailType, RailTypes};
use crate::services::Services;
use crate::settings_type::SettingAddresses;
use crate::signal_type::{SignalState, SignalType};
use crate::station_type::{SAT_PATH_RESERVATION, SRT_PATH_RESERVATION, StationID, StationType};
use crate::tile_type::{TILE_SIZE, TileIndex, TileType};
use crate::track_func::{
    diagdir_reaches_tracks, find_first_trackdir, has_trackdir, is_diagonal_trackdir,
    is_uphill_trackdir, is_valid_trackdir, next_trackdir, reverse_trackdir, track_overlaps_tracks,
    track_status_to_trackdir_bits, trackdir_crosses_trackdirs, trackdir_to_exitdir,
    trackdir_to_track,
};
use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
use crate::transport_type::TransportType;
use crate::vehicle::OpenTTDVehicle;
use crate::witness;
use crate::yapf_common::octile_distance_cost;
use crate::yapf_node::CYapfNodeKeyTrackDir;
use crate::yapf_type::{
    ESRF_ABORT_PF_MASK, ESRF_CACHED_MASK, ESRF_POSSIBLE_TARGET, EndSegmentReason, EndSegmentReasons,
};

/// `CYapfRailSegmentKey`: key for cached segment cost for rail YAPF.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CYapfRailSegmentKey(u32);
impl CYapfRailSegmentKey {
    /// `CYapfRailSegmentKey(const CYapfNodeKeyTrackDir &)`.
    #[inline]
    fn new(node_key: CYapfNodeKeyTrackDir) -> Self {
        Self((node_key.0.tile.0 << 4) | u32::from(node_key.0.td.0))
    }
}

/// `CYapfRailSegment`: cached segment cost for rail YAPF. Its `key` is the
/// global cache map's key, and only the map reads it.
#[derive(Clone, Copy)]
struct CYapfRailSegment {
    last_tile: TileIndex,
    last_td: Trackdir,
    cost: i32,
    last_signal_tile: TileIndex,
    last_signal_td: Trackdir,
    end_segment_reason: EndSegmentReasons,
}
impl CYapfRailSegment {
    #[inline]
    fn new() -> Self {
        Self {
            last_tile: TileIndex::INVALID,
            last_td: Trackdir::INVALID_TRACKDIR,
            cost: -1,
            last_signal_tile: TileIndex::INVALID,
            last_signal_td: Trackdir::INVALID_TRACKDIR,
            end_segment_reason: EndSegmentReasons::default(),
        }
    }
}

/// A node's `CYapfRailSegment *segment`: an index into the search's
/// `local_cache` or into its instantiation's global cache `heap`. Both only
/// grow while a search lives, so a reference stays valid as the original's
/// pointer into a `std::deque` does.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CachedDataRef {
    global: bool,
    index: u32,
}
impl CachedDataRef {
    /// `segment = nullptr`, which `Set` leaves until the cache fetch connects it.
    const NULL: Self = Self {
        global: false,
        index: u32::MAX,
    };
}

/// `CYapfRailNode::flags_u`: the `inherited_flags` word whose first three
/// bytes are `flags_s.target_seen`, `choice_seen` and `last_signal_was_red`.
#[repr(transparent)]
#[derive(Clone, Copy, Default)]
struct CYapfRailNodeFlags(u32);
impl CYapfRailNodeFlags {
    const TARGET_SEEN: u32 = 1;
    const CHOICE_SEEN: u32 = 1 << 8;
    const LAST_SIGNAL_WAS_RED: u32 = 1 << 16;
    #[inline]
    fn get(self, flag: u32) -> bool {
        self.0 & flag != 0
    }
    #[inline]
    fn set(&mut self, flag: u32, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }
}

/// `CYapfRailNode`: Yapf Node for rail YAPF. The intrusive hash link belongs
/// to the native hash table, and `is_choice` is unobserved.
#[derive(Clone, Copy)]
struct CYapfRailNode {
    key: CYapfNodeKeyTrackDir,
    parent: Option<usize>,
    cost: i32,
    estimate: i32,
    segment: CachedDataRef,
    num_signals_passed: u16,
    flags_u: CYapfRailNodeFlags,
    last_red_signal_type: SignalType,
    last_signal_type: SignalType,
}
impl CYapfRailNode {
    /// `CYapfRailNode::Set`, with the parent's index and value.
    #[inline]
    fn set(parent: Option<(usize, &Self)>, tile: TileIndex, td: Trackdir, is_choice: bool) -> Self {
        let mut n = match parent {
            None => Self {
                key: CYapfNodeKeyTrackDir::set(tile, td),
                parent: None,
                cost: 0,
                estimate: 0,
                segment: CachedDataRef::NULL,
                num_signals_passed: 0,
                flags_u: CYapfRailNodeFlags(0),
                last_red_signal_type: SignalType::SIGTYPE_BLOCK,
                /* We use PBS as initial signal type because if we are in
                 * a PBS section and need to route, i.e. we're at a safe
                 * waiting point of a station, we need to account for the
                 * reservation costs. If we are in a normal block then we
                 * should be alone in there and as such the reservation
                 * costs should be 0 anyway. If there would be another
                 * train in the block, i.e. passing signals at danger
                 * then avoiding that train with help of the reservation
                 * costs is not a bad thing, actually it would probably
                 * be a good thing to do. */
                last_signal_type: SignalType::SIGTYPE_PBS,
            },
            Some((index, parent)) => Self {
                key: CYapfNodeKeyTrackDir::set(tile, td),
                parent: Some(index),
                cost: 0,
                estimate: 0,
                segment: CachedDataRef::NULL,
                num_signals_passed: parent.num_signals_passed,
                flags_u: parent.flags_u,
                last_red_signal_type: parent.last_red_signal_type,
                last_signal_type: parent.last_signal_type,
            },
        };
        n.flags_u.set(
            CYapfRailNodeFlags::CHOICE_SEEN,
            n.flags_u.get(CYapfRailNodeFlags::CHOICE_SEEN) | is_choice,
        );
        n
    }
}

/// `NodeList<CYapfRailNode, 8, 10>`: hash table based node list
/// multi-container. Implements open list, closed list and priority queue for
/// A-star pathfinder.
struct NodeList {
    /// Storage of the nodes.
    items: Vec<CYapfRailNode>,
    /// Hash table of pointers to open nodes.
    open_nodes: NodeMap<CYapfNodeKeyTrackDir, usize>,
    /// Hash table of pointers to closed nodes.
    closed_nodes: NodeMap<CYapfNodeKeyTrackDir, usize>,
    /// Priority queue of pointers to open nodes.
    open_queue: BinaryHeap,
    /// New node under construction.
    new_node: Option<usize>,
}
impl NodeList {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            open_nodes: NodeMap::default(),
            closed_nodes: NodeMap::default(),
            open_queue: BinaryHeap::new(2048),
            new_node: None,
        }
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "HashTable::Count returns the original's int item count"
    )]
    fn open_count(&self) -> i32 {
        self.open_nodes.len() as i32
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "HashTable::Count returns the original's int item count"
    )]
    fn closed_count(&self) -> i32 {
        self.closed_nodes.len() as i32
    }
    /// `CreateNewNode`: allocate new data item from items. `Set` fills it.
    #[inline]
    fn create_new_node(&mut self) -> usize {
        if let Some(item) = self.new_node {
            return item;
        }
        let item = self.items.len();
        self.items.push(CYapfRailNode::set(
            None,
            TileIndex::INVALID,
            Trackdir::INVALID_TRACKDIR,
            false,
        ));
        self.new_node = Some(item);
        item
    }
    /// `GetBestOpenNode`: return the best open node.
    #[inline]
    fn get_best_open_node(&self) -> Option<usize> {
        self.open_queue.first()
    }
    /// `FindOpenNode`: return the open node specified by a key or nullptr if not found.
    #[inline]
    fn find_open_node(&self, key: CYapfNodeKeyTrackDir) -> Option<usize> {
        self.open_nodes.get(&key).copied()
    }
    /// `FindClosedNode`: return the closed node specified by a key or nullptr if not found.
    #[inline]
    fn find_closed_node(&self, key: CYapfNodeKeyTrackDir) -> Option<usize> {
        self.closed_nodes.get(&key).copied()
    }
}

/// `CSegmentCostCacheT<CYapfRailSegment>`: hash-map and storage (heap) of
/// segments. Nodes reference the segment that contains cached (or non-cached)
/// segment cost information. Segment key is always the same (tile and trackdir)
/// and represents the beginning of the segment.
struct CSegmentCostCacheT {
    map: NodeMap<CYapfRailSegmentKey, usize>,
    heap: Vec<CYapfRailSegment>,
}
impl CSegmentCostCacheT {
    const fn new() -> Self {
        Self {
            map: NodeMap::with_hasher(std::hash::BuildHasherDefault::new()),
            heap: Vec::new(),
        }
    }
    /// `Flush`: flush (clear) the cache.
    #[inline]
    fn flush(&mut self) {
        self.map.clear();
        self.heap.clear();
    }
    /// `Get`: the segment of `key`, created when the map lacks it.
    #[inline]
    fn get(&mut self, key: CYapfRailSegmentKey, found: &mut bool) -> usize {
        if let Some(&item) = self.map.get(&key) {
            *found = true;
            item
        } else {
            *found = false;
            let item = self.heap.len();
            self.heap.push(CYapfRailSegment::new());
            self.map.insert(key, item);
            item
        }
    }
}

/// One instantiation's `stGetGlobalCache` statics: `last_rail_change_counter`
/// and the cache `C`.
struct GlobalCache {
    last_rail_change_counter: i32,
    c: CSegmentCostCacheT,
}

/// `CSegmentCostCacheBase::s_rail_change_counter`: if any track changes, this
/// counter is incremented - that will invalidate segment cost cache.
static S_RAIL_CHANGE_COUNTER: GameState<i32> = GameState::new(0);

/// The six instantiations' global caches, indexed by [`Types::GLOBAL_CACHE`].
static GLOBAL_CACHES: GameState<[GlobalCache; 6]> = GameState::new(
    [const {
        GlobalCache {
            last_rail_change_counter: 0,
            c: CSegmentCostCacheT::new(),
        }
    }; 6],
);

/// `CYapfRail_TypesT`: what distinguishes the six rail pathfinder
/// instantiations, chosen at compile time as the original's templates are.
trait Types: Sized {
    /// `TrackFollower::Allow90degTurns()`.
    const ALLOW_90DEG_TURNS: bool;
    /// `TrackFollower::DoTrackMasking()`, true for `CFollowTrackFreeRail`,
    /// which only the safe tile search (`CYapfFollowAnySafeTileRailT`) uses.
    const DO_TRACK_MASKING: bool;
    /// The instantiation's `stGetGlobalCache` statics in [`GLOBAL_CACHES`].
    const GLOBAL_CACHE: usize;
    /// `PfDestination`.
    type PfDestination: PfDestination<Self>;
}

/// `CYapfRail`: `CFollowTrackRail`, `CYapfDestinationTileOrStationRailT`, `CYapfFollowRailT`.
struct CYapfRail;
impl Types for CYapfRail {
    const ALLOW_90DEG_TURNS: bool = true;
    const DO_TRACK_MASKING: bool = false;
    const GLOBAL_CACHE: usize = 0;
    type PfDestination = CYapfDestinationTileOrStationRailT;
}
/// `CYapfRailNo90`: `CFollowTrackRailNo90`, `CYapfDestinationTileOrStationRailT`, `CYapfFollowRailT`.
struct CYapfRailNo90;
impl Types for CYapfRailNo90 {
    const ALLOW_90DEG_TURNS: bool = false;
    const DO_TRACK_MASKING: bool = false;
    const GLOBAL_CACHE: usize = 1;
    type PfDestination = CYapfDestinationTileOrStationRailT;
}
/// `CYapfAnyDepotRail`: `CFollowTrackRail`, `CYapfDestinationAnyDepotRailT`, `CYapfFollowAnyDepotRailT`.
struct CYapfAnyDepotRail;
impl Types for CYapfAnyDepotRail {
    const ALLOW_90DEG_TURNS: bool = true;
    const DO_TRACK_MASKING: bool = false;
    const GLOBAL_CACHE: usize = 2;
    type PfDestination = CYapfDestinationAnyDepotRailT;
}
/// `CYapfAnyDepotRailNo90`: `CFollowTrackRailNo90`, `CYapfDestinationAnyDepotRailT`, `CYapfFollowAnyDepotRailT`.
struct CYapfAnyDepotRailNo90;
impl Types for CYapfAnyDepotRailNo90 {
    const ALLOW_90DEG_TURNS: bool = false;
    const DO_TRACK_MASKING: bool = false;
    const GLOBAL_CACHE: usize = 3;
    type PfDestination = CYapfDestinationAnyDepotRailT;
}
/// `CYapfAnySafeTileRail`: `CFollowTrackFreeRail`, `CYapfDestinationAnySafeTileRailT`, `CYapfFollowAnySafeTileRailT`.
struct CYapfAnySafeTileRail;
impl Types for CYapfAnySafeTileRail {
    const ALLOW_90DEG_TURNS: bool = true;
    const DO_TRACK_MASKING: bool = true;
    const GLOBAL_CACHE: usize = 4;
    type PfDestination = CYapfDestinationAnySafeTileRailT;
}
/// `CYapfAnySafeTileRailNo90`: `CFollowTrackFreeRailNo90`, `CYapfDestinationAnySafeTileRailT`, `CYapfFollowAnySafeTileRailT`.
struct CYapfAnySafeTileRailNo90;
impl Types for CYapfAnySafeTileRailNo90 {
    const ALLOW_90DEG_TURNS: bool = false;
    const DO_TRACK_MASKING: bool = true;
    const GLOBAL_CACHE: usize = 5;
    type PfDestination = CYapfDestinationAnySafeTileRailT;
}

/// The destination provider of an instantiation (`yapf_destrail.hpp`).
trait PfDestination<T: Types>: Sized {
    /// The provider before `SetDestination`.
    fn new() -> Self;
    /// `SetDestination(v, override_rail_type)`.
    fn set_destination(pf: &mut CYapfRailT<'_, T>, override_rail_type: bool);
    /// `PfDetectDestination(TileIndex, Trackdir)`: called by YAPF to detect if
    /// node ends in the desired destination.
    fn pf_detect_destination_tile(pf: &CYapfRailT<'_, T>, tile: TileIndex, td: Trackdir) -> bool;
    /// `PfCalcEstimate`: called by YAPF to calculate cost estimate. Calculates
    /// distance to the destination adds it to the actual cost from origin and
    /// stores the sum to the `Node::estimate`.
    fn pf_calc_estimate(pf: &CYapfRailT<'_, T>, n: &mut CYapfRailNode) -> bool;
}

/// `CYapfDestinationAnyDepotRailT`.
struct CYapfDestinationAnyDepotRailT;
impl<T: Types> PfDestination<T> for CYapfDestinationAnyDepotRailT {
    fn new() -> Self {
        Self
    }
    fn set_destination(pf: &mut CYapfRailT<'_, T>, override_rail_type: bool) {
        pf.set_destination_base(override_rail_type);
    }
    #[inline]
    fn pf_detect_destination_tile(pf: &CYapfRailT<'_, T>, tile: TileIndex, _: Trackdir) -> bool {
        pf.world.rail_depot(tile.0)
    }
    #[inline]
    fn pf_calc_estimate(_: &CYapfRailT<'_, T>, n: &mut CYapfRailNode) -> bool {
        n.estimate = n.cost;
        true
    }
}

/// `CYapfDestinationAnySafeTileRailT`.
struct CYapfDestinationAnySafeTileRailT;
impl<T: Types> PfDestination<T> for CYapfDestinationAnySafeTileRailT {
    fn new() -> Self {
        Self
    }
    fn set_destination(pf: &mut CYapfRailT<'_, T>, override_rail_type: bool) {
        pf.set_destination_base(override_rail_type);
    }
    #[inline]
    fn pf_detect_destination_tile(pf: &CYapfRailT<'_, T>, tile: TileIndex, td: Trackdir) -> bool {
        pf.pbs
            .is_safe_waiting_position(pf.v, tile, td, true, !T::ALLOW_90DEG_TURNS)
            && pf
                .pbs
                .is_waiting_position_free(pf.v, tile, td, !T::ALLOW_90DEG_TURNS)
    }
    #[inline]
    fn pf_calc_estimate(_: &CYapfRailT<'_, T>, n: &mut CYapfRailNode) -> bool {
        n.estimate = n.cost;
        true
    }
}

/// `CYapfDestinationTileOrStationRailT`.
struct CYapfDestinationTileOrStationRailT {
    dest_tile: TileIndex,
    dest_trackdirs: TrackdirBits,
    dest_station_id: StationID,
    any_depot: bool,
}
impl<T: Types<PfDestination = Self>> PfDestination<T> for CYapfDestinationTileOrStationRailT {
    fn new() -> Self {
        Self {
            dest_tile: TileIndex::INVALID,
            dest_trackdirs: TrackdirBits::TRACKDIR_BIT_NONE,
            dest_station_id: StationID::INVALID,
            any_depot: false,
        }
    }
    /// `SetDestination(const Train *v)`; the override is the base's default `false`.
    fn set_destination(pf: &mut CYapfRailT<'_, T>, override_rail_type: bool) {
        let v = pf.v.vehicle();
        pf.dest.any_depot = false;
        let order = v.current_order();
        let order_type = OrderType(order.kind());
        if order_type == OrderType::OT_GOTO_WAYPOINT || order_type == OrderType::OT_GOTO_STATION {
            if order_type == OrderType::OT_GOTO_WAYPOINT
                && !(pf.s.waypoint_is_single_tile)(order.destination)
            {
                /* In case of 'complex' waypoints we need to do a look
                 * ahead. This look ahead messes a bit about, which
                 * means that it 'corrupts' the cache. To prevent this
                 * we disable caching when we're looking for a complex
                 * waypoint. */
                pf.disable_cache(true);
            }
            pf.dest.dest_tile = calc_closest_station_tile(
                pf.s,
                // SAFETY: The shared table returns the static station command table.
                unsafe { &*(pf.s.station_commands)() },
                &pf.world,
                StationID(order.destination),
                TileIndex(v.tile()),
                if OrderType(v.current_order().kind()) == OrderType::OT_GOTO_STATION {
                    StationType::Rail
                } else {
                    StationType::RailWaypoint
                },
            );
            pf.dest.dest_station_id = StationID(order.destination);
            pf.dest.dest_trackdirs = TrackdirBits::INVALID_TRACKDIR_BIT;
        } else {
            if order_type == OrderType::OT_GOTO_DEPOT
                && order.depot_action_flags() & ODATF_NEAREST_DEPOT != 0
            {
                pf.dest.any_depot = true;
            }
            pf.dest.dest_tile = TileIndex(v.dest_tile());
            pf.dest.dest_station_id = StationID::INVALID;
            pf.dest.dest_trackdirs = TrackdirBits(track_status_to_trackdir_bits(
                crate::movement::get_tile_track_status(
                    pf.s,
                    TileIndex(v.dest_tile()),
                    TransportType::TRANSPORT_RAIL,
                    0,
                    DiagDirection::INVALID_DIAGDIR,
                ),
            ));
        }
        pf.set_destination_base(override_rail_type);
    }
    #[inline]
    fn pf_detect_destination_tile(pf: &CYapfRailT<'_, T>, tile: TileIndex, td: Trackdir) -> bool {
        if pf.dest.dest_station_id != StationID::INVALID {
            return pf.world.has_station_tile_rail(tile)
                && (pf.world.get_station_index(tile) == pf.dest.dest_station_id)
                && (pf.world.get_rail_station_track(tile) == Track(trackdir_to_track(td.0)));
        }

        if pf.dest.any_depot {
            return pf.world.rail_depot(tile.0);
        }

        (tile == pf.dest.dest_tile)
            && has_trackdir(pf.dest.dest_trackdirs.0, td.0, |ok| pf.check(ok))
    }
    #[inline]
    fn pf_calc_estimate(pf: &CYapfRailT<'_, T>, n: &mut CYapfRailNode) -> bool {
        if pf.pf_detect_destination(n) {
            n.estimate = n.cost;
            return true;
        }

        n.estimate = n.cost.wrapping_add(octile_distance_cost(
            pf.world.dimensions,
            pf.get_last_tile(n),
            pf.get_last_trackdir(n),
            pf.dest.dest_tile,
        ));
        pf.world.assert_with(|| {
            n.parent
                .is_some_and(|parent| n.estimate >= pf.nodes.items[parent].estimate)
        });
        true
    }
}

/// `TILE`: structure used inside `PfCalcCost()` to keep basic tile information.
#[derive(Clone, Copy)]
struct TileInfo {
    tile: TileIndex,
    td: Trackdir,
    tile_type: TileType,
    rail_type: RailType,
}
impl TileInfo {
    /// `TILE()`.
    fn invalid() -> Self {
        Self {
            tile: TileIndex::INVALID,
            td: Trackdir::INVALID_TRACKDIR,
            tile_type: TileType::MP_VOID,
            rail_type: RailType::INVALID_RAILTYPE,
        }
    }
    /// `TILE(tile, td)`, with `GetTileRailType`.
    #[inline]
    fn new(world: &World<'_>, tile: TileIndex, td: Trackdir) -> Self {
        Self {
            tile,
            td,
            tile_type: world.kind(tile),
            rail_type: RailType(world.tile_rail_type(tile.0)),
        }
    }
}

/// `MAX_SEGMENT_COST`.
const MAX_SEGMENT_COST: i32 = 10000;

/// One rail pathfinder: `CYapfT` over `CYapfBaseT`, `CYapfCostRailT`,
/// `CYapfSegmentCostCacheGlobalT`, `CYapfOriginTileTwoWayT`, the
/// instantiation's destination, its follow class and `CYapfReserveTrack`
/// (whose fields the depot search leaves unused), with `CYapfRailBase`.
struct CYapfRailT<'a, T: Types> {
    s: &'a Services,
    world: World<'a>,
    pbs: Pbs<'a>,
    /// `GetVehicle()`, which `FindPath` sets; nothing reads it before.
    v: crate::train::Handle,

    // `CYapfBaseT`.
    /// Node list multi-container.
    nodes: NodeList,
    /// Pointer to the destination node found at last round.
    best_dest_node: Option<usize>,
    /// Here should be node closest to the destination if path not found.
    best_intermediate_node: Option<usize>,
    /// Maximum number of nodes we are allowed to visit before we give up.
    max_search_nodes: i32,
    /// Stats - how many node's costs were calculated.
    stats_cost_calcs: i32,
    /// Stats - how many node's costs were reused from cache.
    stats_cache_hits: i32,
    /// This is there for debugging purposes (hope it doesn't hurt).
    num_steps: i32,

    // `CYapfCostRailT`.
    /// Maximum cost doesn't work with caching enabled (FS#2900).
    max_cost: i32,
    disable_cache: bool,
    sig_look_ahead_costs: Vec<i32>,
    treat_first_red_two_way_signal_as_eol: bool,
    stopped_on_first_two_way_signal: bool,

    // `CYapfSegmentCostCacheGlobalT`; the global cache is
    // `GLOBAL_CACHES[T::GLOBAL_CACHE]`.
    local_cache: Vec<CYapfRailSegment>,

    // `CYapfDestinationRailBase` and the instantiation's destination.
    compatible_railtypes: RailTypes,
    dest: T::PfDestination,

    // `CYapfReserveTrack`.
    /// The reservation target tile.
    res_dest_tile: TileIndex,
    /// The reservation target trackdir.
    res_dest_td: Trackdir,
    /// The reservation target node, which `SetReservationTarget` sets before any read.
    res_dest_node: usize,
    /// The tile where the reservation failed.
    res_fail_tile: TileIndex,
    /// The trackdir where the reservation failed.
    res_fail_td: Trackdir,
    /// Tile our reservation will originate from.
    origin_tile: TileIndex,
    /// List of signals turned red during a path reservation.
    signals_set_to_red: Vec<(TileIndex, Trackdir)>,
}

impl<'a, T: Types> CYapfRailT<'a, T> {
    /// `Tpf pf`: the constructors of `CYapfBaseT`, `CYapfCostRailT` and
    /// `CYapfSegmentCostCacheGlobalT`, in that order.
    fn new(s: &'a Services, v: crate::train::Handle) -> Self {
        let settings = s.settings();
        // SAFETY: The settings catalog holds canonical process-lifetime addresses.
        let (max_search_nodes, p0, p1, p2, max_signals) = unsafe {
            (
                settings.max_search_nodes.read(),
                settings.rail_look_ahead_signal_p0.read(),
                settings.rail_look_ahead_signal_p1.read(),
                settings.rail_look_ahead_signal_p2.read(),
                settings.rail_look_ahead_max_signals.read(),
            )
        };
        /* pre-compute look-ahead penalties into array */
        let mut sig_look_ahead_costs = Vec::with_capacity(max_signals as usize);
        for i in 0..max_signals {
            // `p0 + i * (p1 + i * p2)` promotes the int constants to the unsigned `i`.
            sig_look_ahead_costs.push(
                p0.cast_unsigned()
                    .wrapping_add(
                        i.wrapping_mul(
                            p1.cast_unsigned()
                                .wrapping_add(i.wrapping_mul(p2.cast_unsigned())),
                        ),
                    )
                    .cast_signed(),
            );
        }
        st_get_global_cache::<T>();
        Self {
            s,
            world: World::new(s),
            pbs: Pbs::acquire(s),
            v,
            nodes: NodeList::new(),
            best_dest_node: None,
            best_intermediate_node: None,
            max_search_nodes: max_search_nodes.cast_signed(),
            stats_cost_calcs: 0,
            stats_cache_hits: 0,
            num_steps: 0,
            max_cost: 0,
            disable_cache: false,
            sig_look_ahead_costs,
            treat_first_red_two_way_signal_as_eol: false,
            stopped_on_first_two_way_signal: false,
            local_cache: Vec::new(),
            compatible_railtypes: RailTypes::EMPTY,
            dest: T::PfDestination::new(),
            res_dest_tile: TileIndex::INVALID,
            res_dest_td: Trackdir::INVALID_TRACKDIR,
            res_dest_node: 0,
            res_fail_tile: TileIndex::INVALID,
            res_fail_td: Trackdir::INVALID_TRACKDIR,
            origin_tile: TileIndex::INVALID,
            signals_set_to_red: Vec::new(),
        }
    }

    /// The native assertion on a failed condition.
    #[inline]
    fn check(&self, condition: bool) {
        crate::movement::native_assert(self.s, condition);
    }
    /// `PfGetSettings().<field>`, loaded at its observation point.
    #[inline]
    fn setting<V: Copy>(&self, field: fn(&SettingAddresses) -> *const V) -> V {
        // SAFETY: The settings catalog holds canonical process-lifetime addresses.
        unsafe { field(self.s.settings()).read() }
    }
    /// A `PfGetSettings()` penalty, added into the original's `int` cost.
    #[inline]
    fn penalty(&self, field: fn(&SettingAddresses) -> *const u32) -> i32 {
        self.setting(field).cast_signed()
    }

    // Segment storage.

    /// `*n.segment`, read.
    #[inline]
    fn segment(&self, r: CachedDataRef) -> CYapfRailSegment {
        if r.global {
            // SAFETY: Serialized under the game-state mutex; the read ends here.
            unsafe { (&(*GLOBAL_CACHES.get()))[T::GLOBAL_CACHE].c.heap[r.index as usize] }
        } else {
            self.local_cache[r.index as usize]
        }
    }
    /// `*n.segment`, written.
    #[inline]
    fn set_segment(&mut self, r: CachedDataRef, value: CYapfRailSegment) {
        if r.global {
            // SAFETY: Serialized under the game-state mutex; the write ends here.
            unsafe {
                (&mut (*GLOBAL_CACHES.get()))[T::GLOBAL_CACHE].c.heap[r.index as usize] = value;
            }
        } else {
            self.local_cache[r.index as usize] = value;
        }
    }
    /// `CYapfRailNode::GetLastTile`.
    #[inline]
    fn get_last_tile(&self, n: &CYapfRailNode) -> TileIndex {
        self.world.assert_with(|| n.segment != CachedDataRef::NULL);
        self.segment(n.segment).last_tile
    }
    /// `CYapfRailNode::GetLastTrackdir`.
    #[inline]
    fn get_last_trackdir(&self, n: &CYapfRailNode) -> Trackdir {
        self.world.assert_with(|| n.segment != CachedDataRef::NULL);
        self.segment(n.segment).last_td
    }
    /// `CYapfRailNode::SetLastTileTrackdir`.
    #[inline]
    fn set_last_tile_trackdir(&mut self, n: &CYapfRailNode, tile: TileIndex, td: Trackdir) {
        self.world.assert_with(|| n.segment != CachedDataRef::NULL);
        let mut segment = self.segment(n.segment);
        segment.last_tile = tile;
        segment.last_td = td;
        self.set_segment(n.segment, segment);
    }

    // `NodeList`.

    /// `InsertOpenNode`: insert given item as open node (into `open_nodes` and `open_queue`).
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
        if self.nodes.new_node == Some(item) {
            self.nodes.new_node = None;
        }
    }
    /// `PopOpenNode`: remove and return the open node specified by a key.
    #[inline]
    fn pop_open_node(&mut self, key: CYapfNodeKeyTrackDir) -> usize {
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
    /// `InsertClosedNode`: close node.
    #[inline]
    fn insert_closed_node(&mut self, item: usize) {
        let key = self.nodes.items[item].key;
        self.world
            .assert_with(|| !self.nodes.open_nodes.contains_key(&key));
        self.nodes.closed_nodes.insert(key, item);
    }

    // `CYapfOriginTileTwoWayT`.

    /// `SetOrigin`: set origin (tiles, trackdirs, etc.).
    fn set_origin(
        &mut self,
        forward_tile: TileIndex,
        forward_td: Trackdir,
        reverse_tile: TileIndex,
        reverse_td: Trackdir,
        reverse_penalty: i32,
    ) {
        if forward_tile != TileIndex::INVALID && forward_td != Trackdir::INVALID_TRACKDIR {
            let node = self.nodes.create_new_node();
            self.nodes.items[node] = CYapfRailNode::set(None, forward_tile, forward_td, false);
            self.add_startup_node(node);
        }
        if reverse_tile != TileIndex::INVALID && reverse_td != Trackdir::INVALID_TRACKDIR {
            let node = self.nodes.create_new_node();
            self.nodes.items[node] = CYapfRailNode::set(None, reverse_tile, reverse_td, false);
            self.nodes.items[node].cost = reverse_penalty;
            self.add_startup_node(node);
        }
    }

    // `CYapfBaseT`.

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
            let Some(best_open_node) = self.nodes.get_best_open_node() else {
                break;
            };

            if self.pf_detect_destination(&self.nodes.items[best_open_node]) {
                self.best_dest_node = Some(best_open_node);
                break;
            }

            self.pf_follow_node(best_open_node);
            if self.max_search_nodes != 0 && self.nodes.closed_count() >= self.max_search_nodes {
                witness::hit(witness::RAIL_LIMIT_STOPS);
                break;
            }

            self.pop_open_node(self.nodes.items[best_open_node].key);
            self.insert_closed_node(best_open_node);
        }

        let destination_found = self.best_dest_node.is_some();

        if crate::debug::level(|d| d.yapf) >= 3 {
            let veh_idx = self.v.vehicle().unit_number(self.s);
            // `TransportTypeChar`
            let ttc = 't';
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the original's (float) casts of the int statistics"
            )]
            let cache_hit_ratio = if self.stats_cache_hits == 0 {
                0.0_f32
            } else {
                f64::from(self.stats_cache_hits) as f32
                    / f64::from(self.stats_cache_hits.wrapping_add(self.stats_cost_calcs)) as f32
                    * 100.0_f32
            };
            let cost = self.best_dest_node.map_or(-1, |n| self.nodes.items[n].cost);
            let dist = self.best_dest_node.map_or(-1, |n| {
                let n = &self.nodes.items[n];
                n.estimate.wrapping_sub(n.cost)
            });

            debug!(
                yapf,
                3,
                "[YAPF{}]{}{:4} - {} rounds - {} open - {} closed - CHR {:4.1}% - C {} D {}",
                ttc,
                if destination_found { '-' } else { '!' },
                veh_idx,
                self.num_steps,
                self.nodes.open_count(),
                self.nodes.closed_count(),
                cache_hit_ratio,
                cost,
                dist
            );
        }

        if witness::enabled() {
            witness::hit(if self.disable_cache {
                witness::RAIL_UNCACHED
            } else {
                witness::RAIL_CACHED
            });
            witness::add(
                witness::RAIL_PARTIAL_PATHS,
                u64::from(!destination_found && self.best_intermediate_node.is_some()),
            );
            witness::add(witness::RAIL_BOUNDED_DEPOT, u64::from(self.max_cost != 0));
            witness::add(witness::RAIL_FOUND, u64::from(destination_found));
            witness::hit(if T::ALLOW_90DEG_TURNS {
                witness::RAIL_ALLOW90
            } else {
                witness::RAIL_FORBID90
            });
        }

        destination_found
    }
    /// `GetBestNode`: if path was found return the best node that has reached
    /// the destination. Otherwise return the best visited node (which was
    /// nearest to the destination).
    #[inline]
    fn get_best_node(&self) -> Option<usize> {
        self.best_dest_node.or(self.best_intermediate_node)
    }
    /// `AddStartupNode`: add new node (created by `CreateNewNode` and filled
    /// with data) into open list.
    fn add_startup_node(&mut self, n: usize) {
        self.world
            .assert_with(|| self.nodes.items[n].parent.is_none());
        self.world.assert_with(|| self.num_steps == 0);

        self.pf_node_cache_fetch(n);
        /* insert the new node only if it is not there */
        if self.nodes.find_open_node(self.nodes.items[n].key).is_none() {
            self.insert_open_node(n);
        }
    }
    /// `AddMultipleNodes`: add multiple nodes - direct children of the given node.
    #[inline]
    fn add_multiple_nodes(&mut self, parent: usize, tf: &CFollowTrackT) {
        let is_choice = kill_first_bit_u16(tf.new_td_bits.0) != TrackdirBits::TRACKDIR_BIT_NONE.0;
        let mut rtds = tf.new_td_bits;
        while rtds != TrackdirBits::TRACKDIR_BIT_NONE {
            let td = Trackdir(find_first_bit(u64::from(rtds.0)));
            let n = self.nodes.create_new_node();
            let parent_node = self.nodes.items[parent];
            self.nodes.items[n] =
                CYapfRailNode::set(Some((parent, &parent_node)), tf.new_tile, td, is_choice);
            self.add_new_node(n, tf);
            rtds = TrackdirBits(kill_first_bit_u16(rtds.0));
        }
    }
    /// `AddNewNode`: called by `PfFollowNode` for each child node. Nodes are
    /// evaluated here and added into open list.
    fn add_new_node(&mut self, n: usize, follower: &CFollowTrackT) {
        self.world
            .assert_with(|| self.nodes.items[n].parent.is_some());

        /* evaluate the node */
        let cached = self.pf_node_cache_fetch(n);
        if cached {
            self.stats_cache_hits = self.stats_cache_hits.wrapping_add(1);
        } else {
            self.stats_cost_calcs = self.stats_cost_calcs.wrapping_add(1);
        }

        let mut node = self.nodes.items[n];
        let mut valid = self.pf_calc_cost(&mut node, follower);

        if valid {
            valid = T::PfDestination::pf_calc_estimate(self, &mut node);
        }
        self.nodes.items[n] = node;

        /* have the cost or estimate callbacks marked this node as invalid? */
        if !valid {
            return;
        }

        /* The new node can be set as the best intermediate node only once we're
         * certain it will be finalized by being inserted into the open list. */
        let set_intermediate = self.max_search_nodes > 0
            && self.best_intermediate_node.is_none_or(|i| {
                let best = &self.nodes.items[i];
                best.estimate.wrapping_sub(best.cost) > node.estimate.wrapping_sub(node.cost)
            });

        /* check new node against open list */
        if let Some(open_node) = self.nodes.find_open_node(node.key) {
            /* another node exists with the same key in the open list
             * is it better than new one? */
            if node.estimate < self.nodes.items[open_node].estimate {
                /* update the old node by value from new one */
                self.pop_open_node(node.key);
                self.nodes.items[open_node] = node;
                /* add the updated old node back to open list */
                self.insert_open_node(open_node);
                if set_intermediate {
                    self.best_intermediate_node = Some(open_node);
                }
            }
            return;
        }

        /* check new node against closed list */
        if let Some(closed_node) = self.nodes.find_closed_node(node.key) {
            /* another node exists with the same key in the closed list
             * is it better than new one? */
            let node_est = node.estimate;
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
        self.insert_open_node(n);
        if set_intermediate {
            self.best_intermediate_node = Some(n);
        }
    }

    // `CYapfSegmentCostCacheGlobalT`.

    /// `PfNodeCacheFetch`: called by YAPF to attach cached or local segment
    /// cost data to the given node. Returns true if globally cached data were
    /// used or false if local data was used.
    #[inline]
    fn pf_node_cache_fetch(&mut self, n: usize) -> bool {
        let key = CYapfRailSegmentKey::new(self.nodes.items[n].key);

        if !self.can_use_global_cache(n) {
            let item = self.local_cache.len();
            self.local_cache.push(CYapfRailSegment::new());
            self.connect_node_to_cached_data(
                n,
                CachedDataRef {
                    global: false,
                    index: index_u32(item),
                },
            );
            return false;
        }

        let mut found = false;
        // SAFETY: Serialized under the game-state mutex; the borrow ends here.
        let item = unsafe {
            (*GLOBAL_CACHES.get())[T::GLOBAL_CACHE]
                .c
                .get(key, &mut found)
        };
        self.connect_node_to_cached_data(
            n,
            CachedDataRef {
                global: true,
                index: index_u32(item),
            },
        );
        found
    }

    // `CYapfCostRailT`.

    /// `SetTreatFirstRedTwoWaySignalAsEOL`: sets whether the first two-way
    /// signal should be treated as a dead end.
    fn set_treat_first_red_two_way_signal_as_eol(&mut self, enabled: bool) {
        self.treat_first_red_two_way_signal_as_eol = enabled;
    }
    /// `TreatFirstRedTwoWaySignalAsEOL`: returns whether the first two-way
    /// signal should be treated as a dead end.
    #[inline]
    fn treat_first_red_two_way_signal_as_eol(&self) -> bool {
        self.setting(|s| s.rail_firstred_twoway_eol) && self.treat_first_red_two_way_signal_as_eol
    }
    /// `SlopeCost`.
    #[inline]
    fn slope_cost(&self, tile: TileIndex, td: Trackdir) -> i32 {
        if !st_slope_cost(&self.world, tile, td) {
            return 0;
        }
        self.penalty(|s| s.rail_slope_penalty)
    }
    /// `CurveCost`.
    #[inline]
    fn curve_cost(&self, td1: Trackdir, td2: Trackdir) -> i32 {
        self.world.assert_with(|| is_valid_trackdir(td1.0));
        self.world.assert_with(|| is_valid_trackdir(td2.0));
        let mut cost = 0_i32;
        if T::ALLOW_90DEG_TURNS
            && has_trackdir(trackdir_crosses_trackdirs(td1.0), td2.0, |ok| {
                self.check(ok);
            })
        {
            /* 90-deg curve penalty */
            cost = cost.wrapping_add(self.penalty(|s| s.rail_curve90_penalty));
        } else if td2.0 != next_trackdir(td1.0, |ok| self.check(ok)) {
            /* 45-deg curve penalty */
            cost = cost.wrapping_add(self.penalty(|s| s.rail_curve45_penalty));
        }
        cost
    }
    /// `SwitchCost`.
    #[inline]
    fn switch_cost(&self, tile1: TileIndex, tile2: TileIndex, exitdir: DiagDirection) -> i32 {
        if self.world.plain_rail(tile1.0) && self.world.plain_rail(tile2.0) {
            let t1 = kill_first_bit(
                self.world.tracks(tile1.0)
                    & diagdir_reaches_tracks(reverse_diag_dir(exitdir, |ok| self.check(ok)).0),
            ) != TrackBits::TRACK_BIT_NONE.0;
            let t2 = kill_first_bit(self.world.tracks(tile2.0) & diagdir_reaches_tracks(exitdir.0))
                != TrackBits::TRACK_BIT_NONE.0;
            if t1 && t2 {
                return self.penalty(|s| s.rail_doubleslip_penalty);
            }
        }
        0
    }
    /// `OneTileCost`: return one tile cost (base cost + level crossing penalty).
    #[inline]
    fn one_tile_cost(&self, tile: TileIndex, trackdir: Trackdir) -> i32 {
        let mut cost = 0_i32;
        /* set base cost */
        if is_diagonal_trackdir(trackdir.0) {
            cost = cost.wrapping_add(YAPF_TILE_LENGTH);
            if self.world.kind(tile) == TileType::MP_ROAD {
                /* Increase the cost for level crossings */
                if self.world.level_crossing(tile.0) {
                    cost = cost.wrapping_add(self.penalty(|s| s.rail_crossing_penalty));
                }
            }
        } else {
            /* non-diagonal trackdir */
            cost = YAPF_TILE_CORNER_LENGTH;
        }
        cost
    }
    /// `IsAnyStationTileReserved`: check for a reserved station platform.
    #[inline]
    fn is_any_station_tile_reserved(
        &self,
        mut tile: TileIndex,
        trackdir: Trackdir,
        mut skipped: i32,
    ) -> bool {
        let diff = self.world.dimensions.tile_offs_by_diag_dir(
            DiagDirection(trackdir_to_exitdir(reverse_trackdir(trackdir.0))),
            |ok| self.check(ok),
        );
        while skipped >= 0 {
            if self.world.has_station_reservation(tile) {
                return true;
            }
            skipped -= 1;
            tile = TileIndex(tile.0.wrapping_add_signed(diff));
        }
        false
    }
    /// `ReservationCost`: the cost for reserved tiles, including skipped ones.
    #[inline]
    fn reservation_cost(
        &self,
        n: &CYapfRailNode,
        tile: TileIndex,
        trackdir: Trackdir,
        skipped: i32,
    ) -> i32 {
        if usize::from(n.num_signals_passed) >= self.sig_look_ahead_costs.len() / 2 {
            return 0;
        }
        if !is_pbs_signal(n.last_signal_type.0) {
            return 0;
        }

        if self.world.is_rail_station_tile(tile)
            && self.is_any_station_tile_reserved(tile, trackdir, skipped)
        {
            return self
                .penalty(|s| s.rail_pbs_station_penalty)
                .wrapping_mul(skipped.wrapping_add(1));
        } else if track_overlaps_tracks(
            self.pbs.get_reserved_trackbits(tile).0,
            trackdir_to_track(trackdir.0),
            |ok| self.check(ok),
        ) {
            let mut cost = self.penalty(|s| s.rail_pbs_cross_penalty);
            if !is_diagonal_trackdir(trackdir.0) {
                cost = cost.wrapping_mul(YAPF_TILE_CORNER_LENGTH) / YAPF_TILE_LENGTH;
            }
            return cost.wrapping_mul(skipped.wrapping_add(1));
        }
        0
    }
    /// `SignalCost`.
    #[allow(clippy::if_not_else)]
    fn signal_cost(&mut self, n: &mut CYapfRailNode, tile: TileIndex, trackdir: Trackdir) -> i32 {
        let mut cost = 0_i32;
        /* if there is one-way signal in the opposite direction, then it is not our way */
        if self.world.kind(tile) == TileType::MP_RAILWAY {
            let has_signal_against = self
                .world
                .has_signal_on_trackdir(tile.0, reverse_trackdir(trackdir.0));
            let has_signal_along = self.world.has_signal_on_trackdir(tile.0, trackdir.0);
            if has_signal_against
                && !has_signal_along
                && self
                    .world
                    .oneway_signal(tile.0, trackdir_to_track(trackdir.0))
            {
                /* one-way signal in opposite direction */
                let mut segment = self.segment(n.segment);
                segment.end_segment_reason.set(EndSegmentReason::DEAD_END);
                self.set_segment(n.segment, segment);
            } else {
                if has_signal_along {
                    let sig_state =
                        SignalState(self.world.signal_state_by_trackdir(tile.0, trackdir.0));
                    let sig_type = SignalType(
                        self.world
                            .signal_type(tile.0, trackdir_to_track(trackdir.0)),
                    );

                    n.last_signal_type = sig_type;

                    /* cache the look-ahead polynomial constant only if we didn't pass more signals than the look-ahead limit is */
                    let look_ahead_cost = self
                        .sig_look_ahead_costs
                        .get(usize::from(n.num_signals_passed))
                        .copied()
                        .unwrap_or(0);
                    if sig_state != SignalState::SIGNAL_STATE_RED {
                        /* green signal */
                        n.flags_u
                            .set(CYapfRailNodeFlags::LAST_SIGNAL_WAS_RED, false);
                        /* negative look-ahead red-signal penalties would cause problems later, so use them as positive penalties for green signal */
                        if look_ahead_cost < 0 {
                            /* add its negation to the cost */
                            cost = cost.wrapping_sub(look_ahead_cost);
                        }
                    } else {
                        /* we have a red signal in our direction
                         * was it first signal which is two-way? */
                        if !is_pbs_signal(sig_type.0)
                            && self.treat_first_red_two_way_signal_as_eol()
                            && n.flags_u.get(CYapfRailNodeFlags::CHOICE_SEEN)
                            && has_signal_against
                            && n.num_signals_passed == 0
                        {
                            /* yes, the first signal is two-way red signal => DEAD END. Prune this branch... */
                            self.prune_intermediate_node_branch(n);
                            let mut segment = self.segment(n.segment);
                            segment.end_segment_reason.set(EndSegmentReason::DEAD_END);
                            self.set_segment(n.segment, segment);
                            self.stopped_on_first_two_way_signal = true;
                            return -1;
                        }
                        n.last_red_signal_type = sig_type;
                        n.flags_u.set(CYapfRailNodeFlags::LAST_SIGNAL_WAS_RED, true);

                        /* look-ahead signal penalty */
                        if !is_pbs_signal(sig_type.0) && look_ahead_cost > 0 {
                            /* add the look ahead penalty only if it is positive */
                            cost = cost.wrapping_add(look_ahead_cost);
                        }

                        /* special signal penalties */
                        if n.num_signals_passed == 0 {
                            match sig_type {
                                SignalType::SIGTYPE_COMBO | SignalType::SIGTYPE_EXIT => {
                                    // first signal is red pre-signal-exit
                                    cost = cost.wrapping_add(
                                        self.penalty(|s| s.rail_firstred_exit_penalty),
                                    );
                                }
                                SignalType::SIGTYPE_BLOCK | SignalType::SIGTYPE_ENTRY => {
                                    cost = cost
                                        .wrapping_add(self.penalty(|s| s.rail_firstred_penalty));
                                }
                                _ => {}
                            }
                        }
                    }

                    n.num_signals_passed = n.num_signals_passed.wrapping_add(1);
                    let mut segment = self.segment(n.segment);
                    segment.last_signal_tile = tile;
                    segment.last_signal_td = trackdir;
                    self.set_segment(n.segment, segment);
                }

                if has_signal_against
                    && is_pbs_signal(
                        self.world
                            .signal_type(tile.0, trackdir_to_track(trackdir.0)),
                    )
                {
                    cost = cost.wrapping_add(
                        if u32::from(n.num_signals_passed)
                            < self.setting(|s| s.rail_look_ahead_max_signals)
                        {
                            self.penalty(|s| s.rail_pbs_signal_back_penalty)
                        } else {
                            0
                        },
                    );
                }
            }
        }
        cost
    }
    /// `PlatformLengthPenalty`.
    #[inline]
    fn platform_length_penalty(&self, platform_length: i32) -> i32 {
        let mut cost = 0_i32;
        let v = self.v.vehicle();
        self.world
            .assert_with(|| v.vehicle_type() == crate::vehicle_type::VEH_TRAIN);
        let cached_total_length =
            crate::raw_field::read_field!(v.ground_cache(), cached_total_length);
        self.world.assert_with(|| cached_total_length != 0);
        let missing_platform_length = u32::from(cached_total_length)
            .div_ceil(TILE_SIZE)
            .cast_signed()
            .wrapping_sub(platform_length);
        if missing_platform_length < 0 {
            /* apply penalty for longer platform than needed */
            cost = cost.wrapping_add(
                self.penalty(|s| s.rail_longer_platform_penalty)
                    .wrapping_add(
                        self.penalty(|s| s.rail_longer_platform_per_tile_penalty)
                            .wrapping_mul(missing_platform_length.wrapping_neg()),
                    ),
            );
        } else if missing_platform_length > 0 {
            /* apply penalty for shorter platform than needed */
            cost = cost.wrapping_add(
                self.penalty(|s| s.rail_shorter_platform_penalty)
                    .wrapping_add(
                        self.penalty(|s| s.rail_shorter_platform_per_tile_penalty)
                            .wrapping_mul(missing_platform_length),
                    ),
            );
        }
        cost
    }
    /// `SetMaxCost`.
    fn set_max_cost(&mut self, max_cost: i32) {
        self.max_cost = max_cost;
    }
    /// `PfCalcCost`: called by YAPF to calculate the cost from the origin to
    /// the given node. Calculates only the cost of given node, adds it to the
    /// parent node cost and stores the result into `Node::cost` member.
    #[allow(clippy::too_many_lines)]
    fn pf_calc_cost(&mut self, n: &mut CYapfRailNode, follower: &CFollowTrackT) -> bool {
        self.world
            .assert_with(|| !n.flags_u.get(CYapfRailNodeFlags::TARGET_SEEN));
        self.world.assert_with(|| follower.new_tile == n.key.0.tile);
        self.world.assert_with(|| {
            has_trackdir(follower.new_td_bits.0, n.key.0.td.0, |ok| self.check(ok))
        });

        /* Does the node have some parent node? */
        let has_parent = n.parent.is_some();

        /* Do we already have a cached segment? */
        let segment = n.segment;
        let is_cached_segment = self.segment(segment).cost >= 0;

        let parent_cost = n.parent.map_or(0, |p| self.nodes.items[p].cost);

        /* Each node cost contains 2 or 3 main components:
         *  1. Transition cost - cost of the move from previous node (tile):
         *    - curve cost (or zero for straight move)
         *  2. Tile cost:
         *    - base tile cost
         *      - YAPF_TILE_LENGTH for diagonal tiles
         *      - YAPF_TILE_CORNER_LENGTH for non-diagonal tiles
         *    - tile penalties
         *      - tile slope penalty (upward slopes)
         *      - red signal penalty
         *      - level crossing penalty
         *      - speed-limit penalty (bridges)
         *      - station platform penalty
         *      - penalty for reversing in the depot
         *      - etc.
         *  3. Extra cost (applies to the last node only)
         *    - last red signal penalty
         *    - penalty for too long or too short platform on the destination station
         */
        let mut transition_cost;
        let mut extra_cost = 0_i32;

        /* Segment: one or more tiles connected by contiguous tracks of the same type.
         * Each segment cost includes 'Tile cost' for all its tiles (including the first
         * and last), and the 'Transition cost' between its tiles. The first transition
         * cost of segment entry (move from the 'parent' node) is not included!
         */
        let mut segment_entry_cost = 0_i32;
        let mut segment_cost = 0_i32;

        let v = self.v.vehicle();

        /* start at n.key.tile / n.key.td and walk to the end of segment */
        let mut cur = TileInfo::new(&self.world, n.key.0.tile, n.key.0.td);

        /* the previous tile will be needed for transition cost calculations */
        let mut prev = match n.parent {
            None => TileInfo::invalid(),
            Some(p) => {
                let parent = self.nodes.items[p];
                TileInfo::new(
                    &self.world,
                    self.get_last_tile(&parent),
                    self.get_last_trackdir(&parent),
                )
            }
        };

        let mut end_segment_reason: EndSegmentReasons;

        let mut follower_local = CFollowTrackT::vehicle(
            self.s,
            TransportType::TRANSPORT_RAIL,
            Some(v),
            self.compatible_railtypes,
            T::ALLOW_90DEG_TURNS,
        );
        // `follower` points at the caller's follower until the first move.
        let mut follower_is_local = false;

        // `goto no_entry_cost`.
        let mut no_entry_cost = false;
        if !has_parent {
            /* We will jump to the middle of the cost calculator assuming that segment cache is not used. */
            self.world.assert_with(|| !is_cached_segment);
            /* Skip the first transition cost calculation. */
            no_entry_cost = true;
        }

        loop {
            if !no_entry_cost {
                /* Transition cost (cost of the move from previous tile) */
                transition_cost = self.curve_cost(prev.td, cur.td);
                transition_cost = transition_cost.wrapping_add(self.switch_cost(
                    prev.tile,
                    cur.tile,
                    DiagDirection(trackdir_to_exitdir(prev.td.0)),
                ));

                /* First transition cost counts against segment entry cost, other transitions
                 * inside segment will come to segment cost (and will be cached) */
                if segment_cost == 0 {
                    /* We just entered the loop. First transition cost goes to segment entry cost)*/
                    segment_entry_cost = transition_cost;

                    /* It is the right time now to look if we can reuse the cached segment cost. */
                    if is_cached_segment {
                        let cached = self.segment(segment);
                        /* Yes, we already know the segment cost. */
                        segment_cost = cached.cost;
                        /* We know also the reason why the segment ends. */
                        end_segment_reason = cached.end_segment_reason;
                        /* We will need also some information about the last signal (if it was red). */
                        if cached.last_signal_tile != TileIndex::INVALID {
                            self.world.assert_with(|| {
                                self.world.has_signal_on_trackdir(
                                    cached.last_signal_tile.0,
                                    cached.last_signal_td.0,
                                )
                            });
                            let sig_state = SignalState(self.world.signal_state_by_trackdir(
                                cached.last_signal_tile.0,
                                cached.last_signal_td.0,
                            ));
                            let is_red = sig_state == SignalState::SIGNAL_STATE_RED;
                            n.flags_u
                                .set(CYapfRailNodeFlags::LAST_SIGNAL_WAS_RED, is_red);
                            if is_red {
                                n.last_red_signal_type = SignalType(self.world.signal_type(
                                    cached.last_signal_tile.0,
                                    trackdir_to_track(cached.last_signal_td.0),
                                ));
                            }
                        }
                        /* No further calculation needed. */
                        cur = TileInfo::new(
                            &self.world,
                            self.get_last_tile(n),
                            self.get_last_trackdir(n),
                        );
                        break;
                    }
                } else {
                    /* Other than first transition cost count as the regular segment cost. */
                    segment_cost = segment_cost.wrapping_add(transition_cost);
                }
            }

            // no_entry_cost: jump here at the beginning if the node has no parent (it is the first node)
            no_entry_cost = false;
            let (tiles_skipped, is_station) = if follower_is_local {
                (follower_local.tiles_skipped, follower_local.is_station)
            } else {
                (follower.tiles_skipped, follower.is_station)
            };

            /* All other tile costs will be calculated here. */
            segment_cost = segment_cost.wrapping_add(self.one_tile_cost(cur.tile, cur.td));

            /* If we skipped some tunnel/bridge/station tiles, add their base cost */
            segment_cost = segment_cost.wrapping_add(YAPF_TILE_LENGTH.wrapping_mul(tiles_skipped));

            /* Slope cost. */
            segment_cost = segment_cost.wrapping_add(self.slope_cost(cur.tile, cur.td));

            /* Signal cost (routine can modify segment data). */
            segment_cost = segment_cost.wrapping_add(self.signal_cost(n, cur.tile, cur.td));

            /* Reserved tiles. */
            segment_cost = segment_cost.wrapping_add(self.reservation_cost(
                n,
                cur.tile,
                cur.td,
                tiles_skipped,
            ));

            end_segment_reason = self.segment(segment).end_segment_reason;

            /* Tests for 'potential target' reasons to close the segment. */
            if cur.tile == prev.tile {
                /* Penalty for reversing in a depot. */
                self.world
                    .assert_with(|| self.world.is_rail_depot(cur.tile.0));
                segment_cost =
                    segment_cost.wrapping_add(self.penalty(|s| s.rail_depot_reverse_penalty));
            } else if self.world.rail_depot(cur.tile.0) {
                /* We will end in this pass (depot is possible target) */
                end_segment_reason.set(EndSegmentReason::DEPOT);
            } else if cur.tile_type == TileType::MP_STATION && self.world.is_rail_waypoint(cur.tile)
            {
                let order = v.current_order();
                if OrderType(order.kind()) == OrderType::OT_GOTO_WAYPOINT
                    && self.world.get_station_index(cur.tile) == StationID(order.destination)
                    && !(self.s.waypoint_is_single_tile)(order.destination)
                {
                    /* This waypoint is our destination; maybe this isn't an unreserved
                     * one, so check that and if so see that as the last signal being
                     * red. This way waypoints near stations should work better. */
                    extra_cost = extra_cost.wrapping_add(self.waypoint_look_ahead_cost(cur));
                }
                /* Waypoint is also a good reason to finish. */
                end_segment_reason.set(EndSegmentReason::WAYPOINT);
            } else if is_station {
                /* Station penalties. */
                let platform_length = tiles_skipped.cast_unsigned().wrapping_add(1);
                /* We don't know yet if the station is our target or not. Act like
                 * if it is pass-through station (not our destination). */
                segment_cost = segment_cost.wrapping_add(
                    self.setting(|s| s.rail_station_penalty)
                        .wrapping_mul(platform_length)
                        .cast_signed(),
                );
                /* We will end in this pass (station is possible target) */
                end_segment_reason.set(EndSegmentReason::STATION);
            } else if T::DO_TRACK_MASKING && cur.tile_type == TileType::MP_RAILWAY {
                /* Searching for a safe tile? */
                if self.world.has_signal_on_trackdir(cur.tile.0, cur.td.0)
                    && !is_pbs_signal(
                        self.world
                            .signal_type(cur.tile.0, trackdir_to_track(cur.td.0)),
                    )
                {
                    end_segment_reason.set(EndSegmentReason::SAFE_TILE);
                }
            }

            /* Apply min/max speed penalties only when inside the look-ahead radius. Otherwise
             * it would cause desync in MP. */
            if usize::from(n.num_signals_passed) < self.sig_look_ahead_costs.len() {
                let mut min_speed = 0_i32;
                let speed_follower = if follower_is_local {
                    &follower_local
                } else {
                    follower
                };
                let max_speed = get_speed_limit(
                    self.s,
                    speed_follower.mode,
                    speed_follower.veh,
                    speed_follower.old_tile,
                    Some(&mut min_speed),
                );
                // `GetDisplayMaxSpeed()`, which a train answers from `vcache.cached_max_speed`.
                let max_veh_speed = i32::from(v.max_speed().min(v.current_order().max_speed()));
                if max_speed < max_veh_speed {
                    extra_cost = extra_cost.wrapping_add(
                        YAPF_TILE_LENGTH
                            .wrapping_mul(max_veh_speed.wrapping_sub(max_speed))
                            .wrapping_mul(4_i32.wrapping_add(tiles_skipped))
                            / max_veh_speed,
                    );
                }
                if min_speed > max_veh_speed {
                    extra_cost = extra_cost.wrapping_add(
                        YAPF_TILE_LENGTH.wrapping_mul(min_speed.wrapping_sub(max_veh_speed)),
                    );
                }
            }

            /* Finish if we already exceeded the maximum path cost (i.e. when
             * searching for the nearest depot). */
            if self.max_cost > 0
                && parent_cost
                    .wrapping_add(segment_entry_cost)
                    .wrapping_add(segment_cost)
                    > self.max_cost
            {
                end_segment_reason.set(EndSegmentReason::PATH_TOO_LONG);
            }

            /* Move to the next tile/trackdir. */
            follower_is_local = true;
            follower_local = CFollowTrackT::vehicle(
                self.s,
                TransportType::TRANSPORT_RAIL,
                Some(v),
                self.compatible_railtypes,
                T::ALLOW_90DEG_TURNS,
            );

            if !follower_local.follow(self.s, cur.tile, cur.td) {
                self.world
                    .assert_with(|| follower_local.err != ErrorCode::EC_NONE);
                /* Can't move to the next tile (EOL?). */
                if follower_local.err == ErrorCode::EC_RAIL_ROAD_TYPE {
                    end_segment_reason.set(EndSegmentReason::RAIL_TYPE);
                } else {
                    end_segment_reason.set(EndSegmentReason::DEAD_END);
                }

                if T::DO_TRACK_MASKING && !(self.pbs.0.blocking)(cur.tile.0, cur.td.0) {
                    end_segment_reason.set(EndSegmentReason::SAFE_TILE);
                }
                break;
            }

            /* Check if the next tile is not a choice. */
            if kill_first_bit_u16(follower_local.new_td_bits.0) != TrackdirBits::TRACKDIR_BIT_NONE.0
            {
                /* More than one segment will follow. Close this one. */
                end_segment_reason.set(EndSegmentReason::CHOICE_FOLLOWS);
                break;
            }

            /* Gather the next tile/trackdir/tile_type/rail_type. */
            let next = TileInfo::new(
                &self.world,
                follower_local.new_tile,
                Trackdir(find_first_bit(u64::from(follower_local.new_td_bits.0))),
            );

            if T::DO_TRACK_MASKING && self.world.kind(next.tile) == TileType::MP_RAILWAY {
                if self.world.has_signal_on_trackdir(next.tile.0, next.td.0)
                    && is_pbs_signal(
                        self.world
                            .signal_type(next.tile.0, trackdir_to_track(next.td.0)),
                    )
                {
                    /* Possible safe tile. */
                    end_segment_reason.set(EndSegmentReason::SAFE_TILE);
                } else if self
                    .world
                    .has_signal_on_trackdir(next.tile.0, reverse_trackdir(next.td.0))
                    && SignalType(
                        self.world
                            .signal_type(next.tile.0, trackdir_to_track(next.td.0)),
                    ) == SignalType::SIGTYPE_PBS_ONEWAY
                {
                    /* Possible safe tile, but not so good as it's the back of a signal... */
                    end_segment_reason.set_all(EndSegmentReasons::of(&[
                        EndSegmentReason::SAFE_TILE,
                        EndSegmentReason::DEAD_END,
                    ]));
                    extra_cost =
                        extra_cost.wrapping_add(self.penalty(|s| s.rail_lastred_exit_penalty));
                }
            }

            /* Check the next tile for the rail type. */
            if next.rail_type != cur.rail_type {
                /* Segment must consist from the same rail_type tiles. */
                end_segment_reason.set(EndSegmentReason::RAIL_TYPE);
                break;
            }

            /* Avoid infinite looping. */
            if next.tile == n.key.0.tile && next.td == n.key.0.td {
                end_segment_reason.set(EndSegmentReason::INFINITE_LOOP);
                break;
            }

            if segment_cost > MAX_SEGMENT_COST {
                /* Potentially in the infinite loop (or only very long segment?). We should
                 * not force it to finish prematurely unless we are on a regular tile. */
                if self.world.kind(follower_local.new_tile) == TileType::MP_RAILWAY {
                    end_segment_reason.set(EndSegmentReason::SEGMENT_TOO_LONG);
                    break;
                }
            }

            /* Any other reason bit set? */
            if end_segment_reason.any() {
                break;
            }

            /* For the next loop set new prev and cur tile info. */
            prev = cur;
            cur = next;
        }

        /* Don't consider path any further it if exceeded max_cost. */
        if end_segment_reason.test(EndSegmentReason::PATH_TOO_LONG) {
            return false;
        }

        let mut target_seen = false;
        if end_segment_reason.any_of(ESRF_POSSIBLE_TARGET) {
            /* Depot, station or waypoint. */
            if T::PfDestination::pf_detect_destination_tile(self, cur.tile, cur.td) {
                /* Destination found. */
                target_seen = true;
            }
        }

        /* Update the segment if needed. */
        if !is_cached_segment {
            /* Write back the segment information so it can be reused the next time. */
            let mut written = self.segment(segment);
            written.cost = segment_cost;
            written.end_segment_reason = end_segment_reason & ESRF_CACHED_MASK;
            self.set_segment(segment, written);
            /* Save end of segment back to the node. */
            self.set_last_tile_trackdir(n, cur.tile, cur.td);
        }

        /* Do we have an excuse why not to continue pathfinding in this direction? */
        if !target_seen && end_segment_reason.any_of(ESRF_ABORT_PF_MASK) {
            /* Reason to not continue. Stop this PF branch. */
            return false;
        }

        /* Special costs for the case we have reached our target. */
        if target_seen {
            n.flags_u.set(CYapfRailNodeFlags::TARGET_SEEN, true);
            /* Last-red and last-red-exit penalties. */
            if n.flags_u.get(CYapfRailNodeFlags::LAST_SIGNAL_WAS_RED) {
                if n.last_red_signal_type == SignalType::SIGTYPE_EXIT {
                    /* last signal was red pre-signal-exit */
                    extra_cost =
                        extra_cost.wrapping_add(self.penalty(|s| s.rail_lastred_exit_penalty));
                } else if !is_pbs_signal(n.last_red_signal_type.0) {
                    /* Last signal was red, but not exit or path signal. */
                    extra_cost = extra_cost.wrapping_add(self.penalty(|s| s.rail_lastred_penalty));
                }
            }

            /* Station platform-length penalty. */
            if end_segment_reason.test(EndSegmentReason::STATION) {
                let last_tile = self.get_last_tile(n);
                self.world
                    .assert_with(|| !(self.pbs.0.station)(last_tile.0).is_null());
                let platform_length = crate::station::get_platform_length_by_tile(
                    &self.world,
                    last_tile,
                    reverse_diag_dir(
                        DiagDirection(trackdir_to_exitdir(self.get_last_trackdir(n).0)),
                        |ok| self.check(ok),
                    ),
                );
                /* Reduce the extra cost caused by passing-station penalty (each station receives it in the segment cost). */
                extra_cost = extra_cost.wrapping_sub(
                    self.setting(|s| s.rail_station_penalty)
                        .wrapping_mul(platform_length)
                        .cast_signed(),
                );
                /* Add penalty for the inappropriate platform length. */
                extra_cost = extra_cost
                    .wrapping_add(self.platform_length_penalty(platform_length.cast_signed()));
            }
        }

        /* total node cost */
        n.cost = parent_cost
            .wrapping_add(segment_entry_cost)
            .wrapping_add(segment_cost)
            .wrapping_add(extra_cost);

        true
    }
    /// The waypoint branch of `PfCalcCost`: `rail_lastred_penalty` when the
    /// destination waypoint's platform from `cur` is (possibly) occupied.
    fn waypoint_look_ahead_cost(&self, cur: TileInfo) -> i32 {
        let mut ft = CFollowTrackT::vehicle(
            self.s,
            TransportType::TRANSPORT_RAIL,
            Some(self.v.vehicle()),
            RailTypes::INVALID_RAILTYPES,
            true,
        );
        let mut t = cur.tile;
        let mut td = cur.td;
        let forbid_90_deg = (self.s.forbid_90_deg)();
        /* Arbitrary maximum tiles to follow to avoid infinite loops. */
        let mut max_tiles = 20_u32;
        while ft.follow(self.s, t, td) {
            self.world.assert_with(|| t != ft.new_tile);
            t = ft.new_tile;
            max_tiles = max_tiles.wrapping_sub(1);
            if t == cur.tile || max_tiles == 0 {
                /* We looped back on ourself or found another loop, bail out. */
                td = Trackdir::INVALID_TRACKDIR;
                break;
            }
            if kill_first_bit_u16(ft.new_td_bits.0) != TrackdirBits::TRACKDIR_BIT_NONE.0 {
                /* We encountered a junction; it's going to be too complex to
                 * handle this perfectly, so just bail out. There is no simple
                 * free path, so try the other possibilities. */
                td = Trackdir::INVALID_TRACKDIR;
                break;
            }
            // `RemoveFirstTrackdir(&ft.new_td_bits)`.
            td = Trackdir(find_first_bit(u64::from(ft.new_td_bits.0)));
            ft.new_td_bits = TrackdirBits(kill_first_bit_u16(ft.new_td_bits.0));
            /* If this is a safe waiting position we're done searching for it */
            if self
                .pbs
                .is_safe_waiting_position(self.v, t, td, true, forbid_90_deg)
            {
                break;
            }
        }

        /* In the case this platform is (possibly) occupied we add penalty so the
         * other platforms of this waypoint are evaluated as well, i.e. we assume
         * that there is a red signal in the waypoint when it's occupied. */
        if td == Trackdir::INVALID_TRACKDIR
            || !self
                .pbs
                .is_safe_waiting_position(self.v, t, td, true, forbid_90_deg)
            || !self
                .pbs
                .is_waiting_position_free(self.v, t, td, forbid_90_deg)
        {
            return self.penalty(|s| s.rail_lastred_penalty);
        }
        0
    }
    /// `CanUseGlobalCache`.
    #[inline]
    fn can_use_global_cache(&self, n: usize) -> bool {
        !self.disable_cache
            && self.nodes.items[n].parent.is_some_and(|parent| {
                usize::from(self.nodes.items[parent].num_signals_passed)
                    >= self.sig_look_ahead_costs.len()
            })
    }
    /// `ConnectNodeToCachedData`.
    #[inline]
    fn connect_node_to_cached_data(&mut self, n: usize, ci: CachedDataRef) {
        self.nodes.items[n].segment = ci;
        let mut segment = self.segment(ci);
        if segment.cost < 0 {
            let key = self.nodes.items[n].key.0;
            segment.last_tile = key.tile;
            segment.last_td = key.td;
            self.set_segment(ci, segment);
        }
    }
    /// `DisableCache`.
    fn disable_cache(&mut self, disable: bool) {
        self.disable_cache = disable;
    }

    // `CYapfDestinationRailBase` and the destination providers.

    /// `CYapfDestinationRailBase::SetDestination`.
    fn set_destination_base(&mut self, override_rail_type: bool) {
        self.compatible_railtypes = RailTypes(self.v.get_compatible_railtypes());
        if override_rail_type {
            self.compatible_railtypes = RailTypes(
                self.compatible_railtypes.0
                    | (self.s.all_compatible_railtypes)(self.v.get_railtypes()),
            );
        }
    }
    /// `PfDetectDestination(Node &)`: called by YAPF to detect if node ends in
    /// the desired destination.
    #[inline]
    fn pf_detect_destination(&self, n: &CYapfRailNode) -> bool {
        T::PfDestination::pf_detect_destination_tile(
            self,
            self.get_last_tile(n),
            self.get_last_trackdir(n),
        )
    }

    // The follow classes.

    /// `PfFollowNode`: called by YAPF to move from the given node to the next
    /// tile. For each reachable trackdir on the new tile creates new node,
    /// initializes it and adds it to the open list by calling `AddNewNode`.
    #[inline]
    fn pf_follow_node(&mut self, old_node: usize) {
        let old = self.nodes.items[old_node];
        if T::DO_TRACK_MASKING {
            // `CYapfFollowAnySafeTileRailT`.
            let mut follower = CFollowTrackT::vehicle(
                self.s,
                TransportType::TRANSPORT_RAIL,
                Some(self.v.vehicle()),
                self.compatible_railtypes,
                T::ALLOW_90DEG_TURNS,
            );
            if follower.follow(
                self.s,
                self.get_last_tile(&old),
                self.get_last_trackdir(&old),
            ) && follower.mask_reserved_tracks(self.pbs)
            {
                self.add_multiple_nodes(old_node, &follower);
            }
        } else {
            // `CYapfFollowRailT` and `CYapfFollowAnyDepotRailT`.
            let mut follower = CFollowTrackT::vehicle(
                self.s,
                TransportType::TRANSPORT_RAIL,
                Some(self.v.vehicle()),
                RailTypes::INVALID_RAILTYPES,
                T::ALLOW_90DEG_TURNS,
            );
            if follower.follow(
                self.s,
                self.get_last_tile(&old),
                self.get_last_trackdir(&old),
            ) {
                self.add_multiple_nodes(old_node, &follower);
            }
        }
    }

    // `CYapfRailBase`.

    /// `PruneIntermediateNodeBranch`: in some cases an intermediate node branch
    /// should be pruned. The most prominent case is when a red EOL signal is
    /// encountered, but there was a segment change (e.g. a rail type change)
    /// before that. If the branch would not be pruned, the rail type change
    /// location would remain the best intermediate node, and thus the vehicle
    /// would still go towards the red EOL signal. `n` is the node under
    /// evaluation, whose segment and parent its storage slot shares.
    fn prune_intermediate_node_branch(&mut self, n: &CYapfRailNode) {
        let mut intermediate_on_branch = false;
        // `n` is the new node, which is not yet in the open list: no
        // intermediate node can be it.
        let mut current = Some(*n);
        let mut current_index: Option<usize> = None;
        while let Some(node) = current {
            if self
                .segment(node.segment)
                .end_segment_reason
                .test(EndSegmentReason::CHOICE_FOLLOWS)
            {
                break;
            }
            if current_index.is_some() && current_index == self.best_intermediate_node {
                intermediate_on_branch = true;
            }
            current_index = node.parent;
            current = node.parent.map(|p| self.nodes.items[p]);
        }
        if intermediate_on_branch {
            self.best_intermediate_node = current_index;
        }
    }

    // `CYapfReserveTrack`.

    /// `FindSafePositionProc`.
    fn find_safe_position_proc(&mut self, tile: TileIndex, td: Trackdir) -> bool {
        if self
            .pbs
            .is_safe_waiting_position(self.v, tile, td, true, !T::ALLOW_90DEG_TURNS)
        {
            self.res_dest_tile = tile;
            self.res_dest_td = td;
            return false; // Stop iterating segment
        }
        true
    }
    /// `ReserveRailStationPlatform`: reserve a railway platform. `tile`
    /// contains the failed tile on abort.
    fn reserve_rail_station_platform(&mut self, tile: &mut TileIndex, dir: DiagDirection) -> bool {
        let start = *tile;
        let diff = self
            .world
            .dimensions
            .tile_offs_by_diag_dir(dir, |ok| self.check(ok));

        loop {
            if self.world.has_station_reservation(*tile) {
                return false;
            }
            (self.pbs.0.set_station)(tile.0, true);
            (self.s.mark_dirty)(tile.0);
            *tile = self
                .world
                .dimensions
                .tile_add(*tile, diff, |ok| self.check(ok));
            if !(self.world.is_compatible_train_station_tile(*tile, start)
                && *tile != self.origin_tile)
            {
                break;
            }
        }

        let st = (self.pbs.0.station)(start.0);
        crate::animated_tile::rail_event(st, start.0, SAT_PATH_RESERVATION, SRT_PATH_RESERVATION);

        true
    }
    /// `ReserveSingleTrack`: try to reserve a single track/platform.
    fn reserve_single_track(&mut self, mut tile: TileIndex, td: Trackdir) -> bool {
        let rev_td = Trackdir(reverse_trackdir(td.0));
        if self.world.is_rail_station_tile(tile) {
            if !self.reserve_rail_station_platform(
                &mut tile,
                DiagDirection(trackdir_to_exitdir(rev_td.0)),
            ) {
                /* Platform could not be reserved, undo. */
                self.res_fail_tile = tile;
                self.res_fail_td = td;
            }
        } else {
            if !self
                .pbs
                .try_reserve_rail_track(tile, Track(trackdir_to_track(td.0)), true)
            {
                /* Tile couldn't be reserved, undo. */
                self.res_fail_tile = tile;
                self.res_fail_td = td;
                return false;
            }

            /* Green path signal opposing the path? Turn to red. */
            if (self.pbs.0.has_pbs)(tile.0, rev_td.0)
                && SignalState(self.world.signal_state_by_trackdir(tile.0, rev_td.0))
                    == SignalState::SIGNAL_STATE_GREEN
            {
                self.signals_set_to_red.push((tile, rev_td));
                self.set_signal_state_by_trackdir(tile, rev_td, SignalState::SIGNAL_STATE_RED);
                (self.s.mark_dirty)(tile.0);
                witness::hit(witness::RAIL_SIGNALS_RED);
            }

            if self.world.is_rail_waypoint_tile(tile) {
                let st = (self.pbs.0.station)(tile.0);
                crate::animated_tile::rail_event(
                    st,
                    tile.0,
                    SAT_PATH_RESERVATION,
                    SRT_PATH_RESERVATION,
                );
            }
        }

        tile != self.res_dest_tile || td != self.res_dest_td
    }
    /// `UnreserveSingleTrack`: unreserve a single track/platform. Stops when
    /// the previous failure is reached.
    fn unreserve_single_track(&mut self, mut tile: TileIndex, td: Trackdir) -> bool {
        if self.world.is_rail_station_tile(tile) {
            let start = tile;
            let diff = self.world.dimensions.tile_offs_by_diag_dir(
                DiagDirection(trackdir_to_exitdir(reverse_trackdir(td.0))),
                |ok| self.check(ok),
            );
            while (tile != self.res_fail_tile || td != self.res_fail_td)
                && self.world.is_compatible_train_station_tile(tile, start)
            {
                (self.pbs.0.set_station)(tile.0, false);
                tile = self
                    .world
                    .dimensions
                    .tile_add(tile, diff, |ok| self.check(ok));
            }
        } else if tile != self.res_fail_tile || td != self.res_fail_td {
            self.pbs
                .unreserve_rail_track(tile, Track(trackdir_to_track(td.0)));
        }
        (tile != self.res_dest_tile || td != self.res_dest_td)
            && (tile != self.res_fail_tile || td != self.res_fail_td)
    }
    /// `SetSignalStateByTrackdir`.
    fn set_signal_state_by_trackdir(&self, tile: TileIndex, td: Trackdir, state: SignalState) {
        // SAFETY: The PBS table names the process-lifetime rail map table.
        let rail = unsafe { &*self.pbs.0.rail };
        (rail.set_signal_state)(tile.0, td.0, state.0);
    }
    /// `SetReservationTarget`: set the target to where the reservation should be extended.
    #[inline]
    fn set_reservation_target(&mut self, node: usize, tile: TileIndex, td: Trackdir) {
        self.res_dest_node = node;
        self.res_dest_tile = tile;
        self.res_dest_td = td;
    }
    /// `FindSafePositionOnNode`: check the node for a possible reservation target.
    #[inline]
    fn find_safe_position_on_node(&mut self, node: usize) {
        let parent = self.nodes.items[node].parent;
        self.world.assert_with(|| parent.is_some());

        /* We will never pass more than two signals, no need to check for a safe tile. */
        if parent.is_some_and(|p| self.nodes.items[p].num_signals_passed >= 2) {
            return;
        }

        if !self.iterate_tiles(node, Self::find_safe_position_proc) {
            self.res_dest_node = node;
        }
    }
    /// `TryReservePath`: try to reserve the path till the reservation target.
    fn try_reserve_path(
        &mut self,
        mut target: Option<&mut PBSTileInfo>,
        origin: TileIndex,
    ) -> bool {
        self.res_fail_tile = TileIndex::INVALID;
        self.origin_tile = origin;

        if let Some(target) = target.as_deref_mut() {
            target.tile = self.res_dest_tile;
            target.trackdir = self.res_dest_td;
            target.okay = false;
        }

        witness::hit(witness::RAIL_RESERVE_ATTEMPTS);
        /* Don't bother if the target is reserved. */
        if !self
            .pbs
            .is_waiting_position_free(self.v, self.res_dest_tile, self.res_dest_td, false)
        {
            witness::hit(witness::RAIL_TARGET_BUSY);
            return false;
        }

        self.signals_set_to_red.clear();
        let mut node = self.res_dest_node;
        while let Some(parent) = self.nodes.items[node].parent {
            self.iterate_tiles(node, Self::reserve_single_track);
            if self.res_fail_tile != TileIndex::INVALID {
                /* Reservation failed, undo. */
                let mut fail_node = self.res_dest_node;
                let stop_tile = self.res_fail_tile;
                loop {
                    /* If this is the node that failed, stop at the failed tile. */
                    self.res_fail_tile = if fail_node == node {
                        stop_tile
                    } else {
                        TileIndex::INVALID
                    };
                    self.iterate_tiles(fail_node, Self::unreserve_single_track);
                    if fail_node == node {
                        break;
                    }
                    let Some(next) = self.nodes.items[fail_node].parent else {
                        break;
                    };
                    fail_node = next;
                }

                /* Re-instate green path signals we turned to red. */
                for &(sig_tile, td) in &self.signals_set_to_red {
                    self.set_signal_state_by_trackdir(
                        sig_tile,
                        td,
                        SignalState::SIGNAL_STATE_GREEN,
                    );
                    witness::hit(witness::RAIL_SIGNALS_RESTORED);
                }

                witness::hit(witness::RAIL_ROLLBACK);
                return false;
            }
            node = parent;
        }

        if let Some(target) = target {
            target.okay = true;
        }

        if self.can_use_global_cache(self.res_dest_node) {
            yapf_notify_track_layout_change(TileIndex::INVALID, Track::INVALID_TRACK);
        }

        witness::hit(witness::RAIL_RESERVE_SUCCESS);
        true
    }
    /// `CYapfRailNode::IterateTiles`, with the reservation's member function.
    fn iterate_tiles(
        &mut self,
        node: usize,
        func: fn(&mut Self, TileIndex, Trackdir) -> bool,
    ) -> bool {
        let mut follower = CFollowTrackT::vehicle(
            self.s,
            TransportType::TRANSPORT_RAIL,
            Some(self.v.vehicle()),
            self.compatible_railtypes,
            T::ALLOW_90DEG_TURNS,
        );
        let n = self.nodes.items[node];
        let mut cur = n.key.0.tile;
        let mut cur_td = n.key.0.td;

        while cur != self.get_last_tile(&n) || cur_td != self.get_last_trackdir(&n) {
            if !func(self, cur, cur_td) {
                return false;
            }

            if !follower.follow(self.s, cur, cur_td) {
                break;
            }
            cur = follower.new_tile;
            self.world.assert_with(|| {
                kill_first_bit_u16(follower.new_td_bits.0) == TrackdirBits::TRACKDIR_BIT_NONE.0
            });
            cur_td = Trackdir(find_first_trackdir(follower.new_td_bits.0));
        }

        func(self, cur, cur_td)
    }

    /// `DumpBase` into the words `yapf_rail_dump_state` reads: the file number,
    /// `num_steps`, then 17 words per node with its segment's identity.
    fn dump_base(&self, file: u32) -> Vec<u32> {
        let mut words = vec![file, self.num_steps.cast_unsigned()];
        let mut identities = std::collections::HashMap::new();
        for (i, n) in self.nodes.items.iter().enumerate() {
            let segment = self.segment(n.segment);
            let next = index_u32(identities.len());
            let identity = *identities.entry(n.segment).or_insert(next);
            words.extend_from_slice(&[
                index_u32(i),
                n.key.0.tile.0,
                u32::from(n.key.0.td.0),
                n.parent.map_or(u32::MAX, index_u32),
                n.cost.cast_unsigned(),
                n.estimate.cast_unsigned(),
                segment.last_tile.0,
                u32::from(segment.last_td.0),
                segment.cost.cast_unsigned(),
                segment.last_signal_tile.0,
                u32::from(segment.last_signal_td.0),
                u32::from(segment.end_segment_reason.0),
                u32::from(n.num_signals_passed),
                u32::from(n.flags_u.get(CYapfRailNodeFlags::TARGET_SEEN))
                    | (u32::from(n.flags_u.get(CYapfRailNodeFlags::CHOICE_SEEN)) << 1)
                    | (u32::from(n.flags_u.get(CYapfRailNodeFlags::LAST_SIGNAL_WAS_RED)) << 2),
                u32::from(n.last_red_signal_type.0),
                u32::from(n.last_signal_type.0),
                identity,
            ]);
        }
        words
    }
}

/// A node list or cache index as the 32-bit word segment references and the
/// dump carry; storage never holds 2^32 items.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the original's deque positions; a search never stores 2^32 items"
)]
fn index_u32(index: usize) -> u32 {
    index as u32
}

/// `stGetGlobalCache`: delete the cache sometimes...
fn st_get_global_cache<T: Types>() {
    // SAFETY: Serialized under the game-state mutex; the borrows end here.
    unsafe {
        let counter = *S_RAIL_CHANGE_COUNTER.get();
        let cache = &mut (*GLOBAL_CACHES.get())[T::GLOBAL_CACHE];
        if cache.last_rail_change_counter != counter {
            cache.last_rail_change_counter = counter;
            cache.c.flush();
            witness::hit(witness::RAIL_CACHE_FLUSHES);
        }
    }
}

/// `CYapfCostBase::stSlopeCost`: does the given track direction on the given
/// tile yield an uphill penalty?
#[inline]
fn st_slope_cost(world: &World<'_>, tile: TileIndex, td: Trackdir) -> bool {
    if is_diagonal_trackdir(td.0) {
        if world.kind(tile) == TileType::MP_TUNNELBRIDGE && world.bridge(tile.0) {
            /* it is bridge ramp, check if we are entering the bridge */
            if world.tunnel_bridge_dir(tile.0) != trackdir_to_exitdir(td.0) {
                return false; // no, we are leaving it, no penalty
            }
            /* we are entering the bridge */
            let tile_slope = world.tile_slope(tile);
            let axis = diag_dir_to_axis(DiagDirection(world.tunnel_bridge_dir(tile.0)), |ok| {
                world.check(ok);
            });
            return !crate::bridge::has_bridge_flat_ramp(world.shared, tile_slope.0, axis.0);
        }
        /* not bridge ramp */
        if world.kind(tile) == TileType::MP_TUNNELBRIDGE && world.tunnel(tile.0) {
            return false; // tunnel entry/exit doesn't slope
        }
        let tile_slope = world.tile_slope(tile);
        return is_uphill_trackdir(tile_slope, td.0, |ok| world.check(ok)); // slopes uphill => apply penalty
    }
    false
}

/// `DumpState`: the two searches' states, through the native `DumpTarget`
/// that #725 retires with the template headers.
fn dump_state<T: Types>(pf1: &CYapfRailT<'_, T>, pf2: &CYapfRailT<'_, T>) {
    for (file, pf) in [(1, pf1), (2, pf2)] {
        let words = pf.dump_base(file);
        // SAFETY: The words live for the synchronous call.
        unsafe { (pf.s.yapf_rail_dump_state)(words.as_ptr(), index_u32(words.len())) };
    }
}

/// The `_debug_desync_level >= 2` double run of the `st*` functions.
fn desync_level() -> i32 {
    crate::debug::level(|d| d.desync)
}

/// `"T"` or `"F"`.
fn tf(value: bool) -> &'static str {
    if value { "T" } else { "F" }
}

// `CYapfFollowRailT`.
impl<'a, T: Types<PfDestination = CYapfDestinationTileOrStationRailT>> CYapfRailT<'a, T> {
    /// `stChooseRailTrack`. `ChooseRailTrack` ignores its tile, entry direction
    /// and tracks, so neither function takes them.
    fn st_choose_rail_track(
        s: &'a Services,
        v: crate::train::Handle,
        path_found: &mut bool,
        reserve_track: bool,
        target: Option<&mut PBSTileInfo>,
        dest: Option<&mut TileIndex>,
    ) -> Trackdir {
        /* create pathfinder instance */
        let mut pf1 = Self::new(s, v);
        let result1;

        if desync_level() < 2 {
            result1 = pf1.choose_rail_track(path_found, reserve_track, target, dest);
        } else {
            result1 = pf1.choose_rail_track(path_found, false, None, None);
            let mut pf2 = Self::new(s, v);
            pf2.disable_cache(true);
            let result2 = pf2.choose_rail_track(path_found, reserve_track, target, dest);
            if result1 != result2 {
                debug!(
                    desync,
                    2, "warning: ChooseRailTrack cache mismatch: {} vs {}", result1.0, result2.0
                );
                dump_state(&pf1, &pf2);
            }
        }

        result1
    }
    /// `ChooseRailTrack`.
    fn choose_rail_track(
        &mut self,
        path_found: &mut bool,
        reserve_track: bool,
        mut target: Option<&mut PBSTileInfo>,
        dest: Option<&mut TileIndex>,
    ) -> Trackdir {
        if let Some(target) = target.as_deref_mut() {
            target.tile = TileIndex::INVALID;
        }
        let mut dest = dest;
        if let Some(dest) = dest.as_deref_mut() {
            *dest = TileIndex::INVALID;
        }

        /* set origin and destination nodes */
        let origin = self
            .pbs
            .follow_train_reservation(self.v, None::<&mut crate::train::Handle>);
        self.set_origin(
            origin.tile,
            origin.trackdir,
            TileIndex::INVALID,
            Trackdir::INVALID_TRACKDIR,
            1,
        );
        self.set_treat_first_red_two_way_signal_as_eol(true);
        CYapfDestinationTileOrStationRailT::set_destination(self, false);

        /* find the best path */
        *path_found = self.find_path();
        witness::hit(witness::RAIL_CHOOSE);

        /* if path not found - return INVALID_TRACKDIR */
        let mut next_trackdir = Trackdir::INVALID_TRACKDIR;
        if let Some(best) = self.get_best_node() {
            /* reserve till end of path */
            let best_node = self.nodes.items[best];
            self.set_reservation_target(
                best,
                self.get_last_tile(&best_node),
                self.get_last_trackdir(&best_node),
            );

            /* path was found or at least suggested
             * walk through the path back to the origin */
            let mut node = best;
            let mut prev = None;
            while let Some(parent) = self.nodes.items[node].parent {
                let child = node;
                prev = Some(child);
                node = parent;

                self.find_safe_position_on_node(child);
            }

            /* If the best PF node has no parent, then there is no (valid) best next trackdir to return.
             * This occurs when the PF is called while the train is already at its destination. */
            let Some(prev) = prev else {
                return Trackdir::INVALID_TRACKDIR;
            };

            /* return trackdir from the best origin node (one of start nodes) */
            next_trackdir = self.nodes.items[prev].key.0.td;

            if reserve_track && *path_found {
                if let Some(dest) = dest {
                    *dest = self.get_last_tile(&best_node);
                }
                let origin_node = self.nodes.items[node];
                self.try_reserve_path(target, self.get_last_tile(&origin_node));
            }
        }

        /* Treat the path as found if stopped on the first two way signal(s). */
        *path_found |= self.stopped_on_first_two_way_signal;
        next_trackdir
    }
    /// `stCheckReverseTrain`.
    fn st_check_reverse_train(
        s: &'a Services,
        v: crate::train::Handle,
        t1: TileIndex,
        td1: Trackdir,
        t2: TileIndex,
        td2: Trackdir,
        reverse_penalty: i32,
    ) -> bool {
        let mut pf1 = Self::new(s, v);
        let result1 = pf1.check_reverse_train(t1, td1, t2, td2, reverse_penalty);

        if desync_level() >= 2 {
            let mut pf2 = Self::new(s, v);
            pf2.disable_cache(true);
            let result2 = pf2.check_reverse_train(t1, td1, t2, td2, reverse_penalty);
            if result1 != result2 {
                debug!(
                    desync,
                    2,
                    "warning: CheckReverseTrain cache mismatch: {} vs {}",
                    tf(result1),
                    tf(result2)
                );
                dump_state(&pf1, &pf2);
            }
        }

        result1
    }
    /// `CheckReverseTrain`.
    fn check_reverse_train(
        &mut self,
        t1: TileIndex,
        td1: Trackdir,
        t2: TileIndex,
        td2: Trackdir,
        reverse_penalty: i32,
    ) -> bool {
        /* create pathfinder instance
         * set origin and destination nodes */
        self.set_origin(t1, td1, t2, td2, reverse_penalty);
        self.set_treat_first_red_two_way_signal_as_eol(false);
        CYapfDestinationTileOrStationRailT::set_destination(self, false);

        /* find the best path */
        let found = self.find_path();
        witness::hit(witness::RAIL_REVERSE);
        if !found {
            return false;
        }

        /* path was found
         * walk through the path back to the origin */
        let Some(mut node) = self.get_best_node() else {
            return false;
        };
        while let Some(parent) = self.nodes.items[node].parent {
            node = parent;
        }

        /* check if it was reversed origin */
        self.nodes.items[node].cost != 0
    }
}

// `CYapfFollowAnyDepotRailT`.
impl<'a, T: Types<PfDestination = CYapfDestinationAnyDepotRailT>> CYapfRailT<'a, T> {
    /// `stFindNearestDepotTwoWay`.
    fn st_find_nearest_depot_two_way(
        s: &'a Services,
        v: crate::train::Handle,
        t1: TileIndex,
        td1: Trackdir,
        t2: TileIndex,
        td2: Trackdir,
        max_penalty: i32,
        reverse_penalty: i32,
    ) -> FindDepotData {
        let mut pf1 = Self::new(s, v);
        /*
         * With caching enabled it simply cannot get a reliable result when you
         * have limited the distance a train may travel. This means that the
         * cached result does not match uncached result in all cases and that
         * causes desyncs. So disable caching when finding for a depot that is
         * nearby. This only happens with automatic servicing of vehicles,
         * so it will only impact performance when you do not manually set
         * depot orders and you do not disable automatic servicing.
         */
        if max_penalty != 0 {
            pf1.disable_cache(true);
        }
        let result1 =
            pf1.find_nearest_depot_two_way(t1, td1, t2, td2, max_penalty, reverse_penalty);

        if desync_level() >= 2 {
            let mut pf2 = Self::new(s, v);
            pf2.disable_cache(true);
            let result2 =
                pf2.find_nearest_depot_two_way(t1, td1, t2, td2, max_penalty, reverse_penalty);
            if result1.tile != result2.tile || (result1.reverse != result2.reverse) {
                debug!(
                    desync,
                    2,
                    "warning: FindNearestDepotTwoWay cache mismatch: {} vs {}",
                    tf(result1.tile != TileIndex::INVALID),
                    tf(result2.tile != TileIndex::INVALID)
                );
                dump_state(&pf1, &pf2);
            }
        }

        result1
    }
    /// `FindNearestDepotTwoWay`.
    fn find_nearest_depot_two_way(
        &mut self,
        t1: TileIndex,
        td1: Trackdir,
        t2: TileIndex,
        td2: Trackdir,
        max_penalty: i32,
        reverse_penalty: i32,
    ) -> FindDepotData {
        /* set origin and destination nodes */
        self.set_origin(t1, td1, t2, td2, reverse_penalty);
        self.set_treat_first_red_two_way_signal_as_eol(true);
        CYapfDestinationAnyDepotRailT::set_destination(self, false);
        self.set_max_cost(max_penalty);

        /* find the best path */
        let found = self.find_path();
        witness::hit(witness::RAIL_DEPOT);
        if !found {
            return FindDepotData::default();
        }

        /* Some path found. */
        let Some(n) = self.get_best_node() else {
            return FindDepotData::default();
        };

        /* walk through the path back to the origin */
        let mut node = n;
        while let Some(parent) = self.nodes.items[node].parent {
            node = parent;
        }

        /* if the origin node is our front vehicle tile/Trackdir then we didn't reverse
         * but we can also look at the cost (== 0 -> not reversed, == reverse_penalty -> reversed) */
        let best = self.nodes.items[n];
        FindDepotData::new(
            self.get_last_tile(&best),
            best.cost.cast_unsigned(),
            self.nodes.items[node].cost != 0,
        )
    }
}

// `CYapfFollowAnySafeTileRailT`.
impl<'a, T: Types<PfDestination = CYapfDestinationAnySafeTileRailT>> CYapfRailT<'a, T> {
    /// `stFindNearestSafeTile`.
    fn st_find_nearest_safe_tile(
        s: &'a Services,
        v: crate::train::Handle,
        t1: TileIndex,
        td: Trackdir,
        override_railtype: bool,
    ) -> bool {
        /* Create pathfinder instance */
        let mut pf1 = Self::new(s, v);
        let result1;
        if desync_level() < 2 {
            result1 = pf1.find_nearest_safe_tile(t1, td, override_railtype, false);
        } else {
            let result2 = pf1.find_nearest_safe_tile(t1, td, override_railtype, true);
            let mut pf2 = Self::new(s, v);
            pf2.disable_cache(true);
            result1 = pf2.find_nearest_safe_tile(t1, td, override_railtype, false);
            if result1 != result2 {
                debug!(
                    desync,
                    2,
                    "warning: FindSafeTile cache mismatch: {} vs {}",
                    tf(result2),
                    tf(result1)
                );
                dump_state(&pf1, &pf2);
            }
        }

        result1
    }
    /// `FindNearestSafeTile`.
    fn find_nearest_safe_tile(
        &mut self,
        t1: TileIndex,
        td: Trackdir,
        override_railtype: bool,
        dont_reserve: bool,
    ) -> bool {
        /* Set origin and destination. */
        self.set_origin(t1, td, TileIndex::INVALID, Trackdir::INVALID_TRACKDIR, 0);
        self.set_treat_first_red_two_way_signal_as_eol(true);
        CYapfDestinationAnySafeTileRailT::set_destination(self, override_railtype);

        let found = self.find_path();
        witness::hit(witness::RAIL_SAFE);
        if !found {
            return false;
        }

        /* Found a destination, set as reservation target. */
        let Some(mut node) = self.get_best_node() else {
            return false;
        };
        let best = self.nodes.items[node];
        self.set_reservation_target(
            node,
            self.get_last_tile(&best),
            self.get_last_trackdir(&best),
        );

        /* Walk through the path back to the origin. */
        while let Some(parent) = self.nodes.items[node].parent {
            let prev = node;
            node = parent;

            self.find_safe_position_on_node(prev);
        }

        let origin = self.nodes.items[node];
        dont_reserve || self.try_reserve_path(None, self.get_last_tile(&origin))
    }
}

/// `YapfTrainChooseTrack`: finds the best path for given train using YAPF.
/// Its eight parameters after the service table are the original's list, the
/// one place the seven-parameter bound yields to the original's signature.
/// The tile and entry direction pass unused, as in the original.
pub(crate) fn yapf_train_choose_track(
    s: &Services,
    v: crate::train::Handle,
    _tile: TileIndex,
    _enterdir: DiagDirection,
    tracks: TrackBits,
    path_found: &mut bool,
    reserve_track: bool,
    target: Option<&mut PBSTileInfo>,
    dest: Option<&mut TileIndex>,
) -> Track {
    let td_ret = if (s.forbid_90_deg)() {
        CYapfRailT::<CYapfRailNo90>::st_choose_rail_track(
            s,
            v,
            path_found,
            reserve_track,
            target,
            dest,
        )
    } else {
        CYapfRailT::<CYapfRail>::st_choose_rail_track(s, v, path_found, reserve_track, target, dest)
    };

    if td_ret == Trackdir::INVALID_TRACKDIR {
        Track(crate::track_func::find_first_track(tracks.0))
    } else {
        Track(trackdir_to_track(td_ret.0))
    }
}

/// `YapfTrainCheckReverse`: returns true if it is better to reverse the train
/// before leaving station.
pub(crate) fn yapf_train_check_reverse(s: &Services, v: crate::train::Handle) -> bool {
    let pbs = Pbs::acquire(s);
    // SAFETY: The PBS table names the process-lifetime train table.
    let leaves = unsafe { &*pbs.0.train };
    let map = s.map();
    let last_veh = v.last();

    /* get trackdirs of both ends */
    let td = crate::train::get_vehicle_trackdir(leaves, s, v);
    let td_rev = Trackdir(reverse_trackdir(
        crate::train::get_vehicle_trackdir(leaves, s, last_veh).0,
    ));

    /* tiles where front and back are */
    let mut tile = TileIndex(v.vehicle().tile());
    let mut tile_rev = TileIndex(last_veh.vehicle().tile());

    let mut reverse_penalty = 0_i32;

    if TrackBits(v.track()) == TrackBits::TRACK_BIT_WORMHOLE {
        /* front in tunnel / on bridge */
        let dir_into_wormhole = World::new(s).tunnel_bridge_dir(tile.0);

        if trackdir_to_exitdir(td.0) == dir_into_wormhole {
            // SAFETY: The PBS table names the process-lifetime rail map table.
            tile = TileIndex((unsafe { &*pbs.0.rail }.other_tunnel_bridge_end)(tile.0));
        }
        /* Now 'tile' is the tunnel entry/bridge ramp the train will reach when driving forward */

        /* Current position of the train in the wormhole */
        let cur_tile = virt_tile(map, v);

        /* Add distance to drive in the wormhole as penalty for the forward path, i.e. bonus for the reverse path
         * Note: Negative penalties are ok for the start tile. */
        reverse_penalty = reverse_penalty.wrapping_sub(
            map.distance_manhattan(cur_tile, tile)
                .wrapping_mul(YAPF_TILE_LENGTH.cast_unsigned())
                .cast_signed(),
        );
    }

    if TrackBits(last_veh.track()) == TrackBits::TRACK_BIT_WORMHOLE {
        /* back in tunnel / on bridge */
        let dir_into_wormhole = World::new(s).tunnel_bridge_dir(tile_rev.0);

        if trackdir_to_exitdir(td_rev.0) == dir_into_wormhole {
            // SAFETY: The PBS table names the process-lifetime rail map table.
            tile_rev = TileIndex((unsafe { &*pbs.0.rail }.other_tunnel_bridge_end)(
                tile_rev.0,
            ));
        }
        /* Now 'tile_rev' is the tunnel entry/bridge ramp the train will reach when reversing */

        /* Current position of the last wagon in the wormhole */
        let cur_tile = virt_tile(map, last_veh);

        /* Add distance to drive in the wormhole as penalty for the revere path. */
        reverse_penalty = reverse_penalty.wrapping_add(
            map.distance_manhattan(cur_tile, tile_rev)
                .wrapping_mul(YAPF_TILE_LENGTH.cast_unsigned())
                .cast_signed(),
        );
    }

    /* slightly hackish: If the pathfinders finds a path, the cost of the first node is tested to distinguish between forward- and reverse-path. */
    if reverse_penalty == 0 {
        reverse_penalty = 1;
    }

    let reverse = if (s.forbid_90_deg)() {
        CYapfRailT::<CYapfRailNo90>::st_check_reverse_train(
            s,
            v,
            tile,
            td,
            tile_rev,
            td_rev,
            reverse_penalty,
        )
    } else {
        CYapfRailT::<CYapfRail>::st_check_reverse_train(
            s,
            v,
            tile,
            td,
            tile_rev,
            td_rev,
            reverse_penalty,
        )
    };

    if reverse {
        witness::hit(witness::RAIL_REVERSE_CHOSEN);
    }
    reverse
}

/// `TileVirtXY(v->x_pos, v->y_pos)`.
#[allow(
    clippy::cast_sign_loss,
    reason = "TileVirtXY takes the int x_pos and y_pos as its uint parameters"
)]
fn virt_tile(map: crate::map_func::Map, v: crate::train::Handle) -> TileIndex {
    map.tile_virt_xy(v.vehicle().x() as u32, v.vehicle().y() as u32)
}

/// `YapfTrainFindNearestDepot`: used when user sends train to the nearest
/// depot or if train needs servicing using YAPF.
pub(crate) fn yapf_train_find_nearest_depot(
    s: &Services,
    v: crate::train::Handle,
    max_penalty: i32,
) -> FindDepotData {
    let pbs = Pbs::acquire(s);
    // SAFETY: The PBS table names the process-lifetime train table.
    let leaves = unsafe { &*pbs.0.train };
    let last_veh = v.last();

    let origin = pbs.follow_train_reservation(v, None::<&mut crate::train::Handle>);
    let last_tile = TileIndex(last_veh.vehicle().tile());
    let td_rev = Trackdir(reverse_trackdir(
        crate::train::get_vehicle_trackdir(leaves, s, last_veh).0,
    ));

    let result = if (s.forbid_90_deg)() {
        CYapfRailT::<CYapfAnyDepotRailNo90>::st_find_nearest_depot_two_way(
            s,
            v,
            origin.tile,
            origin.trackdir,
            last_tile,
            td_rev,
            max_penalty,
            YAPF_INFINITE_PENALTY,
        )
    } else {
        CYapfRailT::<CYapfAnyDepotRail>::st_find_nearest_depot_two_way(
            s,
            v,
            origin.tile,
            origin.trackdir,
            last_tile,
            td_rev,
            max_penalty,
            YAPF_INFINITE_PENALTY,
        )
    };
    if result.tile != TileIndex::INVALID {
        witness::hit(witness::RAIL_DEPOT_FOUND);
    }
    result
}

/// `YapfTrainFindNearestSafeTile`: try to extend the reserved path of a train
/// to the nearest safe tile using YAPF.
pub(crate) fn yapf_train_find_nearest_safe_tile(
    s: &Services,
    v: crate::train::Handle,
    tile: TileIndex,
    td: Trackdir,
    override_railtype: bool,
) -> bool {
    let found = if (s.forbid_90_deg)() {
        CYapfRailT::<CYapfAnySafeTileRailNo90>::st_find_nearest_safe_tile(
            s,
            v,
            tile,
            td,
            override_railtype,
        )
    } else {
        CYapfRailT::<CYapfAnySafeTileRail>::st_find_nearest_safe_tile(
            s,
            v,
            tile,
            td,
            override_railtype,
        )
    };
    if found {
        witness::hit(witness::RAIL_SAFE_FOUND);
    }
    found
}

/// `YapfNotifyTrackLayoutChange`: use this function to notify YAPF that track
/// layout (or signal configuration) has change.
pub(crate) fn yapf_notify_track_layout_change(_tile: TileIndex, _track: Track) {
    // `CSegmentCostCacheBase::NotifyTrackLayoutChange`.
    // SAFETY: Serialized under the game-state mutex; the write ends here.
    unsafe {
        let counter = S_RAIL_CHANGE_COUNTER.get();
        *counter = (*counter).wrapping_add(1);
    }
}

/// The native train controller's [`yapf_train_choose_track`]. `target` and
/// `dest` may be null; a written `target` field overwrites the record's.
///
/// # Safety
/// The caller holds the game-state mutex and supplies the live shared table, a
/// live front train record and valid `path_found`, `target` and `dest`
/// pointers, which nothing else reads during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_yapf_train_choose_track(
    shared: *const Services,
    v: *const OpenTTDVehicle,
    tile: u32,
    enterdir: u8,
    tracks: u8,
    path_found: *mut bool,
    reserve_track: u8,
    target: *mut crate::train_reservation::Pbs,
    dest: *mut u32,
) -> u8 {
    // SAFETY: As this function's contract.
    let (s, path_found) = unsafe { (&*shared, &mut *path_found) };
    let mut info = None;
    // SAFETY: As this function's contract.
    if let Some(record) = unsafe { target.as_ref() } {
        info = Some(PBSTileInfo {
            tile: TileIndex(record.tile),
            trackdir: Trackdir(record.td),
            okay: record.okay != 0,
        });
    }
    // SAFETY: As this function's contract.
    let mut dest_tile = unsafe { dest.as_ref() }.map(|&d| TileIndex(d));
    let track = yapf_train_choose_track(
        s,
        crate::train::Handle::from_record(v.cast_mut()),
        TileIndex(tile),
        DiagDirection(enterdir),
        TrackBits(tracks),
        path_found,
        reserve_track != 0,
        info.as_mut(),
        dest_tile.as_mut(),
    );
    // SAFETY: As this function's contract.
    if let (Some(record), Some(info)) = (unsafe { target.as_mut() }, info) {
        record.tile = info.tile.0;
        record.td = info.trackdir.0;
        record.okay = u8::from(info.okay);
    }
    // SAFETY: As this function's contract.
    if let (Some(d), Some(dest_tile)) = (unsafe { dest.as_mut() }, dest_tile) {
        *d = dest_tile.0;
    }
    track.0
}

/// The native train controller's [`yapf_train_check_reverse`].
///
/// # Safety
/// The caller holds the game-state mutex and supplies the live shared table
/// and a live front train record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_yapf_train_check_reverse(
    shared: *const Services,
    v: *const OpenTTDVehicle,
) -> u8 {
    // SAFETY: As this function's contract.
    let s = unsafe { &*shared };
    u8::from(yapf_train_check_reverse(
        s,
        crate::train::Handle::from_record(v.cast_mut()),
    ))
}

/// The native train controller's [`yapf_train_find_nearest_depot`].
///
/// # Safety
/// The caller holds the game-state mutex and supplies the live shared table
/// and a live front train record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_yapf_train_find_nearest_depot(
    shared: *const Services,
    v: *const OpenTTDVehicle,
    max_penalty: i32,
) -> FindDepotDataRecord {
    // SAFETY: As this function's contract.
    let s = unsafe { &*shared };
    FindDepotDataRecord::from(yapf_train_find_nearest_depot(
        s,
        crate::train::Handle::from_record(v.cast_mut()),
        max_penalty,
    ))
}

/// The native train controller's [`yapf_train_find_nearest_safe_tile`].
///
/// # Safety
/// The caller holds the game-state mutex and supplies the live shared table
/// and a live front train record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_yapf_train_find_nearest_safe_tile(
    shared: *const Services,
    v: *const OpenTTDVehicle,
    tile: u32,
    td: u8,
    override_railtype: u8,
) -> u8 {
    // SAFETY: As this function's contract.
    let s = unsafe { &*shared };
    u8::from(yapf_train_find_nearest_safe_tile(
        s,
        crate::train::Handle::from_record(v.cast_mut()),
        TileIndex(tile),
        Trackdir(td),
        override_railtype != 0,
    ))
}

/// The native track layout notification's [`yapf_notify_track_layout_change`].
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_invalidate(tile: u32, track: u8) {
    yapf_notify_track_layout_change(TileIndex(tile), Track(track));
    witness::hit(witness::RAIL_INVALIDATIONS);
}

/// What the native rail search tests compare after one search: the entry's
/// result and the statistics `FindPath` prints, with the best node's cost and
/// distance `-1` when no destination was found.
#[cfg(feature = "native-test-support")]
pub(crate) struct NativeTestSearch {
    pub(crate) found: bool,
    pub(crate) td: Trackdir,
    pub(crate) num_steps: i32,
    pub(crate) open: i32,
    pub(crate) closed: i32,
    pub(crate) cache_hits: i32,
    pub(crate) cost_calcs: i32,
    pub(crate) cost: i32,
    pub(crate) distance: i32,
}

/// The native tests' search: the production `ChooseRailTrack` of `CYapfRail`
/// without a reservation target, or `FindNearestSafeTile` of
/// `CYapfAnySafeTileRail` from `tile` and `td`, each over one pathfinder
/// instance as `st*` builds it below the desync level.
#[cfg(feature = "native-test-support")]
pub(crate) fn native_test_search(
    s: &Services,
    v: crate::train::Handle,
    safe_tile: bool,
    tile: TileIndex,
    td: Trackdir,
    reserve_track: bool,
) -> NativeTestSearch {
    fn statistics<T: Types>(pf: &CYapfRailT<'_, T>, found: bool, td: Trackdir) -> NativeTestSearch {
        NativeTestSearch {
            found,
            td,
            num_steps: pf.num_steps,
            open: pf.nodes.open_count(),
            closed: pf.nodes.closed_count(),
            cache_hits: pf.stats_cache_hits,
            cost_calcs: pf.stats_cost_calcs,
            cost: pf.best_dest_node.map_or(-1, |n| pf.nodes.items[n].cost),
            distance: pf.best_dest_node.map_or(-1, |n| {
                let n = &pf.nodes.items[n];
                n.estimate.wrapping_sub(n.cost)
            }),
        }
    }
    if safe_tile {
        let mut pf = CYapfRailT::<CYapfAnySafeTileRail>::new(s, v);
        let found = pf.find_nearest_safe_tile(tile, td, false, false);
        statistics(&pf, found, Trackdir::INVALID_TRACKDIR)
    } else {
        let mut pf = CYapfRailT::<CYapfRail>::new(s, v);
        let mut path_found = false;
        let td = pf.choose_rail_track(&mut path_found, reserve_track, None, None);
        statistics(&pf, path_found, td)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CYapfAnyDepotRail, CYapfNodeKeyTrackDir, CYapfRail, CYapfRailNo90, CYapfRailNode,
        CYapfRailSegment, CYapfRailSegmentKey, CYapfRailT, CachedDataRef, EndSegmentReason,
        GLOBAL_CACHES, PfDestination, S_RAIL_CHANGE_COUNTER, Types,
        yapf_notify_track_layout_change,
    };
    use crate::follow_track::CFollowTrackT;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
    use crate::pbs::PbsServices;
    use crate::rail_type::RailTypes;
    use crate::services::Services;
    use crate::settings_type::SettingAddresses;
    use crate::signal_type::SignalType;
    use crate::test_tables::{unreachable0, unreachable1, unreachable2};
    use crate::tile_type::{TileIndex, TileType};
    use crate::track_type::{Track, TrackBits, Trackdir, TrackdirBits};
    use crate::transport_type::TransportType;
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::{Mutex, PoisonError};

    /// Serializes the tests that reach the process-wide caches and counter.
    static STATICS: Mutex<()> = Mutex::new(());

    thread_local! {
        static FAILED_ASSERTIONS: Cell<u32> = const { Cell::new(0) };
        static PBS: Cell<*const PbsServices> = const { Cell::new(ptr::null()) };
    }
    extern "C" fn record_assertion(okay: bool) {
        if !okay {
            FAILED_ASSERTIONS.set(FAILED_ASSERTIONS.get() + 1);
        }
    }
    extern "C" fn unexpected_random(_: *mut c_void) -> u32 {
        unreachable!()
    }
    extern "C" fn unexpected_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        unreachable!()
    }
    extern "C" fn pbs_services(_: *const Services) -> *const PbsServices {
        PBS.get()
    }
    extern "C" fn reserved_x(_: u32) -> TrackBits {
        TrackBits::TRACK_BIT_X
    }

    /// One distinct value per setting the search reads, so that two swapped
    /// settings change a cost, which the play scenarios cannot show at
    /// default settings.
    struct Values {
        max_search_nodes: u32,
        crossing: u32,
        curve45: u32,
        curve90: u32,
        depot_reverse: u32,
        doubleslip: u32,
        firstred_exit: u32,
        firstred: u32,
        twoway_eol: bool,
        lastred_exit: u32,
        lastred: u32,
        longer: u32,
        longer_tile: u32,
        max_signals: u32,
        p0: i32,
        p1: i32,
        p2: i32,
        pbs_cross: u32,
        pbs_back: u32,
        pbs_station: u32,
        shorter: u32,
        shorter_tile: u32,
        uphill: u32,
        passing: u32,
    }
    static VALUES: Values = Values {
        max_search_nodes: 777,
        crossing: 3,
        curve45: 5,
        curve90: 7,
        depot_reverse: 11,
        doubleslip: 13,
        firstred_exit: 17,
        firstred: 19,
        twoway_eol: true,
        lastred_exit: 23,
        lastred: 29,
        longer: 31,
        longer_tile: 37,
        max_signals: 4,
        p0: 41,
        p1: 43,
        p2: 47,
        pbs_cross: 53,
        pbs_back: 59,
        pbs_station: 61,
        shorter: 67,
        shorter_tile: 71,
        uphill: 73,
        passing: 79,
    };
    fn catalog() -> SettingAddresses {
        let v = &VALUES;
        SettingAddresses {
            max_search_nodes: &raw const v.max_search_nodes,
            rail_crossing_penalty: &raw const v.crossing,
            rail_curve45_penalty: &raw const v.curve45,
            rail_curve90_penalty: &raw const v.curve90,
            rail_depot_reverse_penalty: &raw const v.depot_reverse,
            rail_doubleslip_penalty: &raw const v.doubleslip,
            rail_firstred_exit_penalty: &raw const v.firstred_exit,
            rail_firstred_penalty: &raw const v.firstred,
            rail_firstred_twoway_eol: &raw const v.twoway_eol,
            rail_lastred_exit_penalty: &raw const v.lastred_exit,
            rail_lastred_penalty: &raw const v.lastred,
            rail_longer_platform_penalty: &raw const v.longer,
            rail_longer_platform_per_tile_penalty: &raw const v.longer_tile,
            rail_look_ahead_max_signals: &raw const v.max_signals,
            rail_look_ahead_signal_p0: &raw const v.p0,
            rail_look_ahead_signal_p1: &raw const v.p1,
            rail_look_ahead_signal_p2: &raw const v.p2,
            rail_pbs_cross_penalty: &raw const v.pbs_cross,
            rail_pbs_signal_back_penalty: &raw const v.pbs_back,
            rail_pbs_station_penalty: &raw const v.pbs_station,
            rail_shorter_platform_penalty: &raw const v.shorter,
            rail_shorter_platform_per_tile_penalty: &raw const v.shorter_tile,
            rail_slope_penalty: &raw const v.uphill,
            rail_station_penalty: &raw const v.passing,
            ..SettingAddresses::NULL
        }
    }

    const SIZE: usize = 64 * 64;
    /// A 64 by 64 map of `MP_CLEAR` tiles that the test writes before its
    /// searches, a train whose ground cache holds a 40-unit length, and
    /// recorded native assertions. Everything lives for the test process.
    struct Fixture {
        s: &'static Services,
        base: *mut BaseTileLayout,
        extended: *mut ExtendedTileLayout,
        v: crate::train::Handle,
    }
    fn fixture() -> Fixture {
        let base: &'static mut [BaseTileLayout] = Box::leak(
            (0..SIZE)
                .map(|_| BaseTileLayout {
                    kind: 0,
                    height: 0,
                    m2: 0,
                    m1: 0,
                    m3: 0,
                    m4: 0,
                    m5: 0,
                })
                .collect(),
        );
        let extended: &'static mut [ExtendedTileLayout] = Box::leak(
            (0..SIZE)
                .map(|_| ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect(),
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
            ..crate::test_tables::MOVEMENT
        }));
        let rail = Box::leak(Box::new(crate::services::RailMapServices {
            rail_reservation: reserved_x,
            ..crate::test_tables::RAIL_MAP
        }));
        let s: &'static Services = Box::leak(Box::new(Services {
            movement,
            settings: Box::leak(Box::new(catalog())),
            pbs_services,
            ..crate::services::fixture(ptr::null_mut(), unexpected_random, unexpected_industry)
        }));
        movement.shared = s;
        PBS.set(Box::leak(Box::new(PbsServices {
            shared: s,
            rail,
            train: ptr::null(),
            depot_bits: unreachable1,
            crossing_bits: unreachable1,
            station_bits: unreachable1,
            wormhole_bits: unreachable1,
            set_depot: unreachable2,
            set_station: unreachable2,
            set_wormhole: unreachable2,
            try_track: unreachable2,
            unreserve_track: unreachable2,
            show_reservation: unreachable0,
            station: unreachable1,
            is_crossing: unreachable1,
            read_train_on_res: unreachable1,
            write_train_on_res: unreachable2,
            assertions: true,
            handle: unreachable1,
            compatible: unreachable1,
            rail90: crate::test_tables::unreachable3,
            blocking: unreachable2,
            has_pbs: unreachable2,
            index: unreachable1,
        })));
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_TRAIN,
        );
        let mut ground = crate::ground_vehicle::GroundState::default();
        ground.cache.cached_total_length = 40;
        // SAFETY: The record is live and owned by this test process.
        unsafe { (*record).ground = Box::leak(Box::new(ground)) };
        Fixture {
            s,
            base: base.as_mut_ptr(),
            extended: extended.as_mut_ptr(),
            v: crate::train::Handle::from_record(record),
        }
    }
    impl Fixture {
        /// Write tile `t`'s type and `m5`; the other fields keep their values.
        fn tile(&self, t: u32, kind: TileType, m5: u8) -> *mut BaseTileLayout {
            // SAFETY: `t` is on the fixture's map, which only this test writes.
            unsafe {
                let tile = self.base.add(t as usize);
                (*tile).kind = kind.0 << 4;
                (*tile).m5 = m5;
                tile
            }
        }
    }
    /// A node on `tile` and `td` over its own local segment ending there.
    fn node<T: Types>(pf: &mut CYapfRailT<'_, T>, tile: TileIndex, td: Trackdir) -> CYapfRailNode {
        let mut n = CYapfRailNode::set(None, tile, td, false);
        let mut segment = CYapfRailSegment::new();
        segment.last_tile = tile;
        segment.last_td = td;
        n.segment = CachedDataRef {
            global: false,
            index: super::index_u32(pf.local_cache.len()),
        };
        pf.local_cache.push(segment);
        n
    }

    #[test]
    fn each_setting_reaches_the_cost_the_original_adds_it_to() {
        let _statics = STATICS.lock().unwrap_or_else(PoisonError::into_inner);
        let f = fixture();
        let mut pf = CYapfRailT::<CYapfRail>::new(f.s, f.v);
        let v = &VALUES;

        assert_eq!(pf.max_search_nodes, 777);
        // `p0 + i * (p1 + i * p2)` for each of the four look-ahead signals.
        assert_eq!(pf.sig_look_ahead_costs, [41, 131, 315, 593]);

        // `CurveCost`: a 90-degree turn, a 45-degree turn, straight on; the
        // no-90 search charges its 90-degree turn as 45 degrees.
        let (x_ne, y_se) = (Trackdir::TRACKDIR_X_NE, Trackdir::TRACKDIR_Y_SE);
        assert_eq!(pf.curve_cost(x_ne, y_se), v.curve90.cast_signed());
        assert_eq!(
            pf.curve_cost(x_ne, Trackdir::TRACKDIR_LOWER_E),
            v.curve45.cast_signed()
        );
        assert_eq!(pf.curve_cost(x_ne, x_ne), 0);
        let no90 = CYapfRailT::<CYapfRailNo90>::new(f.s, f.v);
        assert_eq!(no90.curve_cost(x_ne, y_se), v.curve45.cast_signed());

        // `SwitchCost`: two plain rail tiles whose tracks branch on both sides.
        f.tile(100, TileType::MP_RAILWAY, TrackBits::TRACK_BIT_ALL.0);
        f.tile(99, TileType::MP_RAILWAY, TrackBits::TRACK_BIT_ALL.0);
        assert_eq!(
            pf.switch_cost(
                TileIndex(100),
                TileIndex(99),
                crate::direction_type::DiagDirection::DIAGDIR_NE
            ),
            v.doubleslip.cast_signed()
        );

        // `OneTileCost`: a level crossing on a diagonal trackdir, and a corner.
        f.tile(
            200,
            TileType::MP_ROAD,
            crate::road_map::ROAD_TILE_CROSSING << 6,
        );
        assert_eq!(
            pf.one_tile_cost(TileIndex(200), x_ne),
            100 + v.crossing.cast_signed()
        );
        assert_eq!(
            pf.one_tile_cost(TileIndex(200), Trackdir::TRACKDIR_LOWER_E),
            71
        );

        // `SlopeCost`: raised north and east corners make SLOPE_NE, uphill for X_NE only.
        // SAFETY: Tiles 650 and 714 are on the fixture's map.
        unsafe {
            (*f.base.add(650)).height = 1;
            (*f.base.add(714)).height = 1;
        }
        assert_eq!(pf.slope_cost(TileIndex(650), x_ne), v.uphill.cast_signed());
        assert_eq!(pf.slope_cost(TileIndex(650), Trackdir::TRACKDIR_X_SW), 0);

        // `ReservationCost`: a reserved X track, crossed diagonally and by a
        // corner, then a reserved rail station tile.
        f.tile(300, TileType::MP_RAILWAY, TrackBits::TRACK_BIT_X.0);
        let n = node(&mut pf, TileIndex(300), x_ne);
        assert_eq!(
            pf.reservation_cost(&n, TileIndex(300), x_ne, 2),
            3 * v.pbs_cross.cast_signed()
        );
        assert_eq!(
            pf.reservation_cost(&n, TileIndex(300), Trackdir::TRACKDIR_LOWER_E, 0),
            v.pbs_cross.cast_signed() * 71 / 100
        );
        f.tile(400, TileType::MP_STATION, 0);
        // SAFETY: Tile 400 is on the fixture's map: a rail station (type 0) with its reservation bit.
        unsafe { (*f.extended.add(400)).m6 = 1 << 2 };
        assert_eq!(
            pf.reservation_cost(&n, TileIndex(400), x_ne, 1),
            2 * v.pbs_station.cast_signed()
        );

        // `SignalCost`: a red block, then a red exit signal ahead, each the
        // first signal, with the first look-ahead cost; a path signal behind.
        let signal = |m2: u16, present: u8| {
            let tile = f.tile(
                500,
                TileType::MP_RAILWAY,
                (1 << 6) | TrackBits::TRACK_BIT_X.0,
            );
            // SAFETY: Tile 500 is on the fixture's map.
            unsafe {
                (*tile).m2 = m2;
                (*tile).m3 = present << 4;
                (*tile).m4 = 0;
            }
        };
        signal(SignalType::SIGTYPE_BLOCK.0.into(), 0x8);
        let mut n = node(&mut pf, TileIndex(500), x_ne);
        assert_eq!(
            pf.signal_cost(&mut n, TileIndex(500), x_ne),
            v.p0 + v.firstred.cast_signed()
        );
        signal(SignalType::SIGTYPE_EXIT.0.into(), 0x8);
        let mut n = node(&mut pf, TileIndex(500), x_ne);
        assert_eq!(
            pf.signal_cost(&mut n, TileIndex(500), x_ne),
            v.p0 + v.firstred_exit.cast_signed()
        );
        signal(SignalType::SIGTYPE_PBS.0.into(), 0x4);
        let mut n = node(&mut pf, TileIndex(500), x_ne);
        assert_eq!(
            pf.signal_cost(&mut n, TileIndex(500), x_ne),
            v.pbs_back.cast_signed()
        );
        // A red two-way block signal as the first signal after a choice ends
        // the branch when `rail_firstred_twoway_eol` is set.
        signal(SignalType::SIGTYPE_BLOCK.0.into(), 0xC);
        pf.set_treat_first_red_two_way_signal_as_eol(true);
        let mut n = node(&mut pf, TileIndex(500), x_ne);
        n.flags_u.set(super::CYapfRailNodeFlags::CHOICE_SEEN, true);
        assert_eq!(pf.signal_cost(&mut n, TileIndex(500), x_ne), -1);
        assert!(pf.stopped_on_first_two_way_signal);
        assert!(
            pf.segment(n.segment)
                .end_segment_reason
                .test(EndSegmentReason::DEAD_END)
        );

        // `PlatformLengthPenalty`: the 40-unit train needs three tiles.
        assert_eq!(
            pf.platform_length_penalty(5),
            (v.longer + 2 * v.longer_tile).cast_signed()
        );
        assert_eq!(
            pf.platform_length_penalty(1),
            (v.shorter + 2 * v.shorter_tile).cast_signed()
        );
        assert_eq!(pf.platform_length_penalty(3), 0);
        assert_eq!(FAILED_ASSERTIONS.get(), 0);
    }

    #[test]
    fn child_estimate_below_its_parent_records_the_estimate_assertion() {
        let _statics = STATICS.lock().unwrap_or_else(PoisonError::into_inner);
        let f = fixture();
        let mut pf = CYapfRailT::<CYapfRail>::new(f.s, f.v);
        pf.dest.dest_tile = TileIndex(40 + 40 * 64);
        let mut parent = node(&mut pf, TileIndex(10 + 10 * 64), Trackdir::TRACKDIR_X_NE);
        parent.estimate = 10_000;
        pf.nodes.items.push(parent);
        let mut child = node(&mut pf, TileIndex(9 + 10 * 64), Trackdir::TRACKDIR_X_NE);
        child.parent = Some(0);
        child.cost = 100;
        assert!(<CYapfRail as Types>::PfDestination::pf_calc_estimate(
            &pf, &mut child
        ));
        assert_eq!(FAILED_ASSERTIONS.get(), 1);

        // An estimate at or above the parent's passes.
        pf.nodes.items[0].estimate = child.estimate;
        assert!(<CYapfRail as Types>::PfDestination::pf_calc_estimate(
            &pf, &mut child
        ));
        assert_eq!(FAILED_ASSERTIONS.get(), 1);
    }

    #[test]
    fn invalid_trackdir_into_curve_cost_records_its_assertion() {
        let _statics = STATICS.lock().unwrap_or_else(PoisonError::into_inner);
        let f = fixture();
        // The no-90 search, so that `HasTrackdir` adds no assertion of its own.
        let pf = CYapfRailT::<CYapfRailNo90>::new(f.s, f.v);
        let reversing = Trackdir(6);
        assert_eq!(
            pf.curve_cost(Trackdir::TRACKDIR_X_NE, reversing),
            VALUES.curve45.cast_signed()
        );
        assert_eq!(FAILED_ASSERTIONS.get(), 1);
    }

    #[test]
    fn counter_advance_flushes_only_the_next_constructed_cache() {
        let _statics = STATICS.lock().unwrap_or_else(PoisonError::into_inner);
        let f = fixture();
        let key = CYapfRailSegmentKey::new(CYapfNodeKeyTrackDir::set(
            TileIndex(5),
            Trackdir::TRACKDIR_X_NE,
        ));
        let mut found = false;
        drop(CYapfRailT::<CYapfRail>::new(f.s, f.v));
        drop(CYapfRailT::<CYapfAnyDepotRail>::new(f.s, f.v));
        // SAFETY: STATICS serializes the process-wide caches for this test.
        unsafe {
            let caches = &mut *GLOBAL_CACHES.get();
            caches[<CYapfRail as Types>::GLOBAL_CACHE]
                .c
                .get(key, &mut found);
            caches[<CYapfAnyDepotRail as Types>::GLOBAL_CACHE]
                .c
                .get(key, &mut found);
        }

        // A search keeps its local segment through the change and the next
        // search's construction.
        let mut earlier = CYapfRailT::<CYapfRail>::new(f.s, f.v);
        let mut n = node(&mut earlier, TileIndex(6), Trackdir::TRACKDIR_X_NE);
        n.cost = 1;
        let mut segment = earlier.segment(n.segment);
        segment.cost = 123;
        earlier.set_segment(n.segment, segment);

        yapf_notify_track_layout_change(TileIndex::INVALID, Track::INVALID_TRACK);
        let _later = CYapfRailT::<CYapfRail>::new(f.s, f.v);
        // SAFETY: STATICS serializes the process-wide caches for this test.
        unsafe {
            let caches = &*GLOBAL_CACHES.get();
            assert!(caches[<CYapfRail as Types>::GLOBAL_CACHE].c.heap.is_empty());
            assert_eq!(
                caches[<CYapfAnyDepotRail as Types>::GLOBAL_CACHE]
                    .c
                    .heap
                    .len(),
                1
            );
            assert_eq!(
                caches[<CYapfRail as Types>::GLOBAL_CACHE].last_rail_change_counter,
                *S_RAIL_CHANGE_COUNTER.get()
            );
        }
        assert_eq!(earlier.segment(n.segment).cost, 123);
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
        const NAME: &str = "rail_yapf::tests::closed_node_bettered_by_a_new_node_takes_the_native_unreachable_path";
        const CHILD: &str = "OPENTTD_RAIL_YAPF_NOT_REACHED_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(NOT_REACHED_EXIT));
            return;
        }
        let f = fixture();
        let s: &'static Services = Box::leak(Box::new(Services {
            not_reached,
            ..*f.s
        }));
        // SAFETY: `s` lives for the child process's only search.
        unsafe { crate::services_assert::openttd_rust_assert_bind(no_assertion, s) };
        let mut pf = CYapfRailT::<CYapfAnyDepotRail>::new(s, f.v);
        // No look-ahead signals, so that the new node takes its global segment.
        pf.sig_look_ahead_costs.clear();
        let destination = TileIndex(10 + 10 * 64);
        // The origin, one tile behind, and a closed node on the destination.
        let origin = node(&mut pf, TileIndex(11 + 10 * 64), Trackdir::TRACKDIR_X_NE);
        pf.nodes.items.push(origin);
        let mut closed = node(&mut pf, destination, Trackdir::TRACKDIR_X_NE);
        closed.estimate = 1000;
        pf.nodes.items.push(closed);
        pf.insert_closed_node(1);
        // A global segment for the destination's key, already costed at 0 and
        // ending at a choice: the new node's cost and estimate are 0.
        let mut found = false;
        // SAFETY: The child process runs this test alone.
        unsafe {
            let cache = &mut (*GLOBAL_CACHES.get())[<CYapfAnyDepotRail as Types>::GLOBAL_CACHE].c;
            let item = cache.get(
                CYapfRailSegmentKey::new(CYapfNodeKeyTrackDir::set(
                    destination,
                    Trackdir::TRACKDIR_X_NE,
                )),
                &mut found,
            );
            cache.heap[item].cost = 0;
            cache.heap[item].last_tile = destination;
            cache.heap[item].last_td = Trackdir::TRACKDIR_X_NE;
            cache.heap[item]
                .end_segment_reason
                .set(EndSegmentReason::CHOICE_FOLLOWS);
        }
        let n = pf.nodes.create_new_node();
        let parent = pf.nodes.items[0];
        pf.nodes.items[n] = CYapfRailNode::set(
            Some((0, &parent)),
            destination,
            Trackdir::TRACKDIR_X_NE,
            false,
        );
        let mut follower = CFollowTrackT::new(
            s,
            TransportType::TRANSPORT_RAIL,
            None,
            crate::company_type::CompanyID(0),
            RailTypes::EMPTY,
            true,
        );
        follower.new_tile = destination;
        follower.new_td_bits = TrackdirBits::TRACKDIR_BIT_X_NE;
        pf.add_new_node(n, &follower);
    }
}
