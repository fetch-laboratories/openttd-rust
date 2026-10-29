/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `road_map.h` values.
use crate::services::Roadside;
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
pub fn set_road_type_road(s: &crate::services::Services, tile: u32, rt: u8) {
    (s.set_road_type)(tile, 0, rt);
}
/// `SetRoadTypeTram` delegates to the canonical typed map setter.
pub fn set_road_type_tram(s: &crate::services::Services, tile: u32, rt: u8) {
    (s.set_road_type)(tile, 1, rt);
}

/// `Roadside::GrassRoadWorks`.
pub const ROADSIDE_GRASS_ROAD_WORKS: Roadside = Roadside(6);

/// `GetRoadTileType`, retaining its `IsTileType` assertion.
#[inline]
pub(crate) fn get_road_tile_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::services::RoadTileKind> {
    m.require_type(tile, crate::tile_type::MP_ROAD)?;
    Some(crate::services::RoadTileKind(m.m5(tile)? >> 6))
}
/// `IsNormalRoad`.
#[inline]
pub(crate) fn is_normal_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<bool> {
    Some(get_road_tile_type(m, tile)?.0 == ROAD_TILE_NORMAL)
}
#[inline]
fn require_normal_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<()> {
    m.available(tile)?;
    if m.assertions() && !is_normal_road(m, tile)? {
        return None;
    }
    Some(())
}
/// `GetRoadBits`.
#[inline]
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
#[inline]
pub(crate) fn get_disallowed_road_directions(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<u8> {
    require_normal_road(m, tile)?;
    Some((m.m5(tile)? >> 4) & 3)
}
/// `GetRoadside`.
#[inline]
pub(crate) fn get_roadside(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<Roadside> {
    Some(Roadside((m.m6(tile)? >> 3) & 7))
}
/// `HasRoadWorks`.
#[inline]
pub(crate) fn has_road_works(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<bool> {
    Some(get_roadside(m, tile)?.0 >= ROADSIDE_GRASS_ROAD_WORKS.0)
}
#[inline]
fn require_may_have_road(
    m: &crate::map_func::TileTypeAccess,
    s: &crate::services::Services,
    tile: u32,
) -> Option<()> {
    m.available(tile)?;
    if m.assertions() && !(s.may_have_road)(tile) {
        return None;
    }
    Some(())
}
/// `GetRoadTypeRoad`, with the canonical `MayHaveRoad` precondition.
#[inline]
pub(crate) fn get_road_type_road(
    m: &crate::map_func::TileTypeAccess,
    s: &crate::services::Services,
    tile: u32,
) -> Option<crate::services::RoadType> {
    require_may_have_road(m, s, tile)?;
    Some(crate::services::RoadType(m.m4(tile)? & 63))
}
/// `GetRoadTypeTram`, with the canonical `MayHaveRoad` precondition.
#[inline]
pub(crate) fn get_road_type_tram(
    m: &crate::map_func::TileTypeAccess,
    s: &crate::services::Services,
    tile: u32,
) -> Option<crate::services::RoadType> {
    require_may_have_road(m, s, tile)?;
    Some(crate::services::RoadType(
        u8::try_from((m.m8(tile)? >> 6) & 63).expect("six-bit road type"),
    ))
}
/// `GetRoadType`.
#[inline]
pub(crate) fn get_road_type(
    m: &crate::map_func::TileTypeAccess,
    s: &crate::services::Services,
    tile: u32,
    kind: crate::services::RoadTramType,
) -> Option<crate::services::RoadType> {
    if kind.0 == crate::road_type::RTT_TRAM.0 {
        get_road_type_tram(m, s, tile)
    } else {
        get_road_type_road(m, s, tile)
    }
}
/// `HasTileRoadType`.
#[inline]
pub(crate) fn has_tile_road_type(
    m: &crate::map_func::TileTypeAccess,
    s: &crate::services::Services,
    tile: u32,
    kind: crate::services::RoadTramType,
) -> Option<bool> {
    Some(get_road_type(m, s, tile, kind)?.0 != crate::road_type::INVALID_ROADTYPE)
}

impl crate::map_func::World<'_> {
    pub(crate) fn normal_road(&self, t: u32) -> bool {
        self.kind(t) == crate::tile_type::MP_ROAD
            && self.byte(self.types.m5(t)) >> 6 == ROAD_TILE_NORMAL
    }
    pub(crate) fn road_bits(&self, t: u32, tram: bool) -> u8 {
        self.check(self.normal_road(t));
        self.byte(if tram {
            self.types.m3(t)
        } else {
            self.types.m5(t)
        }) & 15
    }
    pub(crate) fn road_depot(&self, t: u32) -> bool {
        self.kind(t) == crate::tile_type::MP_ROAD
            && self.byte(self.types.m5(t)) >> 6 == ROAD_TILE_DEPOT
    }
    pub(crate) fn road_depot_dir(&self, t: u32) -> u8 {
        self.check(self.road_depot(t));
        self.byte(self.types.m5(t)) & 3
    }
    pub(crate) fn road_type(&self, t: u32, tram: bool) -> u8 {
        self.check(matches!(
            self.kind(t),
            crate::tile_type::MP_ROAD
                | crate::tile_type::MP_STATION
                | crate::tile_type::MP_TUNNELBRIDGE
        ));
        if tram {
            ((self.word(self.types.m8(t)) >> 6) & 63) as u8
        } else {
            self.byte(self.types.m4(t)) & 63
        }
    }
}
