/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `StringConsumer`'s byte algorithms (`core/string_consumer.hpp`/`.cpp`); C++
//! retains the borrowed views, the diagnostics and the cursor commit.

/// `StringConsumer::SeparatorUsage`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SeparatorUsage(pub i32);
impl SeparatorUsage {
    /// Read all consecutive separators, and include them all in the result
    pub const READ_ALL_SEPARATORS: Self = Self(0);
    /// Read one separator, and include it in the result
    pub const READ_ONE_SEPARATOR: Self = Self(1);
    /// Keep the separator in the data as next value to be read.
    pub const KEEP_SEPARATOR: Self = Self(2);
    /// Read and discard one separator, do not include it in the result.
    pub const SKIP_ONE_SEPARATOR: Self = Self(3);
    /// Read and discard all consecutive separators, do not include any in the result.
    pub const SKIP_ALL_SEPARATORS: Self = Self(4);
}

/// [`find`]'s `StringConsumer::Find`: the first occurrence of the pattern.
pub const FIND_STRING: u8 = 0;
/// [`find`]'s `StringConsumer::FindCharIn`: the first byte in the pattern.
pub const FIND_CHAR_IN: u8 = 1;
/// [`find`]'s `StringConsumer::FindCharNotIn`: the first byte not in the pattern.
pub const FIND_CHAR_NOT_IN: u8 = 2;

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

/// `Peek`, `Skip` and `Read`'s clamping: `requested` is `npos` (`usize::MAX`) for
/// all remaining bytes; a longer request is a shortfall, which `Skip` and `Read`
/// log as "Source buffer too short" before moving to the end.
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

