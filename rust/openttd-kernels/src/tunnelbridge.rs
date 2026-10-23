/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `tunnelbridge.h` helpers.
use crate::map_func::Map;
/// Number of infrastructure pieces per tunnel/bridge tile.
pub const TUNNELBRIDGE_TRACKBIT_FACTOR: u32 = 4;
/// `GetTunnelBridgeLength`, excluding the endpoints.
#[must_use]
pub fn get_tunnel_bridge_length(map: Map, begin: u32, end: u32) -> u32 {
    let first = map.tile_x(begin).wrapping_add(map.tile_y(begin));
    let last = map.tile_x(end).wrapping_add(map.tile_y(end));
    last.abs_diff(first).wrapping_sub(1)
}

/// Native tunnel portal sound and visibility frames from `tunnelbridge_cmd.cpp`.
pub(crate) const TUNNEL_SOUND_FRAME: u8 = 1;
pub(crate) const TUNNEL_VISIBILITY: [u8; 4] = [12, 8, 8, 12];
/// Native afterload uses the same portal table as movement.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_movement_tunnel_visibility(direction: u8) -> u8 {
    TUNNEL_VISIBILITY[usize::from(direction)]
}
