/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The master language file's string-ID assignment: `StringReader::ParseFile` and
//! `StringReader::HandleString` (`strgen_base.cpp`), the `##id` pragma of
//! `FileStringReader::HandlePragma` (`strgen.cpp`) and `HeaderWriter::WriteHeader`.
//!
//! Only the IDs are derived. Every other pragma is ignored, and strings are neither compiled nor
//! checked against the command table, so the language pack and `LANGUAGE_PACK_VERSION` stay with
//! C++ strgen. Where strgen reports an error or a fatal error for the master file, this fails.
//! The file is read as bytes, as strgen reads it; only string values are decoded, by `DecodeUtf8`.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

/// `TAB_SIZE_BITS` (`strings_type.h`). A copy of `openttd-kernels`' `strings_type::TAB_SIZE_BITS`
/// until #998's shared crate; a kernels test holds the two equal.
pub const TAB_SIZE_BITS: u32 = 11;
/// `TEXT_TAB_END` (`strings_type.h`), the end of the language files' tabs. A copy of
/// `openttd-kernels`' `strings_type::StringTab::TEXT_TAB_END` until #998's shared crate; a kernels
/// test holds the two equal.
pub const TEXT_TAB_END: u32 = 32;
/// `StringData::max_strings` of the language files: `StringData(TEXT_TAB_END)` holds
/// `tabs * TAB_SIZE`.
const MAX_STRINGS: u32 = TEXT_TAB_END * (1 << TAB_SIZE_BITS);
/// `StringConsumer::WHITESPACE_NO_NEWLINE`.
const WHITESPACE_NO_NEWLINE: &[u8] = b"\t\x0B\x0C\r ";
/// `StringConsumer::WHITESPACE_OR_NEWLINE`.
const WHITESPACE_OR_NEWLINE: &[u8] = b"\t\n\x0B\x0C\r ";

/// A line strgen rejects, with strgen's message.
#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    /// `_strgen.cur_line`, counted from 1.
    pub line: u32,
    /// The message strgen reports.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// The parts of `StringData` the master file's IDs depend on.
#[derive(Default)]
struct StringData {
    /// `StringData::strings`: the name at each populated ID.
    strings: BTreeMap<u32, Vec<u8>>,
    /// The names in `strings`, for `StringData::Find`.
    names: HashSet<Vec<u8>>,
    /// `StringData::next_string_id`.
    next_string_id: u32,
}

/// Assigns each string of the master file `text` its ID as strgen does, and returns the names and
/// IDs `HeaderWriter::WriteHeader` writes: in ID order, then `STR_LAST_STRINGID`.
///
/// # Errors
/// The first line strgen would report as an error or a fatal error, and a name that is not UTF-8,
/// which strgen writes but no Rust or C++ identifier can hold.
pub fn master_string_ids(text: &[u8]) -> Result<Vec<(String, u32)>, Error> {
    let mut data = StringData::default();
    let mut lines = text.split(|&b| b == b'\n');
    let mut cur_line = 1;
    while data.next_string_id < MAX_STRINGS {
        let Some(line) = lines.next() else {
            break;
        };
        handle_string(&mut data, trim(line, WHITESPACE_OR_NEWLINE), cur_line)?;
        cur_line += 1;
    }

    if data.next_string_id == MAX_STRINGS {
        return Err(Error {
            line: cur_line,
            message: format!("Too many strings, maximum allowed is {MAX_STRINGS}"),
        });
    }

    let mut header = Vec::with_capacity(data.strings.len() + 1);
    let mut last = 0;
    for (&id, name) in &data.strings {
        let name = String::from_utf8(name.clone()).map_err(|_| Error {
            line: cur_line,
            message: format!(
                "String name '{}' at 0x{id:X} is not UTF-8",
                String::from_utf8_lossy(name)
            ),
        })?;
        header.push((name, id));
        last = id;
    }
    header.push(("STR_LAST_STRINGID".to_owned(), last));
    Ok(header)
}

/// `StrTrimView`.
fn trim<'a>(src: &'a [u8], characters_to_trim: &[u8]) -> &'a [u8] {
    let Some(first) = src.iter().position(|b| !characters_to_trim.contains(b)) else {
        return &[];
    };
    let last = src
        .iter()
        .rposition(|b| !characters_to_trim.contains(b))
        .unwrap_or(first);
    &src[first..=last]
}

