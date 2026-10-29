/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared field access from `tunnelbridge_map.h`.
use crate::bitmath_func::gb;
use crate::map_func::TileTypeAccess;
use crate::services::TransportType;
use crate::tile_type::{MP_TUNNELBRIDGE, TileIndex};

/// `GetTunnelBridgeTransportType`, with the native tile-type assertion policy.
#[inline]
#[allow(clippy::cast_possible_truncation)] // GB returns the original two-bit value.
pub(crate) fn get_tunnel_bridge_transport_type(
    m: &TileTypeAccess,
    tile: TileIndex,
) -> Option<TransportType> {
    m.require_type(tile.0, MP_TUNNELBRIDGE)?;
    Some(TransportType(gb(u32::from(m.m5(tile.0)?), 2, 2) as u8))
}

impl crate::map_func::World<'_> {
    pub(crate) fn tunnel_bridge_dir(&self, t: u32) -> u8 {
        self.check(self.kind(t) == crate::tile_type::MP_TUNNELBRIDGE);
        Self::byte(self.types.m5(t)) & 3
    }
}
