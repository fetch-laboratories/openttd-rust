/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native evidence bridges, linked by `openttd_test` and the fixture-only game
//! adapters. Each export hands a native fixture one production Rust function.
//!
//! # Safety
//! Fixtures run serialized. Every pointer, record and table an export receives
//! is live for the call; a `*mut` or `&mut` argument is borrowed exclusively
//! for the call, and a `*const` or `&` table is not mutated during it. An
//! export states its own `# Safety` only where it adds a condition.
#![allow(unsafe_code)]
use crate::clear_map::{
    ClearGround, get_clear_counter, get_clear_density, get_clear_ground, get_fence, get_field_type,
    get_industry_index_of_field, is_clear_ground, is_snow_tile, require_field,
};
use crate::company_type::CompanyID;
use crate::direction_type::DiagDirection;
use crate::follow_track::{CFollowTrackT as State, get_speed_limit};
use crate::genworld::Callback;
use crate::heightmap::Input as HeightmapInput;
use crate::landscape_generation::{Frame as GenerationFrame, GenerationServices, Settings};
use crate::map_func::{MapStatics, TileTypeAccess, World};
use crate::movement::native_test_support::{MapLeafOracle, map_read_probe};
use crate::pbs::Pbs;
use crate::rail_type::RailTypes;
use crate::road_type::RoadBits;
use crate::services::Services;
use crate::station::{BaseStation, get_platform_length, get_platform_length_in_direction};
use crate::tile_map::{get_tile_z, get_tropic_zone};
use crate::tile_type::{TileIndex, TileType};
use crate::track_type::Trackdir;
use crate::transport_type::TransportType;
use crate::tree_map::{
    get_tree_count, get_tree_density, get_tree_ground, get_tree_growth, get_tree_type,
};
use crate::vehicle::{OpenTTDVehicle, VehicleRef};
use crate::witness::{
    RAIL_CACHE_FLUSHES, RAIL_LIMIT_STOPS, RAIL_SIGNALS_RED, RAIL_SIGNALS_RESTORED,
    openttd_rust_witness_count as count, openttd_rust_witness_enable,
};

/// `Edge::LastUpdate`, compared with the original graph trace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_last_update(
    edge: *const crate::linkgraph::live::Edge,
) -> i32 {
    unsafe { &*edge }.last_update()
}

/// `Edge::Update` on the economy date, with the original trace's inputs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_edge_update(
    edge: *mut crate::linkgraph::live::Edge,
    capacity: u32,
    usage: u32,
    time: u32,
    modes: u8,
) {
    let date = crate::timer_game_common::economy_date().0;
    unsafe { &mut *edge }.update(
        capacity,
        usage,
        time,
        crate::linkgraph_type::EdgeUpdateModes(modes),
        date,
    );
}
/// `Edge::Restrict`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_edge_restrict(
    edge: *mut crate::linkgraph::live::Edge,
) {
    unsafe { &mut *edge }.restrict();
}
/// `Edge::Release`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_edge_release(
    edge: *mut crate::linkgraph::live::Edge,
) {
    unsafe { &mut *edge }.release();
}

/// Compare production industry removal arithmetic with native `Money` operators.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_industry_removal_cost(multiplier: u32) -> i64 {
    crate::industry::removal_cost(multiplier)
}

/// `GetTileType` through the shared map descriptor, with `callback` as the
/// accessor it calls.
/// # Safety
/// For a nonzero map count, `TileTypeAccess::raw`'s contract holds for the
/// call. `callback` is the original noexcept accessor or a fixture equivalent,
/// and keeps the native out-of-range assertion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_shared_tile_type(
    statics: *const MapStatics,
    callback: extern "C" fn(u32) -> u8,
    tile: u32,
) -> u8 {
    unsafe { TileTypeAccess::raw((*statics).load().0, callback) }
        .read(TileIndex(tile))
        .0
}

