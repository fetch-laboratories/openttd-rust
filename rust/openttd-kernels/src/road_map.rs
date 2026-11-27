/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `road_map.h` values.
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
/// `Roadside::Trees`.
pub const ROADSIDE_TREES: Roadside = Roadside::Trees;
/// `Roadside::Paved`.
pub const ROADSIDE_PAVED: Roadside = Roadside::Paved;

/// `RoadTileType::Normal`.
pub const ROAD_TILE_NORMAL: u8 = RoadTileType::Normal.0;
/// `RoadTileType::Crossing`.
pub const ROAD_TILE_CROSSING: u8 = RoadTileType::Crossing.0;
/// `RoadTileType::Depot`.
pub const ROAD_TILE_DEPOT: u8 = RoadTileType::Depot.0;

/// `SetRoadTypeRoad` delegates to the canonical typed map setter.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn set_road_type_road(
    s: &crate::services::Services,
    tile: TileIndex,
    rt: crate::road_type::RoadType,
) {
    (s.set_road_type)(
        tile.0,
        u8::from(crate::road_type::RoadTramType::RTT_ROAD.0),
        rt.0,
    );
}
/// `SetRoadTypeTram` delegates to the canonical typed map setter.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn set_road_type_tram(
    s: &crate::services::Services,
    tile: TileIndex,
    rt: crate::road_type::RoadType,
) {
    (s.set_road_type)(
        tile.0,
        u8::from(crate::road_type::RoadTramType::RTT_TRAM.0),
        rt.0,
    );
}

/// `Roadside::GrassRoadWorks`.
pub const ROADSIDE_GRASS_ROAD_WORKS: Roadside = Roadside(6);

/// `GetRoadTileType`, retaining its `IsTileType` assertion.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_road_tile_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::services::RoadTileKind> {
    let fields = m.require_type(TileIndex(tile), TileType::MP_ROAD)?;
    Some(crate::services::RoadTileKind(fields.m5() >> 6))
}
/// `IsNormalRoad`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_normal_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<bool> {
    Some(get_road_tile_type(m, tile)?.0 == ROAD_TILE_NORMAL)
}
#[allow(clippy::inline_always)]
#[inline(always)]
fn require_normal_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<()> {
    m.available(TileIndex(tile))?;
    if m.assertions() {
        m.check(is_normal_road(m, tile)?)?;
    }
    Some(())
}
/// `GetRoadBits`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_road_bits(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    kind: crate::services::RoadTramType,
) -> Option<crate::services::RoadBits> {
    require_normal_road(m, tile)?;
    Some(crate::services::RoadBits(
        if kind.0 == crate::road_type::RTT_TRAM.0 {
            m.m3(TileIndex(tile))?
        } else {
            m.m5(TileIndex(tile))?
        } & 15,
    ))
}
/// `GetDisallowedRoadDirections`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_disallowed_road_directions(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<u8> {
    require_normal_road(m, tile)?;
    Some((m.m5(TileIndex(tile))? >> 4) & 3)
}
/// `GetRoadside`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_roadside(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<Roadside> {
    Some(Roadside((m.m6(TileIndex(tile))? >> 3) & 7))
}
/// `HasRoadWorks`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn has_road_works(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<bool> {
    Some(get_roadside(m, tile)?.0 >= ROADSIDE_GRASS_ROAD_WORKS.0)
}
/// `MayHaveRoad`, retaining the source tile dispatch and named predicates.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn may_have_road(m: &crate::map_func::TileTypeAccess, tile: TileIndex) -> Option<bool> {
    Some(match crate::tile_map::get_tile_type(m, tile)? {
        TileType::MP_ROAD => true,
        TileType::MP_STATION => crate::station_map::is_any_road_stop(m, tile)?,
        TileType::MP_TUNNELBRIDGE => {
            crate::tunnelbridge_map::get_tunnel_bridge_transport_type(m, tile)?.0
                == crate::transport_type::TRANSPORT_ROAD
        }
        _ => false,
    })
}
#[allow(clippy::inline_always)]
#[inline(always)]
fn require_may_have_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<()> {
    m.available(TileIndex(tile))?;
    if m.assertions() {
        m.check(may_have_road(m, TileIndex(tile))?)?;
    }
    Some(())
}
/// `GetRoadTypeRoad`, with the canonical `MayHaveRoad` precondition.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_road_type_road(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::services::RoadType> {
    require_may_have_road(m, tile)?;
    Some(crate::services::RoadType(m.m4(TileIndex(tile))? & 63))
}
/// `GetRoadTypeTram`, with the canonical `MayHaveRoad` precondition.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_road_type_tram(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::services::RoadType> {
    require_may_have_road(m, tile)?;
    Some(crate::services::RoadType(
        u8::try_from((m.m8(TileIndex(tile))? >> 6) & 63).expect("six-bit road type"),
    ))
}
/// `GetRoadType`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_road_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    kind: crate::services::RoadTramType,
) -> Option<crate::services::RoadType> {
    if kind.0 == crate::road_type::RTT_TRAM.0 {
        get_road_type_tram(m, tile)
    } else {
        get_road_type_road(m, tile)
    }
}
/// `HasTileRoadType`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn has_tile_road_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    kind: crate::services::RoadTramType,
) -> Option<bool> {
    Some(get_road_type(m, tile, kind)?.0 != crate::road_type::INVALID_ROADTYPE)
}

