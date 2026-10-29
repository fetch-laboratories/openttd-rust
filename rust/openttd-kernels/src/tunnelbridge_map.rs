/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared field access from `tunnelbridge_map.h`.
use crate::map_func::TileTypeAccess;
use crate::services::TransportType;
use crate::tile_type::{MP_TUNNELBRIDGE, TileIndex};

/// `GetTunnelBridgeTransportType`, with the native tile-type assertion policy.
#[inline]
pub(crate) fn get_tunnel_bridge_transport_type(
    m: &TileTypeAccess,
    tile: TileIndex,
) -> Option<TransportType> {
    m.require_type(tile.0, MP_TUNNELBRIDGE)?;
    Some(TransportType((m.m5(tile.0)? >> 2) & 3))
}
