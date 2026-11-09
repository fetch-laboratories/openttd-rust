/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `StringConsumer::ParseIntegerBase` and `StringConsumer::SkipIntegerBase` from
//! `core/string_consumer.hpp` and `core/string_consumer.cpp`.

/// `IntegerResult::error_kind` of the "Cannot parse integer" diagnostic.
pub(crate) const INTEGER_INVALID: u8 = 1;
/// `IntegerResult::error_kind` of the "Integer out of range" diagnostic.
pub(crate) const INTEGER_OUT_OF_RANGE: u8 = 2;
/// `IntegerResult::error_kind` of the negative-hex "Integer out of range" diagnostic.
pub(crate) const INTEGER_NEGATIVE_HEX_OUT_OF_RANGE: u8 = 3;

/// Scalar parse result with byte spans for the original C++ diagnostic formatter.
/// Value bits are zero-extended from the requested integer width. Failure has
/// zero matched length/value; error kind 1/2/3 means invalid/range/negative-hex.
#[repr(C)]
#[derive(Default)]
pub struct IntegerResult {
    pub(crate) value_bits: u64,
    pub(crate) length: usize,
    pub(crate) error_offset: usize,
    pub(crate) error_length: usize,
    pub(crate) error_kind: u8,
}

/// The value of one digit of `base`, as `std::from_chars` reads it.
fn digit(byte: u8, base: u8) -> Option<u64> {
    let value = match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => return None,
    };
    (value < base).then_some(u64::from(value))
}

/// `src.starts_with("0x") || src.starts_with("0X")`.
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

/// `ParseIntegerBase<T>`, with `T` given as its `width` and signedness. If `base == 0`, then
/// a prefix '0x' decides between base 16 or base 10. With `clamp`, a valid number out of
/// range for T returns the maximum (or lowest) representable value; negative values for
/// unsigned results are still treated as invalid. The parser rejects leading whitespace
/// and unary plus.
pub(crate) fn parse(src: &[u8], base: u8, width: u8, signed: bool, clamp: bool) -> IntegerResult {
    let mask = u64::MAX >> (64 - width);
    let signed_min = 1_u64 << (width - 1);
    if base == 0 {
        // Try positive hex
        if hexadecimal(src) {
            let mut result = parse(&src[2..], 16, width, signed, clamp);
            if result.length > 0 {
                result.length += 2;
            } else {
                result.error_offset += 2;
            }
            return result;
        }
        // Try negative hex
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
                    return failure(INTEGER_NEGATIVE_HEX_OUT_OF_RANGE, 0, result.length);
                }
                result.value_bits = signed_min;
            }
            return result;
        }
        // Try decimal
        return parse(src, 10, width, signed, clamp);
    }

    // Base is 8, 10 or 16: we only support these bases when skipping.
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
        return failure(INTEGER_INVALID, 0, 0);
    }
    if overflow {
        if !clamp {
            return failure(INTEGER_OUT_OF_RANGE, 0, pos);
        }
        // The lowest value when negative, otherwise the maximum value.
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

/// `SkipIntegerBase`: skip an optional minus sign, a '0x'/'0X' prefix when `base == 0`, and
/// the digits of the resulting base. Returns the skipped length.
pub(crate) fn skip(src: &[u8], mut base: u8) -> usize {
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

/// Retired `tools/integer_probe.cpp` API stream (#736 U), frozen from the pinned original in
/// `integer/probe-oracle.txt`: `PeekIntegerBase`, `TryReadIntegerBase`, `ReadIntegerBase`
/// (with its logged diagnostics), `SkipIntegerBase` and `ParseInteger` for every width,
/// signedness, base and clamp over the `compare-integers.py` `cases()` corpus.
#[cfg(test)]
mod tests {
    use crate::consumer::tests::{Consumer, hex_or_dash, messages, parse_integer};

    fn octal(value: u128) -> String {
        format!("{value:o}")
    }

    /// `cases()` samples for one width, deduplicated in first-appearance order.
    fn samples(width: u32) -> Vec<Vec<u8>> {
        let mut samples: Vec<Vec<u8>> = [
            &b""[..],
            b"-",
            b"+1",
            b" 1",
            b"0x",
            b"-0x",
            b"0X-1",
            b"0x-1",
            b"0x-",
            b"-0x-1",
            b"0x+1",
            b"01",
            b"0129",
            b"0y",
            b"0x0x1",
            b"42\x00junk",
            b"x\x00\xff\n",
            b"0x\x00\xffab",
            b"999999999999999999999999999999999999999999x\x00\xff\nmore",
        ]
        .iter()
        .map(|sample| sample.to_vec())
        .collect();
        let half = 1_u128 << (width - 1);
        let full = 1_u128 << width;
        let mut bounds = vec![0, 1, half - 1, half, half + 1, full - 1, full, full + 1];
        bounds.sort_unstable();
        bounds.dedup();
        for value in bounds {
            let decimal = value.to_string();
            let hexadecimal = format!("{value:x}");
            let octal = octal(value);
            for text in [
                decimal.clone(),
                format!("-{decimal}"),
                format!("{decimal}xyz!tail"),
                format!("0x{hexadecimal}"),
                format!("-0x{hexadecimal}"),
                hexadecimal.clone(),
                format!("-{hexadecimal}"),
                octal.clone(),
                format!("-{octal}"),
            ] {
                samples.push(text.into_bytes());
            }
        }
        let mut seen = std::collections::HashSet::new();
        samples.retain(|sample| seen.insert(sample.clone()));
        samples
    }

    /// One probe line: peek, try-read, read (default 42, with diagnostics), skip, parse.
    fn probe(src: &[u8], width: u8, signed: bool, base: u8, clamp: bool) -> String {
        let mut peek = Consumer::new(src);
        let (len, value) = peek.peek_integer(base, width, signed, clamp);
        let mut attempt = Consumer::new(src);
        let tried = attempt.try_read_integer(base, width, signed, clamp);
        let mut read = Consumer::new(src);
        let result = read.read_integer(base, width, signed, 42, clamp);
        let mut skip = Consumer::new(src);
        skip.skip_integer(base);
        let whole = parse_integer(src, base, width, signed, clamp);
        format!(
            "{len} {value} {} {} {} {} {result} {} {} {} {} {}",
            peek.position,
            u8::from(tried.is_some()),
            tried.unwrap_or(42),
            attempt.position,
            read.position,
            skip.position,
            u8::from(whole.is_some()),
            whole.unwrap_or(42),
            messages(&read.log),
        )
    }

    /// `cases()` loop order outside the samples: width, signedness, base, clamp.
    fn modes() -> impl Iterator<Item = (u8, bool, u8, bool)> {
        let types = [8_u8, 16, 32, 64]
            .into_iter()
            .flat_map(|width| [(width, false), (width, true)]);
        let bases =
            types.flat_map(|(width, signed)| [0_u8, 8, 10, 16].map(|base| (width, signed, base)));
        bases.flat_map(|(width, signed, base)| {
            [(width, signed, base, false), (width, signed, base, true)]
        })
    }

    #[test]
    fn integer_probe_api_matches_original() {
        let oracle = crate::oracle::parse(include_str!("integer/probe-oracle.txt"));
        let mut cases = Vec::new();
        for (width, signed, base, clamp) in modes() {
            for src in &samples(u32::from(width)) {
                let inputs = format!(
                    "width {width} signed {signed} base {base} clamp {clamp} src {}",
                    hex_or_dash(src)
                );
                cases.push((inputs, probe(src, width, signed, base, clamp)));
            }
        }
        assert_eq!(crate::oracle::check(&oracle, "api", cases), 5312);
    }
}