/// Compare the in-place map reads with the typed services and with the native
/// header accessors that `observe` runs, before and after `mutate`.
/// # Safety
/// The map allocation and dimensions stay unchanged for the call: `mutate`
/// preserves them and cannot reset, load or yield.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_map_read_probe(
    statics: *const MapStatics,
    shared: *const Services,
    tile: u32,
    mutate: extern "C" fn(u32),
    observe: extern "C" fn(u32, *mut MapLeafOracle),
) -> u8 {
    let s = unsafe { &*shared };
    let (allocation, dimensions) = unsafe { (*statics).load() };
    let world = World {
        types: unsafe { TileTypeAccess::raw(allocation, s.tile_type) },
        shared: s,
        dimensions,
    };
    let map = &world.types;
    let x = world.dimensions.size_x();
    let y = world.size_y();
    let index = TileIndex(tile);
    if crate::tile_map::is_tile_flat(map, index, x, y)
        != Some((unsafe { &*(s.station_commands)() }.tile_flat)(tile))
        || get_tile_z(map, index, x, y) != Some((s.tile_z)(tile))
        || get_tropic_zone(map, index) != Some(crate::tile_type::TropicZone((s.tropic_zone)(tile)))
    {
        return u8::from(false);
    }
    match map.read(index) {
        TileType::MP_CLEAR => {
            let mut clear = MapLeafOracle::default();
            observe(tile, &raw mut clear);
            if is_snow_tile(map, index).map(u32::from) != Some(clear.clear_snow)
                || get_clear_ground(map, index) != Some((s.clear_ground)(tile))
                || get_clear_density(map, index).map(u32::from) != Some(clear.clear_density)
                || get_clear_counter(map, index).map(u32::from) != Some(clear.clear_counter)
            {
                return u8::from(false);
            }
            if is_clear_ground(map, index, ClearGround::CLEAR_FIELDS) == Some(true) {
                if [
                    (clear.fence_ne, DiagDirection::DIAGDIR_NE),
                    (clear.fence_se, DiagDirection::DIAGDIR_SE),
                    (clear.fence_sw, DiagDirection::DIAGDIR_SW),
                    (clear.fence_nw, DiagDirection::DIAGDIR_NW),
                ]
                .into_iter()
                .any(|(expected, dir)| {
                    require_field(map, index).map(|fields| {
                        get_fence(&fields, dir, || crate::services_assert::not_reached())
                    }) != Some(expected)
                }) || require_field(map, index).map(|fields| get_field_type(&fields))
                    != Some(clear.field_type)
                    || require_field(map, index)
                        .map(|fields| u32::from(get_industry_index_of_field(&fields).0))
                        != Some(clear.field_industry)
                {
                    return u8::from(false);
                }
            } else if allocation.assertions
                && require_field(map, index)
                    .map(|fields| get_field_type(&fields))
                    .is_some()
            {
                return u8::from(false);
            }
        }
        TileType::MP_TREES => {
            let mut tree = MapLeafOracle::default();
            observe(tile, &raw mut tree);
            if get_tree_ground(map, index).map(|g| u32::from(g.0)) != Some(tree.tree_ground)
                || get_tree_density(map, index).map(u32::from) != Some(tree.tree_density)
                || get_tree_type(map, index).map(|t| u32::from(t.0)) != Some(tree.tree_type)
                || get_tree_count(map, index).map(u32::from) != Some(tree.tree_count)
                || get_tree_growth(map, index).map(|g| u32::from(g.0)) != Some(tree.tree_growth)
            {
                return u8::from(false);
            }
        }
        _ => {}
    }
    let kind = map.read(index);
    u8::from(
        (!allocation.assertions
            || kind == TileType::MP_CLEAR
            || is_snow_tile(map, index).is_none())
            && (!allocation.assertions
                || kind == TileType::MP_TREES
                || get_tree_type(map, index).is_none())
            && (!allocation.assertions
                || kind == TileType::MP_WATER
                || crate::water_map::is_coast(map, tile).is_none())
            && (allocation.assertions || is_snow_tile(map, index).is_some())
            && map_read_probe(&world, index, mutate, observe),
    )
}