/// `Roadside::Barren`.
pub const ROADSIDE_BARREN: Roadside = Roadside(0);
/// `Roadside::Grass`.
pub const ROADSIDE_GRASS: Roadside = Roadside(1);

#[allow(clippy::cast_possible_truncation)] // GB returns the original field width.
impl crate::map_func::World<'_> {
    /// `GetRoadTileType`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn road_tile_type(&self, t: u32) -> crate::services::RoadTileKind {
        self.asserted(|m| get_road_tile_type(m, t))
    }
    /// `IsLevelCrossingTile`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn level_crossing_tile(&self, t: u32) -> bool {
        self.kind(TileIndex(t)) == TileType::MP_ROAD
            && self.road_tile_type(t).0 == ROAD_TILE_CROSSING
    }
    /// `IsLevelCrossing`, with `GetRoadTileType`'s tile-type assertion.
    pub(crate) fn level_crossing(&self, t: u32) -> bool {
        self.road_tile_type(t).0 == ROAD_TILE_CROSSING
    }
    /// `IsCrossingBarred`.
    pub(crate) fn crossing_barred(&self, t: u32) -> bool {
        self.check(self.level_crossing(t));
        crate::bitmath_func::has_bit(Self::byte(self.types.m5(TileIndex(t))), 5)
    }
    /// `SetCrossingBarred`: set the bar state of a level crossing.
    pub(crate) fn set_crossing_barred(&self, t: TileIndex, barred: bool) {
        self.check(self.level_crossing(t.0));
        let mut m5 = u32::from(Self::byte(self.types.m5(t)));
        crate::bitmath_func::assign_bit(&mut m5, 5, barred);
        let _ = self.types.set_m5(t, m5 as u8);
    }
    /// `GetCrossingRailBits`: `AxisToTrackBits(GetCrossingRailAxis(t))`.
    pub(crate) fn crossing_rail_bits(&self, t: u32) -> u8 {
        self.check(self.level_crossing(t));
        let check = |ok| self.check(ok);
        let axis = crate::direction_func::other_axis_raw(self.crossing_road_axis(t), check);
        crate::track_func::axis_to_track_bits_raw(axis, check)
    }
    /// `GetAllRoadBits`.
    pub(crate) fn all_road_bits(&self, t: u32) -> u8 {
        self.road_bits(t, false) | self.road_bits(t, true)
    }
    /// `GetCrossingRoadAxis`, with its `IsLevelCrossing` assertion.
    pub(crate) fn crossing_road_axis(&self, t: u32) -> u8 {
        self.check(self.road_tile_type(t).0 == ROAD_TILE_CROSSING);
        Self::byte(self.types.m5(TileIndex(t))) & 1
    }
    /// `IsRoadDepot`: return whether a tile is a road depot.
    #[inline]
    pub(crate) fn is_road_depot(&self, t: TileIndex) -> bool {
        self.road_tile_type(t.0).0 == ROAD_TILE_DEPOT
    }
    /// `IsNormalRoad`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_normal_road(&self, t: u32) -> bool {
        self.road_tile_type(t).0 == ROAD_TILE_NORMAL
    }
    /// `IsNormalRoadTile`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn is_normal_road_tile(&self, t: u32) -> bool {
        self.asserted(|m| crate::tile_map::get_tile_type(m, TileIndex(t))) == TileType::MP_ROAD
            && self.is_normal_road(t)
    }
    /// `GetDisallowedRoadDirections`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn disallowed_road_directions(&self, t: u32) -> u8 {
        self.asserted(|m| get_disallowed_road_directions(m, t))
    }
    /// `HasRoadWorks`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn has_road_works(&self, t: u32) -> bool {
        self.asserted(|m| has_road_works(m, t))
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn road_bits(&self, t: u32, tram: bool) -> u8 {
        self.asserted(|m| get_road_bits(m, t, crate::services::RoadTramType(tram)))
            .0
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    /// `IsRoadDepotTile`: return whether a tile is a road depot tile.
    pub(crate) fn road_depot(&self, t: TileIndex) -> bool {
        self.kind(t) == TileType::MP_ROAD && self.is_road_depot(t)
    }
    /// `GetRoadDepotDirection`: get the direction of the exit of a road depot. `IsRoadDepot`
    /// and its nested `GetRoadTileType` assertion are evaluated only where the native build
    /// asserts.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn road_depot_dir(&self, t: u32) -> u8 {
        self.assert_with(|| self.is_road_depot(TileIndex(t)));
        crate::bitmath_func::gb(u32::from(self.asserted(|m| m.m5(TileIndex(t)))), 0, 2) as u8
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn road_type(&self, t: u32, tram: bool) -> u8 {
        self.asserted(|m| get_road_type(m, t, crate::services::RoadTramType(tram)))
            .0
    }
    /// `GetRoadOwner`, with its `MayHaveRoad` assertion.
    pub(crate) fn road_owner(&self, t: u32, tram: bool) -> u8 {
        self.asserted(|m| require_may_have_road(m, t));
        if !tram {
            let field = if self.is_normal_road_tile(t) {
                Self::byte(self.types.m1(TileIndex(t)))
            } else {
                Self::byte(self.types.m7(TileIndex(t)))
            };
            return field & 0x1F;
        }
        /* Trams don't need OWNER_TOWN, and remapping OWNER_NONE
         * to OWNER_TOWN makes it use one bit less */
        let o = (Self::byte(self.types.m3(TileIndex(t))) >> 4) & 0xF;
        if o == crate::company_type::OWNER_TOWN {
            crate::company_type::OWNER_NONE
        } else {
            o
        }
    }
}

