/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/// Scalar parse result with byte spans for the original C++ diagnostic formatter.
/// Value bits are zero-extended from the requested integer width. Failure has
/// zero matched length/value; error kind 1/2/3 means invalid/range/negative-hex.
#[repr(C)]
#[derive(Default)]
pub struct IntegerResult {
    pub(super) value_bits: u64,
    pub(super) length: usize,
    pub(super) error_offset: usize,
    pub(super) error_length: usize,
    pub(super) error_kind: u8,
}

fn digit(byte: u8, base: u8) -> Option<u64> {
    let value = match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => return None,
    };
    (value < base).then_some(u64::from(value))
}

fn hexadecimal(src: &[u8]) -> bool {
    src.starts_with(b"0x") || src.starts_with(b"0X")
}

fn failure(kind: u8, offset: usize, length: usize) -> IntegerResult {
    IntegerResult {
        error_kind: kind,
        error_offset: offset,
        error_length: length,
        ..IntegerResult::default()
    }
}

pub(super) fn parse(src: &[u8], base: u8, width: u8, signed: bool, clamp: bool) -> IntegerResult {
    let mask = u64::MAX >> (64 - width);
    let signed_min = 1_u64 << (width - 1);
    if base == 0 {
        if hexadecimal(src) {
            let mut result = parse(&src[2..], 16, width, signed, clamp);
            if result.length > 0 {
                result.length += 2;
            } else {
                result.error_offset += 2;
            }
            return result;
        }
        if signed && src.starts_with(b"-") && hexadecimal(&src[1..]) {
            // The original first parses/clamps the corresponding UNSIGNED type.
            let mut result = parse(&src[3..], 16, width, false, clamp);
            if result.length == 0 {
                result.error_offset += 3;
                return result;
            }
            result.length += 3;
            result.value_bits = 0_u64.wrapping_sub(result.value_bits) & mask;
            // Narrowing follows negation (including C++ integer promotions for
            // 8/16 bits). Only a positive narrowed signed value is rejected.
            if result.value_bits > 0 && result.value_bits < signed_min {
                if !clamp {
                    return failure(3, 0, result.length);
                }
                result.value_bits = signed_min;
            }
            return result;
        }
        return parse(src, 10, width, signed, clamp);
    }

    let negative = signed && src.starts_with(b"-");
    let start = usize::from(negative);
    let limit = if signed {
        if negative { signed_min } else { signed_min - 1 }
    } else {
        mask
    };
    let mut pos = start;
    let mut value = 0_u64;
    let mut overflow = false;
    while let Some(next) = src.get(pos).and_then(|byte| digit(*byte, base)) {
        if !overflow {
            match value
                .checked_mul(u64::from(base))
                .and_then(|v| v.checked_add(next))
            {
                Some(next_value) if next_value <= limit => value = next_value,
                _ => overflow = true,
            }
        }
        pos += 1; // Overflow still consumes the complete digit run.
    }
    if pos == start {
        return failure(1, 0, 0);
    }
    if overflow {
        if !clamp {
            return failure(2, 0, pos);
        }
        value = limit;
    }
    if negative {
        value = 0_u64.wrapping_sub(value) & mask;
    }
    IntegerResult {
        value_bits: value,
        length: pos,
        ..IntegerResult::default()
    }
}

pub(super) fn skip(src: &[u8], mut base: u8) -> usize {
    let mut pos = usize::from(src.starts_with(b"-"));
    if base == 0 {
        if hexadecimal(&src[pos..]) {
            pos += 2;
            base = 16;
        } else {
            base = 10;
        }
    }
    while src.get(pos).and_then(|byte| digit(*byte, base)).is_some() {
        pos += 1;
    }
    pos
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustIntegerResult" ["probe"] => IntegerResult {
        value_bits,
        length,
        error_offset,
        error_length,
        error_kind,
    };
}
