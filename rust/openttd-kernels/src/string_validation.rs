/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Historical sanitation decisions and borrowed in-place write progression.

use crate::control_codes::{
    SCC_ENCODED, SCC_ENCODED_INTERNAL, SCC_ENCODED_NUMERIC, SCC_ENCODED_STRING,
    SCC_RECORD_SEPARATOR, SCC_SPRITE_END, SCC_SPRITE_START,
};
use crate::string_type::{StringValidationSetting, StringValidationSettings};
use crate::utf8;

/// One scan decision. C++ commits consumption before any output/allocation.
#[repr(C)]
pub struct Step {
    pub consumed: usize,
    pub output: utf8::Encoded,
    pub stopped: u8,
}

/// Write position is unchanged on overtake; C++ performs fatal dispatch afterward.
#[repr(C)]
pub struct Write {
    pub position: usize,
    pub accepted: u8,
}

fn sprite(codepoint: u32) -> bool {
    (SCC_SPRITE_START..=SCC_SPRITE_END).contains(&codepoint)
}
/// `IsSccEncodedCode`: whether this is an encoded string control code.
fn is_scc_encoded_code(codepoint: u32) -> bool {
    matches!(
        codepoint,
        SCC_RECORD_SEPARATOR
            | SCC_ENCODED
            | SCC_ENCODED_INTERNAL
            | SCC_ENCODED_NUMERIC
            | SCC_ENCODED_STRING
    )
}

/// One `StrMakeValid` decision; the caller commits consumption before output.
pub fn step(bytes: &[u8], settings: StringValidationSettings) -> Step {
    let decoded = utf8::decode(bytes);
    let mut result = Step {
        consumed: decoded.length,
        output: utf8::encode(0x0011_0000),
        stopped: 0,
    };
    if bytes.is_empty() {
        result.stopped = 1;
    } else if decoded.length == 0 {
        /* Maybe the next byte is still a valid character? */
        // Malformed bytes never emit '?'.
        result.consumed = 1;
    } else if decoded.codepoint == 0 {
        result.stopped = 1;
    } else {
        let codepoint = decoded.codepoint;
        let newline = settings.test(StringValidationSetting::AllowNewline);
        if (crate::string_func::is_printable(codepoint) && !sprite(codepoint))
            || (settings.test(StringValidationSetting::AllowControlCode)
                && is_scc_encoded_code(codepoint))
            || (newline && codepoint == 10)
        {
            result.output = utf8::encode(codepoint);
        } else if newline && codepoint == 13 && bytes[decoded.length..].starts_with(b"\n") {
            /* Skip carriage return if followed by newline. */
            // LF is consumed by the next decision.
        } else if settings.test(StringValidationSetting::ReplaceTabCrNlWithSpace)
            && matches!(codepoint, 9 | 10 | 13)
        {
            /* Replace the tab, carriage return or newline with a space. */
            result.output = utf8::encode(u32::from(b' '));
        } else if settings.test(StringValidationSetting::ReplaceWithQuestionMark) {
            /* Replace the undesirable character with a question mark. */
            result.output = utf8::encode(u32::from(b'?'));
        }
    }
    result
}

/// `StrMakeValid(str, settings)`: the scan decisions of `step`, appended to a new string.
pub fn make_valid(mut bytes: &[u8], settings: StringValidationSettings) -> Vec<u8> {
    let mut result = Vec::new();
    /* Assume the ABSOLUTE WORST to be in str as it comes from the outside. */
    while !bytes.is_empty() {
        let step = step(bytes, settings);
        bytes = &bytes[step.consumed..];
        if step.stopped != 0 {
            break;
        }
        result.extend_from_slice(&step.output.bytes[..step.output.length]);
    }
    /* String termination, if needed, is left to the caller of this function. */
    result
}

/// `StrValid`: printable characters and NUL termination in a fixed-length buffer.
/// This is a span rather than a C string because termination is being checked.
pub fn valid(bytes: &[u8]) -> bool {
    /* Assume the ABSOLUTE WORST to be in str as it comes from the outside. */
    let mut position = 0;
    while position < bytes.len() {
        let decoded = utf8::decode(&bytes[position..]);
        if decoded.length == 0 {
            return false; // invalid codepoint
        }
        if decoded.codepoint == 0 {
            return true; // NUL termination
        }
        if !crate::string_func::is_printable(decoded.codepoint) || sprite(decoded.codepoint) {
            return false;
        }
        position += decoded.length;
    }
    false // missing NUL termination
}

