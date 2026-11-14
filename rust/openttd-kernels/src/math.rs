/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/math_func.hpp` and `core/math_func.cpp` integer functions: interval tests, rounded
//! division, the rounded square root and width-aware `SoftClamp`.

/// `IsInsideMM`: checks if a value is in an interval. Returns true if a value is in the
/// interval of `[min, max)`.
pub(crate) const fn is_inside_mm(x: usize, min: usize, max: usize) -> bool {
    x.wrapping_sub(min) < max.wrapping_sub(min)
}

/// `IsInsideBS`: checks if a value is between a window started at some base point.
/// Returns true if `x` is in the interval of `[base, base + size)`.
pub(crate) const fn is_inside_bs(x: usize, base: usize, size: usize) -> bool {
    x.wrapping_sub(base) < size
}

/// `Delta`: returns the absolute value of (scalar) variable, for each type the original
/// instantiates (`uint` and `int`).
#[inline]
pub(crate) fn delta<T: Copy + PartialOrd + std::ops::Sub<Output = T>>(a: T, b: T) -> T {
    if a < b { b - a } else { a - b }
}

/// `RoundDivSU`: computes round(a / b) for signed a and unsigned b; the quotient, rounded to
/// nearest.
pub(crate) const fn round_div_su(a: i32, b: u32) -> i32 {
    if a > 0 {
        /* 0.5 is rounded to 1 */
        (a + b.cast_signed() / 2) / b.cast_signed()
    } else {
        /* -0.5 is rounded to 0 */
        (a - (b.cast_signed() - 1) / 2) / b.cast_signed()
    }
}

/// `Align`: return the smallest multiple of `n` equal or greater than `x`. `n` must be a
/// power of 2; the addition wraps as the original's `size_t` arithmetic does.
pub(crate) const fn align(x: usize, n: usize) -> usize {
    let n = n - 1;
    x.wrapping_add(n) & !n
}

/// `CeilDiv`: computes ceil(a / b) for non-negative a and b. The addition wraps as the
/// original's `uint` arithmetic does.
pub(crate) const fn ceil_div(a: u32, b: u32) -> u32 {
    a.wrapping_add(b).wrapping_sub(1) / b
}

/// `Clamp` (`int` overload): clamp a value between an interval. Returns `min` when the
/// value is at or below it, `max` when at or above it, otherwise the value.
pub(crate) fn clamp(a: i32, min: i32, max: i32, check: impl Fn(bool)) -> i32 {
    if min > max {
        check(false);
    }
    if a <= min {
        return min;
    }
    if a >= max {
        return max;
    }
    a
}

/// `ClampU`: clamp an unsigned integer between an interval. Returns `min` when the value
/// is at or below it, `max` when at or above it, otherwise the value.
pub(crate) fn clamp_u(a: u32, min: u32, max: u32, check: impl Fn(bool)) -> u32 {
    if min > max {
        check(false);
    }
    if a <= min {
        return min;
    }
    if a >= max {
        return max;
    }
    a
}

/// `ClampTo<uint8_t>`: clamp the given value down to lie within the `uint8_t` range.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "ClampTo's static_cast<To> of the clamped value"
)]
pub(crate) fn clamp_to_u8(a: i64) -> u8 {
    a.clamp(0, i64::from(u8::MAX)) as u8
}

/// `ClampTo<uint16_t>`: clamp the given value down to lie within the `uint16_t` range.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "ClampTo's static_cast<To> of the clamped value"
)]
pub(crate) fn clamp_to_u16(a: i64) -> u16 {
    a.clamp(0, i64::from(u16::MAX)) as u16
}

/// `ClampTo<int32_t>` of a signed 64-bit value: clamp it down to lie within the `int32_t`
/// range.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ClampTo's static_cast<To> of the clamped value"
)]
pub(crate) fn clamp_to_i32(a: i64) -> i32 {
    a.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// `ClampTo<uint32_t>` of an unsigned 64-bit value: clamp it down to lie within the
/// `uint32_t` range.
#[allow(
    clippy::cast_possible_truncation,
    reason = "ClampTo's static_cast<To> of the clamped value"
)]
pub(crate) fn clamp_to_u32(a: u64) -> u32 {
    a.min(u64::from(u32::MAX)) as u32
}

/// The value mask of a `width`-bit integer, for the template's integer types.
fn mask(width: u8) -> u64 {
    assert!(matches!(width, 1 | 8 | 16 | 32 | 64));
    u64::MAX >> (64 - width)
}

/// The two's-complement value of the low `width` bits.
fn signed_value(bits: u64, width: u8) -> i128 {
    let value = i128::from(bits & mask(width));
    if bits & (1_u64 << (width - 1)) != 0 {
        value - (1_i128 << width)
    } else {
        value
    }
}

/// `IntSqrt`: compute the integer square root of the radicand `num`. Returns the rounded
/// integer square root.
/// Algorithm taken from <http://en.wikipedia.org/wiki/Methods_of_computing_square_roots>.
pub fn int_sqrt(mut num: u32) -> u32 {
    let mut res = 0;
    // Second to top bit number.
    let mut bit = 1_u32 << 30;
    // 'bit' starts at the highest power of four <= the argument.
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
    // Arithmetic rounding to nearest integer.
    res + u32::from(num > res)
}

/// `SoftClamp`: clamp a value between an interval.
///
/// This function returns a value which is between the given interval of min and max. If the
/// given value is in this interval the value itself is returned otherwise the border of the
/// interval is returned, according which side of the interval was 'left'. If the min value
/// is greater than the max, return value is the average of the min and max.
///
/// The template's integer type is given as its `width` and signedness; values travel as
/// their bit patterns.
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

#[cfg(test)]
mod comparison_tests {
    //! Original-only `math-comparison.cpp` observations (#736 U); see the oracle header.
    use super::{int_sqrt, soft_clamp};
    use crate::oracle;

    const ORACLE: &str = include_str!("math/comparison-oracle.txt");

    /// The candidate `SoftClamp<T>` facade: `static_cast<uint64_t>` arguments, then the
    /// narrow result converted to `T` and emitted as `static_cast<uint64_t>`.
    fn soft(value: i128, min: i128, max: i128, width: u8, is_signed: bool) -> u64 {
        let bits = |v: i128| {
            v.to_le_bytes()[..8]
                .iter()
                .rev()
                .fold(0_u64, |a, &b| a << 8 | u64::from(b))
        };
        let result = soft_clamp(bits(value), bits(min), bits(max), width, is_signed);
        if is_signed && width < 64 && result >> (width - 1) & 1 != 0 {
            result | u64::MAX << width
        } else {
            result
        }
    }

    #[test]
    fn int_sqrt_and_soft_clamp_match_original() {
        let o = oracle::parse(ORACLE);
        let route = [
            ("IntSqrt(25)", u64::from(int_sqrt(25))),
            (
                "SoftClamp<int>(0, 1500, 1000)",
                soft(0, 1500, 1000, 32, true),
            ),
        ];
        oracle::check(
            &o,
            "route",
            route.map(|(i, v)| (i.to_owned(), v.to_string())),
        );

        let roots = (0..=65535_u64).flat_map(|k| {
            let square = k * k;
            [square, square + k, square + k + 1, square.saturating_sub(1)]
        });
        let roots: Vec<u64> = roots
            .map(|v| u64::from(int_sqrt(u32::try_from(v).expect("every probe value fits"))))
            .collect();
        oracle::check_rows(&o, "roots", [("k in 0..=65535".to_owned(), roots, 4)]);
        let max = u64::from(int_sqrt(u32::MAX));
        oracle::check(&o, "root-max", [("UINT32_MAX".to_owned(), max.to_string())]);

        let mut rows = Vec::new();
        for (width, is_signed) in [8, 16, 32, 64]
            .into_iter()
            .flat_map(|w| [(w, true), (w, false)])
        {
            let (low, high) = if is_signed {
                (-(1_i128 << (width - 1)), (1_i128 << (width - 1)) - 1)
            } else {
                (0, (1_i128 << width) - 1)
            };
            // T(-3), T(-1) wrap for unsigned types, exactly as the probe's casts.
            let wrap = |v: i128| {
                if is_signed {
                    v
                } else {
                    v.rem_euclid(1 << width)
                }
            };
            let bounds = [low, low + 1, -3, -1, 0, 1, 3, high - 1, high].map(wrap);
            for min in bounds {
                for max in bounds {
                    let row = bounds.map(|value| soft(value, min, max, width, is_signed));
                    let inputs = format!("width {width} signed {is_signed} min {min} max {max}");
                    rows.push((inputs, row.to_vec(), 1));
                }
            }
        }
        oracle::check_rows(&o, "soft", rows);

        // The bounded narrow signed promotion corner: every int8 max below min.
        let narrow = (-127..=127_i128).map(|min| {
            let row = (-128..min).map(|max| soft(0, min, max, 8, true)).collect();
            (format!("SoftClamp<int8_t>(0, {min}, max < min)"), row, 2)
        });
        oracle::check_rows(&o, "narrow", narrow);
    }
}

#[cfg(test)]
mod clamp_to_tests {
    /// `ClampTo<int32_t>` keeps both bounds and clamps one past each.
    #[test]
    fn clamp_to_i32_saturates_at_the_int32_bounds() {
        let min = i64::from(i32::MIN);
        let max = i64::from(i32::MAX);
        assert_eq!(super::clamp_to_i32(min), i32::MIN);
        assert_eq!(super::clamp_to_i32(max), i32::MAX);
        assert_eq!(super::clamp_to_i32(min - 1), i32::MIN);
        assert_eq!(super::clamp_to_i32(max + 1), i32::MAX);
    }
}
