/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `tunnelbridge.h` helpers.
use crate::map_func::Map;
use crate::tile_type::TileIndex;
/// Number of infrastructure pieces per tunnel/bridge tile.
pub const TUNNELBRIDGE_TRACKBIT_FACTOR: u32 = 4;
/// `GetTunnelBridgeLength`, excluding the endpoints.
#[must_use]
pub fn get_tunnel_bridge_length(map: Map, begin: u32, end: u32) -> u32 {
    let first = map
        .tile_x(TileIndex(begin))
        .wrapping_add(map.tile_y(TileIndex(begin)));
    let last = map
        .tile_x(TileIndex(end))
        .wrapping_add(map.tile_y(TileIndex(end)));
    last.abs_diff(first).wrapping_sub(1)
}

/// `SetTunnelBridgeOwner`: sets the ownership of the bridge/tunnel ramps.
pub(crate) fn set_tunnel_bridge_owner(
    begin: TileIndex,
    end: TileIndex,
    owner: crate::company_type::Owner,
    s: &crate::services::Services,
) {
    (s.set_tile_owner)(begin.0, owner.0);
    (s.set_tile_owner)(end.0, owner.0);
}

/// Native tunnel portal sound and visibility frames from `tunnelbridge_cmd.cpp`.
pub(crate) const TUNNEL_SOUND_FRAME: u8 = 1;
pub(crate) const TUNNEL_VISIBILITY: [u8; 4] = [12, 8, 8, 12];
