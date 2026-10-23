/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Historical byte codec and iterator positions; surrogate codepoints are valid.

/// By-value UTF-8 encoding result. Unused bytes are zero, including on failure.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct Encoded {
    /// Four bytes of storage, of which only `length` bytes are part of the encoding.
    pub bytes: [u8; 4],
    /// Encoded length, or zero for values at or above U+110000.
    pub length: usize,
}

/// By-value first-sequence decoding result; both fields are zero on failure.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct Decoded {
    /// Number of bytes in the first sequence.
    pub length: usize,
    /// Original 32-bit codepoint, including surrogates.
    pub codepoint: u32,
}

pub fn is_part(byte: u8) -> bool {
    byte >> 6 == 2
}

pub fn encode(codepoint: u32) -> Encoded {
    // Every conversion is bounded by the branch threshold or an explicit mask.
    let mut result = Encoded {
        bytes: [0; 4],
        length: 0,
    };
    let part = |shift: u32| 0x80 + u8::try_from((codepoint >> shift) & 0x3f).unwrap();
    if codepoint < 0x80 {
        result.bytes[0] = u8::try_from(codepoint).unwrap();
        result.length = 1;
    } else if codepoint < 0x800 {
        result.bytes[0] = 0xc0 + u8::try_from(codepoint >> 6).unwrap();
        result.bytes[1] = part(0);
        result.length = 2;
    } else if codepoint < 0x10000 {
        result.bytes[0] = 0xe0 + u8::try_from(codepoint >> 12).unwrap();
        result.bytes[1] = part(6);
        result.bytes[2] = part(0);
        result.length = 3;
    } else if codepoint < 0x0011_0000 {
        result.bytes[0] = 0xf0 + u8::try_from(codepoint >> 18).unwrap();
        result.bytes[1] = part(12);
        result.bytes[2] = part(6);
        result.bytes[3] = part(0);
        result.length = 4;
    }
    result
}

pub fn decode(bytes: &[u8]) -> Decoded {
    let valid = |length, codepoint| Decoded { length, codepoint };
    if !bytes.is_empty() && bytes[0] & 0x80 == 0 {
        return valid(1, u32::from(bytes[0]));
    } else if bytes.len() >= 2 && bytes[0] >> 5 == 6 {
        if is_part(bytes[1]) {
            let codepoint = u32::from(bytes[0] & 0x1f) << 6 | u32::from(bytes[1] & 0x3f);
            if codepoint >= 0x80 {
                return valid(2, codepoint);
            }
        }
    } else if bytes.len() >= 3 && bytes[0] >> 4 == 14 {
        if is_part(bytes[1]) && is_part(bytes[2]) {
            let codepoint = u32::from(bytes[0] & 0x0f) << 12
                | u32::from(bytes[1] & 0x3f) << 6
                | u32::from(bytes[2] & 0x3f);
            if codepoint >= 0x800 {
                return valid(3, codepoint);
            }
        }
    } else if bytes.len() >= 4
        && bytes[0] >> 3 == 30
        && is_part(bytes[1])
        && is_part(bytes[2])
        && is_part(bytes[3])
    {
        let codepoint = u32::from(bytes[0] & 0x07) << 18
            | u32::from(bytes[1] & 0x3f) << 12
            | u32::from(bytes[2] & 0x3f) << 6
            | u32::from(bytes[3] & 0x3f);
        if (0x10000..=0x0010_ffff).contains(&codepoint) {
            return valid(4, codepoint);
        }
    }
    valid(0, 0)
}

pub fn next(bytes: &[u8], position: usize) -> usize {
    // The C++ facade asserts position < length, so this increment cannot overflow.
    let mut position = position + 1;
    while position < bytes.len() && is_part(bytes[position]) {
        position += 1;
    }
    position
}

pub fn previous(bytes: &[u8], position: usize) -> usize {
    // The C++ facade asserts position > 0; its iterator has position <= length.
    let mut position = position - 1;
    while position > 0 && is_part(bytes[position]) {
        position -= 1;
    }
    position
}

pub fn at_byte(bytes: &[u8], offset: usize) -> usize {
    if offset >= bytes.len() {
        return bytes.len();
    }
    // offset < length bounds offset + 1. This intentionally scans malformed runs.
    previous(bytes, offset + 1)
}

/// Borrow input for one ABI call, accepting null only for an empty view.
///
/// # Safety
/// Nonempty input must address `length` readable initialized bytes in one allocation,
/// remain valid and unmodified for this borrow, and have length at most `isize::MAX`.
/// Read-only overlapping inputs are allowed. Empty input creates no raw-pointer slice.
#[allow(unsafe_code)]
pub unsafe fn borrow<'a>(data: *const u8, length: usize) -> &'a [u8] {
    if length == 0 {
        return &[];
    }
    // SAFETY: The ABI caller supplies the documented readable, immutable span.
    unsafe { std::slice::from_raw_parts(data, length) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustUtf8Encoded" ["probe"] => Encoded {
        bytes,
        length,
    };
    "OpenTTDRustUtf8Decoded" ["probe"] => Decoded {
        length,
        codepoint,
    };
}
