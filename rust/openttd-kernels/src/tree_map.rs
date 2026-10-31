/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `tree_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::TileTypeAccess;
use crate::tile_type::MP_TREES;
/// `TREE_COUNT_TOYLAND`.
pub(crate) const TREE_COUNT_TOYLAND: u8 = 9;
#[inline]
pub(crate) fn get_tree_ground(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_TREES)?;
    Some(((m.m2(t)? >> 6) & 7) as u8)
}
#[inline]
pub(crate) fn get_tree_density(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_TREES)?;
    Some(((m.m2(t)? >> 4) & 3) as u8)
}
#[inline]
pub(crate) fn get_tree_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_TREES)?;
    m.m3(t)
}
#[inline]
pub(crate) fn get_tree_count(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_TREES)?;
    Some((m.m5(t)? >> 6) + 1)
}
#[inline]
pub(crate) fn get_tree_growth(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_TREES)?;
    Some(m.m5(t)? & 7)
}
