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