/// `HasTownOwnedRoad`, preserving the short circuit before `GetRoadOwner`.
pub(crate) fn has_town_owned_road(world: &crate::map_func::World<'_>, tile: u32) -> bool {
    world.road_type(tile, false) != crate::road_type::INVALID_ROADTYPE
        && world.road_owner(tile, false) == crate::company_type::OWNER_TOWN
}

/// `GetPresentRoadTypes` over the canonical map fields.
pub(crate) fn get_present_road_types(world: &crate::map_func::World<'_>, tile: u32) -> u64 {
    let mut types = 0;
    if may_have_road(&world.types, TileIndex(tile)).expect("map tile") {
        for tram in [false, true] {
            if world.road_type(tile, tram) != crate::road_type::INVALID_ROADTYPE {
                types |= 1_u64 << world.road_type(tile, tram);
            }
        }
    }
    types
}
/// `HasTileAnyRoadType`, including the `MayHaveRoad` short circuit.
pub(crate) fn has_tile_any_road_type(
    world: &crate::map_func::World<'_>,
    tile: u32,
    types: u64,
) -> bool {
    may_have_road(&world.types, TileIndex(tile)).expect("map tile")
        && get_present_road_types(world, tile) & types != 0
}

/// `GetCrossingRailAxis`: get the rail axis of a level crossing.
pub(crate) fn get_crossing_rail_axis(
    world: &crate::map_func::World<'_>,
    t: TileIndex,
) -> crate::direction_type::Axis {
    world.check(world.level_crossing(t.0));
    crate::direction_func::other_axis(
        crate::direction_type::Axis(world.crossing_road_axis(t.0)),
        |ok| world.check(ok),
    )
}
/// `GetCrossingRailTrack`: get the rail track of a level crossing.
pub(crate) fn get_crossing_rail_track(
    world: &crate::map_func::World<'_>,
    tile: TileIndex,
) -> crate::track_type::Track {
    crate::track_func::axis_to_track(get_crossing_rail_axis(world, tile), |ok| world.check(ok))
}
/// `SetRoadside`: set the decorations of a road.
#[allow(
    clippy::cast_possible_truncation,
    reason = "SB stores back into the original uint8_t m6"
)]
pub(crate) fn set_roadside(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
    s: Roadside,
) -> Option<()> {
    let mut bits = u32::from(m.m6(tile)?);
    crate::bitmath_func::sb(&mut bits, 3, 3, u32::from(s.0));
    m.set_m6(tile, bits as u8)
}
/// `IncreaseRoadWorksCounter`: increase the progress counter of road works. Returns true if
/// the road works are in the last stage.
#[allow(
    clippy::cast_possible_truncation,
    reason = "AB stores back into the original uint8_t m7"
)]
pub(crate) fn increase_road_works_counter(
    m: &crate::map_func::TileTypeAccess,
    t: TileIndex,
) -> Option<bool> {
    let mut bits = u32::from(m.m7(t)?);
    crate::bitmath_func::ab(&mut bits, 0, 4, 1);
    m.set_m7(t, bits as u8)?;

    Some(crate::bitmath_func::gb(u32::from(m.m7(t)?), 0, 4) == 15)
}
/// `StartRoadWorks`: start road works on a tile. Requires `!HasRoadWorks(t)`.
pub(crate) fn start_road_works(m: &crate::map_func::TileTypeAccess, t: TileIndex) -> Option<()> {
    m.check(!has_road_works(m, t.0)?)?;
    /* Remove any trees or lamps in case or roadwork */
    match get_roadside(m, t.0)? {
        Roadside::Barren | Roadside::Grass => set_roadside(m, t, Roadside::GrassRoadWorks),
        _ => set_roadside(m, t, Roadside::PavedRoadWorks),
    }
}
/// `TerminateRoadWorks`: terminate road works on a tile. Requires `HasRoadWorks(t)`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "SB stores back into the original uint8_t m7"
)]
pub(crate) fn terminate_road_works(
    m: &crate::map_func::TileTypeAccess,
    t: TileIndex,
) -> Option<()> {
    m.check(has_road_works(m, t.0)?)?;
    set_roadside(
        m,
        t,
        if get_roadside(m, t.0)? == Roadside::GrassRoadWorks {
            Roadside::Grass
        } else {
            Roadside::Paved
        },
    )?;
    /* Stop the counter */
    let mut bits = u32::from(m.m7(t)?);
    crate::bitmath_func::sb(&mut bits, 0, 4, 0);
    m.set_m7(t, bits as u8)
}

