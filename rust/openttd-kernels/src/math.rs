/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Rounded square root and width-aware integer saturation.

/// Original `SB` for a 32-bit word: clear the window, OR the unmasked data.
/// The original preconditions are `n < 32` and `s + n <= 32`.
pub(crate) fn sb(value: &mut u32, s: u8, n: u8, data: u32) {
    *value &= !(((1_u32 << n) - 1) << s);
    *value |= data << s;
}

fn mask(width: u8) -> u64 {
    assert!(matches!(width, 1 | 8 | 16 | 32 | 64));
    u64::MAX >> (64 - width)
}

fn signed_value(bits: u64, width: u8) -> i128 {
    let value = i128::from(bits & mask(width));
    if bits & (1_u64 << (width - 1)) != 0 {
        value - (1_i128 << width)
    } else {
        value
    }
}

pub fn int_sqrt(mut num: u32) -> u32 {
    let mut res = 0;
    let mut bit = 1_u32 << 30;
    while bit > num {
        bit >>= 2;
    }
    while bit != 0 {
        if num >= res + bit {
            num -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    res + u32::from(num > res)
}

pub fn soft_clamp(value: u64, min: u64, max: u64, width: u8, is_signed: bool) -> u64 {
    assert!(matches!(width, 8 | 16 | 32 | 64));
    let narrow_mask = mask(width);
    let min_bits = min & narrow_mask;
    let max_bits = max & narrow_mask;
    let interpret = |bits| {
        if is_signed {
            signed_value(bits, width)
        } else {
            i128::from(bits & narrow_mask)
        }
    };
    let low = interpret(min);
    let high = interpret(max);
    if low > high {
        if is_signed && width < 32 {
            // U(min) and signed max both promote to int. This subtraction is
            // NOT a narrow unsigned subtraction, even when min was negative.
            let distance = i128::from(min_bits) - high;
            let result = low - distance / 2;
            return u64::try_from(result.rem_euclid(1_i128 << width))
                .expect("bounded promoted result");
        }
        let distance = min_bits.wrapping_sub(max_bits) & narrow_mask;
        return min_bits.wrapping_sub(distance / 2) & narrow_mask;
    }
    let current = interpret(value);
    if current <= low {
        min_bits
    } else if current >= high {
        max_bits
    } else {
        value & narrow_mask
    }
}
