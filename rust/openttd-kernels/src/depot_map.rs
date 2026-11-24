impl crate::map_func::World<'_> {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn depot(&self, t: u32, mode: u8) -> bool {
        match mode {
            crate::transport_type::TRANSPORT_RAIL => self.rail_depot(t),
            crate::transport_type::TRANSPORT_ROAD => self.road_depot(t),
            _ => unreachable!("non-water follower depot"),
        }
    }
}

/// `IsDepotTile`: is the given tile a tile with a depot on it? It ends in `IsHangarTile`,
/// which reads the station's airport specification, so it stays a native service (#565).
pub(crate) fn is_depot_tile(
    shared: &crate::services::Services,
    tile: crate::tile_type::TileIndex,
) -> bool {
    (shared.is_depot_tile)(tile.0)
}
