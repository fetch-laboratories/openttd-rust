/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Encoded-string compatibility and serialization; results own all returned storage.

use crate::{builder, consumer, integer, utf8};

pub(super) const SEPARATOR: u32 = 0x1e;
pub(super) const ENCODED: u32 = 0xe000;
pub(super) const INTERNAL: u32 = 0xe001;
pub(super) const NUMERIC: u32 = 0xe002;
pub(super) const STRING: u32 = 0xe003;

/// Tagged scalar/borrowed-byte descriptor; never a C++ variant layout.
#[repr(C)]
pub struct Descriptor {
    /// Numeric bits, used only for tag 1.
    pub value: u64,
    /// Borrowed string bytes, used only for tag 2.
    pub bytes: *const u8,
    /// Borrowed byte length, used only for tag 2.
    pub length: usize,
    /// 0 empty, 1 unsigned integer, 2 arbitrary string bytes.
    pub kind: u8,
}

/// Immutable result view, valid only while its opaque Rust owner lives.
#[repr(C)]
pub struct View {
    /// Rust-owned output; empty output returns null.
    pub bytes: *const u8,
    /// Output byte length.
    pub length: usize,
    /// Number of ordered integer diagnostics.
    pub diagnostic_count: usize,
    /// Original-input remainder offset for assertion kind 1.
    pub assertion_offset: usize,
    /// Remainder length for kind 1, decoded first-character length for kind 2.
    pub assertion_length: usize,
    /// Forbidden string's first codepoint for assertion kind 2.
    pub assertion_codepoint: u32,
    /// One when compatibility output replaces the original string, zero for unchanged.
    pub apply: u8,
    /// 0 none, 1 numeric record remainder, 2 forbidden string prefix.
    pub assertion_kind: u8,
}

/// Byte spans are offsets in the operation's complete borrowed encoded input.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Diagnostic {
    /// First byte of the original integer error substring.
    pub offset: usize,
    /// Length of that substring.
    pub length: usize,
    /// Following preview length (up to four), bounded by the original record/input.
    pub tail_length: usize,
    /// Original integer error classification 1 invalid, 2 range, 3 negative hex.
    pub kind: u8,
}

#[derive(Clone, Copy)]
pub(super) enum Parameter<'a> {
    Empty,
    Integer(u64),
    String(&'a [u8]),
}

#[derive(Default)]
pub(super) struct Output {
    bytes: Vec<u8>,
    diagnostics: Vec<Diagnostic>,
    apply: bool,
    assertion_kind: u8,
    assertion_offset: usize,
    assertion_length: usize,
    assertion_codepoint: u32,
}

impl Output {
    pub(super) fn view(&self) -> View {
        View {
            bytes: if self.bytes.is_empty() {
                std::ptr::null()
            } else {
                self.bytes.as_ptr()
            },
            length: self.bytes.len(),
            diagnostic_count: self.diagnostics.len(),
            assertion_offset: self.assertion_offset,
            assertion_length: self.assertion_length,
            assertion_codepoint: self.assertion_codepoint,
            apply: u8::from(self.apply),
            assertion_kind: self.assertion_kind,
        }
    }

    pub(super) fn diagnostic(&self, index: usize) -> Diagnostic {
        self.diagnostics[index]
    }

    fn append(&mut self, bytes: &[u8]) {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .expect("encoded output size overflow");
        assert!(
            isize::try_from(length).is_ok(),
            "encoded output exceeds pointer range"
        );
        self.bytes.extend_from_slice(bytes);
    }

    fn character(&mut self, codepoint: u32) {
        let encoded = utf8::encode(codepoint);
        self.append(&encoded.bytes[..encoded.length]);
    }

    fn number(&mut self, value: u64) {
        let encoded = builder::integer(value, false, 16);
        self.append(&encoded.bytes[..encoded.length]);
    }

    fn error(&mut self, result: &integer::IntegerResult, base: usize, available: usize) {
        assert!(result.error_offset <= available);
        assert!(result.error_length <= available - result.error_offset);
        self.diagnostics.push(Diagnostic {
            offset: base + result.error_offset,
            length: result.error_length,
            tail_length: if result.error_kind == 3 {
                0
            } else {
                (available - result.error_offset - result.error_length).min(4)
            },
            kind: result.error_kind,
        });
    }

