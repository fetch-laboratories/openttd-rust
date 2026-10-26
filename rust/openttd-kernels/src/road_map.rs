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
