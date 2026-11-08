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

/// `IsSteepSlope`.
#[must_use]
pub fn is_steep_slope(s: u8) -> bool {
    (s & SLOPE_STEEP) != 0
}

/// `IsSlopeWithOneCornerRaised`.
#[must_use]
pub fn is_slope_with_one_corner_raised(s: u8) -> bool {
    (s == SLOPE_W) || (s == SLOPE_S) || (s == SLOPE_E) || (s == SLOPE_N)
}

/// `IsHalftileSlope`.
#[must_use]
pub fn is_halftile_slope(s: u8) -> bool {
    (s & SLOPE_HALFTILE) != 0
}

/// `ComplementSlope`; callers pass no steep or halftile slope.
#[must_use]
pub fn complement_slope(s: u8) -> u8 {
    s ^ SLOPE_ELEVATED
}

/// `IsSlopeWithThreeCornersRaised`.
#[must_use]
pub fn is_slope_with_three_corners_raised(s: u8) -> bool {
    !is_halftile_slope(s)
        && !is_steep_slope(s)
        && is_slope_with_one_corner_raised(complement_slope(s))
}

/// `GetSlopeMaxZ`.
#[must_use]
pub fn get_slope_max_z(s: u8) -> i32 {
    if s == SLOPE_FLAT {
        return 0;
    }
    if is_steep_slope(s) {
        return 2;
    }
    1
}

/// `GetSlopeMaxPixelZ`.
#[must_use]
pub fn get_slope_max_pixel_z(s: u8) -> i32 {
    get_slope_max_z(s) * crate::tile_type::TILE_HEIGHT.cast_signed()
}

/// `IsInclinedSlope`.
#[must_use]
pub fn is_inclined_slope(s: u8) -> bool {
    (s == SLOPE_NW) || (s == SLOPE_SW) || (s == SLOPE_SE) || (s == SLOPE_NE)
}

/// `GetInclinedSlopeDirection`.
#[must_use]
pub fn get_inclined_slope_direction(s: u8) -> u8 {
    use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW, INVALID_DIAGDIR};
    match s {
        SLOPE_NE => DIAGDIR_NE,
        SLOPE_SE => DIAGDIR_SE,
        SLOPE_SW => DIAGDIR_SW,
        SLOPE_NW => DIAGDIR_NW,
        _ => INVALID_DIAGDIR,
    }
}

/// `RemoveHalftileSlope`.
pub const fn remove_halftile_slope(s: u8) -> u8 {
    s & !SLOPE_HALFTILE_MASK
}
/// `HasSlopeHighestCorner`.
pub fn has_slope_highest_corner(s: u8) -> bool {
    let s = remove_halftile_slope(s);
    is_steep_slope(s) || is_slope_with_one_corner_raised(s)
}
/// `FlatteningFoundation`.
pub const fn flattening_foundation(s: u8) -> u8 {
    if s == SLOPE_FLAT {
        FOUNDATION_NONE
    } else {
        FOUNDATION_LEVELED
    }
}
/// `InclinedFoundation`.
pub const fn inclined_foundation(axis: u8) -> u8 {
    if axis == 0 {
        FOUNDATION_INCLINED_X
    } else {
        FOUNDATION_INCLINED_Y
    }
}

/// `GetHighestSlopeCorner`; the caller supplies its non-returning failure operation.
pub fn get_highest_slope_corner(s: u8, not_reached: impl FnOnce()) -> u8 {
    match remove_halftile_slope(s) {
        1 | 27 => 0,
        2 | 23 => 1,
        4 | 30 => 2,
        8 | 29 => 3,
        _ => {
            not_reached();
            std::process::abort()
        }
    }
}
/// `InclinedSlope`: the slope inclined in the given direction (up when moving
/// in it); the caller supplies its non-returning failure operation.
pub fn inclined_slope(
    dir: crate::direction_type::DiagDirection,
    not_reached: impl FnOnce(),
) -> Slope {
    use crate::direction_type::DiagDirection;
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
/// `IsValidCorner`: `CORNER_W` to `CORNER_N`.
pub const fn is_valid_corner(c: u8) -> bool {
    c < 4
}
/// `SteepSlope`.
pub fn steep_slope(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    SLOPE_STEEP | slope_with_three_corners_raised(opposite_corner(c), check)
}
/// `OppositeCorner`.
pub const fn opposite_corner(c: u8) -> u8 {
    c ^ 2
}
/// `SlopeWithOneCornerRaised`.
pub fn slope_with_one_corner_raised(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    if !is_valid_corner(c) {
        check(false);
    }
    (1_u32.wrapping_shl(u32::from(c)) & 0xff) as u8
}
/// `SlopeWithThreeCornersRaised`.
pub fn slope_with_three_corners_raised(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    complement_slope(slope_with_one_corner_raised(c, check))
}
/// `HalftileFoundation`, with a valid native corner.
pub fn halftile_foundation(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    if !is_valid_corner(c) {
        check(false);
    }
    FOUNDATION_HALFTILE_W.wrapping_add(c)
}
/// `SpecialRailFoundation`, with a valid native corner.
pub fn special_rail_foundation(c: u8, check: impl Fn(bool) + Copy) -> u8 {
    if !is_valid_corner(c) {
        check(false);
    }
    FOUNDATION_RAIL_W.wrapping_add(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helpers_match_the_header_tables() {
        for s in 0..=u8::MAX {
            assert_eq!(
                is_slope_with_three_corners_raised(s),
                matches!(s, 7 | 11 | 13 | 14)
            );
            assert_eq!(is_inclined_slope(s), matches!(s, 3 | 6 | 9 | 12));
            assert_eq!(
                get_inclined_slope_direction(s) != 0xFF,
                is_inclined_slope(s)
            );
        }
        assert_eq!(
            get_slope_max_z(SLOPE_STEEP | SLOPE_S | SLOPE_E | SLOPE_N),
            2
        );
        assert_eq!(get_inclined_slope_direction(SLOPE_NE), 0);
        assert_eq!(get_inclined_slope_direction(SLOPE_NW), 3);
    }
}
