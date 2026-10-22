/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Numeric byte encoders with the original 32-byte integer formatting capacity.

/// Eight little-endian bytes returned by value; the typed C++ adapter selects its width.
#[repr(C)]
pub struct LittleEndian {
    /// Low byte first, independent of host byte order.
    pub bytes: [u8; 8],
}

/// Fixed-capacity integer text returned by value.
#[repr(C)]
#[derive(Default)]
pub struct FormattedInteger {
    /// ASCII digits and an optional minus sign; only the first `length` bytes are used.
    pub bytes: [u8; 32],
    /// Zero on capacity failure, otherwise the length including any minus sign.
    pub length: usize,
}

pub fn little_endian(value: u64) -> LittleEndian {
    LittleEndian {
        bytes: value.to_le_bytes(),
    }
}

pub fn integer(bits: u64, negative: bool, base: i32) -> FormattedInteger {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    // std::to_chars requires this range too; unsupported bases are not an API extension.
    assert!((2..=36).contains(&base));
    let base = u64::try_from(base).unwrap();
    // C++ converts signed inputs directly to uint64_t, sign-extending modulo 2^64.
    // Unsigned negation gives the magnitude even for INT64_MIN without signed overflow.
    let mut magnitude = if negative { bits.wrapping_neg() } else { bits };
    let mut result = FormattedInteger::default();
    loop {
        if result.length == result.bytes.len() {
            return FormattedInteger::default();
        }
        // The remainder is at most 35; the bounded length increment cannot overflow.
        let digit = usize::try_from(magnitude % base).unwrap();
        result.bytes[result.length] = DIGITS[digit];
        result.length += 1;
        magnitude /= base;
        if magnitude == 0 {
            break;
        }
    }
    if negative {
        if result.length == result.bytes.len() {
            return FormattedInteger::default();
        }
        result.bytes[result.length] = b'-';
        result.length += 1;
    }
    result.bytes[..result.length].reverse();
    result
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustLittleEndian" ["probe"] => LittleEndian {
        bytes,
    };
    "OpenTTDRustFormattedInteger" ["probe"] => FormattedInteger {
        bytes,
        length,
    };
}
