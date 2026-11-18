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

use std::collections::{BTreeMap, HashSet};
use std::fmt;

/// `StringData::max_strings` of the language files: `TEXT_TAB_END * TAB_SIZE`.
const MAX_STRINGS: u32 = 32 << 11;
/// `StringConsumer::WHITESPACE_NO_NEWLINE`.
const WHITESPACE_NO_NEWLINE: &[char] = &['\t', '\x0B', '\x0C', '\r', ' '];
/// `StringConsumer::WHITESPACE_OR_NEWLINE`.
const WHITESPACE_OR_NEWLINE: &[char] = &['\t', '\n', '\x0B', '\x0C', '\r', ' '];

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
    strings: BTreeMap<u32, String>,
    /// The names in `strings`, for `StringData::Find`.
    names: HashSet<String>,
    /// `StringData::next_string_id`.
    next_string_id: u32,
}

/// Assigns each string of the master file `text` its ID as strgen does, and returns the names and
/// IDs `HeaderWriter::WriteHeader` writes: in ID order, then `STR_LAST_STRINGID`.
///
/// # Errors
/// The first line strgen would report as an error or a fatal error.
pub fn master_string_ids(text: &str) -> Result<Vec<(String, u32)>, Error> {
    let mut data = StringData::default();
    let mut lines = text.split('\n');
    let mut cur_line = 1;
    while data.next_string_id < MAX_STRINGS {
        let Some(line) = lines.next() else {
            break;
        };
        handle_string(
            &mut data,
            line.trim_matches(WHITESPACE_OR_NEWLINE),
            cur_line,
        )?;
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
        header.push((name.clone(), id));
        last = id;
    }
    header.push(("STR_LAST_STRINGID".to_owned(), last));
    Ok(header)
}

/// `StringReader::HandleString` for the master file.
fn handle_string(data: &mut StringData, src: &str, cur_line: u32) -> Result<(), Error> {
    let error = |message: String| Error {
        line: cur_line,
        message,
    };

    /* Ignore blank lines */
    if src.is_empty() {
        return Ok(());
    }

    if let Some(rest) = src.strip_prefix('#') {
        if let Some(pragma) = rest.strip_prefix('#')
            && !pragma.starts_with('#')
        {
            handle_pragma(data, pragma);
        }
        return Ok(()); // ignore comments
    }

    /* Read string name */
    let Some((str_name, value)) = src.split_once(':') else {
        return Err(error("Line has no ':' delimiter".to_owned()));
    };
    let mut str_name = str_name.trim_matches(WHITESPACE_NO_NEWLINE);

    /* Read string case */
    let mut casep = None;
    if let Some(index) = str_name.find('.') {
        casep = Some(&str_name[index + 1..]);
        str_name = &str_name[..index];
    }

    /* Check string is valid UTF-8 */
    if let Some(c) = value.chars().find(|&c| {
        c <= '\u{1F}' // ASCII control character range
            || c == '\u{200B}' // Zero width space
            || ('\u{E000}'..='\u{F8FF}').contains(&c) // Private range
            || ('\u{FFF0}'..='\u{FFFF}').contains(&c) // Specials range
    }) {
        return Err(error(format!(
            "Unwanted UTF-8 character U+{:04X} in sequence '{value}'",
            u32::from(c)
        )));
    }

    if casep.is_some() {
        return Err(error(
            "Cases in the base translation are not supported.".to_owned(),
        ));
    }

    /* Check if this string already exists.. */
    if data.names.contains(str_name) {
        return Err(error(format!(
            "String name '{str_name}' is used multiple times"
        )));
    }

    if let Some(used) = data.strings.get(&data.next_string_id) {
        return Err(error(format!(
            "String ID 0x{:X} for '{str_name}' already in use by '{used}'",
            data.next_string_id
        )));
    }

    /* Allocate a new LangString */
    data.names.insert(str_name.to_owned());
    data.strings
        .insert(data.next_string_id, str_name.to_owned());
    data.next_string_id += 1;
    Ok(())
}

/// The `id` branch of `FileStringReader::HandlePragma`; the master file's other pragmas set
/// language-pack metadata.
fn handle_pragma(data: &mut StringData, src: &str) {
    let (name, rest) = match src.split_once(' ') {
        Some((name, rest)) => (name, rest.trim_start_matches(' ')),
        None => (src, ""),
    };
    if name == "id" {
        data.next_string_id = read_integer_base_0(rest);
    }
}

/// `StringConsumer::ReadIntegerBase<uint32_t>(0)`: a `0x` or `0X` prefix selects hexadecimal,
/// otherwise decimal; the longest digit prefix is parsed, and no digits or an out-of-range value
/// give 0.
fn read_integer_base_0(src: &str) -> u32 {
    let (digits, radix) = match src.strip_prefix("0x").or_else(|| src.strip_prefix("0X")) {
        Some(hex) => (hex, 16),
        None => (src, 10),
    };
    let end = digits
        .find(|c: char| !c.is_digit(radix))
        .unwrap_or(digits.len());
    u32::from_str_radix(&digits[..end], radix).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[(&str, u32)]) -> Vec<(String, u32)> {
        list.iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    }

    fn failure(text: &str) -> Error {
        master_string_ids(text).unwrap_err()
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
            master_string_ids(text),
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
            master_string_ids("##name x\n"),
            Ok(ids(&[("STR_LAST_STRINGID", 0)]))
        );
    }

    /// `ReadIntegerBase(0)` parses a digit prefix and gives 0 for no digits or overflow.
    #[test]
    fn id_pragma_reads_integer_prefix() {
        assert_eq!(read_integer_base_0("0x1fg"), 0x1F);
        assert_eq!(read_integer_base_0("42 junk"), 42);
        assert_eq!(read_integer_base_0("junk"), 0);
        assert_eq!(read_integer_base_0("0x"), 0);
        assert_eq!(read_integer_base_0("-1"), 0);
        assert_eq!(read_integer_base_0("4294967296"), 0);
        assert_eq!(read_integer_base_0(""), 0);
    }

    /// `ParseFile` stops reading at `max_strings` and reports an error only at exactly that ID.
    #[test]
    fn stops_at_max_strings() {
        assert_eq!(
            master_string_ids("##id 0x20000\nSTR_A :a\n"),
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
