/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native evidence bridges; linked by `openttd_test` and fixture-only game adapters.
#![allow(unsafe_code, clippy::too_many_lines)]
use crate::map_func::{MapStatics, TileTypeAccess};
use crate::tile_map::{get_tile_z, get_tropic_zone};
use crate::tile_type::{TileIndex, TileType};

/// Observe the production edge timestamp calculation in the original graph trace.
/// # Safety
/// `edge` is a live canonical edge, read under serialized fixture execution.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_last_update(
    edge: *const crate::linkgraph::live::Edge,
) -> i32 {
    unsafe { &*edge }.last_update()
}

/// Exercise the production edge update with the original native trace inputs.
/// # Safety
/// `edge` is a live canonical edge borrowed exclusively for this fixture call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_edge_update(
    edge: *mut crate::linkgraph::live::Edge,
    capacity: u32,
    usage: u32,
    time: u32,
    modes: crate::linkgraph_type::EdgeUpdateModes,
) {
    let date = crate::timer_game_common::economy_date().0;
    unsafe { &mut *edge }.update(capacity, usage, time, modes, date);
}
/// # Safety
/// `edge` is a live canonical edge borrowed exclusively for this fixture call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_linkgraph_edge_restrict(
    edge: *mut crate::linkgraph::live::Edge,
) {
    unsafe { &mut *edge }.restrict();
}
/// # Safety
/// `edge` is a live canonical edge borrowed exclusively for this fixture call.
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

/// # Safety
/// For nonzero counts, uphold `TileTypeAccess::raw`'s validity/serialization
/// contract for this call. The callback is the original noexcept accessor or an
/// equivalent live fixture; it keeps the native out-of-range assertion policy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_shared_tile_type(
    statics: *const MapStatics,
    callback: extern "C" fn(u32) -> u8,
    tile: u32,
) -> u8 {
    unsafe { TileTypeAccess::raw((*statics).load().0, callback) }
        .read(TileIndex(tile))
        .0
}

