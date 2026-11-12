/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `slope_func.h`.
//! `services.cpp` asserts every value.

/// Compatibility constants from the original type header.
pub use crate::slope_type::*;

use crate::direction_type::{Axis, DiagDirection};

/// `IsValidCorner`: rangecheck for `Corner` enumeration.
#[must_use]
pub const fn is_valid_corner(corner: Corner) -> bool {
    corner.0 < Corner::CORNER_END.0
}
/// `IsSteepSlope`: checks if a slope is steep.
#[must_use]
pub fn is_steep_slope(s: Slope) -> bool {
    (s & Slope::SLOPE_STEEP) != Slope::SLOPE_FLAT
}
/// `IsHalftileSlope`: checks for non-continuous slope on halftile foundations.
#[must_use]
pub fn is_halftile_slope(s: Slope) -> bool {
    (s & Slope::SLOPE_HALFTILE) != Slope::SLOPE_FLAT
}
/// `RemoveHalftileSlope`: removes a halftile slope from a slope.
/// Non-halftile slopes remain unmodified.
#[must_use]
pub const fn remove_halftile_slope(s: Slope) -> Slope {
    Slope(s.0 & !Slope::SLOPE_HALFTILE_MASK.0)
}
/// `ComplementSlope`: return the complement of a slope.
///
/// This method returns the complement of a slope. The complement of a
/// slope is a slope with raised corner which aren't raised in the given
/// slope. The slope must be neither steep nor a halftile slope.
pub fn complement_slope(s: Slope, check: impl Fn(bool)) -> Slope {
    if is_steep_slope(s) || is_halftile_slope(s) {
        check(false);
    }
    s ^ Slope::SLOPE_ELEVATED
}
/// `IsSlopeWithOneCornerRaised`: tests if a specific slope has exactly one corner raised.
#[must_use]
pub fn is_slope_with_one_corner_raised(s: Slope) -> bool {
    (s == Slope::SLOPE_W) || (s == Slope::SLOPE_S) || (s == Slope::SLOPE_E) || (s == Slope::SLOPE_N)
}
/// `SlopeWithOneCornerRaised`: returns the slope with a specific corner raised.
pub fn slope_with_one_corner_raised(corner: Corner, check: impl Fn(bool)) -> Slope {
    if !is_valid_corner(corner) {
        check(false);
    }
    Slope((1_u32.wrapping_shl(u32::from(corner.0)) & 0xff) as u8)
}
/// `HasSlopeHighestCorner`: tests if a slope has a highest corner (i.e. one
/// corner raised or a steep slope). Note: A halftile slope is ignored.
#[must_use]
pub fn has_slope_highest_corner(s: Slope) -> bool {
    let s = remove_halftile_slope(s);
    is_steep_slope(s) || is_slope_with_one_corner_raised(s)
}
/// `GetHighestSlopeCorner`: returns the highest corner of a slope (one corner
/// raised or a steep slope). A halftile slope is ignored. The caller supplies
/// its non-returning failure operation.
pub fn get_highest_slope_corner(s: Slope, not_reached: impl FnOnce()) -> Corner {
    match remove_halftile_slope(s) {
        Slope::SLOPE_W | Slope::SLOPE_STEEP_W => Corner::CORNER_W,
        Slope::SLOPE_S | Slope::SLOPE_STEEP_S => Corner::CORNER_S,
        Slope::SLOPE_E | Slope::SLOPE_STEEP_E => Corner::CORNER_E,
        Slope::SLOPE_N | Slope::SLOPE_STEEP_N => Corner::CORNER_N,
        _ => {
            not_reached();
            std::process::abort()
        }
    }
}
/// `GetSlopeMaxZ`: returns the height of the highest corner of a slope relative
/// to `TileZ` (= minimal height).
#[must_use]
pub fn get_slope_max_z(s: Slope) -> i32 {
    if s == Slope::SLOPE_FLAT {
        return 0;
    }
    if is_steep_slope(s) {
        return 2;
    }
    1
}
/// `GetSlopeMaxPixelZ`: returns the height of the highest corner of a slope
/// relative to `TileZ` (= minimal height), in pixels.
#[must_use]
pub fn get_slope_max_pixel_z(s: Slope) -> i32 {
    get_slope_max_z(s) * crate::tile_type::TILE_HEIGHT.cast_signed()
}
/// `OppositeCorner`: returns the opposite corner.
#[must_use]
pub const fn opposite_corner(corner: Corner) -> Corner {
    Corner(corner.0 ^ 2)
}
/// `IsSlopeWithThreeCornersRaised`: tests if a specific slope has exactly three
/// corners raised.
#[must_use]
pub fn is_slope_with_three_corners_raised(s: Slope) -> bool {
    !is_halftile_slope(s)
        && !is_steep_slope(s)
        && is_slope_with_one_corner_raised(complement_slope(s, |ok| {
            crate::services_assert::check(ok);
        }))
}
/// `SlopeWithThreeCornersRaised`: returns the slope with all except one corner raised.
pub fn slope_with_three_corners_raised(corner: Corner, check: impl Fn(bool)) -> Slope {
    let s = slope_with_one_corner_raised(corner, &check);
    complement_slope(s, check)
}
/// `SteepSlope`: returns a specific steep slope.
pub fn steep_slope(corner: Corner, check: impl Fn(bool)) -> Slope {
    Slope::SLOPE_STEEP | slope_with_three_corners_raised(opposite_corner(corner), check)
}
/// `IsInclinedSlope`: tests if a specific slope is an inclined slope.
#[must_use]
pub fn is_inclined_slope(s: Slope) -> bool {
    (s == Slope::SLOPE_NW)
        || (s == Slope::SLOPE_SW)
        || (s == Slope::SLOPE_SE)
        || (s == Slope::SLOPE_NE)
}
/// `GetInclinedSlopeDirection`: returns the direction of an inclined slope, or
/// `INVALID_DIAGDIR` for any other slope.
#[must_use]
pub fn get_inclined_slope_direction(s: Slope) -> DiagDirection {
    match s {
        Slope::SLOPE_NE => DiagDirection::DIAGDIR_NE,
        Slope::SLOPE_SE => DiagDirection::DIAGDIR_SE,
        Slope::SLOPE_SW => DiagDirection::DIAGDIR_SW,
        Slope::SLOPE_NW => DiagDirection::DIAGDIR_NW,
        _ => DiagDirection::INVALID_DIAGDIR,
    }
}
/// `InclinedSlope`: the slope inclined in the given direction (up when moving
/// in it); the caller supplies its non-returning failure operation.
pub fn inclined_slope(dir: DiagDirection, not_reached: impl FnOnce()) -> Slope {
    match dir {
        DiagDirection::DIAGDIR_NE => Slope::SLOPE_NE,
        DiagDirection::DIAGDIR_SE => Slope::SLOPE_SE,
        DiagDirection::DIAGDIR_SW => Slope::SLOPE_SW,
        DiagDirection::DIAGDIR_NW => Slope::SLOPE_NW,
        _ => {
            not_reached();
            std::process::abort()
        }
    }
}
/// `FlatteningFoundation`: returns the foundation needed to flatten a slope.
/// The returned foundation is either `FOUNDATION_NONE` if the tile was already
/// flat, or `FOUNDATION_LEVELED`.
#[must_use]
pub fn flattening_foundation(s: Slope) -> Foundation {
    if s == Slope::SLOPE_FLAT {
        Foundation::FOUNDATION_NONE
    } else {
        Foundation::FOUNDATION_LEVELED
    }
}
/// `InclinedFoundation`: returns the along a specific axis inclined foundation.
#[must_use]
pub fn inclined_foundation(axis: Axis) -> Foundation {
    if axis == Axis::AXIS_X {
        Foundation::FOUNDATION_INCLINED_X
    } else {
        Foundation::FOUNDATION_INCLINED_Y
    }
}
/// `HalftileFoundation`: returns the halftile foundation for single
/// horizontal/vertical track.
pub fn halftile_foundation(corner: Corner, check: impl Fn(bool)) -> Foundation {
    if !is_valid_corner(corner) {
        check(false);
    }
    Foundation(Foundation::FOUNDATION_HALFTILE_W.0.wrapping_add(corner.0))
}
/// `SpecialRailFoundation`: returns the special rail foundation for single
/// horizontal/vertical track.
pub fn special_rail_foundation(corner: Corner, check: impl Fn(bool)) -> Foundation {
    if !is_valid_corner(corner) {
        check(false);
    }
    Foundation(Foundation::FOUNDATION_RAIL_W.0.wrapping_add(corner.0))
}

