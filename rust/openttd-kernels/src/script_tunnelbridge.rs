/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `ScriptBridge`, `ScriptBridgeList`, `ScriptBridgeList_Length` and `ScriptTunnel`
//! (`script_bridge.cpp`, `script_bridgelist.cpp`, `script_tunnel.cpp`): bridge
//! specifications and lengths, bridge and tunnel tile queries, the build
//! preconditions and the road pieces connecting a built bridge or tunnel.
//! The module contract is `script_tile.rs`'s. Bridge specifications are read
//! through the integrated tunnel/bridge command table at each original call.
#![allow(unsafe_code)] // Exported symbol attributes and the list owner.

use crate::direction_type::DiagDirection;
use crate::script_host::Services;
use crate::script_tile::{ERR_PRECONDITION_FAILED, company_mode_valid, enforce, valid, world};
use crate::script_vehicle::{VT_RAIL, VT_ROAD, VT_WATER};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
use crate::tunnelbridge_cmd::TunnelBridgeServices;

/// `ScriptBridge::IsValidBridge`: `bridge_type < MAX_BRIDGES` and the type is
/// available by the current calendar year.
fn valid_bridge(tb: &TunnelBridgeServices, bridge_type: u32) -> bool {
    bridge_type < crate::bridge::MAX_BRIDGES
        && (tb.availability)(bridge_type).year <= crate::timer_game_common::calendar_year().0
}

fn bridge_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    valid(&w, tile) && w.kind(TileIndex(tile)) == TileType::MP_TUNNELBRIDGE && w.bridge(tile)
}

fn tunnel_tile(s: &Services, tile: u32) -> bool {
    let w = world(s);
    valid(&w, tile) && w.kind(TileIndex(tile)) == TileType::MP_TUNNELBRIDGE && w.tunnel(tile)
}

/// `ScriptBridge::GetBridgeType`; `(BridgeType)-1` for other tiles.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_bridge_type(s: &Services, tile: u32) -> u32 {
    if !bridge_tile(s, tile) {
        return u32::MAX;
    }
    world(s).bridge_type(tile)
}

/// `ScriptBridge::GetMaxSpeed`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_bridge_max_speed(
    tb: &TunnelBridgeServices,
    bridge_type: u32,
) -> i64 {
    if !valid_bridge(tb, bridge_type) {
        return -1;
    }
    i64::from((tb.speed)(bridge_type))
}

/// `ScriptBridge::GetPrice`: the length cost factor times `_price[PR_BUILD_BRIDGE]`
/// times the specification price, shifted right by 8, in saturating `Money` arithmetic.
#[unsafe(no_mangle)]
#[allow(clippy::cast_possible_truncation)] // The clamp bounds `length` to `int32_t`.
pub extern "C" fn openttd_rust_script_bridge_price(
    tb: &TunnelBridgeServices,
    bridge_type: u32,
    length: i64,
) -> i64 {
    if !valid_bridge(tb, bridge_type) {
        return -1;
    }
    let length = length.clamp(0, i64::from(i32::MAX)) as i32;
    let factor = crate::bridge::calc_bridge_len_cost_factor(length);
    crate::company::price(crate::economy_type::PR_BUILD_BRIDGE)
        .saturating_mul(i64::from(factor))
        .saturating_mul(i64::from((tb.price)(bridge_type)))
        >> 8
}

/// `ScriptBridge::GetMaxLength`.
fn max_length(s: &Services, tb: &TunnelBridgeServices, bridge_type: u32) -> i64 {
    if !valid_bridge(tb, bridge_type) {
        return -1;
    }
    // SAFETY: the canonical setting outlives the entry.
    let limit = unsafe { s.command.shared.settings().max_bridge_length.read() };
    i64::from((tb.availability)(bridge_type).max.min(limit)) + 2
}

/// `ScriptBridge::GetMinLength`.
fn min_length(tb: &TunnelBridgeServices, bridge_type: u32) -> i64 {
    if !valid_bridge(tb, bridge_type) {
        return -1;
    }
    i64::from((tb.availability)(bridge_type).min) + 2
}

