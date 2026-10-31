/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native evidence bridges; linked by `openttd_test` and fixture-only game adapters.
#![allow(unsafe_code, clippy::too_many_lines)]
use crate::map_func::{TileTypeAccess, TileTypeAllocation};
use crate::tile_map::{get_tile_slope, get_tile_z, get_tropic_zone};

/// # Safety
/// For nonzero counts, uphold `TileTypeAccess::raw`'s validity/serialization
/// contract for this call. The callback is the original noexcept accessor or an
/// equivalent live fixture; it keeps the native out-of-range assertion policy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_shared_tile_type(
    allocation: TileTypeAllocation,
    callback: extern "C" fn(u32) -> u8,
    tile: u32,
) -> u8 {
    unsafe { TileTypeAccess::raw(allocation, callback) }.read(tile)
}

/// Native default probe: compare audited reads with the unchanged typed services.
/// # Safety
/// Same allocation contract as the scheduler; all tables are live and immutable.
/// Mutation preserves the allocation/dimensions and cannot reset/load/yield.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub unsafe extern "C" fn openttd_rust_map_read_probe(
    allocation: crate::services::TileTypeAllocation,
    shared: *const crate::services::Services,
    clear: *const crate::ground::ClearServices,
    trees: *const crate::trees::TreeServices,
    tile: u32,
    mutate: extern "C" fn(u32),
) -> bool {
    let (s, c, trees) = unsafe { (&*shared, &*clear, &*trees) };
    let map = unsafe { TileTypeAccess::raw(allocation, s.tile_type) };
    let x = (s.map_max_x)() + 1;
    let y = (s.map_max_y)() + 1;
    if get_tile_slope(&map, tile, x, y) != Some((s.tile_slope)(tile))
        || crate::tile_map::is_tile_flat(&map, tile, x, y)
            != Some((unsafe { &*(s.station_commands)() }.tile_flat)(tile))
        || get_tile_z(&map, tile, x, y) != Some((s.tile_z)(tile))
        || get_tropic_zone(&map, tile) != Some((s.tropic_zone)(tile))
    {
        return false;
    }
    match map.read(tile) {
        0 => {
            use crate::clear_map::{
                get_clear_counter, get_clear_density, get_clear_ground, get_fence, get_field_type,
                get_industry_index_of_field, is_snow_tile,
            };
            if is_snow_tile(&map, tile) != Some((c.snow)(tile))
                || get_clear_ground(&map, tile) != Some((s.clear_ground)(tile).0)
                || get_clear_density(&map, tile).map(u32::from) != Some((c.density)(tile))
                || get_clear_counter(&map, tile).map(u32::from) != Some((c.counter)(tile))
            {
                return false;
            }
            if get_clear_ground(&map, tile) == Some(crate::clear_map::CLEAR_FIELDS) {
                let fences = (c.fences)(tile);
                let field = (c.field)(tile);
                if [fences.ne, fences.se, fences.sw, fences.nw]
                    .into_iter()
                    .zip(0_u8..)
                    .any(|(expected, dir)| get_fence(&map, tile, dir) != Some(expected))
                    || get_field_type(&map, tile) != Some(field.field_type)
                    || get_industry_index_of_field(&map, tile) != Some(field.industry)
                {
                    return false;
                }
            } else if allocation.assertions && get_field_type(&map, tile).is_some() {
                return false;
            }
        }
        4 => {
            use crate::tree_map::{
                get_tree_count, get_tree_density, get_tree_ground, get_tree_growth, get_tree_type,
            };
            let tree = (trees.tree_observation)(tile);
            if get_tree_ground(&map, tile) != Some(tree.ground)
                || get_tree_density(&map, tile) != Some(tree.density)
                || get_tree_type(&map, tile) != Some(tree.species)
                || get_tree_count(&map, tile) != Some(tree.count)
                || get_tree_growth(&map, tile) != Some(tree.growth)
            {
                return false;
            }
        }
        6 => {
            let water = unsafe { &*(s.water_services)() };
            if crate::water_map::is_coast(&map, tile) != Some((water.coast)(tile))
                || crate::water_map::is_non_flooding_water_tile(&map, tile)
                    != Some((water.non_flooding)(tile))
                || crate::water_map::get_water_class(&map, tile) != Some((c.water_class)(tile).0)
            {
                return false;
            }
        }
        _ => {}
    }
    let kind = map.read(tile);
    (!allocation.assertions || kind == 0 || crate::clear_map::is_snow_tile(&map, tile).is_none())
        && (!allocation.assertions
            || kind == 4
            || crate::tree_map::get_tree_type(&map, tile).is_none())
        && (!allocation.assertions || kind == 6 || crate::water_map::is_coast(&map, tile).is_none())
        && (allocation.assertions || crate::clear_map::is_snow_tile(&map, tile).is_some())
        && crate::movement::native_test_support::map_read_probe(allocation, s, tile, mutate)
}