/// Native default probe: compare audited reads with the unchanged typed services.
/// # Safety
/// Same allocation contract as the scheduler; all tables are live and immutable.
/// Mutation preserves the allocation/dimensions and cannot reset/load/yield.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub unsafe extern "C" fn openttd_rust_map_read_probe(
    statics: *const MapStatics,
    shared: *const crate::services::Services,
    tile: u32,
    mutate: extern "C" fn(u32),
    observe: extern "C" fn(u32, *mut crate::movement::native_test_support::MapLeafOracle),
) -> u8 {
    let s = unsafe { &*shared };
    let allocation = unsafe { (*statics).load().0 };
    let map = unsafe { TileTypeAccess::raw(allocation, s.tile_type) };
    let x = (s.map_max_x)() + 1;
    let y = (s.map_max_y)() + 1;
    let index = crate::tile_type::TileIndex(tile);
    if crate::tile_map::is_tile_flat(&map, index, x, y)
        != Some((unsafe { &*(s.station_commands)() }.tile_flat)(tile))
        || get_tile_z(&map, index, x, y) != Some((s.tile_z)(tile))
        || get_tropic_zone(&map, index) != Some(crate::tile_type::TropicZone((s.tropic_zone)(tile)))
    {
        return u8::from(false);
    }
    match map.read(TileIndex(tile)) {
        TileType::MP_CLEAR => {
            use crate::clear_map::{
                ClearGround, get_clear_counter, get_clear_density, get_clear_ground, get_fence,
                get_field_type, get_industry_index_of_field, is_snow_tile, require_field,
            };
            use crate::direction_type::DiagDirection;
            let mut clear = crate::movement::native_test_support::MapLeafOracle::default();
            observe(tile, &raw mut clear);
            if is_snow_tile(&map, index).map(u32::from) != Some(clear.clear_snow)
                || get_clear_ground(&map, index) != Some((s.clear_ground)(tile))
                || get_clear_density(&map, index).map(u32::from) != Some(clear.clear_density)
                || get_clear_counter(&map, index).map(u32::from) != Some(clear.clear_counter)
            {
                return u8::from(false);
            }
            if get_clear_ground(&map, index) == Some(ClearGround::CLEAR_FIELDS) {
                if [
                    (clear.fence_ne, DiagDirection::DIAGDIR_NE),
                    (clear.fence_se, DiagDirection::DIAGDIR_SE),
                    (clear.fence_sw, DiagDirection::DIAGDIR_SW),
                    (clear.fence_nw, DiagDirection::DIAGDIR_NW),
                ]
                .into_iter()
                .any(|(expected, dir)| {
                    require_field(&map, index).map(|fields| {
                        get_fence(&fields, dir, || crate::services_assert::not_reached())
                    }) != Some(expected)
                }) || require_field(&map, index).map(|fields| get_field_type(&fields))
                    != Some(clear.field_type)
                    || require_field(&map, index)
                        .map(|fields| u32::from(get_industry_index_of_field(&fields).0))
                        != Some(clear.field_industry)
                {
                    return u8::from(false);
                }
            } else if allocation.assertions
                && require_field(&map, index)
                    .map(|fields| get_field_type(&fields))
                    .is_some()
            {
                return u8::from(false);
            }
        }
        TileType::MP_TREES => {
            use crate::tree_map::{
                get_tree_count, get_tree_density, get_tree_ground, get_tree_growth, get_tree_type,
            };
            let mut tree = crate::movement::native_test_support::MapLeafOracle::default();
            observe(tile, &raw mut tree);
            if get_tree_ground(&map, index).map(|g| u32::from(g.0)) != Some(tree.tree_ground)
                || get_tree_density(&map, index).map(u32::from) != Some(tree.tree_density)
                || get_tree_type(&map, index).map(|t| u32::from(t.0)) != Some(tree.tree_type)
                || get_tree_count(&map, index).map(u32::from) != Some(tree.tree_count)
                || get_tree_growth(&map, index).map(|g| u32::from(g.0)) != Some(tree.tree_growth)
            {
                return u8::from(false);
            }
        }
        _ => {}
    }
    let kind = map.read(TileIndex(tile));
    u8::from(
        (!allocation.assertions
            || kind == TileType::MP_CLEAR
            || crate::clear_map::is_snow_tile_raw(&map, tile).is_none())
            && (!allocation.assertions
                || kind == TileType::MP_TREES
                || crate::tree_map::get_tree_type(&map, index).is_none())
            && (!allocation.assertions
                || kind == TileType::MP_WATER
                || crate::water_map::is_coast(&map, tile).is_none())
            && (allocation.assertions || crate::clear_map::is_snow_tile_raw(&map, tile).is_some())
            && crate::movement::native_test_support::map_read_probe(
                allocation, s, tile, mutate, observe,
            ),
    )
}

use crate::follow_track::{CFollowTrackT as State, get_speed_limit_raw};
use crate::pbs::Pbs;
use crate::services::Services;
use crate::vehicle::{OpenTTDVehicle, VehicleRef};

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
/// A boxed production follower; `openttd_test_follower_delete` frees it.
/// # Safety
/// Services and optional native vehicle remain live and serialized.
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
    let v = std::ptr::NonNull::new(v).map(VehicleRef);
    Box::into_raw(Box::new(if from_vehicle {
        State::vehicle_raw(s, mode, v, types, allow90)
    } else {
        State::new_raw(s, mode, v, owner, types, allow90)
    }))
}
/// # Safety
/// `state` came from `openttd_test_follower_new` and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_delete(state: *mut State) {
    drop(unsafe { Box::from_raw(state) });
}
/// # Safety
/// The follower and shared game world remain live and serialized throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_follow(
    s: *const Services,
    state: *mut State,
    tile: u32,
    td: u8,
) -> FollowerOutput {
    let state = unsafe { &mut *state };
    let followed = state.follow_raw(unsafe { &*s }, tile, td);
    follower_output(state, followed)
}
/// The production `MaskReservedTracks` step after a successful follow.
/// # Safety
/// Same follower and serialized world contract as Follow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_mask(
    s: *const Services,
    state: *mut State,
) -> FollowerOutput {
    let state = unsafe { &mut *state };
    let masked = state.mask_reserved_tracks(Pbs::acquire(unsafe { &*s }));
    follower_output(state, masked)
}
/// # Safety
/// Shared services and optional vehicle stay live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_speed(
    s: *const Services,
    mode: u8,
    v: *mut OpenTTDVehicle,
    tile: u32,
) -> i32 {
    get_speed_limit_raw(
        unsafe { &*s },
        mode,
        std::ptr::NonNull::new(v).map(VehicleRef),
        tile,
    )
}