/// `IsUtf8Part` (`core/utf8.hpp`). A copy of `openttd-kernels`' `utf8::is_utf8_part` until #998's
/// shared crate.
const fn is_utf8_part(c: u8) -> bool {
    c & 0xC0 == 0x80
}

/// `DecodeUtf8` (`core/utf8.cpp`): the length and character at the start of `buf`, or `None` for
/// invalid data. It accepts encoded surrogates, as the original does. A copy of `openttd-kernels`'
/// `utf8::decode` until #998's shared crate; a kernels test holds the two equal on the UTF-8
/// oracle's inputs.
#[must_use]
pub fn decode_utf8(buf: &[u8]) -> Option<(usize, u32)> {
    match *buf {
        [b0, ..] if b0 & 0x80 == 0 => Some((1, u32::from(b0))),
        [b0, b1, ..] if b0 >> 5 == 6 => {
            let c = u32::from(b0 & 0x1F) << 6 | u32::from(b1 & 0x3F);
            (is_utf8_part(b1) && c >= 0x80).then_some((2, c))
        }
        [b0, b1, b2, ..] if b0 >> 4 == 14 => {
            let c = u32::from(b0 & 0x0F) << 12 | u32::from(b1 & 0x3F) << 6 | u32::from(b2 & 0x3F);
            (is_utf8_part(b1) && is_utf8_part(b2) && c >= 0x800).then_some((3, c))
        }
        [b0, b1, b2, b3, ..] if b0 >> 3 == 30 => {
            let c = u32::from(b0 & 0x07) << 18
                | u32::from(b1 & 0x3F) << 12
                | u32::from(b2 & 0x3F) << 6
                | u32::from(b3 & 0x3F);
            (is_utf8_part(b1)
                && is_utf8_part(b2)
                && is_utf8_part(b3)
                && (0x1_0000..=0x10_FFFF).contains(&c))
            .then_some((4, c))
        }
        _ => None,
    }
}

/// `StringReader::HandleString` for the master file.
fn handle_string(data: &mut StringData, src: &[u8], cur_line: u32) -> Result<(), Error> {
    let error = |message: String| Error {
        line: cur_line,
        message,
    };

    /* Ignore blank lines */
    if src.is_empty() {
        return Ok(());
    }

    if let Some(rest) = src.strip_prefix(b"#") {
        if let Some(pragma) = rest.strip_prefix(b"#")
            && !pragma.starts_with(b"#")
        {
            handle_pragma(data, pragma).map_err(error)?;
        }
        return Ok(()); // ignore comments
    }

    /* Read string name */
    let Some(colon) = src.iter().position(|&b| b == b':') else {
        return Err(error("Line has no ':' delimiter".to_owned()));
    };
    let mut str_name = trim(&src[..colon], WHITESPACE_NO_NEWLINE);

    /* Read string case */
    let mut casep = None;
    if let Some(index) = str_name.iter().position(|&b| b == b'.') {
        casep = Some(&str_name[index + 1..]);
        str_name = &str_name[..index];
    }

    /* Read string data */
    let value = &src[colon + 1..];

    /* Check string is valid UTF-8 */
    let mut rest = value;
    while !rest.is_empty() {
        let Some((len, c)) = decode_utf8(rest) else {
            return Err(error(format!(
                "Invalid UTF-8 sequence in '{}'",
                String::from_utf8_lossy(value)
            )));
        };
        if c <= 0x001F // ASCII control character range
            || c == 0x200B // Zero width space
            || (0xE000..=0xF8FF).contains(&c) // Private range
            || (0xFFF0..=0xFFFF).contains(&c)
        // Specials range
        {
            return Err(error(format!(
                "Unwanted UTF-8 character U+{c:04X} in sequence '{}'",
                String::from_utf8_lossy(value)
            )));
        }
        rest = &rest[len..];
    }

    let name = || String::from_utf8_lossy(str_name);
    if casep.is_some() {
        return Err(error(
            "Cases in the base translation are not supported.".to_owned(),
        ));
    }

    /* Check if this string already exists.. */
    if data.names.contains(str_name) {
        return Err(error(format!(
            "String name '{}' is used multiple times",
            name()
        )));
    }

    if let Some(used) = data.strings.get(&data.next_string_id) {
        return Err(error(format!(
            "String ID 0x{:X} for '{}' already in use by '{}'",
            data.next_string_id,
            name(),
            String::from_utf8_lossy(used)
        )));
    }

    /* Allocate a new LangString */
    data.names.insert(str_name.to_vec());
    data.strings.insert(data.next_string_id, str_name.to_vec());
    data.next_string_id += 1;
    Ok(())
}

