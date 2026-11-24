use crate::tile_type::TileIndex;

impl crate::map_func::World<'_> {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn depot(&self, t: u32, mode: u8) -> bool {
        match mode {
            crate::transport_type::TRANSPORT_RAIL => self.rail_depot(TileIndex(t)),
            crate::transport_type::TRANSPORT_ROAD => self.road_depot(TileIndex(t)),
            _ => unreachable!("non-water follower depot"),
        }
    }
}

/// `IsDepotTile`: is the given tile a tile with a depot on it?
pub(crate) fn is_depot_tile(world: &crate::map_func::World<'_>, tile: TileIndex) -> bool {
    world.rail_depot(tile)
        || world.road_depot(tile)
        || world.is_ship_depot_tile(tile)
        || crate::station_map::is_hangar_tile(tile, crate::newgrf_airport::runtime())
}
