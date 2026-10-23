/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `rail_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::TileTypeAccess;

/// `GetRailGroundType` has no tile-type precondition in the original.
#[inline]
pub(crate) fn get_rail_ground_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    Some(m.m4(t)? & 15)
}

/// `RailTileType::Normal`, asserted by native compilation.
pub const RAIL_TILE_NORMAL: u8 = 0;
/// `IsPbsSignal`, from `rail_map.h`.
pub const fn is_pbs_signal(kind: u8) -> bool {
    kind == crate::signal_type::SIGTYPE_PBS || kind == crate::signal_type::SIGTYPE_PBS_ONEWAY
}

/// `IsRailDepotTile`, retaining the guarded `IsRailDepot` operation.
pub(crate) fn is_rail_depot_tile(
    shared: &crate::services::Services,
    rail: &crate::services::RailMapServices,
    tile: u32,
) -> bool {
    (shared.tile_type)(tile) == crate::tile_type::MP_RAILWAY && (rail.is_rail_depot)(tile)
}
/// `RailGroundType::HalfTileWater`.
pub const RAIL_GROUND_HALF_TILE_WATER: u8 = 13;
/// `RailTileType::Signals`.
pub const RAIL_TILE_SIGNALS: u8 = 1;
/// `RailTileType::Depot`.
pub const RAIL_TILE_DEPOT: u8 = 3;
