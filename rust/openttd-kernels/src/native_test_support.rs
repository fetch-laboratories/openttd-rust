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
    clear: *const crate::ground::ClearServices,
    trees: *const crate::trees::TreeServices,
    tile: u32,
    mutate: extern "C" fn(u32),
    observe: extern "C" fn(u32, *mut crate::movement::native_test_support::MapLeafOracle),
) -> u8 {
    let (s, c, trees) = unsafe { (&*shared, &*clear, &*trees) };
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
                get_clear_counter_raw, get_clear_density_raw, get_clear_ground_raw, get_fence_raw,
                get_field_type, get_industry_index_of_field_raw, is_snow_tile_raw,
                require_field_raw,
            };
            if is_snow_tile_raw(&map, tile) != Some((c.snow)(tile))
                || get_clear_ground_raw(&map, tile) != Some((s.clear_ground)(tile).0)
                || get_clear_density_raw(&map, tile).map(u32::from) != Some((c.density)(tile))
                || get_clear_counter_raw(&map, tile).map(u32::from) != Some((c.counter)(tile))
            {
                return u8::from(false);
            }
            if get_clear_ground_raw(&map, tile) == Some(crate::clear_map::CLEAR_FIELDS) {
                let fences = (c.fences)(tile);
                let field = (c.field)(tile);
                if [fences.ne, fences.se, fences.sw, fences.nw]
                    .into_iter()
                    .zip(0_u8..)
                    .any(|(expected, dir)| {
                        require_field_raw(&map, tile).map(|fields| get_fence_raw(&fields, dir))
                            != Some(expected)
                    })
                    || require_field_raw(&map, tile).map(|fields| get_field_type(&fields))
                        != Some(field.field_type)
                    || require_field_raw(&map, tile)
                        .map(|fields| get_industry_index_of_field_raw(&fields))
                        != Some(field.industry)
                {
                    return u8::from(false);
                }
            } else if allocation.assertions
                && require_field_raw(&map, tile)
                    .map(|fields| get_field_type(&fields))
                    .is_some()
            {
                return u8::from(false);
            }
        }
        TileType::MP_TREES => {
            use crate::tree_map::{
                get_tree_count_raw, get_tree_density_raw, get_tree_ground_raw, get_tree_growth_raw,
                get_tree_type_raw,
            };
            let tree = (trees.tree_observation)(tile);
            if get_tree_ground_raw(&map, tile) != Some(tree.ground)
                || get_tree_density_raw(&map, tile) != Some(tree.density)
                || get_tree_type_raw(&map, tile) != Some(tree.species)
                || get_tree_count_raw(&map, tile) != Some(tree.count)
                || get_tree_growth_raw(&map, tile) != Some(tree.growth)
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
                || crate::tree_map::get_tree_type_raw(&map, tile).is_none())
            && (!allocation.assertions
                || kind == TileType::MP_WATER
                || crate::water_map::is_coast(&map, tile).is_none())
            && (allocation.assertions || crate::clear_map::is_snow_tile_raw(&map, tile).is_some())
            && crate::movement::native_test_support::map_read_probe(
                allocation, s, tile, mutate, observe,
            ),
    )
}

use crate::follow_track::{State, mask_reserved_tracks, speed_limit};
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
        new_td_bits: state.new_td_bits,
        old_td: state.old_td,
        exitdir: state.exitdir,
        is_tunnel: state.is_tunnel,
        is_bridge: state.is_bridge,
        is_station: state.is_station,
        err: state.err,
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
        State::vehicle(s, mode, v, types, allow90)
    } else {
        State::new(s, mode, v, owner, types, allow90)
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
    let followed = state.follow(unsafe { &*s }, tile, td);
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
    let masked = mask_reserved_tracks(Pbs::acquire(unsafe { &*s }), state);
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
    speed_limit(
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
        crate::follow_track::platform_length_station(&world, tile, dir)
    } else {
        crate::follow_track::platform_length_full(&world, station, tile)
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

/// Install, or with null clear, the scheduler fixture's stand-in for its pause post.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_test_schedule_pause_observer(observer: Option<extern "C" fn(bool)>) {
    // SAFETY: Serialized like every game-state slot; no borrow outlives the write.
    unsafe { *crate::linkgraph_schedule::PAUSE_OBSERVER.get() = observer };
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
