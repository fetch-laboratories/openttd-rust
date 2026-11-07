/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `slope_type.h`.

/// Slope corners use N/E/S/W, with north at the top of the tile. Four corner
/// bits and one steep bit describe a continuous slope. Halftile slopes use
/// another flag and two corner bits for the leveled side. Only one side of a
/// steep slope can be leveled: halftile slopes span only one height level.
/// Every underlying byte remains representable.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slope(pub u8);

/// Native `Foundation` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Foundation(pub u8);

/// `SLOPE_FLAT`.
pub const SLOPE_FLAT: u8 = Slope::SLOPE_FLAT.0;
/// `SLOPE_W`.
pub const SLOPE_W: u8 = Slope::SLOPE_W.0;
/// `SLOPE_S`.
pub const SLOPE_S: u8 = Slope::SLOPE_S.0;
/// `SLOPE_E`.
pub const SLOPE_E: u8 = Slope::SLOPE_E.0;
/// `SLOPE_N`.
pub const SLOPE_N: u8 = Slope::SLOPE_N.0;
/// `SLOPE_STEEP`.
pub const SLOPE_STEEP: u8 = Slope::SLOPE_STEEP.0;
/// `SLOPE_NW`.
pub const SLOPE_NW: u8 = Slope::SLOPE_NW.0;
/// `SLOPE_SW`.
pub const SLOPE_SW: u8 = Slope::SLOPE_SW.0;
/// `SLOPE_SE`.
pub const SLOPE_SE: u8 = Slope::SLOPE_SE.0;
/// `SLOPE_NE`.
pub const SLOPE_NE: u8 = Slope::SLOPE_NE.0;
/// `SLOPE_EW`.
pub const SLOPE_EW: u8 = Slope::SLOPE_EW.0;
/// `SLOPE_WSE`.
pub const SLOPE_WSE: u8 = Slope::SLOPE_WSE.0;
/// `SLOPE_NS`.
pub const SLOPE_NS: u8 = Slope::SLOPE_NS.0;
/// `SLOPE_ELEVATED`.
pub const SLOPE_ELEVATED: u8 = Slope::SLOPE_ELEVATED.0;
/// `SLOPE_HALFTILE`.
pub const SLOPE_HALFTILE: u8 = Slope::SLOPE_HALFTILE.0;
/// `SLOPE_HALFTILE_MASK`.
pub const SLOPE_HALFTILE_MASK: u8 = Slope::SLOPE_HALFTILE_MASK.0;
/// `FOUNDATION_NONE`.
pub const FOUNDATION_NONE: u8 = Foundation::FOUNDATION_NONE.0;
/// `FOUNDATION_LEVELED`.
pub const FOUNDATION_LEVELED: u8 = Foundation::FOUNDATION_LEVELED.0;
/// `FOUNDATION_INCLINED_X`.
pub const FOUNDATION_INCLINED_X: u8 = Foundation::FOUNDATION_INCLINED_X.0;
/// `FOUNDATION_INCLINED_Y`.
pub const FOUNDATION_INCLINED_Y: u8 = Foundation::FOUNDATION_INCLINED_Y.0;
/// `FOUNDATION_STEEP_LOWER`.
pub const FOUNDATION_STEEP_LOWER: u8 = Foundation::FOUNDATION_STEEP_LOWER.0;
/// `FOUNDATION_STEEP_BOTH`.
pub const FOUNDATION_STEEP_BOTH: u8 = Foundation::FOUNDATION_STEEP_BOTH.0;
/// `FOUNDATION_INVALID`.
pub const FOUNDATION_INVALID: u8 = Foundation::FOUNDATION_INVALID.0;
/// `CORNER_W`.
pub const CORNER_W: u8 = Corner::CORNER_W.0;
/// `CORNER_S`.
pub const CORNER_S: u8 = Corner::CORNER_S.0;
/// `CORNER_E`.
pub const CORNER_E: u8 = Corner::CORNER_E.0;
/// `CORNER_N`.
pub const CORNER_N: u8 = Corner::CORNER_N.0;
/// `FOUNDATION_HALFTILE_W`.
pub const FOUNDATION_HALFTILE_W: u8 = Foundation::FOUNDATION_HALFTILE_W.0;
/// `FOUNDATION_RAIL_W`.
pub const FOUNDATION_RAIL_W: u8 = Foundation::FOUNDATION_RAIL_W.0;
/// `FOUNDATION_RAIL_N`.
pub const FOUNDATION_RAIL_N: u8 = Foundation::FOUNDATION_RAIL_N.0;

/// Native `Corner`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Corner(pub u8);

#[allow(non_upper_case_globals)]
impl Corner {
    /// Native `CORNER_W`.
    pub const CORNER_W: Self = Self(0);
    /// Native `CORNER_S`.
    pub const CORNER_S: Self = Self(1);
    /// Native `CORNER_E`.
    pub const CORNER_E: Self = Self(2);
    /// Native `CORNER_N`.
    pub const CORNER_N: Self = Self(3);
    /// Native `CORNER_END`.
    pub const CORNER_END: Self = Self(4);
    /// Native `CORNER_INVALID`.
    pub const CORNER_INVALID: Self = Self(255);
}