/// # Safety
/// The station and tile satisfy the original overload's preconditions in the serialized world.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_platform_length(
    s: *const Services,
    station: *mut crate::station_type::BaseStation,
    tile: u32,
    dir: u8,
    directional: bool,
) -> u32 {
    let world = crate::map_func::World::new(unsafe { &*s });
    if directional {
        crate::station::get_platform_length_in_direction_raw(&world, tile, dir)
    } else {
        crate::station::get_platform_length_raw(&world, station, tile)
    }
}

use crate::genworld::Callback;
use crate::heightmap::Input as HeightmapInput;
use crate::landscape_generation::{Frame as GenerationFrame, GenerationServices, Settings};

/// `GenerateWorldSetAbortCallback`, for fixtures that replace the registered cleanup.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_world_set_abort_callback(callback: Callback) {
    crate::genworld::set_abort_callback(callback);
}

/// The registered `GenWorldInfo::abortp`, which Rust TGP sets without a native call.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_world_abort_callback() -> Callback {
    crate::genworld::abort_callback()
}

/// `GenerateLandscape`.
/// # Safety
/// Same contract as the production generation entry.
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
/// Same contract as the production generation entry.
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
/// # Safety
/// Same native map/service contract as the production generation entry.
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
/// # Safety
/// Live shared services and map for the synchronous call.
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

#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_map_surrounded_by_water_abi(s: &crate::services::Services) -> u8 {
    u8::from(crate::landscape::surrounded_by_water(s))
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_snow_line_highest(s: &crate::services::Services) -> u8 {
    crate::snow_line::highest(s)
}

/// Planned-road evidence enters the production Rust owner without a native facade.
/// # Safety
/// Shared services and canonical map statics are live for this serialized entry.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)] // Native uint32 town plans narrow to RoadBits.
pub unsafe extern "C" fn openttd_test_clean_up_road_bits(
    shared: *const crate::services::Services,
    tile: u32,
    bits: u32,
) -> u32 {
    u32::from(
        crate::road_cleanup::clean_up_road_bits(
            unsafe { &*shared },
            tile,
            crate::services::RoadBits(bits as u8),
        )
        .0,
    )
}