/// The `id` branch of `FileStringReader::HandlePragma`; the master file's other pragmas set
/// language-pack metadata.
fn handle_pragma(data: &mut StringData, src: &[u8]) -> Result<(), String> {
    let (name, rest) = match src.iter().position(|&b| b == b' ') {
        Some(space) => {
            let rest = &src[space..];
            let skipped = rest.iter().position(|&b| b != b' ').unwrap_or(rest.len());
            (&src[..space], &rest[skipped..])
        }
        None => (src, &[][..]),
    };
    if name == b"id" {
        data.next_string_id = read_integer_base_0(rest)?;
    }
    Ok(())
}

/// Why `StringConsumer::ParseIntegerBase` gives no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerError {
    /// No digits: "Cannot parse integer".
    Invalid,
    /// Digits beyond `uint32_t`: "Integer out of range".
    OutOfRange,
}

/// `StringConsumer::ParseIntegerBase<uint32_t>(src, 0, false, ...)` (`core/string_consumer.hpp`):
/// a `0x` or `0X` prefix selects hexadecimal, otherwise decimal, and the longest digit prefix is
/// parsed; the consumed length and the value. A copy of `openttd-kernels`'
/// `integer::parse(src, 0, 32, false, false)` until #998's shared crate; a kernels test holds the
/// two equal on the integer oracle's inputs.
///
/// # Errors
/// No digits, or a value out of range.
pub fn parse_integer_base(src: &[u8]) -> Result<(usize, u32), IntegerError> {
    let (prefix, digits, radix) = match src.strip_prefix(b"0x").or_else(|| src.strip_prefix(b"0X"))
    {
        Some(hex) => (2, hex, 16),
        None => (0, src, 10),
    };
    let end = digits
        .iter()
        .position(|&b| !char::from(b).is_digit(radix))
        .unwrap_or(digits.len());
    if end == 0 {
        return Err(IntegerError::Invalid);
    }
    // The digits are ASCII.
    let text = std::str::from_utf8(&digits[..end]).unwrap_or_default();
    let value = u32::from_str_radix(text, radix).map_err(|_| IntegerError::OutOfRange)?;
    Ok((prefix + end, value))
}