#[allow(non_upper_case_globals)]
impl Slope {
    /// a flat tile
    pub const SLOPE_FLAT: Self = Self(0);
    /// the west corner of the tile is raised
    pub const SLOPE_W: Self = Self(1);
    /// the south corner of the tile is raised
    pub const SLOPE_S: Self = Self(2);
    /// the east corner of the tile is raised
    pub const SLOPE_E: Self = Self(4);
    /// the north corner of the tile is raised
    pub const SLOPE_N: Self = Self(8);
    /// indicates the slope is steep
    pub const SLOPE_STEEP: Self = Self(16);
    /// north and west corner are raised
    pub const SLOPE_NW: Self = Self(9);
    /// south and west corner are raised
    pub const SLOPE_SW: Self = Self(3);
    /// south and east corner are raised
    pub const SLOPE_SE: Self = Self(6);
    /// north and east corner are raised
    pub const SLOPE_NE: Self = Self(12);
    /// east and west corner are raised
    pub const SLOPE_EW: Self = Self(5);
    /// north and south corner are raised
    pub const SLOPE_NS: Self = Self(10);
    /// bit mask containing all 'simple' slopes
    pub const SLOPE_ELEVATED: Self = Self(15);
    /// north, west and south corner are raised
    pub const SLOPE_NWS: Self = Self(11);
    /// west, south and east corner are raised
    pub const SLOPE_WSE: Self = Self(7);
    /// south, east and north corner are raised
    pub const SLOPE_SEN: Self = Self(14);
    /// east, north and west corner are raised
    pub const SLOPE_ENW: Self = Self(13);
    /// a steep slope falling to east (from west)
    pub const SLOPE_STEEP_W: Self = Self(27);
    /// a steep slope falling to north (from south)
    pub const SLOPE_STEEP_S: Self = Self(23);
    /// a steep slope falling to west (from east)
    pub const SLOPE_STEEP_E: Self = Self(30);
    /// a steep slope falling to south (from north)
    pub const SLOPE_STEEP_N: Self = Self(29);
    /// one halftile is leveled (non continuous slope)
    pub const SLOPE_HALFTILE: Self = Self(32);
    /// three bits used for halftile slopes
    pub const SLOPE_HALFTILE_MASK: Self = Self(224);
    /// the west halftile is leveled (non continuous slope)
    pub const SLOPE_HALFTILE_W: Self = Self(32);
    /// the south halftile is leveled (non continuous slope)
    pub const SLOPE_HALFTILE_S: Self = Self(96);
    /// the east halftile is leveled (non continuous slope)
    pub const SLOPE_HALFTILE_E: Self = Self(160);
    /// the north halftile is leveled (non continuous slope)
    pub const SLOPE_HALFTILE_N: Self = Self(224);
}

#[allow(non_upper_case_globals)]
impl Foundation {
    /// The tile has no foundation, the slope remains unchanged.
    pub const FOUNDATION_NONE: Self = Self(0);
    /// The tile is leveled up to a flat slope.
    pub const FOUNDATION_LEVELED: Self = Self(1);
    /// The tile has an along X-axis inclined foundation.
    pub const FOUNDATION_INCLINED_X: Self = Self(2);
    /// The tile has an along Y-axis inclined foundation.
    pub const FOUNDATION_INCLINED_Y: Self = Self(3);
    /// The tile has a steep slope. The lowest corner is raised by a foundation to allow building railroad on the lower halftile.
    pub const FOUNDATION_STEEP_LOWER: Self = Self(4);
    /// The tile has a steep slope. The lowest corner is raised by a foundation and the upper halftile is leveled.
    pub const FOUNDATION_STEEP_BOTH: Self = Self(5);
    /// Level west halftile non-continuously.
    pub const FOUNDATION_HALFTILE_W: Self = Self(6);
    /// Level south halftile non-continuously.
    pub const FOUNDATION_HALFTILE_S: Self = Self(7);
    /// Level east halftile non-continuously.
    pub const FOUNDATION_HALFTILE_E: Self = Self(8);
    /// Level north halftile non-continuously.
    pub const FOUNDATION_HALFTILE_N: Self = Self(9);
    /// Foundation for TRACK_BIT_LEFT, but not a leveled foundation.
    pub const FOUNDATION_RAIL_W: Self = Self(10);
    /// Foundation for TRACK_BIT_LOWER, but not a leveled foundation.
    pub const FOUNDATION_RAIL_S: Self = Self(11);
    /// Foundation for TRACK_BIT_RIGHT, but not a leveled foundation.
    pub const FOUNDATION_RAIL_E: Self = Self(12);
    /// Foundation for TRACK_BIT_UPPER, but not a leveled foundation.
    pub const FOUNDATION_RAIL_N: Self = Self(13);
    /// Used inside "rail_cmd.cpp" to indicate invalid slope/track combination.
    pub const FOUNDATION_INVALID: Self = Self(255);
}

crate::base_bitset_type::impl_bitset_ops!(Slope);
