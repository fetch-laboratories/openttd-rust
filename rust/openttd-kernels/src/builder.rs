/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `BaseStringBuilder`'s numeric encoders (`core/string_builder.hpp`): the binary
//! writers' little-endian bytes and `PutIntegerBase`'s text, with the original
//! 32-byte formatting capacity. The builders and their sinks stay in C++.

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

/// `PutUint16LE`, `PutUint32LE` and `PutUint64LE`'s bytes, low byte first.
pub fn little_endian(value: u64) -> LittleEndian {
    LittleEndian {
        bytes: value.to_le_bytes(),
    }
}

/// `PutIntegerBase`: append integer 'value' in given number 'base'. This is its
/// `std::to_chars` into a 32-byte buffer; a zero length is its failed conversion,
/// which appends nothing.
pub fn integer(bits: u64, negative: bool, base: i32) -> FormattedInteger {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    // std::to_chars requires a base in the range 2 through 36.
    if !(2..=36).contains(&base) {
        crate::services_assert::check(false);
    }
    let base = u64::from(base.unsigned_abs());
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

/// Retired `tools/integer_probe.cpp` builder stream (#736 U), frozen from the pinned
/// original in `integer/probe-oracle.txt`. The tests mirror the candidate C++ glue: integer
/// formatting calls `PutBuffer` once unless the Rust length is zero, the binary writers send
/// the low `width` little-endian bytes, and `PutUtf8` always calls `PutBuffer`, even empty.
#[cfg(test)]
mod tests {
    use super::{integer, little_endian};
    use crate::consumer::tests::hex_or_dash;
    use std::fmt::Write as _;

    /// `DumpBuilder`: event count, then inside-call flag, phase, length and bytes.
    fn dump(events: &[(usize, Vec<u8>)]) -> String {
        events
            .iter()
            .fold(events.len().to_string(), |mut text, (phase, bytes)| {
                let _ = write!(text, " 1 {phase} {} {}", bytes.len(), hex_or_dash(bytes));
                text
            })
    }

    /// `PutIntegerBase<T>` through the adapter: sign-extend to `uint64_t`, negative flag.
    fn put_integer(
        events: &mut Vec<(usize, Vec<u8>)>,
        phase: usize,
        width: u32,
        signed: bool,
        bits: u64,
        base: i32,
    ) {
        let negative = signed && bits >> (width - 1) & 1 != 0;
        let extended = if negative && width < 64 {
            bits | u64::MAX << width
        } else {
            bits
        };
        let result = integer(extended, negative, base);
        if result.length != 0 {
            events.push((phase, result.bytes[..result.length].to_vec()));
        }
    }

    /// Representable range of one probe integer type.
    fn range(width: u32, signed: bool) -> (i128, i128) {
        if signed {
            (-(1_i128 << (width - 1)), (1_i128 << (width - 1)) - 1)
        } else {
            (0, i128::from(u64::MAX >> (64 - width)))
        }
    }

    /// One `builder_cases()` sample set: the range edges, plus each representable
    /// `base**30/31/32` neighbour and (signed) its negation, sorted and deduplicated.
    fn samples(width: u32, signed: bool, base: i32) -> Vec<i128> {
        let (low, high) = range(width, signed);
        let mut samples = vec![low, low + 1, 0, 1, high - 1, high];
        if signed {
            samples.push(-1);
        }
        let neighbours = [30_u32, 31, 32]
            .into_iter()
            .filter_map(|exponent| i128::from(base).checked_pow(exponent))
            .flat_map(|boundary| [boundary - 1, boundary, boundary + 1]);
        let candidates = neighbours
            .flat_map(|value| [Some(value), signed.then_some(-value)])
            .flatten();
        samples.extend(candidates.filter(|value| (low..=high).contains(value)));
        samples.sort_unstable();
        samples.dedup();
        samples
    }

    /// Two's complement masking, as Python's `value & mask`.
    fn masked(value: i128, width: u32) -> u64 {
        u64::try_from(value.rem_euclid(1 << 64)).unwrap() & (u64::MAX >> (64 - width))
    }

    /// `compare-integers.py` `builder_cases()`: width, signedness, base, masked bits.
    fn corpus() -> Vec<(u32, bool, i32, u64)> {
        let types = [8_u32, 16, 32, 64]
            .into_iter()
            .flat_map(|width| [(width, false), (width, true)]);
        let formats =
            types.flat_map(|(width, signed)| (2..=36_i32).map(move |base| (width, signed, base)));
        formats
            .flat_map(|(width, signed, base)| {
                samples(width, signed, base)
                    .into_iter()
                    .map(move |value| (width, signed, base, masked(value, width)))
            })
            .collect()
    }

    #[test]
    fn integer_probe_builder_formats_match_original() {
        let oracle = crate::oracle::parse(include_str!("integer/probe-oracle.txt"));
        let cases = corpus().into_iter().map(|(width, signed, base, bits)| {
            let mut events = Vec::new();
            put_integer(&mut events, 0, width, signed, bits, base);
            (
                format!("width {width} signed {signed} base {base} bits {bits:x}"),
                dump(&events),
            )
        });
        assert_eq!(crate::oracle::check(&oracle, "builder", cases), 1624);
    }

    /// The probe's ordered sink sequence, one phase per builder call.
    #[test]
    fn integer_probe_ordered_sinks_match_original() {
        let oracle = crate::oracle::parse(include_str!("integer/probe-oracle.txt"));
        let mut events: Vec<(usize, Vec<u8>)> = Vec::new();
        let binary: [(u64, usize); 12] = [
            (0, 1),
            (0x80, 1),
            (0xff, 1),
            (0, 2),
            (0x8000, 2),
            (0xffff, 2),
            (0, 4),
            (0x8000_0000, 4),
            (0xffff_ffff, 4),
            (0, 8),
            (0x8000_0000_0000_0000, 8),
            (u64::MAX, 8),
        ];
        for (phase, (value, width)) in binary.into_iter().enumerate() {
            events.push((phase + 1, little_endian(value).bytes[..width].to_vec()));
        }
        events.push((13, little_endian(0xff).bytes[..1].to_vec())); // PutChar(0xFF)
        for (phase, codepoint) in [
            (14, 0),
            (15, 0xd800),
            (16, 0x10_ffff),
            (17, 0x11_0000),
            (18, u32::MAX),
        ] {
            let encoded = crate::utf8::encode(codepoint);
            events.push((phase, encoded.bytes[..encoded.length].to_vec()));
        }
        put_integer(&mut events, 19, 64, false, 1 << 32, 2);
        put_integer(&mut events, 20, 32, false, 0xffff_ffff, 2);
        put_integer(&mut events, 21, 32, true, 0x8000_0000, 2);
        put_integer(&mut events, 22, 32, true, 0x8000_0001, 2);
        let cases = [("ordered sink sequence".to_string(), dump(&events))];
        assert_eq!(crate::oracle::check(&oracle, "ordered-sinks", cases), 1);
    }
}
