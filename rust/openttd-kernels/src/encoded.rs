/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Encoded-string compatibility and serialization; results own all returned storage.

use crate::consumer::SeparatorUsage;
use crate::control_codes::{
    SCC_ENCODED, SCC_ENCODED_INTERNAL, SCC_ENCODED_NUMERIC, SCC_ENCODED_STRING,
    SCC_RECORD_SEPARATOR,
};
use crate::strings_type::{StringID, StringParameterData};
use crate::{builder, consumer, integer, utf8};

// Existing native result protocol, unwrapped only by `view`.
const ASSERTION_NUMERIC_REMAINDER: u8 = 1;
const ASSERTION_ENCODED_PREFIX: u8 = 2;

/// Tagged scalar/borrowed-byte descriptor; never a C++ variant layout.
#[repr(C)]
#[derive(Clone, Copy)]
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

    /// The output bytes.
    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(super) fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
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
            tail_length: if result.error_kind == integer::INTEGER_NEGATIVE_HEX_OUT_OF_RANGE {
                0
            } else {
                (available - result.error_offset - result.error_length).min(4)
            },
            kind: result.error_kind,
        });
    }

    fn get_encoded_string_with_args(
        &mut self,
        id: StringID,
        parameters: &[StringParameterData<'_>],
        assertions: bool,
    ) {
        self.character(SCC_ENCODED_INTERNAL);
        self.number(u64::from(id.0));
        for parameter in parameters {
            self.character(SCC_RECORD_SEPARATOR);
            match parameter {
                StringParameterData::Empty => {}
                StringParameterData::Integer(value) => {
                    self.character(SCC_ENCODED_NUMERIC);
                    self.number(*value);
                }
                StringParameterData::String(bytes) => {
                    if assertions {
                        // The facade supplies the native WITH_ASSERT policy.
                        /* Don't allow an encoded string to contain another encoded string. */
                        let first = utf8::decode(bytes);
                        if first.length > 0
                            && matches!(
                                first.codepoint,
                                SCC_ENCODED | SCC_ENCODED_INTERNAL | SCC_RECORD_SEPARATOR
                            )
                        {
                            self.assertion_kind = ASSERTION_ENCODED_PREFIX;
                            self.assertion_length = first.length;
                            self.assertion_codepoint = first.codepoint;
                            return;
                        }
                    }
                    self.character(SCC_ENCODED_STRING);
                    self.append(bytes);
                }
            }
        }
    }
}

