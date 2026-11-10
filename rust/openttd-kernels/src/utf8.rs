/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/utf8.hpp` / `.cpp` byte codec and iterator positions. The original
//! `char32_t` domain includes surrogate codepoints; it is not Rust `char`.

use crate::bitmath_func::{gb, has_bit};

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

/// `IsUtf8Part`: check whether the byte is part of a UTF-8 sequence.
pub fn is_utf8_part(byte: u8) -> bool {
    gb(u32::from(byte), 6, 2) == 2
}

/// `EncodeUtf8`: binary data and length; length is zero outside its range.
pub fn encode(codepoint: u32) -> Encoded {
    // Every conversion is bounded by the branch threshold or an explicit mask.
    let mut result = Encoded {
        bytes: [0; 4],
        length: 0,
    };
    let part = |shift: u32| 0x80 + u8::try_from(gb(codepoint, shift, 6)).unwrap();
    if codepoint < 0x80 {
        result.bytes[0] = u8::try_from(codepoint).unwrap();
        result.length = 1;
    } else if codepoint < 0x800 {
        result.bytes[0] = 0xc0 + u8::try_from(gb(codepoint, 6, 5)).unwrap();
        result.bytes[1] = part(0);
        result.length = 2;
    } else if codepoint < 0x10000 {
        result.bytes[0] = 0xe0 + u8::try_from(gb(codepoint, 12, 4)).unwrap();
        result.bytes[1] = part(6);
        result.bytes[2] = part(0);
        result.length = 3;
    } else if codepoint < 0x0011_0000 {
        result.bytes[0] = 0xf0 + u8::try_from(gb(codepoint, 18, 3)).unwrap();
        result.bytes[1] = part(12);
        result.bytes[2] = part(6);
        result.bytes[3] = part(0);
        result.length = 4;
    }
    result
}

/// `DecodeUtf8`: length and codepoint; zero length denotes invalid input.
pub fn decode(bytes: &[u8]) -> Decoded {
    let valid = |length, codepoint| Decoded { length, codepoint };
    if !bytes.is_empty() && !has_bit(u32::from(bytes[0]), 7) {
        /* Single byte character: 0xxxxxxx */
        return valid(1, u32::from(bytes[0]));
    } else if bytes.len() >= 2 && gb(u32::from(bytes[0]), 5, 3) == 6 {
        if is_utf8_part(bytes[1]) {
            /* Double byte character: 110xxxxx 10xxxxxx */
            let codepoint = gb(u32::from(bytes[0]), 0, 5) << 6 | gb(u32::from(bytes[1]), 0, 6);
            if codepoint >= 0x80 {
                return valid(2, codepoint);
            }
        }
    } else if bytes.len() >= 3 && gb(u32::from(bytes[0]), 4, 4) == 14 {
        if is_utf8_part(bytes[1]) && is_utf8_part(bytes[2]) {
            /* Triple byte character: 1110xxxx 10xxxxxx 10xxxxxx */
            let codepoint = gb(u32::from(bytes[0]), 0, 4) << 12
                | gb(u32::from(bytes[1]), 0, 6) << 6
                | gb(u32::from(bytes[2]), 0, 6);
            if codepoint >= 0x800 {
                return valid(3, codepoint);
            }
        }
    } else if bytes.len() >= 4
        && gb(u32::from(bytes[0]), 3, 5) == 30
        && is_utf8_part(bytes[1])
        && is_utf8_part(bytes[2])
        && is_utf8_part(bytes[3])
    {
        /* 4 byte character: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
        let codepoint = gb(u32::from(bytes[0]), 0, 3) << 18
            | gb(u32::from(bytes[1]), 0, 6) << 12
            | gb(u32::from(bytes[2]), 0, 6) << 6
            | gb(u32::from(bytes[3]), 0, 6);
        if (0x10000..=0x0010_ffff).contains(&codepoint) {
            return valid(4, codepoint);
        }
    }
    valid(0, 0)
}

/// `Utf8View::iterator::operator++`: skip continuation bytes, including malformed runs.
pub fn next(bytes: &[u8], position: usize) -> usize {
    // The C++ facade asserts position < length, so this increment cannot overflow.
    let mut position = position + 1;
    while position < bytes.len() && is_utf8_part(bytes[position]) {
        position += 1;
    }
    position
}

/// `Utf8View::iterator::operator--`: move to the beginning of the preceding codepoint.
pub fn previous(bytes: &[u8], position: usize) -> usize {
    // The C++ facade asserts position > 0; its iterator has position <= length.
    let mut position = position - 1;
    while position > 0 && is_utf8_part(bytes[position]) {
        position -= 1;
    }
    position
}

/// `Utf8View::GetIterAtByte`: the offset may point inside a codepoint.
pub fn at_byte(bytes: &[u8], offset: usize) -> usize {
    if offset >= bytes.len() {
        return bytes.len();
    }
    /* Sanitize iterator to point to the start of a codepoint. */
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

/// #736: original `utf8-comparison.cpp` codec, view and consumer observations, frozen from
/// the unchanged original (see `utf8/comparison-oracle.txt`) and replayed on the Rust codec.
#[cfg(test)]
pub(crate) mod tests {
    use super::{at_byte, decode, encode, is_utf8_part, next, previous};
    use crate::oracle;
    use std::fmt::Write as _;

    pub(crate) const ORACLE: &str = include_str!("utf8/comparison-oracle.txt");

    /// The original `EncodeUtf8` byte formula, independent of the Rust encoder, for inputs.
    pub(crate) fn original_encoding(codepoint: u32) -> Vec<u8> {
        // GB(c, s, n) extracts n bits from s; the original writes nothing from 0x110000.
        let bits = |shift: u32, count: u32| {
            u8::try_from((codepoint >> shift) & ((1 << count) - 1)).unwrap()
        };
        match codepoint {
            0..0x80 => vec![bits(0, 7)],
            0x80..0x800 => vec![0xc0 + bits(6, 5), 0x80 + bits(0, 6)],
            0x800..0x1_0000 => vec![0xe0 + bits(12, 4), 0x80 + bits(6, 6), 0x80 + bits(0, 6)],
            0x1_0000..0x11_0000 => vec![
                0xf0 + bits(18, 3),
                0x80 + bits(12, 6),
                0x80 + bits(6, 6),
                0x80 + bits(0, 6),
            ],
            _ => Vec::new(),
        }
    }

    fn join(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, " {byte}");
            text
        })
    }

    /// `Decode` line output: decoded length and codepoint (C++ `DecodeUtf8` facade).
    fn decoded(cases: &mut Vec<(String, String)>, bytes: &[u8]) {
        let result = decode(bytes);
        cases.push((
            format!("decode{}", join(bytes)),
            format!("{} {}", result.length, result.codepoint),
        ));
    }

    /// The probe's codec section: encode with every prefix/corruption/trailing decode, then
    /// every single byte with `IsUtf8Part`, then the malformed decode list.
    fn codec_cases() -> Vec<(String, String)> {
        const BOUNDARIES: [u32; 25] = [
            0,
            1,
            0x7e,
            0x7f,
            0x80,
            0x81,
            0x7fe,
            0x7ff,
            0x800,
            0x801,
            0xd7ff,
            0xd800,
            0xd801,
            0xdffe,
            0xdfff,
            0xe000,
            0xfffe,
            0xffff,
            0x1_0000,
            0x1_0001,
            0x10_fffe,
            0x10_ffff,
            0x11_0000,
            0x11_0001,
            u32::MAX,
        ];
        const MALFORMED: [&[u8]; 29] = [
            b"",
            b"\0",
            b"\x80",
            b"\xBF",
            b"\xC0",
            b"\xC1",
            b"\xC0\x80",
            b"\xC1\xBF",
            b"\xE0\x80\x80",
            b"\xE0\x9F\xBF",
            b"\xF0\x80\x80\x80",
            b"\xF0\x8F\xBF\xBF",
            b"\xC2",
            b"\xE1",
            b"\xE1\x80",
            b"\xF1",
            b"\xF1\x80",
            b"\xF1\x80\x80",
            b"\xF4\x8F\xBF\xBF",
            b"\xF4\x90\x80\x80",
            b"\xF5\x80\x80\x80",
            b"\xF8\x80\x80\x80",
            b"\xFF\x80",
            b"\xED\xA0\x80",
            b"\xED\xBF\xBF",
            b"a\xFF\x80",
            b"\xC2\x80\xFF",
            b"\xE0\xA0\x80\xFF",
            b"\xF0\x90\x80\x80\xFF",
        ];
        let mut cases = Vec::new();
        for codepoint in BOUNDARIES {
            // The probe prints the length, then all four buffer bytes (zero past the length).
            let encoded = encode(codepoint);
            cases.push((
                format!("encode {codepoint}"),
                format!("{}{}", encoded.length, join(&encoded.bytes)),
            ));
            // Inputs come from the original formula, so a wrong encoder fails only above.
            let original = original_encoding(codepoint);
            decoded(&mut cases, &original);
            for length in 0..original.len() {
                decoded(&mut cases, &original[..length]);
            }
            for position in 1..original.len() {
                for invalid in [0x41, 0xc0, 0xff] {
                    let mut corrupted = original.clone();
                    corrupted[position] = invalid;
                    decoded(&mut cases, &corrupted);
                }
            }
            let mut trailing = original.clone();
            trailing.extend_from_slice(b"\xFF\x80");
            decoded(&mut cases, &trailing);
        }
        for byte in 0..=u8::MAX {
            decoded(&mut cases, &[byte]);
            cases.push((
                format!("part {byte}"),
                u8::from(is_utf8_part(byte)).to_string(),
            ));
        }
        for bytes in MALFORMED {
            decoded(&mut cases, bytes);
        }
        cases
    }

    /// `Utf8View` traversal through the C++ iterator facade, which dereferences with
    /// `DecodeUtf8` (placeholder `?` on failure) and moves with the Rust cursor helpers.
    fn view(bytes: &[u8]) -> String {
        let star = |position: usize| {
            let result = decode(&bytes[position..]);
            if result.length > 0 {
                result.codepoint
            } else {
                u32::from(b'?')
            }
        };
        let mut line = String::from("forward");
        let mut position = 0;
        while position != bytes.len() {
            let _ = write!(line, " {position}:{}", star(position));
            position = next(bytes, position);
        }
        let _ = write!(line, " end={} backward", bytes.len());
        while position != 0 {
            position = previous(bytes, position);
            let _ = write!(line, " {position}:{}", star(position));
        }
        line += " normalize";
        for offset in 0..=bytes.len() {
            let _ = write!(line, " {offset}:{}", at_byte(bytes, offset));
        }
        line += " next";
        for offset in 0..bytes.len() {
            let _ = write!(line, " {offset}:{}", next(bytes, offset));
        }
        line += " previous";
        for offset in 1..=bytes.len() {
            let _ = write!(line, " {offset}:{}", previous(bytes, offset));
        }
        // NDEBUG facade: offsets at or beyond the end return end.
        let _ = write!(
            line,
            " beyond={},{}",
            at_byte(bytes, bytes.len() + 1),
            at_byte(bytes, usize::MAX)
        );
        line
    }

    #[test]
    fn codec_view_and_consumer_match_original() {
        const VIEWS: [&[u8]; 11] = [
            b"",
            b"\0",
            b"\x80\xBF\x80",
            b"\x80\x80a",
            b"a\x80\x80",
            b"\xFF\x80\x80a\x80\xF0",
            b"\xC0\x80b\xC1\xBF",
            b"\xE0\x80\x80\0\x80",
            b"\x80\xFF\x80\xE1a\xBF",
            b"\xE1\x88\xB4\x80\x80a\xFF\x80\x80b\xF0",
            b"\xE1\x88\xB4a\0b\xF0\x92\x8D\x85",
        ];
        let oracle = oracle::parse(ORACLE);
        assert_eq!(oracle::check(&oracle, "codec", codec_cases()), 782);
        let views = VIEWS
            .iter()
            .map(|bytes| (format!("view{}", join(bytes)), view(bytes)));
        assert_eq!(oracle::check(&oracle, "view", views), 11);

        // Original `StringConsumer` UTF-8 methods (unchanged header bodies over the Rust
        // decoder) advance one byte on failure, unlike the view's continuation skip.
        let binary = b"\xFF\x80\x80a";
        let first = decode(binary);
        // TryReadUtf8 advances by the decoded length only on success (zero on failure).
        let try_read = first.length;
        let read = if first.length > 0 {
            first.codepoint
        } else {
            u32::from(b'?')
        };
        let advance = first.length.max(1);
        let consumer = format!(
            "try={}:{try_read} read={read}:{advance} skip={advance} view={}",
            u8::from(first.length > 0),
            next(binary, 0)
        );
        assert_eq!(
            oracle::check(&oracle, "consumer", [(String::from("FF808061"), consumer)]),
            1
        );
    }
}
