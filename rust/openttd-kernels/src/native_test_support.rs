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
use crate::landscape_generation::{
    Decoder, Frame as GenerationFrame, GenerationServices, Settings,
};

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
    decoder: *const Decoder,
    g: Settings,
    mode: u8,
) -> bool {
    unsafe { crate::landscape_generation::generate_landscape(a, frame, decoder, g, mode) }
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
