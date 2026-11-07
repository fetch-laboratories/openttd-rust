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
/// `TREE_GROUND_SHORE`.
pub(crate) const TREE_GROUND_SHORE: u8 = TreeGround::TREE_GROUND_SHORE.0;
#[inline]
pub(crate) fn get_tree_ground(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_TREES)?;
    Some(((fields.m2() >> 6) & 7) as u8)
}
#[inline]
pub(crate) fn get_tree_density(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_TREES)?;
    Some(((fields.m2() >> 4) & 3) as u8)
}
#[inline]
pub(crate) fn get_tree_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    m.require_type(t, MP_TREES)?;
    m.m3(t)
}
#[inline]
pub(crate) fn get_tree_count(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_TREES)?;
    Some((fields.m5() >> 6) + 1)
}
#[inline]
pub(crate) fn get_tree_growth(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_TREES)?;
    Some(fields.m5() & 7)
}

/// Native `TreeGround` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeGround(pub u8);

impl TreeGround {
    /// normal grass
    pub const TREE_GROUND_GRASS: Self = Self(0);
    /// some rough tile
    pub const TREE_GROUND_ROUGH: Self = Self(1);
    /// a desert or snow tile, depend on landscape
    pub const TREE_GROUND_SNOW_DESERT: Self = Self(2);
    /// shore
    pub const TREE_GROUND_SHORE: Self = Self(3);
    /// A snow tile that is rough underneath.
    pub const TREE_GROUND_ROUGH_SNOW: Self = Self(4);
}
