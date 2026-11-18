/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `table/strings.h`: every string ID of the master language file and `STR_LAST_STRINGID`.
//!
//! The build script derives them from `src/lang/english.txt` with `openttd-strgen`, as strgen
//! assigns them, and every `CMake` build checks them against the `table/strings.h` its strgen wrote.
//! Import each constant by name.

use crate::strings_type::StringID;

include!(concat!(env!("OUT_DIR"), "/strings.rs"));

/// `openttd-strgen` runs before this crate is built, so it keeps copies of three kernels ports
/// until #998's shared crate. These tests hold each copy to its port on the port's oracle inputs;
/// the build check holds the copies' output on `english.txt` to C++ strgen's.
#[cfg(test)]
mod strgen_copies {
    use crate::strings_type::{StringTab, TAB_SIZE_BITS};
    use openttd_strgen::strgen_base;

    #[test]
    fn tab_constants_match_strings_type() {
        assert_eq!(strgen_base::TAB_SIZE_BITS, TAB_SIZE_BITS);
        assert_eq!(
            strgen_base::TEXT_TAB_END,
            u32::from(StringTab::TEXT_TAB_END.0)
        );
    }

    /// Every input the UTF-8 oracle decodes: length and codepoint, zero length on failure.
    #[test]
    fn decode_utf8_matches_utf8_decode() {
        let mut compared = 0;
        for (inputs, _) in crate::utf8::tests::codec_cases() {
            let Some(bytes) = inputs.strip_prefix("decode") else {
                continue;
            };
            let bytes: Vec<u8> = bytes
                .split_whitespace()
                .map(|byte| byte.parse().unwrap())
                .collect();
            let port = crate::utf8::decode(&bytes);
            let copy = strgen_base::decode_utf8(&bytes).unwrap_or((0, 0));
            assert_eq!(copy, (port.length, port.codepoint), "{inputs}");
            compared += 1;
        }
        assert!(compared > 300, "{compared} decode inputs");
    }

    /// Every 32-bit sample of the integer oracle, at base 0, unsigned, unclamped: value,
    /// consumed length and failure.
    #[test]
    fn parse_integer_base_matches_integer_parse() {
        let samples = crate::integer::tests::samples(32);
        for src in &samples {
            let port = crate::integer::parse(src, 0, 32, false, false);
            let copy = strgen_base::parse_integer_base(src);
            assert_eq!(
                copy.map_or((0, 0, true), |(length, value)| (
                    length,
                    u64::from(value),
                    false
                )),
                (port.length, port.value_bits, port.error_kind != 0),
                "{}",
                String::from_utf8_lossy(src)
            );
        }
        assert!(samples.len() > 50);
    }
}
