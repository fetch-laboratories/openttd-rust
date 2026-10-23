/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Byte algorithms; C++ retains borrowed views, diagnostics, and cursor commit.

/// Bounded read/skip decision with a cursor commit performed only after logging.
#[repr(C)]
pub struct BoundResult {
    /// Number of available requested bytes.
    pub length: usize,
    /// Absolute cursor after consuming those bytes.
    pub position: usize,
    /// One for a short request, except the all-remaining sentinel.
    pub shortfall: u8,
}

/// Exact little-endian bits or a matched character, without signed conversion.
#[repr(C)]
pub struct ByteResult {
    /// Unsigned value bits; zero when no value is available.
    pub value_bits: u64,
    /// Width on success, zero for a short read or nonmatching character.
    pub length: usize,
}

/// Prefix result distinguishes an empty successful match from failure.
#[repr(C)]
pub struct MatchResult {
    /// Bytes consumed by a successful conditional read, otherwise zero.
    pub length: usize,
    /// One if the prefix matches, including an empty prefix at end.
    pub matched: u8,
}

/// Separator policies distinguish returned bytes from cursor advancement.
#[repr(C)]
pub struct SeparatorResult {
    /// Length of the view returned by peek/read.
    pub result_length: usize,
    /// Bytes consumed by read/skip.
    pub consumed_length: usize,
}

pub fn bound(size: usize, position: usize, requested: usize) -> BoundResult {
    let remaining = size - position;
    let all = requested == usize::MAX;
    let length = if all {
        remaining
    } else {
        requested.min(remaining)
    };
    BoundResult {
        length,
        position: position + length,
        shortfall: u8::from(!all && requested > remaining),
    }
}

pub fn little_endian(bytes: &[u8], width: u8) -> ByteResult {
    assert!(matches!(width, 1 | 2 | 4 | 8));
    let width = usize::from(width);
    if bytes.len() < width {
        return ByteResult {
            value_bits: 0,
            length: 0,
        };
    }
    let mut value_bits = 0;
    for (index, byte) in bytes[..width].iter().enumerate() {
        value_bits |= u64::from(*byte) << (index * 8);
    }
    ByteResult {
        value_bits,
        length: width,
    }
}

pub fn prefix(bytes: &[u8], pattern: &[u8]) -> MatchResult {
    let matched = bytes.starts_with(pattern);
    MatchResult {
        length: if matched { pattern.len() } else { 0 },
        matched: u8::from(matched),
    }
}

pub fn find(bytes: &[u8], pattern: &[u8], mode: u8) -> usize {
    assert_ne!(pattern, []);
    let found = match mode {
        0 => bytes
            .windows(pattern.len())
            .position(|window| window == pattern),
        1 => bytes.iter().position(|byte| pattern.contains(byte)),
        2 => bytes.iter().position(|byte| !pattern.contains(byte)),
        _ => unreachable!(),
    };
    found.unwrap_or(usize::MAX)
}

pub fn character(bytes: &[u8], pattern: &[u8], member: bool) -> ByteResult {
    assert_ne!(pattern, []);
    match bytes.first() {
        Some(byte) if pattern.contains(byte) == member => ByteResult {
            value_bits: u64::from(*byte),
            length: 1,
        },
        _ => ByteResult {
            value_bits: 0,
            length: 0,
        },
    }
}

pub fn separator(bytes: &[u8], pattern: &[u8], policy: i32) -> SeparatorResult {
    let start = find(bytes, pattern, 0);
    if start == usize::MAX {
        return SeparatorResult {
            result_length: bytes.len(),
            consumed_length: bytes.len(),
        };
    }
    let mut end = start;
    match policy {
        0 | 4 => {
            while bytes[end..].starts_with(pattern) {
                end += pattern.len();
            }
        }
        1 | 3 => end += pattern.len(),
        _ => {} // KEEP and unknown enum values retain the original default.
    }
    SeparatorResult {
        result_length: if matches!(policy, 0 | 1) { end } else { start },
        consumed_length: end,
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustConsumerBound" ["probe"] => BoundResult {
        length,
        position,
        shortfall,
    };
    "OpenTTDRustConsumerByte" ["probe"] => ByteResult {
        value_bits,
        length,
    };
    "OpenTTDRustConsumerMatch" ["probe"] => MatchResult {
        length,
        matched,
    };
    "OpenTTDRustConsumerSeparator" ["probe"] => SeparatorResult {
        result_length,
        consumed_length,
    };
}