/// # Safety
/// Every station, road vehicle and service table is live and serialized throughout these probes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_insert(
    s: *const Services,
    st: *mut crate::station::BaseStation,
) {
    crate::station::spatial::insert(unsafe { &*s }, st);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_move_sign(
    s: *const Services,
    st: *mut crate::station::BaseStation,
    tile: u32,
) {
    crate::station::spatial::station_move_sign(
        unsafe { &*s },
        st,
        crate::tile_type::TileIndex(tile),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_radius(
    s: *const Services,
    center: u32,
    distance: u32,
    context: *mut std::ffi::c_void,
    visit: extern "C" fn(*mut std::ffi::c_void, *mut crate::station::BaseStation),
) {
    crate::station::spatial::for_all_stations_radius(
        unsafe { &*s },
        crate::tile_type::TileIndex(center),
        distance,
        |st| {
            visit(context, st);
        },
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_rating_around(
    s: *const Services,
    tile: u32,
    owner: u8,
    amount: i32,
    radius: u32,
) {
    crate::station::spatial::modify_rating_around(unsafe { &*s }, tile, owner, amount, radius);
}
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
/// # Safety
/// The live record and immutable tables meet `prepare_unload`'s entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_prepare(
    front: VehicleRef,
    world: *const crate::station_service::World,
    loading: *const crate::station_service::Loading,
) {
    crate::station_service::prepare_unload(front, unsafe { &*world }, unsafe { &*loading });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_primary_stop(
    s: *const Services,
    st: *mut crate::station::BaseStation,
    vehicle: VehicleRef,
) -> *mut crate::road_stop::Stop {
    crate::road_stop::primary_for_vehicle(unsafe { &*s }, st, vehicle)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_next_stop(
    s: *const Services,
    stop: *const crate::road_stop::Stop,
    vehicle: VehicleRef,
) -> *mut crate::road_stop::Stop {
    crate::road_stop::next_for_vehicle(unsafe { &*s }, stop, vehicle)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_rail_area(
    s: *const Services,
    st: *mut crate::station::BaseStation,
    tile: u32,
    area: u8,
) -> crate::tilearea::TileArea {
    crate::newgrf_station::rail_tile_area(unsafe { &*s }, st, tile, area)
}

/// Fixed build/mutation observations against the original native template.
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

/// Native default test of distinct nested stack results and early return cleanup.
/// # Safety
/// The supplied table constructs/destroys native objects in the advertised storage.
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
/// # Safety
/// Handles and immutable services satisfy the aircraft module's synchronous contract.
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
/// The immutable table returns only ABI marker values, never dereferenced handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ground_abi_calls(
    calls: *const crate::ground::ABICalls,
) -> u8 {
    unsafe { crate::ground::abi_calls(calls) }
}

/// Exercise production object generation through native fixture storage.
/// # Safety
/// The services, cost objects and frame satisfy the object module's entry contract.
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
/// # Safety
/// Immutable tables and settings satisfy the tree module's serialized entry contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_trees_generate(
    services: *const crate::trees::TreeServices,
    shared: *const crate::services::Services,
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

/// Deep snapshot for the existing native engine fixtures; it has no shell/pool owner.
/// # Safety
/// Source is a live serialized binding record; the result is uniquely owned by the fixture.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_bindings_clone(
    source: &crate::engine::bindings::Bindings,
) -> *mut crate::engine::bindings::Bindings {
    Box::into_raw(Box::new(source.clone()))
}
/// # Safety
/// Retire exactly one fixture snapshot, after all borrowed views end.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_bindings_drop(
    snapshot: *mut crate::engine::bindings::Bindings,
) {
    drop(unsafe { Box::from_raw(snapshot) });
}
/// # Safety
/// Destination is a live serialized Rust-owned vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_sprites_clear(
    destination: &mut crate::industrytype::NativeVec<crate::engine::bindings::CargoBinding>,
) {
    *destination = crate::industrytype::NativeVec::new();
}
/// # Safety
/// Destination is a live serialized Rust-owned vector, with no retained nested views.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_overrides_clear(
    destination: &mut crate::industrytype::NativeVec<crate::engine::bindings::WagonOverride>,
) {
    *destination = crate::industrytype::NativeVec::new();
}
/// # Safety
/// Source IDs are readable for count; destination is live/serialized and may alias source.
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
/// # Safety
/// Live serialized props and borrowed group share the fixture lifetime.
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
/// # Safety
/// Bindings and shared table are live/serialized; IDs are readable for count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_engine_override(
    shared: &crate::services::Services,
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
        s: &'a crate::services::Services,
        leaves: &'a crate::ship_control::Leaves,
    ) -> crate::ship_yapf::Vehicle<'a> {
        crate::ship_yapf::Vehicle::new(
            s,
            self.input,
            crate::track_type::TrackBits(self.state),
            crate::track_type::Trackdir(self.trackdir),
            leaves,
        )
    }
}

/// Run the production `ChooseShipTrack` over the probe's input.
/// # Safety
/// The shared table, leaves, probe and path cache are live and exclusive for the
/// call, under the ship search's module contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_choose_ship_track(
    shared: *const crate::services::Services,
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
    let mut best_origin_dir = crate::track_type::Trackdir::INVALID_TRACKDIR;
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
/// As [`openttd_test_ship_choose_ship_track`]; `trackdir` is null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_check_ship_reverse(
    shared: *const crate::services::Services,
    leaves: *const crate::ship_control::Leaves,
    probe: *const ShipYapfProbe,
    trackdir: *mut u8,
) -> u8 {
    let (s, leaves, probe) = unsafe { (&*shared, &*leaves, &*probe) };
    let v = probe.vehicle(s, leaves);
    let mut out = crate::track_type::Trackdir::INVALID_TRACKDIR;
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
/// As [`openttd_test_ship_choose_ship_track`]; `out` holds `capacity` writable patches.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_ship_find_water_region_path(
    shared: *const crate::services::Services,
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
/// # Safety
/// Shared services and the front train record stay live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_rail_yapf(
    s: *const Services,
    v: *mut OpenTTDVehicle,
    safe_tile: bool,
    tile: u32,
    td: u8,
    reserve_track: bool,
) -> RailYapfSearch {
    use crate::witness::{
        RAIL_CACHE_FLUSHES, RAIL_LIMIT_STOPS, RAIL_SIGNALS_RED, RAIL_SIGNALS_RESTORED,
        openttd_rust_witness_count as count,
    };
    crate::witness::openttd_rust_witness_enable();
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
        crate::tile_type::TileIndex(tile),
        crate::track_type::Trackdir(td),
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