/// `StringConsumer::ReadIntegerBase<uint32_t>(0)`: the value of [`parse_integer_base`]. Its
/// failure is an error strgen reports through `StringConsumer::LogError` as fatal.
fn read_integer_base_0(src: &[u8]) -> Result<u32, String> {
    let text = String::from_utf8_lossy(src);
    match parse_integer_base(src) {
        Ok((_, value)) => Ok(value),
        Err(IntegerError::Invalid) => Err(format!("Cannot parse integer: '{text}'")),
        Err(IntegerError::OutOfRange) => Err(format!("Integer out of range: '{text}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[(&str, u32)]) -> Vec<(String, u32)> {
        list.iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    }

    fn failure(text: impl AsRef<[u8]>) -> Error {
        master_string_ids(text.as_ref()).unwrap_err()
    }

    /// Expectations follow `HandleString`, the `id` pragma and `WriteHeader`: lines are trimmed,
    /// blank lines and `#`/`###` comments are skipped, `##` lines are pragmas, names are trimmed
    /// before the `:`, and the header lists IDs in ascending order with the highest one last.
    #[test]
    fn assigns_ids_as_strgen() {
        let text = "##name English (UK)\n\
                    ##id 0x0000\n\
                    STR_A   :a {STRING}\n\
                    ### STR_COMMENT :not a string\n\
                    # STR_OTHER_COMMENT :not a string\n\
                    \t STR_B\t: b \r\n\
                    \n\
                    ##plural 0\n\
                    ##id 0x10\n\
                    STR_C::c\n\
                    ##id   20\n\
                    STR_D :d\n\
                    ##id 0X8\n\
                    STR_E :\n";
        assert_eq!(
            master_string_ids(text.as_bytes()),
            Ok(ids(&[
                ("STR_A", 0),
                ("STR_B", 1),
                ("STR_E", 8),
                ("STR_C", 0x10),
                ("STR_D", 20),
                ("STR_LAST_STRINGID", 20),
            ]))
        );
    }

    /// `WriteHeader` starts `last` at 0.
    #[test]
    fn empty_file_has_last_id_zero() {
        assert_eq!(
            master_string_ids(b"##name x\n"),
            Ok(ids(&[("STR_LAST_STRINGID", 0)]))
        );
    }

    /// `ReadIntegerBase(0)` parses a digit prefix; no digits or an out-of-range value is an error
    /// `LogError` makes fatal in strgen.
    #[test]
    fn id_pragma_reads_integer_prefix() {
        assert_eq!(read_integer_base_0(b"0x1fg"), Ok(0x1F));
        assert_eq!(read_integer_base_0(b"42 junk"), Ok(42));
        assert_eq!(read_integer_base_0(b"4294967295"), Ok(u32::MAX));
        for bad in [&b"junk"[..], b"0x", b"-1", b""] {
            assert!(
                read_integer_base_0(bad)
                    .unwrap_err()
                    .starts_with("Cannot parse integer")
            );
        }
        assert!(
            read_integer_base_0(b"4294967296")
                .unwrap_err()
                .starts_with("Integer out of range")
        );
        assert_eq!(
            failure("STR_A :a\n##id junk\nSTR_B :b\n"),
            Error {
                line: 2,
                message: "Cannot parse integer: 'junk'".to_owned()
            }
        );
        assert_eq!(failure("##id 4294967296\n").line, 1);
    }

    /// strgen reads bytes: comments are never decoded, and `DecodeUtf8` accepts an encoded
    /// surrogate (`ED A0 80`, U+D800) in a value. Invalid UTF-8 in a value is fatal.
    #[test]
    fn reads_bytes_as_strgen() {
        assert_eq!(
            master_string_ids(b"# \xFF comment\nSTR_A :\xED\xA0\x80\n"),
            Ok(ids(&[("STR_A", 0), ("STR_LAST_STRINGID", 0)]))
        );
        assert_eq!(
            failure(b"STR_A :a\xFF\n").message,
            "Invalid UTF-8 sequence in 'a\u{FFFD}'"
        );
        assert_eq!(
            failure(b"STR_A :\xC0\x80\n").message,
            "Invalid UTF-8 sequence in '\u{FFFD}\u{FFFD}'"
        );
        assert_eq!(
            failure(b"STR_\xFF :a\n").message,
            "String name 'STR_\u{FFFD}' at 0x0 is not UTF-8"
        );
    }

    /// `ParseFile` stops reading at `max_strings` and reports an error only at exactly that ID.
    #[test]
    fn stops_at_max_strings() {
        assert_eq!(
            master_string_ids(b"##id 0x20000\nSTR_A :a\n"),
            Ok(ids(&[("STR_LAST_STRINGID", 0)]))
        );
        assert_eq!(
            failure("##id 0xFFFF\nSTR_A :a\nSTR_B :b\n"),
            Error {
                line: 3,
                message: "Too many strings, maximum allowed is 65536".to_owned()
            }
        );
    }

    #[test]
    fn rejects_lines_strgen_rejects() {
        assert_eq!(
            failure("STR_A a\n"),
            Error {
                line: 1,
                message: "Line has no ':' delimiter".to_owned()
            }
        );
        assert_eq!(
            failure("STR_A :a\nSTR_A.case :a\n").message,
            "Cases in the base translation are not supported."
        );
        assert_eq!(
            failure("STR_A :a\n\nSTR_A :b\n"),
            Error {
                line: 3,
                message: "String name 'STR_A' is used multiple times".to_owned()
            }
        );
        assert_eq!(
            failure("STR_A :a\n##id 0\nSTR_B :b\n").message,
            "String ID 0x0 for 'STR_B' already in use by 'STR_A'"
        );
        assert_eq!(
            failure("STR_A :a\u{200B}\n").message,
            "Unwanted UTF-8 character U+200B in sequence 'a\u{200B}'"
        );
    }
}