    fn serialize(&mut self, id: u32, parameters: &[Parameter<'_>], assertions: bool) {
        self.character(INTERNAL);
        self.number(u64::from(id));
        for parameter in parameters {
            self.character(SEPARATOR);
            match parameter {
                Parameter::Empty => {}
                Parameter::Integer(value) => {
                    self.character(NUMERIC);
                    self.number(*value);
                }
                Parameter::String(bytes) => {
                    if assertions {
                        let first = utf8::decode(bytes);
                        if first.length > 0
                            && matches!(first.codepoint, ENCODED | INTERNAL | SEPARATOR)
                        {
                            self.assertion_kind = 2;
                            self.assertion_length = first.length;
                            self.assertion_codepoint = first.codepoint;
                            return;
                        }
                    }
                    self.character(STRING);
                    self.append(bytes);
                }
            }
        }
    }
}

pub(super) fn serialize(id: u32, parameters: &[Parameter<'_>], assertions: bool) -> Output {
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    result.serialize(id, parameters, assertions);
    result
}

pub(super) fn legacy(src: &[u8], fix_code: bool) -> Output {
    if src.is_empty() {
        return Output::default();
    }
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    let mut position = 0;
    let mut is_encoded = false;
    let mut in_string = false;
    let mut need_type = true;
    while position < src.len() {
        let decoded = utf8::decode(&src[position..]);
        if decoded.length == 0 {
            break;
        }
        position += decoded.length;
        let character = decoded.codepoint;
        if character == ENCODED || (fix_code && matches!(character, 0xe028 | 0xe02a)) {
            result.character(ENCODED);
            need_type = false;
            is_encoded = true;
            continue;
        }
        if !is_encoded {
            return Output::default();
        }
        if character == u32::from(b'"') {
            in_string = !in_string;
            if in_string && need_type {
                result.character(STRING);
                need_type = false;
            }
            continue;
        }
        if !in_string && character == u32::from(b':') {
            result.character(SEPARATOR);
            need_type = true;
            continue;
        }
        if need_type {
            result.character(NUMERIC);
            need_type = false;
        }
        result.character(character);
    }
    result
}

fn marker_length(src: &[u8], codepoint: u32) -> usize {
    let decoded = utf8::decode(src);
    if decoded.length > 0 && decoded.codepoint == codepoint {
        decoded.length
    } else {
        0
    }
}

pub(super) fn negatives(src: &[u8]) -> Output {
    let mut position = marker_length(src, ENCODED);
    if position == 0 {
        return Output::default();
    }
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    result.character(ENCODED);
    let separator = utf8::encode(SEPARATOR);
    while position < src.len() {
        let copied = consumer::separator(&src[position..], &separator.bytes[..separator.length], 1);
        result.append(&src[position..position + copied.result_length]);
        position += copied.consumed_length;
        let length = marker_length(&src[position..], NUMERIC);
        if length == 0 {
            continue;
        }
        position += length;
        result.character(NUMERIC);
        let unsigned = integer::parse(&src[position..], 16, 64, false, false);
        let value = if unsigned.length > 0 {
            unsigned.value_bits
        } else {
            let signed = integer::parse(&src[position..], 16, 64, true, false);
            if signed.error_kind != 0 {
                result.error(&signed, position, src.len() - position);
            }
            signed.value_bits // Failure is the original default zero.
        };
        position += integer::skip(&src[position..], 16);
        result.number(value);
    }
    result
}

pub(super) fn replace<'a>(
    src: &'a [u8],
    index: usize,
    replacement: Parameter<'a>,
    numeric_assertions: bool,
    string_assertions: bool,
) -> Output {
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    let mut position = marker_length(src, INTERNAL);
    if position == 0 {
        return result;
    }
    let id = integer::parse(&src[position..], 16, 32, false, false);
    if id.length == 0 {
        return result;
    }
    position += integer::skip(&src[position..], 16);
    if position < src.len() {
        let length = marker_length(&src[position..], SEPARATOR);
        if length == 0 {
            return result;
        }
        position += length;
    }
    let separator = utf8::encode(SEPARATOR);
    let mut parameters = Vec::new();
    while position < src.len() {
        let lengths =
            consumer::separator(&src[position..], &separator.bytes[..separator.length], 3);
        let start = position;
        let record = &src[start..start + lengths.result_length];
        position += lengths.consumed_length;
        if record.is_empty() {
            parameters.push(Parameter::Empty);
            continue;
        }
        let parameter_type = utf8::decode(record);
        let consumed = if parameter_type.length == 0 {
            1
        } else {
            parameter_type.length
        };
        let payload = &record[consumed..];
        let parameter = match parameter_type.codepoint {
            NUMERIC if parameter_type.length > 0 => {
                let parsed = integer::parse(payload, 16, 64, false, false);
                if parsed.error_kind != 0 {
                    result.error(&parsed, start + consumed, payload.len());
                }
                let skipped = integer::skip(payload, 16);
                if numeric_assertions && skipped < payload.len() {
                    result.assertion_kind = 1;
                    result.assertion_offset = start + consumed + skipped;
                    result.assertion_length = payload.len() - skipped;
                    return result;
                }
                Parameter::Integer(parsed.value_bits)
            }
            STRING if parameter_type.length > 0 => Parameter::String(payload),
            _ => Parameter::Empty,
        };
        parameters.push(parameter);
    }
    if index >= parameters.len() {
        return result;
    }
    parameters[index] = replacement;
    let id = u32::try_from(id.value_bits).unwrap();
    result.serialize(id, &parameters, string_assertions);
    result
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustEncodedParameter" ["probe"] => Descriptor {
        value,
        bytes,
        length,
        kind,
    };
    "OpenTTDRustEncodedView" ["probe"] => View {
        bytes,
        length,
        diagnostic_count,
        assertion_offset,
        assertion_length,
        assertion_codepoint,
        apply,
        assertion_kind,
    };
    "OpenTTDRustEncodedDiagnostic" ["probe"] => Diagnostic {
        offset,
        length,
        tail_length,
        kind,
    };
}