/// Follower fields the native evidence adapters print and compare after each call.
#[repr(C)]
pub struct FollowerOutput {
    pub followed: bool,
    pub old_tile: u32,
    pub new_tile: u32,
    pub tiles_skipped: i32,
    pub new_td_bits: u16,
    pub old_td: u8,
    pub exitdir: u8,
    pub is_tunnel: bool,
    pub is_bridge: bool,
    pub is_station: bool,
    pub err: u8,
}
fn follower_output(state: &State, followed: bool) -> FollowerOutput {
    FollowerOutput {
        followed,
        old_tile: state.old_tile.0,
        new_tile: state.new_tile.0,
        tiles_skipped: state.tiles_skipped,
        new_td_bits: state.new_td_bits.0,
        old_td: state.old_td.0,
        exitdir: state.exitdir.0,
        is_tunnel: state.is_tunnel,
        is_bridge: state.is_bridge,
        is_station: state.is_station,
        err: state.err.0,
    }
}
/// A boxed production follower, initialised by `Init(const VehicleType *v,
/// RailTypes)` when `from_vehicle`, else by `Init(Owner, RailTypes)`.
/// # Safety
/// `v` is null or a vehicle that stays live while the follower is used;
/// `openttd_test_follower_delete` frees the result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_new(
    s: *const Services,
    mode: u8,
    v: *mut OpenTTDVehicle,
    owner: u8,
    types: u64,
    allow90: bool,
    from_vehicle: bool,
) -> *mut State {
    let s = unsafe { &*s };
    let mode = TransportType(mode);
    let v = std::ptr::NonNull::new(v).map(VehicleRef);
    let types = RailTypes(types);
    Box::into_raw(Box::new(if from_vehicle {
        State::vehicle(s, mode, v, types, allow90)
    } else {
        State::new(s, mode, v, CompanyID(owner), types, allow90)
    }))
}
/// Frees a follower from `openttd_test_follower_new`.
/// # Safety
/// `state` came from `openttd_test_follower_new` and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_delete(state: *mut State) {
    drop(unsafe { Box::from_raw(state) });
}
/// `CFollowTrackT::Follow` from `tile` and `td`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_follow(
    s: *const Services,
    state: *mut State,
    tile: u32,
    td: u8,
) -> FollowerOutput {
    let state = unsafe { &mut *state };
    let followed = state.follow(unsafe { &*s }, TileIndex(tile), Trackdir(td));
    follower_output(state, followed)
}
/// The production `MaskReservedTracks` step after a successful follow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_mask(
    s: *const Services,
    state: *mut State,
) -> FollowerOutput {
    let state = unsafe { &mut *state };
    let masked = state.mask_reserved_tracks(Pbs::acquire(unsafe { &*s }));
    follower_output(state, masked)
}
/// `CFollowTrackT::GetSpeedLimit` on `tile`, with no minimum speed requested.
/// # Safety
/// `v` is null or a live vehicle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_speed(
    s: *const Services,
    mode: u8,
    v: *mut OpenTTDVehicle,
    tile: u32,
) -> i32 {
    get_speed_limit(
        unsafe { &*s },
        TransportType(mode),
        std::ptr::NonNull::new(v).map(VehicleRef),
        TileIndex(tile),
        None,
    )
}

/// `Station::GetPlatformLength(TileIndex, DiagDirection)` when `directional`,
/// else `Station::GetPlatformLength(TileIndex)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_platform_length(
    s: *const Services,
    station: *mut BaseStation,
    tile: u32,
    dir: u8,
    directional: bool,
) -> u32 {
    let world = World::new(unsafe { &*s });
    let tile = TileIndex(tile);
    if directional {
        get_platform_length_in_direction(&world, tile, DiagDirection(dir))
    } else {
        get_platform_length(&world, station, tile)
    }
}

/// `GenerateWorldSetAbortCallback`, for fixtures that replace the registered cleanup.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_world_set_abort_callback(callback: Callback) {
    crate::genworld::set_abort_callback(callback);
}