/// Native `Roadside` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Roadside(pub u8);

/// Native road tile kind.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadTileType(pub u8);

#[allow(non_upper_case_globals)]
impl RoadTileType {
    /// Normal road
    pub const Normal: Self = Self(0);
    /// Level crossing
    pub const Crossing: Self = Self(1);
    /// Depot (one entrance)
    pub const Depot: Self = Self(2);
}

#[allow(non_upper_case_globals)]
impl Roadside {
    /// Road on barren land
    pub const Barren: Self = Self(0);
    /// Road on grass
    pub const Grass: Self = Self(1);
    /// Road with paved sidewalks
    pub const Paved: Self = Self(2);
    /// Road with street lights on paved sidewalks
    pub const StreetLights: Self = Self(3);
    /// Road with trees on paved sidewalks
    pub const Trees: Self = Self(5);
    /// Road on grass with road works
    pub const GrassRoadWorks: Self = Self(6);
    /// Road with sidewalks and road works
    pub const PavedRoadWorks: Self = Self(7);
}

#[cfg(test)]
mod crossing_tests {
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation, World};
    use crate::tile_type::TileIndex;
    use std::cell::RefCell;

    thread_local! {
        static CHECKS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn record_check(condition: bool) {
        CHECKS.with(|c| c.borrow_mut().push(condition));
    }
    fn take_checks() -> Vec<bool> {
        CHECKS.with(|c| std::mem::take(&mut *c.borrow_mut()))
    }

    /// A shared table over a four-tile map whose native assertion records: tile 0 a level
    /// crossing (`MP_ROAD`, `ROAD_TILE_CROSSING` in m5 bits 6..7) with every other m5 bit
    /// set, tile 1 a rail depot facing each direction in turn, the rest clear.
    fn map(m5: [u8; 4], kinds: [u8; 4]) -> &'static crate::services::Services {
        let base: Box<[BaseTileLayout]> = (0..4)
            .map(|i| BaseTileLayout {
                kind: kinds[i] << 4,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: m5[i],
            })
            .collect();
        let extended: Box<[ExtendedTileLayout]> = (0..4)
            .map(|_| ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: Box::into_raw(base).cast::<BaseTileLayout>(),
                extended: Box::into_raw(extended).cast::<ExtendedTileLayout>(),
                count: 4,
                assertions: true,
            },
            1,
        );
        Box::leak(Box::new(crate::services::Services {
            map: crate::map_func::fixture::statics(),
            movement: Box::leak(Box::new(crate::movement::MovementServices {
                native_assert: record_check,
                ..crate::test_tables::MOVEMENT
            })),
            ..crate::test_tables::SHARED
        }))
    }

    /// `SetCrossingBarred` stores the bar bit (m5 bit 5) in place, leaves the tile's other
    /// bits, agrees with `IsCrossingBarred`, and asserts `IsLevelCrossing(t)`.
    #[test]
    fn set_crossing_barred_stores_bit_5_and_asserts_a_level_crossing() {
        let road = crate::tile_type::TileType::MP_ROAD.0;
        let shared = map([0x40 | 0x1F, 0, 0, 0], [road, road, 0, 0]);
        let world = World::new(shared);
        world.set_crossing_barred(TileIndex(0), true);
        assert!(world.crossing_barred(0));
        assert_eq!(world.types.m5(TileIndex(0)), Some(0x40 | 0x20 | 0x1F));
        world.set_crossing_barred(TileIndex(0), false);
        assert!(!world.crossing_barred(0));
        assert_eq!(world.types.m5(TileIndex(0)), Some(0x40 | 0x1F));
        assert!(take_checks().iter().all(|&ok| ok));
        // Tile 1 is a normal road: the precondition fails once.
        world.set_crossing_barred(TileIndex(1), true);
        assert_eq!(take_checks().iter().filter(|&&ok| !ok).count(), 1);
    }

    /// `GetRailDepotTrack` is `DiagDirToDiagTrack(GetRailDepotDirection(t))`: `TRACK_X` for
    /// the north-east and south-west depots, `TRACK_Y` for the others.
    #[test]
    fn get_rail_depot_track_follows_the_depot_direction() {
        let rail = crate::tile_type::TileType::MP_RAILWAY.0;
        for (dir, track) in [(0, 0), (1, 1), (2, 0), (3, 1)] {
            let shared = map([0, 0xC0 | dir, 0, 0], [0, rail, 0, 0]);
            let world = World::new(shared);
            assert_eq!(
                world.get_rail_depot_track(TileIndex(1)),
                crate::track_type::Track(track),
                "direction {dir}"
            );
        }
        assert!(take_checks().iter().all(|&ok| ok));
    }
}
