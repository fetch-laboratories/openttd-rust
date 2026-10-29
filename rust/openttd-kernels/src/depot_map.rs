impl crate::map_func::World<'_> {
    pub(crate) fn depot(&self, t: u32, mode: u8) -> bool {
        match mode {
            crate::transport_type::TRANSPORT_RAIL => self.rail_depot(t),
            crate::transport_type::TRANSPORT_ROAD => self.road_depot(t),
            _ => unreachable!("non-water follower depot"),
        }
    }
}