use crate::follow_track::{State, mask_reserved_tracks, speed_limit};
use crate::pbs::Pbs;
use crate::services::Services;
use crate::vehicle::{OpenTTDVehicle, VehicleRef};

/// # Safety
/// The output is writable; services and optional native vehicle remain live and serialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_init(
    out: *mut State,
    s: *const Services,
    mode: u8,
    v: *mut OpenTTDVehicle,
    owner: u8,
    types: u64,
    allow90: bool,
    masking: bool,
    from_vehicle: bool,
) {
    let s = unsafe { &*s };
    let v = std::ptr::NonNull::new(v).map(VehicleRef);
    let state = if from_vehicle {
        State::vehicle(s, mode, v, types, allow90, masking)
    } else {
        State::new(s, mode, v, owner, types, allow90, masking)
    };
    unsafe {
        out.write(state);
    }
}
/// # Safety
/// The stack follower and shared game world remain live and serialized throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_follow(
    s: *const Services,
    state: *mut State,
    tile: u32,
    td: u8,
) -> bool {
    unsafe { (&mut *state).follow(&*s, tile, td) }
}
/// # Safety
/// Same stack follower and serialized world contract as Follow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_follower_mask(s: *const Services, state: *mut State) -> bool {
    let s = unsafe { &*s };
    let state = unsafe { &mut *state };
    if !state.masking {
        return true;
    }
    let mut out = state.output();
    let result = mask_reserved_tracks(Pbs::acquire(s), &mut out);
    state.restore(out);
    result
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
pub unsafe extern "C" fn openttd_test_generate_landscape(
    a: *const GenerationServices,
    frame: *mut GenerationFrame,
    input: *const HeightmapInput,
    g: Settings,
    mode: u8,
) -> bool {
    unsafe { crate::landscape_generation::generate_landscape(a, frame, input, g, mode) }
}

/// `LoadHeightmap`, which has no production caller beside `GenerateLandscape`.
/// # Safety
/// Same contract as the production generation entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_load_heightmap(
    a: *const GenerationServices,
    input: *const HeightmapInput,
    rotation: u8,
    freeform: bool,
    heightmap_height: u8,
    max_height: u8,
) -> bool {
    unsafe {
        crate::landscape_generation::load_heightmap(
            a,
            input,
            rotation,
            freeform,
            heightmap_height,
            max_height,
        )
    }
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

/// Seed a fixture train's `cached_max_curve_speed`; production writes stay in Rust owners.
/// # Safety
/// `state` is a live, exclusively accessed train state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_train_state_set_cached_max_curve_speed(
    state: *mut crate::train_state::State,
    value: u16,
) {
    crate::raw_field::write_field!(state, cached_max_curve_speed, value);
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
    unsafe { &*services }.target(rotation, x, y).0
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
    crate::station::spatial::move_sign(unsafe { &*s }, st, tile);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_test_station_radius(
    s: *const Services,
    center: u32,
    distance: u32,
    context: *mut std::ffi::c_void,
    visit: extern "C" fn(*mut std::ffi::c_void, *mut crate::station::BaseStation),
) {
    crate::station::spatial::radius(unsafe { &*s }, center, distance, |st| visit(context, st));
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
    let split =
        |ids: &mut [u16], dim| unsafe { crate::kdtree::native_split(s, ids, |id| xy(id, dim)) };
    let mut tree = crate::kdtree::Kdtree::new();
    let observe = |tree: &crate::kdtree::Kdtree<u16>, phase| {
        tree.contained([0, 0], [10, 10], &xy, |id| visit(context, phase, id));
        tree.contained([1, 1], [2, 4], &xy, |id| visit(context, phase + 100, id));
        if tree.count() != 0 {
            visit(context, phase + 200, tree.nearest([1, 2], &xy));
        }
    };
    tree.build(&mut (0..20).collect::<Vec<_>>(), &split);
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
    tree.build(&mut [], &split);
    observe(&tree, 43);
    tree.insert(12, &xy, &split);
    observe(&tree, 44);
}