/// The registered `GenWorldInfo::abortp`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_world_abort_callback() -> Callback {
    crate::genworld::abort_callback()
}

/// `GenerateLandscape`.
/// # Safety
/// As `landscape_generation::generate_landscape`: native abortion during
/// progress returns false, and no native exception or Rust panic unwinds
/// across the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_generate_landscape_abi(
    a: *const GenerationServices,
    frame: *mut GenerationFrame,
    input: *const HeightmapInput,
    g: Settings,
    mode: u8,
) -> u8 {
    u8::from(unsafe { crate::landscape_generation::generate_landscape(a, frame, input, g, mode) })
}

/// `LoadHeightmap`, which has no production caller beside `GenerateLandscape`.
/// # Safety
/// As `landscape_generation::load_heightmap`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_load_heightmap_abi(
    a: *const GenerationServices,
    input: *const HeightmapInput,
    rotation: u8,
    freeform: bool,
    heightmap_height: u8,
    max_height: u8,
) -> u8 {
    u8::from(unsafe {
        crate::landscape_generation::load_heightmap(
            a,
            input,
            rotation,
            freeform,
            heightmap_height,
            max_height,
        )
    })
}

/// `FlatEmptyWorld`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_flat_empty_world(
    a: *const GenerationServices,
    height: u8,
    freeform: bool,
    max_height: u8,
) {
    unsafe { crate::landscape_generation::flat_empty_world(a, height, freeform, max_height) }
}

/// `LoadTownData` coordinate policy, compared with the original native expression.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_town_data_target(
    services: *const crate::town::import::ImportServices,
    rotation: u8,
    x: f32,
    y: f32,
) -> u32 {
    unsafe { &*services }
        .target(crate::heightmap::HeightmapRotation(rotation), x, y)
        .0
}

/// `IsMapSurroundedByWater`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_map_surrounded_by_water_abi(s: &Services) -> u8 {
    u8::from(crate::landscape::surrounded_by_water(s))
}

/// `HighestSnowLine`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_snow_line_highest(s: &Services) -> u8 {
    crate::snow_line::highest(s)
}

/// `CleanUpRoadBits` over a town's planned road bits.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the roads console's static_cast<RoadBits>(bits)"
)]
pub unsafe extern "C" fn openttd_test_clean_up_road_bits(
    shared: *const Services,
    tile: u32,
    bits: u32,
) -> u32 {
    u32::from(
        crate::road_cleanup::clean_up_road_bits(unsafe { &*shared }, tile, RoadBits(bits as u8)).0,
    )
}

/// `_station_kdtree.Insert(st->index)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_insert(s: *const Services, st: *mut BaseStation) {
    crate::station::spatial::insert(unsafe { &*s }, st);
}
/// `Station::MoveSign`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_move_sign(
    s: *const Services,
    st: *mut BaseStation,
    tile: u32,
) {
    crate::station::spatial::station_move_sign(unsafe { &*s }, st, TileIndex(tile));
}
/// `ForAllStationsRadius`, visiting each station through `visit`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_radius(
    s: *const Services,
    center: u32,
    distance: u32,
    context: *mut std::ffi::c_void,
    visit: extern "C" fn(*mut std::ffi::c_void, *mut BaseStation),
) {
    crate::station::spatial::for_all_stations_radius(
        unsafe { &*s },
        TileIndex(center),
        distance,
        |st| {
            visit(context, st);
        },
    );
}
/// `ModifyStationRatingAround`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_rating_around(
    s: *const Services,
    tile: u32,
    owner: u8,
    amount: i32,
    radius: u32,
) {
    crate::station::spatial::modify_station_rating_around(
        unsafe { &*s },
        TileIndex(tile),
        CompanyID(owner),
        amount,
        radius,
    );
}
/// `ModifyStationRatingAround`'s per-station update.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_modify_rating(
    st: *mut crate::station_service::Service,
    amount: i32,
) {
    unsafe {
        crate::station_service::modify_rating(st, amount);
    }
}
/// Exercise production unload preparation with native fixture callback frames.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_prepare(
    front: VehicleRef,
    world: *const crate::station_service::World,
    loading: *const crate::station_service::Loading,
) {
    crate::station_service::prepare_unload(front, unsafe { &*world }, unsafe { &*loading });
}
/// `Station::GetPrimaryRoadStop(const RoadVehicle *)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_primary_stop(
    s: *const Services,
    st: *mut BaseStation,
    vehicle: VehicleRef,
) -> *mut crate::road_stop::Stop {
    crate::road_stop::primary_for_vehicle(unsafe { &*s }, st, vehicle)
}
/// `RoadStop::GetNextRoadStop`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_next_stop(
    s: *const Services,
    stop: *const crate::road_stop::Stop,
    vehicle: VehicleRef,
) -> *mut crate::road_stop::Stop {
    crate::road_stop::next_for_vehicle(unsafe { &*s }, stop, vehicle)
}
/// `GetRailTileArea`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_rail_area(
    s: *const Services,
    st: *mut BaseStation,
    tile: u32,
    area: u8,
) -> crate::tilearea::TileArea {
    crate::newgrf_station::rail_tile_area(unsafe { &*s }, st, tile, area)
}