/// `PeekUint8`, `PeekUint16LE`, `PeekUint32LE` and `PeekUint64LE`: `std::nullopt`
/// (a zero length) if not enough data.
pub fn little_endian(bytes: &[u8], width: u8) -> ByteResult {
    if !matches!(width, 1 | 2 | 4 | 8) {
        crate::services_assert::check(false);
    }
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

/// `PeekIf`, `ReadIf` and `SkipIf`'s check: whether the data starts with `pattern`.
pub fn prefix(bytes: &[u8], pattern: &[u8]) -> MatchResult {
    let matched = bytes.starts_with(pattern);
    MatchResult {
        length: if matched { pattern.len() } else { 0 },
        matched: u8::from(matched),
    }
}

/// `Find`, `FindCharIn` or `FindCharNotIn`, by `mode`; `npos` (`usize::MAX`) if
/// not found.
pub fn find(bytes: &[u8], pattern: &[u8], mode: u8) -> usize {
    if pattern.is_empty() {
        crate::services_assert::check(false);
    }
    let found = match mode {
        FIND_STRING => bytes
            .windows(pattern.len())
            .position(|window| window == pattern),
        FIND_CHAR_IN => bytes.iter().position(|byte| pattern.contains(byte)),
        FIND_CHAR_NOT_IN => bytes.iter().position(|byte| !pattern.contains(byte)),
        _ => {
            crate::services_assert::check(false);
            None
        }
    };
    found.unwrap_or(usize::MAX)
}

/// `PeekCharIfIn` (`member`) or `PeekCharIfNotIn`: the first byte if it is (or is
/// not) in `pattern`.
pub fn character(bytes: &[u8], pattern: &[u8], member: bool) -> ByteResult {
    if pattern.is_empty() {
        crate::services_assert::check(false);
    }
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

/// `PeekUntil`'s result and `ReadUntil` and `SkipUntil`'s consumed length for the
/// separator `str` and usage `sep`.
pub fn until(bytes: &[u8], str: &[u8], sep: SeparatorUsage) -> SeparatorResult {
    let start = find(bytes, str, FIND_STRING);
    if start == usize::MAX {
        return SeparatorResult {
            result_length: bytes.len(),
            consumed_length: bytes.len(),
        };
    }
    let mut end = start;
    match sep {
        SeparatorUsage::READ_ALL_SEPARATORS | SeparatorUsage::SKIP_ALL_SEPARATORS => {
            while bytes[end..].starts_with(str) {
                end += str.len();
            }
        }
        SeparatorUsage::READ_ONE_SEPARATOR | SeparatorUsage::SKIP_ONE_SEPARATOR => {
            end += str.len();
        }
        // KEEP_SEPARATOR, and any other value, take the original `default`.
        SeparatorUsage::KEEP_SEPARATOR | SeparatorUsage(_) => {}
    }
    SeparatorResult {
        result_length: if matches!(
            sep,
            SeparatorUsage::READ_ALL_SEPARATORS | SeparatorUsage::READ_ONE_SEPARATOR
        ) {
            end
        } else {
            start
        },
        consumed_length: end,
    }
}

/// Raw adapter: [`until`] over a base `SeparatorUsage`, for the export and
/// `encoded.rs` until its pass.
pub fn separator(bytes: &[u8], pattern: &[u8], policy: i32) -> SeparatorResult {
    until(bytes, pattern, SeparatorUsage(policy))
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

/// Retired `tools/integer_probe.cpp` consumer stream (#736 U), frozen from the pinned
/// original in `integer/probe-oracle.txt`. [`tests::Consumer`] mirrors the candidate C++
/// `StringConsumer` glue line for line (cursor commits, the "Source buffer too short" and
/// integer diagnostics it formats from these Rust results); every computed value comes from
/// the production helpers behind `openttd_rust_consumer_*`, `openttd_rust_parse_integer`,
/// `openttd_rust_skip_integer` and `openttd_rust_decode_utf8`.
#[cfg(test)]
pub(crate) mod tests {
    use super::{bound, find, little_endian};
    use crate::{integer, utf8};
    use std::fmt::Write as _;

    pub(crate) fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }

    /// `"-"` for an empty observation, as the probe printed it.
    pub(crate) fn hex_or_dash(bytes: &[u8]) -> String {
        if bytes.is_empty() {
            "-".into()
        } else {
            hex(bytes)
        }
    }

    /// Candidate `StringConsumer` glue over the production Rust helpers. Each logged
    /// message records the cursor at logging time, as the probe's `DebugPrint` did.
    pub(crate) struct Consumer<'a> {
        pub(crate) src: &'a [u8],
        pub(crate) position: usize,
        pub(crate) log: Vec<(usize, Vec<u8>)>,
    }

    impl<'a> Consumer<'a> {
        pub(crate) fn new(src: &'a [u8]) -> Self {
            Self {
                src,
                position: 0,
                log: Vec::new(),
            }
        }

        fn left(&self) -> &'a [u8] {
            &self.src[self.position..]
        }

        fn log_error(&mut self, message: Vec<u8>) {
            self.log.push((self.position, message));
        }

        fn too_short(&mut self, requested: usize, length: usize) {
            self.log_error(format!("Source buffer too short: {requested} > {length}").into_bytes());
        }

        /// `Skip`, then `SkipClamped`: log before committing the bounded cursor.
        pub(crate) fn skip(&mut self, len: usize) {
            if len <= self.left().len() {
                self.position += len;
            } else {
                let bounds = bound(self.src.len(), self.position, len);
                if bounds.shortfall != 0 {
                    self.too_short(len, bounds.length);
                }
                self.position = bounds.position;
            }
        }

        /// `Peek`, then `PeekClamped`.
        pub(crate) fn peek(&self, len: usize) -> &'a [u8] {
            if len <= self.left().len() {
                return &self.left()[..len];
            }
            let bounds = bound(self.src.len(), self.position, len);
            &self.left()[..bounds.length]
        }

        /// `Read`, then `ReadClamped`: commit only after the logger returns.
        pub(crate) fn read(&mut self, len: usize) -> &'a [u8] {
            if len <= self.left().len() {
                let result = &self.left()[..len];
                self.position += len;
                return result;
            }
            let bounds = bound(self.src.len(), self.position, len);
            let result = &self.left()[..bounds.length];
            if bounds.shortfall != 0 {
                self.too_short(len, bounds.length);
            }
            self.position = bounds.position;
            result
        }

        /// `PeekUintN` through `PeekBinary`.
        pub(crate) fn peek_binary(&self, width: u8) -> Option<u64> {
            let result = little_endian(self.left(), width);
            (result.length != 0).then_some(result.value_bits)
        }

        /// `ReadUintN(def)`: peek, always advance by the width.
        pub(crate) fn read_binary(&mut self, width: u8, default: u64) -> u64 {
            let value = self.peek_binary(width);
            self.skip(usize::from(width));
            value.unwrap_or(default)
        }

        /// `TryReadUintN`: advance only on success.
        pub(crate) fn try_read_binary(&mut self, width: u8) -> Option<u64> {
            let value = self.peek_binary(width);
            if value.is_some() {
                self.skip(usize::from(width));
            }
            value
        }

        /// `PeekUtf8`: the C++ ASCII fast path returns what the Rust decoder returns.
        pub(crate) fn peek_utf8(&self) -> (usize, u32) {
            let decoded = utf8::decode(self.left());
            (decoded.length, decoded.codepoint)
        }

        /// `ReadUtf8(def)`: advance at least one byte.
        pub(crate) fn read_utf8(&mut self, default: u32) -> u32 {
            let (len, value) = self.peek_utf8();
            self.skip(if len > 0 { len } else { 1 });
            if len > 0 { value } else { default }
        }

        /// `SkipUtf8`: advance at least one byte.
        pub(crate) fn skip_utf8(&mut self) {
            let len = self.peek_utf8().0;
            self.skip(if len > 0 { len } else { 1 });
        }

        /// `TryReadUtf8`: advance only on success.
        pub(crate) fn try_read_utf8(&mut self) -> Option<u32> {
            let (len, value) = self.peek_utf8();
            if len == 0 {
                return None;
            }
            self.skip(len);
            Some(value)
        }

        /// `SkipUntilCharNotIn` through `FindBytes` mode 2; not found is `npos`.
        pub(crate) fn skip_until_char_not_in(&mut self, chars: &[u8]) {
            let len = find(self.left(), chars, super::FIND_CHAR_NOT_IN);
            self.skip(len);
        }

        /// `ParseIntegerBase<T>`: the Rust parse, plus the C++ diagnostic formatting.
        fn parse_integer(
            &mut self,
            base: u8,
            width: u8,
            signed: bool,
            clamp: bool,
            log_errors: bool,
        ) -> (usize, u64) {
            let src = self.left();
            let result = integer::parse(src, base, width, signed, clamp);
            if log_errors && result.error_kind != 0 {
                self.log_error(diagnostic(src, &result));
            }
            if result.length == 0 {
                return (0, 0);
            }
            (result.length, result.value_bits)
        }

        /// `PeekIntegerBase<T>`.
        pub(crate) fn peek_integer(
            &mut self,
            base: u8,
            width: u8,
            signed: bool,
            clamp: bool,
        ) -> (usize, u64) {
            self.parse_integer(base, width, signed, clamp, false)
        }

        /// `SkipIntegerBase`.
        pub(crate) fn skip_integer(&mut self, base: u8) {
            let len = integer::skip(self.left(), base);
            self.skip(len);
        }

        /// `TryReadIntegerBase<T>`.
        pub(crate) fn try_read_integer(
            &mut self,
            base: u8,
            width: u8,
            signed: bool,
            clamp: bool,
        ) -> Option<u64> {
            let (len, value) = self.peek_integer(base, width, signed, clamp);
            if len == 0 {
                return None;
            }
            self.skip_integer(base);
            Some(value)
        }

        /// `ReadIntegerBase<T>(base, def, clamp)`: log, then always advance.
        pub(crate) fn read_integer(
            &mut self,
            base: u8,
            width: u8,
            signed: bool,
            default: u64,
            clamp: bool,
        ) -> u64 {
            let (len, value) = self.parse_integer(base, width, signed, clamp, true);
            self.skip_integer(base);
            if len > 0 { value } else { default }
        }
    }

    /// The candidate `ParseIntegerBase<T>` diagnostic text for a failed Rust parse.
    fn diagnostic(src: &[u8], result: &integer::IntegerResult) -> Vec<u8> {
        let part = &src[result.error_offset..];
        let head = &part[..result.error_length.min(part.len())];
        let mut message = Vec::new();
        if result.error_kind == integer::INTEGER_NEGATIVE_HEX_OUT_OF_RANGE {
            message.extend_from_slice(b"Integer out of range: '");
            message.extend_from_slice(head);
            message.push(b'\'');
            return message;
        }
        let tail = &part[result.error_length..];
        message.extend_from_slice(if result.error_kind == integer::INTEGER_OUT_OF_RANGE {
            b"Integer out of range: '"
        } else {
            b"Cannot parse integer: '"
        });
        message.extend_from_slice(head);
        message.extend_from_slice(b"'+'");
        message.extend_from_slice(&tail[..tail.len().min(4)]);
        message.push(b'\'');
        message
    }

    /// `ParseInteger<T>`: surrounding non-newline whitespace, no trailing bytes.
    pub(crate) fn parse_integer(
        src: &[u8],
        base: u8,
        width: u8,
        signed: bool,
        clamp: bool,
    ) -> Option<u64> {
        const WHITESPACE_NO_NEWLINE: &[u8] = b"\t\x0b\x0c\r ";
        let mut consumer = Consumer::new(src);
        consumer.skip_until_char_not_in(WHITESPACE_NO_NEWLINE);
        let result = consumer.try_read_integer(base, width, signed, clamp);
        consumer.skip_until_char_not_in(WHITESPACE_NO_NEWLINE);
        if consumer.position < consumer.src.len() {
            return None;
        }
        result
    }

    pub(crate) fn messages(log: &[(usize, Vec<u8>)]) -> String {
        log.iter()
            .fold(log.len().to_string(), |mut text, (position, message)| {
                let _ = write!(text, " {position} {}", hex(message));
                text
            })
    }

    /// `compare-integers.py` `compare_consumers` corpus: operation, offset, request, bytes.
    fn corpus() -> Vec<(u8, usize, usize, &'static [u8])> {
        let maximum = usize::MAX;
        let mut corpus = Vec::new();
        for operation in 0..3 {
            for (offset, requested, src) in [
                (0, 0, &b""[..]),
                (0, maximum, b""),
                (1, 0, b"A\x00\xff"),
                (1, 8, b"A\x00\xff"),
                (1, maximum - 1, b"A\x00\xff"),
                (3, 0, b"A\x00\xff"),
                (3, maximum, b"A\x00\xff"),
                (1, 1, b"A\x00\xff"),
                (1, 2, b"A\x00\xff"),
            ] {
                corpus.push((operation, offset, requested, src));
            }
        }
        for operation in 3..12 {
            for (offset, src) in [
                (0, &b""[..]),
                (1, b"ABC"),
                (3, b"ABC"),
                (1, b"X\x01\x02\x03\x04\x05\x06\x07\x80"),
            ] {
                corpus.push((operation, offset, 0, src));
            }
        }
        for operation in 12..16 {
            for (offset, src) in [
                (0, &b""[..]),
                (0, b"\x00A"),
                (1, b"\x00A"),
                (1, b"A\xc2\x80B"),
                (3, b"A\xc2\x80B"),
                (4, b"A\xc2\x80B"),
                (1, b"A\xed\xa0\x80B"),
                (1, b"A\xff\x80B"),
                (2, b"A\xff\x80B"),
                (1, b"A\xf1\x80\x80"),
            ] {
                corpus.push((operation, offset, 0, src));
            }
        }
        corpus
    }

    fn optional(value: Option<u64>) -> String {
        format!("{}:{}", u8::from(value.is_some()), value.unwrap_or(42))
    }

    /// One probe line without its echoed operation number.
    fn observe(operation: u8, offset: usize, requested: usize, src: &[u8]) -> String {
        let mut consumer = Consumer::new(src);
        consumer.skip(offset);
        let observation = match operation {
            0 => hex(consumer.peek(requested)),
            1 => hex(consumer.read(requested)),
            2 => {
                consumer.skip(requested);
                String::new()
            }
            // ReadChar(42) is ReadUint8 through PeekChar; the probe prints its byte.
            3 | 8 => consumer.read_binary(1, 42).to_string(),
            4 => consumer.read_binary(2, 42).to_string(),
            5 => consumer.read_binary(4, 42).to_string(),
            6 => consumer.read_binary(8, 42).to_string(),
            7 | 10 => optional(consumer.try_read_binary(4)),
            9 => optional(consumer.try_read_binary(2)),
            11 => optional(consumer.try_read_binary(8)),
            12 => {
                let (length, value) = consumer.peek_utf8();
                format!("{length}:{value}")
            }
            13 => consumer.read_utf8(42).to_string(),
            14 => {
                consumer.skip_utf8();
                String::new()
            }
            15 => optional(consumer.try_read_utf8().map(u64::from)),
            _ => unreachable!(),
        };
        // Peek/Read return src.substr(position, length) in the C++ glue, so the borrowed
        // view always starts at the cursor; the probe's pointer flag is that fact.
        format!(
            "{} 1 {} {}",
            if observation.is_empty() {
                "-"
            } else {
                &observation
            },
            consumer.position,
            messages(&consumer.log)
        )
    }

    #[test]
    #[cfg_attr(
        not(target_pointer_width = "64"),
        ignore = "oracle captured from the original on 64-bit GNU/Linux; its size_t-dependent outputs differ on 32-bit targets"
    )]
    fn integer_probe_consumer_gaps_match_original() {
        let oracle = crate::oracle::parse(include_str!("integer/probe-oracle.txt"));
        let count = crate::oracle::check(
            &oracle,
            "consumer",
            corpus()
                .into_iter()
                .map(|(operation, offset, requested, src)| {
                    (
                        format!(
                            "op {operation} offset {offset} requested {requested} src {}",
                            hex_or_dash(src)
                        ),
                        observe(operation, offset, requested, src),
                    )
                }),
        );
        assert_eq!(count, 103);
    }
}
