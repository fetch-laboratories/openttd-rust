/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `tree_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::TileTypeAccess;
use crate::tile_type::{MP_TREES, TileIndex};

/// Native `TreeType`, retaining every underlying `u8` value.
///
/// List of tree types along all landscape types.
///
/// This enumeration contains a list of the different tree types along
/// all landscape types. The values for the enumerations may be used for
/// offsets from the grfs files. These points to the start of
/// the tree list for a landscape. See the `TREE_COUNT_*` enumerations
/// for the amount of different trees for a specific landscape.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeType(pub u8);

/// Native `TreeGrowthStage`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeGrowthStage(pub u8);

/// `TREE_COUNT_TOYLAND`: number of tree types on a toyland map.
pub(crate) const TREE_COUNT_TOYLAND: u8 = 9;
/// `TREE_GROUND_SHORE`.
pub(crate) const TREE_GROUND_SHORE: u8 = TreeGround::TREE_GROUND_SHORE.0;

/// `GetTreeType`.
#[inline]
pub(crate) fn get_tree_type(m: &TileTypeAccess, t: TileIndex) -> Option<TreeType> {
    m.require_type(t.0, MP_TREES)?;
    m.m3(t.0).map(TreeType)
}
/// `GetTreeGround`.
#[inline]
pub(crate) fn get_tree_ground(m: &TileTypeAccess, t: TileIndex) -> Option<TreeGround> {
    let fields = m.require_type(t.0, MP_TREES)?;
    Some(TreeGround(((fields.m2() >> 6) & 7) as u8))
}
/// `GetTreeDensity`.
#[inline]
pub(crate) fn get_tree_density(m: &TileTypeAccess, t: TileIndex) -> Option<u8> {
    let fields = m.require_type(t.0, MP_TREES)?;
    Some(((fields.m2() >> 4) & 3) as u8)
}
/// `GetTreeCount`.
#[inline]
pub(crate) fn get_tree_count(m: &TileTypeAccess, t: TileIndex) -> Option<u8> {
    let fields = m.require_type(t.0, MP_TREES)?;
    Some((fields.m5() >> 6) + 1)
}
/// `GetTreeGrowth`.
#[inline]
pub(crate) fn get_tree_growth(m: &TileTypeAccess, t: TileIndex) -> Option<TreeGrowthStage> {
    let fields = m.require_type(t.0, MP_TREES)?;
    Some(TreeGrowthStage(fields.m5() & 7))
}

/// Raw adapter: `GetTreeType` over base values, for files no pass has finished yet.
#[inline]
pub(crate) fn get_tree_type_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_tree_type(m, TileIndex(t)).map(|v| v.0)
}
/// Raw adapter: `GetTreeGround` over base values.
#[inline]
pub(crate) fn get_tree_ground_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_tree_ground(m, TileIndex(t)).map(|v| v.0)
}
/// Raw adapter: `GetTreeDensity` over a base tile.
#[inline]
pub(crate) fn get_tree_density_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_tree_density(m, TileIndex(t))
}
/// Raw adapter: `GetTreeCount` over a base tile.
#[inline]
pub(crate) fn get_tree_count_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_tree_count(m, TileIndex(t))
}
/// Raw adapter: `GetTreeGrowth` over base values.
#[inline]
pub(crate) fn get_tree_growth_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_tree_growth(m, TileIndex(t)).map(|v| v.0)
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
