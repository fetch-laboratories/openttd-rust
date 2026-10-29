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
/// `MayHaveRoad`, retaining the source tile dispatch and named predicates.
#[inline]
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
#[inline]
fn require_may_have_road(m: &crate::map_func::TileTypeAccess, tile: u32) -> Option<()> {
    m.available(tile)?;
    if m.assertions() && !may_have_road(m, TileIndex(tile))? {
        return None;
    }
    Some(())
}
/// `GetRoadTypeRoad`, with the canonical `MayHaveRoad` precondition.
#[inline]
pub(crate) fn get_road_type_road(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::services::RoadType> {
    require_may_have_road(m, tile)?;
    Some(crate::services::RoadType(m.m4(tile)? & 63))
}
/// `GetRoadTypeTram`, with the canonical `MayHaveRoad` precondition.
#[inline]
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
#[inline]
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
#[inline]
pub(crate) fn has_tile_road_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    kind: crate::services::RoadTramType,
) -> Option<bool> {
    Some(get_road_type(m, tile, kind)?.0 != crate::road_type::INVALID_ROADTYPE)
}