/// The station kd-tree's builds, insertions, removals and searches over
/// fixed ids, compared with the original `Kdtree` template.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_tree_sequence(
    s: *const Services,
    context: *mut std::ffi::c_void,
    visit: extern "C" fn(*mut std::ffi::c_void, u32, u16),
) {
    let s = unsafe { &*s };
    let xy = |id: u16, dim: usize| if dim == 0 { id % 3 } else { id % 5 };
    let split = |keys: &mut [crate::kdtree::SplitKey<u16>]| crate::kdtree::native_split(s, keys);
    let mut tree = crate::kdtree::Kdtree::new();
    let observe = |tree: &crate::kdtree::Kdtree<u16>, phase| {
        tree.contained([0, 0], [10, 10], &xy, |id| visit(context, phase, id));
        tree.contained([1, 1], [2, 4], &xy, |id| visit(context, phase + 100, id));
        if tree.count() != 0 {
            visit(context, phase + 200, tree.nearest([1, 2], &xy));
        }
    };
    tree.build(0..20, &xy, &split);
    observe(&tree, 0);
    let mut first = Vec::new();
    tree.contained([0, 0], [10, 10], &xy, |id| first.push(id));
    for (phase, id) in [(1, first[0]), (2, first[1])] {
        tree.remove(id, &xy, &split);
        observe(&tree, phase);
    }
    for id in 20..40 {
        tree.insert(id, &xy, &split);
        observe(&tree, u32::from(id));
    }
    for (phase, id) in [(40, 20), (41, 30), (42, 39)] {
        tree.remove(id, &xy, &split);
        observe(&tree, phase);
    }
    tree.build([], &xy, &split);
    observe(&tree, 43);
    tree.insert(12, &xy, &split);
    observe(&tree, 44);
}

// Mixed widths force padding; the recursive pointer checks the generated native type.
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDAbiRecordFixture" ["record_test"] => pub struct AbiRecordFixture {
        pub tag: u8,
        pub value: crate::tile_type::TileIndex,
        pub count: usize,
        pub data: *mut u8,
        pub next: *const AbiRecordFixture,
    }
}

/// Nested `CommandCost` results on the stack stay distinct, and each is
/// destroyed on return.
/// # Safety
/// The supplied table constructs and destroys native objects in the advertised storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_command_stack_probe(
    s: &crate::services::CommandCostServices,
) -> u8 {
    u8::from(unsafe {
        /* SAFETY: synchronous costs never escape or outlive this call. */
        crate::command::with_cost(s, |outer| {
            crate::command::with_cost(s, |inner| {
                outer.zero();
                inner.zero();
                outer.0 != inner.0 && outer.ok() && inner.ok()
            })
        })
    })
}

