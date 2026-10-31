/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `road_map.h` values.
use crate::services::Roadside;
use crate::tile_type::TileIndex;
/// `Roadside::Trees`.
pub const ROADSIDE_TREES: Roadside = Roadside(5);
/// `Roadside::Paved`.
pub const ROADSIDE_PAVED: Roadside = Roadside(2);

/// `RoadTileType::Normal`.
pub const ROAD_TILE_NORMAL: u8 = 0;
/// `RoadTileType::Crossing`.
pub const ROAD_TILE_CROSSING: u8 = 1;
/// `RoadTileType::Depot`.
pub const ROAD_TILE_DEPOT: u8 = 2;

/// `SetRoadTypeRoad` delegates to the canonical typed map setter.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn set_road_type_road(s: &crate::services::Services, tile: u32, rt: u8) {
    (s.set_road_type)(tile, 0, rt);
}
/// `SetRoadTypeTram` delegates to the canonical typed map setter.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn set_road_type_tram(s: &crate::services::Services, tile: u32, rt: u8) {
    (s.set_road_type)(tile, 1, rt);
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
    m.require_type(tile, crate::tile_type::MP_ROAD)?;
    Some(crate::services::RoadTileKind(m.m5(tile)? >> 6))
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
    m.available(tile)?;
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
            m.m3(tile)?
        } else {
            m.m5(tile)?
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
    Some((m.m5(tile)? >> 4) & 3)
}
/// `GetRoadside`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_roadside(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<Roadside> {
    Some(Roadside((m.m6(tile)? >> 3) & 7))
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
    use crate::tile_type::{MP_ROAD, MP_STATION, MP_TUNNELBRIDGE};
    Some(match crate::tile_map::get_tile_type(m, tile.0)? {
        MP_ROAD => true,
        MP_STATION => crate::station_map::is_any_road_stop(m, tile)?,
        MP_TUNNELBRIDGE => {
            crate::tunnelbridge_map::get_tunnel_bridge_transport_type(m, tile)?.0
                == crate::transport_type::TRANSPORT_ROAD
        }
        _ => false,
    })
}
#[allow(clippy::inline_always)]
#[inline(always)]
fn require_may_have_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<()> {
    m.available(tile)?;
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
    Some(crate::services::RoadType(m.m4(tile)? & 63))
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
        u8::try_from((m.m8(tile)? >> 6) & 63).expect("six-bit road type"),
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
        self.asserted(|m| crate::tile_map::get_tile_type(m, t)) == crate::tile_type::MP_ROAD
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
    pub(crate) fn road_depot(&self, t: u32) -> bool {
        self.kind(t) == crate::tile_type::MP_ROAD
            && get_road_tile_type(&self.types, t).is_some_and(|kind| kind.0 == ROAD_TILE_DEPOT)
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn road_depot_dir(&self, t: u32) -> u8 {
        self.check(self.road_depot(t));
        crate::bitmath_func::gb(u32::from(Self::byte(self.types.m5(t))), 0, 2) as u8
    }
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn road_type(&self, t: u32, tram: bool) -> u8 {
        self.asserted(|m| get_road_type(m, t, crate::services::RoadTramType(tram)))
            .0
    }
}
