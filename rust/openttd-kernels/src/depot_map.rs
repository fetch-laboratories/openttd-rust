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

/// `GetDepotVehicleType`.
pub(crate) fn vehicle_type(shared: &crate::services::Services, tile: u32) -> u8 {
    match crate::map_func::World::new(shared).kind(tile) {
        crate::tile_type::MP_RAILWAY => crate::vehicle_type::VEH_TRAIN,
        crate::tile_type::MP_ROAD => crate::vehicle_type::VEH_ROAD,
        crate::tile_type::MP_WATER => crate::vehicle_type::VEH_SHIP,
        crate::tile_type::MP_STATION => crate::vehicle_type::VEH_AIRCRAFT,
        _ => {
            (shared.not_reached)();
            std::process::abort();
        }
    }
}
/// `IsDepotTile`, including hangars in the last short-circuit branch.
pub(crate) fn is_depot(shared: &crate::services::Services, tile: u32) -> bool {
    let world = crate::map_func::World::new(shared);
    world.rail_depot(tile)
        || world.road_depot(tile)
        || world.ship_depot(tile)
        || crate::station_map::is_hangar_tile(tile, crate::newgrf_airport::runtime())
}