/// Raw adapter: `IsSteepSlope` over a base value, for files no pass has finished yet.
#[must_use]
pub fn is_steep_slope_raw(s: u8) -> bool {
    is_steep_slope(Slope(s))
}
/// Raw adapter: `IsSlopeWithOneCornerRaised`.
#[must_use]
pub fn is_slope_with_one_corner_raised_raw(s: u8) -> bool {
    is_slope_with_one_corner_raised(Slope(s))
}
/// Raw adapter: `IsHalftileSlope`.
#[must_use]
pub fn is_halftile_slope_raw(s: u8) -> bool {
    is_halftile_slope(Slope(s))
}
/// Raw adapter: `ComplementSlope`.
pub fn complement_slope_raw(s: u8, check: impl Fn(bool)) -> u8 {
    complement_slope(Slope(s), check).0
}
/// Raw adapter: `IsSlopeWithThreeCornersRaised`.
#[must_use]
pub fn is_slope_with_three_corners_raised_raw(s: u8) -> bool {
    is_slope_with_three_corners_raised(Slope(s))
}
/// Raw adapter: `GetSlopeMaxZ`.
#[must_use]
pub fn get_slope_max_z_raw(s: u8) -> i32 {
    get_slope_max_z(Slope(s))
}
/// Raw adapter: `GetSlopeMaxPixelZ`.
#[must_use]
pub fn get_slope_max_pixel_z_raw(s: u8) -> i32 {
    get_slope_max_pixel_z(Slope(s))
}
/// Raw adapter: `IsInclinedSlope`.
#[must_use]
pub fn is_inclined_slope_raw(s: u8) -> bool {
    is_inclined_slope(Slope(s))
}
/// Raw adapter: `GetInclinedSlopeDirection`.
#[must_use]
pub fn get_inclined_slope_direction_raw(s: u8) -> u8 {
    get_inclined_slope_direction(Slope(s)).0
}
/// Raw adapter: `HasSlopeHighestCorner`.
#[must_use]
pub fn has_slope_highest_corner_raw(s: u8) -> bool {
    has_slope_highest_corner(Slope(s))
}
/// Raw adapter: `FlatteningFoundation`.
#[must_use]
pub fn flattening_foundation_raw(s: u8) -> u8 {
    flattening_foundation(Slope(s)).0
}
/// Raw adapter: `InclinedFoundation`.
#[must_use]
pub fn inclined_foundation_raw(axis: u8) -> u8 {
    inclined_foundation(Axis(axis)).0
}
/// Raw adapter: `GetHighestSlopeCorner`.
pub fn get_highest_slope_corner_raw(s: u8, not_reached: impl FnOnce()) -> u8 {
    get_highest_slope_corner(Slope(s), not_reached).0
}
/// Raw adapter: `IsValidCorner`.
#[must_use]
pub const fn is_valid_corner_raw(c: u8) -> bool {
    is_valid_corner(Corner(c))
}
/// Raw adapter: `SteepSlope`.
pub fn steep_slope_raw(c: u8, check: impl Fn(bool)) -> u8 {
    steep_slope(Corner(c), check).0
}
/// Raw adapter: `SlopeWithOneCornerRaised`.
pub fn slope_with_one_corner_raised_raw(c: u8, check: impl Fn(bool)) -> u8 {
    slope_with_one_corner_raised(Corner(c), check).0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helpers_match_the_header_tables() {
        for s in 0..=u8::MAX {
            let slope = Slope(s);
            assert_eq!(
                is_slope_with_three_corners_raised(slope),
                matches!(s, 7 | 11 | 13 | 14)
            );
            assert_eq!(is_inclined_slope(slope), matches!(s, 3 | 6 | 9 | 12));
            assert_eq!(
                get_inclined_slope_direction(slope) != DiagDirection::INVALID_DIAGDIR,
                is_inclined_slope(slope)
            );
        }
        assert_eq!(
            get_slope_max_z(Slope::SLOPE_STEEP | Slope::SLOPE_S | Slope::SLOPE_E | Slope::SLOPE_N),
            2
        );
        assert_eq!(
            get_inclined_slope_direction(Slope::SLOPE_NE),
            DiagDirection::DIAGDIR_NE
        );
        assert_eq!(
            get_inclined_slope_direction(Slope::SLOPE_NW),
            DiagDirection::DIAGDIR_NW
        );
    }
}