/// Observe GRF text bytes and language state, which saves do not contain.
/// # Safety
/// The visitor is synchronous and may read, but cannot mutate, the text table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_grf_text_dump(
    context: *const std::ffi::c_void,
    visitor: unsafe extern "C" fn(
        *const std::ffi::c_void,
        u32,
        u32,
        u8,
        u32,
        u32,
        crate::services::ByteSpan,
    ),
) -> u8 {
    unsafe { crate::newgrf_text::visit(context, visitor) }
}

/// Exercise the aircraft controller over native fixture handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_aircraft_event(
    leaves: *const crate::aircraft::Leaves,
    id: crate::aircraft::Vehicle,
    pass: bool,
) -> u8 {
    u8::from(unsafe { crate::aircraft::event(leaves, id, pass) })
}

/// Exercise native record returns and enum arguments.
/// # Safety
/// The table returns only ABI marker values, never dereferenced handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ground_abi_calls(
    calls: *const crate::ground::ABICalls,
) -> u8 {
    unsafe { crate::ground::abi_calls(calls) }
}

/// Exercise production object generation through native fixture storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_object_generate(
    s: *const crate::objects::ObjectServices,
    out: *mut crate::services::CommandCost,
    tmp: *mut crate::services::CommandCost,
    aux: *mut crate::services::CommandCost,
    frame: *mut crate::objects::GenerationFrame,
) -> u8 {
    u8::from(unsafe { crate::objects::generate(s, out, tmp, aux, frame) })
}

/// Exercise production tree generation with native fixture callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_trees_generate(
    services: *const crate::trees::TreeServices,
    shared: *const Services,
) -> u8 {
    u8::from(unsafe { crate::trees::generate(services, shared) })
}

/// `_local_wait_queue` for the commands console, which inspects it in place.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_network_wait_queue() -> *const crate::network_command::CommandQueue {
    crate::network_command::wait_queue()
}

/// `_local_wait_queue.clear()` through the queue's own clear.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_network_wait_queue_clear() {
    crate::network_command::clear_wait_queue();
}

/// A deep copy of an engine's bindings, which no pool owns.
/// # Safety
/// The fixture owns the result, and `openttd_test_engine_bindings_drop` frees it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_bindings_clone(
    source: &crate::engine::bindings::Bindings,
) -> *mut crate::engine::bindings::Bindings {
    Box::into_raw(Box::new(source.clone()))
}
/// Frees a copy from `openttd_test_engine_bindings_clone`.
/// # Safety
/// `snapshot` is freed once, after every view of it has ended.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_bindings_drop(
    snapshot: *mut crate::engine::bindings::Bindings,
) {
    drop(unsafe { Box::from_raw(snapshot) });
}
/// Empties an engine's cargo sprite groups.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_sprites_clear(
    destination: &mut crate::industrytype::NativeVec<crate::engine::bindings::CargoBinding>,
) {
    *destination = crate::industrytype::NativeVec::new();
}
/// Empties an engine's wagon overrides.
/// # Safety
/// No view of the overrides' engine lists outlives the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_overrides_clear(
    destination: &mut crate::industrytype::NativeVec<crate::engine::bindings::WagonOverride>,
) {
    *destination = crate::industrytype::NativeVec::new();
}
/// Replaces an engine's badges with `count` ids from `items`.
/// # Safety
/// `items` is readable for `count` ids and may alias `destination`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_badges_assign(
    destination: &mut crate::industrytype::NativeVec<u32>,
    items: *const u32,
    count: usize,
) {
    let items = if count == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(items, count) }.to_vec()
    };
    *destination = crate::industrytype::NativeVec::from_vec(items);
}
/// `GRFFilePropsBase::SetSpriteGroup` for `cargo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_sprite(
    props: *mut crate::engine::bindings::GrfProps,
    cargo: u8,
    group: *const std::ffi::c_void,
) {
    unsafe {
        crate::engine::bindings::set_sprite_group(props, cargo, group);
    }
}
/// Appends a wagon override, as `SetWagonOverrideSprites`.
/// # Safety
/// `ids` is readable for `count` ids.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_override(
    shared: &Services,
    bindings: *mut crate::engine::bindings::Bindings,
    cargo: u8,
    group: *const std::ffi::c_void,
    ids: *const crate::engine_type::EngineID,
    count: usize,
) {
    let ids = if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ids, count) }
    };
    unsafe {
        crate::engine::bindings::append_override(shared, bindings, cargo, group, ids);
    }
}

