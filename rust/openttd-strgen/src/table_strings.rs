/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The source of `openttd-kernels`' `table::strings`, the Rust counterpart of what
//! `HeaderFileWriter::WriteStringID` writes to `table/strings.h`. The build script writes
//! [`write`]'s text unchanged, and the build check reads the same text back with [`read`], so the
//! check compares the constants Rust compiles, not only the derived list.

use std::fmt::Write as _;

/// The prefix of each constant line.
const PREFIX: &str = "pub const ";

/// One documented `StringID` constant per name and ID.
#[must_use]
pub fn write(ids: &[(String, u32)]) -> String {
    let mut source = String::new();
    for (name, id) in ids {
        writeln!(
            source,
            "/// `{name}` of `table/strings.h`.\n{PREFIX}{name}: StringID = StringID(0x{id:X});"
        )
        .unwrap_or_default();
    }
    source
}

/// The names and IDs of the constant lines of `source`, in order.
///
/// # Errors
/// A constant line not in [`write`]'s form.
pub fn read(source: &str) -> Result<Vec<(String, u32)>, String> {
    let mut ids = Vec::new();
    for line in source.lines() {
        let Some(definition) = line.strip_prefix(PREFIX) else {
            continue;
        };
        let parsed = definition
            .strip_suffix(");")
            .and_then(|definition| definition.split_once(": StringID = StringID(0x"))
            .and_then(|(name, id)| Some((name.to_owned(), u32::from_str_radix(id, 16).ok()?)));
        ids.push(parsed.ok_or_else(|| format!("unreadable constant line: {line}"))?);
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_constants_read_back() {
        let ids = vec![
            ("STR_NULL".to_owned(), 0),
            ("STR_LAST_STRINGID".to_owned(), 0x8858),
        ];
        let source = write(&ids);
        assert_eq!(
            source,
            "/// `STR_NULL` of `table/strings.h`.\n\
             pub const STR_NULL: StringID = StringID(0x0);\n\
             /// `STR_LAST_STRINGID` of `table/strings.h`.\n\
             pub const STR_LAST_STRINGID: StringID = StringID(0x8858);\n"
        );
        assert_eq!(read(&source), Ok(ids));
    }

    #[test]
    fn unreadable_constant_line_fails() {
        assert_eq!(
            read("pub const STR_A: u32 = 1;\n"),
            Err("unreadable constant line: pub const STR_A: u32 = 1;".to_owned())
        );
    }
}