/// `GetEncodedStringWithArgs`: encode a string with its parameters, so it can
/// be stored and decoded later without storing parameters separately.
pub(super) fn get_encoded_string_with_args(
    id: StringID,
    parameters: &[StringParameterData<'_>],
    assertions: bool,
) -> Output {
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    result.get_encoded_string_with_args(id, parameters, assertions);
    result
}

/// `FixSCCEncoded`: fix old encoded markers before string validation runs.
/// Bail out on invalid input rather than continuing token replacement.
pub(super) fn fix_scc_encoded(src: &[u8], fix_code: bool) -> Output {
    if src.is_empty() {
        return Output::default();
    }
    /* Convert old escape-style encoding to record separator encoding.
     * Initial <SCC_ENCODED><STRINGID> stays the same.
     * :<SCC_ENCODED><STRINGID> becomes <RS><SCC_ENCODED><STRINGID>.
     * :<HEX> becomes <RS><SCC_ENCODED_NUMERIC><HEX>.
     * :"<STRING>" becomes <RS><SCC_ENCODED_STRING><STRING>.
     */
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    let mut position = 0;
    // Presence of SCC_ENCODED determines whether this is an encoded string.
    let mut is_encoded = false;
    // Inside a string between double quotes.
    let mut in_string = false;
    // A parameter type needs to be emitted.
    let mut need_type = true;
    while position < src.len() {
        let decoded = utf8::decode(&src[position..]);
        if decoded.length == 0 {
            break;
        }
        position += decoded.length;
        let character = decoded.codepoint;
        if character == SCC_ENCODED || (fix_code && matches!(character, 0xe028 | 0xe02a)) {
            result.character(SCC_ENCODED);
            need_type = false;
            is_encoded = true;
            continue;
        }
        /* If the first character is not SCC_ENCODED then no conversion is needed. */
        if !is_encoded {
            return Output::default();
        }
        if character == u32::from(b'"') {
            in_string = !in_string;
            if in_string && need_type {
                /* Started a new string parameter. */
                result.character(SCC_ENCODED_STRING);
                need_type = false;
            }
            continue;
        }
        if !in_string && character == u32::from(b':') {
            result.character(SCC_RECORD_SEPARATOR);
            need_type = true;
            continue;
        }
        if need_type {
            /* Started a new numeric parameter. */
            result.character(SCC_ENCODED_NUMERIC);
            need_type = false;
        }
        result.character(character);
    }
    result
}

/// `FixSCCEncodedNegative`: reencode negative numeric records as `uint64_t`.
pub(super) fn fix_scc_encoded_negative(src: &[u8]) -> Output {
    /* Check whether this is an encoded string. */
    let mut position = consumer::read_utf8_if(src, SCC_ENCODED);
    if position == 0 {
        return Output::default();
    }
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    result.character(SCC_ENCODED);
    let separator = utf8::encode(SCC_RECORD_SEPARATOR);
    while position < src.len() {
        /* Copy until next record. */
        let copied = consumer::until(
            &src[position..],
            &separator.bytes[..separator.length],
            SeparatorUsage::READ_ONE_SEPARATOR,
        );
        result.append(&src[position..position + copied.result_length]);
        position += copied.consumed_length;
        /* Check whether this is a numeric parameter. */
        let length = consumer::read_utf8_if(&src[position..], SCC_ENCODED_NUMERIC);
        if length == 0 {
            continue;
        }
        position += length;
        result.character(SCC_ENCODED_NUMERIC);
        /* First try unsigned. */
        let unsigned = integer::parse(&src[position..], 16, 64, false, false);
        let value = if unsigned.length > 0 {
            unsigned.value_bits
        } else {
            /* Read as signed, store as unsigned. */
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

/// `EncodedString::ReplaceParam`: return an empty result if decoding fails.
pub(super) fn replace_param<'a>(
    src: &'a [u8],
    index: usize,
    replacement: StringParameterData<'a>,
    numeric_assertions: bool,
    string_assertions: bool,
) -> Output {
    let mut result = Output {
        apply: true,
        ..Output::default()
    };
    let mut position = consumer::read_utf8_if(src, SCC_ENCODED_INTERNAL);
    if position == 0 {
        return result;
    }
    let id = integer::parse(&src[position..], 16, 32, false, false);
    if id.length == 0 {
        return result;
    }
    position += integer::skip(&src[position..], 16);
    if position < src.len() {
        let length = consumer::read_utf8_if(&src[position..], SCC_RECORD_SEPARATOR);
        if length == 0 {
            return result;
        }
        position += length;
    }
    let separator = utf8::encode(SCC_RECORD_SEPARATOR);
    let mut parameters = Vec::new();
    while position < src.len() {
        let lengths = consumer::until(
            &src[position..],
            &separator.bytes[..separator.length],
            SeparatorUsage::SKIP_ONE_SEPARATOR,
        );
        let start = position;
        let record = &src[start..start + lengths.result_length];
        position += lengths.consumed_length;
        if record.is_empty() {
            /* This is an empty parameter. */
            parameters.push(StringParameterData::Empty);
            continue;
        }
        /* Get the parameter type. */
        let parameter_type = utf8::decode(record);
        let consumed = if parameter_type.length == 0 {
            1
        } else {
            parameter_type.length
        };
        let payload = &record[consumed..];
        let parameter = match parameter_type.codepoint {
            SCC_ENCODED_NUMERIC if parameter_type.length > 0 => {
                let parsed = integer::parse(payload, 16, 64, false, false);
                if parsed.error_kind != 0 {
                    result.error(&parsed, start + consumed, payload.len());
                }
                let skipped = integer::skip(payload, 16);
                if numeric_assertions && skipped < payload.len() {
                    result.assertion_kind = ASSERTION_NUMERIC_REMAINDER;
                    result.assertion_offset = start + consumed + skipped;
                    result.assertion_length = payload.len() - skipped;
                    return result;
                }
                StringParameterData::Integer(parsed.value_bits)
            }
            SCC_ENCODED_STRING if parameter_type.length > 0 => StringParameterData::String(payload),
            // Unknown parameter, make it blank.
            _ => StringParameterData::Empty,
        };
        parameters.push(parameter);
    }
    if index >= parameters.len() {
        return result;
    }
    parameters[index] = replacement;
    let id = StringID(u32::try_from(id.value_bits).unwrap());
    result.get_encoded_string_with_args(id, &parameters, string_assertions);
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

/// #736: original `encoded-comparison.cpp` observations, frozen from the unchanged original
/// (`encoded/comparison-oracle.txt`) and replayed on the Rust operations. The C++ adapter
/// steps (`src/rust/encoded_adapter.hpp` and the candidate `strings.cpp`/`saveload.cpp`
/// facades) are reconstructed where they turn Rust results into logs, assertions and strings.
#[cfg(test)]
mod tests {
    use super::{
        Output, StringID, StringParameterData, fix_scc_encoded, fix_scc_encoded_negative,
        get_encoded_string_with_args, replace_param,
    };
    use crate::oracle;
    use std::collections::HashSet;
    use std::fmt::Write as _;

    const ORACLE: &str = include_str!("encoded/comparison-oracle.txt");
    const NUMERIC_ASSERTION: &str = "!record.AnyBytesLeft()";
    const STRING_ASSERTION: &str =
        "len == 0 || (c != SCC_ENCODED && c != SCC_ENCODED_INTERNAL && c != SCC_RECORD_SEPARATOR)";

    fn utf8(codepoint: u32) -> Vec<u8> {
        char::from_u32(codepoint).unwrap().to_string().into_bytes()
    }

    fn hex(bytes: &[u8]) -> String {
        if bytes.is_empty() {
            return "-".into();
        }
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }

    fn unhex(text: &str) -> Vec<u8> {
        if text == "-" {
            return Vec::new();
        }
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    /// The driver's `token`: E, N<hex>, or S<hex> ("S-" when empty).
    #[derive(Clone)]
    enum Token {
        E,
        N(u64),
        S(Vec<u8>),
    }

    fn token(value: &Token) -> String {
        match value {
            Token::E => "E".into(),
            Token::N(value) => format!("N{value:x}"),
            Token::S(bytes) => format!("S{}", hex(bytes)),
        }
    }

    fn s(bytes: &[u8]) -> Token {
        Token::S(bytes.to_vec())
    }

    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// `tools/encoded-comparison.py` `corpus()`, deduplicated in first-appearance order.
    fn corpus() -> Vec<String> {
        let mut records = Vec::new();
        legacy_records(&mut records);
        negative_records(&mut records);
        replace_records(&mut records);
        public_records(&mut records);
        let mut seen = HashSet::new();
        records.retain(|record| seen.insert(record.clone()));
        records
    }

    /// Legacy quote/colon conversions, without and with code fixing.
    fn legacy_records(records: &mut Vec<String>) {
        let encoded = utf8(0xe000);
        let e = &encoded[..];
        let mut legacy_values: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"text".to_vec(),
            b"\xff".to_vec(),
            cat(&[b"\xff", e]),
            cat(&[b"\xf0\x9f\x98\x80", e]),
            e.to_vec(),
            cat(&[e, b"0"]),
            cat(&[e, b"0\xffignored"]),
            cat(&[e, b"0:\xffignored"]),
            cat(&[e, b"0:\"x:y\""]),
            cat(&[e, b"0:\"unmatched"]),
            cat(&[e, b"0:\"", e, b"12:q\""]),
            cat(&[e, b"0:\"x\":"]),
            cat(&[e, b"0::1"]),
            cat(&[e, b"0:\x00:1"]),
            cat(&[e, b"0:\"x\"2"]),
            cat(&[e, b"0:\"x\"\"y\""]),
            cat(&[e, b"0:abc::\"\":-xyz"]),
        ];
        for marker in [utf8(0xe028), utf8(0xe02a)] {
            let m = &marker[..];
            legacy_values.push(cat(&[m, b"1"]));
            legacy_values.push(cat(&[m, b"1:\"x:y\":", m, b"2"]));
            legacy_values.push(cat(&[e, b"0:\"", m, b":x\""]));
        }
        for fix in 0..2 {
            for value in &legacy_values {
                records.push(format!("legacy {fix} {}", hex(value)));
            }
        }
    }

    /// Negative numeric repair records.
    fn negative_records(records: &mut Vec<String>) {
        let (encoded, internal, numeric, string) =
            (utf8(0xe000), utf8(0xe001), utf8(0xe002), utf8(0xe003));
        let (e, i, n, st) = (&encoded[..], &internal[..], &numeric[..], &string[..]);
        let rs: &[u8] = b"\x1e";
        for value in [
            b"".to_vec(),
            b"text".to_vec(),
            b"\xff".to_vec(),
            cat(&[i, b"0"]),
            e.to_vec(),
            cat(&[e, b"000A"]),
        ] {
            records.push(format!("negative {}", hex(&value)));
        }
        let values: [&[u8]; 21] = [
            b"0",
            b"0000",
            b"AaFF",
            b"00ABC",
            b"-0",
            b"-1",
            b"-7fffffffffffffff",
            b"-8000000000000000",
            b"-8000000000000001",
            b"ffffffffffffffff",
            b"10000000000000000",
            b"FFFFFFFFFFFFFFFF",
            b"",
            b"-",
            b"+1",
            b"xyz",
            b"-xyz",
            b"0x2",
            b"deadTAIL",
            b"-deadTAIL",
            b"\xff\x00bad",
        ];
        let suffixes: [Vec<u8>; 3] = [b"".to_vec(), cat(&[rs, st, b"tail"]), b"!suffix".to_vec()];
        for value in values {
            for suffix in &suffixes {
                let record = cat(&[e, b"0", rs, n, value, suffix]);
                records.push(format!("negative {}", hex(&record)));
            }
        }
        let last = cat(&[e, b"0", rs, n, b"-xyz", rs, n, b"10000000000000000"]);
        records.push(format!("negative {}", hex(&last)));
    }

    /// Internal-record parameter replacement.
    fn replace_records(records: &mut Vec<String>) {
        let (encoded, internal, numeric, string) =
            (utf8(0xe000), utf8(0xe001), utf8(0xe002), utf8(0xe003));
        let (e, i, n, st) = (&encoded[..], &internal[..], &numeric[..], &string[..]);
        let rs: &[u8] = b"\x1e";
        let replacements = [
            Token::E,
            Token::N(42),
            Token::N(u64::MAX),
            s(b""),
            s(b"new"),
            s(b"a\x00b"),
            s(&cat(&[b"a", rs, b"x"])),
        ];
        let items: Vec<Vec<Vec<u8>>> = vec![
            vec![],
            vec![b"".to_vec()],
            vec![b"".to_vec(), b"".to_vec()],
            vec![b"".to_vec(), b"".to_vec(), b"".to_vec()],
            vec![b"unknown".to_vec(), st.to_vec()],
            vec![cat(&[st, b"old"]), b"".to_vec(), cat(&[n, b"2a"])],
            vec![cat(&[n, b"-1"]), cat(&[st, b"tail"])],
            vec![cat(&[n, b"10000000000000000"]), b"unknown".to_vec()],
            vec![b"\xffunknown".to_vec(), cat(&[st, b"last"])],
            vec![cat(&[n, b"2fTAIL"]), cat(&[st, b"last"])],
            vec![cat(&[n, b"-"]), cat(&[n, b"1x"]), cat(&[n, b"xyz"])],
        ];
        for record in &items {
            let mut src = cat(&[i, b"000A"]);
            if !record.is_empty() {
                src.extend_from_slice(rs);
                src.extend_from_slice(&record.join(rs));
            }
            for index in [0, 1, 2, 99] {
                for replacement in &replacements {
                    records.push(format!(
                        "replace {index} {} {}",
                        token(replacement),
                        hex(&src)
                    ));
                }
            }
        }
        for src in [
            b"".to_vec(),
            cat(&[e, b"0"]),
            b"plain".to_vec(),
            b"\xff".to_vec(),
            i.to_vec(),
            cat(&[i, b"-1"]),
            cat(&[i, b"100000000"]),
            cat(&[i, b"0x2"]),
            cat(&[i, b"0x"]),
            cat(&[i, b"f", rs, n, b"1"]),
        ] {
            records.push(format!("replace 0 N2a {}", hex(&src)));
        }
    }

    /// Public serialization, clearing, replacement and default records.
    fn public_records(records: &mut Vec<String>) {
        let (encoded, internal) = (utf8(0xe000), utf8(0xe001));
        let (e, i) = (&encoded[..], &internal[..]);
        let rs: &[u8] = b"\x1e";
        let parameter_sets: Vec<Vec<Token>> = vec![
            vec![],
            vec![Token::E],
            vec![Token::E, Token::E],
            vec![Token::E, s(b""), Token::E],
            vec![Token::N(0), Token::N(u64::MAX)],
            vec![s(b"a\x00b"), Token::E, s(&cat(&[b"a", rs, b"x"]))],
            vec![s(b"\xffbad"), s(b"")],
            vec![s(b"old"), Token::N(42), s(b"tail")],
        ];
        for id in [0_u64, 42, u64::from(u32::MAX)] {
            for parameters in &parameter_sets {
                id_records(records, id, parameters);
            }
        }
        for prefix in [e, i, rs] {
            records.push(format!("serialize 0 S{}", hex(prefix)));
            records.push(format!("public-replace 0 0 S{} N1", hex(prefix)));
            records.push(format!("public-replace 0 99 S{} N1", hex(prefix)));
        }
        records.push("serialize 0 D-1 D-2".into());
        records.push("public-replace 0 0 D-2 D-1".into());
        for index in [0, 99, u64::MAX] {
            records.push(format!("default {index} N1"));
        }
    }

    /// Serialize, clear and each public replacement for one id and parameter set.
    fn id_records(records: &mut Vec<String>, id: u64, parameters: &[Token]) {
        let arguments = parameters.iter().map(token).collect::<Vec<_>>().join(" ");
        records.push(format!("serialize {id} {arguments}"));
        records.push(format!("clear {id} {arguments}"));
        let replacements = [Token::E, s(b""), s(b"n\x00w"), Token::N(15)];
        let pairs = [0, 1, 2, 99].into_iter().flat_map(|index| {
            replacements
                .iter()
                .map(move |replacement| (index, replacement))
        });
        for (index, replacement) in pairs {
            let replacement = token(replacement);
            records.push(format!(
                "public-replace {id} {index} {replacement} {arguments}"
            ));
        }
    }

    /// One compile mode of the retired probe: `NDEBUG` and `WITH_ASSERT`.
    #[derive(Clone, Copy)]
    struct Mode {
        ndebug: bool,
        with_assert: bool,
    }

    /// A failed assertion: libc abort (stderr expression) or the `WITH_ASSERT` handler.
    struct Failed(&'static str);

    /// `StringParameter` from the probe's `Parameter(token)`; D<dec> converts int64 to uint64.
    fn parameter(text: &str) -> StringParameterData<'_> {
        match text.as_bytes()[0] {
            b'E' => StringParameterData::Empty,
            b'N' => StringParameterData::Integer(u64::from_str_radix(&text[1..], 16).unwrap()),
            b'D' => StringParameterData::Integer(text[1..].parse::<i64>().unwrap().cast_unsigned()),
            _ => unreachable!("string parameters are decoded by the caller"),
        }
    }

    /// `LogRustEncodedDiagnostic`: the integer message, replayed against the original input.
    fn log_message(diagnostic: &super::Diagnostic, input: &[u8]) -> Vec<u8> {
        let part = &input[diagnostic.offset..diagnostic.offset + diagnostic.length];
        if diagnostic.kind == crate::integer::INTEGER_NEGATIVE_HEX_OUT_OF_RANGE {
            return [&b"Integer out of range: '"[..], part, b"'"].concat();
        }
        let label: &[u8] = if diagnostic.kind == crate::integer::INTEGER_OUT_OF_RANGE {
            b"Integer out of range"
        } else {
            b"Cannot parse integer"
        };
        let tail = diagnostic.offset + diagnostic.length;
        let tail = &input[tail..tail + diagnostic.tail_length];
        [label, b": '", part, b"'+'", tail, b"'"].concat()
    }

    struct Run {
        mode: Mode,
        stdout: Vec<String>,
    }

    impl Run {
        /// `CopyRustEncoded`: replay diagnostics against the input, assert, then copy.
        fn copy(&mut self, output: &Output, input: &[u8]) -> Result<Vec<u8>, Failed> {
            for diagnostic in output.diagnostics() {
                // The probe's DebugPrint prints each message as hex.
                self.stdout
                    .push(format!("log {}", hex(&log_message(diagnostic, input))));
            }
            let view = output.view();
            // Numeric assertions: !NDEBUG || WITH_ASSERT; string assertions: WITH_ASSERT.
            // The numeric check asserts that the trailing record text is empty.
            let start = view.assertion_offset.min(input.len());
            let record = &input[start..input.len().min(start + view.assertion_length)];
            if self.numeric()
                && view.assertion_kind == super::ASSERTION_NUMERIC_REMAINDER
                && !record.is_empty()
            {
                return Err(Failed(NUMERIC_ASSERTION));
            }
            // assert(len == 0 || (c != SCC_ENCODED && c != SCC_ENCODED_INTERNAL && ...)).
            if self.mode.with_assert
                && view.assertion_kind == super::ASSERTION_ENCODED_PREFIX
                && view.assertion_length != 0
                && matches!(view.assertion_codepoint, 0xe000 | 0xe001 | 0x1e)
            {
                return Err(Failed(STRING_ASSERTION));
            }
            Ok(output.bytes.clone())
        }

        fn numeric(&self) -> bool {
            !self.mode.ndebug || self.mode.with_assert
        }

        fn string(&self) -> bool {
            self.mode.with_assert
        }

        /// The probe's `Execute` for one record.
        fn execute(&mut self, command: &str) -> Result<(), Failed> {
            let words: Vec<&str> = command.split_whitespace().collect();
            let strings: Vec<Vec<u8>> = words
                .iter()
                .map(|word| {
                    word.strip_prefix('S')
                        .filter(|_| word.len() > 1)
                        .map(unhex)
                        .unwrap_or_default()
                })
                .collect();
            let param = |index: usize| -> StringParameterData<'_> {
                if words[index].starts_with('S') {
                    StringParameterData::String(&strings[index])
                } else {
                    parameter(words[index])
                }
            };
            let output = match words[0] {
                "legacy" | "negative" => {
                    let input = unhex(words[words.len() - 1]);
                    let result = if words[0] == "legacy" {
                        fix_scc_encoded(&input, words[1] == "1")
                    } else {
                        fix_scc_encoded_negative(&input)
                    };
                    // The facade leaves the string unchanged unless the result applies.
                    if result.apply {
                        self.copy(&result, &input)?
                    } else {
                        input
                    }
                }
                "replace" => {
                    // EndianBufferReader::ToValue reads the string up to its terminator.
                    let mut input = unhex(words[3]);
                    input.truncate(input.iter().position(|&b| b == 0).unwrap_or(input.len()));
                    let index = words[1].parse().unwrap();
                    let result =
                        replace_param(&input, index, param(2), self.numeric(), self.string());
                    self.copy(&result, &input)?
                }
                "default" => {
                    let index = words[1].parse().unwrap();
                    let result = replace_param(&[], index, param(2), self.numeric(), self.string());
                    self.copy(&result, &[])?
                }
                mode => {
                    let id = StringID(words[1].parse().unwrap());
                    let first = if mode == "public-replace" { 4 } else { 2 };
                    let parameters: Vec<_> = (first..words.len()).map(param).collect();
                    let encoded = self.copy(
                        &get_encoded_string_with_args(id, &parameters, self.string()),
                        &[],
                    )?;
                    let (input, index, replacement) = match mode {
                        "serialize" => {
                            self.stdout.push(format!("out {}", hex(&encoded)));
                            return Ok(());
                        }
                        // EncodedString::clear(), then ReplaceParam(0, 42).
                        "clear" => (Vec::new(), 0, StringParameterData::Integer(42)),
                        "public-replace" => (encoded, words[2].parse().unwrap(), param(3)),
                        _ => unreachable!("unknown probe command {command}"),
                    };
                    let result =
                        replace_param(&input, index, replacement, self.numeric(), self.string());
                    self.copy(&result, &input)?
                }
            };
            self.stdout.push(format!("out {}", hex(&output)));
            Ok(())
        }
    }

    /// Exit status, stdout lines, and a normalized libc assertion, as the oracle stores them.
    fn outcome(command: &str, mode: Mode) -> String {
        let mut run = Run {
            mode,
            stdout: Vec::new(),
        };
        let result = run.execute(command);
        let mut text = match &result {
            Ok(()) => "0".to_string(),
            Err(_) if !mode.ndebug => "-6".to_string(),
            Err(Failed(expression)) => {
                // NDEBUG + WITH_ASSERT: the probe's AssertFailedError prints and exits 3.
                run.stdout
                    .push(format!("assert {}", hex(expression.as_bytes())));
                "3".to_string()
            }
        };
        for line in &run.stdout {
            text += " ";
            text += line;
        }
        if let (Err(Failed(expression)), false) = (&result, mode.ndebug) {
            text += " abort ";
            text += expression;
        }
        text
    }

    #[test]
    #[cfg_attr(
        not(target_pointer_width = "64"),
        ignore = "oracle captured from the original on 64-bit GNU/Linux; its size_t-dependent outputs differ on 32-bit targets"
    )]
    fn encoded_operations_match_original() {
        let modes = [
            Mode {
                ndebug: false,
                with_assert: false,
            },
            Mode {
                ndebug: false,
                with_assert: true,
            },
            Mode {
                ndebug: true,
                with_assert: false,
            },
            Mode {
                ndebug: true,
                with_assert: true,
            },
        ];
        let records = corpus();
        assert_eq!(records.len(), 882);
        let cases = records.iter().flat_map(|command| {
            modes.iter().map(move |&mode| {
                (
                    format!(
                        "{command} (NDEBUG {}, WITH_ASSERT {})",
                        mode.ndebug, mode.with_assert
                    ),
                    outcome(command, mode),
                )
            })
        });
        let oracle = oracle::parse(ORACLE);
        assert_eq!(oracle::check(&oracle, "encoded", cases), 3528);
    }
}