/// A ship search's input and the ship facts the controller supplies, with the
/// search's `path_found` and `best_origin_dir`, for the native ship tests.
#[repr(C)]
pub struct ShipYapfProbe {
    pub input: crate::ship_yapf::Input,
    pub state: u8,
    pub trackdir: u8,
    pub path_found: u8,
    pub best_origin_dir: u8,
}
impl ShipYapfProbe {
    fn vehicle<'a>(
        &self,
        s: &'a Services,
        leaves: &'a crate::ship_control::Leaves,
    ) -> crate::ship_yapf::Vehicle<'a> {
        crate::ship_yapf::Vehicle::new(
            s,
            self.input,
            crate::track_type::TrackBits(self.state),
            Trackdir(self.trackdir),
            leaves,
        )
    }
}

/// Run the production `ChooseShipTrack` over the probe's input.
/// # Safety
/// The probe and path cache are exclusive for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_choose_ship_track(
    shared: *const Services,
    leaves: *const crate::ship_control::Leaves,
    probe: *mut ShipYapfProbe,
    tile: u32,
    forward_dirs: u16,
    reverse_dirs: u16,
    path_cache: *mut crate::ship_yapf::ShipPathCache,
) -> u8 {
    let (s, leaves, probe, path_cache) =
        unsafe { (&*shared, &*leaves, &mut *probe, &mut *path_cache) };
    let v = probe.vehicle(s, leaves);
    let mut path_found = false;
    let mut best_origin_dir = Trackdir::INVALID_TRACKDIR;
    let td = crate::ship_yapf::choose_ship_track(
        &v,
        TileIndex(tile),
        crate::track_type::TrackdirBits(forward_dirs),
        crate::track_type::TrackdirBits(reverse_dirs),
        &mut path_found,
        path_cache,
        &mut best_origin_dir,
    );
    probe.path_found = u8::from(path_found);
    probe.best_origin_dir = best_origin_dir.0;
    td.0
}

/// Run the production `CheckShipReverse` over the probe's input; `trackdir`
/// may be null, as the original's.
/// # Safety
/// `trackdir` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_check_ship_reverse(
    shared: *const Services,
    leaves: *const crate::ship_control::Leaves,
    probe: *const ShipYapfProbe,
    trackdir: *mut u8,
) -> u8 {
    let (s, leaves, probe) = unsafe { (&*shared, &*leaves, &*probe) };
    let v = probe.vehicle(s, leaves);
    let mut out = Trackdir::INVALID_TRACKDIR;
    let reverse =
        crate::ship_yapf::check_ship_reverse(&v, (!trackdir.is_null()).then_some(&mut out));
    if !trackdir.is_null() {
        unsafe { trackdir.write(out.0) };
    }
    u8::from(reverse)
}

/// Run the production `FindWaterRegionPath` over the probe's input, writing at
/// most `capacity` patches of the path to `out`; returns the path's length.
/// # Safety
/// `out` holds `capacity` writable patches.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_find_water_region_path(
    shared: *const Services,
    leaves: *const crate::ship_control::Leaves,
    probe: *const ShipYapfProbe,
    start_tile: u32,
    max_returned_path_length: i32,
    out: *mut crate::water_regions::WaterRegionPatchDesc,
    capacity: usize,
) -> usize {
    let (s, leaves, probe) = unsafe { (&*shared, &*leaves, &*probe) };
    let v = probe.vehicle(s, leaves);
    let path = crate::ship_yapf::find_water_region_path(
        &v,
        TileIndex(start_tile),
        max_returned_path_length,
    );
    for (index, patch) in path.iter().take(capacity).enumerate() {
        unsafe { out.add(index).write(*patch) };
    }
    path.len()
}

