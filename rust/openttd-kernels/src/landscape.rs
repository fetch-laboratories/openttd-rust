/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

use crate::slope_func::{get_slope_max_pixel_z, is_halftile_slope};
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE};

/// Original ordered `IsMapSurroundedByWater` scan, including duplicate corners.
pub(crate) fn scan_water_edges(
    width: u32,
    height: u32,
    mut slope: impl FnMut(u32, u32) -> (u8, i32),
) -> bool {
    let mut check = |x, y| {
        let (slope, h) = slope(x, y);
        slope == crate::slope_func::SLOPE_FLAT && h == 0
    };
    if !check(0, 0) || !check(0, height) || !check(width, 0) || !check(width, height) {
        return false;
    }
    for x in 0..=width {
        if !check(x, 0) || !check(x, height) {
            return false;
        }
    }
    for y in 1..height {
        if !check(0, y) || !check(width, y) {
            return false;
        }
    }
    true
}
pub(crate) fn surrounded_by_water(s: &crate::services::Services) -> bool {
    let w = crate::map_func::World::new(s);
    scan_water_edges(w.dimensions.size_x(), w.size_y(), |x, y| {
        w.tile_pixel_slope_outside_map(x.cast_signed(), y.cast_signed())
    })
}

/// `landscape.h::ApplyPixelFoundationToSlope`, using the canonical native
/// `ApplyFoundationToSlope` leaf and preserving unsigned pixel multiplication.
#[inline]
pub(crate) fn apply_pixel_foundation_to_slope(
    services: &crate::services::Services,
    foundation: crate::services::Foundation,
    slope: &mut crate::services::Slope,
) -> u32 {
    let applied = (services.apply_foundation)(foundation, *slope);
    *slope = applied.slope;
    applied.z.cast_unsigned().wrapping_mul(TILE_HEIGHT)
}

/// Preserve C++'s unsigned `TILE_SIZE`/`TILE_HEIGHT` promotions and logical shifts.
/// Signed expressions use wrapping operations to avoid adding a panic path;
/// equivalence covers inputs whose executed signed C++ operations are defined.
// Casts implement the original unsigned promotions and return conversion;
// keeping its slope table together makes the arithmetic directly comparable.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
#[inline]
pub(super) fn partial_pixel_z(x: i32, y: i32, corners: u8) -> Option<u32> {
    let sum = x.wrapping_add(y);
    if is_halftile_slope(corners) {
        let leveled = match (corners >> 6) & 3 {
            0 => x > y,
            1 => sum >= TILE_SIZE as i32,
            2 => x <= y,
            3 => sum < TILE_SIZE as i32,
            _ => unreachable!(), // A two-bit value is always 0..=3.
        };
        if leveled {
            return Some(get_slope_max_pixel_z(corners).cast_unsigned());
        }
    }

    // Clear all half-tile bits even when SLOPE_HALFTILE was unset, after its return.
    let north = || TILE_SIZE.wrapping_sub(x as u32).wrapping_sub(y as u32) >> 1;
    let south = || (1_i32.wrapping_add(x).wrapping_add(y) as u32).wrapping_sub(TILE_SIZE) >> 1;
    let east = || (1_i32.wrapping_add(y).wrapping_sub(x) >> 1) as u32;
    let west = || (x.wrapping_sub(y) >> 1) as u32;
    Some(match corners & 0x1f {
        0x00 => 0,
        0x08 => {
            if sum <= TILE_SIZE as i32 {
                north()
            } else {
                0
            }
        }
        0x04 => {
            if y >= x {
                east()
            } else {
                0
            }
        }
        0x02 => {
            if sum >= TILE_SIZE as i32 {
                south()
            } else {
                0
            }
        }
        0x01 => {
            if x >= y {
                west()
            } else {
                0
            }
        }
        0x0c => TILE_SIZE.wrapping_sub(x as u32) >> 1,
        0x06 => (y.wrapping_add(1) >> 1) as u32,
        0x03 => (x.wrapping_add(1) >> 1) as u32,
        0x09 => TILE_SIZE.wrapping_sub(y as u32) >> 1,
        0x0d => {
            if sum >= TILE_SIZE as i32 {
                TILE_HEIGHT.wrapping_sub(south())
            } else {
                TILE_HEIGHT
            }
        }
        0x0e => {
            if y < x {
                TILE_HEIGHT.wrapping_sub(west())
            } else {
                TILE_HEIGHT
            }
        }
        0x07 => {
            if sum <= TILE_SIZE as i32 {
                TILE_HEIGHT.wrapping_sub(north())
            } else {
                TILE_HEIGHT
            }
        }
        0x0b => {
            if x < y {
                TILE_HEIGHT.wrapping_sub(east())
            } else {
                TILE_HEIGHT
            }
        }
        0x0a => {
            if sum < TILE_SIZE as i32 {
                north()
            } else {
                south()
            }
        }
        0x05 => {
            if x >= y {
                west()
            } else {
                east()
            }
        }
        0x0f => TILE_HEIGHT,
        0x1d => {
            TILE_SIZE
                .wrapping_sub(x as u32)
                .wrapping_add(TILE_SIZE)
                .wrapping_sub(y as u32)
                >> 1
        }
        0x1e => {
            TILE_SIZE
                .wrapping_add(1)
                .wrapping_add(y as u32)
                .wrapping_sub(x as u32)
                >> 1
        }
        0x17 => (1_i32.wrapping_add(x).wrapping_add(y) >> 1) as u32,
        0x1b => TILE_SIZE.wrapping_add(x as u32).wrapping_sub(y as u32) >> 1,
        _ => return None,
    })
}

/// Original `TILE_UPDATE_FREQUENCY` in landscape.cpp.
pub(crate) const TILE_UPDATE_FREQUENCY: u32 = 256;

#[cfg(test)]
mod water_edges_tests {
    #[test]
    fn visits_duplicates_in_original_order_and_stops_at_each_failure() {
        // Independent literal stream for a 2 by 3 map, including repeated corners.
        let expected = [
            (0, 0),
            (0, 3),
            (2, 0),
            (2, 3),
            (0, 0),
            (0, 3),
            (1, 0),
            (1, 3),
            (2, 0),
            (2, 3),
            (0, 1),
            (2, 1),
            (0, 2),
            (2, 2),
        ];
        for failure in 0..=expected.len() {
            for bad in [(1, 0), (0, 8)] {
                let mut observed = Vec::new();
                let success = super::scan_water_edges(2, 3, |x, y| {
                    let index = observed.len();
                    observed.push((x, y));
                    if index == failure { bad } else { (0, 0) }
                });
                assert_eq!(success, failure == expected.len());
                assert_eq!(observed, expected[..(failure + 1).min(expected.len())]);
            }
        }
    }
}
