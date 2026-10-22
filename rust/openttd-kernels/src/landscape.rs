/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

use crate::slope_func::{get_slope_max_pixel_z, is_halftile_slope};
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE};

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

#[cfg(test)]
mod tests {
    use crate::openttd_rust_get_partial_pixel_z as height;

    #[test]
    fn direct_flat_and_elevated_gap() {
        for x in 0..16 {
            for y in 0..16 {
                assert_eq!(height(x, y, 0), 0);
                assert_eq!(height(x, y, 0x0f), 8);
            }
        }
    }

    #[test]
    fn half_tile_precedes_invalid_base_validation() {
        // Invalid steep-flat base: northern leveled half still has defined height.
        assert_eq!(height(0, 0, 0xf0), 16);
        assert_eq!(height(15, 15, 0xf0), u64::MAX);
        // Half-flat maximum is eight, even though the removed base is flat.
        assert_eq!(height(0, 0, 0xe0), 8);
        assert_eq!(height(15, 15, 0xe0), 0);
    }

    #[test]
    fn out_of_tile_coordinates_keep_unsigned_promotions_and_signed_returns() {
        assert_eq!(height(i32::MIN, i32::MAX, 0), 0);
        assert_eq!(height(-2, 0, 0x03), u64::from(u32::MAX));
        assert_eq!(height(0, -3, 0x06), u64::from(u32::MAX));
        assert_eq!(height(17, 0, 0x0c), 0x7fff_ffff);
        assert_eq!(height(0, 34, 0x0d), u64::from(u32::MAX));
        assert_eq!(height(i32::MIN, 0, 0x1b), 0x4000_0008);
        assert_eq!(height(-1, 16, 0x20), 0);
    }

    #[test]
    fn unsupported_slopes_report_fatal_sentinel() {
        assert_eq!(height(0, 0, 0x10), u64::MAX);
    }
}