/// The ship node list's open-node heap (`CBinaryHeapT` over `CYapfShipNode`
/// estimates) for tie and removal order, which no saved state records.
/// Commands use bits 31..30: include, remove, update; low bits are node indices.
/// The native test runs the same commands through the original `CBinaryHeapT`.
/// # Safety
/// `costs[nodes]`, `commands[count]` and `values[count]` are readable, `out[count]`
/// writable; node indices and heap membership obey `Include`/`Remove` preconditions.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the native test's static_cast<uint32_t>(heap.Begin() - nodes.data())"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_heap_probe(
    costs: *const i32,
    nodes: usize,
    commands: *const u32,
    values: *const i32,
    count: usize,
    out: *mut u32,
) {
    let mut items: Vec<crate::yapf_node::Node<u32>> = (0..nodes)
        .map(|index| {
            let mut node = crate::yapf_node::Node::set(None, 0);
            node.estimate = unsafe { costs.add(index).read() };
            node
        })
        .collect();
    let mut heap = crate::binaryheap::BinaryHeap::new(2048);
    for step in 0..count {
        let command = unsafe { commands.add(step).read() };
        let index = (command & 0x3fff_ffff) as usize;
        match command >> 30 {
            0 => heap.include(index, |a, b| items[a].less(&items[b])),
            1 => heap.remove(index, |a, b| items[a].less(&items[b])),
            _ => {
                heap.remove(index, |a, b| items[a].less(&items[b]));
                items[index].estimate = unsafe { values.add(step).read() };
                heap.include(index, |a, b| items[a].less(&items[b]));
            }
        }
        unsafe {
            out.add(step)
                .write(heap.first().map_or(u32::MAX, |i| i as u32));
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDShipYapfProbe" ["probe"] => ShipYapfProbe {
        input,
        state,
        trackdir,
        path_found,
        best_origin_dir,
    };
}

/// One rail search's result, statistics and the witness counts it added.
#[repr(C)]
pub struct RailYapfSearch {
    pub num_steps: i32,
    pub open: i32,
    pub closed: i32,
    pub cache_hits: i32,
    pub cost_calcs: i32,
    pub cost: i32,
    pub distance: i32,
    pub flushes: u32,
    pub limits: u32,
    pub red: u32,
    pub restored: u32,
    pub found: u8,
    pub td: u8,
}
/// The production rail search over the native fixture: `ChooseRailTrack`, or
/// `FindNearestSafeTile` from `tile` and `td` with `safe_tile`. Witnesses turn
/// on, and the counts are the differences around the search.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_rail_yapf(
    s: *const Services,
    v: *mut OpenTTDVehicle,
    safe_tile: bool,
    tile: u32,
    td: u8,
    reserve_track: bool,
) -> RailYapfSearch {
    openttd_rust_witness_enable();
    let before = [
        RAIL_CACHE_FLUSHES,
        RAIL_LIMIT_STOPS,
        RAIL_SIGNALS_RED,
        RAIL_SIGNALS_RESTORED,
    ]
    .map(|i| count(i));
    let result = crate::rail_yapf::native_test_search(
        unsafe { &*s },
        crate::train::Handle::from_record(v),
        safe_tile,
        TileIndex(tile),
        Trackdir(td),
        reserve_track,
    );
    let after = [
        RAIL_CACHE_FLUSHES,
        RAIL_LIMIT_STOPS,
        RAIL_SIGNALS_RED,
        RAIL_SIGNALS_RESTORED,
    ]
    .map(|i| count(i));
    let delta = |i: usize| u32::try_from(after[i] - before[i]).unwrap_or(u32::MAX);
    RailYapfSearch {
        num_steps: result.num_steps,
        open: result.open,
        closed: result.closed,
        cache_hits: result.cache_hits,
        cost_calcs: result.cost_calcs,
        cost: result.cost,
        distance: result.distance,
        flushes: delta(0),
        limits: delta(1),
        red: delta(2),
        restored: delta(3),
        found: u8::from(result.found),
        td: result.td.0,
    }
}