/// Capacity uses the current external consumer, including native unsigned wrap.
pub fn write_plan(position: usize, consumed: usize, length: usize) -> Write {
    if length > consumed.wrapping_sub(position) {
        Write {
            position,
            accepted: 0,
        }
    } else {
        Write {
            position: position.wrapping_add(length),
            accepted: 1,
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDValidationStep" ["probe"] => Step {
        consumed,
        output,
        stopped,
    };
    "OpenTTDInplaceWrite" ["probe"] => Write {
        position,
        accepted,
    };
}

/// #736: original `utf8-comparison.cpp` validation, `StrValid` and in-place replacement
/// observations, frozen from the unchanged original (`utf8/comparison-oracle.txt`).
#[cfg(test)]
mod tests {
    use super::{make_valid, step, valid};
    use crate::oracle;
    use crate::utf8::tests::{ORACLE, original_encoding};
    use std::fmt::Write as _;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }

    /// One `InPlaceReplacement`: its consumer cursor and span, and its builder position.
    /// Copies and assignment keep each builder bound to its own consumer, as in C++.
    #[derive(Clone, Copy)]
    struct Pair {
        read: usize,
        span: usize,
        written: usize,
    }

    impl Pair {
        fn new(span: usize) -> Self {
            Self {
                read: 0,
                span,
                written: 0,
            }
        }

        /// `StringConsumer::Skip` within the span (all probe skips fit).
        fn skip(&mut self, length: usize) {
            assert!(length <= self.span - self.read);
            self.read += length;
        }

        /// `InPlaceBuilder::PutBuffer` through the production export; false is the C++
        /// facade's `NOT_REACHED` after an overtake, which leaves the position unchanged.
        #[allow(unsafe_code)]
        fn put(&mut self, buffer: *mut u8, source: *const u8, length: usize) -> bool {
            // SAFETY: Both pointers address the probe's live test buffers; accepted writes
            // stay within the consumer span, matching the export's contract.
            let result = unsafe {
                crate::openttd_rust_inplace_write(buffer, self.written, self.read, source, length)
            };
            if result.accepted == 0 {
                return false;
            }
            self.written = result.position;
            true
        }

        fn put_bytes(&mut self, buffer: &mut [u8], bytes: &[u8]) -> bool {
            self.put(buffer.as_mut_ptr(), bytes.as_ptr(), bytes.len())
        }

        /// The probe's `Pair` line: read, written, unused (native wrap), any-unused, buffer.
        fn line(&self, buffer: &[u8]) -> String {
            format!(
                "{} {} {} {} {}/",
                self.read,
                self.written,
                self.read.wrapping_sub(self.written),
                u8::from(self.read > self.written),
                hex(buffer)
            )
        }
    }

    /// The candidate `StrMakeValid` template loop (src/string.cpp) over an in-place pair:
    /// skip each step's consumption before writing its output.
    fn make_valid_in_place(buffer: &mut [u8], span: usize, settings: u8) -> Pair {
        let mut pair = Pair::new(span);
        while pair.read < span {
            let decision = step(
                &buffer[pair.read..span],
                crate::string_type::StringValidationSettings(settings),
            );
            pair.skip(decision.consumed);
            if decision.stopped != 0 {
                break;
            }
            if decision.output.length != 0 {
                let output = decision.output;
                assert!(pair.put_bytes(buffer, &output.bytes[..output.length]));
            }
        }
        pair
    }

    fn corpus() -> Vec<Vec<u8>> {
        const BOUNDARIES: [u32; 23] = [
            0, 1, 9, 10, 13, 0x1e, 0x1f, 0x20, 0x7f, 0x80, 0xd7ff, 0xd800, 0xdfff, 0xe000, 0xe001,
            0xe002, 0xe003, 0xe004, 0xe1ff, 0xe200, 0xe2ff, 0xe300, 0x10_ffff,
        ];
        let mut corpus: Vec<Vec<u8>> = [
            &b""[..],
            b"plain",
            b"\r\n\r\t\n",
            b"\0tail",
            b"a\0bad\xFF",
            b"\x80\xBF\xC0\x80",
            b"\xE0\x80\x80",
            b"\xF0\x80\x80\x80",
            b"a\xC2",
            b"\xE1\x80",
            b"\xF1\x80\x80",
            b"\xF4\x90\x80\x80",
            b"x\xFF\x80y",
            b"\xED\xA0\x80\xED\xBF\xBF",
        ]
        .map(<[u8]>::to_vec)
        .to_vec();
        let mut mixed = b"a\r\n\r\t\n".to_vec();
        for codepoint in BOUNDARIES {
            let bytes = original_encoding(codepoint);
            if codepoint != 0 {
                mixed.extend_from_slice(&bytes);
            }
            corpus.push(bytes);
        }
        corpus.push(mixed);
        corpus
    }

    /// One validation row: the `string_view`, in-place `std::string` and in-place `char *` results.
    fn validation_case(input: &[u8], settings: u8) -> String {
        // StrMakeValid(std::string_view): the same loop appending to a string.
        let output = make_valid(
            input,
            crate::string_type::StringValidationSettings(settings),
        );
        // StrMakeValidInPlace(std::string &): empty returns early; erase tail.
        let mut inplace = input.to_vec();
        if !inplace.is_empty() {
            let length = inplace.len();
            let written = make_valid_in_place(&mut inplace, length, settings).written;
            inplace.truncate(written);
        }
        // StrMakeValidInPlace(char *): strlen span, then a NUL if bytes remain.
        let mut cstring = input.to_vec();
        cstring.extend_from_slice(b"\0T!");
        let span = cstring.iter().position(|&byte| byte == 0).unwrap();
        let mut pair = make_valid_in_place(&mut cstring, span, settings);
        if pair.read > pair.written {
            assert!(pair.put_bytes(&mut cstring, b"\0"));
        }
        format!("{}/{}/{}/", hex(&output), hex(&inplace), hex(&cstring))
    }

    #[test]
    #[cfg_attr(
        not(target_pointer_width = "64"),
        ignore = "oracle captured from the original on 64-bit GNU/Linux; its size_t-dependent outputs differ on 32-bit targets"
    )]
    fn validation_and_in_place_replacement_match_original() {
        let oracle = oracle::parse(ORACLE);
        let corpus = corpus();
        let mut cases = Vec::new();
        for flags in 0..16_u8 {
            for unknown in [0, 0xf0] {
                // Unknown settings bits reach Rust unchanged; the original ignores them.
                let settings = flags | unknown;
                cases.extend(corpus.iter().enumerate().map(|(index, input)| {
                    (
                        format!("settings {settings} corpus {index}"),
                        validation_case(input, settings),
                    )
                }));
            }
        }
        assert_eq!(oracle::check(&oracle, "validation", cases), 1216);

        let checks = corpus.iter().enumerate().map(|(index, input)| {
            let mut terminated = input.clone();
            terminated.extend_from_slice(b"\0\xFFtail");
            (
                format!("corpus {index}"),
                format!(
                    "{} {}",
                    u8::from(valid(input)),
                    u8::from(valid(&terminated))
                ),
            )
        });
        assert_eq!(oracle::check(&oracle, "valid", checks), 38);
        assert_eq!(oracle::check(&oracle, "inplace", in_place_cases()), 12);
    }

    /// The probe's `InPlaceReplacement` copy/assign/overlap/wrap/overtake sequence.
    fn in_place_cases() -> Vec<(String, String)> {
        let mut cases = Vec::new();
        let mut buffer = *b"abcdefgh";
        let mut first = Pair::new(8);
        first.skip(4);
        assert!(first.put_bytes(&mut buffer, b"12"));
        let mut copy = first;
        first.skip(2);
        assert!(first.put_bytes(&mut buffer, b"A"));
        assert!(copy.put_bytes(&mut buffer, b"B"));
        copy.skip(1);
        assert!(copy.put_bytes(&mut buffer, b"C"));
        cases.push(("pair-first".into(), first.line(&buffer)));
        cases.push(("pair-copy".into(), copy.line(&buffer)));
        // Assignment copies the consumer and the builder's buffer/position.
        let mut assigned = first;
        assigned.skip(1);
        assert!(assigned.put_bytes(&mut buffer, b"D"));
        cases.push(("pair-assigned".into(), assigned.line(&buffer)));
        cases.push(("pair-self".into(), first.line(&buffer)));
        cases.push(("pair-copy-after".into(), copy.line(&buffer)));
        // No move constructor: std::move(copy) copies.
        let mut rvalue = copy;
        rvalue.skip(1);
        assert!(rvalue.put_bytes(&mut buffer, b"E"));
        cases.push(("pair-rvalue".into(), rvalue.line(&buffer)));
        cases.push(("pair-rvalue-source".into(), copy.line(&buffer)));

        let mut buffer = *b"abcdefgh";
        let mut pair = Pair::new(8);
        pair.skip(6);
        // Defined left overlap: source buffer[2..6] written from offset zero.
        let base = buffer.as_mut_ptr();
        // Both pointers derive from one base, so the overlapping copy aliases soundly.
        let source = base.wrapping_add(2).cast_const();
        assert!(pair.put(base, source, 4));
        cases.push(("left-overlap".into(), pair.line(&buffer)));
        // A reassigned consumer exposes the native subtraction wrap in capacity.
        pair.read = 0;
        pair.skip(1);
        assert!(pair.put_bytes(&mut buffer, b"Z"));
        cases.push(("live-wrap-capacity".into(), pair.line(&buffer)));

        let mut buffer = *b"abcd";
        let mut pair = Pair::new(4);
        let overtaken = !pair.put_bytes(&mut buffer, b"X");
        cases.push(("overtake".into(), u8::from(overtaken).to_string()));
        cases.push(("overtake-state".into(), pair.line(&buffer)));
        assert!(pair.put_bytes(&mut buffer, b""));
        cases.push(("zero-write".into(), pair.line(&buffer)));
        cases
    }
}