/// `ScriptBridge::GetMaxLength`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_bridge_max_length(
    s: &Services,
    tb: &TunnelBridgeServices,
    bridge_type: u32,
) -> i64 {
    max_length(s, tb, bridge_type)
}

/// `ScriptBridge::GetMinLength`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_bridge_min_length(
    tb: &TunnelBridgeServices,
    bridge_type: u32,
) -> i64 {
    min_length(tb, bridge_type)
}

/// `ScriptBridge::GetOtherBridgeEnd`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_bridge_other_end(
    s: &Services,
    tb: &TunnelBridgeServices,
    tile: u32,
) -> u32 {
    if !valid(&world(s), tile) || !bridge_tile(s, tile) {
        return crate::tile_type::INVALID_TILE;
    }
    (tb.other_bridge_end)(tile)
}

/// `ScriptTunnel::GetOtherTunnelEnd`: the other end of a tunnel, or the first
/// tile at the start height along an inclined slope's direction.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tunnel_other_end(s: &Services, mut tile: u32) -> u32 {
    let w = world(s);
    if !valid(&w, tile) {
        return crate::tile_type::INVALID_TILE;
    }
    /* If it's a tunnel already, take the easy way out! */
    if tunnel_tile(s, tile) {
        return (s.command.shared.other_tunnel_end)(tile);
    }
    let (x, y) = w.size();
    let (start_tileh, start_z) =
        w.asserted(|m| crate::tile_map::get_tile_slope_z(m, TileIndex(tile), x, y));
    let direction = crate::slope_func::get_inclined_slope_direction_raw(start_tileh.0);
    if direction == crate::direction_type::INVALID_DIAGDIR {
        return crate::tile_type::INVALID_TILE;
    }
    let delta = w
        .dimensions
        .tile_offs_by_diag_dir(DiagDirection(direction), crate::map_func::native_check)
        .cast_unsigned();
    loop {
        tile = tile.wrapping_add(delta);
        if !valid(&w, tile) {
            return crate::tile_type::INVALID_TILE;
        }
        let (_, end_z) =
            w.asserted(|m| crate::tile_map::get_tile_slope_z(m, TileIndex(tile), x, y));
        if start_z == end_z {
            return tile;
        }
    }
}

/// The checks `ScriptBridge::BuildBridge` and `ScriptTunnel::BuildTunnel` share
/// after their own tile checks: the vehicle type, the current rail or road type,
/// and company mode unless building road.
fn vehicle_preconditions(s: &Services, vehicle_type: u32, water: bool) -> bool {
    use crate::script_rail::{current_rail_type, rail_type_available};
    use crate::script_road::{current_road_type, road_type_available};
    let kinds =
        vehicle_type == VT_ROAD || vehicle_type == VT_RAIL || (water && vehicle_type == VT_WATER);
    enforce(kinds, ERR_PRECONDITION_FAILED)
        && enforce(
            vehicle_type != VT_RAIL || rail_type_available(s, current_rail_type()),
            ERR_PRECONDITION_FAILED,
        )
        && enforce(
            vehicle_type != VT_ROAD || road_type_available(s, current_road_type()),
            ERR_PRECONDITION_FAILED,
        )
        && enforce(
            company_mode_valid(s) || vehicle_type == VT_ROAD,
            ERR_PRECONDITION_FAILED,
        )
}

/// `ScriptBridge::BuildBridge` preconditions after the native mode check.
fn build_bridge(s: &Services, vehicle_type: u32, start: u32, end: u32) -> bool {
    let w = world(s);
    let m = w.dimensions;
    enforce(start != end, ERR_PRECONDITION_FAILED)
        && enforce(valid(&w, start) && valid(&w, end), ERR_PRECONDITION_FAILED)
        && enforce(
            m.tile_x(TileIndex(start)) == m.tile_x(TileIndex(end))
                || m.tile_y(TileIndex(start)) == m.tile_y(TileIndex(end)),
            ERR_PRECONDITION_FAILED,
        )
        && vehicle_preconditions(s, vehicle_type, true)
}

