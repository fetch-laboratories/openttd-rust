/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Inline helpers of `slope_func.h` and the `slope_type.h` values they use.
//! `services.cpp` asserts every value.

/// `SLOPE_FLAT`.
pub const SLOPE_FLAT: u8 = 0x00;
/// `SLOPE_W`.
pub const SLOPE_W: u8 = 0x01;
/// `SLOPE_S`.
pub const SLOPE_S: u8 = 0x02;
/// `SLOPE_E`.
pub const SLOPE_E: u8 = 0x04;
/// `SLOPE_N`.
pub const SLOPE_N: u8 = 0x08;
/// `SLOPE_STEEP`.
pub const SLOPE_STEEP: u8 = 0x10;
/// `SLOPE_NW`.
pub const SLOPE_NW: u8 = SLOPE_N | SLOPE_W;
/// `SLOPE_SW`.
pub const SLOPE_SW: u8 = SLOPE_S | SLOPE_W;
/// `SLOPE_SE`.
pub const SLOPE_SE: u8 = SLOPE_S | SLOPE_E;
/// `SLOPE_NE`.
pub const SLOPE_NE: u8 = SLOPE_N | SLOPE_E;
/// `SLOPE_EW`.
pub const SLOPE_EW: u8 = SLOPE_E | SLOPE_W;
/// `SLOPE_NS`.
pub const SLOPE_NS: u8 = SLOPE_N | SLOPE_S;
/// `SLOPE_ELEVATED`.
pub const SLOPE_ELEVATED: u8 = SLOPE_W | SLOPE_S | SLOPE_E | SLOPE_N;
/// `SLOPE_HALFTILE`.
pub const SLOPE_HALFTILE: u8 = 0x20;
/// `SLOPE_HALFTILE_MASK`.
pub const SLOPE_HALFTILE_MASK: u8 = 0xE0;

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

/// `FOUNDATION_NONE`.
pub const FOUNDATION_NONE: u8 = 0;
/// `FOUNDATION_LEVELED`.
pub const FOUNDATION_LEVELED: u8 = 1;
/// `FOUNDATION_INCLINED_X`.
pub const FOUNDATION_INCLINED_X: u8 = 2;
/// `FOUNDATION_INCLINED_Y`.
pub const FOUNDATION_INCLINED_Y: u8 = 3;

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

/// `FOUNDATION_STEEP_LOWER`.
pub const FOUNDATION_STEEP_LOWER: u8 = 4;
/// `FOUNDATION_STEEP_BOTH`.
pub const FOUNDATION_STEEP_BOTH: u8 = 5;
/// `FOUNDATION_INVALID`.
pub const FOUNDATION_INVALID: u8 = 255;
/// `GetHighestSlopeCorner`, on the native helper's reachable domain.
pub fn get_highest_slope_corner(s: u8) -> u8 {
    match remove_halftile_slope(s) {
        1 | 27 => 0,
        2 | 23 => 1,
        4 | 30 => 2,
        8 | 29 => 3,
        _ => unreachable!("native highest slope corner"),
    }
}
/// `OppositeCorner`.
pub const fn opposite_corner(c: u8) -> u8 {
    c ^ 2
}
/// `SlopeWithOneCornerRaised`.
pub const fn slope_with_one_corner_raised(c: u8) -> u8 {
    1 << c
}
/// `SlopeWithThreeCornersRaised`.
pub const fn slope_with_three_corners_raised(c: u8) -> u8 {
    slope_with_one_corner_raised(c) ^ SLOPE_ELEVATED
}
/// `HalftileFoundation`, with a valid native corner.
pub const fn halftile_foundation(c: u8) -> u8 {
    6 + c
}
/// `SpecialRailFoundation`, with a valid native corner.
pub const fn special_rail_foundation(c: u8) -> u8 {
    10 + c
}
