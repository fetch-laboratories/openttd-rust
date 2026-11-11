/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared field access from `tunnelbridge_map.h`.
use crate::bitmath_func::gb;
use crate::map_func::TileTypeAccess;
use crate::services::TransportType;
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;

/// `GetTunnelBridgeTransportType`, with the native tile-type assertion policy.
#[inline]
#[allow(clippy::cast_possible_truncation)] // GB returns the original two-bit value.
pub(crate) fn get_tunnel_bridge_transport_type(
    m: &TileTypeAccess,
    tile: TileIndex,
) -> Option<TransportType> {
    let fields = m.require_type(tile, TileType::MP_TUNNELBRIDGE)?;
    Some(TransportType(gb(u32::from(fields.m5()), 2, 2) as u8))
}

#[allow(clippy::cast_possible_truncation)] // GB returns the original field width.
impl crate::map_func::World<'_> {
    /// `GetTunnelBridgeTransportType`.
    #[inline]
    pub(crate) fn tunnel_bridge_transport(&self, t: u32) -> u8 {
        self.asserted(|m| get_tunnel_bridge_transport_type(m, TileIndex(t)))
            .0
    }
    #[inline]
    pub(crate) fn tunnel_bridge_dir(&self, t: u32) -> u8 {
        self.check(self.kind(TileIndex(t)) == TileType::MP_TUNNELBRIDGE);
        gb(u32::from(Self::byte(self.types.m5(TileIndex(t)))), 0, 2) as u8
    }
}