/// `ScriptTunnel::BuildTunnel` preconditions after the native mode check.
fn build_tunnel(s: &Services, vehicle_type: u32, start: u32) -> bool {
    enforce(valid(&world(s), start), ERR_PRECONDITION_FAILED)
        && vehicle_preconditions(s, vehicle_type, false)
}

/// The road piece `_BuildBridgeRoad1`/`2` and `_BuildTunnelRoad1`/`2` build.
#[repr(C)]
pub struct RoadPiece {
    pub tile: u32,
    pub bits: u8,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScriptRoadPiece" ["script"] => RoadPiece { tile, bits };
}

/// The road piece on the start side (`first`) or the end side of a bridge or
/// tunnel from `end` to `start`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_tunnelbridge_road_piece(
    s: &Services,
    end: u32,
    start: u32,
    first: u8,
) -> RoadPiece {
    let w = world(s);
    let check = |ok| w.check(ok);
    let dir_1 = w
        .dimensions
        .diagdir_between_tiles(TileIndex(end), TileIndex(start))
        .0;
    let dir_2 = crate::direction_func::reverse_diag_dir_raw(dir_1, check);
    let (tile, dir, bits_dir) = if first != 0 {
        (start, dir_1, dir_2)
    } else {
        (end, dir_2, dir_1)
    };
    check(dir < 4);
    RoadPiece {
        tile: tile.wrapping_add(
            w.dimensions
                .tile_offs_by_diag_dir(DiagDirection(dir), crate::map_func::native_check)
                .cast_unsigned(),
        ),
        bits: crate::road_func::diag_dir_to_road_bits_raw(bits_dir, check),
    }
}

/// `ScriptBridgeList` and `ScriptBridgeList_Length` (`length`): the valid bridge
/// types in index order, optionally within their length limits.
/// # Safety
/// `owner` is the live list of the constructing object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_bridge_list(
    s: &Services,
    tb: &TunnelBridgeServices,
    owner: *mut crate::script_list::List,
    filter: u8,
    length: i64,
) {
    for j in 0..crate::bridge::MAX_BRIDGES {
        if !valid_bridge(tb, j) {
            continue;
        }
        if filter != 0 && !(length >= min_length(tb, j) && length <= max_length(s, tb, j)) {
            continue;
        }
        // SAFETY: caller contract.
        unsafe { (*owner).add(i64::from(j), 0) };
    }
}

/// C byte exports of the boolean entries.
macro_rules! byte_entries {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) => $body:expr;)*) => {$(
        $(#[$doc])*
        #[unsafe(no_mangle)]
        pub extern "C" fn $name($($arg: $ty),*) -> u8 {
            u8::from($body)
        }
    )*};
}

byte_entries! {
    /// `ScriptBridge::IsValidBridge`.
    openttd_rust_script_bridge_is_valid(tb: &TunnelBridgeServices, bridge_type: u32) => valid_bridge(tb, bridge_type);
    /// `ScriptBridge::IsBridgeTile`.
    openttd_rust_script_bridge_is_tile(s: &Services, tile: u32) => bridge_tile(s, tile);
    /// `ScriptBridge::BuildBridge` preconditions after the native mode check.
    openttd_rust_script_bridge_build(s: &Services, vehicle_type: u32, start: u32, end: u32) => build_bridge(s, vehicle_type, start, end);
    /// `ScriptTunnel::IsTunnelTile`.
    openttd_rust_script_tunnel_is_tile(s: &Services, tile: u32) => tunnel_tile(s, tile);
    /// `ScriptTunnel::BuildTunnel` preconditions after the native mode check.
    openttd_rust_script_tunnel_build(s: &Services, vehicle_type: u32, start: u32) => build_tunnel(s, vehicle_type, start);
}
